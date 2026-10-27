use crate::runtime_profiles::{self, RuntimeProfile};
use std::collections::BTreeSet;
use std::collections::HashMap;
use std::io::Write;
use std::net::IpAddr;
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::path::{Path, PathBuf};

const MAX_PREFIX_RESPONSE_BYTES: usize = 1024 * 1024;
const MAX_LIST_BYTES: usize = 512 * 1024;

pub fn parse_announced_prefixes(json: &[u8]) -> Result<Vec<String>, String> {
    if json.len() > MAX_PREFIX_RESPONSE_BYTES {
        return Err("Ответ RIPEstat превышает лимит 1 МиБ".into());
    }
    let response: serde_json::Value = serde_json::from_slice(json)
        .map_err(|_| "RIPEstat вернул некорректный JSON со списком префиксов".to_owned())?;
    let entries = response
        .pointer("/data/prefixes")
        .and_then(serde_json::Value::as_array)
        .ok_or_else(|| "В ответе RIPEstat не найден список анонсируемых префиксов".to_owned())?;
    if entries.is_empty() {
        return Err("У проверенной AS нет анонсируемых префиксов".into());
    }

    let mut prefixes = BTreeSet::new();
    for entry in entries {
        let prefix = entry
            .get("prefix")
            .and_then(serde_json::Value::as_str)
            .ok_or_else(|| "RIPEstat вернул запись без IP-префикса".to_owned())?;
        let (address, length) = prefix
            .split_once('/')
            .ok_or_else(|| format!("RIPEstat вернул некорректный префикс: {prefix}"))?;
        let address = address
            .parse::<IpAddr>()
            .map_err(|_| format!("RIPEstat вернул некорректный IP-префикс: {prefix}"))?;
        let max_length = if address.is_ipv4() { 32 } else { 128 };
        let length = length
            .parse::<u8>()
            .ok()
            .filter(|length| *length <= max_length)
            .ok_or_else(|| format!("RIPEstat вернул некорректную длину префикса: {prefix}"))?;
        prefixes.insert(canonical_prefix(address, length));
    }

    let prefixes = prefixes.into_iter().collect::<Vec<_>>();
    let output_bytes = prefixes
        .iter()
        .map(|prefix| prefix.len() + 1)
        .sum::<usize>();
    if output_bytes > MAX_LIST_BYTES {
        return Err("Список ASN превышает лимит 512 КиБ".into());
    }
    Ok(prefixes)
}

fn canonical_prefix(address: IpAddr, length: u8) -> String {
    match address {
        IpAddr::V4(address) => {
            let mask = if length == 0 {
                0
            } else {
                u32::MAX << (32 - u32::from(length))
            };
            format!(
                "{}/{length}",
                std::net::Ipv4Addr::from(u32::from(address) & mask)
            )
        }
        IpAddr::V6(address) => {
            let mask = if length == 0 {
                0
            } else {
                u128::MAX << (128 - u32::from(length))
            };
            format!(
                "{}/{length}",
                std::net::Ipv6Addr::from(u128::from(address) & mask)
            )
        }
    }
}

pub fn address_is_in_prefixes(address: IpAddr, prefixes: &[String]) -> bool {
    prefixes
        .iter()
        .any(|prefix| crate::site_diagnostics::list_entry_matches("", &[address], prefix, true))
}

pub fn find_joinable_profile(
    config: &str,
    tested_host: &str,
    tested_ip: IpAddr,
    strategy_step: &str,
    list_contents: &HashMap<String, String>,
) -> Result<Option<usize>, String> {
    if !strategy_step.starts_with("--lua-desync=")
        || strategy_step.chars().any(char::is_whitespace)
        || tested_host.is_empty()
    {
        return Ok(None);
    }
    let parsed = runtime_profiles::from_config(config, "freeze-join")?;
    let custom = parsed
        .profiles
        .iter()
        .filter(|profile| profile.source == "NFQWS_ARGS_CUSTOM")
        .collect::<Vec<_>>();
    let candidates = custom
        .iter()
        .copied()
        .filter(|profile| {
            is_joinable_target(
                profile,
                tested_host,
                tested_ip,
                strategy_step,
                list_contents,
            )
        })
        .collect::<Vec<_>>();
    if candidates.len() != 1 {
        return Ok(None);
    }
    let profile = candidates[0];
    let has_ambiguous_predecessor = custom.iter().any(|earlier| {
        earlier.index < profile.index
            && profile_may_match_address(earlier, tested_host, tested_ip, list_contents)
                != Some(false)
    });
    Ok((!has_ambiguous_predecessor).then_some(profile.index))
}

pub fn build_joined_config(
    config: &str,
    profile_index: usize,
    list_id: &str,
) -> Result<String, String> {
    let asn = list_id
        .strip_prefix("ipset_as")
        .and_then(|value| value.strip_suffix(".list"))
        .and_then(|value| value.parse::<u32>().ok())
        .filter(|asn| *asn > 0)
        .ok_or_else(|| "Недопустимое имя ASN-списка".to_owned())?;
    let list_argument = format!("--ipset=lists/ipset_as{asn}.list");
    let document = crate::config_document::ConfigDocument::parse(config)?;
    let custom_args = document.literal_or_mode_value("NFQWS_ARGS_CUSTOM")?;
    let mut profiles = runtime_profiles::split_custom_profile_args(&custom_args)?;
    let target = profiles
        .get_mut(
            profile_index
                .checked_sub(1)
                .ok_or_else(|| "Некорректный номер профиля".to_owned())?,
        )
        .ok_or_else(|| "Профиль NFQWS_ARGS_CUSTOM больше не существует".to_owned())?;
    let ipset_references = option_values(target, "--ipset")
        .ok_or_else(|| "В профиле найден параметр --ipset без значения".to_owned())?;
    if ipset_references
        .iter()
        .any(|path| path.trim_end_matches('/').rsplit('/').next() == Some(list_id))
    {
        return Err("ASN-список уже указан в выбранном профиле".into());
    }
    target.push(list_argument);
    let encoded = runtime_profiles::encode_custom_profile_args(&profiles)?;
    document.set_literal_all("NFQWS_ARGS_CUSTOM", &encoded)
}

pub fn profile_list_references(args: &[String]) -> Result<Vec<String>, String> {
    let mut references = Vec::new();
    for flag in [
        "--ipset",
        "--hostlist",
        "--ipset-exclude",
        "--hostlist-exclude",
    ] {
        let values = option_values(args, flag)
            .ok_or_else(|| format!("В профиле найден параметр {flag} без пути"))?;
        for value in values {
            if value.is_empty() {
                return Err(format!("В профиле найден параметр {flag} с пустым путём"));
            }
            references.push(value.to_owned());
        }
    }
    Ok(references)
}

pub fn valid_asn_list_id(list_id: &str) -> bool {
    list_id
        .strip_prefix("ipset_as")
        .and_then(|value| value.strip_suffix(".list"))
        .and_then(|value| value.parse::<u32>().ok())
        .is_some_and(|asn| asn > 0)
}

pub fn asn_list_exists(lists_directory: &Path, list_id: &str) -> Result<bool, String> {
    if !valid_asn_list_id(list_id) {
        return Err("Недопустимое имя ASN-списка".into());
    }
    let entries = std::fs::read_dir(lists_directory)
        .map_err(|error| format!("Не удалось прочитать каталог IP-списков: {error}"))?;
    for entry in entries {
        let entry = entry.map_err(|error| error.to_string())?;
        if entry
            .file_name()
            .to_str()
            .is_some_and(|name| name.eq_ignore_ascii_case(list_id))
        {
            return Ok(true);
        }
    }
    Ok(false)
}

pub fn create_list_exclusive(
    lists_directory: &Path,
    list_id: &str,
    content: &str,
) -> Result<PathBuf, String> {
    if !valid_asn_list_id(list_id) {
        return Err("Недопустимое имя ASN-списка".into());
    }
    if content.is_empty() || content.len() > MAX_LIST_BYTES || content.as_bytes().contains(&0) {
        return Err("Содержимое ASN-списка пустое, слишком большое или содержит NUL-байт".into());
    }
    let metadata = std::fs::symlink_metadata(lists_directory)
        .map_err(|error| format!("Не удалось проверить каталог IP-списков: {error}"))?;
    if !metadata.is_dir() || metadata.file_type().is_symlink() {
        return Err("Каталог IP-списков не является обычным каталогом".into());
    }
    let directory = lists_directory
        .canonicalize()
        .map_err(|error| format!("Не удалось проверить путь IP-списков: {error}"))?;
    let path = directory.join(list_id);
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| {
            if error.kind() == std::io::ErrorKind::AlreadyExists {
                "ASN-список уже существует; существующий файл не изменён".to_owned()
            } else {
                format!("Не удалось безопасно создать ASN-список: {error}")
            }
        })?;
    let result = (|| {
        file.write_all(content.as_bytes())
            .map_err(|error| format!("Не удалось записать ASN-список: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("Не удалось синхронизировать ASN-список: {error}"))?;
        #[cfg(unix)]
        file.set_permissions(std::fs::Permissions::from_mode(0o600))
            .map_err(|error| format!("Не удалось защитить ASN-список: {error}"))?;
        Ok::<(), String>(())
    })();
    if let Err(message) = result {
        let _ = std::fs::remove_file(&path);
        return Err(message);
    }
    Ok(path)
}

fn is_joinable_target(
    profile: &RuntimeProfile,
    tested_host: &str,
    tested_ip: IpAddr,
    strategy_step: &str,
    list_contents: &HashMap<String, String>,
) -> bool {
    if profile.source != "NFQWS_ARGS_CUSTOM"
        || profile.args.iter().any(|argument| {
            argument.starts_with("--hostlist")
                || argument.starts_with("--autohostlist")
                || argument.starts_with("--ipset-ip")
                || argument.starts_with("--ipset-exclude")
                || argument.starts_with("--hostlist-exclude")
        })
        || !profile
            .steps
            .iter()
            .any(|step| normalize_strategy_step(step) == normalize_strategy_step(strategy_step))
        || !profile_matches_tls_443(&profile.args)
    {
        return false;
    }
    let Some(ipsets) = option_values(&profile.args, "--ipset") else {
        return false;
    };
    !ipsets.is_empty()
        && ipsets.iter().all(|path| list_contents.contains_key(*path))
        && ipsets.iter().any(|path| {
            list_contents.get(*path).is_some_and(|content| {
                content.lines().any(|line| {
                    crate::site_diagnostics::list_entry_matches(
                        tested_host,
                        &[tested_ip],
                        line,
                        true,
                    )
                })
            })
        })
}

fn normalize_strategy_step(step: &str) -> Option<String> {
    let value = step.strip_prefix("--lua-desync=")?;
    if value.is_empty() || value.chars().any(char::is_whitespace) {
        return None;
    }
    let mut parts = value.split(':');
    let function = parts.next()?.trim();
    if function.is_empty() {
        return None;
    }
    let mut parameters = Vec::new();
    let mut seen = BTreeSet::new();
    for part in parts {
        if part.is_empty() {
            return None;
        }
        let (key, value) = part
            .split_once('=')
            .map_or((part, None), |(key, value)| (key, Some(value)));
        if key.is_empty() || !seen.insert(key.to_owned()) {
            return None;
        }
        if key != "strategy" {
            parameters.push((key, value));
        }
    }
    parameters.sort_by(|left, right| left.0.cmp(right.0));
    let mut normalized = format!("--lua-desync={function}");
    for (key, value) in parameters {
        normalized.push(':');
        normalized.push_str(key);
        if let Some(value) = value {
            normalized.push('=');
            normalized.push_str(value);
        }
    }
    Some(normalized)
}

fn profile_may_match_address(
    profile: &RuntimeProfile,
    tested_host: &str,
    tested_ip: IpAddr,
    list_contents: &HashMap<String, String>,
) -> Option<bool> {
    if profile.args.iter().any(|argument| {
        argument.starts_with("--autohostlist")
            || argument.starts_with("--ipset-ip")
            || argument.starts_with("--ipset-exclude")
            || argument.starts_with("--hostlist-exclude")
    }) {
        return None;
    }
    if !profile_matches_tls_443(&profile.args) {
        return Some(false);
    }
    let ipsets = option_values(&profile.args, "--ipset")?;
    let hostlists = option_values(&profile.args, "--hostlist")?;
    if ipsets.is_empty() && hostlists.is_empty() {
        return None;
    }
    let ip_matches = if ipsets.is_empty() {
        true
    } else {
        let mut found = false;
        for path in ipsets {
            let content = list_contents.get(path)?;
            found |= content.lines().any(|line| {
                crate::site_diagnostics::list_entry_matches(tested_host, &[tested_ip], line, true)
            });
        }
        found
    };
    let host_matches = if hostlists.is_empty() {
        true
    } else {
        let mut found = false;
        for path in hostlists {
            let content = list_contents.get(path)?;
            found |= content.lines().any(|line| {
                crate::site_diagnostics::list_entry_matches(tested_host, &[], line, false)
            });
        }
        found
    };
    Some(ip_matches && host_matches)
}

fn option_values<'a>(args: &'a [String], option: &str) -> Option<Vec<&'a str>> {
    let mut values = Vec::new();
    let mut index = 0;
    while index < args.len() {
        let argument = &args[index];
        if let Some(value) = argument.strip_prefix(&format!("{option}=")) {
            values.push(value);
        } else if argument == option {
            index += 1;
            values.push(args.get(index)?.as_str());
        }
        index += 1;
    }
    Some(values)
}

fn profile_matches_tls_443(args: &[String]) -> bool {
    let Some(tcp_filters) = option_values(args, "--filter-tcp") else {
        return false;
    };
    if !tcp_filters
        .iter()
        .any(|filter| port_filter_matches(filter, 443))
    {
        return false;
    }
    let Some(l7_filters) = option_values(args, "--filter-l7") else {
        return false;
    };
    if !l7_filters.is_empty()
        && !l7_filters.iter().any(|filter| {
            filter
                .split(',')
                .any(|protocol| matches!(protocol, "tls" | "all"))
        })
    {
        return false;
    }
    let Some(payload_filters) = option_values(args, "--payload") else {
        return false;
    };
    payload_filters.is_empty()
        || payload_filters.iter().any(|filter| {
            filter
                .split(',')
                .any(|payload| matches!(payload, "tls_client_hello" | "known" | "all"))
        })
}

fn port_filter_matches(value: &str, wanted: u16) -> bool {
    let mut included = false;
    for part in value.split(',') {
        let (excluded, part) = part
            .strip_prefix('~')
            .map_or((false, part), |part| (true, part));
        let matches = part == "*"
            || part.parse::<u16>().is_ok_and(|port| port == wanted)
            || part.split_once('-').is_some_and(|(low, high)| {
                low.parse::<u16>().is_ok_and(|low| {
                    high.parse::<u16>()
                        .is_ok_and(|high| low <= wanted && wanted <= high)
                })
            });
        if matches && excluded {
            return false;
        }
        included |= matches;
    }
    included
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;
    use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

    use super::{
        address_is_in_prefixes, build_joined_config, create_list_exclusive, find_joinable_profile,
        normalize_strategy_step, parse_announced_prefixes,
    };

    #[test]
    fn parses_and_deduplicates_ripe_announced_prefixes() {
        let json = br#"{"data":{"prefixes":[{"prefix":"8.8.8.1/24"},{"prefix":"2001:4860::/32"},{"prefix":"8.8.8.0/24"}]}}"#;
        assert_eq!(
            parse_announced_prefixes(json).unwrap(),
            ["2001:4860::/32", "8.8.8.0/24"]
        );
    }

    #[test]
    fn rejects_malformed_or_empty_ripe_announcements() {
        assert!(parse_announced_prefixes(b"not json").is_err());
        assert!(parse_announced_prefixes(br#"{"data":{"prefixes":[]}}"#).is_err());
        assert!(
            parse_announced_prefixes(br#"{"data":{"prefixes":[{"prefix":"not-a-cidr"}]}}"#)
                .is_err()
        );
    }

    #[test]
    fn rejects_announced_prefixes_that_would_exceed_the_list_size_limit() {
        let prefixes = (0..40_000_u32)
            .map(|value| {
                let third = (value / 256) % 256;
                let fourth = value % 256;
                format!(r#"{{"prefix":"198.18.{third}.{fourth}/32"}}"#)
            })
            .collect::<Vec<_>>()
            .join(",");
        let json = format!(r#"{{"data":{{"prefixes":[{prefixes}]}}}}"#);
        assert!(parse_announced_prefixes(json.as_bytes()).is_err());
    }

    #[test]
    fn checks_both_address_families_and_rejects_non_members() {
        let prefixes = ["8.8.8.0/24".to_owned(), "2001:4860::/32".to_owned()];
        assert!(address_is_in_prefixes(
            IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)),
            &prefixes
        ));
        assert!(address_is_in_prefixes(
            IpAddr::V6("2001:4860:1::1".parse::<Ipv6Addr>().unwrap()),
            &prefixes
        ));
        assert!(!address_is_in_prefixes(
            IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)),
            &prefixes
        ));
    }

    fn profile_config(custom: &str) -> String {
        format!(
            "NFQWS_ARGS_CUSTOM=\"{custom}\"\nNFQWS_ARGS_UDP=\"\"\nNFQWS_ARGS_QUIC=\"\"\nNFQWS_ARGS_IPSET=\"\"\nNFQWS_ARGS=\"\"\nNFQWS_EXTRA_ARGS=\"\"\n"
        )
    }

    #[test]
    fn selects_one_ip_only_custom_profile_containing_ip_and_confirmed_step() {
        let config = profile_config(
            "--filter-tcp=443 --filter-l7=tls --ipset=lists/ipset.list --lua-desync=fake:strategy=3:blob=tls_clienthello",
        );
        let lists = HashMap::from([("lists/ipset.list".to_owned(), "203.0.113.0/24\n".to_owned())]);
        assert_eq!(
            find_joinable_profile(
                &config,
                "example.com",
                "203.0.113.15".parse().unwrap(),
                "--lua-desync=fake:blob=tls_clienthello",
                &lists,
            )
            .unwrap(),
            Some(1)
        );
    }

    #[test]
    fn refuses_custom_profile_when_scope_or_route_is_ambiguous() {
        let ip = "203.0.113.15".parse().unwrap();
        let step = "--lua-desync=fake:blob=tls_clienthello";
        let lists = HashMap::from([
            ("lists/ipset.list".to_owned(), "203.0.113.0/24\n".to_owned()),
            ("lists/other.list".to_owned(), "192.0.2.0/24\n".to_owned()),
            ("lists/sites.list".to_owned(), "example.com\n".to_owned()),
        ]);

        let with_hostlist = profile_config(
            "--filter-tcp=443 --ipset=lists/ipset.list --hostlist=lists/sites.list --lua-desync=fake:blob=tls_clienthello",
        );
        assert_eq!(
            find_joinable_profile(&with_hostlist, "example.com", ip, step, &lists).unwrap(),
            None
        );

        let with_ipset_ip = profile_config(
            "--filter-tcp=443 --ipset=lists/ipset.list --ipset-ip=203.0.113.15 --lua-desync=fake:blob=tls_clienthello",
        );
        assert_eq!(
            find_joinable_profile(&with_ipset_ip, "example.com", ip, step, &lists).unwrap(),
            None
        );

        let changed_step = profile_config(
            "--filter-tcp=443 --ipset=lists/ipset.list --lua-desync=fake:blob=other",
        );
        assert_eq!(
            find_joinable_profile(&changed_step, "example.com", ip, step, &lists).unwrap(),
            None
        );

        let unresolved_list = profile_config(
            "--filter-tcp=443 --ipset=lists/missing.list --lua-desync=fake:blob=tls_clienthello",
        );
        assert_eq!(
            find_joinable_profile(&unresolved_list, "example.com", ip, step, &lists).unwrap(),
            None
        );

        let earlier_broad = profile_config(
            "--filter-tcp=443 --lua-desync=fake:blob=other --new --filter-tcp=443 --ipset=lists/ipset.list --lua-desync=fake:blob=tls_clienthello",
        );
        assert_eq!(
            find_joinable_profile(&earlier_broad, "example.com", ip, step, &lists).unwrap(),
            None
        );

        let two_matches = profile_config(
            "--filter-tcp=443 --ipset=lists/ipset.list --lua-desync=fake:blob=tls_clienthello --new --filter-tcp=443 --ipset=lists/ipset.list --lua-desync=fake:blob=tls_clienthello",
        );
        assert_eq!(
            find_joinable_profile(&two_matches, "example.com", ip, step, &lists).unwrap(),
            None
        );
    }

    #[test]
    fn strategy_match_ignores_parameter_order_and_selected_strategy_number() {
        assert_eq!(
            normalize_strategy_step(
                "--lua-desync=hostfakesplit:tcp_md5:repeats=4:strategy=2:host=ya.ru:tcp_ts=-600000"
            ),
            normalize_strategy_step(
                "--lua-desync=hostfakesplit:repeats=4:tcp_ts=-600000:tcp_md5:host=ya.ru"
            )
        );
        assert_ne!(
            normalize_strategy_step("--lua-desync=hostfakesplit:repeats=4:tcp_md5"),
            normalize_strategy_step("--lua-desync=hostfakesplit:repeats=5:tcp_md5")
        );
        assert_eq!(
            normalize_strategy_step("--lua-desync=hostfakesplit:repeats=4:repeats=5"),
            None
        );
    }

    #[test]
    fn adds_ipset_to_exact_custom_profile_and_preserves_other_profile() {
        let config = profile_config(
            "--filter-tcp=443 --ipset=lists/ipset.list --lua-desync=fake:blob=first --new=video --filter-tcp=443 --ipset=lists/other.list --lua-desync=fake:blob=second",
        );
        let updated = build_joined_config(&config, 2, "ipset_as64500.list").unwrap();
        let profiles = crate::runtime_profiles::from_config(&updated, "revision").unwrap();
        assert_eq!(
            profiles.profiles[1].args.last().unwrap(),
            "--ipset=lists/ipset_as64500.list"
        );
        assert_eq!(
            profiles.profiles[0].args.last().unwrap(),
            "--lua-desync=fake:blob=first"
        );
    }

    #[test]
    fn rejects_invalid_profile_index_list_id_and_duplicate_list_reference() {
        let config =
            profile_config("--filter-tcp=443 --ipset=lists/ipset_as64500.list --lua-desync=fake");
        assert!(
            build_joined_config("NFQWS_ARGS_CUSTOM=\"unterminated", 1, "ipset_as64500.list")
                .is_err()
        );
        assert!(build_joined_config(&config, 1, "ipset_as64500.list").is_err());
        assert!(build_joined_config(&config, 2, "ipset_as64501.list").is_err());
        assert!(build_joined_config(&config, 1, "../outside.list").is_err());
    }

    #[test]
    fn creates_asn_list_exclusively_and_never_overwrites_existing_content() {
        let directory = tempfile::tempdir().unwrap();
        let content = "8.8.8.0/24\n2001:4860::/32\n";
        let path = create_list_exclusive(directory.path(), "ipset_as64500.list", content).unwrap();
        assert_eq!(std::fs::read_to_string(&path).unwrap(), content);
        assert!(
            create_list_exclusive(directory.path(), "ipset_as64500.list", "1.1.1.0/24\n").is_err()
        );
        assert_eq!(std::fs::read_to_string(path).unwrap(), content);
    }
}
