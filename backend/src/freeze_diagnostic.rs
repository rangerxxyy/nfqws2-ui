//! Bounded network-level 16–20 KB diagnostics adapted from the audited
//! `freezeJob`, `padProbe`, and `freezeCandidates` behavior.
use crate::{
    site_diagnostics::{normalize_host, public_ip},
    transfer_diagnostic::{TransferAttempt, TransferClass, classify_transfer},
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashSet,
    net::{IpAddr, Ipv4Addr},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{net::lookup_host, process::Command, time::timeout};

pub const PAD_REQUESTS: usize = 12;
pub const PAD_HEADER_BYTES: usize = 4_000;
pub const MAX_EXTRA_HOSTS: usize = 20;
pub const MAX_TARGETS: usize = 40;
pub const FREEZE_NAMES: [&str; 8] = [
    "ya.ru",
    "300.ya.ru",
    "hcaptcha.com",
    "ad.adriver.ru",
    "vk.com",
    "gosuslugi.ru",
    "ozon.ru",
    "www.google.com",
];

const HOST_REFERENCES: [(&str, &str); 7] = [
    ("OVH", "proof.ovh.net/files/1Mb.dat"),
    (
        "Linode (Akamai)",
        "speedtest.frankfurt.linode.com/100MB-frankfurt.bin",
    ),
    ("Vultr", "fra-de-ping.vultr.com/vultr.com.100MB.bin"),
    (
        "DataPacket (CDN77)",
        "fra.download.datapacket.com/100mb.bin",
    ),
    (
        "Amazon CloudFront",
        "ip-ranges.amazonaws.com/ip-ranges.json",
    ),
    ("Cloudflare", "speed.cloudflare.com/__down?bytes=200000"),
    ("Selectel — Россия, сравнение", "speedtest.selectel.ru/10MB"),
];

const PINNED_REFERENCES: [(&str, &str, u16, u32, Option<&str>); 13] = [
    ("Hetzner", "91.98.156.82", 443, 24940, None),
    ("Hetzner Cloud", "5.161.249.234", 443, 213230, None),
    ("DigitalOcean", "157.230.91.62", 443, 14061, None),
    ("Contabo", "173.249.28.198", 443, 51167, None),
    ("Scaleway", "62.4.10.75", 443, 12876, None),
    ("Oracle Cloud", "132.145.253.77", 443, 31898, None),
    ("Google Cloud", "35.227.66.60", 443, 396982, None),
    ("Akamai", "23.222.76.4", 443, 20940, None),
    (
        "Fastly (GitHub)",
        "185.199.108.133",
        443,
        54113,
        Some("release-assets.githubusercontent.com"),
    ),
    ("Gcore", "62.112.221.146", 443, 199524, None),
    ("Melbicom", "109.122.201.147", 443, 8849, None),
    ("M247", "146.70.158.22", 443, 9009, None),
    ("FranTech (BuyVM)", "107.189.29.143", 443, 53667, None),
];

pub const FREEZE_TEMPLATES: [&str; 3] = [
    "--lua-desync=hostfakesplit:repeats=4:tcp_ts=-600000:tcp_md5:host=NAME",
    "--lua-desync=hostfakesplit:tcp_md5:tcp_ts_up:repeats=16:host=NAME",
    "--lua-desync=fake:blob=fake_default_tls:tls_mod=rnd,dupsid,sni=NAME:tcp_md5:tcp_ts=-600000:repeats=16",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreezeStrategyMatch {
    pub sni: String,
    pub step: String,
    pub attempts: usize,
    pub successes: usize,
    pub average_ms: Option<u64>,
    pub reused_from: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreezeCandidateAttempt {
    pub sni: String,
    pub step: String,
    pub attempts: usize,
    pub successes: usize,
    pub average_ms: Option<u64>,
    pub confirmed: bool,
    pub reason: Option<String>,
}

pub fn freeze_candidates(functions: &HashSet<String>, blobs: &[String]) -> Vec<(String, String)> {
    let mut candidates = Vec::new();
    for template in FREEZE_TEMPLATES {
        for name in FREEZE_NAMES {
            let Ok(steps) = instantiate_template(template, name) else {
                continue;
            };
            if crate::strategy_picker::validate_candidate(&steps, functions, blobs).is_ok() {
                candidates.push((name.to_owned(), steps[0].clone()));
            }
        }
    }
    candidates
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", tag = "state")]
pub enum AsnVerification {
    Verified { asn: u32 },
    Changed { expected: u32, actual: u32 },
    Unavailable,
}

/// RIPEstat is used only to decide whether results may be reused across two
/// targets. A catalog hint is never treated as a current ASN observation.
pub fn verify_asn_hint(hint: Option<u32>, response: &[u8]) -> AsnVerification {
    let parsed = serde_json::from_slice::<serde_json::Value>(response).ok();
    let asn = parsed
        .as_ref()
        .and_then(|value| value.get("data"))
        .and_then(|value| value.get("asns"))
        .and_then(serde_json::Value::as_array)
        .and_then(|asns| asns.iter().find_map(serde_json::Value::as_u64))
        .and_then(|asn| u32::try_from(asn).ok())
        .filter(|asn| *asn > 0);
    match (hint.filter(|hint| *hint > 0), asn) {
        (Some(expected), Some(actual)) if expected == actual => {
            AsnVerification::Verified { asn: actual }
        }
        (Some(expected), Some(actual)) => AsnVerification::Changed { expected, actual },
        (None, Some(actual)) => AsnVerification::Verified { asn: actual },
        (_, None) => AsnVerification::Unavailable,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FreezeTargetKind {
    HostReference,
    PinnedAddress,
    UserHost,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreezeTarget {
    pub label: String,
    pub host: String,
    pub path: String,
    pub address: Option<Ipv4Addr>,
    pub port: u16,
    pub asn_hint: Option<u32>,
    pub sni: String,
    pub kind: FreezeTargetKind,
}

pub fn build_targets(extra_hosts: &[String]) -> Result<Vec<FreezeTarget>, String> {
    if extra_hosts.len() > MAX_EXTRA_HOSTS {
        return Err(format!(
            "Можно добавить не более {MAX_EXTRA_HOSTS} собственных доменов"
        ));
    }
    let mut targets = Vec::new();
    let mut seen_hosts = HashSet::new();
    for raw in extra_hosts {
        let host = normalize_host(raw)?;
        if seen_hosts.insert(host.clone()) {
            targets.push(FreezeTarget {
                label: host.clone(),
                host,
                path: "/".into(),
                address: None,
                port: 443,
                asn_hint: None,
                sni: String::new(),
                kind: FreezeTargetKind::UserHost,
            });
        }
    }
    for (label, reference) in HOST_REFERENCES {
        let (host, path) = reference
            .split_once('/')
            .expect("reference includes a path");
        targets.push(FreezeTarget {
            label: label.into(),
            host: host.into(),
            path: format!("/{path}"),
            address: None,
            port: 443,
            asn_hint: None,
            sni: host.into(),
            kind: FreezeTargetKind::HostReference,
        });
    }
    for (label, ip, port, asn, sni) in PINNED_REFERENCES {
        let address = ip
            .parse::<Ipv4Addr>()
            .map_err(|_| format!("Некорректный адрес в каталоге: {ip}"))?;
        if !public_ip(IpAddr::V4(address)) || port == 0 || asn == 0 {
            return Err(format!(
                "Небезопасная или неполная запись адреса сети {label}"
            ));
        }
        let sni = sni.unwrap_or("example.com");
        targets.push(FreezeTarget {
            label: label.into(),
            host: sni.into(),
            path: "/".into(),
            address: Some(address),
            port,
            asn_hint: Some(asn),
            sni: sni.into(),
            kind: FreezeTargetKind::PinnedAddress,
        });
    }
    if targets.len() > MAX_TARGETS {
        return Err("Каталог проверки превышает внутренний лимит целей".into());
    }
    Ok(targets)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PadClassification {
    Complete,
    PossibleFreeze,
    TimeoutBeforeResponse,
    ServerDoesNotKeepConnection,
    ConnectFailure,
    TlsFailure,
    Reset,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PadProbeResult {
    pub classification: PadClassification,
    pub replies: usize,
    pub reconnects: bool,
    pub elapsed_ms: u64,
    pub reason: String,
}

pub fn padded_head_curl_args(
    address: Ipv4Addr,
    port: u16,
    sni: &str,
) -> Result<Vec<String>, String> {
    let sni = normalize_host(sni)?;
    if port == 0 || !public_ip(IpAddr::V4(address)) {
        return Err("Проверка требует публичный IPv4-адрес и корректный порт".into());
    }
    let authority = if port == 443 {
        sni.clone()
    } else {
        format!("{sni}:{port}")
    };
    let connect_to = format!("{sni}:{port}:{address}:{port}");
    let protocol = if port == 80 { "http" } else { "https" };
    let base_url = format!("{protocol}://{authority}/");
    let mut args = vec![
        "--disable".into(),
        "--silent".into(),
        "--show-error".into(),
        "--insecure".into(),
        "--noproxy".into(),
        "*".into(),
        "--head".into(),
        "--fail-early".into(),
        "--connect-timeout".into(),
        "4".into(),
        "--max-time".into(),
        "8".into(),
        "--local-port".into(),
        "40000-40999".into(),
        "--connect-to".into(),
        connect_to,
        "-A".into(),
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 Chrome/130 Safari/537.36".into(),
        "-H".into(),
        format!("X-Pad: {}", "a".repeat(PAD_HEADER_BYTES)),
        "-w".into(),
        "%{http_code}|%{num_connects}|%{time_total}\n".into(),
    ];
    for index in 0..PAD_REQUESTS {
        args.push("-o".into());
        args.push(if cfg!(windows) { "NUL" } else { "/dev/null" }.into());
        args.push(format!("{base_url}?n={index}"));
    }
    Ok(args)
}

pub fn classify_padded_head(exit_code: i32, metadata: &str) -> PadProbeResult {
    let mut replies = 0;
    let mut reconnects = false;
    let mut elapsed_ms = 0;
    for (index, line) in metadata.lines().enumerate() {
        let fields = line.trim().split('|').collect::<Vec<_>>();
        let Some(status) = fields.first().and_then(|value| value.parse::<u16>().ok()) else {
            continue;
        };
        let connects = fields
            .get(1)
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or_default();
        let time = fields
            .get(2)
            .and_then(|value| value.parse::<f64>().ok())
            .filter(|value| value.is_finite() && *value >= 0.0)
            .unwrap_or_default();
        if (100..600).contains(&status) {
            replies += 1;
            elapsed_ms += (time * 1000.0).round() as u64;
            reconnects |= index > 0 && connects > 0;
        }
    }
    let (classification, reason) = if reconnects || (exit_code == 0 && replies < PAD_REQUESTS) {
        (
            PadClassification::ServerDoesNotKeepConnection,
            "Сервер не удержал одно соединение; объёмный тест не показателен",
        )
    } else if exit_code == 0 && replies == PAD_REQUESTS {
        (
            PadClassification::Complete,
            "Все padded HEAD-ответы получены через одно соединение",
        )
    } else if exit_code == 28 && replies > 0 {
        (
            PadClassification::PossibleFreeze,
            "Тайм-аут после начала ответов; возможна остановка передачи",
        )
    } else if exit_code == 28 {
        (
            PadClassification::TimeoutBeforeResponse,
            "Тайм-аут до первого HTTP-ответа; обрыв объёма не подтверждён",
        )
    } else if exit_code == 7 {
        (
            PadClassification::ConnectFailure,
            "Не удалось установить TCP-соединение",
        )
    } else if exit_code == 35 {
        (
            PadClassification::TlsFailure,
            "Ошибка TLS до передачи padded HEAD",
        )
    } else if matches!(exit_code, 52 | 56) {
        (
            PadClassification::Reset,
            "Соединение сброшено до завершения padded HEAD",
        )
    } else {
        (
            PadClassification::Inconclusive,
            "Ответов недостаточно для классификации",
        )
    };
    PadProbeResult {
        classification,
        replies,
        reconnects,
        elapsed_ms,
        reason: reason.into(),
    }
}

pub fn network_has_freeze(attempts: &[TransferAttempt], expected_bytes: Option<u64>) -> bool {
    let diagnosis = classify_transfer(attempts, expected_bytes);
    diagnosis.classification == TransferClass::PossibleBoundaryStall
}

/// A successful connection or a timeout before any HTTP response does not
/// establish a 16–20 KB transfer stall. Only an observed stall after response
/// data (or the bounded upload-stall probe) is actionable evidence.
pub fn is_freeze_classification(classification: &str) -> bool {
    matches!(
        classification,
        "PossibleFreeze" | "PossibleBoundaryStall" | "PossibleUploadStall"
    )
}

pub fn instantiate_template(template: &str, name: &str) -> Result<Vec<String>, String> {
    let normalized = normalize_host(name)?;
    if normalized != name || !template.starts_with("--lua-desync=") || !template.contains("NAME") {
        return Err("Шаблон SNI или имя недопустимы".into());
    }
    let step = template.replace("NAME", &normalized);
    if step.contains("NAME")
        || step.len() > 512
        || step.bytes().any(|byte| {
            byte.is_ascii_control()
                || matches!(
                    byte,
                    b' ' | b'\'' | b'"' | b'`' | b'$' | b'\\' | b';' | b'&' | b'|'
                )
        })
    {
        return Err("Сформированный Lua-шаг содержит недопустимые данные".into());
    }
    Ok(vec![step])
}

pub fn candidate_confirmed(outcomes: &[bool]) -> bool {
    outcomes
        .windows(3)
        .any(|window| window.iter().all(|success| *success))
}

/// A strategy candidate is run only on the current isolated route. Its direct
/// baseline has already been measured before candidate enumeration.
pub fn candidate_route_succeeded(current_route_ok: Option<bool>) -> bool {
    current_route_ok == Some(true)
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreezeMeasurement {
    pub classification: String,
    pub ok: bool,
    pub bytes_received: Option<u64>,
    pub replies: Option<usize>,
    pub elapsed_ms: u64,
    pub reason: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FreezeNetworkResult {
    pub label: String,
    pub host: String,
    pub path: String,
    pub address: Option<Ipv4Addr>,
    pub asn_hint: Option<u32>,
    pub asn: AsnVerification,
    pub direct: FreezeMeasurement,
    pub current: FreezeMeasurement,
    pub freeze_detected: bool,
    #[serde(default)]
    pub tried_candidates: usize,
    #[serde(default)]
    pub found_strategy: Option<FreezeStrategyMatch>,
    #[serde(default)]
    pub candidate_attempts: Vec<FreezeCandidateAttempt>,
}

/// Convert only strategies confirmed by the 16 KB scan into picker candidates.
/// The picker still probes them against its requested host; a success on a
/// reference network is a ranking hint, never a claim that it works elsewhere.
pub fn confirmed_strategy_candidates(
    results: &[FreezeNetworkResult],
) -> Vec<crate::strategy_picker::StrategyCandidate> {
    let mut seen = HashSet::new();
    results
        .iter()
        .filter(|result| result.freeze_detected)
        .filter_map(|result| {
            let found = result.found_strategy.as_ref()?;
            if found.attempts < 3 || found.successes < 3 || !seen.insert(found.step.clone()) {
                return None;
            }
            Some(crate::strategy_picker::StrategyCandidate {
                name: format!("Обрыв на 16 КБ · {} · {}", found.sni, result.label),
                source: "Найдено проверкой сетей 16 КБ".into(),
                steps: vec![found.step.clone()],
            })
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FreezeScanOutcome {
    pub results: Vec<FreezeNetworkResult>,
    pub done: usize,
    pub total: usize,
    /// The upstream-compatible global or per-network search budget ended
    /// before all affected networks had been exhausted.
    pub limited: bool,
}

fn curl_program() -> &'static str {
    if std::path::Path::new("/opt/bin/curl").is_file() {
        "/opt/bin/curl"
    } else if cfg!(windows) {
        "curl.exe"
    } else {
        "curl"
    }
}

async fn resolve_address(target: &FreezeTarget) -> Option<Ipv4Addr> {
    if let Some(address) = target.address {
        return Some(address);
    }
    timeout(
        Duration::from_secs(5),
        lookup_host((target.host.as_str(), target.port)),
    )
    .await
    .ok()?
    .ok()?
    .filter_map(|socket| match socket.ip() {
        IpAddr::V4(ip) if public_ip(IpAddr::V4(ip)) => Some(ip),
        _ => None,
    })
    .next()
}

async fn ripe_asn(address: Ipv4Addr, hint: Option<u32>) -> AsnVerification {
    let url = format!(
        "https://stat.ripe.net/data/network-info/data.json?sourceapp=nfqws2-ui&resource={address}"
    );
    let output = timeout(Duration::from_secs(9), async {
        Command::new(curl_program())
            .args([
                "--disable",
                "--silent",
                "--show-error",
                "--fail",
                "--noproxy",
                "*",
                "--connect-timeout",
                "3",
                "--max-time",
                "7",
                "--max-filesize",
                "32768",
                &url,
            ])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .output()
            .await
    })
    .await;
    match output {
        Ok(Ok(result)) if result.status.success() && result.stdout.len() <= 32_768 => {
            verify_asn_hint(hint, &result.stdout)
        }
        _ => AsnVerification::Unavailable,
    }
}

async fn probe_padded_head(
    target: &FreezeTarget,
    address: Ipv4Addr,
    direct: bool,
    cancel: &AtomicBool,
) -> Result<PadProbeResult, String> {
    if cancel.load(Ordering::Acquire) {
        return Err("Проверка отменена".into());
    }
    let args = padded_head_curl_args(address, target.port, &target.sni)?;
    let probe = async move {
        let started = Instant::now();
        let result = timeout(Duration::from_secs(105), async {
            Command::new(curl_program())
                .args(&args)
                .stdin(std::process::Stdio::null())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::null())
                .kill_on_drop(true)
                .output()
                .await
        })
        .await;
        match result {
            Ok(Ok(output)) => Ok(classify_padded_head(
                output.status.code().unwrap_or(-1),
                &String::from_utf8_lossy(&output.stdout),
            )),
            Ok(Err(error)) => Err(format!("Не удалось запустить curl: {error}")),
            Err(_) => Ok(classify_padded_head(28, "")),
        }
        .map(|mut result| {
            result.elapsed_ms = started.elapsed().as_millis() as u64;
            result
        })
    };
    let result = if direct {
        crate::site_trace::bypass_probe(
            address,
            target.port,
            crate::site_trace::ProbeTransport::Tcp,
            probe,
        )
        .await?
    } else {
        probe.await
    };
    let result = result?;
    if cancel.load(Ordering::Acquire) {
        return Err("Проверка отменена".into());
    }
    Ok(result)
}

async fn measure_target(
    target: &FreezeTarget,
    address: Ipv4Addr,
    direct: bool,
    cancel: &AtomicBool,
) -> Result<FreezeMeasurement, String> {
    if target.kind == FreezeTargetKind::PinnedAddress {
        let result = probe_padded_head(target, address, direct, cancel).await?;
        return Ok(FreezeMeasurement {
            classification: format!("{:?}", result.classification),
            ok: result.classification == PadClassification::Complete,
            bytes_received: None,
            replies: Some(result.replies),
            elapsed_ms: result.elapsed_ms,
            reason: result.reason,
        });
    }
    let mut attempts = Vec::with_capacity(2);
    let mut expected_bytes = None;
    for _ in 0..2 {
        let parsed = crate::transfer_diagnostic::probe_transfer_at_path(
            &target.host,
            address,
            &target.path,
            direct,
            cancel,
        )
        .await?;
        expected_bytes = parsed.expected_bytes.or(expected_bytes);
        attempts.push(parsed.attempt);
    }
    let diagnosis = classify_transfer(&attempts, expected_bytes);
    let elapsed_ms = attempts.iter().map(|attempt| attempt.elapsed_ms).sum();
    let bytes_received = attempts.iter().map(|attempt| attempt.bytes_received).min();
    Ok(FreezeMeasurement {
        classification: format!("{:?}", diagnosis.classification),
        ok: crate::transfer_diagnostic::successful_http_response(&attempts),
        bytes_received,
        replies: None,
        elapsed_ms,
        reason: diagnosis.summary,
    })
}

/// Tests direct and isolated current-route baselines, then tries the upstream
/// fake-SNI templates against affected networks. The main nfqws2 process and
/// its config are not modified.
pub async fn scan_network_baselines(
    extra_hosts: &[String],
    cancel: Arc<AtomicBool>,
    active_args: Vec<String>,
    binary: String,
    functions: HashSet<String>,
    blobs: Vec<String>,
    mut progress: impl FnMut(usize, usize, &str, &[FreezeNetworkResult]) + Send,
) -> Result<FreezeScanOutcome, String> {
    let targets = build_targets(extra_hosts)?;
    let total = targets.len();
    let mut results = Vec::with_capacity(total);
    let started = Instant::now();
    let mut limited = false;
    let mut baseline_done = 0;
    for (index, target) in targets.iter().enumerate() {
        if cancel.load(Ordering::Acquire) {
            return Ok(FreezeScanOutcome {
                results,
                done: baseline_done,
                total,
                limited,
            });
        }
        if started.elapsed() > Duration::from_secs(900) {
            limited = true;
            break;
        }
        progress(index, total, &target.label, &results);
        let Some(address) = resolve_address(target).await else {
            let unavailable = FreezeMeasurement {
                classification: "dns_failure".into(),
                ok: false,
                bytes_received: None,
                replies: None,
                elapsed_ms: 0,
                reason: "Имя не разрешилось в публичный IPv4-адрес".into(),
            };
            results.push(FreezeNetworkResult {
                label: target.label.clone(),
                host: target.host.clone(),
                path: target.path.clone(),
                address: None,
                asn_hint: target.asn_hint,
                asn: AsnVerification::Unavailable,
                direct: unavailable.clone(),
                current: unavailable,
                freeze_detected: false,
                tried_candidates: 0,
                found_strategy: None,
                candidate_attempts: Vec::new(),
            });
            baseline_done = index + 1;
            progress(baseline_done, total, &target.label, &results);
            continue;
        };
        let pending = FreezeMeasurement {
            classification: "pending".into(),
            ok: false,
            bytes_received: None,
            replies: None,
            elapsed_ms: 0,
            reason: "Проверка выполняется".into(),
        };
        results.push(FreezeNetworkResult {
            label: target.label.clone(),
            host: target.host.clone(),
            path: target.path.clone(),
            address: Some(address),
            asn_hint: target.asn_hint,
            asn: AsnVerification::Unavailable,
            direct: pending.clone(),
            current: pending,
            freeze_detected: false,
            tried_candidates: 0,
            found_strategy: None,
            candidate_attempts: Vec::new(),
        });
        let result_index = results.len() - 1;
        progress(index, total, &target.label, &results);
        let asn_hint = target.asn_hint;
        let asn_task = tokio::spawn(async move { ripe_asn(address, asn_hint).await });
        let direct = match measure_target(target, address, true, &cancel).await {
            Ok(direct) => direct,
            Err(_) if cancel.load(Ordering::Acquire) => {
                return Ok(FreezeScanOutcome { results, done: baseline_done, total, limited });
            }
            Err(_) => return Err("Прямая сетевая проба завершилась ошибкой; проверьте временные firewall-правила и повторите".into()),
        };
        if cancel.load(Ordering::Acquire) {
            return Ok(FreezeScanOutcome {
                results,
                done: baseline_done,
                total,
                limited,
            });
        }
        results[result_index].direct = direct.clone();
        progress(
            index,
            total,
            &format!("{}: прямой запрос завершён", target.label),
            &results,
        );
        results[result_index].current.reason = "Проверяется текущий маршрут".into();
        progress(
            index,
            total,
            &format!("{}: проверка через текущий маршрут", target.label),
            &results,
        );
        let traced = if target.kind == FreezeTargetKind::PinnedAddress {
            crate::site_trace::run_padded_target_with_baseline(
                target.host.clone(),
                address,
                active_args.clone(),
                binary.clone(),
                direct.ok,
                cancel.clone(),
            )
            .await
        } else {
            crate::site_trace::run_transfer_target_with_baseline(
                target.host.clone(),
                address,
                target.path.clone(),
                active_args.clone(),
                binary.clone(),
                direct.ok,
                cancel.clone(),
            )
            .await
        };
        let asn = asn_task.await.unwrap_or(AsnVerification::Unavailable);
        let phase = traced.with_nfqws2;
        let classification = if phase.ok == Some(true) {
            "Complete"
        } else if phase.reason.contains("отправке тестового тела") {
            "PossibleUploadStall"
        } else if phase.reason.contains("16–20 КБ") || phase.reason.contains("после начала ответов")
        {
            if target.kind == FreezeTargetKind::PinnedAddress {
                "PossibleFreeze"
            } else {
                "PossibleBoundaryStall"
            }
        } else if phase.reason.to_lowercase().contains("тайм-аут") {
            "TimeoutBeforeBody"
        } else if phase.state != "complete" {
            "route_unavailable"
        } else {
            "Inconclusive"
        };
        let current = FreezeMeasurement {
            classification: classification.into(),
            ok: phase.ok == Some(true),
            bytes_received: None,
            replies: None,
            elapsed_ms: phase.duration_ms.unwrap_or_default(),
            reason: phase.reason,
        };
        let freeze_detected = is_freeze_classification(&direct.classification)
            || is_freeze_classification(&current.classification);
        results[result_index].asn = asn;
        results[result_index].current = current;
        results[result_index].freeze_detected = freeze_detected;
        baseline_done = index + 1;
        progress(baseline_done, total, &target.label, &results);
    }
    let global_started = started;
    let candidates = freeze_candidates(&functions, &blobs);
    let affected_indices = results
        .iter()
        .enumerate()
        .filter_map(|(index, result)| result.freeze_detected.then_some(index))
        .collect::<Vec<_>>();
    let candidate_total = total + affected_indices.len();
    if results.len() < total {
        return Ok(FreezeScanOutcome {
            results,
            done: baseline_done,
            total,
            limited,
        });
    }
    let mut candidate_done = 0;
    for (affected_index, index) in affected_indices.into_iter().enumerate() {
        if cancel.load(Ordering::Acquire) {
            break;
        }
        if global_started.elapsed() > Duration::from_secs(900) {
            limited = true;
            break;
        }
        if let AsnVerification::Verified { asn } = results[index].asn
            && let Some(source) = results[..index].iter().find(|previous| {
                previous.asn == AsnVerification::Verified { asn }
                    && previous.found_strategy.is_some()
            })
            && let Some(found) = source.found_strategy.clone()
        {
            results[index].found_strategy = Some(FreezeStrategyMatch {
                reused_from: Some(source.label.clone()),
                ..found
            });
            progress(
                total + affected_index + 1,
                candidate_total,
                &format!("{}: найден вариант в той же AS", results[index].label),
                &results,
            );
            candidate_done = affected_index + 1;
            continue;
        }
        let network_started = Instant::now();
        for (sni, step) in &candidates {
            if cancel.load(Ordering::Acquire) {
                break;
            }
            if global_started.elapsed() > Duration::from_secs(900)
                || network_started.elapsed() > Duration::from_secs(150)
            {
                limited = true;
                break;
            }
            let index_attempt = results[index].tried_candidates;
            progress(
                total + affected_index,
                candidate_total,
                &format!(
                    "{}: проверка SNI {sni} ({})",
                    results[index].label,
                    index_attempt + 1
                ),
                &results,
            );
            let steps = vec![step.clone()];
            let candidate_args = match crate::strategy_picker::candidate_argv(
                &active_args,
                crate::strategy_picker::StrategyProtocol::Https,
                &steps,
                &functions,
                &blobs,
            ) {
                Ok(args) => args,
                Err(_) => continue,
            };
            let candidate_args = std::iter::once(binary.clone())
                .chain(candidate_args)
                .collect::<Vec<_>>();
            let mut successes = 0;
            let mut attempts = 0;
            let mut elapsed = 0u64;
            let mut consecutive = true;
            let mut reason = None;
            for success_index in 0..3 {
                if cancel.load(Ordering::Acquire) {
                    consecutive = false;
                    break;
                }
                if global_started.elapsed() > Duration::from_secs(900)
                    || network_started.elapsed() > Duration::from_secs(150)
                {
                    limited = true;
                    consecutive = false;
                    break;
                }
                let report = if results[index].asn_hint.is_some() && results[index].path == "/" {
                    let address = results[index].address.expect("address was resolved above");
                    crate::site_trace::run_padded_target_with_baseline(
                        results[index].host.clone(),
                        address,
                        candidate_args.clone(),
                        binary.clone(),
                        results[index].direct.ok,
                        cancel.clone(),
                    )
                    .await
                } else {
                    let address = results[index].address.expect("address was resolved above");
                    crate::site_trace::run_transfer_target_with_baseline(
                        results[index].host.clone(),
                        address,
                        results[index].path.clone(),
                        candidate_args.clone(),
                        binary.clone(),
                        results[index].direct.ok,
                        cancel.clone(),
                    )
                    .await
                };
                attempts += 1;
                reason = Some(report.with_nfqws2.reason.clone());
                let success = candidate_route_succeeded(report.with_nfqws2.ok);
                elapsed += report.with_nfqws2.duration_ms.unwrap_or_default();
                if success {
                    successes += 1;
                    continue;
                }
                if success_index == 0 {
                    tokio::select! {
                        _ = tokio::time::sleep(Duration::from_secs(3)) => {},
                        _ = async {
                            while !cancel.load(Ordering::Acquire) {
                                tokio::time::sleep(Duration::from_millis(50)).await;
                            }
                        } => {
                            consecutive = false;
                            break;
                        }
                    }
                    if cancel.load(Ordering::Acquire)
                        || network_started.elapsed() > Duration::from_secs(150)
                    {
                        consecutive = false;
                        break;
                    }
                    let retry = if results[index].asn_hint.is_some() && results[index].path == "/" {
                        let address = results[index].address.expect("address was resolved above");
                        crate::site_trace::run_padded_target_with_baseline(
                            results[index].host.clone(),
                            address,
                            candidate_args.clone(),
                            binary.clone(),
                            results[index].direct.ok,
                            cancel.clone(),
                        )
                        .await
                    } else {
                        let address = results[index].address.expect("address was resolved above");
                        crate::site_trace::run_transfer_target_with_baseline(
                            results[index].host.clone(),
                            address,
                            results[index].path.clone(),
                            candidate_args.clone(),
                            binary.clone(),
                            results[index].direct.ok,
                            cancel.clone(),
                        )
                        .await
                    };
                    attempts += 1;
                    reason = Some(retry.with_nfqws2.reason.clone());
                    let retry_ok = candidate_route_succeeded(retry.with_nfqws2.ok);
                    elapsed += retry.with_nfqws2.duration_ms.unwrap_or_default();
                    if retry_ok {
                        successes += 1;
                        continue;
                    }
                }
                consecutive = false;
                break;
            }
            results[index].tried_candidates += 1;
            let confirmed = consecutive && successes == 3;
            let candidate_attempt = FreezeCandidateAttempt {
                sni: sni.clone(),
                step: step.clone(),
                attempts,
                successes,
                average_ms: (attempts > 0).then_some(elapsed / attempts.max(1) as u64),
                confirmed,
                reason: if confirmed {
                    None
                } else {
                    reason.map(|value| value.chars().take(256).collect())
                },
            };
            results[index]
                .candidate_attempts
                .push(candidate_attempt.clone());
            if confirmed {
                results[index].found_strategy = Some(FreezeStrategyMatch {
                    sni: sni.clone(),
                    step: step.clone(),
                    attempts: candidate_attempt.attempts,
                    successes: candidate_attempt.successes,
                    average_ms: candidate_attempt.average_ms,
                    reused_from: None,
                });
                break;
            }
        }
        progress(
            total + affected_index + 1,
            candidate_total,
            &format!("{}: подбор завершён", results[index].label),
            &results,
        );
        candidate_done = affected_index + 1;
        if global_started.elapsed() > Duration::from_secs(900) {
            limited = true;
            break;
        }
    }
    Ok(FreezeScanOutcome {
        results,
        done: total + candidate_done,
        total: candidate_total,
        limited,
    })
}
