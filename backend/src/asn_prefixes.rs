//! Bounded RIPEstat lookup used by the ASN-to-ipset helper.

use serde_json::Value;
use std::{net::Ipv4Addr, path::Path, process::Stdio, time::Duration};
use tokio::{process::Command, time::timeout};

const MAX_RIPESTAT_BYTES: usize = 1024 * 1024;
const MAX_PREFIXES: usize = 4096;

pub async fn fetch_ipv4_prefixes(asn: u32) -> Result<Vec<String>, String> {
    if asn == 0 {
        return Err("Укажите номер ASN больше нуля".into());
    }
    let url = format!(
        "https://stat.ripe.net/data/announced-prefixes/data.json?sourceapp=nfqws2-ui&resource=AS{asn}&min_peers_seeing=10"
    );
    let curl = if Path::new("/opt/bin/curl").is_file() {
        "/opt/bin/curl"
    } else if cfg!(windows) {
        "curl.exe"
    } else {
        "curl"
    };
    let output = timeout(
        Duration::from_secs(12),
        Command::new(curl)
            .args([
                "--disable",
                "--silent",
                "--show-error",
                "--noproxy",
                "*",
                "--connect-timeout",
                "3",
                "--max-time",
                "10",
                "--max-filesize",
                "1048576",
                "--write-out",
                "\n%{http_code}",
                &url,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| "RIPEstat не ответил за отведённое время".to_string())?
    .map_err(|error| format!("Не удалось запустить curl для RIPEstat: {error}"))?;
    if !output.status.success() {
        return Err(
            "Не удалось загрузить данные RIPEstat (ответ слишком большой или соединение прервано)"
                .into(),
        );
    }
    let split = output
        .stdout
        .iter()
        .rposition(|byte| *byte == b'\n')
        .ok_or_else(|| "RIPEstat не вернул HTTP-статус".to_string())?;
    let status = std::str::from_utf8(&output.stdout[split + 1..])
        .ok()
        .and_then(|value| value.trim().parse::<u16>().ok())
        .ok_or_else(|| "RIPEstat вернул некорректный HTTP-статус".to_string())?;
    if status != 200 {
        return Err(format!("RIPEstat вернул HTTP {status}"));
    }
    parse_announced_ipv4_prefixes(&output.stdout[..split])
}

pub async fn resolve_asn_query(query: &str) -> Result<(u32, Option<String>), String> {
    let query = query.trim();
    if let Some(asn) = parse_asn_query(query)? {
        return Ok((asn, None));
    }

    let host = crate::site_diagnostics::normalize_site_target(query)?;
    let address = if let Ok(address) = host.parse::<std::net::IpAddr>() {
        if !crate::site_diagnostics::public_ip(address) {
            return Err("Укажите публичный IP-адрес или домен".into());
        }
        address
    } else {
        crate::site_diagnostics::public_ipv4_references(&host)
            .await
            .into_iter()
            .find(|address| matches!(address, std::net::IpAddr::V4(_)))
            .ok_or_else(|| format!("Не удалось получить публичный IPv4-адрес для {host}"))?
    };
    let url = format!(
        "https://stat.ripe.net/data/network-info/data.json?sourceapp=nfqws2-ui&resource={address}"
    );
    let output = fetch_ripe_response(&url).await?;
    let asn =
        parse_network_info_asn(&output).map_err(|message| format!("{message} для {address}"))?;
    Ok((asn, Some(format!("{query} → {address}"))))
}

fn parse_network_info_asn(bytes: &[u8]) -> Result<u32, String> {
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|_| "RIPEstat вернул некорректные данные о сети".to_string())?;
    value
        .pointer("/data/asns/0")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<u32>().ok())
        .or_else(|| {
            value
                .pointer("/data/asns/0")
                .and_then(Value::as_u64)
                .and_then(|value| u32::try_from(value).ok())
        })
        .filter(|asn| *asn > 0)
        .ok_or_else(|| "RIPEstat не указал ASN".to_string())
}

fn parse_asn_query(query: &str) -> Result<Option<u32>, String> {
    let numeric = query
        .get(..2)
        .filter(|prefix| prefix.eq_ignore_ascii_case("AS"))
        .map(|_| &query[2..])
        .unwrap_or(query);
    if numeric.is_empty() || !numeric.bytes().all(|byte| byte.is_ascii_digit()) {
        return Ok(None);
    }
    let asn = numeric
        .parse::<u32>()
        .map_err(|_| "Номер ASN вне допустимого диапазона".to_string())?;
    if asn == 0 {
        return Err("Укажите номер ASN больше нуля".into());
    }
    Ok(Some(asn))
}

async fn fetch_ripe_response(url: &str) -> Result<Vec<u8>, String> {
    let curl = if Path::new("/opt/bin/curl").is_file() {
        "/opt/bin/curl"
    } else if cfg!(windows) {
        "curl.exe"
    } else {
        "curl"
    };
    let output = timeout(
        Duration::from_secs(12),
        Command::new(curl)
            .args([
                "--disable",
                "--silent",
                "--show-error",
                "--noproxy",
                "*",
                "--connect-timeout",
                "3",
                "--max-time",
                "10",
                "--max-filesize",
                "1048576",
                "--fail",
                url,
            ])
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| "RIPEstat не ответил за отведённое время".to_string())?
    .map_err(|error| format!("Не удалось запустить curl для RIPEstat: {error}"))?;
    if !output.status.success() {
        return Err(
            "Не удалось загрузить данные RIPEstat (ответ слишком большой или соединение прервано)"
                .into(),
        );
    }
    if output.stdout.len() > MAX_RIPESTAT_BYTES {
        return Err("Ответ RIPEstat превышает лимит 1 МБ".into());
    }
    Ok(output.stdout)
}

fn parse_announced_ipv4_prefixes(bytes: &[u8]) -> Result<Vec<String>, String> {
    if bytes.len() > MAX_RIPESTAT_BYTES {
        return Err("Ответ RIPEstat превышает лимит 1 МБ".into());
    }
    let value: Value = serde_json::from_slice(bytes)
        .map_err(|_| "RIPEstat вернул некорректные данные о префиксах".to_string())?;
    let records = value
        .pointer("/data/prefixes")
        .and_then(Value::as_array)
        .ok_or_else(|| "В ответе RIPEstat нет списка объявленных префиксов".to_string())?;
    let mut ranges = Vec::new();
    for record in records {
        let Some(prefix) = record.get("prefix").and_then(Value::as_str) else {
            continue;
        };
        let Some((address, length)) = prefix.trim().split_once('/') else {
            continue;
        };
        let (Ok(address), Ok(length)) = (address.parse::<Ipv4Addr>(), length.parse::<u8>()) else {
            continue;
        };
        if length > 32 || length == 0 {
            continue;
        }
        let host_bits = 32 - u32::from(length);
        let mask = u32::MAX.checked_shl(host_bits).unwrap_or(0);
        let start = u64::from(u32::from(address) & mask);
        let end = start + (1u64 << host_bits) - 1;
        ranges.push((start, end, length));
        if ranges.len() > MAX_PREFIXES * 4 {
            return Err("RIPEstat вернул слишком много записей; список не изменён".into());
        }
    }
    if ranges.is_empty() {
        return Err("RIPEstat не нашёл IPv4-префиксы, объявленные этим ASN".into());
    }
    ranges.sort_unstable_by_key(|(start, end, _)| (*start, std::cmp::Reverse(*end)));
    let mut prefixes = Vec::new();
    let mut covered_until = None;
    for (start, end, length) in ranges {
        if covered_until.is_some_and(|covered| start <= covered) {
            continue;
        }
        let address = Ipv4Addr::from(start as u32);
        prefixes.push(format!("{address}/{length}"));
        covered_until = Some(end);
        if prefixes.len() > MAX_PREFIXES {
            return Err(format!(
                "ASN объявляет более {MAX_PREFIXES} отдельных IPv4-префиксов; список не изменён"
            ));
        }
    }
    Ok(prefixes)
}

#[cfg(test)]
mod tests {
    use super::{
        MAX_PREFIXES, MAX_RIPESTAT_BYTES, parse_announced_ipv4_prefixes, parse_asn_query,
        parse_network_info_asn, resolve_asn_query,
    };

    #[tokio::test]
    async fn direct_asn_lookup_skips_network_resolution() {
        assert_eq!(resolve_asn_query("aS15169").await.unwrap(), (15169, None));
    }

    #[test]
    fn parses_asn_numbers_case_insensitively_and_rejects_invalid_ranges() {
        assert_eq!(parse_asn_query("AS15169").unwrap(), Some(15169));
        assert_eq!(parse_asn_query("aS13335").unwrap(), Some(13335));
        assert_eq!(parse_asn_query("4294967295").unwrap(), Some(u32::MAX));
        assert!(parse_asn_query("AS0").is_err());
        assert!(parse_asn_query("4294967296").is_err());
        assert_eq!(parse_asn_query("example.com").unwrap(), None);
    }

    #[test]
    fn reads_asn_from_ripe_network_info_responses() {
        assert_eq!(
            parse_network_info_asn(br#"{"data":{"asns":["15169"]}}"#).unwrap(),
            15169
        );
        assert_eq!(
            parse_network_info_asn(br#"{"data":{"asns":[13335]}}"#).unwrap(),
            13335
        );
        assert!(parse_network_info_asn(br#"{"data":{"asns":[]}}"#).is_err());
        assert!(parse_network_info_asn(b"not json").is_err());
    }

    #[test]
    fn parses_unique_ipv4_prefixes_and_removes_nested_ranges() {
        let json = br#"{"data":{"prefixes":[{"prefix":"8.8.8.0/24"},{"prefix":"8.8.8.0/24"},{"prefix":"8.8.8.0/25"},{"prefix":"8.8.8.0/26"},{"prefix":"2001:db8::/32"},{"prefix":"0.0.0.0/0"},{"prefix":"bad/24"},{"prefix":"1.2.3.4/33"},{"prefix":"9.9.9.9/24"}]}}"#;
        assert_eq!(
            parse_announced_ipv4_prefixes(json).unwrap(),
            ["8.8.8.0/24", "9.9.9.0/24"]
        );
    }

    #[test]
    fn rejects_missing_or_empty_prefix_lists() {
        assert!(parse_announced_ipv4_prefixes(br#"{"data":{}}"#).is_err());
        assert!(parse_announced_ipv4_prefixes(br#"{"data":{"prefixes":[]}}"#).is_err());
    }

    #[test]
    fn rejects_oversized_responses_and_prefix_sets() {
        assert!(parse_announced_ipv4_prefixes(&vec![b' '; MAX_RIPESTAT_BYTES + 1]).is_err());
        let records = (0..=MAX_PREFIXES)
            .map(|index| format!("{{\"prefix\":\"10.{}.{}.0/24\"}}", index / 256, index % 256))
            .collect::<Vec<_>>()
            .join(",");
        let json = format!("{{\"data\":{{\"prefixes\":[{records}]}}}}");
        assert!(parse_announced_ipv4_prefixes(json.as_bytes()).is_err());
    }
}
