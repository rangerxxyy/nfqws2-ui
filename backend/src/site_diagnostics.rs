//! Bounded router-side website probes. No shell, arbitrary URLs or firewall edits.

use serde::Serialize;
use std::{
    net::{IpAddr, SocketAddr},
    process::Stdio,
    time::{Duration, Instant},
};
use tokio::{
    io::AsyncReadExt,
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

impl DiagnosticStep {
    fn new(id: &str, status: &'static str, detail: impl Into<String>) -> Self {
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
    pub steps: Vec<DiagnosticStep>,
    pub summary: String,
    pub limitations: Vec<String>,
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

fn public_ip(ip: IpAddr) -> bool {
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

async fn capture(
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

fn curl_program() -> &'static str {
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
        format!(
            "Получен HTTP {}{}",
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
    let mut args = curl_args();
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

async fn dns_reference(host: &str) -> (DiagnosticStep, Vec<IpAddr>) {
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

async fn quic_step(host: &str, ip: IpAddr) -> DiagnosticStep {
    let supported = capture(
        curl_program(),
        &["--disable".into(), "--version".into()],
        2,
        16384,
    )
    .await
    .is_ok_and(|(code, bytes)| code == 0 && String::from_utf8_lossy(&bytes).contains("HTTP3"));
    if supported {
        probe("quic", host, ip, 443, true).await
    } else {
        DiagnosticStep::new(
            "quic",
            "unavailable",
            "Установленный curl не поддерживает HTTP/3; QUIC не проверен",
        )
    }
}

async fn sni_step(host: &str, ip: IpAddr) -> DiagnosticStep {
    if host == "example.com" {
        return DiagnosticStep::new(
            "sni",
            "unavailable",
            "Имя сайта совпадает с контрольным именем; отдельное сравнение SNI не выполнялось",
        );
    }
    let mut result = probe("sni", "example.com", ip, 443, false).await;
    result.detail = format!(
        "Контрольное SNI example.com на адресе {ip}: {}. Ошибка сертификата для чужого имени ожидаема; это не доказательство блокировки.",
        result.detail
    );
    if result.status == "error" {
        result.status = "warning";
    }
    result
}

pub async fn diagnose(host: String) -> DiagnosticReport {
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
    let mut addresses = system_addresses
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
    let mut limitations = vec!["Проверки выполняются с роутера. Маршрут клиентского трафика и выбранный профиль могут отличаться.".into(),
        "Изолированное сравнение с обходом nfqws2 и точное сопоставление профилей ещё не выполнены; причину DPI или поломку стратегией эти пробы сами по себе не доказывают.".into()];
    if addresses.is_empty() && !reference.is_empty() {
        addresses.clone_from(&reference);
        limitations.push("Для HTTP-проб использованы адреса DoH: системный DNS не дал публичного адреса (возможен FakeIP).".into());
    }
    let mut steps = vec![dns, reference_step];
    let Some(ip) = addresses.first().copied() else {
        return DiagnosticReport {
            host,
            addresses,
            steps,
            summary: "Нет публичного адреса для безопасной сетевой проверки".into(),
            limitations,
        };
    };
    let (tcp, https, http, route, quic, sni) = tokio::join!(
        tcp_step(&addresses),
        probe("https", &host, ip, 443, false),
        probe("http", &host, ip, 80, false),
        route_step(ip),
        quic_step(&host, ip),
        sni_step(&host, ip)
    );
    let summary = if https.http_code.is_some() {"HTTPS-сервер ответил; HTTP-код и детали приведены ниже"}
        else if tcp.status == "ok" {"TCP доступен, но HTTPS не завершился; проверьте TLS и сообщения проб"}
        else {"Не удалось установить соединение с проверенными адресами; причина требует дополнительной диагностики"}.into();
    steps.extend([tcp, https, http, route, sni, quic]);
    DiagnosticReport {
        host,
        addresses,
        steps,
        summary,
        limitations,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
