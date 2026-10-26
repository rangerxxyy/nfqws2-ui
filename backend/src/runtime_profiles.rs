//! nfqws2 runtime profiles derived from argument groups in nfqws2.conf.

use crate::config_document::ConfigDocument;
use serde::Serialize;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LuaFunctionInfo {
    pub name: String,
    pub file: String,
    pub docs: Vec<String>,
    pub arguments: Vec<String>,
    pub file_arguments: Vec<String>,
    pub standard_groups: Vec<String>,
    pub required: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct LuaStandardGroup {
    pub name: String,
    pub arguments: Vec<String>,
}

fn lua_argument_key_after_reference(value: &str) -> Option<(&str, usize)> {
    let (candidate, consumed) = if let Some(rest) = value.strip_prefix('.') {
        let length = rest
            .bytes()
            .take_while(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'_')
            .count();
        (rest.get(..length)?, length + 1)
    } else if let Some(rest) = value.strip_prefix("['") {
        let length = rest.find("']")?;
        (rest.get(..length)?, length + 4)
    } else {
        let rest = value.strip_prefix("[\"")?;
        let length = rest.find("\"]")?;
        (rest.get(..length)?, length + 4)
    };
    (!candidate.is_empty()
        && candidate
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_'))
    .then_some((candidate, consumed))
}

pub fn parse_lua_standard_args(source: &str) -> Vec<LuaStandardGroup> {
    let mut groups = std::collections::BTreeMap::<String, Vec<String>>::new();
    let mut in_block = false;
    let mut group: Option<String> = None;
    for raw in source.lines() {
        let line = raw.trim();
        if line.starts_with("--[[") {
            in_block = true;
            group = None;
            continue;
        }
        if !in_block {
            continue;
        }
        if line.starts_with("]]") {
            in_block = false;
            group = None;
            continue;
        }
        if let Some(name) = line
            .strip_prefix("standard ")
            .and_then(|value| value.strip_suffix(':'))
        {
            let name = name.trim().replace(' ', "_");
            if !name.is_empty() {
                groups.entry(name.clone()).or_default();
                group = Some(name);
            }
            continue;
        }
        if let (Some(name), Some(entry)) = (&group, line.strip_prefix('*')) {
            let entry = entry.trim();
            let entry = entry
                .split_once(':')
                .filter(|(prefix, _)| !prefix.contains('='))
                .map_or(entry, |(_, rest)| rest.trim());
            let description_start = [" — ", " - "]
                .into_iter()
                .filter_map(|separator| entry.find(separator).map(|position| (position, separator)))
                .min_by_key(|(position, _)| *position);
            let (key, description) =
                description_start.map_or((entry, ""), |(position, separator)| {
                    (
                        &entry[..position],
                        entry[position + separator.len()..].trim(),
                    )
                });
            let key = key.trim();
            if !key.is_empty()
                && key
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || b"_=<>[],-".contains(&byte))
            {
                groups
                    .entry(name.clone())
                    .or_default()
                    .push(if description.is_empty() {
                        key.to_string()
                    } else {
                        format!("{key} — {description}")
                    });
            }
        }
    }
    groups
        .into_iter()
        .map(|(name, arguments)| LuaStandardGroup { name, arguments })
        .collect()
}

/// Extract function help from installed nfqws2 Lua comment blocks. This keeps
/// the editor catalog tied to the actual scripts shipped on the router.
pub fn parse_lua_catalog(file: &str, source: &str) -> Vec<LuaFunctionInfo> {
    let lines = source.lines().collect::<Vec<_>>();
    let mut result = Vec::new();
    let mut docs = Vec::<String>::new();
    let mut file_arguments = std::collections::BTreeSet::<String>::new();
    let mut in_block_comment = false;
    for (index, raw) in lines.iter().enumerate() {
        let line = raw.trim();
        if line.starts_with("--[[") {
            in_block_comment = true;
            docs.clear();
            let inline = line.trim_start_matches("--[[").trim();
            if !inline.is_empty() && !inline.ends_with("]]") {
                docs.push(inline.to_string());
            }
            continue;
        }
        if in_block_comment {
            if line.starts_with("]]") {
                in_block_comment = false;
                continue;
            }
            docs.push(line.to_string());
            continue;
        }
        if let Some(doc) = line.strip_prefix("--").map(str::trim_start) {
            docs.push(doc.to_string());
            if let Some(definition) = doc
                .strip_prefix("arg")
                .and_then(|definition| definition.trim_start().strip_prefix(':'))
            {
                let key = definition
                    .trim()
                    .split([' ', '—', ':', '='])
                    .next()
                    .unwrap_or_default();
                if !key.is_empty()
                    && key.bytes().enumerate().all(|(index, byte)| {
                        byte.is_ascii_lowercase()
                            || (index > 0 && byte.is_ascii_digit())
                            || byte == b'_'
                    })
                {
                    file_arguments.insert(key.to_string());
                }
            }
            continue;
        }
        if let Some(signature) = line.strip_prefix("function ")
            && let Some((name, signature)) = signature.split_once('(')
            && signature.split_once(')').is_some_and(|(parameters, _)| {
                parameters
                    .split(',')
                    .map(str::trim)
                    .take(2)
                    .eq(["ctx", "desync"])
            })
            && !name.is_empty()
            && name.len() <= 48
            && name
                .bytes()
                .all(|byte| byte.is_ascii_lowercase() || byte == b'_' || byte.is_ascii_digit())
        {
            let mut arguments = Vec::new();
            let mut required = Vec::new();
            let mut function_docs = Vec::new();
            let mut standard_groups = Vec::new();
            for doc in &docs {
                if let Some(value) = doc
                    .strip_prefix("arg")
                    .and_then(|value| value.trim_start().strip_prefix(':'))
                {
                    arguments.push(value.trim().to_string());
                } else if let Some(value) = doc
                    .strip_prefix("standard args")
                    .and_then(|value| value.trim_start().strip_prefix(':'))
                {
                    let (groups, note) = value.trim().split_once('.').unwrap_or((value.trim(), ""));
                    standard_groups.extend(
                        groups
                            .split(',')
                            .map(str::trim)
                            .filter(|name| !name.is_empty())
                            .map(|name| name.replace(' ', "_")),
                    );
                    if !note.trim().is_empty() {
                        function_docs.push(note.trim().to_string());
                    }
                } else if !doc.trim().is_empty() && !doc.starts_with("standard args:") {
                    function_docs.push(doc.trim().to_string());
                }
            }
            for body_line in lines.iter().skip(index + 1).take(4096) {
                let trimmed_body_line = body_line.trim();
                if body_line.len() == body_line.trim_start().len()
                    && (trimmed_body_line.starts_with("function ")
                        || trimmed_body_line.starts_with("local function "))
                {
                    break;
                }
                let mut rest = *body_line;
                while let Some(position) = rest.find("desync.arg") {
                    rest = &rest[position + "desync.arg".len()..];
                    let Some((key, consumed)) = lua_argument_key_after_reference(rest) else {
                        continue;
                    };
                    let already_documented = arguments.iter().any(|argument| {
                        argument
                            .split([' ', '—', ':', '='])
                            .next()
                            .is_some_and(|documented| documented == key)
                    });
                    if !already_documented {
                        arguments.push(format!("{key} — обнаружен в коде функции"));
                    }
                    rest = &rest[consumed..];
                }
                for marker in ["'", "\""] {
                    let needle = format!("{marker} arg required");
                    let mut rest = *body_line;
                    while let Some(position) = rest.find(&needle) {
                        let before = &rest[..position];
                        if let Some(start) = before.rfind(marker) {
                            let argument = &before[start + 1..];
                            if !argument.is_empty()
                                && argument
                                    .bytes()
                                    .all(|byte| byte.is_ascii_lowercase() || byte == b'_')
                            {
                                required.push(argument.to_string());
                            }
                        }
                        rest = &rest[position + needle.len()..];
                    }
                }
            }
            required.sort();
            required.dedup();
            result.push(LuaFunctionInfo {
                name: name.to_string(),
                file: file.to_string(),
                docs: function_docs,
                arguments,
                file_arguments: Vec::new(),
                standard_groups,
                required,
            });
        }
        docs.clear();
    }
    for function in &mut result {
        function.file_arguments = file_arguments.iter().cloned().collect();
    }
    result
}

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
    pub payloads: Vec<String>,
    pub lists: Vec<String>,
    pub steps: Vec<String>,
    /// Exact config variables that make up a managed profile. Some profiles
    /// combine shared base arguments with IP or site-list settings.
    pub components: Vec<RuntimeProfileComponent>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProfileComponent {
    pub source: String,
    pub args: Vec<String>,
    pub editable: bool,
    pub note: Option<String>,
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
                name.chars().count() > 32
                    || !name.chars().all(|character| {
                        character.is_alphanumeric() || matches!(character, '_' | '.' | '-')
                    })
            }) {
                return Err("Имя профиля допускает буквы, цифры, точку, дефис и подчёркивание (до 32 символов)".into());
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
    // Keep one argument per physical line in the shell config. Newlines are
    // whitespace to nfqws2, but make custom strategies readable and preserve
    // the multiline profile layout after edits in the UI.
    let value = result.join("\n");
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
    let (mut filters, mut protocols, mut payloads, mut lists, mut steps) =
        (Vec::new(), Vec::new(), Vec::new(), Vec::new(), Vec::new());
    for arg in &args {
        if arg.starts_with("--filter-")
            && !arg.starts_with("--filter-7=")
            && !arg.starts_with("--filter-l7=")
            && filters.len() < 32
        {
            filters.push(arg.clone());
        }
        if (arg.starts_with("--filter-7=") || arg.starts_with("--filter-l7="))
            && protocols.len() < 32
        {
            protocols.push(arg.clone());
        }
        if arg.starts_with("--payload=") && payloads.len() < 32 {
            payloads.push(arg.clone());
        }
        if (arg.starts_with("--hostlist")
            || arg.starts_with("--ipset")
            || arg.starts_with("--autohostlist"))
            && lists.len() < 32
        {
            lists.push(arg.clone());
        }
        if (arg.starts_with("--lua-desync=")
            || arg.starts_with("--payload=")
            || arg.starts_with("--in-range=")
            || arg.starts_with("--out-range="))
            && steps.len() < 64
        {
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
        payloads,
        lists,
        steps,
        components: Vec::new(),
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
    let quic = groups.get("NFQWS_ARGS_QUIC").cloned().unwrap_or_default();
    let ipset = groups.get("NFQWS_ARGS_IPSET").cloned().unwrap_or_default();
    let tcp = groups.get("NFQWS_ARGS").cloned().unwrap_or_default();
    let extra = groups.get("NFQWS_EXTRA_ARGS").cloned().unwrap_or_default();
    // Keep all five managed entries visible even when a corresponding group is
    // empty. This lets the UI configure a previously unused transport/profile
    // without making users first edit raw config or infer a missing profile.
    definitions.push(("NFQWS_ARGS_UDP".into(), udp.clone(), None));
    let mut quic_ip = quic.clone();
    quic_ip.extend(ipset.clone());
    if !ipset.is_empty() {
        quic_ip.push("--ipset-ip=0.0.0.0".into());
    }
    definitions.push(("NFQWS_ARGS_QUIC_IPSET".into(), quic_ip, None));
    let mut quic_sites = quic.clone();
    quic_sites.extend(extra.clone());
    definitions.push(("NFQWS_ARGS_QUIC_EXTRA".into(), quic_sites, None));
    let mut tcp_ip = tcp.clone();
    tcp_ip.extend(ipset.clone());
    if !ipset.is_empty() {
        tcp_ip.push("--ipset-ip=0.0.0.0".into());
    }
    definitions.push(("NFQWS_ARGS_IPSET".into(), tcp_ip, None));
    let mut args = tcp.clone();
    args.extend(extra.clone());
    definitions.push(("NFQWS_ARGS".into(), args, None));
    let mut profiles: Vec<RuntimeProfile> = definitions
        .into_iter()
        .enumerate()
        .map(|(index, (source, args, part))| describe_profile(index + 1, &source, args, part))
        .collect();
    let extra_mode = document.referenced_mode("NFQWS_EXTRA_ARGS");
    for profile in &mut profiles {
        let components: Vec<(&str, &Vec<String>)> = match profile.source.as_str() {
            "NFQWS_ARGS_UDP" => vec![("NFQWS_ARGS_UDP", &udp)],
            "NFQWS_ARGS_QUIC_IPSET" => {
                vec![("NFQWS_ARGS_QUIC", &quic), ("NFQWS_ARGS_IPSET", &ipset)]
            }
            "NFQWS_ARGS_QUIC_EXTRA" => {
                vec![("NFQWS_ARGS_QUIC", &quic), ("NFQWS_EXTRA_ARGS", &extra)]
            }
            "NFQWS_ARGS_IPSET" => vec![("NFQWS_ARGS", &tcp), ("NFQWS_ARGS_IPSET", &ipset)],
            "NFQWS_ARGS" => vec![("NFQWS_ARGS", &tcp), ("NFQWS_EXTRA_ARGS", &extra)],
            _ => Vec::new(),
        };
        profile.components = components
            .into_iter()
            .map(|(source, args)| RuntimeProfileComponent {
                source: source.into(),
                args: args.clone(),
                editable: source != "NFQWS_EXTRA_ARGS" || extra_mode.is_none(),
                note: (source == "NFQWS_EXTRA_ARGS")
                    .then(|| extra_mode.as_ref().map(|mode| format!("Используется режим ${mode}; его выбор изменяется в общих настройках конфигурации.")))
                    .flatten(),
            })
            .collect();
    }
    Ok(RuntimeProfilesResponse {
        profiles,
        custom_args,
        revision: revision.into(),
        limitation: None,
    })
}

#[cfg(test)]
mod tests {
    use super::{
        encode_custom_profile_args, from_config, parse_lua_catalog, parse_lua_standard_args,
        split_custom_profile_args,
    };

    #[test]
    fn lua_catalog_extracts_help_declared_arguments_and_required_keys() {
        let source = "--[[\nSplit a TLS record.\nstandard args: general. Uses common options.\narg: pos — split positions\narg: fooling — fooling mode\n]]\nfunction multisplit(ctx, desync)\n  check_and_die(desync, 'pos' arg required)\nend\n";
        let catalog = parse_lua_catalog("zapret-auto.lua", source);
        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog[0].name, "multisplit");
        assert_eq!(catalog[0].file, "zapret-auto.lua");
        assert_eq!(
            catalog[0].docs,
            ["Split a TLS record.", "Uses common options."]
        );
        assert_eq!(
            catalog[0].arguments,
            ["pos — split positions", "fooling — fooling mode"]
        );
        assert_eq!(catalog[0].standard_groups, ["general"]);
        assert_eq!(catalog[0].required, ["pos"]);
        assert!(catalog[0].file_arguments.is_empty());
    }

    #[test]
    fn lua_catalog_infers_undocumented_keys_read_directly_by_the_function() {
        let source = "--[[\nSplit packets.\narg: pos=<marker> — split position\n]]\nfunction fakedsplit(ctx, desync)\n  if desync.arg.oob then\n    send_oob(desync.arg.oob_data)\n  end\n  local order = desync.arg.altorder\n  if desync.arg.nofake2 then return end\nend\nfunction following(ctx, desync)\n  local ignored = desync.arg.other\nend\n";
        let catalog = parse_lua_catalog("zapret-auto.lua", source);
        let arguments = &catalog[0].arguments;
        assert!(
            arguments
                .iter()
                .any(|argument| argument.starts_with("pos="))
        );
        for key in ["oob", "oob_data", "altorder", "nofake2"] {
            assert!(
                arguments.iter().any(|argument| argument.starts_with(key)),
                "missing inferred Lua argument {key}"
            );
        }
    }

    #[test]
    fn lua_catalog_infers_keys_from_dot_and_quoted_bracket_access() {
        let source = "function custom(ctx, desync)\n  local a = desync.arg.dot_key\n  local b = desync.arg['single_key']\n  local c = desync.arg[\"double_key\"]\nend\n";
        let catalog = parse_lua_catalog("custom.lua", source);
        for key in ["dot_key", "single_key", "double_key"] {
            assert!(
                catalog[0]
                    .arguments
                    .iter()
                    .any(|argument| argument.starts_with(key)),
                "missing Lua key {key}"
            );
        }
    }

    #[test]
    fn lua_catalog_does_not_leak_helper_parameters_into_the_previous_function() {
        let source = "function first(ctx, desync)\n  local a = desync.arg.first_key\nend\nfunction helper()\n  local b = desync.arg.helper_key\nend\nfunction second(ctx, desync)\n  local c = desync.arg.second_key\nend\n";
        let catalog = parse_lua_catalog("custom.lua", source);
        assert!(
            catalog[0]
                .arguments
                .iter()
                .any(|item| item.starts_with("first_key"))
        );
        assert!(
            !catalog[0]
                .arguments
                .iter()
                .any(|item| item.starts_with("helper_key"))
        );
        assert!(
            catalog[1]
                .arguments
                .iter()
                .any(|item| item.starts_with("second_key"))
        );
    }

    #[test]
    fn lua_catalog_attaches_file_level_arguments_to_every_function_in_that_script() {
        let source = "-- arg: detect_host — hostname detector\nfunction first(ctx, desync)\nend\nfunction second(ctx, desync)\nend\n";
        let catalog = parse_lua_catalog("zapret-auto.lua", source);
        assert_eq!(catalog.len(), 2);
        assert_eq!(catalog[0].file_arguments, ["detect_host"]);
        assert_eq!(catalog[1].file_arguments, ["detect_host"]);
    }

    #[test]
    fn lua_standard_argument_groups_keep_keys_and_descriptions() {
        let source =
            "--[[\nstandard general:\n* repeats=2 - repeat count\n* tcp_ack - send ACK\n]]\n";
        let groups = parse_lua_standard_args(source);
        assert_eq!(groups.len(), 1);
        assert_eq!(groups[0].name, "general");
        assert_eq!(
            groups[0].arguments,
            ["repeats=2 — repeat count", "tcp_ack — send ACK"]
        );
    }

    #[test]
    fn lua_standard_arguments_match_reference_prefixes_and_unicode_descriptions() {
        let source = "--[[\nstandard general:\n* payload: payload=all — accept any payload\n* direction: pos=<marker> - split positions\n* tcp_ack — acknowledge offset\n]]\n";
        let groups = parse_lua_standard_args(source);
        assert_eq!(groups.len(), 1);
        assert_eq!(
            groups[0].arguments,
            [
                "payload=all — accept any payload",
                "pos=<marker> — split positions",
                "tcp_ack — acknowledge offset",
            ]
        );
    }

    #[test]
    fn lua_catalog_accepts_reference_function_spacing_and_file_argument_spacing() {
        let source = "--arg : detector2=<name> — host detector\nfunction custom(ctx,desync)\n  return desync.arg.detector2\nend\n";
        let catalog = parse_lua_catalog("custom.lua", source);
        assert_eq!(catalog.len(), 1);
        assert_eq!(catalog[0].file_arguments, ["detector2"]);
        assert!(
            catalog[0]
                .arguments
                .iter()
                .any(|item| item.starts_with("detector2"))
        );
    }

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
            "--filter-tcp=443\n--lua-desync=fake\n--new=video\n--filter-tcp=80"
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
        assert!(
            encode_custom_profile_args(&[
                vec!["--filter-tcp=443".into()],
                vec![format!("--name={}", "x".repeat(33))]
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
        assert_eq!(response.profiles[3].components[0].source, "NFQWS_ARGS_UDP");
        assert_eq!(response.profiles[6].components[0].source, "NFQWS_ARGS");
        assert_eq!(
            response.profiles[6].components[1].source,
            "NFQWS_ARGS_IPSET"
        );
        assert_eq!(response.profiles[7].components[0].source, "NFQWS_ARGS");
        assert_eq!(
            response.profiles[7].components[1].source,
            "NFQWS_EXTRA_ARGS"
        );
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
        assert!(response.profiles[3..].iter().all(|profile| {
            profile.components.first().is_some_and(|component| {
                component.editable && component.source != "NFQWS_EXTRA_ARGS"
            })
        }));
        assert_eq!(
            response.profiles[3..]
                .iter()
                .map(|profile| profile.components[0].source.as_str())
                .collect::<Vec<_>>(),
            [
                "NFQWS_ARGS_UDP",
                "NFQWS_ARGS_QUIC",
                "NFQWS_ARGS_QUIC",
                "NFQWS_ARGS",
                "NFQWS_ARGS",
            ]
        );
        assert_eq!(
            response.profiles[3..]
                .iter()
                .map(|profile| profile.name.as_str())
                .collect::<Vec<_>>(),
            [
                "UDP: игры, звонки",
                "QUIC по IP",
                "QUIC по сайтам",
                "HTTP/HTTPS по IP",
                "HTTP/HTTPS по сайтам",
            ]
        );
        assert_eq!(response.profiles[1].filters, ["--filter-tcp=443"]);
        assert_eq!(response.profiles[1].steps, ["--lua-desync=fake"]);
        assert!(response.custom_args.contains("--new=video"));
    }

    #[test]
    fn exposes_all_five_managed_profiles_when_their_config_groups_are_empty() {
        let response = from_config("", "rev").unwrap();
        assert_eq!(response.profiles.len(), 5);
        assert_eq!(
            response
                .profiles
                .iter()
                .map(|profile| profile.name.as_str())
                .collect::<Vec<_>>(),
            [
                "UDP: игры, звонки",
                "QUIC по IP",
                "QUIC по сайтам",
                "HTTP/HTTPS по IP",
                "HTTP/HTTPS по сайтам",
            ]
        );
        assert!(response.profiles.iter().all(|profile| {
            profile.args.is_empty()
                && profile
                    .components
                    .first()
                    .is_some_and(|component| component.editable)
        }));
    }

    #[test]
    fn profile_summary_separates_match_protocols_from_packet_types_and_ordered_steps() {
        let response = from_config(
            "NFQWS_ARGS_CUSTOM=\"--filter-tcp=443 --filter-l7=tls --payload=tls_client_hello --out-range=<n2 --lua-desync=fake --in-range=2\"\n",
            "rev",
        )
        .unwrap();
        let profile = &response.profiles[0];

        assert_eq!(profile.protocols, ["--filter-l7=tls"]);
        assert_eq!(profile.filters, ["--filter-tcp=443"]);
        assert_eq!(profile.payloads, ["--payload=tls_client_hello"]);
        assert_eq!(
            profile.steps,
            [
                "--payload=tls_client_hello",
                "--out-range=<n2",
                "--lua-desync=fake",
                "--in-range=2"
            ]
        );
    }

    #[test]
    fn refuses_dynamic_shell_values_in_profile_sources() {
        let config = "NFQWS_ARGS_CUSTOM=\"$PORT --new --filter-tcp=443\"\n";
        assert!(from_config(config, "rev").is_err());
    }
}
