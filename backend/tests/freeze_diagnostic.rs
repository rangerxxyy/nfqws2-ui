use nfqws2_ui::freeze_diagnostic::{
    AsnVerification, FreezeNetworkResult, FreezeStrategyMatch, FreezeTargetKind, PAD_REQUESTS,
    PadClassification, build_targets, candidate_confirmed, candidate_route_succeeded,
    classify_padded_head, confirmed_strategy_candidates, freeze_candidates, instantiate_template,
    is_freeze_classification, network_has_freeze, padded_head_curl_args, verify_asn_hint,
};
use nfqws2_ui::transfer_diagnostic::{TransferAttempt, TransferClass, classify_transfer};
use std::net::Ipv4Addr;

#[test]
fn bounded_catalog_contains_host_references_and_pinned_network_targets() {
    let targets = build_targets(&[]).unwrap();
    assert_eq!(targets.len(), 20);
    assert_eq!(
        targets
            .iter()
            .filter(|target| target.kind == FreezeTargetKind::HostReference)
            .count(),
        7
    );
    assert_eq!(
        targets
            .iter()
            .filter(|target| target.kind == FreezeTargetKind::PinnedAddress)
            .count(),
        13
    );
    assert!(
        targets
            .iter()
            .filter(|target| target.kind == FreezeTargetKind::PinnedAddress)
            .all(|target| target.asn_hint.is_some())
    );
}

#[test]
fn extra_public_hosts_are_normalized_deduplicated_and_bounded() {
    let targets =
        build_targets(&["https://Example.COM/path".into(), "example.com".into()]).unwrap();
    assert_eq!(targets.len(), 21);
    assert_eq!(targets[0].host, "example.com");
    assert!(build_targets(&["router.lan".into()]).is_err());
    assert!(build_targets(&vec!["site.example".into(); 21]).is_err());
}

#[test]
fn freeze_network_results_expose_domain_path_ip_and_asn_for_the_ui() {
    let result = FreezeNetworkResult {
        label: "Hetzner".into(),
        host: "hetzner.com".into(),
        path: "/".into(),
        target_kind: FreezeTargetKind::HostReference,
        address: Some(Ipv4Addr::new(91, 98, 156, 82)),
        asn_hint: Some(24940),
        asn: AsnVerification::Verified { asn: 24940 },
        direct: unreachable_measurement(),
        current: unreachable_measurement(),
        freeze_detected: false,
        tried_candidates: 0,
        found_strategy: None,
        candidate_attempts: Vec::new(),
    };
    let result = serde_json::to_value(result).unwrap();
    assert_eq!(result["host"], "hetzner.com");
    assert_eq!(result["address"], "91.98.156.82");
    assert_eq!(result["asnHint"], 24940);
    let mut legacy = result.clone();
    legacy.as_object_mut().unwrap().remove("targetKind");
    let old_result: FreezeNetworkResult = serde_json::from_value(legacy).unwrap();
    assert_eq!(old_result.target_kind, FreezeTargetKind::HostReference);
}

fn unreachable_measurement() -> nfqws2_ui::freeze_diagnostic::FreezeMeasurement {
    nfqws2_ui::freeze_diagnostic::FreezeMeasurement {
        classification: "Inconclusive".into(),
        ok: false,
        bytes_received: None,
        replies: None,
        elapsed_ms: 0,
        reason: "not probed".into(),
    }
}

#[test]
fn padded_head_probe_pins_the_public_address_and_bounds_each_request() {
    let args = padded_head_curl_args(Ipv4Addr::new(93, 184, 216, 34), 443, "example.com").unwrap();
    assert!(args.contains(&"--fail-early".into()));
    assert!(args.contains(&"--local-port".into()));
    assert!(
        args.windows(2)
            .any(|pair| pair == ["--connect-to", "example.com:443:93.184.216.34:443"])
    );
    assert!(
        args.iter()
            .any(|arg| arg == &format!("X-Pad: {}", "a".repeat(4000)))
    );
    assert_eq!(
        args.iter()
            .filter(|arg| arg.ends_with("?n=0") || arg.contains("?n="))
            .count(),
        PAD_REQUESTS
    );
    assert!(padded_head_curl_args(Ipv4Addr::new(93, 184, 216, 34), 443, "example.com;id").is_err());
}

#[test]
fn padded_head_requires_all_responses_on_one_connection() {
    let success = (0..PAD_REQUESTS)
        .map(|_| "200|0|0.02")
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        classify_padded_head(0, &success).classification,
        PadClassification::Complete
    );
    let reconnect = (0..PAD_REQUESTS)
        .map(|index| {
            if index == 5 {
                "200|1|0.02"
            } else {
                "200|0|0.02"
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    assert_eq!(
        classify_padded_head(0, &reconnect).classification,
        PadClassification::ServerDoesNotKeepConnection
    );
    assert_eq!(
        classify_padded_head(0, "200|0|0.02\n200|0|0.02").classification,
        PadClassification::ServerDoesNotKeepConnection
    );
}

#[test]
fn padded_timeout_after_response_is_distinct_from_timeout_before_response() {
    assert_eq!(
        classify_padded_head(28, "200|0|0.02\n200|0|0.02").classification,
        PadClassification::PossibleFreeze
    );
    assert_eq!(
        classify_padded_head(28, "").classification,
        PadClassification::TimeoutBeforeResponse
    );
}

#[test]
fn baseline_freeze_requires_repeatable_boundary_not_only_a_tcp_connected_timeout() {
    let before_body = [
        attempt(0, Some("timeout"), 0),
        attempt(0, Some("timeout"), 0),
    ];
    assert!(!network_has_freeze(&before_body, Some(65_536)));
    let boundary = [
        attempt(18_000, Some("timeout"), 200),
        attempt(18_400, Some("timeout"), 200),
    ];
    assert_eq!(
        classify_transfer(&boundary, Some(65_536)).classification,
        TransferClass::PossibleBoundaryStall
    );
    assert!(network_has_freeze(&boundary, Some(65_536)));
}

#[test]
fn freeze_candidate_search_starts_only_after_a_response_or_upload_stall() {
    for inconclusive in [
        "TimeoutBeforeResponse",
        "TimeoutBeforeBody",
        "TimeoutAfterBody",
        "ConnectFailure",
        "TlsFailure",
        "route_unavailable",
        "Inconclusive",
    ] {
        assert!(
            !is_freeze_classification(inconclusive),
            "{inconclusive} must not trigger fake-SNI candidate search"
        );
    }
    for confirmed_signal in [
        "PossibleFreeze",
        "PossibleBoundaryStall",
        "PossibleUploadStall",
    ] {
        assert!(is_freeze_classification(confirmed_signal));
    }
}

#[test]
fn sni_template_accepts_only_normalized_public_hostnames() {
    assert_eq!(
        instantiate_template("--lua-desync=hostfakesplit:repeats=4:host=NAME", "ya.ru").unwrap(),
        vec!["--lua-desync=hostfakesplit:repeats=4:host=ya.ru"]
    );
    assert!(instantiate_template("--lua-desync=hostfakesplit:host=NAME", "bad.name;id").is_err());
}

#[test]
fn strategy_candidate_needs_three_consecutive_successes() {
    assert!(candidate_confirmed(&[false, true, true, true]));
    assert!(!candidate_confirmed(&[true, true, false, true, true]));
    assert!(!candidate_confirmed(&[true, true]));
}

#[test]
fn candidate_success_uses_the_current_route_probe_without_repeating_baseline() {
    assert!(candidate_route_succeeded(Some(true)));
    assert!(!candidate_route_succeeded(Some(false)));
    assert!(!candidate_route_succeeded(None));
}

#[test]
fn fake_sni_candidates_follow_template_then_name_order_and_installed_catalog() {
    use std::collections::HashSet;

    let functions = HashSet::from(["hostfakesplit".to_owned(), "fake".to_owned()]);
    let candidates = freeze_candidates(&functions, &["fake_default_tls".into()]);
    assert_eq!(candidates.len(), 24);
    assert_eq!(candidates[0].0, "ya.ru");
    assert!(candidates[0].1.contains("hostfakesplit:repeats=4"));
    assert!(candidates[8].1.contains("tcp_ts_up"));
    assert!(candidates[16].1.contains("fake_default_tls"));

    let only_fake = HashSet::from(["fake".to_owned()]);
    let filtered = freeze_candidates(&only_fake, &["fake_default_tls".into()]);
    assert_eq!(filtered.len(), 8);
    assert!(
        filtered
            .iter()
            .all(|(_, step)| step.contains("--lua-desync=fake:"))
    );
}

#[test]
fn confirmed_freeze_scan_results_are_reusable_candidates_but_inconclusive_rows_are_not() {
    let mut confirmed = sample_freeze_result("Hetzner", true);
    confirmed.found_strategy = Some(FreezeStrategyMatch {
        sni: "ya.ru".into(),
        step: "--lua-desync=hostfakesplit:host=ya.ru:repeats=4".into(),
        attempts: 3,
        successes: 3,
        average_ms: Some(120),
        reused_from: None,
    });
    let mut unconfirmed = sample_freeze_result("OVH", true);
    unconfirmed.found_strategy = Some(FreezeStrategyMatch {
        sni: "google.com".into(),
        step: "--lua-desync=fake:blob=fake_default_tls".into(),
        attempts: 3,
        successes: 2,
        average_ms: Some(180),
        reused_from: None,
    });
    let not_affected = sample_freeze_result("Cloudflare", false);
    let candidates = confirmed_strategy_candidates(&[confirmed, unconfirmed, not_affected]);
    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].name, "Обрыв на 16 КБ · ya.ru · Hetzner");
    assert_eq!(candidates[0].source, "Найдено проверкой сетей 16 КБ");
}

fn sample_freeze_result(label: &str, freeze_detected: bool) -> FreezeNetworkResult {
    use std::net::Ipv4Addr;
    FreezeNetworkResult {
        label: label.into(),
        host: format!("{label}.example"),
        path: "/large.bin".into(),
        target_kind: FreezeTargetKind::PinnedAddress,
        address: Some(Ipv4Addr::new(203, 0, 113, 1)),
        asn_hint: None,
        asn: AsnVerification::Unavailable,
        direct: nfqws2_ui::freeze_diagnostic::FreezeMeasurement {
            classification: "PossibleFreeze".into(),
            ok: false,
            bytes_received: Some(18_000),
            replies: None,
            elapsed_ms: 4_000,
            reason: "повторяемый тайм-аут после начала ответа".into(),
        },
        current: nfqws2_ui::freeze_diagnostic::FreezeMeasurement {
            classification: "PossibleFreeze".into(),
            ok: false,
            bytes_received: Some(18_000),
            replies: None,
            elapsed_ms: 4_000,
            reason: "повторяемый тайм-аут после начала ответа".into(),
        },
        freeze_detected,
        tried_candidates: 3,
        found_strategy: None,
        candidate_attempts: Vec::new(),
    }
}

#[test]
fn pinned_catalog_asn_is_only_a_hint_until_current_ripe_data_matches() {
    let current = br#"{"data":{"asns":[24940]}}"#;
    assert_eq!(
        verify_asn_hint(Some(24940), current),
        AsnVerification::Verified { asn: 24940 }
    );
    assert_eq!(
        verify_asn_hint(Some(24940), br#"{"data":{"asns":[6805]}}"#),
        AsnVerification::Changed {
            expected: 24940,
            actual: 6805
        }
    );
    assert_eq!(
        verify_asn_hint(Some(24940), br#"{"data":{"asns":[]}}"#),
        AsnVerification::Unavailable
    );
    assert_eq!(
        verify_asn_hint(Some(24940), b"not-json"),
        AsnVerification::Unavailable
    );
}

fn attempt(bytes: u64, error: Option<&str>, status: u16) -> TransferAttempt {
    TransferAttempt {
        bytes_received: bytes,
        sample_bytes: 65_536,
        http_status: Some(status),
        elapsed_ms: 4_000,
        first_byte_ms: None,
        transport_error: error.map(str::to_string),
        upload_stalled_after_tls: false,
    }
}
