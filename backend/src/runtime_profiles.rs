//! nfqws2 runtime profiles derived from argument groups in nfqws2.conf.

use crate::config_document::ConfigDocument;
use serde::Serialize;

const ARG_GROUPS: [&str; 6] = [
    "NFQWS_ARGS_CUSTOM",
    "NFQWS_ARGS_UDP",
    "NFQWS_ARGS_QUIC",
    "NFQWS_ARGS_IPSET",
    "NFQWS_ARGS",
    "NFQWS_EXTRA_ARGS",
];

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProfile {
    pub id: String,
    pub index: usize,
    pub name: String,
    pub source: String,
    pub editable: bool,
    pub args: Vec<String>,
    pub filters: Vec<String>,
    pub protocols: Vec<String>,
    pub lists: Vec<String>,
    pub steps: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProfilesResponse {
    pub profiles: Vec<RuntimeProfile>,
    pub custom_args: String,
    pub revision: String,
    pub limitation: Option<String>,
}

fn tokens(value: &str) -> Result<Vec<String>, String> {
    if value.len() > 512 * 1024
        || value.chars().any(|character| {
            (character.is_control() && !matches!(character, '\n' | '\r' | '\t'))
                || matches!(character, '\'' | '"' | '`' | '$' | '\\' | ';' | '&' | '|')
        })
    {
        return Err("Параметры профиля содержат shell-синтаксис или превышают 512 КиБ".into());
    }
    Ok(value.split_whitespace().map(str::to_owned).collect())
}

/// Matches nfqws2's `--new` separators. A named separator is represented in
/// the profile body as `--name=...`, the equivalent nfqws2 argument.
pub fn split_custom_profile_args(value: &str) -> Result<Vec<Vec<String>>, String> {
    let args = tokens(value)?;
    if args.len() > 8192 {
        return Err("В пользовательских профилях слишком много параметров".into());
    }
    let mut profiles = vec![Vec::new()];
    for argument in args {
        if argument == "--new" || argument.starts_with("--new=") {
            let name = argument.strip_prefix("--new=").unwrap_or_default();
            if !name.is_empty() {
                profiles.push(vec![format!("--name={name}")]);
            } else {
                profiles.push(Vec::new());
            }
        } else {
            profiles
                .last_mut()
                .expect("first profile exists")
                .push(argument);
        }
    }
    if profiles.len() > 64 {
        return Err("Можно редактировать не больше 64 пользовательских профилей".into());
    }
    Ok(profiles)
}

/// Encodes ordered custom groups back into the single NFQWS_ARGS_CUSTOM
/// argument value without shell quoting or evaluation.
pub fn encode_custom_profile_args(profiles: &[Vec<String>]) -> Result<String, String> {
    if profiles.len() > 64 {
        return Err("Можно сохранить не больше 64 пользовательских профилей".into());
    }
    let mut result = Vec::new();
    for (index, profile) in profiles.iter().enumerate() {
        let mut args = profile.iter();
        if index > 0 {
            let name = profile
                .first()
                .and_then(|argument| argument.strip_prefix("--name="))
                .filter(|name| !name.is_empty());
            if name.is_some_and(|name| {
                name.chars().count() > 64
                    || !name.chars().all(|character| {
                        character.is_alphanumeric() || matches!(character, '_' | '.' | '-')
                    })
            }) {
                return Err("Имя профиля допускает буквы, цифры, точку, дефис и подчёркивание (до 64 символов)".into());
            }
            result.push(name.map_or_else(|| "--new".to_owned(), |name| format!("--new={name}")));
            if name.is_some() {
                args.next();
            }
        }
        for arg in args {
            if arg == "--new" || arg.starts_with("--new=") {
                return Err("Разделители профилей нельзя вводить как параметр шага".into());
            }
            let checked = tokens(arg)?;
            if checked.len() != 1 || checked[0] != *arg {
                return Err("Каждый параметр должен быть одним аргументом nfqws2".into());
            }
            result.push(arg.clone());
        }
    }
    let value = result.join(" ");
    if value.len() > 512 * 1024 {
        return Err("Пользовательские профили превышают лимит 512 КиБ".into());
    }
    Ok(value)
}

fn optional_group(document: &ConfigDocument<'_>, name: &str) -> Result<Vec<String>, String> {
    if !document.contains_assignment(name) {
        return Ok(Vec::new());
    }
    let value = if name == "NFQWS_EXTRA_ARGS" || name == "NFQWS_ARGS_CUSTOM" {
        document.literal_or_mode_value(name)?
    } else {
        document.literal_value(name)?
    };
    tokens(&value)
}

fn describe_profile(
    index: usize,
    source: &str,
    args: Vec<String>,
    part: Option<usize>,
) -> RuntimeProfile {
    let explicit_name = args.iter().find_map(|arg| arg.strip_prefix("--name="));
    let name = explicit_name
        .map(str::to_owned)
        .unwrap_or_else(|| match source {
            "NFQWS_ARGS_CUSTOM" => format!("Свой профиль {}", part.unwrap_or(index)),
            "NFQWS_ARGS_UDP" => "UDP: игры, звонки".into(),
            "NFQWS_ARGS_QUIC_IPSET" => "QUIC по IP".into(),
            "NFQWS_ARGS_QUIC_EXTRA" => "QUIC по сайтам".into(),
            "NFQWS_ARGS_IPSET" => "HTTP/HTTPS по IP".into(),
            _ => "HTTP/HTTPS по сайтам".into(),
        });
    let (mut filters, mut protocols, mut lists, mut steps) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for arg in &args {
        if arg.starts_with("--filter-") && filters.len() < 32 {
            filters.push(arg.clone());
        }
        if (arg.starts_with("--payload=") || arg.starts_with("--filter-7=")) && protocols.len() < 32
        {
            protocols.push(arg.clone());
        }
        if (arg.starts_with("--hostlist")
            || arg.starts_with("--ipset")
            || arg.starts_with("--autohostlist"))
            && lists.len() < 32
        {
            lists.push(arg.clone());
        }
        if arg.starts_with("--lua-desync=") && steps.len() < 64 {
            steps.push(arg.clone());
        }
    }
    RuntimeProfile {
        id: if source == "NFQWS_ARGS_CUSTOM" {
            format!("custom-{}", part.unwrap_or(index))
        } else {
            format!("managed-{index}")
        },
        index,
        name,
        source: source.into(),
        editable: source == "NFQWS_ARGS_CUSTOM",
        args,
        filters,
        protocols,
        lists,
        steps,
    }
}

/// Reproduces the reference's expected profile source order and argument
/// composition, while refusing to evaluate arbitrary shell code.
pub fn from_config(source: &str, revision: &str) -> Result<RuntimeProfilesResponse, String> {
    let document = ConfigDocument::parse(source)?;
    let mut groups = std::collections::HashMap::new();
    for name in ARG_GROUPS {
        groups.insert(name, optional_group(&document, name)?);
    }
    let custom_args = if document.contains_assignment("NFQWS_ARGS_CUSTOM") {
        document.literal_or_mode_value("NFQWS_ARGS_CUSTOM")?
    } else {
        String::new()
    };
    let mut definitions: Vec<(String, Vec<String>, Option<usize>)> = Vec::new();
    if !custom_args.trim().is_empty() {
        for (part, args) in split_custom_profile_args(&custom_args)?
            .into_iter()
            .enumerate()
        {
            definitions.push(("NFQWS_ARGS_CUSTOM".into(), args, Some(part + 1)));
        }
    }
    let udp = groups.get("NFQWS_ARGS_UDP").cloned().unwrap_or_default();
    if !udp.is_empty() {
        definitions.push(("NFQWS_ARGS_UDP".into(), udp, None));
    }
    let quic = groups.get("NFQWS_ARGS_QUIC").cloned().unwrap_or_default();
    let ipset = groups.get("NFQWS_ARGS_IPSET").cloned().unwrap_or_default();
    let tcp = groups.get("NFQWS_ARGS").cloned().unwrap_or_default();
    let extra = groups.get("NFQWS_EXTRA_ARGS").cloned().unwrap_or_default();
    if !quic.is_empty() {
        if !ipset.is_empty() {
            let mut args = quic.clone();
            args.extend(ipset.clone());
            args.push("--ipset-ip=0.0.0.0".into());
            definitions.push(("NFQWS_ARGS_QUIC_IPSET".into(), args, None));
        }
        let mut args = quic;
        args.extend(extra.clone());
        definitions.push(("NFQWS_ARGS_QUIC_EXTRA".into(), args, None));
    }
    if !ipset.is_empty() {
        let mut args = tcp.clone();
        args.extend(ipset);
        args.push("--ipset-ip=0.0.0.0".into());
        definitions.push(("NFQWS_ARGS_IPSET".into(), args, None));
    }
    let mut args = tcp;
    args.extend(extra);
    definitions.push(("NFQWS_ARGS".into(), args, None));
    let profiles = definitions
        .into_iter()
        .enumerate()
        .map(|(index, (source, args, part))| describe_profile(index + 1, &source, args, part))
        .collect();
    Ok(RuntimeProfilesResponse {
        profiles,
        custom_args,
        revision: revision.into(),
        limitation: None,
    })
}

#[cfg(test)]
mod tests {
    use super::{encode_custom_profile_args, from_config, split_custom_profile_args};

    #[test]
    fn splits_custom_runtime_profiles_at_new_and_keeps_ordered_arguments() {
        let profiles = split_custom_profile_args(
            "--filter-tcp=443 --lua-desync=fake --new=video --filter-tcp=80 --lua-desync=split",
        )
        .unwrap();

        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0], ["--filter-tcp=443", "--lua-desync=fake"]);
        assert_eq!(
            profiles[1],
            ["--name=video", "--filter-tcp=80", "--lua-desync=split"]
        );
    }

    #[test]
    fn accepts_multiline_nfqs2_argument_groups() {
        let profiles = split_custom_profile_args(
            "--filter-tcp=443\n--filter-l7=tls\n--lua-desync=fake:repeats=2",
        )
        .unwrap();
        assert_eq!(
            profiles,
            vec![vec![
                "--filter-tcp=443".to_owned(),
                "--filter-l7=tls".to_owned(),
                "--lua-desync=fake:repeats=2".to_owned(),
            ]]
        );
    }

    #[test]
    fn round_trips_custom_profile_order_without_changing_profile_tokens() {
        let profiles = vec![
            vec![
                "--filter-tcp=443".to_owned(),
                "--lua-desync=fake".to_owned(),
            ],
            vec!["--name=video".to_owned(), "--filter-tcp=80".to_owned()],
        ];

        let encoded = encode_custom_profile_args(&profiles).unwrap();
        assert_eq!(
            encoded,
            "--filter-tcp=443 --lua-desync=fake --new=video --filter-tcp=80"
        );
        assert_eq!(split_custom_profile_args(&encoded).unwrap(), profiles);
    }

    #[test]
    fn rejects_shell_fragments_but_allows_removing_every_custom_profile() {
        assert!(split_custom_profile_args("--filter-tcp=443; reboot").is_err());
        assert!(split_custom_profile_args("--filter-tcp=$PORT").is_err());
        assert!(
            encode_custom_profile_args(&[
                vec!["--filter-tcp=443".into()],
                vec!["--name=bad name".into()]
            ])
            .is_err()
        );
        assert_eq!(encode_custom_profile_args(&[]).unwrap(), "");
    }

    #[test]
    fn matches_reference_source_order_and_marks_only_custom_profiles_editable() {
        let config = "MODE_LIST='--hostlist=lists/user.list'\nNFQWS_ARGS_CUSTOM=\"--new --filter-tcp=443 --lua-desync=fake --new=video --filter-tcp=80 --lua-desync=split\"\nNFQWS_ARGS_UDP=\"--filter-udp=443 --payload=quic_initial\"\nNFQWS_ARGS_QUIC=\"--filter-udp=443 --payload=quic_initial\"\nNFQWS_ARGS_IPSET=\"--ipset=lists/ipset.list\"\nNFQWS_ARGS=\"--filter-tcp=80,443 --hostlist=lists/user.list\"\nNFQWS_EXTRA_ARGS=\"$MODE_LIST\"\n";
        let response = from_config(config, "rev").unwrap();

        assert_eq!(response.profiles.len(), 8);
        assert_eq!(response.profiles[0].name, "Свой профиль 1");
        assert_eq!(response.profiles[1].name, "Свой профиль 2");
        assert_eq!(response.profiles[2].name, "video");
        assert_eq!(response.profiles[3].source, "NFQWS_ARGS_UDP");
        assert_eq!(response.profiles[4].source, "NFQWS_ARGS_QUIC_IPSET");
        assert!(
            response.profiles[0].editable
                && response.profiles[1].editable
                && response.profiles[2].editable
        );
        assert!(
            response.profiles[3..]
                .iter()
                .all(|profile| !profile.editable)
        );
        assert_eq!(response.profiles[1].filters, ["--filter-tcp=443"]);
        assert_eq!(response.profiles[1].steps, ["--lua-desync=fake"]);
        assert!(response.custom_args.contains("--new=video"));
    }

    #[test]
    fn refuses_dynamic_shell_values_in_profile_sources() {
        let config = "NFQWS_ARGS_CUSTOM=\"$PORT --new --filter-tcp=443\"\n";
        assert!(from_config(config, "rev").is_err());
    }
}
