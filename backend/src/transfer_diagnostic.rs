//! Conservative classification of bounded HTTP transfer samples.
use serde::{Deserialize, Serialize};

pub const BOUNDARY_MIN_BYTES: u64 = 16 * 1024;
pub const BOUNDARY_MAX_BYTES: u64 = 20 * 1024;
use crate::{
    site_diagnostics::{normalize_host, public_ip},
    site_trace::{ProbeTransport, bypass_probe},
};
use std::{
    net::{IpAddr, Ipv4Addr},
    path::PathBuf,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};
use tokio::{net::lookup_host, process::Command, time::timeout};

pub const TRANSFER_SAMPLE_BYTES: u64 = 65_536;
pub const SMALL_RESPONSE_BYTES: u64 = 24_576;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParsedTransferAttempt {
    pub attempt: TransferAttempt,
    pub expected_bytes: Option<u64>,
}

pub fn assemble_curl_args(
    host: &str,
    ip: IpAddr,
    body_path: &str,
    headers_path: &str,
) -> Vec<String> {
    assemble_curl_args_for_path(host, ip, "/", body_path, headers_path)
}

pub fn assemble_curl_args_for_path(
    host: &str,
    ip: IpAddr,
    request_path: &str,
    body_path: &str,
    headers_path: &str,
) -> Vec<String> {
    let address = match ip {
        IpAddr::V4(ip) => ip.to_string(),
        IpAddr::V6(ip) => format!("[{ip}]"),
    };
    [
        "--disable".to_string(),
        "--silent".into(),
        "--show-error".into(),
        "--insecure".into(),
        "--noproxy".into(),
        "*".into(),
        "--proto".into(),
        "=https".into(),
        "--connect-timeout".into(),
        "4".into(),
        "--max-time".into(),
        "8".into(),
        "--max-filesize".into(),
        "262144".into(),
        "--max-redirs".into(),
        "0".into(),
        "--local-port".into(),
        "40000-40999".into(),
        "--range".into(),
        "0-65535".into(),
        "--resolve".into(),
        format!("{host}:443:{address}"),
        "--output".into(),
        body_path.into(),
        "--dump-header".into(),
        headers_path.into(),
        "--write-out".into(),
        "NFQWS2_TRANSFER|%{http_code}|%{time_total}|%{time_starttransfer}|%{size_download}".into(),
        format!("https://{host}{request_path}"),
    ]
    .into_iter()
    .collect()
}

pub fn upload_stall_confirmed(exit_code: i32, metadata: &str, marker: &str) -> bool {
    exit_code == 28
        && metadata
            .rsplit_once(&format!("{marker}|"))
            .and_then(|(_, value)| value.trim().parse::<f64>().ok())
            .is_some_and(|appconnect| appconnect.is_finite() && appconnect > 0.0)
}

pub fn parse_curl_attempt(
    exit_code: i32,
    metadata: &[u8],
    body_bytes: u64,
    headers: &[u8],
    elapsed_ms: u64,
) -> ParsedTransferAttempt {
    let text = String::from_utf8_lossy(metadata);
    let fields = text
        .rsplit_once("NFQWS2_TRANSFER|")
        .map(|(_, tail)| tail.trim().split('|').collect::<Vec<_>>())
        .unwrap_or_default();
    let status = fields
        .first()
        .and_then(|field| field.parse::<u16>().ok())
        .filter(|status| (100..600).contains(status));
    let time_ms = |index: usize| {
        fields
            .get(index)
            .and_then(|field| field.parse::<f64>().ok())
            .filter(|time| time.is_finite() && *time >= 0.0 && *time <= 60.0)
            .map(|time| (time * 1000.0).round() as u64)
    };
    let header_text = String::from_utf8_lossy(headers);
    let last_response = header_text
        .split("\r\n\r\n")
        .filter(|block| !block.trim().is_empty())
        .last()
        .unwrap_or_default();
    let content_range_total = last_response.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("content-range")
            .then(|| {
                value
                    .trim()
                    .rsplit('/')
                    .next()
                    .unwrap_or_default()
                    .parse::<u64>()
                    .ok()
            })
            .flatten()
    });
    let content_length = last_response.lines().find_map(|line| {
        let (name, value) = line.split_once(':')?;
        name.eq_ignore_ascii_case("content-length")
            .then(|| value.trim().parse::<u64>().ok())
            .flatten()
    });
    let expected_bytes = content_range_total
        .or(content_length)
        .map(|size| size.min(TRANSFER_SAMPLE_BYTES));
    let transport_error = match exit_code {
        0 | 63 => None,
        6 => Some("dns".to_string()),
        7 => Some("connect".to_string()),
        28 => Some("timeout".to_string()),
        18 => Some("partial".to_string()),
        35 | 60 => Some("tls".to_string()),
        52 | 56 => Some("reset".to_string()),
        code => Some(format!("curl_exit_{code}")),
    };
    ParsedTransferAttempt {
        attempt: TransferAttempt {
            bytes_received: body_bytes.min(TRANSFER_SAMPLE_BYTES),
            sample_bytes: TRANSFER_SAMPLE_BYTES,
            http_status: status,
            elapsed_ms,
            first_byte_ms: time_ms(2),
            transport_error,
            upload_stalled_after_tls: false,
        },
        expected_bytes,
    }
}
const REPEAT_TOLERANCE_BYTES: u64 = 2 * 1024;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TransferClass {
    Complete,
    PossibleBoundaryStall,
    PossibleUploadStall,
    TimeoutBeforeBody,
    TimeoutAfterBody,
    ConnectionReset,
    PartialTransfer,
    Inconclusive,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferAttempt {
    pub bytes_received: u64,
    pub sample_bytes: u64,
    pub http_status: Option<u16>,
    pub elapsed_ms: u64,
    pub first_byte_ms: Option<u64>,
    pub transport_error: Option<String>,
    #[serde(default)]
    pub upload_stalled_after_tls: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferDiagnosis {
    pub classification: TransferClass,
    pub attempts: usize,
    pub min_bytes: Option<u64>,
    pub max_bytes: Option<u64>,
    pub repeatable_boundary_stall: bool,
    pub summary: String,
    pub limitations: Vec<String>,
}

fn error_is(attempt: &TransferAttempt, value: &str) -> bool {
    attempt
        .transport_error
        .as_deref()
        .is_some_and(|error| error.eq_ignore_ascii_case(value))
}

pub fn classify_transfer(
    attempts: &[TransferAttempt],
    expected_bytes: Option<u64>,
) -> TransferDiagnosis {
    let sizes = attempts
        .iter()
        .map(|attempt| attempt.bytes_received)
        .collect::<Vec<_>>();
    let min_bytes = sizes.iter().min().copied();
    let max_bytes = sizes.iter().max().copied();
    let successful_complete = attempts.iter().any(|attempt| {
        attempt.transport_error.is_none()
            && attempt
                .http_status
                .is_some_and(|status| (200..400).contains(&status))
            && (attempt.bytes_received >= attempt.sample_bytes
                || expected_bytes.is_some_and(|expected| attempt.bytes_received >= expected))
    });
    let timed_out_at_boundary = attempts
        .iter()
        .filter(|attempt| {
            error_is(attempt, "timeout")
                && (BOUNDARY_MIN_BYTES..=BOUNDARY_MAX_BYTES).contains(&attempt.bytes_received)
                && expected_bytes.is_none_or(|expected| expected > attempt.bytes_received)
        })
        .map(|attempt| attempt.bytes_received)
        .collect::<Vec<_>>();
    let repeatable_boundary_stall = timed_out_at_boundary.len() >= 2
        && timed_out_at_boundary.iter().max().unwrap()
            - timed_out_at_boundary.iter().min().unwrap()
            <= REPEAT_TOLERANCE_BYTES;
    let upload_stall = attempts
        .iter()
        .any(|attempt| attempt.upload_stalled_after_tls);

    let classification = if successful_complete && !upload_stall {
        TransferClass::Complete
    } else if upload_stall {
        TransferClass::PossibleUploadStall
    } else if repeatable_boundary_stall {
        TransferClass::PossibleBoundaryStall
    } else if attempts.is_empty() {
        TransferClass::Inconclusive
    } else if attempts.len() >= 2
        && attempts
            .iter()
            .all(|attempt| error_is(attempt, "timeout") && attempt.bytes_received == 0)
    {
        TransferClass::TimeoutBeforeBody
    } else if attempts.iter().any(|attempt| error_is(attempt, "reset")) {
        TransferClass::ConnectionReset
    } else if attempts
        .iter()
        .any(|attempt| error_is(attempt, "partial") && attempt.bytes_received > 0)
    {
        TransferClass::PartialTransfer
    } else if attempts.len() >= 2
        && attempts.iter().all(|attempt| error_is(attempt, "timeout"))
        && min_bytes.is_some_and(|min| min > 0)
        && max_bytes.unwrap() - min_bytes.unwrap() <= REPEAT_TOLERANCE_BYTES
    {
        TransferClass::TimeoutAfterBody
    } else if attempts
        .iter()
        .any(|attempt| attempt.transport_error.is_some())
    {
        TransferClass::Inconclusive
    } else {
        TransferClass::PartialTransfer
    };

    let summary = match classification {
        TransferClass::Complete => "Передача завершилась; повторяемый обрыв не обнаружен".into(),
        TransferClass::PossibleBoundaryStall => "Передача повторно остановилась около 16–20 КБ; это указывает на возможный сетевой обрыв, но не доказывает DPI".into(),
        TransferClass::PossibleUploadStall => "После TLS handshake истёк тайм-аут при отправке тестового тела; возможна остановка передачи, причина не установлена".into(),
        TransferClass::TimeoutBeforeBody => "Тайм-аут до получения тела ответа; обрыв на 16–20 КБ не подтверждён".into(),
        TransferClass::TimeoutAfterBody => "Передача завершилась по тайм-ауту, но точка обрыва не повторяется в диапазоне 16–20 КБ".into(),
        TransferClass::ConnectionReset => "Соединение было сброшено; причина и связь с объёмом передачи не установлены".into(),
        TransferClass::PartialTransfer => "Получена неполная передача без подтверждённого повторяемого тайм-аута".into(),
        TransferClass::Inconclusive => "Результатов недостаточно для классификации".into(),
    };
    TransferDiagnosis {
        classification,
        attempts: attempts.len(),
        min_bytes,
        max_bytes,
        repeatable_boundary_stall,
        summary,
        limitations: vec!["Результат относится только к выполненным запросам и не устанавливает причину сетевого поведения.".into()],
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferSideResult {
    pub attempts: Vec<TransferAttempt>,
    pub diagnosis: TransferDiagnosis,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TransferComparison {
    pub host: String,
    pub address: String,
    pub direct: TransferSideResult,
    pub current: TransferSideResult,
}

struct ScratchDir(PathBuf);
impl Drop for ScratchDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

async fn curl_attempt(host: &str, ip: Ipv4Addr, request_path: &str) -> ParsedTransferAttempt {
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|time| time.as_nanos())
        .unwrap_or_default()
        ^ u128::from(NEXT.fetch_add(1, Ordering::Relaxed));
    let dir =
        std::env::temp_dir().join(format!("nfqws2-ui-transfer-{}-{nonce}", std::process::id()));
    if tokio::fs::create_dir(&dir).await.is_err() {
        return parse_curl_attempt(-1, &[], 0, &[], 0);
    }
    let _scratch = ScratchDir(dir.clone());
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if tokio::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700))
            .await
            .is_err()
        {
            return parse_curl_attempt(-1, &[], 0, &[], 0);
        }
    }
    let body = dir.join("body");
    let headers = dir.join("headers");
    let (Some(body), Some(headers)) = (body.to_str(), headers.to_str()) else {
        return parse_curl_attempt(-1, &[], 0, &[], 0);
    };
    let args = assemble_curl_args_for_path(host, IpAddr::V4(ip), request_path, body, headers);
    let program = if std::path::Path::new("/opt/bin/curl").is_file() {
        "/opt/bin/curl"
    } else if cfg!(windows) {
        "curl.exe"
    } else {
        "curl"
    };
    let started = Instant::now();
    let result = timeout(Duration::from_secs(10), async {
        Command::new(program)
            .args(&args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .output()
            .await
    })
    .await;
    let (code, stdout) = match result {
        Ok(Ok(output)) => (output.status.code().unwrap_or(-1), output.stdout),
        Ok(Err(_)) => (-1, Vec::new()),
        Err(_) => (28, Vec::new()),
    };
    let body_bytes = tokio::fs::metadata(dir.join("body"))
        .await
        .map(|metadata| metadata.len())
        .unwrap_or(0);
    let header_bytes = tokio::fs::read(dir.join("headers"))
        .await
        .unwrap_or_default();
    let mut parsed = parse_curl_attempt(
        code,
        &stdout,
        body_bytes,
        &header_bytes,
        started.elapsed().as_millis() as u64,
    );
    if parsed.attempt.transport_error.is_none()
        && parsed.attempt.bytes_received < SMALL_RESPONSE_BYTES
        && parsed
            .attempt
            .http_status
            .is_some_and(|status| (200..400).contains(&status))
    {
        parsed.attempt.upload_stalled_after_tls =
            upload_stalled_after_tls(host, ip, request_path, &dir).await;
    }
    parsed
}

async fn upload_stalled_after_tls(
    host: &str,
    ip: Ipv4Addr,
    request_path: &str,
    directory: &std::path::Path,
) -> bool {
    let payload_path = directory.join("upload-body");
    let mut payload = vec![0u8; TRANSFER_SAMPLE_BYTES as usize];
    if getrandom::fill(&mut payload).is_err()
        || tokio::fs::write(&payload_path, payload).await.is_err()
    {
        return false;
    }
    let Some(payload_path) = payload_path.to_str() else {
        return false;
    };
    let nonce = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_nanos())
        .unwrap_or_default();
    let marker = format!("nfqws2-ui-upload-{nonce}");
    let args = [
        "--disable".to_owned(),
        "--silent".into(),
        "--show-error".into(),
        "--insecure".into(),
        "--noproxy".into(),
        "*".into(),
        "--proto".into(),
        "=https".into(),
        "--connect-timeout".into(),
        "5".into(),
        "--max-time".into(),
        "8".into(),
        "--max-filesize".into(),
        "262144".into(),
        "--max-redirs".into(),
        "0".into(),
        "--local-port".into(),
        "40000-40999".into(),
        "--resolve".into(),
        format!("{host}:443:{ip}"),
        "-H".into(),
        "Expect:".into(),
        "-H".into(),
        "Content-Type: application/octet-stream".into(),
        "--data-binary".into(),
        format!("@{payload_path}"),
        "--write-out".into(),
        format!("{marker}|%{{time_appconnect}}"),
        format!(
            "https://{host}{request_path}{}t={nonce}",
            if request_path.contains('?') { "&" } else { "?" }
        ),
    ];
    let program = if std::path::Path::new("/opt/bin/curl").is_file() {
        "/opt/bin/curl"
    } else if cfg!(windows) {
        "curl.exe"
    } else {
        "curl"
    };
    let result = timeout(Duration::from_secs(10), async {
        Command::new(program)
            .args(args)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::null())
            .kill_on_drop(true)
            .output()
            .await
    })
    .await;
    let Ok(Ok(output)) = result else {
        return false;
    };
    let exit_code = output.status.code().unwrap_or(-1);
    let metadata = String::from_utf8_lossy(&output.stdout);
    upload_stall_confirmed(exit_code, &metadata, &marker)
}

async fn resolve_public_ipv4(host: &str) -> Result<Ipv4Addr, String> {
    let values = timeout(Duration::from_secs(5), lookup_host((host, 443)))
        .await
        .map_err(|_| "Тайм-аут DNS".to_string())?
        .map_err(|_| "Не удалось разрешить домен".to_string())?;
    values
        .filter_map(|address| match address.ip() {
            IpAddr::V4(ip) if public_ip(IpAddr::V4(ip)) => Some(ip),
            _ => None,
        })
        .next()
        .ok_or_else(|| "DNS не вернул публичный IPv4-адрес".into())
}

/// Performs one bounded HTTPS byte-volume probe. The direct variant is scoped
/// through the existing temporary bypass rules; no primary nfqws2 config changes.
pub async fn probe_transfer_once(
    host: &str,
    direct: bool,
    cancel: &AtomicBool,
) -> Result<(Ipv4Addr, ParsedTransferAttempt), String> {
    if cancel.load(Ordering::Acquire) {
        return Err("Проверка отменена".into());
    }
    let host = normalize_host(host)?;
    let address = resolve_public_ipv4(&host).await?;
    let attempt = probe_transfer_at(&host, address, direct, cancel).await?;
    Ok((address, attempt))
}

pub async fn probe_transfer_at(
    host: &str,
    address: Ipv4Addr,
    direct: bool,
    cancel: &AtomicBool,
) -> Result<ParsedTransferAttempt, String> {
    probe_transfer_at_path(host, address, "/", direct, cancel).await
}

pub async fn probe_transfer_at_path(
    host: &str,
    address: Ipv4Addr,
    request_path: &str,
    direct: bool,
    cancel: &AtomicBool,
) -> Result<ParsedTransferAttempt, String> {
    if cancel.load(Ordering::Acquire) || !public_ip(IpAddr::V4(address)) {
        return Err("Проверка отменена или адрес не является публичным".into());
    }
    let host = normalize_host(host)?;
    if !request_path.starts_with('/')
        || request_path.len() > 512
        || request_path
            .bytes()
            .any(|byte| byte.is_ascii_control() || byte.is_ascii_whitespace() || byte == b'\\')
    {
        return Err("Путь проверки должен быть безопасным относительным URL-путём".into());
    }
    let probe = curl_attempt(&host, address, request_path);
    let attempt = if direct {
        bypass_probe(address, 443, ProbeTransport::Tcp, probe).await?
    } else {
        probe.await
    };
    if cancel.load(Ordering::Acquire) {
        return Err("Проверка отменена".into());
    }
    Ok(attempt)
}

pub async fn compare_transfer(
    host: String,
    cancel: Arc<AtomicBool>,
    mut progress: impl FnMut(&str, usize) + Send,
) -> Result<TransferComparison, String> {
    let host = normalize_host(&host)?;
    let mut direct = Vec::new();
    let mut current = Vec::new();
    let address = resolve_public_ipv4(&host).await?;
    let mut expected_direct = None;
    let mut expected_current = None;
    for (label, is_direct, attempts, expected) in [
        ("direct", true, &mut direct, &mut expected_direct),
        ("current", false, &mut current, &mut expected_current),
    ] {
        for index in 0..2 {
            if cancel.load(Ordering::Acquire) {
                return Err("Проверка отменена".into());
            }
            progress(label, index + 1);
            let parsed = probe_transfer_at(&host, address, is_direct, &cancel).await?;
            *expected = parsed.expected_bytes.or(*expected);
            attempts.push(parsed.attempt);
        }
    }
    let direct_diagnosis = classify_transfer(&direct, expected_direct);
    let current_diagnosis = classify_transfer(&current, expected_current);
    Ok(TransferComparison {
        host,
        address: address.to_string(),
        direct: TransferSideResult {
            attempts: direct,
            diagnosis: direct_diagnosis,
        },
        current: TransferSideResult {
            attempts: current,
            diagnosis: current_diagnosis,
        },
    })
}
