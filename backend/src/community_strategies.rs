use serde::{Deserialize, Serialize};
use std::{
    net::IpAddr,
    path::Path,
    time::{SystemTime, UNIX_EPOCH},
};

pub const COMMUNITY_REPO: &str = "https://github.com/zemidala/nfqws2-strategies";
const COMMUNITY_RAW: &str = "https://raw.githubusercontent.com/zemidala/nfqws2-strategies/main/";
const COMMUNITY_TTL: u64 = 24 * 60 * 60;
const COMMUNITY_ERROR_TTL: u64 = 10 * 60;
const COMMUNITY_MAX_FILE_BYTES: usize = 1024 * 1024;
pub const COMMUNITY_MAX_CANDIDATES: usize = 8;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct ProviderSettings {
    pub asn: Option<u32>,
    pub name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommunityTarget {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub asn: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommunityItem {
    pub id: String,
    pub asn: u32,
    pub provider: String,
    pub proto: String,
    pub steps: Vec<String>,
    pub targets: Vec<CommunityTarget>,
    pub reports: u32,
    pub fails: u32,
    pub last_fail: String,
    pub issues: Vec<u64>,
    pub first: String,
    pub last: String,
    pub nfqws2: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct CachedFile {
    asn: u32,
    checked_at: u64,
    fetched_at: u64,
    error: Option<String>,
    items: Vec<CommunityItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
struct CachedIndex {
    checked_at: u64,
    total: u32,
    providers: u32,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunityResponse {
    pub asn: Option<u32>,
    pub provider: String,
    pub fetched_at: Option<u64>,
    pub error: Option<String>,
    pub total: Option<u32>,
    pub providers: Option<u32>,
    pub items: Vec<CommunityViewItem>,
    pub repo: String,
    pub page: String,
    pub mine: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CommunityViewItem {
    #[serde(flatten)]
    pub item: CommunityItem,
    pub name: String,
    pub fresh: bool,
    pub skipped: bool,
}

#[derive(Debug, Deserialize)]
struct RipeResponse {
    data: Option<serde_json::Value>,
}

pub fn read_provider_settings(data_dir: &Path) -> ProviderSettings {
    std::fs::read(data_dir.join("community-provider.json"))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default()
}

pub fn write_provider_settings(data_dir: &Path, settings: &ProviderSettings) -> Result<(), String> {
    if settings.asn.is_some_and(|asn| asn == 0) || settings.name.len() > 120 {
        return Err("Неверные данные провайдера".into());
    }
    if settings.name.chars().any(char::is_control) {
        return Err("Название провайдера содержит управляющие символы".into());
    }
    std::fs::create_dir_all(data_dir).map_err(|error| error.to_string())?;
    let path = data_dir.join("community-provider.json");
    let temp = data_dir.join("community-provider.json.tmp");
    let bytes = serde_json::to_vec(settings).map_err(|error| error.to_string())?;
    std::fs::write(&temp, bytes).map_err(|error| error.to_string())?;
    std::fs::rename(temp, path).map_err(|error| error.to_string())
}

pub fn parse_provider_file(body: &[u8], expected_asn: u32) -> Result<Vec<CommunityItem>, String> {
    if body.len() > COMMUNITY_MAX_FILE_BYTES {
        return Err("Файл стратегий сообщества превышает лимит 1 МиБ".into());
    }
    let value: serde_json::Value = serde_json::from_slice(body)
        .map_err(|error| format!("Некорректный JSON базы сообщества: {error}"))?;
    let items = value
        .get("items")
        .and_then(serde_json::Value::as_array)
        .ok_or("В файле базы нет списка стратегий")?;
    let mut output = Vec::new();
    let mut ids = std::collections::HashSet::new();
    for value in items.iter().take(1000) {
        let Some(item) = parse_item(value, expected_asn) else {
            continue;
        };
        if ids.insert(item.id.clone()) {
            output.push(item);
        }
    }
    Ok(output)
}

fn parse_item(value: &serde_json::Value, expected_asn: u32) -> Option<CommunityItem> {
    let asn = value.get("asn")?.as_u64()?.try_into().ok()?;
    let id = safe_text(value.get("id")?.as_str()?, 16);
    if asn != expected_asn || id.len() < 8 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    let proto = value.get("proto")?.as_str()?;
    if !matches!(proto, "tls" | "http") {
        return None;
    }
    let raw_steps = value.get("steps")?.as_array()?;
    if raw_steps.is_empty() || raw_steps.len() > 8 {
        return None;
    }
    let steps: Vec<String> = raw_steps
        .iter()
        .map(|step| step.as_str().map(str::to_owned))
        .collect::<Option<_>>()?;
    if steps.iter().any(|step| !valid_community_step(step)) {
        return None;
    }
    let mut targets = Vec::new();
    if let Some(raw_targets) = value.get("targets").and_then(serde_json::Value::as_array) {
        for target in raw_targets.iter().take(30) {
            let host = target
                .get("host")
                .and_then(serde_json::Value::as_str)
                .and_then(valid_public_host);
            let target_asn = target
                .get("asn")
                .and_then(serde_json::Value::as_u64)
                .and_then(|asn| u32::try_from(asn).ok())
                .filter(|asn| *asn > 0);
            if host.is_some() || target_asn.is_some() {
                targets.push(CommunityTarget {
                    host,
                    asn: target_asn,
                });
            }
        }
    }
    Some(CommunityItem {
        id,
        asn,
        provider: safe_text(
            value
                .get("provider")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default(),
            120,
        ),
        proto: proto.to_owned(),
        steps,
        targets,
        reports: value
            .get("reports")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(1)
            .min(u32::MAX as u64) as u32,
        fails: value
            .get("fails")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            .min(u32::MAX as u64) as u32,
        last_fail: safe_text(
            value
                .get("last_fail")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default(),
            10,
        ),
        issues: value
            .get("issues")
            .and_then(serde_json::Value::as_array)
            .into_iter()
            .flatten()
            .filter_map(serde_json::Value::as_u64)
            .take(20)
            .collect(),
        first: safe_text(
            value
                .get("first")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default(),
            10,
        ),
        last: safe_text(
            value
                .get("last")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default(),
            10,
        ),
        nfqws2: safe_text(
            value
                .get("nfqws2")
                .and_then(serde_json::Value::as_str)
                .unwrap_or_default(),
            30,
        ),
    })
}

fn safe_text(input: &str, max_chars: usize) -> String {
    input
        .chars()
        .filter(|char| !char.is_control() && !matches!(char, '<' | '>' | '`' | '$' | '\\'))
        .take(max_chars)
        .collect()
}

fn valid_public_host(input: &str) -> Option<String> {
    let input = input.trim().trim_end_matches('.').to_ascii_lowercase();
    if input.is_empty()
        || input.len() > 253
        || input.parse::<IpAddr>().is_ok()
        || input.split('.').any(|label| {
            label.is_empty()
                || label.len() > 63
                || label.starts_with('-')
                || label.ends_with('-')
                || !label
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
        })
    {
        return None;
    }
    Some(input)
}

fn valid_community_step(step: &str) -> bool {
    let Some(value) = step.strip_prefix("--lua-desync=") else {
        return false;
    };
    let mut pieces = value.split(':');
    let Some(function) = pieces.next() else {
        return false;
    };
    if function.is_empty()
        || function.len() > 41
        || !function
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
    {
        return false;
    }
    let mut count = 0;
    for argument in pieces {
        count += 1;
        if argument.is_empty()
            || argument.len() > 200
            || !argument
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_.,=+%-".contains(&byte))
        {
            return false;
        }
    }
    count <= 40
}

pub fn parse_index(body: &[u8]) -> Result<(u32, u32), String> {
    if body.len() > COMMUNITY_MAX_FILE_BYTES {
        return Err("Индекс базы сообщества превышает лимит 1 МиБ".into());
    }
    let value: serde_json::Value = serde_json::from_slice(body)
        .map_err(|error| format!("Некорректный индекс базы сообщества: {error}"))?;
    let providers = value
        .get("providers")
        .and_then(serde_json::Value::as_array)
        .ok_or("В индексе базы нет списка провайдеров")?;
    Ok((
        value
            .get("total")
            .and_then(serde_json::Value::as_u64)
            .unwrap_or(0)
            .min(u32::MAX as u64) as u32,
        providers.len().min(u32::MAX as usize) as u32,
    ))
}

pub fn is_fresh(item: &CommunityItem, now: u64) -> bool {
    let Some(last) = parse_iso_date(&item.last) else {
        return false;
    };
    now.saturating_sub(last) < 60 * 24 * 60 * 60
}

pub fn picker_candidates(
    items: Vec<CommunityItem>,
    host: &str,
    protocol: &str,
    now: u64,
) -> Vec<crate::strategy_picker::StrategyCandidate> {
    let mut eligible = items
        .into_iter()
        .filter(|item| {
            item.proto == protocol
                && is_fresh(item, now)
                && !(item.fails >= 2 && item.fails > item.reports)
        })
        .map(|item| {
            let exact_target = item
                .targets
                .iter()
                .any(|target| target.host.as_deref() == Some(host));
            let candidate = crate::strategy_picker::StrategyCandidate {
                name: item
                    .steps
                    .iter()
                    .filter_map(|step| step.strip_prefix("--lua-desync="))
                    .map(|step| step.split(':').next().unwrap_or(step))
                    .collect::<Vec<_>>()
                    .join(" + "),
                source: format!(
                    "Стратегии сообщества · AS{}{}",
                    item.asn,
                    if exact_target {
                        " · этот сайт"
                    } else {
                        ""
                    }
                ),
                steps: item.steps.clone(),
            };
            (
                candidate,
                exact_target,
                item.reports.saturating_sub(item.fails),
                item.last,
            )
        })
        .collect::<Vec<_>>();
    eligible.sort_by(|left, right| {
        (
            !left.1,
            std::cmp::Reverse(left.2),
            std::cmp::Reverse(&left.3),
        )
            .cmp(&(
                !right.1,
                std::cmp::Reverse(right.2),
                std::cmp::Reverse(&right.3),
            ))
    });
    eligible
        .into_iter()
        .take(COMMUNITY_MAX_CANDIDATES)
        .map(|entry| entry.0)
        .collect()
}

fn parse_iso_date(value: &str) -> Option<u64> {
    let mut pieces = value.split('-');
    let year: i64 = pieces.next()?.parse().ok()?;
    let month: i64 = pieces.next()?.parse().ok()?;
    let day: i64 = pieces.next()?.parse().ok()?;
    if pieces.next().is_some() || !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let adjusted_year = year - i64::from(month <= 2);
    let era = adjusted_year.div_euclid(400);
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    let days = era * 146097 + day_of_era - 719468;
    u64::try_from(days).ok()?.checked_mul(86400)
}

fn cache_dir(data_dir: &Path) -> std::path::PathBuf {
    data_dir.join("community-strategies")
}

fn cached_file_path(data_dir: &Path, asn: u32) -> std::path::PathBuf {
    cache_dir(data_dir).join(format!("AS{asn}.json"))
}

fn read_cached_file(data_dir: &Path, asn: u32) -> CachedFile {
    std::fs::read(cached_file_path(data_dir, asn))
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .filter(|cache: &CachedFile| cache.asn == asn)
        .unwrap_or(CachedFile {
            asn,
            ..CachedFile::default()
        })
}

fn write_cached_file(data_dir: &Path, cache: &CachedFile) -> Result<(), String> {
    let directory = cache_dir(data_dir);
    std::fs::create_dir_all(&directory).map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec(cache).map_err(|error| error.to_string())?;
    let path = cached_file_path(data_dir, cache.asn);
    let temporary = path.with_extension("json.tmp");
    std::fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    std::fs::rename(temporary, path).map_err(|error| error.to_string())
}

async fn fetch_raw(path: &str) -> Result<(u16, Vec<u8>), String> {
    if !matches!(path, "index.json")
        && !path.strip_prefix("data/AS").is_some_and(|suffix| {
            suffix.strip_suffix(".json").is_some_and(|number| {
                !number.is_empty() && number.bytes().all(|byte| byte.is_ascii_digit())
            })
        })
    {
        return Err("Недопустимый путь базы стратегий".into());
    }
    let url = format!("{COMMUNITY_RAW}{path}");
    let args = vec![
        "--disable".into(),
        "--silent".into(),
        "--show-error".into(),
        "--location".into(),
        "--proto".into(),
        "=https".into(),
        "--connect-timeout".into(),
        "8".into(),
        "--max-time".into(),
        "20".into(),
        "--user-agent".into(),
        "nfqws2-ui-community".into(),
        "--write-out".into(),
        "\n%{http_code}".into(),
        url,
    ];
    let (status, bytes) = crate::site_diagnostics::capture(
        crate::site_diagnostics::curl_program(),
        &args,
        22,
        COMMUNITY_MAX_FILE_BYTES + 64,
    )
    .await?;
    if status != 0 {
        return Err("Не удалось загрузить базу стратегий сообщества".into());
    }
    let Some(split) = bytes.iter().rposition(|byte| *byte == b'\n') else {
        return Err("Ответ базы сообщества не содержит HTTP-статус".into());
    };
    let code = std::str::from_utf8(&bytes[split + 1..])
        .ok()
        .and_then(|value| value.trim().parse().ok())
        .ok_or("Некорректный HTTP-статус базы сообщества")?;
    Ok((code, bytes[..split].to_vec()))
}

async fn fetch_with_retry(path: &str) -> Result<(u16, Vec<u8>), String> {
    match fetch_raw(path).await {
        Ok((0, _)) | Err(_) => {
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            fetch_raw(path).await
        }
        result => result,
    }
}

pub async fn load(
    data_dir: &Path,
    asn: u32,
    force: bool,
) -> Result<(Vec<CommunityItem>, u64, Option<String>), String> {
    if asn == 0 {
        return Err("Сначала определите номер AS провайдера".into());
    }
    let mut cache = read_cached_file(data_dir, asn);
    let ttl = if cache.error.is_some() {
        COMMUNITY_ERROR_TTL
    } else {
        COMMUNITY_TTL
    };
    if !force && cache.checked_at > 0 && now().saturating_sub(cache.checked_at) < ttl {
        return Ok((cache.items, cache.fetched_at, cache.error));
    }
    cache.checked_at = now();
    match fetch_with_retry(&format!("data/AS{asn}.json")).await {
        Ok((404, _)) => {
            cache.fetched_at = now();
            cache.items.clear();
            cache.error = None;
        }
        Ok((200, body)) => match parse_provider_file(&body, asn) {
            Ok(items) => {
                cache.fetched_at = now();
                cache.items = items;
                cache.error = None;
            }
            Err(message) => cache.error = Some(message),
        },
        Ok((status, _)) => cache.error = Some(format!("GitHub ответил HTTP {status}")),
        Err(message) => cache.error = Some(message),
    }
    write_cached_file(data_dir, &cache)?;
    Ok((cache.items, cache.fetched_at, cache.error))
}

pub async fn load_index(data_dir: &Path, force: bool) -> Option<(u32, u32)> {
    let path = cache_dir(data_dir).join("index.json");
    let cached: CachedIndex = std::fs::read(&path)
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    if !force && cached.checked_at > 0 && now().saturating_sub(cached.checked_at) < COMMUNITY_TTL {
        return Some((cached.total, cached.providers));
    }
    let Ok((200, body)) = fetch_with_retry("index.json").await else {
        return (cached.checked_at > 0).then_some((cached.total, cached.providers));
    };
    let Ok((total, providers)) = parse_index(&body) else {
        return (cached.checked_at > 0).then_some((cached.total, cached.providers));
    };
    let updated = CachedIndex {
        checked_at: now(),
        total,
        providers,
    };
    if let Ok(bytes) = serde_json::to_vec(&updated) {
        let _ = std::fs::create_dir_all(cache_dir(data_dir));
        let temporary = path.with_extension("json.tmp");
        if std::fs::write(&temporary, bytes).is_ok() {
            let _ = std::fs::rename(temporary, path);
        }
    }
    Some((total, providers))
}

pub async fn detect_provider() -> Result<ProviderSettings, String> {
    let (status, body) = fetch_ripe("https://stat.ripe.net/data/whats-my-ip/data.json").await?;
    if status != 0 {
        return Err("RIPEstat не смог определить внешний адрес".into());
    }
    let response: RipeResponse =
        serde_json::from_slice(&body).map_err(|_| "Некорректный ответ RIPEstat")?;
    let ip = response
        .data
        .and_then(|data| data.get("ip")?.as_str().map(str::to_owned))
        .ok_or("RIPEstat не сообщил внешний адрес")?;
    let (status, body) = fetch_ripe(&format!(
        "https://stat.ripe.net/data/network-info/data.json?resource={ip}"
    ))
    .await?;
    if status != 0 {
        return Err("RIPEstat не определил ASN внешнего адреса".into());
    }
    let response: RipeResponse =
        serde_json::from_slice(&body).map_err(|_| "Некорректный ответ RIPEstat")?;
    let data = response.data.ok_or("RIPEstat не вернул сведения о сети")?;
    let asn = data
        .get("asns")
        .and_then(serde_json::Value::as_array)
        .and_then(|values| values.first())
        .and_then(|value| {
            value
                .as_u64()
                .or_else(|| value.as_str()?.trim_start_matches("AS").parse().ok())
        })
        .and_then(|value| u32::try_from(value).ok())
        .filter(|value| *value > 0)
        .ok_or("RIPEstat не сообщил ASN внешнего адреса")?;
    let (status, body) = fetch_ripe(&format!(
        "https://stat.ripe.net/data/as-overview/data.json?resource=AS{asn}"
    ))
    .await?;
    let name = if status == 0 {
        serde_json::from_slice::<RipeResponse>(&body)
            .ok()
            .and_then(|response| response.data)
            .and_then(|data| data.get("holder")?.as_str().map(str::to_owned))
            .unwrap_or_default()
    } else {
        String::new()
    };
    Ok(ProviderSettings {
        asn: Some(asn),
        name: safe_text(&name, 120),
    })
}

async fn fetch_ripe(url: &str) -> Result<(i32, Vec<u8>), String> {
    let args = [
        "--disable",
        "--silent",
        "--show-error",
        "--fail",
        "--location",
        "--proto",
        "=https",
        "--connect-timeout",
        "6",
        "--max-time",
        "20",
    ]
    .into_iter()
    .map(str::to_owned)
    .chain(std::iter::once(url.to_owned()))
    .collect::<Vec<_>>();
    crate::site_diagnostics::capture(
        crate::site_diagnostics::curl_program(),
        &args,
        22,
        512 * 1024,
    )
    .await
}

fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_only_bounded_safe_provider_strategies() {
        let body = br#"{"items":[{"id":"0123456789ab","asn":12389,"provider":"ISP","proto":"tls","steps":["--lua-desync=hostfakesplit:host=example.com"],"targets":[{"host":"example.com","asn":64500}],"reports":3,"fails":1,"first":"2026-10-01","last":"2026-10-24"},{"id":"badbadbadbad","asn":12389,"proto":"tls","steps":["--lua-desync=bad:@/etc/passwd"]},{"id":"sameproto1234","asn":12389,"proto":"quic","steps":["--lua-desync=fake"]}]}"#;
        let items = parse_provider_file(body, 12389).unwrap();
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].targets[0].host.as_deref(), Some("example.com"));
        assert_eq!(items[0].reports, 3);
    }

    #[test]
    fn refuses_provider_mismatch_and_unsafe_lua_steps() {
        let body = br#"{"items":[{"id":"0123456789ab","asn":12390,"proto":"tls","steps":["--lua-desync=fake"]}]}"#;
        assert!(parse_provider_file(body, 12389).unwrap().is_empty());
        assert!(!valid_community_step("--lua-desync=fake:@secret.bin"));
        assert!(!valid_community_step("--filter-tcp=443"));
    }

    #[test]
    fn isolates_public_hostnames_and_parses_iso_date_age() {
        assert_eq!(
            valid_public_host("EXAMPLE.com."),
            Some("example.com".into())
        );
        assert_eq!(valid_public_host("127.0.0.1"), None);
        assert_eq!(parse_iso_date("2026-10-27"), Some(1_793_059_200));
        assert!(parse_iso_date("yesterday").is_none());
    }

    #[test]
    fn picker_candidates_skip_stale_and_failed_entries_and_prioritize_matching_site() {
        let body = br#"{"items":[{"id":"0123456789ab","asn":12389,"proto":"tls","steps":["--lua-desync=split"],"targets":[{"host":"other.example"}],"reports":4,"fails":0,"last":"2026-10-27"},{"id":"1123456789ab","asn":12389,"proto":"tls","steps":["--lua-desync=split2"],"targets":[{"host":"example.com"}],"reports":2,"fails":0,"last":"2026-10-26"},{"id":"2123456789ab","asn":12389,"proto":"tls","steps":["--lua-desync=split3"],"reports":1,"fails":3,"last":"2026-10-27"},{"id":"3123456789ab","asn":12389,"proto":"http","steps":["--lua-desync=split4"],"last":"2026-10-27"}]}"#;
        let items = parse_provider_file(body, 12389).unwrap();
        let candidates = picker_candidates(items, "example.com", "tls", 1_793_059_200);
        assert_eq!(candidates.len(), 2);
        assert!(candidates[0].source.ends_with("этот сайт"));
        assert!(candidates[1].source.contains("AS12389"));
    }
}
