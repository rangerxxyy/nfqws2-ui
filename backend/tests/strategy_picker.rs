use nfqws2_ui::strategy_picker::{
    StrategyMode, StrategyProbeResult, StrategyProtocol, build_plan, candidate_argv,
    configured_candidates, parse_lua_functions, run_candidates, validate_candidate,
};
use std::collections::HashSet;

#[test]
fn default_https_plan_is_small_deterministic_and_filters_missing_lua_functions() {
    let available = HashSet::from(["multisplit".to_string(), "multidisorder".to_string()]);
    let first = build_plan(
        StrategyProtocol::Https,
        StrategyMode::Quick,
        &[],
        &available,
        &[],
    );
    let second = build_plan(
        StrategyProtocol::Https,
        StrategyMode::Quick,
        &[],
        &available,
        &[],
    );
    assert_eq!(first, second);
    assert!(first.candidates.len() <= 16);
    assert!(
        first
            .candidates
            .iter()
            .all(|candidate| candidate.steps.iter().all(|step| {
                let function = step
                    .strip_prefix("--lua-desync=")
                    .unwrap()
                    .split(':')
                    .next()
                    .unwrap();
                available.contains(function)
            }))
    );
    assert!(
        first
            .candidates
            .iter()
            .any(|candidate| candidate.steps == ["--lua-desync=multisplit:pos=2"])
    );
}

#[test]
fn standard_candidate_catalog_contains_actual_http_https_and_quic_families() {
    let functions = HashSet::from([
        "multisplit".into(),
        "multidisorder".into(),
        "fake".into(),
        "http_methodeol".into(),
        "http_hostcase".into(),
        "udplen".into(),
        "fakedsplit".into(),
        "fakeddisorder".into(),
        "hostfakesplit".into(),
    ]);
    let https = build_plan(
        StrategyProtocol::Https,
        StrategyMode::Quick,
        &[],
        &functions,
        &["tls_clienthello".into()],
    );
    let http = build_plan(
        StrategyProtocol::Http,
        StrategyMode::Quick,
        &[],
        &functions,
        &[],
    );
    let quic = build_plan(
        StrategyProtocol::Quic,
        StrategyMode::Quick,
        &[],
        &functions,
        &["fake_default_quic".into()],
    );
    assert!(
        https
            .candidates
            .iter()
            .any(|item| item.name == "fake tcp_md5 + multisplit")
    );
    assert!(
        http.candidates
            .iter()
            .any(|item| item.name == "http_hostcase")
    );
    assert!(
        quic.candidates
            .iter()
            .any(|item| item.name == "fake QUIC ×6")
    );
}

#[test]
fn extended_mode_adds_only_candidates_supported_by_the_installed_function_and_blob_catalog() {
    let functions = HashSet::from(["multisplit".into(), "fake".into(), "hostfakesplit".into()]);
    let blobs = ["fake_default_tls".into()];
    let quick = build_plan(
        StrategyProtocol::Https,
        StrategyMode::Quick,
        &[],
        &functions,
        &blobs,
    );
    let extended = build_plan(
        StrategyProtocol::Https,
        StrategyMode::Extended,
        &[],
        &functions,
        &blobs,
    );
    assert!(extended.candidates.len() > quick.candidates.len());
    assert!(
        extended
            .candidates
            .iter()
            .any(|item| item.name == "fake default TLS tcp_md5")
    );
    assert!(
        !extended
            .candidates
            .iter()
            .any(|item| item.name.contains("multidisorder"))
    );
}

#[test]
fn candidate_rejects_unknown_function_unavailable_blob_and_shell_metacharacters() {
    let functions = HashSet::from(["fake".to_string()]);
    let blobs = vec!["fake_default_tls".to_string()];
    assert!(validate_candidate(&["--lua-desync=other:pos=1".into()], &functions, &blobs).is_err());
    assert!(
        validate_candidate(
            &["--lua-desync=fake:blob=missing".into()],
            &functions,
            &blobs
        )
        .is_err()
    );
    assert!(
        validate_candidate(
            &["--lua-desync=fake:blob=fake_default_tls;reboot".into()],
            &functions,
            &blobs
        )
        .is_err()
    );
    assert!(
        validate_candidate(
            &["--lua-desync=fake:blob=fake_default_tls".into()],
            &functions,
            &blobs
        )
        .is_ok()
    );
}

#[test]
fn config_candidates_are_first_and_exact_duplicate_steps_are_removed() {
    let available = HashSet::from(["multisplit".to_string()]);
    let configured = vec![
        vec!["--lua-desync=multisplit:pos=2".into()],
        vec!["--lua-desync=multisplit:pos=2".into()],
    ];
    let plan = build_plan(
        StrategyProtocol::Https,
        StrategyMode::Quick,
        &configured,
        &available,
        &[],
    );
    assert_eq!(plan.candidates[0].source, "Текущий профиль");
    assert_eq!(
        plan.candidates
            .iter()
            .filter(|candidate| candidate.steps == ["--lua-desync=multisplit:pos=2"])
            .count(),
        1
    );
}

#[test]
fn candidate_argv_keeps_only_global_flags_and_adds_one_isolated_protocol_profile() {
    let active = vec![
        "/opt/usr/bin/nfqws2".into(),
        "--qnum=300".into(),
        "--user=nobody".into(),
        "--lua-init=@/opt/share/zapret2/lua/zapret-lib.lua".into(),
        "--filter-tcp=443".into(),
        "--payload=tls_client_hello".into(),
        "--lua-desync=fake:blob=fake_default_tls".into(),
        "--new".into(),
        "--filter-udp=443".into(),
        "--lua-desync=fake:blob=fake_default_quic".into(),
    ];
    let functions = HashSet::from(["multisplit".to_string()]);
    let args = candidate_argv(
        &active,
        StrategyProtocol::Https,
        &["--lua-desync=multisplit:pos=2".into()],
        &functions,
        &[],
    )
    .unwrap();
    assert!(args.contains(&"--lua-init=@/opt/share/zapret2/lua/zapret-lib.lua".into()));
    assert!(args.contains(&"--filter-tcp=443".into()));
    assert!(args.contains(&"--filter-l7=tls".into()));
    assert!(
        !args
            .iter()
            .any(|arg| arg.contains("fake_default_quic") || arg.contains("fake_default_tls"))
    );
    assert_eq!(args.iter().filter(|arg| *arg == "--new").count(), 0);
}

#[test]
fn candidate_argv_preserves_blob_declarations_that_were_after_a_profile_boundary() {
    let functions = HashSet::from(["multisplit".to_string(), "fake".to_string()]);
    let args = vec![
        "nfqws2".into(),
        "--lua-init=@/opt/share/zapret-lib.lua".into(),
        "--filter-tcp=443".into(),
        "--lua-desync=multisplit:pos=2".into(),
        "--blob=tls_clienthello:@/opt/share/tls.bin".into(),
    ];
    let candidate = vec!["--lua-desync=fake:blob=tls_clienthello:badsum".into()];
    let argv = candidate_argv(
        &args,
        StrategyProtocol::Https,
        &candidate,
        &functions,
        &["tls_clienthello".into()],
    )
    .unwrap();
    assert!(
        argv.iter()
            .any(|arg| arg == "--blob=tls_clienthello:@/opt/share/tls.bin")
    );
    assert!(
        !argv
            .iter()
            .any(|arg| arg.starts_with("--lua-desync=multisplit"))
    );
    assert!(argv.iter().any(|arg| arg == &candidate[0]));
}

#[test]
fn lua_catalog_uses_actual_nfqueue_function_signatures_and_ignores_comments() {
    let functions = parse_lua_functions(
        "-- function fake(ctx, desync)\nfunction multisplit(ctx, desync)\nfunction helper(value)\n",
    );
    assert!(functions.contains("multisplit"));
    assert!(!functions.contains("fake"));
    assert!(!functions.contains("helper"));
}

#[tokio::test]
async fn strategy_is_confirmed_only_after_two_real_probe_successes() {
    let mut plan = build_plan(
        StrategyProtocol::Https,
        StrategyMode::Quick,
        &[],
        &HashSet::from(["multisplit".into()]),
        &[],
    );
    plan.candidates.truncate(1);
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut calls = 0;
    let result = run_candidates(
        plan,
        cancel,
        |_, _| {
            calls += 1;
            std::future::ready(StrategyProbeResult {
                baseline_ok: Some(false),
                candidate_ok: Some(true),
                detail: "HTTP 200".into(),
            })
        },
        |_, _, _| {},
    )
    .await;
    assert_eq!(calls, 2);
    assert_eq!(result.results[0].successes, 2);
    assert!(result.results[0].confirmed);
}

#[tokio::test]
async fn strategy_picker_stops_before_any_candidate_when_direct_site_is_open() {
    let mut plan = build_plan(
        StrategyProtocol::Https,
        StrategyMode::Quick,
        &[],
        &HashSet::from(["multisplit".into()]),
        &[],
    );
    plan.candidates.truncate(1);
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let mut calls = 0;
    let result = run_candidates(
        plan,
        cancel,
        |_, _| {
            calls += 1;
            std::future::ready(StrategyProbeResult {
                baseline_ok: Some(true),
                candidate_ok: Some(true),
                detail: "HTTP 200".into(),
            })
        },
        |_, _, _| {},
    )
    .await;
    assert_eq!(calls, 1);
    assert_eq!(result.state, "site_unblocked");
    assert!(result.results.is_empty());
}

#[tokio::test]
async fn inconclusive_network_probe_ends_as_persistable_failure() {
    let mut plan = build_plan(
        StrategyProtocol::Https,
        StrategyMode::Quick,
        &[],
        &HashSet::from(["multisplit".into()]),
        &[],
    );
    plan.candidates.truncate(1);
    let result = run_candidates(
        plan,
        std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false)),
        |_, _| {
            std::future::ready(StrategyProbeResult {
                baseline_ok: None,
                candidate_ok: None,
                detail: "Тайм-аут до результата".into(),
            })
        },
        |_, _, _| {},
    )
    .await;
    assert_eq!(result.state, "failed");
    assert_eq!(result.results.len(), 1);
    assert!(!result.results[0].confirmed);
}

#[tokio::test]
async fn cancellation_between_repeats_does_not_confirm_a_partial_success() {
    let mut plan = build_plan(
        StrategyProtocol::Https,
        StrategyMode::Quick,
        &[],
        &HashSet::from(["multisplit".into()]),
        &[],
    );
    plan.candidates.truncate(1);
    let cancel = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));
    let runner_cancel = cancel.clone();
    let calls = std::sync::Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let runner_calls = calls.clone();
    let result = run_candidates(
        plan,
        cancel,
        move |_, _| {
            runner_calls.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            runner_cancel.store(true, std::sync::atomic::Ordering::Release);
            std::future::ready(StrategyProbeResult {
                baseline_ok: Some(false),
                candidate_ok: Some(true),
                detail: "HTTP 200".into(),
            })
        },
        |_, _, _| {},
    )
    .await;
    assert_eq!(calls.load(std::sync::atomic::Ordering::Relaxed), 1);
    assert_eq!(result.state, "cancelled");
    assert!(result.results.is_empty());
}

#[test]
fn configured_candidates_are_filtered_by_transport_port_and_payload() {
    let args = vec![
        "nfqws2".into(),
        "--filter-tcp=80,443".into(),
        "--filter-l7=tls,http".into(),
        "--payload=tls_client_hello".into(),
        "--lua-desync=multisplit:pos=2".into(),
        "--new".into(),
        "--filter-udp=443".into(),
        "--filter-l7=quic".into(),
        "--payload=quic_initial".into(),
        "--lua-desync=fake:blob=fake_default_quic".into(),
    ];
    assert_eq!(
        configured_candidates(&args, StrategyProtocol::Https),
        vec![vec!["--lua-desync=multisplit:pos=2"]]
    );
    assert_eq!(
        configured_candidates(&args, StrategyProtocol::Quic),
        vec![vec!["--lua-desync=fake:blob=fake_default_quic"]]
    );
}
