//! Safe import and ranking of provider-specific community strategy candidates.
//! Remote entries are untrusted data and are never applied without a fresh
//! local nfqws2 probe.

use crate::{
    site_diagnostics::normalize_host,
    strategy_picker::{StrategyCandidate, StrategyProtocol, validate_candidate},
};
use serde_json::Value;
use std::{collections::HashSet, net::IpAddr, path::Path, process::Stdio, time::Duration};
use tokio::{
    net::lookup_host,
    process::Command,
    time::{sleep, timeout},
};

pub const COMMUNITY_REPOSITORY: &str =
    "https://raw.githubusercontent.com/zemidala/nfqws2-strategies/main/";
pub const MAX_COMMUNITY_BYTES: usize = 64 * 1024;
pub const MAX_COMMUNITY_ITEMS: usize = 8;
const COMMUNITY_MAX_STEP_BYTES: usize = 200;
const COMMUNITY_CACHE_TTL: Duration = Duration::from_secs(24 * 60 * 60);

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommunityDiscovery {
    pub provider_asn: u32,
    pub target_asn: Option<u32>,
    pub candidates: Vec<StrategyCandidate>,
}

pub struct CommunityParseContext<'a> {
    pub provider_asn: u32,
    pub target_asn: Option<u32>,
    pub host: &'a str,
    pub protocol: StrategyProtocol,
    pub available_functions: &'a HashSet<String>,
    pub available_blobs: &'a [String],
    pub today: &'a str,
}

/// Discover the router's public provider ASN via RIPEstat, look up the target
/// site's ASN, and download only that provider's bounded community file. Any
/// candidate is still locally validated here and then freshly tested by the
/// caller's isolated strategy runner.
pub async fn discover_candidates(
    host: &str,
    protocol: StrategyProtocol,
    available_functions: &HashSet<String>,
    available_blobs: &[String],
    cache_directory: &Path,
) -> Result<CommunityDiscovery, String> {
    if protocol == StrategyProtocol::Quic {
        return Err("Каталог сообщества пока содержит только HTTP и TLS стратегии".into());
    }
    let provider_asn = router_provider_asn(cache_directory).await?;
    // The site's ASN is only a ranking hint. If name resolution or RIPEstat
    // fails, same-host/provider candidates remain usable.
    let target_asn = match resolve_target_address(host, protocol).await {
        Some(address) => query_asn(address).await.ok().flatten(),
        None => None,
    };
    let Some(provider_data) = fetch_provider_file(cache_directory, provider_asn).await? else {
        return Ok(CommunityDiscovery {
            provider_asn,
            target_asn,
            candidates: Vec::new(),
        });
    };
    let today = unix_days_today();
    let today = format_date_days(today);
    let candidates = parse_candidates(
        &provider_data,
        CommunityParseContext {
            provider_asn,
            target_asn,
            host,
            protocol,
            available_functions,
            available_blobs,
            today: &today,
        },
    )?;
    Ok(CommunityDiscovery {
        provider_asn,
        target_asn,
        candidates,
    })
}

async fn router_provider_asn(cache_directory: &Path) -> Result<u32, String> {
    tokio::fs::create_dir_all(cache_directory)
        .await
        .map_err(|error| format!("Не удалось создать кэш стратегий сообщества: {error}"))?;
    let cache = cache_directory.join("provider-asn.txt");
    if let Ok(metadata) = tokio::fs::metadata(&cache).await
        && metadata
            .modified()
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age < COMMUNITY_CACHE_TTL)
        && let Ok(value) = tokio::fs::read_to_string(&cache).await
        && let Ok(asn) = value.trim().parse::<u32>()
        && asn > 0
    {
        return Ok(asn);
    }
    let address = public_router_address().await?;
    let asn = query_asn(address)
        .await?
        .ok_or_else(|| "RIPEstat не определил ASN подключения роутера".to_string())?;
    let temporary = cache_directory.join(format!("provider-asn.{}.tmp", std::process::id()));
    if tokio::fs::write(&temporary, asn.to_string()).await.is_ok()
        && tokio::fs::rename(&temporary, &cache).await.is_err()
    {
        let _ = tokio::fs::remove_file(&temporary).await;
    }
    Ok(asn)
}

async fn public_router_address() -> Result<IpAddr, String> {
    let (status, bytes) =
        fetch_http("https://stat.ripe.net/data/whats-my-ip/data.json?sourceapp=nfqws2-ui").await?;
    if status != 200 {
        return Err(format!(
            "RIPEstat не определил внешний адрес роутера (HTTP {status})"
        ));
    }
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|_| "RIPEstat вернул некорректный адрес роутера".to_string())?;
    let address = value
        .pointer("/data/ip")
        .and_then(Value::as_str)
        .and_then(|value| value.parse::<IpAddr>().ok())
        .filter(|address| crate::site_diagnostics::public_ip(*address))
        .ok_or_else(|| "RIPEstat вернул непубличный или некорректный адрес роутера".to_string())?;
    Ok(address)
}

async fn query_asn(address: IpAddr) -> Result<Option<u32>, String> {
    let url = format!(
        "https://stat.ripe.net/data/network-info/data.json?sourceapp=nfqws2-ui&resource={address}"
    );
    let (status, bytes) = fetch_http(&url).await?;
    if status != 200 {
        return Ok(None);
    }
    let value: Value = serde_json::from_slice(&bytes)
        .map_err(|_| "RIPEstat вернул некорректные сведения об ASN".to_string())?;
    Ok(value
        .pointer("/data/asns")
        .and_then(Value::as_array)
        .and_then(|asns| asns.first())
        .and_then(parse_asn))
}

fn parse_asn(value: &Value) -> Option<u32> {
    value
        .as_u64()
        .and_then(|value| u32::try_from(value).ok())
        .or_else(|| {
            value
                .as_str()
                .and_then(|value| value.strip_prefix("AS").unwrap_or(value).parse().ok())
        })
        .filter(|asn| *asn > 0)
}

async fn resolve_target_address(host: &str, protocol: StrategyProtocol) -> Option<IpAddr> {
    let port = if protocol == StrategyProtocol::Http {
        80
    } else {
        443
    };
    // Resolve the caller's domain using the router's resolver while keeping
    // errors optional; community candidates can still be ranked by site name.
    let lookup = timeout(Duration::from_secs(4), lookup_host((host, port)))
        .await
        .ok()?
        .ok()?;
    lookup
        .map(|socket| socket.ip())
        .find(|address| crate::site_diagnostics::public_ip(*address))
}

async fn fetch_provider_file(directory: &Path, asn: u32) -> Result<Option<Vec<u8>>, String> {
    tokio::fs::create_dir_all(directory)
        .await
        .map_err(|error| format!("Не удалось создать кэш стратегий сообщества: {error}"))?;
    let cache = directory.join(format!("community-AS{asn}.json"));
    let missing = directory.join(format!("community-AS{asn}.not-found"));
    if let Ok(metadata) = tokio::fs::metadata(&missing).await
        && metadata
            .modified()
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age < COMMUNITY_CACHE_TTL)
    {
        return Ok(None);
    }
    if let Ok(metadata) = tokio::fs::metadata(&cache).await
        && metadata.len() <= MAX_COMMUNITY_BYTES as u64
        && metadata
            .modified()
            .ok()
            .and_then(|modified| modified.elapsed().ok())
            .is_some_and(|age| age < COMMUNITY_CACHE_TTL)
    {
        return tokio::fs::read(&cache)
            .await
            .map(Some)
            .map_err(|error| format!("Не удалось прочитать кэш сообщества: {error}"));
    }
    let url = format!("{COMMUNITY_REPOSITORY}data/AS{asn}.json");
    let mut response = None;
    for attempt in 0..2 {
        match fetch_http(&url).await {
            Ok((404, _)) => {
                let _ = tokio::fs::remove_file(&cache).await;
                tokio::fs::write(&missing, b"not found")
                    .await
                    .map_err(|error| format!("Не удалось сохранить состояние каталога: {error}"))?;
                return Ok(None);
            }
            Ok((200, body)) => {
                response = Some(body);
                break;
            }
            Ok((status, _)) => {
                if attempt == 1 {
                    return Err(format!("GitHub не отдал файл стратегий (HTTP {status})"));
                }
            }
            Err(message) if attempt == 1 => return Err(message),
            Err(_) => {}
        }
        sleep(Duration::from_millis(250)).await;
    }
    let Some(body) = response else {
        return Ok(None);
    };
    if body.len() > MAX_COMMUNITY_BYTES {
        return Err("Файл стратегий сообщества превышает 64 КБ".into());
    }
    let temporary = directory.join(format!("community-AS{asn}.{}.tmp", std::process::id()));
    tokio::fs::write(&temporary, &body)
        .await
        .map_err(|error| format!("Не удалось сохранить кэш сообщества: {error}"))?;
    if let Err(error) = tokio::fs::rename(&temporary, &cache).await {
        let _ = tokio::fs::remove_file(&temporary).await;
        return Err(format!(
            "Не удалось завершить запись кэша сообщества: {error}"
        ));
    }
    let _ = tokio::fs::remove_file(&missing).await;
    Ok(Some(body))
}

async fn fetch_http(url: &str) -> Result<(u16, Vec<u8>), String> {
    let curl = if Path::new("/opt/bin/curl").is_file() {
        "/opt/bin/curl"
    } else if cfg!(windows) {
        "curl.exe"
    } else {
        "curl"
    };
    let command = Command::new(curl)
        .args([
            "--disable",
            "--silent",
            "--show-error",
            "--noproxy",
            "*",
            "--connect-timeout",
            "3",
            "--max-time",
            "6",
            "--max-filesize",
            "65536",
            "--write-out",
            "\n%{http_code}",
            url,
        ])
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .output();
    let output = timeout(Duration::from_secs(7), command)
        .await
        .map_err(|_| "Сетевой запрос стратегий сообщества превысил лимит времени".to_string())?
        .map_err(|error| format!("Не удалось запустить curl для каталога сообщества: {error}"))?;
    if !output.status.success() {
        return Err(format!(
            "curl завершил запрос каталога сообщества с кодом {}",
            output.status.code().unwrap_or(-1)
        ));
    }
    let split = output
        .stdout
        .iter()
        .rposition(|byte| *byte == b'\n')
        .ok_or_else(|| "curl не вернул HTTP-статус каталога сообщества".to_string())?;
    let status = std::str::from_utf8(&output.stdout[split + 1..])
        .ok()
        .and_then(|value| value.trim().parse::<u16>().ok())
        .ok_or_else(|| "curl вернул некорректный HTTP-статус каталога сообщества".to_string())?;
    if split > MAX_COMMUNITY_BYTES {
        return Err("Ответ каталога сообщества превышает 64 КБ".into());
    }
    Ok((status, output.stdout[..split].to_vec()))
}

fn unix_days_today() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| (duration.as_secs() / 86_400) as i64)
        .unwrap_or_default()
}

fn format_date_days(days: i64) -> String {
    // Inverse Gregorian civil-date conversion, needed only to compare the
    // repository's YYYY-MM-DD freshness field without pulling another crate.
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    format!("{year:04}-{month:02}-{day:02}")
}

/// Parse one provider-ASN JSON file from the community catalog. Entries must
/// match the router's detected ASN and requested protocol, pass the installed
/// Lua/blob allow-list, and contain only the accepted Lua strategy tokens.
/// Ranking follows the source UI: fresh same-site, fresh same-site-network,
/// then fresh/high-confirmation provider candidates. Stale entries can still
/// be tried after fresh ones, but never gain freshness priority.
pub fn parse_candidates(
    bytes: &[u8],
    context: CommunityParseContext<'_>,
) -> Result<Vec<StrategyCandidate>, String> {
    let CommunityParseContext {
        provider_asn,
        target_asn,
        host,
        protocol,
        available_functions,
        available_blobs,
        today,
    } = context;
    if bytes.len() > MAX_COMMUNITY_BYTES {
        return Err("Файл стратегий сообщества превышает 64 КБ".into());
    }
    let root: Value = serde_json::from_slice(bytes)
        .map_err(|_| "Файл стратегий сообщества содержит некорректный JSON".to_string())?;
    let items = root
        .get("items")
        .and_then(Value::as_array)
        .ok_or_else(|| "В файле стратегий сообщества нет списка items".to_string())?;
    let expected_protocol = match protocol {
        StrategyProtocol::Http => "http",
        StrategyProtocol::Https => "tls",
        StrategyProtocol::Quic => return Ok(Vec::new()),
    };
    let today_days = parse_date_days(today);
    let normalized_host = normalize_host(host).ok();
    let mut ranked = Vec::new();
    let mut seen = HashSet::<Vec<String>>::new();

    for item in items.iter().take(512) {
        if item.get("asn").and_then(Value::as_u64) != Some(provider_asn as u64)
            || item.get("proto").and_then(Value::as_str) != Some(expected_protocol)
        {
            continue;
        }
        let Some(raw_steps) = item.get("steps").and_then(Value::as_array) else {
            continue;
        };
        if raw_steps.is_empty() || raw_steps.len() > 8 {
            continue;
        }
        let Some(steps) = raw_steps
            .iter()
            .map(Value::as_str)
            .collect::<Option<Vec<_>>>()
            .map(|values| values.into_iter().map(str::to_owned).collect::<Vec<_>>())
        else {
            continue;
        };
        if steps.iter().any(|step| !valid_community_step(step))
            || validate_candidate(&steps, available_functions, available_blobs).is_err()
            || !seen.insert(steps.clone())
        {
            continue;
        }

        let reports = nonnegative_count(item.get("reports"));
        let fails = nonnegative_count(item.get("fails"));
        // The source UI drops variants which failed for a majority of at least
        // two reporters; one failure never suppresses a strategy.
        if fails >= 2 && fails > reports {
            continue;
        }
        let last = item.get("last").and_then(Value::as_str).unwrap_or("");
        let last_days = parse_date_days(last);
        let fresh = today_days
            .zip(last_days)
            .is_some_and(|(today, last)| (0..60).contains(&(today - last)));
        let mut same_host = false;
        let mut same_target_asn = false;
        if let Some(targets) = item.get("targets").and_then(Value::as_array) {
            for target in targets.iter().take(30) {
                if let Some(target_host) = target.get("host").and_then(Value::as_str)
                    && let (Some(expected), Ok(actual)) =
                        (normalized_host.as_deref(), normalize_host(target_host))
                    && expected == actual
                {
                    same_host = true;
                }
                if target_asn.is_some()
                    && target.get("asn").and_then(Value::as_u64) == target_asn.map(u64::from)
                {
                    same_target_asn = true;
                }
            }
        }
        let match_rank = if same_host {
            2
        } else if same_target_asn {
            1
        } else {
            0
        };
        let match_description = if same_host {
            "этот сайт".to_owned()
        } else if same_target_asn {
            format!(
                "сеть AS{}",
                target_asn.expect("same_target_asn requires an ASN")
            )
        } else {
            format!("провайдер AS{provider_asn}")
        };
        let source = format!(
            "Стратегии сообщества · {match_description} · подтвердили {reports}{}{}",
            if fails > 0 {
                format!(", не сработала у {fails}")
            } else {
                String::new()
            },
            if fresh {
                String::new()
            } else {
                ", давно не подтверждалась".to_owned()
            }
        );
        let name = steps
            .iter()
            .filter_map(|step| {
                step.strip_prefix("--lua-desync=")
                    .map(|value| value.split(':').next().unwrap_or(value))
            })
            .collect::<Vec<_>>()
            .join(" + ");
        ranked.push((
            fresh,
            match_rank,
            reports.saturating_sub(fails),
            last.to_owned(),
            StrategyCandidate {
                name,
                source,
                steps,
            },
        ));
    }

    ranked.sort_by(|left, right| {
        (right.0, right.1, right.2, &right.3).cmp(&(left.0, left.1, left.2, &left.3))
    });
    Ok(ranked
        .into_iter()
        .take(MAX_COMMUNITY_ITEMS)
        .map(|item| item.4)
        .collect())
}

fn valid_community_step(step: &str) -> bool {
    let Some(value) = step.strip_prefix("--lua-desync=") else {
        return false;
    };
    !value.is_empty()
        && value.len() <= COMMUNITY_MAX_STEP_BYTES
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric()
                || matches!(byte, b'_' | b':' | b'.' | b',' | b'=' | b'+' | b'-')
        })
}

fn nonnegative_count(value: Option<&Value>) -> u64 {
    value
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .clamp(0, 1_000_000) as u64
}

fn parse_date_days(value: &str) -> Option<i64> {
    let bytes = value.as_bytes();
    if bytes.len() != 10
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || !bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
    {
        return None;
    }
    let year = value[0..4].parse::<i64>().ok()?;
    let month = value[5..7].parse::<i64>().ok()?;
    let day = value[8..10].parse::<i64>().ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    // Gregorian civil date to days from 1970-01-01 (Howard Hinnant's formula).
    let year = year - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    Some(era * 146_097 + day_of_era - 719_468)
}

#[cfg(test)]
mod tests {
    use super::router_provider_asn;

    #[tokio::test]
    async fn uses_a_fresh_cached_router_asn_without_network_access() {
        let directory = tempfile::tempdir().unwrap();
        tokio::fs::write(directory.path().join("provider-asn.txt"), "64500\n")
            .await
            .unwrap();
        assert_eq!(router_provider_asn(directory.path()).await.unwrap(), 64500);
    }
}
