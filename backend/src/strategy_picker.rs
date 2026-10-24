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

const REFINEMENT_FAKE_FUNCTIONS: &[&str] =
    &["fake", "fakedsplit", "fakeddisorder", "hostfakesplit"];
const REFINEMENT_FOOL_KEYS: &[&str] = &[
    "tcp_ts",
    "tcp_md5",
    "badsum",
    "tcp_seq",
    "tcp_ack",
    "ip_ttl",
    "ip_autottl",
];
const CLONE_BLOB: &str = "tls_clone";
const CLONE_STEP: &str =
    "--lua-desync=tls_client_hello_clone:blob=tls_clone:fallback=fake_default_tls";
type RefinementParameters = Vec<(String, Option<String>)>;
type RefinementVariant = (String, String, bool);
type RefinementAxis = (String, Vec<RefinementVariant>);

fn parse_refinement_step(step: &str) -> Option<(String, RefinementParameters)> {
    let value = step.strip_prefix("--lua-desync=")?;
    let mut pieces = value.split(':');
    let function = pieces.next()?.to_owned();
    let parameters = pieces
        .map(|piece| {
            piece.split_once('=').map_or_else(
                || (piece.to_owned(), None),
                |(key, value)| (key.to_owned(), Some(value.to_owned())),
            )
        })
        .collect();
    Some((function, parameters))
}

fn refinement_fake_index(steps: &[String]) -> Option<usize> {
    steps.iter().position(|step| {
        parse_refinement_step(step)
            .is_some_and(|(function, _)| REFINEMENT_FAKE_FUNCTIONS.contains(&function.as_str()))
    })
}

fn build_refinement_step(function: &str, parameters: &[(String, Option<String>)]) -> String {
    let mut step = format!("--lua-desync={function}");
    for (key, value) in parameters {
        step.push(':');
        step.push_str(key);
        if let Some(value) = value {
            step.push('=');
            step.push_str(value);
        }
    }
    step
}

fn parameter(parameters: &mut Vec<(String, Option<String>)>, key: &str, value: Option<&str>) {
    parameters.retain(|(existing, _)| existing != key);
    if let Some(value) = value {
        parameters.push((key.to_owned(), Some(value.to_owned())));
    }
}

fn set_candidate_fake_step(
    candidate: &StrategyCandidate,
    fake_index: usize,
    fake_step: String,
    clone: bool,
) -> Vec<String> {
    let mut steps = candidate.steps.clone();
    steps[fake_index] = fake_step.clone();
    steps.retain(|step| step != CLONE_STEP);
    if clone {
        steps.insert(fake_index, CLONE_STEP.to_owned());
    }
    steps
}

/// Create the bounded, one-axis fake-strategy variants used by blockcheck2's
/// refinement phase. Every result is filtered against the installed Lua/blob
/// inventory before it can reach the live probe runner.
pub fn refinement_variants(
    candidate: &StrategyCandidate,
    protocol: StrategyProtocol,
    available_functions: &HashSet<String>,
    available_blobs: &[String],
) -> Vec<(String, StrategyCandidate)> {
    if protocol == StrategyProtocol::Http {
        return Vec::new();
    }
    let Some((fake_index, (function, parameters))) =
        candidate
            .steps
            .iter()
            .enumerate()
            .find_map(|(index, step)| {
                let parsed = parse_refinement_step(step)?;
                REFINEMENT_FAKE_FUNCTIONS
                    .contains(&parsed.0.as_str())
                    .then_some((index, parsed))
            })
    else {
        return Vec::new();
    };

    let mut axes: Vec<RefinementAxis> = Vec::new();
    let fool_options = [
        ("tcp_ts", vec![("tcp_ts", Some("-600000"))]),
        ("tcp_md5", vec![("tcp_md5", None)]),
        ("badsum", vec![("badsum", None)]),
        ("tcp_seq", vec![("tcp_seq", Some("-10000"))]),
        ("autottl", vec![("ip_autottl", Some("-2,3-20"))]),
        (
            "tcp_ts + tcp_md5",
            vec![("tcp_ts", Some("-600000")), ("tcp_md5", None)],
        ),
    ];
    let fool_variants = fool_options
        .into_iter()
        .map(|(label, params)| {
            let mut refined = parameters
                .iter()
                .filter(|(key, _)| !REFINEMENT_FOOL_KEYS.contains(&key.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            for (key, value) in params {
                parameter(&mut refined, key, value);
            }
            (
                label.to_owned(),
                build_refinement_step(&function, &refined),
                false,
            )
        })
        .collect();
    axes.push(("fool".into(), fool_variants));

    let repeat_variants = [1, 2, 4, 6, 8, 12, 16, 20]
        .into_iter()
        .map(|repeats| {
            let mut refined = parameters.clone();
            refined.retain(|(key, _)| key != "repeats");
            if repeats > 1 {
                refined.push(("repeats".into(), Some(repeats.to_string())));
            }
            (
                repeats.to_string(),
                build_refinement_step(&function, &refined),
                false,
            )
        })
        .collect();
    axes.push(("repeats".into(), repeat_variants));

    let blob = parameters
        .iter()
        .find(|(key, _)| key == "blob")
        .and_then(|(_, value)| value.as_deref());
    if protocol == StrategyProtocol::Https
        && (function == "hostfakesplit"
            || (function == "fake" && blob.is_some_and(|b| b.contains("tls"))))
    {
        let name_variants = ["ya.ru", "www.google.com", ""]
            .into_iter()
            .map(|name| {
                let mut refined = parameters.clone();
                let label = if name.is_empty() {
                    "случайное".to_owned()
                } else {
                    name.to_owned()
                };
                if function == "hostfakesplit" {
                    parameter(&mut refined, "host", (!name.is_empty()).then_some(name));
                } else {
                    parameter(
                        &mut refined,
                        "tls_mod",
                        Some(if name.is_empty() {
                            "rnd,rndsni,dupsid"
                        } else if name == "ya.ru" {
                            "rnd,dupsid,sni=ya.ru"
                        } else {
                            "rnd,dupsid,sni=www.google.com"
                        }),
                    );
                }
                (label, build_refinement_step(&function, &refined), false)
            })
            .collect();
        axes.push(("name".into(), name_variants));
    }

    if function == "fake" && blob.is_some_and(|b| b.contains("tls")) {
        let default_blob = if blob == Some(CLONE_BLOB) {
            "fake_default_tls"
        } else {
            blob.unwrap_or_default()
        };
        let mut blob_variants = Vec::new();
        for (label, target_blob, clone) in [
            ("заготовка", default_blob, false),
            ("копия приветствия", CLONE_BLOB, true),
        ] {
            if clone
                && (!available_functions.contains("tls_client_hello_clone")
                    || !available_blobs
                        .iter()
                        .any(|available| available == CLONE_BLOB)
                    || !available_blobs
                        .iter()
                        .any(|available| available == "fake_default_tls"))
            {
                continue;
            }
            if !available_blobs
                .iter()
                .any(|available| available == target_blob)
            {
                continue;
            }
            let mut refined = parameters.clone();
            parameter(&mut refined, "blob", Some(target_blob));
            let refined_step = build_refinement_step(&function, &refined);
            let clone_flag = clone && function == "fake";
            blob_variants.push((label.to_owned(), refined_step, clone_flag));
        }
        axes.push(("blob".into(), blob_variants));
    }

    let mut out = Vec::new();
    let mut seen = HashSet::new();
    for (axis, variants) in axes {
        for (label, step, clone) in variants {
            let steps = set_candidate_fake_step(candidate, fake_index, step, clone);
            if steps == candidate.steps
                || !seen.insert(steps.clone())
                || validate_candidate(&steps, available_functions, available_blobs).is_err()
            {
                continue;
            }
            out.push((
                format!("{axis}: {label}"),
                StrategyCandidate {
                    name: format!("{} · {axis}: {label}", candidate.name),
                    source: "Уточнение blockcheck2".into(),
                    steps,
                },
            ));
        }
    }
    out
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
        let strategy_id = steps.iter().find_map(|step| {
            step.strip_prefix("--lua-desync=")?
                .split(':')
                .find_map(|part| part.strip_prefix("strategy="))
        });
        let normalized_steps = steps
            .iter()
            .map(|step| strip_circular_strategy_tag(step))
            .collect::<Vec<_>>();
        if validate_candidate(&normalized_steps, available_functions, available_blobs).is_ok()
            && seen.insert(normalized_steps.clone())
        {
            candidates.push(StrategyCandidate {
                name: normalized_steps
                    .iter()
                    .filter_map(|step| parse_step(step).ok().map(|(function, _)| function))
                    .collect::<Vec<_>>()
                    .join(" + "),
                source: strategy_id.map_or_else(
                    || "Текущий профиль".to_owned(),
                    |id| format!("Текущий профиль · стратегия {id}"),
                ),
                steps: normalized_steps,
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

fn strip_circular_strategy_tag(step: &str) -> String {
    let Some(value) = step.strip_prefix("--lua-desync=") else {
        return step.to_owned();
    };
    let (function, Some(arguments)) = value
        .split_once(':')
        .map_or((value, None), |(function, arguments)| {
            (function, Some(arguments))
        })
    else {
        return step.to_owned();
    };
    let arguments = arguments
        .split(':')
        .filter(|argument| !argument.starts_with("strategy="))
        .collect::<Vec<_>>();
    if arguments.is_empty() {
        format!("--lua-desync={function}")
    } else {
        format!("--lua-desync={function}:{}", arguments.join(":"))
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

/// Build one nfqws2 process with a disjoint profile for each isolated mark
/// lane. Callers must bind every lane mark to its own source-port range and
/// serially confirm any batch-positive candidate before presenting it as usable.
pub fn candidate_batch_argv(
    active_args: &[String],
    protocol: StrategyProtocol,
    candidates: &[(Vec<String>, u32, u32)],
    available_functions: &HashSet<String>,
    available_blobs: &[String],
) -> Result<Vec<String>, String> {
    if candidates.is_empty() || candidates.len() > crate::probe_lanes::MAX_BATCH_LANES {
        return Err("Размер пакета стратегий вне безопасного диапазона".into());
    }
    if protocol == StrategyProtocol::Quic {
        return Err("Пакетная проверка QUIC не поддерживается".into());
    }
    let common_mask = candidates[0].2;
    let mut marks = HashSet::new();
    for (_, mark, mask) in candidates {
        if *mask == 0 || *mask != common_mask || *mark == 0 || *mark & !*mask != 0 {
            return Err("Метки профилей должны иметь общую маску и быть внутри неё".into());
        }
        if !marks.insert(*mark) {
            return Err("Метки профилей должны быть уникальными".into());
        }
    }

    let mut profiles = Vec::with_capacity(candidates.len());
    for (steps, mark, mask) in candidates {
        let argv = candidate_argv(
            active_args,
            protocol,
            steps,
            available_functions,
            available_blobs,
        )?;
        let profile_start = argv
            .iter()
            .position(|arg| {
                ["--filter-tcp", "--filter-udp", "--filter-l7", "--payload"]
                    .iter()
                    .any(|prefix| arg == *prefix || arg.starts_with(&format!("{prefix}=")))
            })
            .ok_or_else(|| "Не удалось отделить профиль от глобальных параметров".to_owned())?;
        profiles.push((
            argv[..profile_start].to_vec(),
            argv[profile_start..].to_vec(),
            *mark,
            *mask,
        ));
    }
    let prefix = profiles[0].0.clone();
    if profiles
        .iter()
        .any(|(candidate_prefix, ..)| candidate_prefix != &prefix)
    {
        return Err("Общие параметры тестовых профилей расходятся".into());
    }

    let mut result = prefix;
    for (index, (_, mut profile, mark, mask)) in profiles.into_iter().enumerate() {
        if index > 0 {
            result.push("--new".into());
        }
        result.push(format!("--filter-mark=0x{mark:08x}/0x{mask:08x}"));
        result.append(&mut profile);
    }
    Ok(result)
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
mod batch_argv_tests {
    use super::{StrategyProtocol, candidate_batch_argv};
    use std::collections::HashSet;

    #[test]
    fn builds_distinct_http_profiles_and_keeps_only_shared_global_options() {
        let active = vec![
            "nfqws2".into(),
            "--qnum=300".into(),
            "--blob=tls_clienthello,/opt/blobs/tls.bin".into(),
            "--new".into(),
            "--filter-tcp=80".into(),
            "--filter-l7=http".into(),
            "--lua-desync=old:pos=1".into(),
        ];
        let candidates = vec![
            (vec!["--lua-desync=multisplit:pos=1".into()], 1, 3),
            (vec!["--lua-desync=multisplit:pos=2".into()], 2, 3),
        ];
        let argv = candidate_batch_argv(
            &active,
            StrategyProtocol::Https,
            &candidates,
            &HashSet::from(["multisplit".into()]),
            &[],
        )
        .unwrap();

        assert!(!argv.iter().any(|arg| arg.contains("qnum")));
        assert!(!argv.iter().any(|arg| arg.contains("old:pos")));
        assert_eq!(
            argv.iter().filter(|arg| arg.starts_with("--blob=")).count(),
            1
        );
        assert_eq!(argv.iter().filter(|arg| *arg == "--new").count(), 1);
        assert_eq!(
            argv.iter()
                .filter(|arg| arg.starts_with("--filter-mark="))
                .count(),
            2
        );
        assert!(
            argv.iter()
                .any(|arg| arg == "--filter-mark=0x00000001/0x00000003")
        );
        assert!(
            argv.iter()
                .any(|arg| arg == "--filter-mark=0x00000002/0x00000003")
        );
        assert_eq!(
            argv.iter().filter(|arg| *arg == "--filter-tcp=443").count(),
            2
        );
    }

    #[test]
    fn rejects_duplicate_marks_quic_and_unbounded_batches() {
        let active = vec!["nfqws2".into(), "--qnum=300".into()];
        let steps = vec!["--lua-desync=multisplit:pos=1".into()];
        let duplicate = vec![(steps.clone(), 1, 3), (steps.clone(), 1, 3)];
        assert!(
            candidate_batch_argv(
                &active,
                StrategyProtocol::Https,
                &duplicate,
                &HashSet::from(["multisplit".into()]),
                &[],
            )
            .is_err()
        );
        let too_many = vec![
            (steps.clone(), 1, 7),
            (steps.clone(), 2, 7),
            (steps.clone(), 3, 7),
            (steps.clone(), 4, 7),
        ];
        assert!(
            candidate_batch_argv(
                &active,
                StrategyProtocol::Https,
                &too_many,
                &HashSet::from(["multisplit".into()]),
                &[],
            )
            .is_err()
        );
        assert!(
            candidate_batch_argv(
                &active,
                StrategyProtocol::Quic,
                &[(steps, 1, 1)],
                &HashSet::from(["multisplit".into()]),
                &[],
            )
            .is_err()
        );
    }
}

#[cfg(test)]
mod repeat_tests {
    use super::{
        FullBatchOptions, StrategyBatchProbeResult, StrategyCandidate, StrategyPlan,
        StrategyProbeResult, StrategyProtocol, run_candidates, run_full_candidates_with_batches,
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
            move |_, requested_repeats| {
                runner_calls.fetch_add(1, Ordering::SeqCst);
                async move {
                    (0..requested_repeats)
                        .map(|_| StrategyProbeResult {
                            baseline_ok: Some(false),
                            candidate_ok: Some(true),
                            detail: "router probe completed".into(),
                        })
                        .collect::<Vec<_>>()
                }
            },
            |_, _, _| {},
        )
        .await;

        assert_eq!(calls.load(Ordering::SeqCst), 1);
        assert_eq!(result.results[0].attempts, 3);
        assert_eq!(result.results[0].successes, 3);
        assert!(result.results[0].confirmed);
        assert!(result.message.contains("3 последовательными"));
    }

    #[tokio::test]
    async fn cancellation_during_a_candidate_batch_preserves_completed_repeats_without_confirming()
    {
        let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let runner_cancel = cancel.clone();
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
            cancel,
            3,
            false,
            move |_, _| {
                runner_cancel.store(true, Ordering::Release);
                async {
                    vec![
                        StrategyProbeResult {
                            baseline_ok: Some(false),
                            candidate_ok: Some(true),
                            detail: "first response succeeded".into(),
                        },
                        StrategyProbeResult {
                            baseline_ok: Some(false),
                            candidate_ok: None,
                            detail: "second response was cancelled".into(),
                        },
                    ]
                }
            },
            |_, _, _| {},
        )
        .await;

        assert_eq!(result.state, "cancelled");
        assert_eq!(result.results[0].attempts, 1);
        assert_eq!(result.results[0].successes, 1);
        assert!(!result.results[0].confirmed);
    }

    #[tokio::test]
    async fn full_mode_batches_catalog_candidates_then_serially_confirms_hits_by_latency() {
        let plan = StrategyPlan {
            protocol: StrategyProtocol::Https,
            candidates: vec![
                StrategyCandidate {
                    name: "priority".into(),
                    source: "Текущий профиль".into(),
                    steps: vec!["--lua-desync=split:pos=1".into()],
                },
                StrategyCandidate {
                    name: "slow hit".into(),
                    source: "Каталог zapret2 (полный)".into(),
                    steps: vec!["--lua-desync=split:pos=2".into()],
                },
                StrategyCandidate {
                    name: "fast hit".into(),
                    source: "Каталог zapret2 (полный)".into(),
                    steps: vec!["--lua-desync=split:pos=3".into()],
                },
            ],
        };
        let confirmed_order = Arc::new(std::sync::Mutex::new(Vec::new()));
        let output = run_full_candidates_with_batches(
            plan,
            Arc::new(std::sync::atomic::AtomicBool::new(false)),
            FullBatchOptions {
                repeats: 2,
                stop_after_first_confirmed: false,
                max_batch_size: 3,
            },
            |candidates| async move {
                Ok(candidates
                    .into_iter()
                    .enumerate()
                    .map(|(index, candidate)| {
                        let latency = if candidate.name == "fast hit" { 10 } else { 20 };
                        StrategyBatchProbeResult {
                            candidate_index: index,
                            elapsed_ms: latency,
                            probe: StrategyProbeResult {
                                baseline_ok: Some(false),
                                candidate_ok: Some(true),
                                detail: "batch response".into(),
                            },
                        }
                    })
                    .collect())
            },
            {
                let confirmed_order = confirmed_order.clone();
                move |candidate: &StrategyCandidate, repeats| {
                    confirmed_order.lock().unwrap().push(candidate.name.clone());
                    let outcomes = if candidate.name == "slow hit" {
                        vec![true, false]
                    } else {
                        vec![true; repeats]
                    };
                    async move {
                        outcomes
                            .into_iter()
                            .map(|success| StrategyProbeResult {
                                baseline_ok: Some(false),
                                candidate_ok: Some(success),
                                detail: "serial confirmation".into(),
                            })
                            .collect::<Vec<_>>()
                    }
                }
            },
            |_, _, _| {},
        )
        .await;

        assert_eq!(
            *confirmed_order.lock().unwrap(),
            vec!["priority", "fast hit", "slow hit"]
        );
        assert!(
            output
                .results
                .iter()
                .any(|row| row.candidate.name == "fast hit" && row.confirmed)
        );
        assert!(
            output
                .results
                .iter()
                .any(|row| row.candidate.name == "slow hit" && !row.confirmed)
        );
        assert_eq!(output.state, "complete");
    }

    #[tokio::test]
    async fn cancellation_after_preliminary_batch_hits_never_confirms_them() {
        let cancel = Arc::new(std::sync::atomic::AtomicBool::new(false));
        let batch_cancel = cancel.clone();
        let plan = StrategyPlan {
            protocol: StrategyProtocol::Https,
            candidates: vec![StrategyCandidate {
                name: "catalog hit".into(),
                source: "Каталог zapret2 (полный)".into(),
                steps: vec!["--lua-desync=split:pos=2".into()],
            }],
        };
        let output = run_full_candidates_with_batches(
            plan,
            cancel,
            FullBatchOptions {
                repeats: 3,
                stop_after_first_confirmed: false,
                max_batch_size: 3,
            },
            move |candidates| {
                batch_cancel.store(true, Ordering::Release);
                async move {
                    Ok(candidates
                        .into_iter()
                        .enumerate()
                        .map(|(candidate_index, _candidate)| StrategyBatchProbeResult {
                            candidate_index,
                            elapsed_ms: 1,
                            probe: StrategyProbeResult {
                                baseline_ok: Some(false),
                                candidate_ok: Some(true),
                                detail: "preliminary only".into(),
                            },
                        })
                        .collect())
                }
            },
            |_: &StrategyCandidate, _: usize| async { Vec::<StrategyProbeResult>::new() },
            |_, _, _| {},
        )
        .await;

        assert_eq!(output.state, "cancelled");
        assert!(output.results.iter().all(|row| !row.confirmed));
    }
}

#[cfg(test)]
mod refinement_tests {
    use super::{StrategyCandidate, StrategyProtocol, refinement_variants};
    use std::collections::HashSet;

    async fn forbidden_probe() -> Vec<super::StrategyProbeResult> {
        panic!("refinement must not start a probe in this branch")
    }

    #[test]
    fn builds_upstream_one_axis_variants_without_changing_other_steps() {
        let candidate = StrategyCandidate {
            name: "hostfake + split".into(),
            source: "blockcheck2".into(),
            steps: vec![
                "--lua-desync=hostfakesplit:repeats=4:tcp_md5:host=old.example".into(),
                "--lua-desync=multisplit:pos=1,midsld".into(),
            ],
        };
        let functions = HashSet::from(["hostfakesplit".into(), "multisplit".into()]);
        let variants = refinement_variants(
            &candidate,
            StrategyProtocol::Https,
            &functions,
            &["tls_client_hello".into()],
        );

        assert!(variants.iter().any(|(label, variant)| {
            label == "fool: tcp_ts"
                && variant.steps[0]
                    == "--lua-desync=hostfakesplit:repeats=4:host=old.example:tcp_ts=-600000"
                && variant.steps[1] == candidate.steps[1]
        }));
        assert!(variants.iter().any(|(label, variant)| {
            label == "repeats: 1"
                && variant.steps[0] == "--lua-desync=hostfakesplit:tcp_md5:host=old.example"
        }));
        assert!(variants.iter().any(|(label, variant)| {
            label == "name: www.google.com" && variant.steps[0].ends_with(":host=www.google.com")
        }));
    }

    #[test]
    fn refinement_skips_http_and_rejects_unavailable_functions_or_blobs() {
        let candidate = StrategyCandidate {
            name: "fake".into(),
            source: "blockcheck2".into(),
            steps: vec!["--lua-desync=fake:blob=tls_client_hello".into()],
        };
        let functions = HashSet::from(["fake".into()]);
        assert!(
            refinement_variants(&candidate, StrategyProtocol::Http, &functions, &[]).is_empty()
        );
        let variants = refinement_variants(
            &candidate,
            StrategyProtocol::Https,
            &functions,
            &["tls_client_hello".into()],
        );
        assert!(!variants.is_empty());
        assert!(
            variants.iter().all(|(_, variant)| {
                variant.steps.iter().all(|step| !step.contains("tls_clone"))
            })
        );
    }

    #[test]
    fn chooses_at_most_two_fake_bases_by_success_count_and_forced_mode_by_duration() {
        use super::{StrategyCandidateResult, select_refinement_bases};
        let result = |function: &str, successes, elapsed_ms| StrategyCandidateResult {
            candidate: StrategyCandidate {
                name: function.into(),
                source: "blockcheck2".into(),
                steps: vec![format!("--lua-desync={function}:blob=tls_client_hello")],
            },
            attempts: 3,
            successes,
            confirmed: successes == 3,
            elapsed_ms,
            detail: "timeout".into(),
        };
        let candidates = vec![
            result("fake", 1, 500),
            result("hostfakesplit", 2, 400),
            result("fakedsplit", 1, 100),
        ];
        let selected = select_refinement_bases(&candidates, StrategyProtocol::Https, false, false);
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].candidate.name, "hostfakesplit");
        assert_ne!(
            selected[0].candidate.steps[0].split(':').next(),
            selected[1].candidate.steps[0].split(':').next()
        );
        assert!(
            select_refinement_bases(&candidates, StrategyProtocol::Https, true, false).is_empty()
        );
        let confirmed = vec![result("fake", 3, 500), result("hostfakesplit", 3, 100)];
        let forced = select_refinement_bases(&confirmed, StrategyProtocol::Https, true, true);
        assert_eq!(forced.len(), 1);
        assert_eq!(forced[0].candidate.name, "hostfakesplit");
    }

    #[tokio::test]
    async fn refinement_reuses_the_real_probe_callback_and_confirms_full_repeats() {
        use super::{StrategyCandidateResult, StrategyProbeResult, refine_candidates};
        use std::sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        };
        let calls = Arc::new(AtomicUsize::new(0));
        let runner_calls = calls.clone();
        let base = StrategyCandidateResult {
            candidate: StrategyCandidate {
                name: "host fake".into(),
                source: "blockcheck2".into(),
                steps: vec!["--lua-desync=hostfakesplit:host=ya.ru".into()],
            },
            attempts: 2,
            successes: 0,
            confirmed: false,
            elapsed_ms: 20,
            detail: "timeout".into(),
        };
        let functions = HashSet::from(["hostfakesplit".into()]);
        let candidates = [base];
        let outcome = refine_candidates(
            super::StrategyRefinementConfig {
                candidates: &candidates,
                protocol: StrategyProtocol::Https,
                repeats: 2,
                already_confirmed: false,
                force: false,
                cancel: Arc::new(AtomicBool::new(false)),
                available_functions: &functions,
                available_blobs: &[],
            },
            move |_candidate, _repeat| {
                runner_calls.fetch_add(1, Ordering::SeqCst);
                async {
                    vec![
                        StrategyProbeResult {
                            baseline_ok: Some(false),
                            candidate_ok: Some(true),
                            detail: "HTTP response received".into(),
                        };
                        2
                    ]
                }
            },
            |_done, _total, _current| {},
        )
        .await;
        assert_eq!(outcome.state, "complete");
        assert!(outcome.results.iter().any(|result| result.confirmed));
        assert!(outcome.total >= 13 && outcome.total <= 17);
        assert_eq!(calls.load(Ordering::SeqCst), outcome.total);
    }

    #[tokio::test]
    async fn refinement_does_not_probe_after_cancellation_or_without_force_when_a_fake_works() {
        use super::{StrategyCandidateResult, StrategyProbeResult, refine_candidates};
        use std::sync::{
            Arc,
            atomic::{AtomicBool, AtomicUsize, Ordering},
        };
        let base = StrategyCandidateResult {
            candidate: StrategyCandidate {
                name: "host fake".into(),
                source: "blockcheck2".into(),
                steps: vec!["--lua-desync=hostfakesplit:host=ya.ru".into()],
            },
            attempts: 2,
            successes: 2,
            confirmed: true,
            elapsed_ms: 1,
            detail: "HTTP 200".into(),
        };
        let functions = HashSet::from(["hostfakesplit".into()]);
        let calls = Arc::new(AtomicUsize::new(0));
        let runner_calls = calls.clone();
        let result = refine_candidates(
            super::StrategyRefinementConfig {
                candidates: std::slice::from_ref(&base),
                protocol: StrategyProtocol::Https,
                repeats: 2,
                already_confirmed: true,
                force: false,
                cancel: Arc::new(AtomicBool::new(true)),
                available_functions: &functions,
                available_blobs: &[],
            },
            move |_candidate, _repeat| {
                runner_calls.fetch_add(1, Ordering::SeqCst);
                async {
                    vec![
                        StrategyProbeResult {
                            baseline_ok: Some(false),
                            candidate_ok: Some(true),
                            detail: "ok".into(),
                        };
                        2
                    ]
                }
            },
            |_done, _total, _current| {},
        )
        .await;
        assert_eq!(result.state, "complete");
        assert_eq!(calls.load(Ordering::SeqCst), 0);

        let cancelled_base = StrategyCandidateResult {
            confirmed: false,
            successes: 1,
            ..base.clone()
        };
        let cancelled = refine_candidates(
            super::StrategyRefinementConfig {
                candidates: std::slice::from_ref(&cancelled_base),
                protocol: StrategyProtocol::Https,
                repeats: 2,
                already_confirmed: false,
                force: false,
                cancel: Arc::new(AtomicBool::new(true)),
                available_functions: &functions,
                available_blobs: &[],
            },
            |_candidate, _repeat| forbidden_probe(),
            |_done, _total, _current| {},
        )
        .await;
        assert_eq!(cancelled.state, "cancelled");

        let skipped = refine_candidates(
            super::StrategyRefinementConfig {
                candidates: std::slice::from_ref(&base),
                protocol: StrategyProtocol::Https,
                repeats: 2,
                already_confirmed: true,
                force: false,
                cancel: Arc::new(AtomicBool::new(false)),
                available_functions: &functions,
                available_blobs: &[],
            },
            |_candidate, _repeat| forbidden_probe(),
            |_done, _total, _current| {},
        )
        .await;
        assert_eq!(skipped.total, 0);
        assert_eq!(skipped.message, "Уточнение не требуется");
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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrategyBatchProbeResult {
    /// Index in the owned candidate slice passed to the batch callback.
    pub candidate_index: usize,
    pub elapsed_ms: u64,
    pub probe: StrategyProbeResult,
}

/// Normalize one-result test runners and real batched runners to a bounded
/// sequence. Production candidate execution uses a batch so router setup is
/// performed once; the single-result implementation keeps lightweight unit
/// fixtures source-compatible.
pub trait IntoProbeBatch {
    fn into_probe_batch(self, requested: usize) -> Vec<StrategyProbeResult>;
}

impl IntoProbeBatch for StrategyProbeResult {
    fn into_probe_batch(self, requested: usize) -> Vec<StrategyProbeResult> {
        vec![self; requested]
    }
}

impl IntoProbeBatch for Vec<StrategyProbeResult> {
    fn into_probe_batch(self, requested: usize) -> Vec<StrategyProbeResult> {
        self.into_iter().take(requested).collect()
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyCandidateResult {
    pub candidate: StrategyCandidate,
    pub attempts: usize,
    pub successes: usize,
    pub confirmed: bool,
    #[serde(default)]
    pub elapsed_ms: u64,
    pub detail: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrategyRunResult {
    pub state: String,
    pub checked: usize,
    pub total: usize,
    pub results: Vec<StrategyCandidateResult>,
    pub refinement_bases: Vec<StrategyCandidateResult>,
    pub failures: usize,
    pub omitted_results: usize,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StrategyRefinementOutcome {
    pub state: String,
    pub checked: usize,
    pub total: usize,
    pub results: Vec<StrategyCandidateResult>,
    pub message: String,
}

pub struct StrategyRefinementConfig<'a> {
    pub candidates: &'a [StrategyCandidateResult],
    pub protocol: StrategyProtocol,
    pub repeats: usize,
    pub already_confirmed: bool,
    pub force: bool,
    pub cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    pub available_functions: &'a HashSet<String>,
    pub available_blobs: &'a [String],
}

fn refinement_stage(detail: &str) -> u8 {
    let detail = detail.to_ascii_lowercase();
    if detail.contains("16–20") || detail.contains("16-20") || detail.contains("possiblefreeze") {
        2
    } else if detail.contains("tls") || detail.contains("сброш") || detail.contains("reset") {
        1
    } else {
        0
    }
}

fn select_refinement_bases(
    candidates: &[StrategyCandidateResult],
    protocol: StrategyProtocol,
    already_confirmed: bool,
    force: bool,
) -> Vec<StrategyCandidateResult> {
    if protocol == StrategyProtocol::Http || (already_confirmed && !force) {
        return Vec::new();
    }
    let mut eligible = candidates
        .iter()
        .filter(|candidate| {
            refinement_fake_index(&candidate.candidate.steps).is_some()
                && (!already_confirmed || candidate.confirmed)
        })
        .cloned()
        .collect::<Vec<_>>();
    if already_confirmed && force {
        eligible.sort_by_key(|candidate| candidate.elapsed_ms);
        eligible.truncate(1);
        return eligible;
    }
    eligible.sort_by(|a, b| {
        b.successes
            .cmp(&a.successes)
            .then_with(|| refinement_stage(&b.detail).cmp(&refinement_stage(&a.detail)))
    });
    let mut functions = HashSet::new();
    eligible.retain(|candidate| {
        refinement_fake_index(&candidate.candidate.steps)
            .and_then(|index| parse_refinement_step(&candidate.candidate.steps[index]))
            .is_some_and(|(function, _)| functions.insert(function))
    });
    eligible.truncate(2);
    eligible
}

/// Run the upstream-style second phase: mutate one fake axis at a time,
/// confirm each candidate with the same real runner, and adopt an axis winner
/// only when its success count improves or ties at full confirmation.
pub async fn refine_candidates<F, Fut, P>(
    config: StrategyRefinementConfig<'_>,
    mut runner: F,
    mut progress: P,
) -> StrategyRefinementOutcome
where
    F: FnMut(&StrategyCandidate, usize) -> Fut,
    Fut: std::future::Future,
    Fut::Output: IntoProbeBatch,
    P: FnMut(usize, usize, &str),
{
    use std::sync::atomic::Ordering;
    let StrategyRefinementConfig {
        candidates,
        protocol,
        repeats,
        already_confirmed,
        force,
        cancel,
        available_functions,
        available_blobs,
    } = config;
    let bases = select_refinement_bases(candidates, protocol, already_confirmed, force);
    let mut outcome = StrategyRefinementOutcome {
        state: "complete".into(),
        checked: 0,
        total: 0,
        results: Vec::new(),
        message: "Уточнение не требуется".into(),
    };
    if bases.is_empty() {
        return outcome;
    }
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(300);
    const AXES: [&str; 4] = ["fool:", "repeats:", "name:", "blob:"];
    for base in bases {
        let mut current = base.clone();
        let mut improved = false;
        for axis in AXES {
            if cancel.load(Ordering::Acquire) {
                outcome.state = "cancelled".into();
                outcome.message = "Уточнение отменено; завершённые проверки сохранены".into();
                return outcome;
            }
            if std::time::Instant::now() >= deadline {
                outcome.state = "partial".into();
                outcome.message =
                    "Уточнение остановлено по лимиту 300 секунд; завершённые проверки сохранены"
                        .into();
                return outcome;
            }
            let variants = refinement_variants(
                &current.candidate,
                protocol,
                available_functions,
                available_blobs,
            )
            .into_iter()
            .filter(|(label, _)| label.starts_with(axis))
            .collect::<Vec<_>>();
            outcome.total = outcome.total.saturating_add(variants.len());
            let mut best: Option<StrategyCandidateResult> = None;
            for (label, candidate) in variants {
                progress(outcome.checked, outcome.total, &candidate.name);
                if cancel.load(Ordering::Acquire) {
                    outcome.state = "cancelled".into();
                    outcome.message = "Уточнение отменено; завершённые проверки сохранены".into();
                    return outcome;
                }
                if std::time::Instant::now() >= deadline {
                    outcome.state = "partial".into();
                    outcome.message = "Уточнение остановлено по лимиту 300 секунд; завершённые проверки сохранены".into();
                    return outcome;
                }
                let started = std::time::Instant::now();
                let mut attempts = 0;
                let mut successes = 0;
                let mut detail = String::new();
                let probes = runner(&candidate, repeats.max(1))
                    .await
                    .into_probe_batch(repeats.max(1));
                for probe in probes {
                    attempts += 1;
                    detail = probe.detail;
                    match (probe.baseline_ok, probe.candidate_ok) {
                        (Some(false), Some(true)) => successes += 1,
                        (Some(false), Some(false)) => break,
                        _ => break,
                    }
                    if cancel.load(Ordering::Acquire) || std::time::Instant::now() >= deadline {
                        break;
                    }
                }
                outcome.checked += 1;
                progress(outcome.checked, outcome.total, &label);
                let result = StrategyCandidateResult {
                    candidate,
                    attempts,
                    successes,
                    confirmed: attempts == repeats.max(1) && successes == repeats.max(1),
                    elapsed_ms: started.elapsed().as_millis().min(u64::MAX as u128) as u64,
                    detail,
                };
                if successes > 0 {
                    outcome.results.push(result.clone());
                }
                if best
                    .as_ref()
                    .is_none_or(|previous| result.successes > previous.successes)
                {
                    best = Some(result);
                }
                if cancel.load(Ordering::Acquire) {
                    outcome.state = "cancelled".into();
                    outcome.message = "Уточнение отменено; завершённые проверки сохранены".into();
                    return outcome;
                }
                if std::time::Instant::now() >= deadline {
                    outcome.state = "partial".into();
                    outcome.message = "Уточнение остановлено по лимиту 300 секунд; завершённые проверки сохранены".into();
                    return outcome;
                }
            }
            if let Some(best) = best
                && (best.successes > current.successes
                    || (best.successes == current.successes && best.confirmed))
            {
                improved |= best.candidate.steps != current.candidate.steps;
                current = best;
            }
        }
        if improved && current.successes > 0 {
            current.candidate.source = "Уточнение blockcheck2".into();
            current.candidate.name = format!("{} · лучший уточнённый вариант", base.candidate.name);
            outcome.results.push(current);
        }
    }
    outcome.message = if outcome.results.iter().any(|result| result.confirmed) {
        "Уточнение завершено: найдены проверенные варианты; исходная стратегия не менялась".into()
    } else if outcome.results.is_empty() {
        "Уточнение завершено; подтверждённый вариант лучше исходного не найден".into()
    } else {
        "Уточнение завершено: есть частично успешные варианты, повторное подтверждение не получено"
            .into()
    };
    outcome
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
    Fut: std::future::Future,
    Fut::Output: IntoProbeBatch,
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
    Fut: std::future::Future,
    Fut::Output: IntoProbeBatch,
    P: FnMut(usize, usize, &str),
{
    let total = plan.candidates.len();
    let mut result = StrategyRunResult {
        state: "running".into(),
        checked: 0,
        total,
        results: Vec::new(),
        refinement_bases: Vec::new(),
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
        let candidate_started = std::time::Instant::now();
        let mut attempts = 0;
        let mut successes = 0;
        let mut detail = String::new();
        progress(result.checked, total, &candidate.name);
        if cancel.load(std::sync::atomic::Ordering::Acquire) {
            result.state = "cancelled".into();
            result.message = "Подбор отменён".into();
            return result;
        }
        let probes = runner(&candidate, repeats.max(1))
            .await
            .into_probe_batch(repeats.max(1));
        for probe in probes {
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
                _ if cancel.load(std::sync::atomic::Ordering::Acquire) => {
                    result.results.push(StrategyCandidateResult {
                        candidate: candidate.clone(),
                        attempts,
                        successes,
                        confirmed: false,
                        elapsed_ms: candidate_started
                            .elapsed()
                            .as_millis()
                            .min(u64::MAX as u128) as u64,
                        detail,
                    });
                    result.checked += 1;
                    result.state = "cancelled".into();
                    result.message = "Подбор отменён; завершённые повторы сохранены".into();
                    return result;
                }
                _ => {
                    result.state = "failed".into();
                    result.message = "Сетевая проверка не дала достаточного результата".into();
                    result.results.push(StrategyCandidateResult {
                        candidate,
                        attempts,
                        successes,
                        confirmed: false,
                        elapsed_ms: candidate_started
                            .elapsed()
                            .as_millis()
                            .min(u64::MAX as u128) as u64,
                        detail,
                    });
                    return result;
                }
            }
            if cancel.load(std::sync::atomic::Ordering::Acquire) {
                result.results.push(StrategyCandidateResult {
                    candidate: candidate.clone(),
                    attempts,
                    successes,
                    confirmed: false,
                    elapsed_ms: candidate_started
                        .elapsed()
                        .as_millis()
                        .min(u64::MAX as u128) as u64,
                    detail,
                });
                result.checked += 1;
                result.state = "cancelled".into();
                result.message = "Подбор отменён; завершённые повторы сохранены".into();
                return result;
            }
        }
        let confirmed = attempts == repeats.max(1) && successes == repeats.max(1);
        let candidate_result = StrategyCandidateResult {
            candidate: candidate.clone(),
            attempts,
            successes,
            confirmed,
            elapsed_ms: candidate_started
                .elapsed()
                .as_millis()
                .min(u64::MAX as u128) as u64,
            detail,
        };
        if refinement_fake_index(&candidate.steps).is_some() && result.refinement_bases.len() < 64 {
            result.refinement_bases.push(candidate_result.clone());
        }
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

/// Full mode keeps the high-priority candidates sequential, batches only the
/// generated catalog, then serially confirms batch-positive candidates in
/// observed-latency order. Batch outcomes are never considered confirmation.
#[derive(Debug, Clone, Copy)]
pub struct FullBatchOptions {
    pub repeats: usize,
    pub stop_after_first_confirmed: bool,
    pub max_batch_size: usize,
}

pub async fn run_full_candidates_with_batches<B, BFut, S, SFut, P>(
    plan: StrategyPlan,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
    options: FullBatchOptions,
    mut batch_runner: B,
    mut serial_runner: S,
    mut progress: P,
) -> StrategyRunResult
where
    B: FnMut(Vec<StrategyCandidate>) -> BFut,
    BFut: std::future::Future<Output = Result<Vec<StrategyBatchProbeResult>, String>>,
    S: FnMut(&StrategyCandidate, usize) -> SFut,
    SFut: std::future::Future,
    SFut::Output: IntoProbeBatch,
    P: FnMut(usize, usize, &str),
{
    let FullBatchOptions {
        repeats,
        stop_after_first_confirmed,
        max_batch_size,
    } = options;
    let total = plan.candidates.len();
    let split = plan
        .candidates
        .iter()
        .position(|candidate| candidate.source.starts_with("Каталог zapret2"))
        .unwrap_or(total);
    let prefix = plan.candidates[..split].to_vec();
    let catalog = plan.candidates[split..].to_vec();
    let mut result = StrategyRunResult {
        state: "running".into(),
        checked: 0,
        total,
        results: Vec::new(),
        refinement_bases: Vec::new(),
        failures: 0,
        omitted_results: 0,
        message: "Подготовка кандидатов".into(),
    };

    if !prefix.is_empty() {
        let prefix_result = run_candidates_with_compaction(
            StrategyPlan {
                protocol: plan.protocol,
                candidates: prefix,
            },
            cancel.clone(),
            repeats,
            stop_after_first_confirmed,
            true,
            &mut serial_runner,
            |done, _, current| progress(done, total, current),
        )
        .await;
        result.checked = prefix_result.checked;
        result.results = prefix_result.results;
        result.refinement_bases = prefix_result.refinement_bases;
        result.failures = prefix_result.failures;
        result.omitted_results = prefix_result.omitted_results;
        result.state = prefix_result.state;
        result.message = prefix_result.message;
        if matches!(
            result.state.as_str(),
            "cancelled" | "failed" | "site_unblocked"
        ) || (stop_after_first_confirmed && result.state == "complete")
            || catalog.is_empty()
        {
            result.total = total;
            return result;
        }
        result.state = "running".into();
    }

    let batch_size = max_batch_size.clamp(1, crate::probe_lanes::MAX_BATCH_LANES);
    let mut cursor = 0;
    while cursor < catalog.len() {
        if cancel.load(std::sync::atomic::Ordering::Acquire) {
            result.state = "cancelled".into();
            result.message =
                "Подбор отменён; пакетные результаты без подтверждения сохранены".into();
            break;
        }
        let end = (cursor + batch_size).min(catalog.len());
        let batch = catalog[cursor..end].to_vec();
        for candidate in &batch {
            progress(
                result.checked,
                total,
                &format!("Пакетная проверка: {}", candidate.name),
            );
        }
        let batch_result = batch_runner(batch.clone()).await;
        let probes = match batch_result {
            Ok(probes) if !probes.is_empty() && probes.len() <= batch.len() => probes,
            Ok(_) => Vec::new(),
            Err(message) if message.starts_with("CLEANUP_UNCONFIRMED:") => {
                result.state = "failed".into();
                result.message = message;
                return result;
            }
            Err(_) => Vec::new(),
        };

        // If the lane runner cannot form a safe batch (or returns malformed
        // output), fall back to the exact serial runner for one candidate.
        if probes.is_empty()
            || probes
                .iter()
                .any(|probe| probe.candidate_index >= batch.len())
            || (0..probes.len()).any(|index| {
                probes
                    .iter()
                    .filter(|probe| probe.candidate_index == index)
                    .count()
                    != 1
            })
        {
            let mut one = run_candidates_with_compaction(
                StrategyPlan {
                    protocol: plan.protocol,
                    candidates: vec![batch[0].clone()],
                },
                cancel.clone(),
                repeats,
                stop_after_first_confirmed,
                true,
                &mut serial_runner,
                |done, _, current| progress(result.checked + done, total, current),
            )
            .await;
            result.checked += one.checked;
            result.failures += one.failures;
            result.omitted_results += one.omitted_results;
            result.refinement_bases.append(&mut one.refinement_bases);
            result.results.append(&mut one.results);
            cursor += 1;
            if matches!(
                one.state.as_str(),
                "cancelled" | "failed" | "site_unblocked"
            ) || (stop_after_first_confirmed && one.state == "complete")
            {
                result.state = one.state;
                result.message = one.message;
                break;
            }
            continue;
        }

        let tested = probes.len();
        let mut positives = Vec::new();
        for probe in probes {
            let candidate = batch[probe.candidate_index].clone();
            match (probe.probe.baseline_ok, probe.probe.candidate_ok) {
                (Some(true), _) => {
                    result.state = "site_unblocked".into();
                    result.message =
                        "Прямой запрос открыл сайт; подбор стратегии не требуется".into();
                    return result;
                }
                (Some(false), Some(true)) => positives.push((probe.elapsed_ms, candidate)),
                (Some(false), Some(false)) => {
                    result.failures += 1;
                    result.checked += 1;
                }
                _ if cancel.load(std::sync::atomic::Ordering::Acquire) => {
                    result.state = "cancelled".into();
                    result.message =
                        "Пакетная проверка отменена; положительные варианты не подтверждены".into();
                    return result;
                }
                _ => {
                    result.state = "failed".into();
                    result.message =
                        "Пакетная сетевая проверка не дала достаточного результата".into();
                    return result;
                }
            }
        }
        positives.sort_by_key(|(elapsed, _)| *elapsed);
        if cancel.load(std::sync::atomic::Ordering::Acquire) {
            result.state = "cancelled".into();
            result.message =
                "Пакетная проверка отменена; положительные варианты не подтверждены".into();
            return result;
        }
        for (_, candidate) in positives {
            progress(
                result.checked,
                total,
                &format!("Подтверждение: {}", candidate.name),
            );
            let mut confirmation = run_candidates_with_compaction(
                StrategyPlan {
                    protocol: plan.protocol,
                    candidates: vec![candidate],
                },
                cancel.clone(),
                repeats,
                false,
                true,
                &mut serial_runner,
                |done, _, current| progress(result.checked + done, total, current),
            )
            .await;
            result.checked += confirmation.checked;
            result.failures += confirmation.failures;
            result.omitted_results += confirmation.omitted_results;
            result
                .refinement_bases
                .append(&mut confirmation.refinement_bases);
            result.results.append(&mut confirmation.results);
            if confirmation.state == "cancelled" {
                result.state = "cancelled".into();
                result.message = confirmation.message;
                return result;
            }
            if confirmation.state == "failed" || confirmation.state == "site_unblocked" {
                result.state = confirmation.state;
                result.message = confirmation.message;
                return result;
            }
            if stop_after_first_confirmed && confirmation.state == "complete" {
                result.state = "complete".into();
                result.message = confirmation.message;
                return result;
            }
        }
        cursor += tested;
    }

    if result.state == "running" {
        result.state = if result.results.iter().any(|row| row.confirmed) {
            "complete"
        } else {
            "no_match"
        }
        .into();
        result.message = if result.state == "complete" {
            format!(
                "Найдены варианты, подтверждённые {repeats} последовательными сетевыми проверками"
            )
        } else {
            "Подтверждённая стратегия не найдена".into()
        };
        result
            .message
            .push_str(&format!(" · без успешного ответа: {}", result.failures));
    }
    result.total = total;
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
    let mut candidates = Vec::new();
    for group in groups {
        let mut port_spec = None;
        let mut l7 = None;
        let mut payload: Option<String> = None;
        let mut profile_steps = Vec::<(Option<String>, Vec<String>)>::new();
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
                && payload.as_deref().is_none_or(|value| {
                    value
                        .split(',')
                        .any(|item| item == wanted_payload || item == "all" || item == "known")
                })
            {
                let strategy_id = value
                    .split(':')
                    .find_map(|part| part.strip_prefix("strategy="));
                if let Some((_, steps)) = profile_steps
                    .iter_mut()
                    .find(|(existing_strategy, _)| existing_strategy.as_deref() == strategy_id)
                {
                    steps.push(arg.clone());
                } else {
                    profile_steps.push((strategy_id.map(str::to_owned), vec![arg.clone()]));
                }
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
        if !ports_match || !layer_match {
            continue;
        }
        candidates.extend(profile_steps.into_iter().map(|(_, steps)| steps));
    }
    candidates
}

#[cfg(test)]
mod configured_profile_tests {
    use super::{StrategyMode, StrategyProtocol, build_plan, configured_candidates};

    #[test]
    fn splits_numbered_strategy_steps_into_separate_candidates() {
        let active = vec![
            "nfqws2".into(),
            "--filter-tcp=443".into(),
            "--filter-l7=tls".into(),
            "--payload=tls_client_hello".into(),
            "--lua-desync=circular:fails=2:time=300".into(),
            "--lua-desync=fake:blob=tls_clienthello:strategy=1".into(),
            "--lua-desync=multisplit:pos=1:strategy=1".into(),
            "--lua-desync=fake:blob=tls_clienthello:strategy=2".into(),
            "--lua-desync=hostfakesplit:host=google.com:strategy=2".into(),
        ];

        let candidates = configured_candidates(&active, StrategyProtocol::Https);

        assert_eq!(
            candidates,
            vec![
                vec![
                    "--lua-desync=fake:blob=tls_clienthello:strategy=1",
                    "--lua-desync=multisplit:pos=1:strategy=1",
                ],
                vec![
                    "--lua-desync=fake:blob=tls_clienthello:strategy=2",
                    "--lua-desync=hostfakesplit:host=google.com:strategy=2",
                ],
            ]
        );
    }

    #[test]
    fn exposes_each_numbered_profile_strategy_as_its_own_plan_candidate() {
        let active = vec![
            "nfqws2".into(),
            "--filter-tcp=443".into(),
            "--filter-l7=tls".into(),
            "--payload=tls_client_hello".into(),
            "--lua-desync=circular:fails=2:time=300".into(),
            "--lua-desync=fake:blob=tls_clienthello:strategy=1".into(),
            "--lua-desync=multisplit:pos=1:strategy=1".into(),
            "--lua-desync=hostfakesplit:host=google.com:strategy=2".into(),
        ];
        let functions = ["fake", "multisplit", "hostfakesplit"]
            .into_iter()
            .map(str::to_owned)
            .collect();
        let plan = build_plan(
            StrategyProtocol::Https,
            StrategyMode::Extended,
            &configured_candidates(&active, StrategyProtocol::Https),
            &functions,
            &["tls_clienthello".into()],
        );

        assert_eq!(plan.candidates[0].source, "Текущий профиль · стратегия 1");
        assert_eq!(plan.candidates[0].steps.len(), 2);
        assert_eq!(plan.candidates[1].source, "Текущий профиль · стратегия 2");
        assert_eq!(plan.candidates[1].steps.len(), 1);
        assert!(
            plan.candidates[0]
                .steps
                .iter()
                .all(|step| !step.contains("strategy="))
        );
        assert!(
            plan.candidates[1]
                .steps
                .iter()
                .all(|step| !step.contains("strategy="))
        );
    }

    #[test]
    fn combines_same_numbered_strategy_across_matching_payload_contexts() {
        let active = vec![
            "nfqws2".into(),
            "--filter-tcp=443".into(),
            "--filter-l7=tls".into(),
            "--lua-desync=fake:blob=tls_clienthello:strategy=1".into(),
            "--payload=tls_client_hello".into(),
            "--lua-desync=multisplit:pos=1:strategy=1".into(),
        ];

        assert_eq!(
            configured_candidates(&active, StrategyProtocol::Https),
            vec![vec![
                "--lua-desync=fake:blob=tls_clienthello:strategy=1",
                "--lua-desync=multisplit:pos=1:strategy=1",
            ]],
        );
    }

    #[test]
    fn keeps_unnumbered_profile_steps_as_one_candidate() {
        let active = vec![
            "nfqws2".into(),
            "--filter-tcp=443".into(),
            "--filter-l7=tls".into(),
            "--payload=tls_client_hello".into(),
            "--lua-desync=fake:blob=tls_clienthello".into(),
            "--lua-desync=multisplit:pos=1".into(),
        ];

        assert_eq!(
            configured_candidates(&active, StrategyProtocol::Https),
            vec![vec![
                "--lua-desync=fake:blob=tls_clienthello",
                "--lua-desync=multisplit:pos=1",
            ]],
        );
    }

    #[test]
    fn keeps_payload_context_for_each_step_and_includes_unnumbered_http_profile() {
        let active = vec![
            "nfqws2".into(),
            "--filter-tcp=443,80".into(),
            "--filter-l7=http,tls".into(),
            "--payload=tls_client_hello".into(),
            "--lua-desync=fake:blob=tls_clienthello:strategy=1".into(),
            "--lua-desync=multisplit:pos=1:strategy=1".into(),
            "--lua-desync=hostfakesplit:host=google.com:strategy=2".into(),
            "--payload=http_req".into(),
            "--lua-desync=http_methodeol:badsum".into(),
        ];

        assert_eq!(
            configured_candidates(&active, StrategyProtocol::Https),
            vec![
                vec![
                    "--lua-desync=fake:blob=tls_clienthello:strategy=1",
                    "--lua-desync=multisplit:pos=1:strategy=1",
                ],
                vec!["--lua-desync=hostfakesplit:host=google.com:strategy=2"],
            ],
        );
        assert_eq!(
            configured_candidates(&active, StrategyProtocol::Http),
            vec![vec!["--lua-desync=http_methodeol:badsum"]],
        );
    }
}
