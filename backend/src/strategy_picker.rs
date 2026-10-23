//! Safe, bounded strategy candidate construction based on the installed nfqws2 Lua catalog.
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StrategyProtocol {
    Http,
    Https,
    Quic,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StrategyMode {
    Quick,
    Extended,
    Full,
    Custom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyCandidate {
    pub name: String,
    pub source: String,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyPlan {
    pub protocol: StrategyProtocol,
    pub candidates: Vec<StrategyCandidate>,
}

/// Upper bound accommodates the blockcheck2 full catalog plus local candidates.
pub const MAX_CANDIDATES: usize = 4096;
const MAX_STEPS: usize = 8;
const MAX_STEP_BYTES: usize = 512;

fn parse_step(step: &str) -> Result<(&str, Option<&str>), String> {
    let value = step
        .strip_prefix("--lua-desync=")
        .ok_or_else(|| "Разрешены только параметры --lua-desync".to_string())?;
    if value.is_empty()
        || value.len() > MAX_STEP_BYTES
        || value.bytes().any(|byte| {
            byte.is_ascii_control()
                || matches!(
                    byte,
                    b' ' | b'\'' | b'"' | b'`' | b'$' | b'\\' | b';' | b'&' | b'|'
                )
        })
    {
        return Err("Параметр стратегии содержит недопустимые символы или превышает лимит".into());
    }
    let (function, arguments) = value
        .split_once(':')
        .map_or((value, None), |(f, a)| (f, Some(a)));
    if function.is_empty()
        || function.len() > 48
        || !function.bytes().enumerate().all(|(index, byte)| {
            byte.is_ascii_lowercase() || byte == b'_' || (index > 0 && byte.is_ascii_digit())
        })
        || arguments.is_some_and(|args| args.is_empty() || args.len() > MAX_STEP_BYTES)
    {
        return Err("Некорректный формат параметра --lua-desync".into());
    }
    Ok((function, arguments))
}

pub fn validate_candidate(
    steps: &[String],
    available_functions: &HashSet<String>,
    available_blobs: &[String],
) -> Result<(), String> {
    if steps.is_empty() || steps.len() > MAX_STEPS {
        return Err(format!(
            "Стратегия должна содержать от 1 до {MAX_STEPS} шагов"
        ));
    }
    for step in steps {
        let (function, args) = parse_step(step)?;
        if !available_functions.contains(function) {
            return Err(format!(
                "В установленном nfqws2 отсутствует Lua-функция «{function}»"
            ));
        }
        if let Some(args) = args {
            for field in args.split(':') {
                if let Some(blob) = field.strip_prefix("blob=")
                    && !blob.starts_with("0x")
                    && !available_blobs.iter().any(|known| known == blob)
                {
                    return Err(format!(
                        "Стратегия ссылается на неустановленный blob «{blob}»"
                    ));
                }
            }
        }
    }
    Ok(())
}

fn candidates_for(
    protocol: StrategyProtocol,
    mode: StrategyMode,
) -> Vec<(&'static str, Vec<&'static str>)> {
    if mode == StrategyMode::Custom {
        return Vec::new();
    }
    match protocol {
        StrategyProtocol::Https => {
            let mut candidates = vec![
                (
                    "multisplit pos=1,midsld",
                    vec!["--lua-desync=multisplit:pos=1,midsld"],
                ),
                ("multisplit pos=2", vec!["--lua-desync=multisplit:pos=2"]),
                (
                    "multidisorder pos=1,midsld",
                    vec!["--lua-desync=multidisorder:pos=1,midsld"],
                ),
                (
                    "multisplit с seqovl",
                    vec!["--lua-desync=multisplit:pos=1:seqovl=1:seqovl_pattern=tls_clienthello"],
                ),
                (
                    "fake tcp_md5 + multisplit",
                    vec![
                        "--lua-desync=fake:blob=tls_clienthello:tcp_md5",
                        "--lua-desync=multisplit:pos=1,midsld",
                    ],
                ),
                (
                    "fake badsum + multisplit",
                    vec![
                        "--lua-desync=fake:blob=tls_clienthello:badsum",
                        "--lua-desync=multisplit:pos=1,midsld",
                    ],
                ),
                (
                    "fake tcp_seq + multidisorder",
                    vec![
                        "--lua-desync=fake:blob=tls_clienthello:tcp_seq=-10000",
                        "--lua-desync=multidisorder:pos=1,midsld",
                    ],
                ),
                (
                    "fake autottl + multisplit",
                    vec![
                        "--lua-desync=fake:blob=tls_clienthello:tls_mod=rnd,dupsid,sni=www.google.com:ip_autottl=-2,3-20",
                        "--lua-desync=multisplit:pos=1,midsld",
                    ],
                ),
                (
                    "fake x6 tcp_md5 + fakedsplit",
                    vec![
                        "--lua-desync=fake:blob=tls_clienthello:repeats=6:tcp_md5",
                        "--lua-desync=fakedsplit:pos=1:tcp_md5",
                    ],
                ),
                (
                    "fakedsplit tcp_md5",
                    vec!["--lua-desync=fakedsplit:pos=midsld:tcp_md5"],
                ),
                (
                    "fakeddisorder tcp_md5",
                    vec!["--lua-desync=fakeddisorder:pos=midsld:tcp_md5"],
                ),
                (
                    "hostfakesplit google.com",
                    vec!["--lua-desync=hostfakesplit:host=google.com:tcp_md5:tcp_ts_up"],
                ),
            ];
            if matches!(mode, StrategyMode::Extended | StrategyMode::Full) {
                candidates.extend([
                    (
                        "fake default TLS tcp_md5",
                        vec!["--lua-desync=fake:blob=fake_default_tls:tcp_md5"],
                    ),
                    (
                        "fake default TLS badsum",
                        vec!["--lua-desync=fake:blob=fake_default_tls:badsum"],
                    ),
                    (
                        "fake default TLS autottl",
                        vec!["--lua-desync=fake:blob=fake_default_tls:ip_autottl=-1,3-20"],
                    ),
                    (
                        "multisplit SNI +4",
                        vec!["--lua-desync=multisplit:pos=sniext+4"],
                    ),
                    (
                        "hostfakesplit SNI",
                        vec!["--lua-desync=hostfakesplit:host=google.com:tcp_md5"],
                    ),
                ]);
            }
            candidates
        }
        StrategyProtocol::Http => {
            let mut candidates = vec![
                ("http_methodeol", vec!["--lua-desync=http_methodeol"]),
                ("http_hostcase", vec!["--lua-desync=http_hostcase"]),
                (
                    "multisplit pos=method+2",
                    vec!["--lua-desync=multisplit:pos=method+2"],
                ),
                (
                    "fake badsum + multisplit",
                    vec![
                        "--lua-desync=fake:blob=0x00000000:badsum",
                        "--lua-desync=multisplit:pos=method+2",
                    ],
                ),
            ];
            if matches!(mode, StrategyMode::Extended | StrategyMode::Full) {
                candidates.extend([
                    (
                        "http multisplit midsld",
                        vec!["--lua-desync=multisplit:pos=midsld"],
                    ),
                    (
                        "http multidisorder",
                        vec!["--lua-desync=multidisorder:pos=method+2"],
                    ),
                    (
                        "fake default HTTP tcp_md5",
                        vec!["--lua-desync=fake:blob=fake_default_http:tcp_md5"],
                    ),
                    (
                        "fake zero HTTP badsum",
                        vec!["--lua-desync=fake:blob=0x00000000:badsum"],
                    ),
                ]);
            }
            candidates
        }
        StrategyProtocol::Quic => {
            let mut candidates = vec![
                (
                    "fake QUIC ×2",
                    vec!["--lua-desync=fake:blob=fake_default_quic:repeats=2"],
                ),
                (
                    "fake QUIC ×4",
                    vec!["--lua-desync=fake:blob=fake_default_quic:repeats=4"],
                ),
                (
                    "fake QUIC ×6",
                    vec!["--lua-desync=fake:blob=fake_default_quic:repeats=6"],
                ),
                (
                    "fake QUIC ×11",
                    vec!["--lua-desync=fake:blob=fake_default_quic:repeats=11"],
                ),
                (
                    "fake QUIC ×16",
                    vec!["--lua-desync=fake:blob=fake_default_quic:repeats=16"],
                ),
                ("udplen +2", vec!["--lua-desync=udplen:increment=2"]),
                ("udplen +8", vec!["--lua-desync=udplen:increment=8"]),
            ];
            if matches!(mode, StrategyMode::Extended | StrategyMode::Full) {
                candidates.push((
                    "fake QUIC ×6 + udplen +2",
                    vec![
                        "--lua-desync=fake:blob=fake_default_quic:repeats=6",
                        "--lua-desync=udplen:increment=2",
                    ],
                ));
            }
            candidates
        }
    }
}

const CATALOG_FOOLING: [&str; 8] = [
    "tcp_md5",
    "badsum",
    "tcp_seq=-3000",
    "tcp_seq=1000000",
    "tcp_ack=-66000:tcp_ts_up",
    "tcp_ts=-1000",
    "tcp_flags_unset=ACK",
    "tcp_flags_set=SYN",
];
const CATALOG_TLS_SPLITS: [&str; 9] = [
    "2",
    "1",
    "sniext+1",
    "sniext+4",
    "host+1",
    "midsld",
    "1,midsld",
    "1,midsld,1220",
    "1,sniext+1,host+1,midsld-2,midsld,midsld+2,endhost-1",
];
const CATALOG_HTTP_SPLITS: [&str; 3] = ["method+2", "midsld", "method+2,midsld"];

fn catalog_candidates(protocol: StrategyProtocol, full: bool) -> Vec<Vec<String>> {
    let http = protocol == StrategyProtocol::Http;
    let splits: &[&str] = if http {
        &CATALOG_HTTP_SPLITS
    } else {
        &CATALOG_TLS_SPLITS
    };
    let fake_blob = if http {
        "fake_default_http"
    } else {
        "fake_default_tls"
    };
    let mut fooling = CATALOG_FOOLING
        .iter()
        .map(|value| (*value).to_owned())
        .collect::<Vec<_>>();
    fooling.extend((1..=5).map(|ttl| format!("ip_autottl=-{ttl},3-20")));
    if full {
        fooling.extend((1..=12).map(|ttl| format!("ip_ttl={ttl}")));
    }
    let mut candidates = Vec::new();
    let mut seen = HashSet::new();
    let mut add = |steps: Vec<String>| {
        if seen.insert(steps.clone()) {
            candidates.push(steps);
        }
    };
    let desync = |function: &str, params: &str| format!("--lua-desync={function}:{params}");
    let fakes = |fooling: &str| {
        let mut variants = vec![
            vec![desync("fake", &format!("blob={fake_blob}:{fooling}"))],
            vec![desync("fake", &format!("blob=0x00000000:{fooling}"))],
        ];
        if !http {
            variants.push(vec![
                desync("fake", &format!("blob=0x00000000:{fooling}")),
                desync(
                    "fake",
                    &format!("blob={fake_blob}:{fooling}:tls_mod=rnd,dupsid"),
                ),
            ]);
            variants.push(vec![desync(
                "multisplit",
                &format!("blob={fake_blob}:{fooling}:pos=2:nodrop"),
            )]);
            variants.push(vec![desync(
                "fake",
                &format!("blob={fake_blob}:{fooling}:tls_mod=rnd,dupsid,padencap"),
            )]);
        }
        variants
    };

    for function in ["multisplit", "multidisorder"] {
        for position in splits {
            add(vec![desync(function, &format!("pos={position}"))]);
        }
    }
    for fool in &fooling {
        for variant in fakes(fool) {
            add(variant);
        }
    }
    for fool in &fooling {
        let middle_options: &[&str] = if full {
            &[
                "",
                "nofake1:",
                "nofake2:",
                "midhost=midsld:",
                "nofake1:midhost=midsld:",
                "nofake2:midhost=midsld:",
            ]
        } else {
            &["", "midhost=midsld:"]
        };
        let disorder_options: &[&str] = if full {
            &["", "disorder_after:"]
        } else {
            &[""]
        };
        for disorder in disorder_options {
            for middle in middle_options {
                add(vec![desync(
                    "hostfakesplit",
                    &format!("{disorder}{middle}{fool}"),
                )]);
            }
        }
    }
    let mixed_splits: &[&str] = if full || http {
        splits
    } else {
        &["2", "midsld", "1,midsld"]
    };
    for function in ["multisplit", "multidisorder"] {
        for position in mixed_splits {
            for fool in &fooling {
                let fake_variants = fakes(fool);
                let selected = if full { fake_variants.len() } else { 2 };
                for variant in fake_variants.into_iter().take(selected) {
                    let mut steps = variant;
                    steps.push(desync(function, &format!("pos={position}")));
                    add(steps);
                }
            }
        }
    }

    if full {
        let split_positions: &[&str] = if http {
            splits
        } else {
            &[
                "2",
                "1",
                "sniext+1",
                "sniext+4",
                "host+1",
                "midsld",
                "1,midsld",
                "1,sniext+1,host+1,midsld-2,midsld,midsld+2,endhost-1",
            ]
        };
        for function in ["fakedsplit", "fakeddisorder"] {
            for position in split_positions {
                for fool in &fooling {
                    add(vec![desync(function, &format!("pos={position}:{fool}"))]);
                }
            }
        }
        add(vec![
            desync("tcpseg", "pos=0,-1:seqovl=1"),
            "--lua-desync=drop".into(),
        ]);
        let tcpseg_positions: &[&str] = if http {
            &["0,method+2", "0,midsld"]
        } else {
            &["0,1", "0,midsld"]
        };
        for repeats in [1, 20, 100, 260] {
            for position in tcpseg_positions {
                add(vec![desync(
                    "tcpseg",
                    &format!("pos={position}:ip_id=rnd:repeats={repeats}"),
                )]);
            }
        }
    }

    let seq_positions: &[&str] = if http {
        &["method+2", "method+2,midsld"]
    } else {
        &["10", "10,sniext+1", "10,sniext+4", "10,midsld"]
    };
    for position in seq_positions {
        add(vec![desync(
            "multisplit",
            &format!("pos={position}:seqovl=1"),
        )]);
    }
    let disorder_pairs: &[(&str, &str)] = if http {
        &[
            ("method+1", "method+2"),
            ("midsld-1", "midsld"),
            ("method+1", "method+2,midsld"),
        ]
    } else {
        &[
            ("1", "2"),
            ("sniext", "sniext+1"),
            ("sniext+3", "sniext+4"),
            ("midsld-1", "midsld"),
            ("1", "2,midsld"),
        ]
    };
    for (overlap, position) in disorder_pairs {
        add(vec![desync(
            "multidisorder",
            &format!("pos={position}:seqovl={overlap}"),
        )]);
    }
    candidates
}

pub fn build_plan(
    protocol: StrategyProtocol,
    mode: StrategyMode,
    configured_steps: &[Vec<String>],
    available_functions: &HashSet<String>,
    available_blobs: &[String],
) -> StrategyPlan {
    let mut candidates = Vec::new();
    let mut seen = HashSet::<Vec<String>>::new();
    for steps in configured_steps.iter().take(MAX_CANDIDATES) {
        if validate_candidate(steps, available_functions, available_blobs).is_ok()
            && seen.insert(steps.clone())
        {
            candidates.push(StrategyCandidate {
                name: steps
                    .iter()
                    .filter_map(|step| parse_step(step).ok().map(|(function, _)| function))
                    .collect::<Vec<_>>()
                    .join(" + "),
                source: "Текущий профиль".into(),
                steps: steps.clone(),
            });
        }
    }
    for (name, raw_steps) in candidates_for(protocol, mode)
        .into_iter()
        .take(MAX_CANDIDATES.saturating_sub(candidates.len()))
    {
        let steps = raw_steps
            .into_iter()
            .map(str::to_string)
            .collect::<Vec<_>>();
        if validate_candidate(&steps, available_functions, available_blobs).is_ok()
            && seen.insert(steps.clone())
        {
            candidates.push(StrategyCandidate {
                name: name.into(),
                source: "Стандартный набор nfqws2-ui".into(),
                steps,
            });
        }
    }
    if matches!(mode, StrategyMode::Extended | StrategyMode::Full)
        && protocol != StrategyProtocol::Quic
    {
        for steps in catalog_candidates(protocol, mode == StrategyMode::Full) {
            if candidates.len() >= MAX_CANDIDATES {
                break;
            }
            if validate_candidate(&steps, available_functions, available_blobs).is_ok()
                && seen.insert(steps.clone())
            {
                candidates.push(StrategyCandidate {
                    name: steps
                        .iter()
                        .filter_map(|step| parse_step(step).ok().map(|(function, _)| function))
                        .collect::<Vec<_>>()
                        .join(" + "),
                    source: if mode == StrategyMode::Full {
                        "Каталог zapret2 (полный)".into()
                    } else {
                        "Каталог zapret2 (расширенный)".into()
                    },
                    steps,
                });
            }
        }
    }
    StrategyPlan {
        protocol,
        candidates,
    }
}

/// Place confirmed candidates from prior diagnostics after active-profile and
/// standard candidates, but before the larger generated catalog. They remain
/// hints: `run_candidates` sends every one through a fresh real probe.
pub fn insert_reusable_candidates(
    plan: &mut StrategyPlan,
    reusable: impl IntoIterator<Item = StrategyCandidate>,
    available_functions: &HashSet<String>,
    available_blobs: &[String],
) {
    let mut accepted = Vec::new();
    let mut accepted_steps = HashSet::new();
    for candidate in reusable {
        if validate_candidate(&candidate.steps, available_functions, available_blobs).is_err()
            || accepted_steps.contains(&candidate.steps)
        {
            continue;
        }
        let catalog_duplicate = plan.candidates.iter().position(|existing| {
            existing.source.starts_with("Каталог zapret2") && existing.steps == candidate.steps
        });
        let catalog_start = plan
            .candidates
            .iter()
            .position(|existing| existing.source.starts_with("Каталог zapret2"))
            .unwrap_or(plan.candidates.len());
        if plan.candidates[..catalog_start]
            .iter()
            .any(|existing| existing.steps == candidate.steps)
        {
            continue;
        }
        if let Some(index) = catalog_duplicate {
            plan.candidates.remove(index);
        } else if plan.candidates.len() + accepted.len() >= MAX_CANDIDATES {
            break;
        }
        accepted_steps.insert(candidate.steps.clone());
        accepted.push(candidate);
    }
    let insertion = plan
        .candidates
        .iter()
        .position(|candidate| candidate.source.starts_with("Каталог zapret2"))
        .unwrap_or(plan.candidates.len());
    plan.candidates.splice(insertion..insertion, accepted);
}

/// Build a single candidate from user-entered nfqws2 Lua desync arguments.
/// The candidate is validated against the active installation before it can
/// reach the isolated runner; user input is never interpreted by a shell.
pub fn build_custom_plan(
    protocol: StrategyProtocol,
    steps: &[String],
    available_functions: &HashSet<String>,
    available_blobs: &[String],
) -> Result<StrategyPlan, String> {
    validate_candidate(steps, available_functions, available_blobs)?;
    let name = steps
        .iter()
        .filter_map(|step| parse_step(step).ok().map(|(function, _)| function))
        .collect::<Vec<_>>()
        .join(" + ");
    Ok(StrategyPlan {
        protocol,
        candidates: vec![StrategyCandidate {
            name,
            source: "Пользовательская стратегия".into(),
            steps: steps.to_vec(),
        }],
    })
}

/// Build a one-profile test argv from the active process. Global options/Lua
/// initializers are retained; every live profile is discarded so the tested
/// candidate cannot silently coexist with the primary strategies.
pub fn candidate_argv(
    active_args: &[String],
    protocol: StrategyProtocol,
    steps: &[String],
    available_functions: &HashSet<String>,
    available_blobs: &[String],
) -> Result<Vec<String>, String> {
    if active_args.len() < 2 || active_args[0].is_empty() {
        return Err("Не найден полный argv активного nfqws2".into());
    }
    validate_candidate(steps, available_functions, available_blobs)?;
    let mut args = Vec::new();
    let mut blob_declarations = Vec::new();
    let mut seen_blob_declarations = HashSet::new();
    let mut blob_index = 1;
    while blob_index < active_args.len() {
        let argument = &active_args[blob_index];
        if argument.starts_with("--blob=") {
            if seen_blob_declarations.insert(argument.clone()) {
                blob_declarations.push(argument.clone());
            }
        } else if argument == "--blob"
            && let Some(value) = active_args.get(blob_index + 1)
        {
            let key = format!("--blob\0{value}");
            if seen_blob_declarations.insert(key) {
                blob_declarations.push(argument.clone());
                blob_declarations.push(value.clone());
            }
            blob_index += 1;
        }
        blob_index += 1;
    }
    let mut skip_value = false;
    for arg in active_args.iter().skip(1) {
        if skip_value {
            skip_value = false;
            continue;
        }
        if ["--qnum", "--user", "--uid", "--pidfile"].contains(&arg.as_str()) {
            skip_value = true;
            continue;
        }
        if ["--daemon", "--daemonize", "--debug"].contains(&arg.as_str())
            || ["--qnum=", "--user=", "--uid=", "--pidfile=", "--debug="]
                .iter()
                .any(|prefix| arg.starts_with(prefix))
        {
            continue;
        }
        if arg == "--blob" {
            skip_value = true;
            continue;
        }
        if arg.starts_with("--blob=") {
            continue;
        }
        let begins_profile = arg == "--new"
            || [
                "--filter-tcp",
                "--filter-udp",
                "--filter-l7",
                "--filter-mark",
                "--payload",
                "--lua-desync",
                "--hostlist",
                "--ipset",
            ]
            .iter()
            .any(|prefix| arg == *prefix || arg.starts_with(&format!("{prefix}=")));
        if begins_profile {
            break;
        }
        args.push(arg.clone());
    }
    args.extend(blob_declarations);
    let (transport, layer, payload) = match protocol {
        StrategyProtocol::Http => ("--filter-tcp=80", "--filter-l7=http", "--payload=http_req"),
        StrategyProtocol::Https => (
            "--filter-tcp=443",
            "--filter-l7=tls",
            "--payload=tls_client_hello",
        ),
        StrategyProtocol::Quic => (
            "--filter-udp=443",
            "--filter-l7=quic",
            "--payload=quic_initial",
        ),
    };
    args.extend([transport.into(), layer.into(), payload.into()]);
    args.extend(steps.iter().cloned());
    Ok(args)
}

/// Construct one domain-scoped profile for an explicitly selected, locally
/// validated HTTP/HTTPS candidate. QUIC application needs its own real runner.
pub fn build_domain_profile(
    host: &str,
    protocol: StrategyProtocol,
    steps: &[String],
    available_functions: &HashSet<String>,
    available_blobs: &[String],
) -> Result<String, String> {
    let normalized = crate::site_diagnostics::normalize_host(host)?;
    if normalized != host {
        return Err("Имя домена должно быть нормализовано перед подготовкой профиля".into());
    }
    validate_candidate(steps, available_functions, available_blobs)?;
    let (transport, layer, payload) = match protocol {
        StrategyProtocol::Http => ("--filter-tcp=80", "--filter-l7=http", "--payload=http_req"),
        StrategyProtocol::Https => (
            "--filter-tcp=443",
            "--filter-l7=tls",
            "--payload=tls_client_hello",
        ),
        StrategyProtocol::Quic => {
            return Err("Применение QUIC-стратегии пока не поддерживается".into());
        }
    };
    let mut profile = vec![
        "--new".to_owned(),
        transport.to_owned(),
        layer.to_owned(),
        format!("--hostlist-domains={normalized}"),
        payload.to_owned(),
    ];
    profile.extend(steps.iter().cloned());
    Ok(profile.join(" "))
}

#[cfg(test)]
mod application_tests {
    use super::{StrategyProtocol, build_custom_plan, build_domain_profile};
    use std::collections::HashSet;

    #[test]
    fn builds_domain_scoped_http_and_https_profiles_from_validated_steps() {
        let functions = HashSet::from(["multisplit".to_string()]);
        let https = build_domain_profile(
            "youtube.com",
            StrategyProtocol::Https,
            &["--lua-desync=multisplit:pos=1,midsld".into()],
            &functions,
            &[],
        )
        .unwrap();
        assert_eq!(
            https,
            "--new --filter-tcp=443 --filter-l7=tls --hostlist-domains=youtube.com --payload=tls_client_hello --lua-desync=multisplit:pos=1,midsld"
        );

        let http = build_domain_profile(
            "example.org",
            StrategyProtocol::Http,
            &["--lua-desync=multisplit:pos=method+2".into()],
            &functions,
            &[],
        )
        .unwrap();
        assert_eq!(
            http,
            "--new --filter-tcp=80 --filter-l7=http --hostlist-domains=example.org --payload=http_req --lua-desync=multisplit:pos=method+2"
        );
    }

    #[test]
    fn refuses_unsafe_hosts_unknown_lua_and_quic_application_profiles() {
        let functions = HashSet::from(["multisplit".to_string()]);
        assert!(
            build_domain_profile(
                "example.org;id",
                StrategyProtocol::Https,
                &["--lua-desync=multisplit:pos=1".into()],
                &functions,
                &[],
            )
            .is_err()
        );
        assert!(
            build_domain_profile(
                "example.org",
                StrategyProtocol::Https,
                &["--lua-desync=unknown:pos=1".into()],
                &functions,
                &[],
            )
            .is_err()
        );
        assert!(
            build_domain_profile(
                "example.org",
                StrategyProtocol::Quic,
                &["--lua-desync=multisplit:pos=1".into()],
                &functions,
                &[],
            )
            .is_err()
        );
    }

    #[test]
    fn accepts_a_custom_strategy_only_when_every_step_is_available() {
        let functions = HashSet::from(["multisplit".to_string()]);
        let plan = build_custom_plan(
            StrategyProtocol::Https,
            &["--lua-desync=multisplit:pos=1".into()],
            &functions,
            &[],
        )
        .unwrap();
        assert_eq!(plan.candidates.len(), 1);
        assert_eq!(plan.candidates[0].source, "Пользовательская стратегия");
        assert!(
            build_custom_plan(
                StrategyProtocol::Https,
                &["--lua-desync=not_installed".into()],
                &functions,
                &[],
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod repeat_tests {
    use super::{
        StrategyCandidate, StrategyPlan, StrategyProbeResult, StrategyProtocol, run_candidates,
    };
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    #[tokio::test]
    async fn confirms_a_candidate_only_after_the_requested_number_of_real_probe_results() {
        let calls = Arc::new(AtomicUsize::new(0));
        let runner_calls = calls.clone();
        let plan = StrategyPlan {
            protocol: StrategyProtocol::Https,
            candidates: vec![StrategyCandidate {
                name: "test candidate".into(),
                source: "test".into(),
                steps: vec!["--lua-desync=multisplit:pos=1".into()],
            }],
        };
        let result = run_candidates(
            plan,
            Arc::new(std::sync::atomic::AtomicBool::new(false)),
            3,
            false,
            move |_, _| {
                runner_calls.fetch_add(1, Ordering::SeqCst);
                async {
                    StrategyProbeResult {
                        baseline_ok: Some(false),
                        candidate_ok: Some(true),
                        detail: "router probe completed".into(),
                    }
                }
            },
            |_, _, _| {},
        )
        .await;

        assert_eq!(calls.load(Ordering::SeqCst), 3);
        assert_eq!(result.results[0].attempts, 3);
        assert_eq!(result.results[0].successes, 3);
        assert!(result.results[0].confirmed);
        assert!(result.message.contains("3 последовательными"));
    }
}

/// Parse only public nfqws2 desync functions from inspected Lua source lines,
/// matching the upstream catalog's `function name(ctx, desync)` signature.
pub fn parse_lua_functions(source: &str) -> HashSet<String> {
    source
        .lines()
        .filter_map(|line| {
            let line = line.trim_end();
            let rest = line.strip_prefix("function ")?;
            let (name, signature) = rest.split_once('(')?;
            if signature != "ctx, desync)"
                || name.is_empty()
                || name.len() > 48
                || !name.bytes().enumerate().all(|(index, byte)| {
                    byte.is_ascii_lowercase()
                        || byte == b'_'
                        || (index > 0 && byte.is_ascii_digit())
                })
            {
                return None;
            }
            Some(name.to_string())
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyProbeResult {
    pub baseline_ok: Option<bool>,
    pub candidate_ok: Option<bool>,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyCandidateResult {
    pub candidate: StrategyCandidate,
    pub attempts: usize,
    pub successes: usize,
    pub confirmed: bool,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrategyRunResult {
    pub state: String,
    pub checked: usize,
    pub total: usize,
    pub results: Vec<StrategyCandidateResult>,
    pub failures: usize,
    pub omitted_results: usize,
    pub message: String,
}

/// Sequentially execute candidates for the requested number of real probes.
/// A direct success means there is no evidence that mitigation is needed;
/// partial/random success is never marked confirmed. The runner is the isolated
/// network probe supplied by the application, not a simulated result.
pub async fn run_candidates<F, Fut, P>(
    plan: StrategyPlan,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    repeats: usize,
    stop_after_first_confirmed: bool,
    runner: F,
    progress: P,
) -> StrategyRunResult
where
    F: FnMut(&StrategyCandidate, usize) -> Fut,
    Fut: std::future::Future<Output = StrategyProbeResult>,
    P: FnMut(usize, usize, &str),
{
    run_candidates_with_compaction(
        plan,
        cancel,
        repeats,
        stop_after_first_confirmed,
        false,
        runner,
        progress,
    )
    .await
}

/// Run the full blockcheck2 catalog without persisting thousands of negative
/// rows. Upstream likewise keeps only catalog strategies that opened the site
/// and aggregates the rest into a failure count.
pub async fn run_candidates_with_compaction<F, Fut, P>(
    plan: StrategyPlan,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    repeats: usize,
    stop_after_first_confirmed: bool,
    compact_failures: bool,
    mut runner: F,
    mut progress: P,
) -> StrategyRunResult
where
    F: FnMut(&StrategyCandidate, usize) -> Fut,
    Fut: std::future::Future<Output = StrategyProbeResult>,
    P: FnMut(usize, usize, &str),
{
    let total = plan.candidates.len();
    let mut result = StrategyRunResult {
        state: "running".into(),
        checked: 0,
        total,
        results: Vec::new(),
        failures: 0,
        omitted_results: 0,
        message: "Подготовка кандидатов".into(),
    };
    for candidate in plan.candidates {
        if cancel.load(std::sync::atomic::Ordering::Acquire) {
            result.state = "cancelled".into();
            result.message = "Подбор отменён".into();
            return result;
        }
        let mut attempts = 0;
        let mut successes = 0;
        let mut detail = String::new();
        for repeat in 1..=repeats {
            progress(result.checked, total, &candidate.name);
            if cancel.load(std::sync::atomic::Ordering::Acquire) {
                result.state = "cancelled".into();
                result.message = "Подбор отменён".into();
                return result;
            }
            let probe = runner(&candidate, repeat).await;
            attempts += 1;
            detail = probe.detail;
            match (probe.baseline_ok, probe.candidate_ok) {
                (Some(true), _) => {
                    result.state = "site_unblocked".into();
                    result.message =
                        "Прямой запрос открыл сайт; подбор стратегии не требуется".into();
                    return result;
                }
                (Some(false), Some(true)) => successes += 1,
                (Some(false), Some(false)) => break,
                _ => {
                    result.state = "failed".into();
                    result.message = "Сетевая проверка не дала достаточного результата".into();
                    result.results.push(StrategyCandidateResult {
                        candidate,
                        attempts,
                        successes,
                        confirmed: false,
                        detail,
                    });
                    return result;
                }
            }
        }
        let confirmed = successes == repeats;
        let candidate_result = StrategyCandidateResult {
            candidate: candidate.clone(),
            attempts,
            successes,
            confirmed,
            detail,
        };
        if compact_failures && successes == 0 {
            result.failures += 1;
        } else if compact_failures && result.results.len() >= 256 {
            result.omitted_results += 1;
        } else {
            result.results.push(candidate_result);
        }
        result.checked += 1;
        if confirmed && stop_after_first_confirmed {
            break;
        }
    }
    result.state = if result.results.iter().any(|item| item.confirmed) {
        "complete"
    } else {
        "no_match"
    }
    .into();
    result.message = if result.state == "complete" {
        if stop_after_first_confirmed {
            format!(
                "Автоматический подбор остановлен: стратегия подтверждена {repeats} последовательными сетевыми проверками"
            )
        } else {
            format!(
                "Найдены варианты, подтверждённые {repeats} последовательными сетевыми проверками"
            )
        }
    } else {
        "Подтверждённая стратегия не найдена".into()
    };
    if compact_failures {
        result.message.push_str(&format!(
            " · без успешного ответа: {} · скрыто строк каталога: {}",
            result.failures, result.omitted_results
        ));
    }
    result
}

fn port_filter_matches(value: &str, wanted: u16) -> bool {
    let mut included = false;
    for item in value.split(',') {
        let (excluded, item) = item
            .strip_prefix('~')
            .map_or((false, item), |rest| (true, rest));
        let matches = item == "*"
            || item.parse::<u16>().is_ok_and(|port| port == wanted)
            || item.split_once('-').is_some_and(|(low, high)| {
                low.parse::<u16>().is_ok_and(|low| {
                    high.parse::<u16>()
                        .is_ok_and(|high| low <= wanted && wanted <= high)
                })
            });
        if matches && !excluded {
            included = true;
        }
        if matches && excluded {
            return false;
        }
    }
    included
}

/// Extract current-profile strategies whose transport, port and payload match
/// the requested application protocol. Circular profile selectors are skipped.
pub fn configured_candidates(
    active_args: &[String],
    protocol: StrategyProtocol,
) -> Vec<Vec<String>> {
    let (transport, port, wanted_l7, wanted_payload) = match protocol {
        StrategyProtocol::Http => ("tcp", 80, "http", "http_req"),
        StrategyProtocol::Https => ("tcp", 443, "tls", "tls_client_hello"),
        StrategyProtocol::Quic => ("udp", 443, "quic", "quic_initial"),
    };
    let mut groups = vec![Vec::<String>::new()];
    for arg in active_args.iter().skip(1).take(16_384) {
        if arg == "--new" || arg.starts_with("--new=") {
            if groups.len() >= 512 {
                break;
            }
            groups.push(Vec::new());
        } else {
            groups.last_mut().unwrap().push(arg.clone());
        }
    }
    groups
        .into_iter()
        .filter_map(|group| {
            let mut port_spec = None;
            let mut l7 = None;
            let mut payload = None;
            let mut steps = Vec::new();
            let mut index = 0;
            while index < group.len() {
                let arg = &group[index];
                for (prefix, target) in [("--filter-tcp", "tcp"), ("--filter-udp", "udp")] {
                    if target != transport {
                        continue;
                    }
                    if let Some(value) = arg.strip_prefix(&format!("{prefix}=")) {
                        port_spec = Some(value.to_string());
                    } else if arg == prefix {
                        port_spec = group.get(index + 1).cloned();
                        index += 1;
                    }
                }
                if let Some(value) = arg.strip_prefix("--filter-l7=") {
                    l7 = Some(value.to_string());
                } else if arg == "--filter-l7" {
                    l7 = group.get(index + 1).cloned();
                    index += 1;
                }
                if let Some(value) = arg.strip_prefix("--payload=") {
                    payload = Some(value.to_string());
                } else if arg == "--payload" {
                    payload = group.get(index + 1).cloned();
                    index += 1;
                }
                if let Some(value) = arg.strip_prefix("--lua-desync=")
                    && value.split([':', '=']).next() != Some("circular")
                {
                    steps.push(arg.clone());
                }
                index += 1;
            }
            let ports_match = port_spec
                .as_deref()
                .is_some_and(|spec| port_filter_matches(spec, port));
            let layer_match = l7.as_deref().is_none_or(|value| {
                value
                    .split(',')
                    .any(|item| item == wanted_l7 || item == "all")
            });
            let payload_match = payload.as_deref().is_none_or(|value| {
                value
                    .split(',')
                    .any(|item| item == wanted_payload || item == "all" || item == "known")
            });
            (ports_match && layer_match && payload_match && !steps.is_empty()).then_some(steps)
        })
        .collect()
}
