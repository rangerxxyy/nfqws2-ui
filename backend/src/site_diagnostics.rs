//! Bounded router-side probes, comparing an isolated bypass and the current route.

use crate::{
    quic_probe,
    site_trace::{ProbeTransport, bypass_probe},
};
use serde::Serialize;
use std::{
    net::{IpAddr, SocketAddr},
    path::Path,
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::{TcpStream, lookup_host},
    process::Command,
    time::timeout,
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticStep {
    pub id: String,
    pub status: &'static str,
    pub detail: String,
    pub duration_ms: Option<u64>,
    pub http_code: Option<u16>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorProbeResult {
    pub summary: String,
    pub current: DiagnosticStep,
    pub steps: Vec<DiagnosticStep>,
    pub received_bytes: u64,
}

impl DiagnosticStep {
    pub(crate) fn new(id: &str, status: &'static str, detail: impl Into<String>) -> Self {
        Self {
            id: id.into(),
            status,
            detail: detail.into(),
            duration_ms: None,
            http_code: None,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    pub host: String,
    pub addresses: Vec<IpAddr>,
    pub reverse_dns: Vec<ReverseDnsRecord>,
    pub steps: Vec<DiagnosticStep>,
    pub summary: String,
    pub limitations: Vec<String>,
    pub direct: DiagnosticStep,
    pub current: DiagnosticStep,
    pub quic: Option<QuicComparison>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReverseDnsRecord {
    pub address: IpAddr,
    pub names: Vec<String>,
    pub resolver: Option<String>,
    pub detail: String,
}

fn parse_ptr_names(program: &str, output: &[u8]) -> Vec<String> {
    let program = Path::new(program)
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or(program);
    let text = String::from_utf8_lossy(output);
    let candidates = if program == "nslookup" {
        text.lines()
            .filter_map(|line| {
                line.trim()
                    .strip_prefix("Name:")
                    .map(str::trim)
                    .or_else(|| line.split_once("name =").map(|(_, name)| name.trim()))
            })
            .collect::<Vec<_>>()
    } else {
        text.lines().map(str::trim).collect::<Vec<_>>()
    };
    let mut names = Vec::new();
    for candidate in candidates {
        let name = candidate.trim_end_matches('.').to_ascii_lowercase();
        let valid = name.len() <= 253
            && !name.is_empty()
            && name.parse::<IpAddr>().is_err()
            && name.split('.').all(|label| {
                !label.is_empty()
                    && label.len() <= 63
                    && label.as_bytes()[0].is_ascii_alphanumeric()
                    && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
                    && label
                        .bytes()
                        .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-' || byte == b'_')
            });
        if valid && !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

fn available_reverse_dns_utilities() -> Vec<(&'static str, String)> {
    let mut directories = std::env::var_os("PATH")
        .map(|path| std::env::split_paths(&path).collect::<Vec<_>>())
        .unwrap_or_default();
    directories.extend(
        [
            "/opt/bin",
            "/opt/sbin",
            "/usr/bin",
            "/usr/sbin",
            "/bin",
            "/sbin",
        ]
        .into_iter()
        .map(Path::new)
        .map(Path::to_path_buf),
    );
    let mut utilities = Vec::new();
    for name in ["kdig", "dig", "nslookup"] {
        if let Some(path) = directories.iter().find_map(|directory| {
            let candidate = directory.join(name);
            if candidate.is_file() {
                Some(candidate)
            } else if cfg!(windows) {
                let candidate = directory.join(format!("{name}.exe"));
                candidate.is_file().then_some(candidate)
            } else {
                None
            }
        }) {
            utilities.push((name, path.to_string_lossy().into_owned()));
        }
    }
    utilities
}

async fn reverse_dns_for_ip(
    address: IpAddr,
    utilities: &[(&'static str, String)],
) -> ReverseDnsRecord {
    if utilities.is_empty() {
        return ReverseDnsRecord {
            address,
            names: Vec::new(),
            resolver: None,
            detail: "kdig, dig и nslookup не найдены".into(),
        };
    }
    let started = Instant::now();
    let mut last_error = "PTR-запись не получена".to_owned();
    let mut last_resolver = None;
    for (name, program) in utilities {
        if started.elapsed() >= Duration::from_secs(4) {
            last_error = "Истекло время обратного DNS-запроса".into();
            break;
        }
        last_resolver = Some((*name).to_owned());
        let args = if *name == "nslookup" {
            vec![address.to_string()]
        } else {
            vec!["-x".into(), address.to_string(), "+short".into()]
        };
        let remaining = Duration::from_secs(4).saturating_sub(started.elapsed());
        let seconds = remaining.as_secs().clamp(1, 2);
        match capture(program, &args, seconds, 4096).await {
            Ok((0, output)) => {
                let names = parse_ptr_names(name, &output);
                let detail = if names.is_empty() {
                    format!("PTR-запись не найдена ({name})")
                } else {
                    format!("Найдено обратным DNS ({name})")
                };
                return ReverseDnsRecord {
                    address,
                    names,
                    resolver: Some((*name).into()),
                    detail,
                };
            }
            Ok((code, output)) => {
                let names = parse_ptr_names(name, &output);
                if !names.is_empty() {
                    return ReverseDnsRecord {
                        address,
                        detail: format!("Найдено обратным DNS ({name})"),
                        names,
                        resolver: Some((*name).into()),
                    };
                }
                last_error = format!("{name} завершился с кодом {code}");
            }
            Err(message) => last_error = format!("{name}: {message}"),
        }
    }
    ReverseDnsRecord {
        address,
        names: Vec::new(),
        resolver: last_resolver,
        detail: last_error,
    }
}

async fn reverse_dns_records(addresses: &[IpAddr]) -> Vec<ReverseDnsRecord> {
    let utilities = available_reverse_dns_utilities();
    if utilities.is_empty() {
        return addresses
            .iter()
            .take(4)
            .copied()
            .map(|address| ReverseDnsRecord {
                address,
                names: Vec::new(),
                resolver: None,
                detail: "kdig, dig и nslookup не найдены".into(),
            })
            .collect();
    }
    let mut tasks = tokio::task::JoinSet::new();
    for address in addresses.iter().take(4).copied() {
        let utilities = utilities.clone();
        tasks.spawn(async move { reverse_dns_for_ip(address, &utilities).await });
    }
    let mut records = Vec::new();
    while let Some(Ok(record)) = tasks.join_next().await {
        records.push(record);
    }
    records.sort_by_key(|record| record.address);
    records
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct QuicComparison {
    pub direct: DiagnosticStep,
    pub current: DiagnosticStep,
    pub method: &'static str,
    pub http3_advertised: Option<bool>,
}

fn probe_open(step: &DiagnosticStep) -> bool {
    step.status == "ok" && step.http_code.is_none_or(|code| code < 400)
}

fn diagnosis_summary(direct: &DiagnosticStep, current: &DiagnosticStep) -> &'static str {
    if direct.status == "unavailable" || current.status == "unavailable" {
        return "Сравнение неполное: один из способов не проверен";
    }
    match (probe_open(direct), probe_open(current)) {
        (true, true) => "Сайт ответил напрямую и через текущий маршрут",
        (false, true) => "Сейчас сайт отвечает, напрямую успешного ответа нет",
        (true, false) => "Напрямую сайт отвечает, через текущий маршрут — нет",
        (false, false) => {
            "Сайт не открылся ни напрямую, ни через текущий маршрут; причина требует проверки"
        }
    }
}

pub fn list_entry_matches(host: &str, ips: &[IpAddr], line: &str, ip_list: bool) -> bool {
    let entry = line.split_whitespace().next().unwrap_or_default();
    if entry.is_empty() || entry.starts_with('#') {
        return false;
    }
    if !ip_list {
        let entry = entry
            .trim_start_matches("*.")
            .trim_end_matches('.')
            .to_ascii_lowercase();
        return host == entry || host.ends_with(&format!(".{entry}"));
    }
    let (network, prefix) = entry
        .split_once('/')
        .map_or((entry, None), |(network, prefix)| (network, Some(prefix)));
    let Ok(network) = network.parse::<IpAddr>() else {
        return false;
    };
    ips.iter().any(|ip| match (network, *ip) {
        (IpAddr::V4(network), IpAddr::V4(ip)) => {
            let Some(prefix) = prefix
                .map_or(Some(32), |prefix| prefix.parse::<u32>().ok())
                .filter(|prefix| *prefix <= 32)
            else {
                return false;
            };
            prefix == 0 || (u32::from(network) >> (32 - prefix)) == (u32::from(ip) >> (32 - prefix))
        }
        (IpAddr::V6(network), IpAddr::V6(ip)) => {
            let Some(prefix) = prefix
                .map_or(Some(128), |prefix| prefix.parse::<u32>().ok())
                .filter(|prefix| *prefix <= 128)
            else {
                return false;
            };
            prefix == 0
                || (u128::from(network) >> (128 - prefix)) == (u128::from(ip) >> (128 - prefix))
        }
        _ => false,
    })
}

pub fn normalize_host(input: &str) -> Result<String, String> {
    let input = input.trim();
    let target = input
        .strip_prefix("https://")
        .or_else(|| input.strip_prefix("http://"))
        .unwrap_or(input);
    let host = target
        .split(['/', '?', '#'])
        .next()
        .unwrap_or_default()
        .trim_end_matches('.')
        .to_ascii_lowercase();
    let valid = input.len() <= 2048
        && host.len() <= 253
        && host.contains('.')
        && host.parse::<IpAddr>().is_err()
        && ![".local", ".lan", ".localhost", ".internal"]
            .iter()
            .any(|suffix| host.ends_with(suffix))
        && host.split('.').all(|label| {
            !label.is_empty()
                && label.len() <= 63
                && !label.starts_with('-')
                && !label.ends_with('-')
                && label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        });
    if valid {
        Ok(host)
    } else {
        Err("Введите публичное доменное имя без порта и учётных данных (для IDN используйте punycode)".into())
    }
}

/// Accept a public DNS name or a public IPv4 literal for site diagnosis and
/// strategy picking. The isolated strategy runner currently uses IPv4 rules.
pub fn normalize_site_target(input: &str) -> Result<String, String> {
    if let Ok(ip) = input.trim().parse::<IpAddr>() {
        return match ip {
            IpAddr::V4(ip) if public_ip(ip.into()) => Ok(ip.to_string()),
            IpAddr::V4(_) => Err("Для проверки укажите публичный IPv4-адрес".into()),
            IpAddr::V6(_) => Err("Подбор и диагноз по IP пока поддерживают публичный IPv4".into()),
        };
    }
    normalize_host(input)
}

pub fn public_ip(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [a, b, c, _] = ip.octets();
            !(a == 0
                || a == 10
                || a == 127
                || a >= 224
                || (a == 100 && (64..=127).contains(&b))
                || (a == 169 && b == 254)
                || (a == 172 && (16..=31).contains(&b))
                || (a == 192 && (b == 168 || (b == 0 && (c == 0 || c == 2))))
                || (a == 198 && (b == 18 || b == 19 || (b == 51 && c == 100)))
                || (a == 203 && b == 0 && c == 113))
        }
        IpAddr::V6(ip) => {
            let segments = ip.segments();
            (segments[0] & 0xe000) == 0x2000
                && !(segments[0] == 0x2001 && (segments[1] == 0x0db8 || segments[1] < 0x0200))
        }
    }
}

pub async fn capture(
    program: &str,
    args: &[String],
    seconds: u64,
    limit: usize,
) -> Result<(i32, Vec<u8>), String> {
    let task = async {
        let mut child = Command::new(program)
            .args(args)
            .env("no_proxy", "*")
            .env("NO_PROXY", "*")
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| format!("Утилита {program} недоступна"))?;
        let mut bytes = Vec::new();
        child
            .stdout
            .take()
            .ok_or("Нет stdout")?
            .take((limit + 1) as u64)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| "Не удалось прочитать результат проверки")?;
        if bytes.len() > limit {
            return Err("Результат проверки превышает лимит".into());
        }
        let status = child
            .wait()
            .await
            .map_err(|_| "Проверочный процесс не завершился")?;
        Ok((status.code().unwrap_or(-1), bytes))
    };
    timeout(Duration::from_secs(seconds), task)
        .await
        .map_err(|_| "Истекло время проверки".to_string())?
}

pub fn curl_program() -> &'static str {
    if std::path::Path::new("/opt/bin/curl").is_file() {
        "/opt/bin/curl"
    } else if cfg!(windows) {
        "curl.exe"
    } else {
        "curl"
    }
}

fn curl_args() -> Vec<String> {
    // --disable must be first: do not inherit credentials/options from .curlrc.
    [
        "--disable",
        "--silent",
        "--noproxy",
        "*",
        "--proto",
        "=http,https",
        "--connect-timeout",
        "3",
        "--max-time",
        "8",
        "--max-filesize",
        "65536",
        "--max-redirs",
        "0",
        "--user-agent",
        "nfqws2-ui-diagnostic",
    ]
    .into_iter()
    .map(str::to_owned)
    .collect()
}

fn monitor_curl_args(host: &str, ip: std::net::Ipv4Addr) -> Vec<String> {
    let mut args = curl_args();
    args.extend([
        "--ipv4".into(),
        "--insecure".into(),
        "--connect-timeout".into(),
        "5".into(),
        "--user-agent".into(),
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130 Safari/537.36".into(),
        "--resolve".into(),
        format!("{host}:443:{ip}"),
        "--range".into(),
        "0-65535".into(),
        "--max-filesize".into(),
        "65536".into(),
        "--output".into(),
        if cfg!(windows) {
            "NUL".into()
        } else {
            "/dev/null".into()
        },
        "--write-out".into(),
        "\\nNFQWS2_MON|%{http_code}|%{time_connect}|%{time_appconnect}|%{time_starttransfer}|%{time_total}|%{size_download}".into(),
        format!("https://{host}/"),
    ]);
    args
}

fn monitor_ms(value: Option<&&str>) -> Option<u64> {
    value
        .and_then(|value| value.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0 && *value <= 60.0)
        .map(|value| (value * 1000.0).round() as u64)
}

const MONITOR_VOLUME_SMALL_BYTES: u64 = 24_576;
const MONITOR_VOLUME_TEST_BYTES: usize = 65_536;

fn monitor_should_test_upload(received_bytes: u64, status: &str) -> bool {
    status == "ok" && received_bytes < MONITOR_VOLUME_SMALL_BYTES
}

fn monitor_upload_stall_confirmed(exit_code: i32, appconnect_seconds: Option<f64>) -> bool {
    exit_code == 28
        && appconnect_seconds.is_some_and(|seconds| seconds.is_finite() && seconds > 0.0)
}

fn unavailable_monitor_result(detail: String) -> MonitorProbeResult {
    let steps = ["tcp", "tls", "http", "transfer"]
        .into_iter()
        .map(|id| DiagnosticStep::new(id, "unavailable", detail.clone()))
        .collect();
    MonitorProbeResult {
        summary: detail.clone(),
        current: DiagnosticStep::new("current", "unavailable", detail),
        steps,
        received_bytes: 0,
    }
}

fn parse_monitor_curl(exit_code: i32, stdout: &[u8]) -> MonitorProbeResult {
    let text = String::from_utf8_lossy(stdout);
    let fields = text
        .rsplit_once("\nNFQWS2_MON|")
        .map(|(_, tail)| tail.trim().split('|').collect::<Vec<_>>())
        .unwrap_or_default();
    let http_code = fields
        .first()
        .and_then(|field| field.parse::<u16>().ok())
        .filter(|code| (100..600).contains(code));
    let connect_ms = monitor_ms(fields.get(1));
    let tls_ms = monitor_ms(fields.get(2));
    let first_byte_ms = monitor_ms(fields.get(3));
    let total_ms = monitor_ms(fields.get(4));
    let received = fields
        .get(5)
        .and_then(|field| field.parse::<u64>().ok())
        .unwrap_or_default();
    let complete =
        http_code.is_some() && (exit_code == 0 || (exit_code == 63 && received >= 65_536));
    let after_response_timeout = http_code.is_some() && exit_code == 28;
    let tcp_connected = connect_ms.is_some_and(|ms| ms > 0) || tls_ms.is_some_and(|ms| ms > 0);
    let tls_connected = tls_ms.is_some_and(|ms| ms > 0);

    let mut steps = vec![DiagnosticStep {
        id: "tcp".into(),
        status: if tcp_connected { "ok" } else { "error" },
        detail: connect_ms.map_or_else(
            || "TCP-соединение не установлено".into(),
            |ms| format!("TCP connect: {ms} мс"),
        ),
        duration_ms: connect_ms,
        http_code: None,
    }];
    steps.push(DiagnosticStep {
        id: "tls".into(),
        status: if tls_connected {
            "ok"
        } else if tcp_connected {
            "error"
        } else {
            "unavailable"
        },
        detail: tls_ms.map_or_else(
            || {
                if exit_code == 35 {
                    "Ошибка TLS handshake".into()
                } else if tcp_connected {
                    "TLS handshake не завершён".into()
                } else {
                    "TLS не проверен: TCP не установлено".into()
                }
            },
            |ms| format!("TLS handshake: {ms} мс"),
        ),
        duration_ms: tls_ms,
        http_code: None,
    });
    steps.push(DiagnosticStep {
        id: "http".into(),
        status: if http_code.is_some() {
            "ok"
        } else if tls_connected {
            if exit_code == 28 { "warning" } else { "error" }
        } else {
            "unavailable"
        },
        detail: http_code.map_or_else(
            || {
                if tls_connected && exit_code == 28 {
                    "Тайм-аут после TLS до получения HTTP-ответа; причина не установлена".into()
                } else if tls_connected {
                    "HTTP-ответ не получен".into()
                } else {
                    "HTTP не проверен: TLS не установлен".into()
                }
            },
            |code| {
                format!(
                    "Получен HTTP {code}{}",
                    first_byte_ms.map_or_else(String::new, |ms| format!(" · первый байт {ms} мс"))
                )
            },
        ),
        duration_ms: first_byte_ms,
        http_code,
    });
    let transfer_status = if complete {
        "ok"
    } else if after_response_timeout || http_code.is_some() {
        "warning"
    } else {
        "unavailable"
    };
    let transfer_detail = if complete {
        format!(
            "Получено {received} байт{}",
            if received >= 65_536 {
                " · лимит 64 КиБ достигнут"
            } else {
                ""
            }
        )
    } else if after_response_timeout {
        format!(
            "Тайм-аут тела HTTP-ответа после {received} байт · возможный обрыв передачи, причина не доказана"
        )
    } else if http_code.is_some() {
        format!("Передача ответа завершилась с ошибкой после {received} байт")
    } else {
        "Передача тела не проверена: HTTP-ответ не получен".into()
    };
    steps.push(DiagnosticStep {
        id: "transfer".into(),
        status: transfer_status,
        detail: transfer_detail,
        duration_ms: total_ms,
        http_code,
    });

    let current_status = if complete {
        "ok"
    } else if after_response_timeout || (http_code.is_some() && exit_code != 0) {
        "warning"
    } else {
        "error"
    };
    let detail = if complete {
        format!(
            "HTTPS ответ HTTP {}, получено {received} байт за {} мс",
            http_code.unwrap_or_default(),
            total_ms.unwrap_or_default()
        )
    } else if after_response_timeout {
        format!(
            "HTTP {} получен, тело ответа зависло после {received} байт за {} мс; возможный обрыв передачи",
            http_code.unwrap_or_default(),
            total_ms.unwrap_or_default()
        )
    } else if tls_connected && exit_code == 28 {
        "Тайм-аут после TLS до HTTP-ответа; возможная проблема соединения, причина не доказана"
            .into()
    } else if exit_code == 35 {
        "TCP установлено, TLS handshake не завершён".into()
    } else if exit_code == 7 {
        "Не удалось подключиться по TCP/443".into()
    } else if exit_code == 6 {
        "Не удалось разрешить домен при HTTPS-проверке".into()
    } else if exit_code == 52 {
        "Сервер закрыл соединение без HTTP-ответа".into()
    } else if exit_code == 56 {
        "Соединение сброшено до завершения HTTPS-проверки".into()
    } else if exit_code == -1 {
        "HTTPS-проверка превысила внутренний лимит времени".into()
    } else {
        format!("HTTPS-проверка не завершена (curl {exit_code}); причина не установлена")
    };
    let current = DiagnosticStep {
        id: "current".into(),
        status: current_status,
        detail: detail.clone(),
        duration_ms: total_ms,
        http_code,
    };
    MonitorProbeResult {
        summary: detail,
        current,
        steps,
        received_bytes: received,
    }
}

async fn monitor_probe_with_program(
    program: &str,
    host: &str,
    ip: std::net::Ipv4Addr,
) -> MonitorProbeResult {
    let args = monitor_curl_args(host, ip);
    let mut result = match capture(program, &args, 10, 4096).await {
        Ok((exit_code, output)) => parse_monitor_curl(exit_code, &output),
        Err(error) => return unavailable_monitor_result(error),
    };
    if monitor_should_test_upload(result.received_bytes, result.current.status) {
        match monitor_upload_stalled(host, ip).await {
            Some(true) => {
                result.current.status = "warning";
                result.current.detail = format!(
                    "{} · тестовая отправка 64 КиБ зависла после TLS handshake",
                    result.current.detail
                );
                result.summary = result.current.detail.clone();
                if let Some(transfer) = result.steps.iter_mut().find(|step| step.id == "transfer") {
                    transfer.status = "warning";
                    transfer.detail = "Контрольная отправка 64 КиБ зависла после TLS handshake · возможное ограничение передачи".into();
                }
            }
            Some(false) => {
                if let Some(transfer) = result.steps.iter_mut().find(|step| step.id == "transfer") {
                    transfer.detail = format!(
                        "{} · короткий ответ, контрольная отправка 64 КиБ не зависла",
                        transfer.detail
                    );
                }
            }
            None => {}
        }
    }
    result
}

pub async fn monitor_probe(host: &str, ip: std::net::Ipv4Addr) -> MonitorProbeResult {
    monitor_probe_with_program(curl_program(), host, ip).await
}

async fn monitor_upload_stalled(host: &str, ip: std::net::Ipv4Addr) -> Option<bool> {
    let mut payload = vec![0_u8; MONITOR_VOLUME_TEST_BYTES];
    getrandom::fill(&mut payload).ok()?;
    let cache_buster = hex::encode(&payload[..8]);
    let mut args = curl_args();
    args.extend([
        "--ipv4".into(),
        "--insecure".into(),
        "--connect-timeout".into(),
        "5".into(),
        "--max-time".into(),
        "8".into(),
        "--user-agent".into(),
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130 Safari/537.36".into(),
        "--resolve".into(),
        format!("{host}:443:{ip}"),
        "--header".into(),
        "Expect:".into(),
        "--header".into(),
        "Content-Type: application/octet-stream".into(),
        "--data-binary".into(),
        "@-".into(),
        "--output".into(),
        if cfg!(windows) { "NUL".into() } else { "/dev/null".into() },
        "--write-out".into(),
        "\\nNFQWS2_MON_UPLOAD|%{time_appconnect}".into(),
        format!("https://{host}/?t={cache_buster}"),
    ]);
    let (exit_code, output) = capture_with_input(curl_program(), &args, &payload, 10, 1024)
        .await
        .ok()?;
    let appconnect_seconds = String::from_utf8_lossy(&output)
        .rsplit_once("\nNFQWS2_MON_UPLOAD|")
        .and_then(|(_, value)| value.trim().parse::<f64>().ok())
        .filter(|seconds| seconds.is_finite() && *seconds >= 0.0 && *seconds <= 60.0);
    Some(monitor_upload_stall_confirmed(
        exit_code,
        appconnect_seconds,
    ))
}

async fn capture_with_input(
    program: &str,
    args: &[String],
    input: &[u8],
    seconds: u64,
    limit: usize,
) -> Result<(i32, Vec<u8>), String> {
    let task = async {
        let mut child = Command::new(program)
            .args(args)
            .env("no_proxy", "*")
            .env("NO_PROXY", "*")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|_| format!("Утилита {program} недоступна"))?;
        let mut stdin = child.stdin.take().ok_or("Нет stdin")?;
        stdin
            .write_all(input)
            .await
            .map_err(|_| "Не удалось передать данные проверочного запроса")?;
        drop(stdin);
        let mut bytes = Vec::new();
        child
            .stdout
            .take()
            .ok_or("Нет stdout")?
            .take((limit + 1) as u64)
            .read_to_end(&mut bytes)
            .await
            .map_err(|_| "Не удалось прочитать результат проверки")?;
        if bytes.len() > limit {
            return Err("Результат проверки превышает лимит".into());
        }
        let status = child
            .wait()
            .await
            .map_err(|_| "Проверочный процесс не завершился")?;
        Ok((status.code().unwrap_or(-1), bytes))
    };
    timeout(Duration::from_secs(seconds), task)
        .await
        .map_err(|_| "Истекло время проверки".to_string())?
}

fn parse_curl(id: &str, exit_code: i32, stdout: &[u8]) -> DiagnosticStep {
    let text = String::from_utf8_lossy(stdout);
    let fields = text
        .rsplit_once("\nNFQWS2_DIAG|")
        .map(|(_, tail)| tail.trim().split('|').collect::<Vec<_>>())
        .unwrap_or_default();
    let code = fields
        .first()
        .and_then(|field| field.parse::<u16>().ok())
        .filter(|code| (100..600).contains(code));
    let seconds = fields
        .get(3)
        .and_then(|field| field.parse::<f64>().ok())
        .filter(|value| value.is_finite() && *value >= 0.0 && *value <= 60.0);
    let received = code.is_some() && (exit_code == 0 || exit_code == 63);
    let detail = if received {
        let protocol = if id == "quic" { "HTTP/3: " } else { "" };
        format!(
            "{protocol}получен HTTP {}{}",
            code.unwrap_or_default(),
            if exit_code == 63 {
                "; загрузка ограничена по размеру"
            } else {
                ""
            }
        )
    } else {
        match exit_code {
            6 => "Не удалось разрешить домен",
            7 => "Соединение отклонено или адрес недоступен",
            28 => "Истекло время сетевой проверки",
            35 => "Ошибка TLS handshake",
            60 => "Ошибка проверки TLS-сертификата",
            52 => "Сервер закрыл соединение без ответа",
            56 => "Соединение сброшено",
            2 => "curl не поддерживает необходимые параметры",
            95 if id == "quic" => "Ошибка уровня HTTP/3; успешный QUIC-запрос не подтверждён",
            96 if id == "quic" => "Не удалось установить QUIC-соединение для HTTP/3",
            _ => "Запрос не завершён; причина блокировки не установлена",
        }
        .to_string()
    };
    DiagnosticStep {
        id: id.into(),
        status: if received {
            if code.is_some_and(|code| code >= 400) {
                "warning"
            } else {
                "ok"
            }
        } else {
            "error"
        },
        detail,
        duration_ms: seconds.map(|seconds| (seconds * 1000.0).round() as u64),
        http_code: code,
    }
}

async fn probe(id: &str, host: &str, ip: IpAddr, port: u16, http3: bool) -> DiagnosticStep {
    probe_scoped(id, host, ip, port, http3, false).await
}

async fn probe_scoped(
    id: &str,
    host: &str,
    ip: IpAddr,
    port: u16,
    http3: bool,
    direct: bool,
) -> DiagnosticStep {
    let mut args = curl_args();
    if direct {
        args.extend(["--local-port".into(), "40000-40999".into()]);
    }
    if id == "sni" {
        args.push("--insecure".into());
    }
    let address = match ip {
        IpAddr::V6(_) => format!("[{ip}]"),
        _ => ip.to_string(),
    };
    args.extend(["--resolve".into(), format!("{host}:{port}:{address}"), "--range".into(), "0-8191".into(),
        "--write-out".into(), "\nNFQWS2_DIAG|%{http_code}|%{time_connect}|%{time_appconnect}|%{time_total}|%{size_download}".into()]);
    if http3 {
        args.push("--http3-only".into());
    }
    args.push(format!(
        "{}://{host}/",
        if port == 80 { "http" } else { "https" }
    ));
    match capture(curl_program(), &args, 10, 70000).await {
        Ok((code, output)) => parse_curl(id, code, &output),
        Err(message) => DiagnosticStep::new(id, "unavailable", message),
    }
}

pub async fn dns_reference(host: &str) -> (DiagnosticStep, Vec<IpAddr>) {
    let mut args = curl_args();
    args.extend([
        "--resolve".into(),
        "cloudflare-dns.com:443:1.1.1.1".into(),
        "--header".into(),
        "Accept: application/dns-json".into(),
        format!("https://cloudflare-dns.com/dns-query?name={host}&type=A"),
    ]);
    let data = match capture(curl_program(), &args, 10, 65536).await {
        Ok((0, bytes)) => serde_json::from_slice::<serde_json::Value>(&bytes).ok(),
        _ => None,
    };
    let addresses = data
        .as_ref()
        .and_then(|value| value.get("Answer"))
        .and_then(|value| value.as_array())
        .map(|answers| {
            answers
                .iter()
                .filter(|value| value.get("type").and_then(|value| value.as_u64()) == Some(1))
                .filter_map(|value| value.get("data")?.as_str()?.parse::<IpAddr>().ok())
                .filter(|ip| public_ip(*ip))
                .take(8)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let step = DiagnosticStep::new(
        "doh",
        if addresses.is_empty() {
            "unavailable"
        } else {
            "ok"
        },
        if addresses.is_empty() {
            "Сверка с Cloudflare DoH не дала публичных адресов".into()
        } else {
            format!(
                "Cloudflare DoH: {}",
                addresses
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        },
    );
    (step, addresses)
}

pub async fn public_ipv4_references(host: &str) -> Vec<IpAddr> {
    for (server, ip, url) in [
        (
            "cloudflare-dns.com",
            "1.1.1.1",
            format!("https://cloudflare-dns.com/dns-query?name={host}&type=A"),
        ),
        (
            "dns.google",
            "8.8.8.8",
            format!("https://dns.google/resolve?name={host}&type=A"),
        ),
    ] {
        let mut args = curl_args();
        args.extend([
            "--resolve".into(),
            format!("{server}:443:{ip}"),
            "--header".into(),
            "Accept: application/dns-json".into(),
            url,
        ]);
        let data = match capture(curl_program(), &args, 7, 65536).await {
            Ok((0, bytes)) => serde_json::from_slice::<serde_json::Value>(&bytes).ok(),
            _ => None,
        };
        let addresses = data
            .as_ref()
            .and_then(|value| value.get("Answer"))
            .and_then(|value| value.as_array())
            .map(|answers| {
                answers
                    .iter()
                    .filter(|value| value.get("type").and_then(|value| value.as_u64()) == Some(1))
                    .filter_map(|value| value.get("data")?.as_str()?.parse::<IpAddr>().ok())
                    .filter(|ip| public_ip(*ip))
                    .take(8)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        if !addresses.is_empty() {
            return addresses;
        }
    }
    Vec::new()
}

async fn tcp_step(addresses: &[IpAddr]) -> DiagnosticStep {
    let started = Instant::now();
    let mut results = Vec::new();
    for ip in addresses.iter().take(3) {
        let connected = matches!(
            timeout(
                Duration::from_secs(3),
                TcpStream::connect(SocketAddr::new(*ip, 443))
            )
            .await,
            Ok(Ok(_))
        );
        results.push(format!(
            "{ip}:443 — {}",
            if connected {
                "соединение установлено"
            } else {
                "соединение не установлено"
            }
        ));
        if connected {
            let mut step = DiagnosticStep::new("tcp", "ok", results.join("; "));
            step.duration_ms = Some(started.elapsed().as_millis() as u64);
            return step;
        }
    }
    DiagnosticStep::new("tcp", "error", results.join("; "))
}

async fn route_step(ip: IpAddr) -> DiagnosticStep {
    let program = ["/opt/sbin/ip", "/sbin/ip", "/usr/sbin/ip"]
        .into_iter()
        .find(|path| std::path::Path::new(path).is_file())
        .unwrap_or("ip");
    match capture(
        program,
        &["route".into(), "get".into(), ip.to_string()],
        3,
        8192,
    )
    .await
    {
        Ok((0, bytes)) => DiagnosticStep::new(
            "route",
            "ok",
            String::from_utf8_lossy(&bytes).trim().to_owned(),
        ),
        _ => DiagnosticStep::new(
            "route",
            "unavailable",
            "Маршрут ядра не удалось прочитать (нужна утилита ip)",
        ),
    }
}

fn http3_advertised(headers: &[u8]) -> Option<bool> {
    let text = String::from_utf8_lossy(headers);
    if !text.lines().any(|line| line.starts_with("HTTP/")) {
        return None;
    }
    Some(
        text.lines()
            .filter_map(|line| line.split_once(':'))
            .filter(|(name, _)| name.eq_ignore_ascii_case("alt-svc"))
            .flat_map(|(_, value)| value.split(','))
            .any(|entry| {
                let protocol = entry.trim().split('=').next().unwrap_or_default();
                protocol == "h3"
                    || protocol
                        .strip_prefix("h3-")
                        .is_some_and(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))
            }),
    )
}

async fn advertised_http3(host: &str, ip: IpAddr) -> Option<bool> {
    let mut args = curl_args();
    args.extend([
        "--head".into(),
        "--dump-header".into(),
        "-".into(),
        "--output".into(),
        if cfg!(windows) { "NUL" } else { "/dev/null" }.into(),
        "--resolve".into(),
        format!("{host}:443:{ip}"),
        format!("https://{host}/"),
    ]);
    match capture(curl_program(), &args, 10, 16384).await {
        Ok((0, bytes)) => http3_advertised(&bytes),
        _ => None,
    }
}

async fn quic_comparison(
    host: &str,
    ip: std::net::Ipv4Addr,
    binary: &str,
    advertised: Option<bool>,
) -> QuicComparison {
    let capability = capture(
        curl_program(),
        &[
            "--disable".into(),
            "--http3-only".into(),
            "--version".into(),
        ],
        2,
        16384,
    )
    .await;
    let full_http3 = matches!(capability, Ok((0, ref bytes)) if has_http3_feature(bytes));
    let (method, current, direct) = if full_http3 {
        let current = probe("quic", host, ip.into(), 443, true).await;
        let direct = bypass_probe(
            ip,
            443,
            ProbeTransport::Udp,
            probe_scoped("quic", host, ip.into(), 443, true, true),
        )
        .await;
        ("http3", current, direct)
    } else {
        let packet = match quic_probe::generate(binary, host).await {
            Ok(packet) => packet,
            Err(message) => {
                return QuicComparison {
                    direct: DiagnosticStep::new("quic", "unavailable", &message),
                    current: DiagnosticStep::new("quic", "unavailable", message),
                    method: "quic_initial",
                    http3_advertised: advertised,
                };
            }
        };
        let current = quic_probe::probe(&packet, ip, false).await;
        // Independent routes need independent QUIC connection IDs. Replaying
        // one Initial from a new source port can be suppressed as a duplicate.
        let direct_packet = match quic_probe::generate(binary, host).await {
            Ok(packet) => packet,
            Err(message) => {
                return QuicComparison {
                    direct: DiagnosticStep::new("quic", "unavailable", message),
                    current,
                    method: "quic_initial",
                    http3_advertised: advertised,
                };
            }
        };
        let direct = bypass_probe(
            ip,
            443,
            ProbeTransport::Udp,
            quic_probe::probe(&direct_packet, ip, true),
        )
        .await;
        ("quic_initial", current, direct)
    };
    QuicComparison {
        direct: direct
            .unwrap_or_else(|message| DiagnosticStep::new("quic", "unavailable", message)),
        current,
        method,
        http3_advertised: advertised,
    }
}

fn has_http3_feature(version_output: &[u8]) -> bool {
    String::from_utf8_lossy(version_output)
        .lines()
        .filter_map(|line| line.trim().split_once(':'))
        .filter(|(label, _)| label.eq_ignore_ascii_case("Features"))
        .flat_map(|(_, features)| features.split_whitespace())
        .any(|feature| feature.eq_ignore_ascii_case("HTTP3"))
}

async fn sni_step(host: &str, ip: IpAddr) -> DiagnosticStep {
    if host == "example.com" {
        return DiagnosticStep::new(
            "sni",
            "unavailable",
            "Имя сайта совпадает с контрольным именем; отдельное сравнение SNI не выполнялось",
        );
    }
    let mut result = probe_scoped("sni", "example.com", ip, 443, false, true).await;
    result.detail = format!(
        "Контрольное SNI example.com закреплено на {ip}: {}. Для чужого имени проверка сертификата отключена; результат этой отдельной пробы сам по себе не доказывает блокировку.",
        result.detail
    );
    if result.status == "error" {
        result.status = "warning";
    }
    result
}

pub async fn diagnose(host: String, binary: String) -> DiagnosticReport {
    let literal_ip = host.parse::<IpAddr>().ok();
    let (mut steps, mut addresses, reference) = if let Some(ip) = literal_ip {
        let skipped_dns = DiagnosticStep::new(
            "dns",
            "unavailable",
            "Указан IP-адрес; DNS для него не запрашивался",
        );
        let skipped_doh = DiagnosticStep::new(
            "doh",
            "unavailable",
            "Сверка с DoH доступна только для доменного имени",
        );
        (vec![skipped_dns, skipped_doh], vec![ip], Vec::new())
    } else {
        let (system, (mut reference_step, reference)) = tokio::join!(
            timeout(Duration::from_secs(4), lookup_host(format!("{host}:443"))),
            dns_reference(&host)
        );
        let mut system_addresses = match system {
            Ok(Ok(addresses)) => addresses
                .map(|address| address.ip())
                .take(16)
                .collect::<Vec<_>>(),
            _ => Vec::new(),
        };
        system_addresses.sort();
        system_addresses.dedup();
        let addresses = system_addresses
            .iter()
            .copied()
            .filter(|ip| public_ip(*ip))
            .collect::<Vec<_>>();
        let dns = DiagnosticStep::new(
            "dns",
            if system_addresses.is_empty() {
                "error"
            } else if addresses.is_empty() {
                "warning"
            } else {
                "ok"
            },
            if system_addresses.is_empty() {
                "Системный DNS не вернул адреса".into()
            } else {
                format!(
                    "DNS роутера: {}",
                    system_addresses
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            },
        );
        if !reference.is_empty()
            && !addresses.is_empty()
            && !addresses.iter().any(|ip| reference.contains(ip))
        {
            reference_step.status = "warning";
            reference_step.detail.push_str(
                ". Ответы отличаются; это возможно у CDN и само по себе не доказывает подмену DNS",
            );
        }
        (vec![dns, reference_step], addresses, reference)
    };
    let mut limitations = vec![
        if literal_ip.is_some() {
            "Проверяется указанный публичный IPv4. Прямой запрос исключён из nfqws2 точечными временными правилами; DNS, DoH и контрольный SNI пропущены.".into()
        } else {
            "Оба режима проверяют один публичный IPv4. Прямой запрос исключён из nfqws2 точечными временными правилами.".into()
        },
        "«Как сейчас» — обычный маршрут самого роутера. В режиме «только устройства политики» этот трафик может идти мимо nfqws2; результат не доказывает выбор профиля для клиентских устройств.".into(),
    ];
    if addresses.is_empty() && !reference.is_empty() {
        addresses.clone_from(&reference);
        limitations.push("Для HTTP-проб использованы адреса DoH: системный DNS не дал публичного адреса (возможен FakeIP).".into());
    }
    let reverse_dns = if literal_ip.is_some() {
        reverse_dns_records(&addresses).await
    } else {
        Vec::new()
    };
    let Some(ip) = addresses.iter().find_map(|ip| match ip {
        IpAddr::V4(ip) => Some(*ip),
        _ => None,
    }) else {
        return DiagnosticReport {
            host,
            addresses,
            reverse_dns,
            steps,
            summary: "Нет публичного IPv4 для сравнения доступности".into(),
            limitations,
            direct: DiagnosticStep::new("direct", "unavailable", "DNS не вернул публичный IPv4"),
            current: DiagnosticStep::new("current", "unavailable", "DNS не вернул публичный IPv4"),
            quic: None,
        };
    };
    let (tcp, current, route, advertised) = tokio::join!(
        tcp_step(&addresses),
        probe("current", &host, ip.into(), 443, false),
        route_step(ip.into()),
        async {
            if literal_ip.is_some() {
                None
            } else {
                advertised_http3(&host, ip.into()).await
            }
        }
    );
    steps.extend([tcp, route]);
    let direct = match bypass_probe(ip, 443, ProbeTransport::Tcp, async {
        let direct = probe_scoped("direct", &host, ip.into(), 443, false, true).await;
        let sni = if literal_ip.is_some() {
            None
        } else {
            Some(sni_step(&host, ip.into()).await)
        };
        (direct, sni)
    })
    .await
    {
        Ok((direct, sni)) => {
            if let Some(sni) = sni {
                steps.push(sni);
            }
            direct
        }
        Err(message) => {
            let direct = DiagnosticStep::new("direct", "unavailable", message);
            return DiagnosticReport {
                summary: diagnosis_summary(&direct, &current).into(),
                host,
                addresses,
                reverse_dns,
                steps,
                limitations,
                direct,
                current,
                quic: None,
            };
        }
    };
    match bypass_probe(
        ip,
        80,
        ProbeTransport::Tcp,
        probe_scoped("http", &host, ip.into(), 80, false, true),
    )
    .await
    {
        Ok(http) => steps.push(http),
        Err(message) => {
            steps.push(DiagnosticStep::new("http", "unavailable", message));
            return DiagnosticReport {
                summary: diagnosis_summary(&direct, &current).into(),
                host,
                addresses,
                reverse_dns,
                steps,
                limitations,
                direct,
                current,
                quic: None,
            };
        }
    }
    let quic = if literal_ip.is_some() {
        steps.push(DiagnosticStep::new(
            "quic",
            "unavailable",
            "Проверка QUIC по IP пропущена: для неё требуется доменное имя и TLS SNI",
        ));
        None
    } else {
        Some(quic_comparison(&host, ip, &binary, advertised).await)
    };
    let summary = diagnosis_summary(&direct, &current).into();
    DiagnosticReport {
        host,
        addresses,
        reverse_dns,
        steps,
        summary,
        limitations,
        direct,
        current,
        quic,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn diagnosis_summary_does_not_label_missing_or_http_errors_as_open() {
        let open = DiagnosticStep::new("direct", "ok", "HTTP 200");
        let failed = DiagnosticStep::new("current", "error", "timeout");
        let unavailable = DiagnosticStep::new("direct", "unavailable", "iptables unavailable");
        assert!(diagnosis_summary(&failed, &open).contains("Сейчас"));
        assert!(diagnosis_summary(&open, &failed).contains("Напрямую"));
        assert!(diagnosis_summary(&failed, &failed).contains("не открылся"));
        assert!(diagnosis_summary(&unavailable, &failed).contains("неполное"));
        let error = parse_curl("direct", 0, b"\nNFQWS2_DIAG|451|0.1|0.2|0.3|7");
        assert!(!probe_open(&error));
    }

    #[test]
    fn alt_svc_is_an_advertisement_not_an_http3_probe() {
        assert_eq!(
            http3_advertised(b"HTTP/1.1 200 OK\r\nAlt-Svc: h3=\":443\"; ma=86400\r\n"),
            Some(true)
        );
        assert_eq!(
            http3_advertised(b"HTTP/1.1 200 OK\r\nX-Test: h3\r\n"),
            Some(false)
        );
        assert_eq!(http3_advertised(b"curl error"), None);
    }

    #[test]
    fn parses_ptr_names_from_dig_kdig_and_nslookup_output() {
        assert_eq!(
            parse_ptr_names("dig", b"dns.google.\nedge.example.net.\n"),
            vec!["dns.google", "edge.example.net"]
        );
        assert_eq!(
            parse_ptr_names("nslookup", b"Server: 192.0.2.1\nAddress: 192.0.2.1#53\n\n8.8.8.8.in-addr.arpa name = dns.google.\n"),
            vec!["dns.google"]
        );
        assert!(
            parse_ptr_names(
                "dig",
                b";; connection timed out; no servers could be reached\n"
            )
            .is_empty()
        );
        assert!(parse_ptr_names("dig", b"not a hostname/../\n").is_empty());
    }

    #[test]
    fn list_membership_matches_domains_subdomains_and_both_ip_families() {
        let ips = [
            "8.8.8.8".parse().unwrap(),
            "2606:4700:4700::1111".parse().unwrap(),
        ];
        assert!(list_entry_matches(
            "www.example.com",
            &ips,
            "example.com",
            false
        ));
        assert!(!list_entry_matches(
            "badexample.com",
            &ips,
            "example.com",
            false
        ));
        assert!(!list_entry_matches(
            "www.example.com",
            &ips,
            "# example.com",
            false
        ));
        assert!(list_entry_matches(
            "www.example.com",
            &ips,
            "8.8.8.0/24",
            true
        ));
        assert!(list_entry_matches(
            "www.example.com",
            &ips,
            "2606:4700::/32",
            true
        ));
        assert!(!list_entry_matches(
            "www.example.com",
            &ips,
            "8.8.8.0/33",
            true
        ));
    }

    #[test]
    fn accepts_only_domain_names_and_normalizes_pasted_urls() {
        assert_eq!(
            normalize_host(" https://Example.COM/path?q=x ").unwrap(),
            "example.com"
        );
        assert_eq!(normalize_host("example.com.").unwrap(), "example.com");
        for host in [
            "127.0.0.1",
            "localhost",
            "router.local",
            "a.lan",
            "a;b.com",
            "--help",
            "x.com:79",
            "a@x.com",
            "file:///etc/passwd",
            "x.com\nHeader:x",
            "-x.com",
        ] {
            assert!(normalize_host(host).is_err(), "accepted {host}");
        }
    }

    #[test]
    fn normalizes_public_ipv4_targets_and_rejects_non_public_or_ipv6_targets() {
        assert_eq!(normalize_site_target(" 8.8.8.8 ").unwrap(), "8.8.8.8");
        assert_eq!(normalize_site_target("youtube.com").unwrap(), "youtube.com");
        for target in [
            "127.0.0.1",
            "192.168.1.1",
            "169.254.1.1",
            "2606:4700:4700::1111",
        ] {
            assert!(normalize_site_target(target).is_err(), "accepted {target}");
        }
    }

    #[test]
    fn refuses_private_reserved_fakeip_and_ipv4_mapped_destinations() {
        for ip in [
            "127.0.0.1",
            "10.0.0.1",
            "192.168.1.1",
            "169.254.1.1",
            "100.64.0.1",
            "198.18.0.1",
            "192.0.2.1",
            "::1",
            "fc00::1",
            "fe80::1",
            "::ffff:127.0.0.1",
            "2001:db8::1",
        ] {
            assert!(!public_ip(ip.parse().unwrap()), "accepted {ip}");
        }
        for ip in ["1.1.1.1", "8.8.8.8", "2606:4700:4700::1111"] {
            assert!(public_ip(ip.parse().unwrap()));
        }
    }

    #[test]
    fn curl_metrics_distinguish_received_http_response_certificate_and_timeout() {
        let probe = parse_curl(
            "https",
            0,
            b"<body>ok</body>\nNFQWS2_DIAG|200|0.01|0.05|0.1|15",
        );
        assert_eq!(probe.status, "ok");
        assert_eq!(probe.http_code, Some(200));
        assert_eq!(probe.duration_ms, Some(100));
        assert_eq!(
            parse_curl("https", 60, b"\nNFQWS2_DIAG|000|0.1|0|0.2|0").status,
            "error"
        );
        assert!(
            parse_curl("https", 60, b"\nNFQWS2_DIAG|000|0.1|0|0.2|0")
                .detail
                .contains("сертификат")
        );
        assert!(
            parse_curl("https", 28, b"\nNFQWS2_DIAG|000|0|0|8|0")
                .detail
                .contains("время")
        );
    }

    #[test]
    fn monitor_probe_distinguishes_response_timeout_from_completed_transfer() {
        let complete = parse_monitor_curl(0, b"\nNFQWS2_MON|200|0.020|0.080|0.150|0.350|65536");
        assert_eq!(complete.current.status, "ok");
        assert_eq!(complete.current.http_code, Some(200));
        assert_eq!(
            complete
                .steps
                .iter()
                .map(|step| step.id.as_str())
                .collect::<Vec<_>>(),
            ["tcp", "tls", "http", "transfer"]
        );
        assert!(complete.steps[3].detail.contains("65536"));

        let incomplete = parse_monitor_curl(28, b"\nNFQWS2_MON|200|0.020|0.080|0.150|8.000|12000");
        assert_eq!(incomplete.current.status, "warning");
        assert!(incomplete.current.detail.contains("возможный обрыв"));
        assert_eq!(incomplete.steps[3].status, "warning");
        assert!(incomplete.steps[3].detail.contains("12000"));

        let size_limited =
            parse_monitor_curl(63, b"\nNFQWS2_MON|200|0.020|0.080|0.150|0.350|65536");
        assert_eq!(size_limited.current.status, "ok");

        let short_size_limited =
            parse_monitor_curl(63, b"\nNFQWS2_MON|200|0.020|0.080|0.150|0.350|12000");
        assert_eq!(short_size_limited.current.status, "warning");
        assert!(short_size_limited.steps[3].detail.contains("12000"));
    }

    #[test]
    fn monitor_probe_counts_http_errors_as_a_response_and_separates_connect_and_tls_failures() {
        let http_error = parse_monitor_curl(0, b"\nNFQWS2_MON|404|0.010|0.060|0.100|0.120|512");
        assert_eq!(http_error.current.status, "ok");
        assert_eq!(http_error.current.http_code, Some(404));
        assert!(http_error.steps[2].detail.contains("404"));

        let connect_error = parse_monitor_curl(7, b"\nNFQWS2_MON|000|0|0|0|2.000|0");
        assert_eq!(connect_error.steps[0].status, "error");
        assert_eq!(connect_error.steps[1].status, "unavailable");
        assert_eq!(connect_error.current.status, "error");

        let tls_error = parse_monitor_curl(35, b"\nNFQWS2_MON|000|0.020|0|0|0.200|0");
        assert_eq!(tls_error.steps[0].status, "ok");
        assert_eq!(tls_error.steps[1].status, "error");
        assert!(tls_error.current.detail.contains("TLS"));
    }

    #[tokio::test]
    async fn monitor_probe_keeps_curl_launch_errors_as_unavailable_instead_of_network_failures() {
        let program = "nfqws2-ui-test-curl-that-does-not-exist";
        let unavailable =
            monitor_probe_with_program(program, "example.com", "8.8.8.8".parse().unwrap()).await;

        let message = format!("Утилита {program} недоступна");
        assert_eq!(unavailable.summary, message);
        assert_eq!(unavailable.current.status, "unavailable");
        assert_eq!(unavailable.current.detail, message);
        assert_eq!(unavailable.steps.len(), 4);
        assert!(
            unavailable
                .steps
                .iter()
                .all(|step| step.status == "unavailable")
        );
    }

    #[test]
    fn monitor_probe_uses_bounded_pinned_https_get_without_post_or_redirects() {
        let args = monitor_curl_args("example.com", "8.8.8.8".parse().unwrap());
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--resolve", "example.com:443:8.8.8.8"])
        );
        assert!(args.windows(2).any(|pair| pair == ["--range", "0-65535"]));
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--max-filesize", "65536"])
        );
        assert!(args.windows(2).any(|pair| pair == ["--max-time", "8"]));
        assert!(args.iter().any(|arg| arg == "--insecure"));
        assert!(
            !args
                .iter()
                .any(|arg| arg == "--location" || arg == "--data-binary" || arg == "--request")
        );
        assert!(args.iter().any(|arg| arg == "https://example.com/"));
        assert!(
            args.windows(2)
                .any(|pair| pair == ["--connect-timeout", "5"])
        );
        assert!(args.windows(2).any(|pair| pair == ["--user-agent", "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130 Safari/537.36"]));
    }

    #[test]
    fn monitor_checks_volume_only_for_small_completed_http_responses() {
        assert!(monitor_should_test_upload(0, "ok"));
        assert!(monitor_should_test_upload(
            MONITOR_VOLUME_SMALL_BYTES - 1,
            "ok"
        ));
        assert!(!monitor_should_test_upload(
            MONITOR_VOLUME_SMALL_BYTES,
            "ok"
        ));
        assert!(!monitor_should_test_upload(512, "warning"));
        assert!(monitor_upload_stall_confirmed(28, Some(0.0001)));
        assert!(!monitor_upload_stall_confirmed(28, None));
        assert!(!monitor_upload_stall_confirmed(28, Some(0.0)));
        assert!(!monitor_upload_stall_confirmed(23, Some(0.08)));
    }

    #[test]
    fn quic_probe_reports_http3_response_and_specific_quic_failures() {
        let response = parse_curl("quic", 0, b"\nNFQWS2_DIAG|200|0.01|0.05|0.1|15");
        assert_eq!(response.status, "ok");
        assert!(response.detail.contains("HTTP/3"));
        assert!(response.detail.contains("200"));

        let layer_error = parse_curl("quic", 95, b"\nNFQWS2_DIAG|000|0|0|0.2|0");
        assert!(layer_error.detail.contains("HTTP/3"));
        assert_eq!(layer_error.status, "error");

        let handshake_error = parse_curl("quic", 96, b"\nNFQWS2_DIAG|000|0|0|0.2|0");
        assert!(handshake_error.detail.contains("QUIC"));
        assert_eq!(handshake_error.status, "error");
    }

    #[test]
    fn http3_capability_uses_the_features_token_not_a_substring() {
        assert!(has_http3_feature(
            b"curl 8.8.0\nFeatures: AsynchDNS HTTP3 IPv6\n"
        ));
        assert!(!has_http3_feature(
            b"curl 8.8.0\nFeatures: AsynchDNS HTTP2 IPv6\n"
        ));
        assert!(!has_http3_feature(
            b"curl 8.8.0\nProtocols: HTTP3\nFeatures: AsynchDNS HTTP2\n"
        ));
    }
}
