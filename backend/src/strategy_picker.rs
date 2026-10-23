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

/// A single router session stays deliberately small and sequential; wider
/// catalog searches need hardware-specific resource measurements first.
pub const MAX_CANDIDATES: usize = 24;
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
    StrategyPlan {
        protocol,
        candidates,
    }
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
    pub message: String,
}

/// Sequentially execute each candidate twice. A direct success means there is
/// no evidence that DPI mitigation is needed; partial/random success is never
/// marked confirmed. The runner is the real isolated network probe supplied by
/// the application, not a simulated result.
pub async fn run_candidates<F, Fut, P>(
    plan: StrategyPlan,
    cancel: std::sync::Arc<std::sync::atomic::AtomicBool>,
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
        for repeat in 1..=2 {
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
        let confirmed = successes == 2;
        result.results.push(StrategyCandidateResult {
            candidate: candidate.clone(),
            attempts,
            successes,
            confirmed,
            detail,
        });
        result.checked += 1;
    }
    result.state = if result.results.iter().any(|item| item.confirmed) {
        "complete"
    } else {
        "no_match"
    }
    .into();
    result.message = if result.state == "complete" {
        "Найдены варианты, подтверждённые двумя последовательными сетевыми проверками".into()
    } else {
        "Подтверждённая стратегия не найдена".into()
    };
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
