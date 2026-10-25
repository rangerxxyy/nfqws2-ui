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
    use super::{MAX_PREFIXES, MAX_RIPESTAT_BYTES, parse_announced_ipv4_prefixes};

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
