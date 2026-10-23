use nfqws2_ui::{
    community_strategies::{CommunityParseContext, parse_candidates},
    strategy_picker::StrategyProtocol,
};
use std::collections::HashSet;

fn catalog() -> HashSet<String> {
    HashSet::from(["multisplit".into(), "hostfakesplit".into()])
}

#[test]
fn community_candidates_are_scoped_ranked_and_compatibility_checked() {
    let body = serde_json::json!({
        "items": [
            {
                "id":"same-site", "asn":64500, "proto":"tls", "provider":"ISP",
                "steps":["--lua-desync=multisplit:pos=1"],
                "targets":[{"host":"example.com", "asn":64501}],
                "reports":5, "fails":0, "last":"2026-10-20"
            },
            {
                "id":"same-net", "asn":64500, "proto":"tls", "provider":"ISP",
                "steps":["--lua-desync=hostfakesplit:host=ya.ru"],
                "targets":[{"host":"other.example", "asn":64501}],
                "reports":4, "fails":0, "last":"2026-10-19"
            },
            {
                "id":"incompatible", "asn":64500, "proto":"tls", "provider":"ISP",
                "steps":["--lua-desync=not-installed:pos=1"], "targets":[],
                "reports":10, "fails":0, "last":"2026-10-22"
            },
            {
                "id":"mostly-failed", "asn":64500, "proto":"tls", "provider":"ISP",
                "steps":["--lua-desync=multisplit:pos=2"], "targets":[],
                "reports":1, "fails":2, "last":"2026-10-22"
            },
            {
                "id":"wrong-asn", "asn":64502, "proto":"tls", "provider":"ISP",
                "steps":["--lua-desync=multisplit:pos=3"], "targets":[],
                "reports":10, "fails":0, "last":"2026-10-22"
            },
            {
                "id":"wrong-proto", "asn":64500, "proto":"http", "provider":"ISP",
                "steps":["--lua-desync=multisplit:pos=4"], "targets":[],
                "reports":10, "fails":0, "last":"2026-10-22"
            }
        ]
    });
    let functions = catalog();
    let candidates = parse_candidates(
        &serde_json::to_vec(&body).unwrap(),
        CommunityParseContext {
            provider_asn: 64500,
            target_asn: Some(64501),
            host: "example.com",
            protocol: StrategyProtocol::Https,
            available_functions: &functions,
            available_blobs: &[],
            today: "2026-10-23",
        },
    )
    .unwrap();
    assert_eq!(candidates.len(), 2);
    assert_eq!(candidates[0].steps, ["--lua-desync=multisplit:pos=1"]);
    assert!(candidates[0].source.contains("этот сайт"));
    assert!(candidates[1].source.contains("сеть AS64501"));
}

#[test]
fn community_candidates_cap_at_eight_and_reject_malformed_step_lists() {
    let items = (0..12)
        .map(|index| {
            serde_json::json!({
                "asn":64500, "proto":"tls",
                "steps":[format!("--lua-desync=multisplit:pos={index}")],
                "targets":[], "reports":index + 1, "fails":0, "last":"2026-10-23"
            })
        })
        .collect::<Vec<_>>();
    let body = serde_json::json!({"items":items});
    let functions = catalog();
    let candidates = parse_candidates(
        &serde_json::to_vec(&body).unwrap(),
        CommunityParseContext {
            provider_asn: 64500,
            target_asn: None,
            host: "example.com",
            protocol: StrategyProtocol::Https,
            available_functions: &functions,
            available_blobs: &[],
            today: "2026-10-23",
        },
    )
    .unwrap();
    assert_eq!(candidates.len(), 8);
    assert_eq!(candidates[0].steps, ["--lua-desync=multisplit:pos=11"]);

    let malformed = serde_json::json!({"items":[{
        "asn":64500, "proto":"tls",
        "steps":["--lua-desync=multisplit:pos=1", "rm -rf /"],
        "targets":[], "reports":5, "fails":0, "last":"2026-10-23"
    }]});
    assert!(
        parse_candidates(
            &serde_json::to_vec(&malformed).unwrap(),
            CommunityParseContext {
                provider_asn: 64500,
                target_asn: None,
                host: "example.com",
                protocol: StrategyProtocol::Https,
                available_functions: &functions,
                available_blobs: &[],
                today: "2026-10-23",
            },
        )
        .unwrap()
        .is_empty()
    );
}

#[test]
fn old_or_invalid_report_dates_are_ranked_below_fresh_candidates() {
    let body = serde_json::json!({"items":[
        {"asn":64500,"proto":"tls","steps":["--lua-desync=multisplit:pos=1"],"targets":[],"reports":2,"fails":0,"last":"2026-10-22"},
        {"asn":64500,"proto":"tls","steps":["--lua-desync=multisplit:pos=2"],"targets":[],"reports":20,"fails":0,"last":"2026-07-01"}
    ]});
    let functions = catalog();
    let candidates = parse_candidates(
        &serde_json::to_vec(&body).unwrap(),
        CommunityParseContext {
            provider_asn: 64500,
            target_asn: None,
            host: "example.com",
            protocol: StrategyProtocol::Https,
            available_functions: &functions,
            available_blobs: &[],
            today: "2026-10-23",
        },
    )
    .unwrap();
    assert_eq!(candidates[0].steps, ["--lua-desync=multisplit:pos=1"]);
}

#[test]
fn accepts_a_real_provider_catalog_entry_from_the_upstream_schema() {
    // Sanitized shape sampled from data/AS8402.json in the community repo.
    let body = serde_json::json!({"items":[{
        "id":"564b5223a232",
        "asn":8402,
        "provider":"CORBINA-AS PJSC",
        "proto":"tls",
        "steps":["--lua-desync=multidisorder:pos=1,sniext+1,host+1,midsld-2,midsld,midsld+2,endhost-1"],
        "targets":[{"host":"www.youtube.com","asn":15169}],
        "reports":1,
        "fails":0,
        "first":"2026-10-18",
        "last":"2026-10-18",
        "nfqws2":"1.3.1"
    }]});
    let functions = HashSet::from(["multidisorder".to_owned()]);
    let candidates = parse_candidates(
        &serde_json::to_vec(&body).unwrap(),
        CommunityParseContext {
            provider_asn: 8402,
            target_asn: Some(15169),
            host: "youtube.com",
            protocol: StrategyProtocol::Https,
            available_functions: &functions,
            available_blobs: &[],
            today: "2026-10-23",
        },
    )
    .unwrap();
    assert_eq!(candidates.len(), 1);
    assert!(candidates[0].source.contains("сеть AS15169"));
    assert!(candidates[0].steps[0].contains("endhost-1"));
}

#[test]
fn community_item_without_freshness_date_is_kept_as_a_low_priority_candidate() {
    let body = serde_json::json!({"items":[{
        "asn":64500,
        "proto":"tls",
        "steps":["--lua-desync=multisplit:pos=1"],
        "targets":[],
        "reports":1,
        "fails":0
    }]});
    let functions = catalog();
    let candidates = parse_candidates(
        &serde_json::to_vec(&body).unwrap(),
        CommunityParseContext {
            provider_asn: 64500,
            target_asn: None,
            host: "example.com",
            protocol: StrategyProtocol::Https,
            available_functions: &functions,
            available_blobs: &[],
            today: "2026-10-23",
        },
    )
    .unwrap();
    assert_eq!(candidates.len(), 1);
    assert!(candidates[0].source.contains("давно не подтверждалась"));
}
