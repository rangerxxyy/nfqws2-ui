use nfqws2_ui::transfer_diagnostic::{
    TransferAttempt, TransferClass, assemble_curl_args, assemble_curl_args_for_path,
    classify_transfer, parse_curl_attempt, should_probe_upload_stall, successful_http_response,
    upload_stall_confirmed,
};
use std::net::IpAddr;

#[test]
fn complete_short_response_is_not_a_boundary_stall() {
    let result = classify_transfer(
        &[attempt(8_000, None, 200), attempt(8_000, None, 200)],
        Some(8_000),
    );
    assert_eq!(result.classification, TransferClass::Complete);
    assert!(!result.repeatable_boundary_stall);
}

#[test]
fn completed_small_http_response_is_reachable_without_claiming_full_volume() {
    let attempts = [attempt(180, None, 200), attempt(180, None, 200)];
    assert!(successful_http_response(&attempts));
    assert_eq!(
        classify_transfer(&attempts, None).classification,
        TransferClass::PartialTransfer
    );
}

#[test]
fn short_response_without_known_expected_size_is_not_assumed_complete() {
    let result = classify_transfer(&[attempt(18_000, None, 200)], None);
    assert_eq!(result.classification, TransferClass::PartialTransfer);
    assert!(!result.repeatable_boundary_stall);
}

#[test]
fn repeated_timeout_after_boundary_is_reported_as_possible_transfer_interruption() {
    let result = classify_transfer(
        &[
            attempt(18_420, Some("timeout"), 200),
            attempt(18_415, Some("timeout"), 200),
        ],
        Some(65_536),
    );
    assert_eq!(result.classification, TransferClass::PossibleBoundaryStall);
    assert!(result.repeatable_boundary_stall);
    assert!(result.summary.contains("не доказывает DPI"));
}

#[test]
fn one_timeout_or_inconsistent_amount_is_inconclusive() {
    let one = classify_transfer(&[attempt(18_000, Some("timeout"), 200)], Some(65_536));
    assert_eq!(one.classification, TransferClass::Inconclusive);
    let inconsistent = classify_transfer(
        &[
            attempt(18_000, Some("timeout"), 200),
            attempt(49_000, Some("timeout"), 200),
        ],
        Some(65_536),
    );
    assert_eq!(inconsistent.classification, TransferClass::Inconclusive);
}

#[test]
fn timeout_before_body_and_reset_have_distinct_diagnoses() {
    let timeout = classify_transfer(
        &[
            attempt(0, Some("timeout"), 0),
            attempt(0, Some("timeout"), 0),
        ],
        Some(65_536),
    );
    assert_eq!(timeout.classification, TransferClass::TimeoutBeforeBody);
    let reset = classify_transfer(
        &[
            attempt(1_000, Some("reset"), 200),
            attempt(1_100, Some("reset"), 200),
        ],
        Some(65_536),
    );
    assert_eq!(reset.classification, TransferClass::ConnectionReset);
}

#[test]
fn curl_attempt_parser_keeps_partial_body_metrics_and_expected_size() {
    let attempt = parse_curl_attempt(
        28,
        b"NFQWS2_TRANSFER|200|8.0|0.04|18420",
        18_420,
        b"HTTP/1.1 200 OK\r\nContent-Length: 65536\r\n\r\n",
        8_000,
    );
    assert_eq!(attempt.attempt.bytes_received, 18_420);
    assert_eq!(attempt.attempt.first_byte_ms, Some(40));
    assert_eq!(attempt.attempt.transport_error.as_deref(), Some("timeout"));
    assert_eq!(attempt.attempt.http_status, Some(200));
    assert_eq!(attempt.expected_bytes, Some(65_536));
}

#[test]
fn curl_partial_file_exit_is_classified_as_incomplete_transfer() {
    let parsed = parse_curl_attempt(
        18,
        b"NFQWS2_TRANSFER|200|2.0|0.04|18000",
        18_000,
        b"HTTP/1.1 200 OK\r\nContent-Length: 65536\r\n\r\n",
        2_000,
    );
    assert_eq!(parsed.attempt.transport_error.as_deref(), Some("partial"));
    let result = classify_transfer(&[parsed.attempt], Some(65_536));
    assert_eq!(result.classification, TransferClass::PartialTransfer);
    assert!(!result.repeatable_boundary_stall);
}

#[test]
fn curl_exit_codes_keep_dns_connect_timeout_tls_and_reset_distinct() {
    for (code, expected) in [
        (6, "dns"),
        (7, "connect"),
        (28, "timeout"),
        (18, "partial"),
        (35, "tls"),
        (60, "tls"),
        (52, "reset"),
        (56, "reset"),
    ] {
        let parsed = parse_curl_attempt(code, b"", 80_000, b"", 2_000);
        assert_eq!(parsed.attempt.transport_error.as_deref(), Some(expected));
        assert_eq!(parsed.attempt.bytes_received, 65_536);
    }
}

#[test]
fn curl_args_pin_https_host_and_cap_volume_without_a_shell() {
    let args = assemble_curl_args(
        "example.com",
        "93.184.216.34".parse::<IpAddr>().unwrap(),
        "body",
        "headers",
    );
    assert!(args.windows(2).any(|pair| pair == ["--range", "0-65535"]));
    assert!(
        args.windows(2)
            .any(|pair| pair == ["--local-port", "40000-40999"])
    );
    assert!(args.contains(&"--disable".to_string()));
    assert!(args.iter().any(|arg| arg == "https://example.com/"));
    assert!(
        args.iter()
            .any(|arg| arg.contains("example.com:443:93.184.216.34"))
    );
    assert!(
        !args
            .iter()
            .any(|arg| arg.contains(';') || arg.contains('`'))
    );
}

#[test]
fn known_network_reference_path_is_preserved_in_the_pinned_probe_url() {
    let args = assemble_curl_args_for_path(
        "proof.ovh.net",
        "93.184.216.34".parse().unwrap(),
        "/files/1Mb.dat",
        "/tmp/body",
        "/tmp/headers",
    );
    assert!(args.contains(&"https://proof.ovh.net/files/1Mb.dat".into()));
    assert!(!args.iter().any(|arg| arg.contains("/files/1Mb.dat;")));
}

#[test]
fn upload_stall_probe_is_limited_to_known_static_download_endpoints() {
    assert!(!should_probe_upload_stall("example.com", "/"));
    assert!(!should_probe_upload_stall("proof.ovh.net", "/"));
    assert!(!should_probe_upload_stall("proof.ovh.net", "/api/action"));
    assert!(should_probe_upload_stall("proof.ovh.net", "/files/1Mb.dat"));
}

#[test]
fn small_response_upload_stall_requires_timeout_after_tls_handshake() {
    assert!(upload_stall_confirmed(
        28,
        "NFQWS2_UPLOAD|0.241000",
        "NFQWS2_UPLOAD"
    ));
    assert!(!upload_stall_confirmed(
        28,
        "NFQWS2_UPLOAD|0.000000",
        "NFQWS2_UPLOAD"
    ));
    assert!(!upload_stall_confirmed(
        28,
        "NFQWS2_UPLOAD|not-a-time",
        "NFQWS2_UPLOAD"
    ));
    assert!(!upload_stall_confirmed(
        0,
        "NFQWS2_UPLOAD|0.241000",
        "NFQWS2_UPLOAD"
    ));
}

fn attempt(bytes: u64, error: Option<&str>, status: u16) -> TransferAttempt {
    TransferAttempt {
        bytes_received: bytes,
        sample_bytes: 65_536,
        http_status: Some(status),
        elapsed_ms: 140,
        first_byte_ms: Some(45),
        transport_error: error.map(str::to_string),
        upload_stalled_after_tls: false,
    }
}

#[test]
fn small_page_upload_timeout_is_distinguished_from_inbound_16kb_stall() {
    let mut first = attempt(180, None, 200);
    first.upload_stalled_after_tls = true;
    let second = attempt(180, None, 200);
    let attempts = [first, second];
    let result = classify_transfer(&attempts, Some(180));
    assert_eq!(result.classification, TransferClass::PossibleUploadStall);
    assert!(!successful_http_response(&attempts));
    assert!(!result.repeatable_boundary_stall);
    assert!(result.summary.contains("После TLS handshake"));
}
