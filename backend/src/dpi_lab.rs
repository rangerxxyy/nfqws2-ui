use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    fmt,
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

pub const MAX_PCAP_BYTES: usize = 2 * 1024 * 1024;
pub const MAX_CAPTURE_PACKETS: usize = 5_000;
pub const MAX_FRAME_BYTES: usize = 65_535;
pub const MAX_TCP_FLOWS: usize = 64;
pub const MAX_REASSEMBLED_STREAM_BYTES: usize = 32 * 1024;
pub const MAX_SAVED_EXPERIMENTS: usize = 20;
pub const MAX_STRATEGY_SOURCE_BYTES: usize = 256 * 1024;
pub const MAX_STRATEGY_STEPS: usize = 128;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyStep {
    pub index: usize,
    pub profile_index: usize,
    pub raw: String,
    pub function: String,
    pub arguments: Vec<StrategyArgument>,
    pub installed: bool,
    pub file: Option<String>,
    pub docs: Vec<String>,
    pub declared_arguments: Vec<String>,
    pub required_arguments: Vec<String>,
    pub unverified_arguments: Vec<String>,
    pub evidence: &'static str,
    pub modeling: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyArgument {
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyProfile {
    pub index: usize,
    pub name: Option<String>,
    pub source: Option<String>,
    pub filters: Vec<String>,
    pub controls: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StrategyAnalysis {
    pub step_count: usize,
    pub steps: Vec<StrategyStep>,
    pub profiles: Vec<StrategyProfile>,
    pub filters: Vec<String>,
    pub other_options: Vec<String>,
    pub warnings: Vec<String>,
    pub lua_executed: bool,
}

fn documented_lua_argument_keys(arguments: &[String]) -> std::collections::HashSet<String> {
    let mut keys = std::collections::HashSet::new();
    for description in arguments {
        let declaration = description
            .split_once('—')
            .map_or(description.as_str(), |(declaration, _)| declaration);
        let declaration = declaration
            .split_once(" - ")
            .map_or(declaration, |(declaration, _)| declaration);
        for item in declaration.split(',') {
            let key = item
                .trim()
                .split(|character: char| {
                    character.is_whitespace()
                        || matches!(character, '=' | ':' | '<' | '>' | '|' | '(' | ')')
                })
                .next()
                .unwrap_or_default()
                .trim_matches(['*', '`']);
            if !key.is_empty()
                && key.len() <= 64
                && key.as_bytes()[0].is_ascii_lowercase()
                && key
                    .bytes()
                    .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
            {
                keys.insert(key.to_string());
            }
        }
    }
    keys
}

fn circular_orchestration_argument_keys(
    catalog: &[crate::runtime_profiles::LuaFunctionInfo],
) -> std::collections::HashSet<String> {
    let Some(circular) = catalog.iter().find(|item| item.name == "circular") else {
        return std::collections::HashSet::new();
    };
    let docs = circular.docs.join(" ").to_ascii_lowercase();
    let mut keys = std::collections::HashSet::new();
    if docs.contains("strategy=n") {
        keys.insert("strategy".to_string());
    }
    if docs.contains("final arg") || docs.contains("final' arg") || docs.contains("final\" arg") {
        keys.insert("final".to_string());
    }
    keys
}

/// Analyze strategy/config text as bounded data only. Never invoke a shell or Lua runtime.
pub fn analyze_strategy(
    source: &str,
    catalog: &[crate::runtime_profiles::LuaFunctionInfo],
) -> Result<StrategyAnalysis, DpiLabError> {
    analyze_strategy_with_standard_groups(source, catalog, &[])
}

pub fn analyze_strategy_with_standard_groups(
    source: &str,
    catalog: &[crate::runtime_profiles::LuaFunctionInfo],
    standard_groups: &[crate::runtime_profiles::LuaStandardGroup],
) -> Result<StrategyAnalysis, DpiLabError> {
    if source.len() > MAX_STRATEGY_SOURCE_BYTES {
        return Err(DpiLabError::Limit(format!(
            "Текст стратегии превышает лимит {} КиБ",
            MAX_STRATEGY_SOURCE_BYTES / 1024
        )));
    }
    if source.trim().is_empty() {
        return Err(DpiLabError::Malformed(
            "Введите стратегию или выберите профиль".into(),
        ));
    }
    if source.contains('\0') {
        return Err(DpiLabError::Malformed("Текст содержит нулевой байт".into()));
    }
    let mut raw_steps = Vec::new();
    let mut filters = Vec::new();
    let mut other_options = Vec::new();
    let mut warnings = Vec::new();
    let config_profiles = decode_config_strategy_profiles(source)?;
    let normalized_source = config_profiles.as_ref().map(|profiles| {
        profiles
            .iter()
            .map(|profile| profile.args.join("\n"))
            .collect::<Vec<_>>()
            .join("\n--new\n")
    });
    let scan_source = normalized_source.as_deref().unwrap_or(source);
    let mut profiles = vec![StrategyProfile {
        index: 1,
        name: None,
        source: None,
        filters: Vec::new(),
        controls: Vec::new(),
    }];
    for line in scan_source.lines() {
        let line = line.trim();
        if line.is_empty() || line.starts_with('#') || line.starts_with("-- ") {
            continue;
        }
        // This is a deliberately small data tokenizer, not a shell parser. Quotes
        // are stripped around individual tokens; expansions and commands are inert.
        for token in line.split_whitespace() {
            let token = token.trim_matches(['\'', '"', '`', ',']);
            let token = token
                .split_once('=')
                .filter(|(name, _)| name.starts_with("NFQWS_"))
                .map_or(token, |(_, value)| {
                    value.trim_matches(['\'', '"', '`', ','])
                });
            if token.is_empty()
                || token.starts_with("${")
                || token.starts_with("$")
                || token.contains("$(")
            {
                if token.starts_with('$') && warnings.len() < 32 {
                    warnings.push("Переменные и shell-выражения не вычисляются; значения нужно раскрыть вручную.".into());
                }
                continue;
            }
            if token == "--new" || token.starts_with("--new=") {
                if profiles.len() == 64 {
                    return Err(DpiLabError::Limit("В стратегии больше 64 профилей".into()));
                }
                profiles.push(StrategyProfile {
                    index: profiles.len() + 1,
                    name: token
                        .strip_prefix("--new=")
                        .filter(|name| !name.is_empty())
                        .map(str::to_owned),
                    source: None,
                    filters: Vec::new(),
                    controls: Vec::new(),
                });
            } else if let Some(name) = token.strip_prefix("--name=") {
                profiles.last_mut().expect("initial profile exists").name = Some(name.to_owned());
            } else if let Some(step) = token.strip_prefix("--lua-desync=") {
                if !step.is_empty() {
                    if raw_steps.len() == MAX_STRATEGY_STEPS {
                        return Err(DpiLabError::Limit(format!(
                            "В стратегии больше {MAX_STRATEGY_STEPS} Lua-шагов"
                        )));
                    }
                    raw_steps.push((profiles.len(), step.to_owned()));
                }
            } else if token == "--lua-desync" {
                if warnings.len() < 32 {
                    warnings.push("Для --lua-desync ожидается значение через знак '='.".into());
                }
            } else if token.starts_with("--filter-") {
                let profile = profiles.last_mut().expect("initial profile exists");
                if profile.filters.len() == 64 {
                    return Err(DpiLabError::Limit(
                        "В одном профиле больше 64 фильтров".into(),
                    ));
                }
                filters.push(token.to_owned());
                profile.filters.push(token.to_owned());
            } else if token.starts_with("--payload=")
                || token.starts_with("--in-range=")
                || token.starts_with("--out-range=")
            {
                let profile = profiles.last_mut().expect("initial profile exists");
                if profile.controls.len() == 64 {
                    return Err(DpiLabError::Limit(
                        "В одном профиле больше 64 управляющих параметров".into(),
                    ));
                }
                other_options.push(token.to_owned());
                profile.controls.push(token.to_owned());
            }
        }
    }
    let circular_profiles = raw_steps
        .iter()
        .filter_map(|(profile_index, raw)| {
            (raw.split(':').next() == Some("circular")).then_some(*profile_index)
        })
        .collect::<std::collections::HashSet<_>>();
    let circular_arguments = circular_orchestration_argument_keys(catalog);
    let steps = raw_steps
        .into_iter()
        .enumerate()
        .map(|(index, (profile_index, raw))| {
            let mut parts = raw.split(':');
            let function = parts.next().unwrap_or_default().to_owned();
            let arguments = parts
                .filter(|part| !part.is_empty())
                .map(|part| {
                    let (name, value) = part
                        .split_once('=')
                        .map_or((part, None), |(key, value)| (key, Some(value.to_owned())));
                    StrategyArgument {
                        name: name.to_owned(),
                        value,
                    }
                })
                .collect::<Vec<_>>();
            let info = catalog.iter().find(|item| item.name == function);
            let documented_keys = info
                .map(|item| {
                    let mut keys = documented_lua_argument_keys(&item.arguments);
                    keys.extend(item.file_arguments.iter().cloned());
                    for group_name in &item.standard_groups {
                        if let Some(group) = standard_groups
                            .iter()
                            .find(|group| group.name == *group_name)
                        {
                            keys.extend(documented_lua_argument_keys(&group.arguments));
                        }
                    }
                    if circular_profiles.contains(&profile_index) {
                        keys.extend(circular_arguments.iter().cloned());
                    }
                    keys
                })
                .unwrap_or_default();
            let unverified_arguments = arguments
                .iter()
                .filter(|argument| !documented_keys.contains(argument.name.as_str()))
                .map(|argument| argument.name.clone())
                .collect();
            StrategyStep {
                index: index + 1,
                profile_index,
                raw,
                function,
                arguments,
                installed: info.is_some(),
                file: info.map(|item| item.file.clone()),
                docs: info.map(|item| item.docs.clone()).unwrap_or_default(),
                declared_arguments: info.map(|item| item.arguments.clone()).unwrap_or_default(),
                required_arguments: info.map(|item| item.required.clone()).unwrap_or_default(),
                unverified_arguments,
                evidence: if info.is_some() {
                    "SOURCE CONFIRMED"
                } else {
                    "UNKNOWN"
                },
                modeling: "Не моделируется",
            }
        })
        .collect::<Vec<_>>();
    if steps.is_empty() && warnings.len() < 32 {
        warnings.push(
            "В тексте не найдены параметры --lua-desync=…; проверьте формат фрагмента.".into(),
        );
    }
    if let Some(config_profiles) = config_profiles {
        for (profile, config_profile) in profiles.iter_mut().zip(config_profiles) {
            profile.name = Some(config_profile.name);
            profile.source = Some(config_profile.source);
        }
        warnings.push("Профили выведены из конфигурации тем же безопасным парсером, что используется конструктором панели. Это не подтверждение выбора профиля работающим процессом.".into());
    }
    Ok(StrategyAnalysis {
        step_count: steps.len(),
        steps,
        profiles,
        filters,
        other_options,
        warnings,
        lua_executed: false,
    })
}

fn decode_config_strategy_profiles(
    source: &str,
) -> Result<Option<Vec<crate::runtime_profiles::RuntimeProfile>>, DpiLabError> {
    const GROUPS: &[&str] = &[
        "NFQWS_ARGS_CUSTOM",
        "NFQWS_ARGS_UDP",
        "NFQWS_ARGS_QUIC",
        "NFQWS_ARGS_IPSET",
        "NFQWS_ARGS",
        "NFQWS_EXTRA_ARGS",
    ];
    let is_config = source.lines().any(|line| {
        line.trim_start()
            .split_once('=')
            .is_some_and(|(name, _)| GROUPS.contains(&name.trim()))
    });
    if !is_config {
        return Ok(None);
    }
    let decoded =
        crate::runtime_profiles::from_config(source, "dpi-static-input").map_err(|error| {
            DpiLabError::Malformed(format!(
                "Не удалось безопасно разобрать конфигурацию: {error}"
            ))
        })?;
    Ok(Some(
        decoded
            .profiles
            .into_iter()
            .filter(|profile| !profile.args.is_empty())
            .collect(),
    ))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DpiLabError {
    Malformed(String),
    Limit(String),
}

impl fmt::Display for DpiLabError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Malformed(message) | Self::Limit(message) => formatter.write_str(message),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Analysis {
    pub capture: CaptureMetadata,
    pub packets: Vec<CapturedPacket>,
    pub flows: Vec<TcpFlow>,
    pub warnings: Vec<String>,
    pub evidence: Vec<&'static str>,
    pub summary: crate::dpi_summary::Summary,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CaptureMetadata {
    pub name: String,
    pub size_bytes: usize,
    pub packet_count: usize,
    pub link_type: u32,
    pub timestamp_resolution: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapturedPacket {
    pub number: usize,
    pub timestamp_micros: u64,
    pub source: String,
    pub source_port: Option<u16>,
    pub destination: String,
    pub destination_port: Option<u16>,
    pub protocol: String,
    pub tcp_sequence: Option<u32>,
    pub tcp_acknowledgement: Option<u32>,
    pub tcp_flags: Option<u16>,
    pub ttl: Option<u8>,
    pub payload_offset: Option<usize>,
    pub payload_len: usize,
    pub frame_hex: String,
    pub evidence: &'static str,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TcpFlow {
    pub source: String,
    pub source_port: u16,
    pub destination: String,
    pub destination_port: u16,
    pub packet_numbers: Vec<usize>,
    pub reassembly_policy: &'static str,
    pub sequence_start: u32,
    pub data_hex: String,
    pub data_ascii: String,
    pub byte_sources: Vec<Option<usize>>,
    pub gap_bytes: usize,
    pub overlap_bytes: usize,
    pub conflicting_overlap_bytes: usize,
    pub stream_truncated: bool,
    pub tls_client_hello: Option<TlsClientHello>,
    pub tls_parse_error: Option<String>,
    pub evidence: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TlsClientHello {
    pub server_name: Option<String>,
    pub fields: Vec<TlsField>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TlsField {
    pub name: String,
    pub offset: usize,
    pub length: usize,
    pub value: String,
    pub hex: String,
    pub evidence: &'static str,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SavedExperiment {
    pub id: String,
    pub name: String,
    pub saved_at: u64,
    pub size_bytes: usize,
    pub packet_count: usize,
    pub flow_count: usize,
}

fn experiment_directory(data_dir: &Path) -> PathBuf {
    data_dir.join("dpi-lab").join("experiments")
}

fn valid_experiment_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn experiment_metadata_path(directory: &Path, id: &str) -> PathBuf {
    directory.join(format!("{id}.json"))
}

fn experiment_capture_path(directory: &Path, id: &str) -> PathBuf {
    directory.join(format!("{id}.pcap"))
}

pub async fn save_experiment(
    data_dir: &Path,
    name: &str,
    bytes: &[u8],
) -> Result<SavedExperiment, String> {
    let analysis = analyze_pcap(name, bytes).map_err(|error| error.to_string())?;
    let directory = experiment_directory(data_dir);
    tokio::fs::create_dir_all(&directory)
        .await
        .map_err(|error| format!("Не удалось создать каталог DPI Lab: {error}"))?;
    let id = uuid::Uuid::new_v4().simple().to_string();
    let saved = SavedExperiment {
        id: id.clone(),
        name: analysis.capture.name,
        saved_at: SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs(),
        size_bytes: bytes.len(),
        packet_count: analysis.capture.packet_count,
        flow_count: analysis.flows.len(),
    };
    let pcap_temp = directory.join(format!("{id}.pcap.tmp"));
    let json_temp = directory.join(format!("{id}.json.tmp"));
    tokio::fs::write(&pcap_temp, bytes)
        .await
        .map_err(|error| format!("Не удалось сохранить PCAP: {error}"))?;
    let metadata = serde_json::to_vec(&saved)
        .map_err(|error| format!("Не удалось подготовить метаданные эксперимента: {error}"))?;
    if let Err(error) = tokio::fs::write(&json_temp, metadata).await {
        let _ = tokio::fs::remove_file(&pcap_temp).await;
        return Err(format!(
            "Не удалось сохранить метаданные эксперимента: {error}"
        ));
    }
    if let Err(error) =
        tokio::fs::rename(&pcap_temp, experiment_capture_path(&directory, &id)).await
    {
        let _ = tokio::fs::remove_file(&pcap_temp).await;
        let _ = tokio::fs::remove_file(&json_temp).await;
        return Err(format!("Не удалось завершить сохранение PCAP: {error}"));
    }
    if let Err(error) =
        tokio::fs::rename(&json_temp, experiment_metadata_path(&directory, &id)).await
    {
        let _ = tokio::fs::remove_file(&experiment_capture_path(&directory, &id)).await;
        let _ = tokio::fs::remove_file(&json_temp).await;
        return Err(format!(
            "Не удалось завершить сохранение метаданных: {error}"
        ));
    }
    prune_experiments(&directory).await;
    Ok(saved)
}

async fn prune_experiments(directory: &Path) {
    let Ok(mut experiments) = list_experiments_in(directory).await else {
        return;
    };
    if experiments.len() <= MAX_SAVED_EXPERIMENTS {
        return;
    }
    experiments.sort_by_key(|item| (item.saved_at, item.id.clone()));
    let excess = experiments.len().saturating_sub(MAX_SAVED_EXPERIMENTS);
    for item in experiments.into_iter().take(excess) {
        let _ = tokio::fs::remove_file(experiment_metadata_path(directory, &item.id)).await;
        let _ = tokio::fs::remove_file(experiment_capture_path(directory, &item.id)).await;
    }
}

async fn list_experiments_in(directory: &Path) -> Result<Vec<SavedExperiment>, String> {
    let mut entries = match tokio::fs::read_dir(directory).await {
        Ok(entries) => entries,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => return Err(format!("Не удалось прочитать эксперименты: {error}")),
    };
    let mut saved = Vec::new();
    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|error| error.to_string())?
    {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let Ok(bytes) = tokio::fs::read(&path).await else {
            continue;
        };
        if bytes.len() > 4096 {
            continue;
        }
        let Ok(item) = serde_json::from_slice::<SavedExperiment>(&bytes) else {
            continue;
        };
        if valid_experiment_id(&item.id) && path == experiment_metadata_path(directory, &item.id) {
            saved.push(item);
        }
    }
    saved.sort_by(|left, right| {
        right
            .saved_at
            .cmp(&left.saved_at)
            .then_with(|| right.id.cmp(&left.id))
    });
    Ok(saved)
}

pub async fn list_experiments(data_dir: &Path) -> Result<Vec<SavedExperiment>, String> {
    list_experiments_in(&experiment_directory(data_dir)).await
}

pub async fn load_experiment(data_dir: &Path, id: &str) -> Result<Analysis, String> {
    if !valid_experiment_id(id) {
        return Err("Некорректный идентификатор эксперимента".into());
    }
    let directory = experiment_directory(data_dir);
    let metadata_bytes = tokio::fs::read(experiment_metadata_path(&directory, id))
        .await
        .map_err(|_| "Эксперимент не найден".to_string())?;
    if metadata_bytes.len() > 4096 {
        return Err("Метаданные эксперимента превышают лимит".into());
    }
    let metadata: SavedExperiment = serde_json::from_slice(&metadata_bytes)
        .map_err(|_| "Метаданные эксперимента повреждены".to_string())?;
    if metadata.id != id {
        return Err("Идентификатор эксперимента не совпадает".into());
    }
    let capture_path = experiment_capture_path(&directory, id);
    let file_metadata = tokio::fs::metadata(&capture_path)
        .await
        .map_err(|_| "Файл PCAP не найден".to_string())?;
    if file_metadata.len() as usize > MAX_PCAP_BYTES
        || file_metadata.len() as usize != metadata.size_bytes
    {
        return Err("Размер сохранённого PCAP некорректен".into());
    }
    let bytes = tokio::fs::read(capture_path)
        .await
        .map_err(|error| error.to_string())?;
    analyze_pcap(&metadata.name, &bytes).map_err(|error| error.to_string())
}

#[derive(Clone, Copy)]
struct PcapFormat {
    big_endian: bool,
    nanos: bool,
}

#[derive(Clone)]
struct ParsedTcp {
    packet_index: usize,
    source: IpAddr,
    source_port: u16,
    destination: IpAddr,
    destination_port: u16,
    sequence: u32,
    flags: u16,
    payload: Vec<u8>,
}

type ParsedFrame = (IpAddr, u16, IpAddr, u16, u8, u32, u32, u16, usize, usize);

fn read_u16(bytes: &[u8], offset: usize, big_endian: bool) -> Option<u16> {
    let value: [u8; 2] = bytes.get(offset..offset + 2)?.try_into().ok()?;
    Some(if big_endian {
        u16::from_be_bytes(value)
    } else {
        u16::from_le_bytes(value)
    })
}

fn read_u32(bytes: &[u8], offset: usize, big_endian: bool) -> Option<u32> {
    let value: [u8; 4] = bytes.get(offset..offset + 4)?.try_into().ok()?;
    Some(if big_endian {
        u32::from_be_bytes(value)
    } else {
        u32::from_le_bytes(value)
    })
}

fn pcap_format(bytes: &[u8]) -> Result<PcapFormat, DpiLabError> {
    match bytes.get(..4) {
        Some([0xd4, 0xc3, 0xb2, 0xa1]) => Ok(PcapFormat {
            big_endian: false,
            nanos: false,
        }),
        Some([0xa1, 0xb2, 0xc3, 0xd4]) => Ok(PcapFormat {
            big_endian: true,
            nanos: false,
        }),
        Some([0x4d, 0x3c, 0xb2, 0xa1]) => Ok(PcapFormat {
            big_endian: false,
            nanos: true,
        }),
        Some([0xa1, 0xb2, 0x3c, 0x4d]) => Ok(PcapFormat {
            big_endian: true,
            nanos: true,
        }),
        Some([0x0a, 0x0d, 0x0d, 0x0a]) => Err(DpiLabError::Malformed(
            "PCAPNG пока не поддерживается; экспортируйте classic PCAP".into(),
        )),
        _ => Err(DpiLabError::Malformed(
            "Неизвестный или повреждённый заголовок classic PCAP".into(),
        )),
    }
}

pub fn analyze_pcap(name: &str, bytes: &[u8]) -> Result<Analysis, DpiLabError> {
    if bytes.len() > MAX_PCAP_BYTES {
        return Err(DpiLabError::Limit(format!(
            "Файл больше лимита {} КиБ",
            MAX_PCAP_BYTES / 1024
        )));
    }
    if bytes.len() < 24 {
        return Err(DpiLabError::Malformed("Заголовок PCAP обрезан".into()));
    }
    let format = pcap_format(bytes)?;
    let major = read_u16(bytes, 4, format.big_endian).unwrap_or_default();
    let minor = read_u16(bytes, 6, format.big_endian).unwrap_or_default();
    if major != 2 || minor != 4 {
        return Err(DpiLabError::Malformed(format!(
            "Неподдерживаемая версия PCAP {major}.{minor}"
        )));
    }
    let snaplen = read_u32(bytes, 16, format.big_endian).unwrap_or_default() as usize;
    if snaplen == 0 {
        return Err(DpiLabError::Limit("Недопустимый snaplen в PCAP".into()));
    }
    let link_type = read_u32(bytes, 20, format.big_endian).unwrap_or_default();
    if !matches!(link_type, 1 | 12 | 101 | 228 | 229 | 276) {
        return Err(DpiLabError::Malformed(format!(
            "Тип канального уровня {link_type} не поддерживается"
        )));
    }

    let mut offset = 24usize;
    let mut packets = Vec::new();
    let mut tcp_packets = Vec::new();
    let mut warnings = Vec::new();
    let mut record_count = 0usize;
    while offset < bytes.len() {
        if bytes.len() - offset < 16 {
            return Err(DpiLabError::Malformed(
                "Заголовок пакета PCAP обрезан".into(),
            ));
        }
        record_count += 1;
        if record_count > MAX_CAPTURE_PACKETS {
            return Err(DpiLabError::Limit(format!(
                "В захвате больше {MAX_CAPTURE_PACKETS} пакетов"
            )));
        }
        let seconds = read_u32(bytes, offset, format.big_endian).unwrap_or_default() as u64;
        let fraction = read_u32(bytes, offset + 4, format.big_endian).unwrap_or_default() as u64;
        let included = read_u32(bytes, offset + 8, format.big_endian).unwrap_or_default() as usize;
        let original = read_u32(bytes, offset + 12, format.big_endian).unwrap_or_default() as usize;
        if included > original || included > snaplen || included > MAX_FRAME_BYTES {
            return Err(DpiLabError::Malformed(
                "Длина кадра не соответствует заголовку PCAP".into(),
            ));
        }
        offset += 16;
        let end = offset
            .checked_add(included)
            .filter(|end| *end <= bytes.len())
            .ok_or_else(|| DpiLabError::Malformed("Данные кадра обрезаны".into()))?;
        if (format.nanos && fraction >= 1_000_000_000) || (!format.nanos && fraction >= 1_000_000) {
            return Err(DpiLabError::Malformed(
                "Некорректная дробная часть времени PCAP".into(),
            ));
        }
        let timestamp_micros = seconds
            .saturating_mul(1_000_000)
            .saturating_add(if format.nanos {
                fraction / 1_000
            } else {
                fraction
            });
        let frame = &bytes[offset..end];
        let packet_number = record_count;
        let mut packet = CapturedPacket {
            number: packet_number,
            timestamp_micros,
            source: "—".into(),
            source_port: None,
            destination: "—".into(),
            destination_port: None,
            protocol: "Другой".into(),
            tcp_sequence: None,
            tcp_acknowledgement: None,
            tcp_flags: None,
            ttl: None,
            payload_offset: None,
            payload_len: 0,
            frame_hex: hex::encode(frame),
            evidence: "CAPTURE OBSERVED",
        };
        match parse_frame(frame, link_type) {
            Ok(Some((
                src,
                src_port,
                dst,
                dst_port,
                ttl,
                seq,
                ack,
                flags,
                payload_offset,
                payload_end,
            ))) => {
                packet.source = src.to_string();
                packet.source_port = Some(src_port);
                packet.destination = dst.to_string();
                packet.destination_port = Some(dst_port);
                packet.protocol = "TCP".into();
                packet.tcp_sequence = Some(seq);
                packet.tcp_acknowledgement = Some(ack);
                packet.tcp_flags = Some(flags);
                packet.ttl = Some(ttl);
                packet.payload_offset = Some(payload_offset);
                packet.payload_len = payload_end.saturating_sub(payload_offset);
                tcp_packets.push(ParsedTcp {
                    packet_index: packets.len(),
                    source: src,
                    source_port: src_port,
                    destination: dst,
                    destination_port: dst_port,
                    sequence: seq.wrapping_add(u32::from(flags & 0x002 != 0)),
                    flags,
                    payload: frame
                        .get(payload_offset..payload_end)
                        .unwrap_or_default()
                        .to_vec(),
                });
            }
            Ok(None) => warnings.push(format!(
                "Пакет {packet_number}: формат или протокол не анализируется"
            )),
            Err(reason) => warnings.push(format!("Пакет {packet_number}: {reason}")),
        }
        packets.push(packet);
        offset = end;
    }
    // Reassembly is currently keyed by endpoints. Repeated connection starts
    // may be retransmissions or a new connection reusing the same tuple.
    let mut starts = std::collections::HashSet::new();
    let mut reused = false;
    for packet in &packets {
        if packet
            .tcp_flags
            .is_some_and(|flags| flags & 0x02 != 0 && flags & 0x10 == 0)
        {
            let key = (
                &packet.source,
                packet.source_port,
                &packet.destination,
                packet.destination_port,
                packet.tcp_sequence,
            );
            if !starts.insert(key) {
                reused = true;
            }
        }
    }
    if reused {
        warnings.push("Повторный SYN: возможно повторное использование адресов и портов; реконструкция может объединять разные соединения.".into());
    }
    let (flows, omitted_flows) = reassemble(tcp_packets);
    if omitted_flows > 0 {
        warnings.push(format!(
            "Пропущено TCP-потоков сверх лимита: {omitted_flows}"
        ));
    }
    let summary = crate::dpi_summary::summarize(&packets, &flows, &warnings);
    Ok(Analysis {
        capture: CaptureMetadata {
            name: safe_display_name(name),
            size_bytes: bytes.len(),
            packet_count: record_count,
            link_type,
            timestamp_resolution: if format.nanos {
                "наносекунды"
            } else {
                "микросекунды"
            }
            .into(),
        },
        packets,
        flows,
        warnings,
        evidence: vec!["CAPTURE OBSERVED", "PROTOCOL DERIVED", "UNKNOWN"],
        summary,
    })
}

fn safe_display_name(name: &str) -> String {
    let basename = name.rsplit(['/', '\\']).next().unwrap_or(name).trim();
    let safe = basename
        .chars()
        .filter(|ch| !ch.is_control())
        .take(120)
        .collect::<String>();
    if safe.is_empty() {
        "capture.pcap".into()
    } else {
        safe
    }
}

fn parse_frame(frame: &[u8], link_type: u32) -> Result<Option<ParsedFrame>, &'static str> {
    let (mut network_offset, ether_type) = match link_type {
        1 => {
            if frame.len() < 14 {
                return Err("Ethernet-заголовок обрезан");
            }
            let mut kind = u16::from_be_bytes([frame[12], frame[13]]);
            let mut pos = 14;
            let mut tags = 0;
            while matches!(kind, 0x8100 | 0x88a8 | 0x9100) && tags < 2 {
                if frame.len() < pos + 4 {
                    return Err("VLAN-заголовок обрезан");
                }
                kind = u16::from_be_bytes([frame[pos + 2], frame[pos + 3]]);
                pos += 4;
                tags += 1;
            }
            (pos, kind)
        }
        12 | 101 => (
            0,
            frame
                .first()
                .map(|value| if value >> 4 == 6 { 0x86dd } else { 0x0800 })
                .unwrap_or(0),
        ),
        228 => (0, 0x0800),
        229 => (0, 0x86dd),
        // LINKTYPE_LINUX_SLL2: fixed 20-byte cooked header. Its protocol
        // field is network byte order independently of the PCAP byte order.
        // Layout: libpcap pcap/sll.h, struct sll2_header.
        276 => {
            if frame.len() < 20 {
                return Err("Linux SLL2-заголовок обрезан");
            }
            (20, u16::from_be_bytes([frame[0], frame[1]]))
        }
        _ => return Err("Канальный уровень не поддерживается"),
    };
    let (source, destination, ttl, transport, network_end) = match ether_type {
        0x0800 => {
            if frame.len() < network_offset + 20 {
                return Err("IPv4-заголовок обрезан");
            }
            let version_header = frame[network_offset];
            if version_header >> 4 != 4 {
                return Ok(None);
            }
            let header_len = usize::from(version_header & 15) * 4;
            if header_len < 20 || frame.len() < network_offset + header_len {
                return Err("Некорректная длина IPv4-заголовка");
            }
            let total_len = usize::from(u16::from_be_bytes([
                frame[network_offset + 2],
                frame[network_offset + 3],
            ]));
            if total_len < header_len || network_offset + total_len > frame.len() {
                return Err("IPv4-пакет обрезан");
            }
            let fragment =
                u16::from_be_bytes([frame[network_offset + 6], frame[network_offset + 7]]);
            if fragment & 0x3fff != 0 {
                return Err("Фрагментированный IPv4 пакет пропущен");
            }
            let src = Ipv4Addr::new(
                frame[network_offset + 12],
                frame[network_offset + 13],
                frame[network_offset + 14],
                frame[network_offset + 15],
            );
            let dst = Ipv4Addr::new(
                frame[network_offset + 16],
                frame[network_offset + 17],
                frame[network_offset + 18],
                frame[network_offset + 19],
            );
            let transport = frame[network_offset + 9];
            let ttl = frame[network_offset + 8];
            let end = network_offset + total_len;
            network_offset += header_len;
            (IpAddr::V4(src), IpAddr::V4(dst), ttl, transport, end)
        }
        0x86dd => {
            if frame.len() < network_offset + 40 {
                return Err("IPv6-заголовок обрезан");
            }
            if frame[network_offset] >> 4 != 6 {
                return Ok(None);
            }
            let payload_len = usize::from(u16::from_be_bytes([
                frame[network_offset + 4],
                frame[network_offset + 5],
            ]));
            let end = network_offset + 40 + payload_len;
            if end > frame.len() {
                return Err("IPv6-пакет обрезан");
            }
            let src = Ipv6Addr::from(
                <[u8; 16]>::try_from(&frame[network_offset + 8..network_offset + 24])
                    .map_err(|_| "Некорректный IPv6 source")?,
            );
            let dst = Ipv6Addr::from(
                <[u8; 16]>::try_from(&frame[network_offset + 24..network_offset + 40])
                    .map_err(|_| "Некорректный IPv6 destination")?,
            );
            let mut transport = frame[network_offset + 6];
            let ttl = frame[network_offset + 7];
            network_offset += 40;
            let mut extension_count = 0usize;
            loop {
                let extension_len = match transport {
                    // Hop-by-Hop, Routing, Destination Options, Mobility, HIP, Shim6.
                    0 | 43 | 60 | 135 | 139 | 140 => {
                        if end < network_offset + 2 {
                            return Err("IPv6 extension заголовок обрезан");
                        }
                        (usize::from(frame[network_offset + 1]) + 1) * 8
                    }
                    // Authentication Header uses 32-bit words, including the first two.
                    51 => {
                        if end < network_offset + 2 {
                            return Err("IPv6 extension AH заголовок обрезан");
                        }
                        (usize::from(frame[network_offset + 1]) + 2) * 4
                    }
                    44 => 8,
                    // ESP is encrypted; No Next Header has no transport payload.
                    50 | 59 => return Ok(None),
                    _ => break,
                };
                extension_count += 1;
                if extension_count > 8 {
                    return Err("Слишком много IPv6 extension заголовков (лимит 8)");
                }
                let extension_end = network_offset
                    .checked_add(extension_len)
                    .filter(|extension_end| *extension_end <= end)
                    .ok_or("IPv6 extension заголовок выходит за границы пакета")?;
                let next = frame[network_offset];
                if transport == 44 {
                    let fragment =
                        u16::from_be_bytes([frame[network_offset + 2], frame[network_offset + 3]]);
                    if fragment & 0xfff8 != 0 || fragment & 1 != 0 {
                        return Err("Фрагментированный IPv6 пакет пропущен");
                    }
                }
                transport = next;
                network_offset = extension_end;
            }
            (IpAddr::V6(src), IpAddr::V6(dst), ttl, transport, end)
        }
        _ => return Ok(None),
    };
    if transport != 6 {
        return Ok(None);
    }
    if frame.len() < network_offset + 20 || network_end < network_offset + 20 {
        return Err("TCP-заголовок обрезан");
    }
    let src_port = u16::from_be_bytes([frame[network_offset], frame[network_offset + 1]]);
    let dst_port = u16::from_be_bytes([frame[network_offset + 2], frame[network_offset + 3]]);
    let seq = u32::from_be_bytes(
        frame[network_offset + 4..network_offset + 8]
            .try_into()
            .map_err(|_| "Некорректный TCP sequence")?,
    );
    let ack = u32::from_be_bytes(
        frame[network_offset + 8..network_offset + 12]
            .try_into()
            .map_err(|_| "Некорректный TCP acknowledgement")?,
    );
    let tcp_header_len = usize::from(frame[network_offset + 12] >> 4) * 4;
    if tcp_header_len < 20 || network_offset + tcp_header_len > network_end {
        return Err("Некорректная длина TCP-заголовка");
    }
    let flags =
        (u16::from(frame[network_offset + 12] & 1) << 8) | u16::from(frame[network_offset + 13]);
    Ok(Some((
        source,
        src_port,
        destination,
        dst_port,
        ttl,
        seq,
        ack,
        flags,
        network_offset + tcp_header_len,
        network_end,
    )))
}

fn reassemble(packets: Vec<ParsedTcp>) -> (Vec<TcpFlow>, usize) {
    type Key = (IpAddr, u16, IpAddr, u16, usize);
    type Connection = (IpAddr, u16, IpAddr, u16);
    // A repeated identical SYN in an open epoch may be a retransmission. A
    // new sequence, a previous close, or pre-existing midstream traffic starts
    // a new epoch. Capture order is not proof of endpoint acceptance.
    let mut epochs = BTreeMap::<Connection, (usize, Option<(IpAddr, u16, u32)>, bool)>::new();
    let mut grouped = BTreeMap::<Key, Vec<ParsedTcp>>::new();
    for packet in packets {
        let forward = (
            packet.source,
            packet.source_port,
            packet.destination,
            packet.destination_port,
        );
        let reverse = (
            packet.destination,
            packet.destination_port,
            packet.source,
            packet.source_port,
        );
        let connection = std::cmp::min(forward, reverse);
        let fresh_syn = packet.flags & 0x02 != 0 && packet.flags & 0x10 == 0;
        let initial = (
            packet.source,
            packet.source_port,
            packet.sequence.wrapping_sub(1),
        );
        let epoch = if let Some(state) = epochs.get_mut(&connection) {
            if fresh_syn && (state.2 || state.1 != Some(initial)) {
                state.0 += 1;
                state.1 = Some(initial);
                state.2 = false;
            }
            if packet.flags & 0x05 != 0 {
                state.2 = true;
            }
            state.0
        } else {
            epochs.insert(
                connection,
                (
                    0,
                    if fresh_syn { Some(initial) } else { None },
                    packet.flags & 0x05 != 0,
                ),
            );
            0
        };
        grouped
            .entry((
                packet.source,
                packet.source_port,
                packet.destination,
                packet.destination_port,
                epoch,
            ))
            .or_default()
            .push(packet);
    }
    let omitted_flows = grouped.len().saturating_sub(MAX_TCP_FLOWS);
    let flows = grouped
        .into_iter()
        .take(MAX_TCP_FLOWS)
        .map(
            |((source, source_port, destination, destination_port, _epoch), packets)| {
                let base = packets
                    .first()
                    .map(|packet| packet.sequence)
                    .unwrap_or_default();
                let segments = packets
                    .iter()
                    .filter(|packet| !packet.payload.is_empty())
                    .map(|packet| {
                        (
                            packet.sequence.wrapping_sub(base) as i32 as i64,
                            packet.packet_index + 1,
                            packet.payload.as_slice(),
                        )
                    })
                    .collect::<Vec<_>>();
                let min_position = segments
                    .iter()
                    .map(|(position, _, _)| *position)
                    .min()
                    .unwrap_or(0);
                let max_position = segments
                    .iter()
                    .map(|(position, _, data)| *position + data.len() as i64)
                    .max()
                    .unwrap_or(0);
                let stream_len = max_position
                    .saturating_sub(min_position)
                    .clamp(0, MAX_REASSEMBLED_STREAM_BYTES as i64)
                    as usize;
                let stream_truncated =
                    max_position.saturating_sub(min_position) > MAX_REASSEMBLED_STREAM_BYTES as i64;
                let mut bytes = vec![None; stream_len];
                let mut sources = vec![None; stream_len];
                let mut overlap_bytes = 0usize;
                let mut conflicting_overlap_bytes = 0usize;
                for (position, number, data) in segments {
                    for (index, byte) in data.iter().enumerate() {
                        let absolute = position + index as i64 - min_position;
                        if !(0..stream_len as i64).contains(&absolute) {
                            continue;
                        }
                        let slot = absolute as usize;
                        if bytes[slot].is_some() {
                            overlap_bytes += 1;
                            if bytes[slot] != Some(*byte) {
                                conflicting_overlap_bytes += 1;
                            }
                        } else {
                            bytes[slot] = Some(*byte);
                            sources[slot] = Some(number);
                        }
                    }
                }
                let gap_bytes = bytes.iter().filter(|byte| byte.is_none()).count();
                let data_hex = bytes
                    .iter()
                    .map(|byte| {
                        byte.map(|value| format!("{value:02x}"))
                            .unwrap_or_else(|| "??".into())
                    })
                    .collect::<Vec<_>>()
                    .join(" ");
                let data_ascii = bytes
                    .iter()
                    .map(|byte| {
                        byte.map(|value| {
                            if value.is_ascii_graphic() || value == b' ' {
                                value as char
                            } else {
                                '.'
                            }
                        })
                        .unwrap_or('·')
                    })
                    .collect::<String>();
                let contiguous = bytes
                    .iter()
                    .take_while(|byte| byte.is_some())
                    .filter_map(|byte| *byte)
                    .collect::<Vec<_>>();
                let (tls_client_hello, tls_parse_error) = if contiguous.starts_with(&[22]) {
                    match parse_client_hello(&contiguous) {
                        Ok(hello) => (Some(hello), None),
                        Err(error) => (None, Some(error)),
                    }
                } else {
                    (None, None)
                };
                TcpFlow {
                    source: source.to_string(),
                    source_port,
                    destination: destination.to_string(),
                    destination_port,
                    packet_numbers: packets
                        .iter()
                        .map(|packet| packet.packet_index + 1)
                        .collect(),
                    reassembly_policy: "first-seen",
                    sequence_start: base.wrapping_add(min_position as i32 as u32),
                    data_hex,
                    data_ascii,
                    byte_sources: sources,
                    gap_bytes,
                    overlap_bytes,
                    conflicting_overlap_bytes,
                    stream_truncated,
                    tls_client_hello,
                    tls_parse_error,
                    evidence: vec!["CAPTURE OBSERVED", "SIMULATED"],
                }
            },
        )
        .collect();
    (flows, omitted_flows)
}

fn parse_client_hello(bytes: &[u8]) -> Result<TlsClientHello, String> {
    fn take_u8(bytes: &[u8], pos: &mut usize, end: usize) -> Result<u8, String> {
        let value = *bytes
            .get(*pos)
            .filter(|_| *pos < end)
            .ok_or("Обрезано поле TLS")?;
        *pos += 1;
        Ok(value)
    }
    fn take_u16(bytes: &[u8], pos: &mut usize, end: usize) -> Result<usize, String> {
        if *pos + 2 > end {
            return Err("Обрезана длина поля TLS".into());
        }
        let value = usize::from(u16::from_be_bytes([bytes[*pos], bytes[*pos + 1]]));
        *pos += 2;
        Ok(value)
    }
    fn field(name: &str, bytes: &[u8], offset: usize, length: usize, value: String) -> TlsField {
        TlsField {
            name: name.into(),
            offset,
            length,
            value,
            hex: hex::encode(&bytes[offset..offset + length]),
            evidence: "PROTOCOL DERIVED",
        }
    }
    if bytes.len() < 5 || bytes[0] != 22 {
        return Err("Не найден TLS Handshake record".into());
    }
    let record_len = usize::from(u16::from_be_bytes([bytes[3], bytes[4]]));
    if record_len > bytes.len().saturating_sub(5) {
        return Err("TLS record обрезан".into());
    }
    let record_end = 5 + record_len;
    let mut pos = 5;
    let handshake_type_at = pos;
    if take_u8(bytes, &mut pos, record_end)? != 1 {
        return Err("Первое TLS-сообщение не ClientHello".into());
    }
    let handshake_length_at = pos;
    let handshake_len = (usize::from(take_u8(bytes, &mut pos, record_end)?) << 16)
        | (usize::from(take_u8(bytes, &mut pos, record_end)?) << 8)
        | usize::from(take_u8(bytes, &mut pos, record_end)?);
    let hello_end = pos
        .checked_add(handshake_len)
        .filter(|end| *end <= record_end)
        .ok_or("ClientHello обрезан или имеет неверную длину")?;
    let mut fields = vec![field(
        "TLS Record Header",
        bytes,
        0,
        5,
        format!("TLS record, {} bytes", record_len),
    )];
    fields.push(field(
        "Handshake Type",
        bytes,
        handshake_type_at,
        1,
        "ClientHello".into(),
    ));
    fields.push(field(
        "Handshake Length",
        bytes,
        handshake_length_at,
        3,
        format!("{handshake_len} bytes"),
    ));
    let version_at = pos;
    if pos + 2 > hello_end {
        return Err("ClientHello: legacy_version обрезан".into());
    }
    let version = u16::from_be_bytes([bytes[pos], bytes[pos + 1]]);
    pos += 2;
    fields.push(field(
        "Legacy Version",
        bytes,
        version_at,
        2,
        format!("0x{version:04x}"),
    ));
    let random_at = pos;
    if pos + 32 > hello_end {
        return Err("ClientHello: random обрезан".into());
    }
    pos += 32;
    fields.push(field("Random", bytes, random_at, 32, "32 байта".into()));
    let sid_len = usize::from(take_u8(bytes, &mut pos, hello_end)?);
    let sid_at = pos;
    if pos + sid_len > hello_end {
        return Err("ClientHello: session ID обрезан".into());
    }
    pos += sid_len;
    fields.push(field(
        "Session ID",
        bytes,
        sid_at,
        sid_len,
        format!("{sid_len} байт"),
    ));
    let cipher_len = take_u16(bytes, &mut pos, hello_end)?;
    if cipher_len % 2 != 0 || pos + cipher_len > hello_end {
        return Err("ClientHello: некорректный список Cipher Suites".into());
    }
    let cipher_at = pos;
    pos += cipher_len;
    fields.push(field(
        "Cipher Suites",
        bytes,
        cipher_at,
        cipher_len,
        format!("{} наборов", cipher_len / 2),
    ));
    let compression_len = usize::from(take_u8(bytes, &mut pos, hello_end)?);
    if pos + compression_len > hello_end {
        return Err("ClientHello: список Compression Methods обрезан".into());
    }
    let compression_at = pos;
    pos += compression_len;
    fields.push(field(
        "Compression Methods",
        bytes,
        compression_at,
        compression_len,
        format!("{compression_len} знач."),
    ));
    let mut server_name = None;
    if pos < hello_end {
        let extension_total = take_u16(bytes, &mut pos, hello_end)?;
        let extensions_end = pos
            .checked_add(extension_total)
            .filter(|end| *end == hello_end)
            .ok_or("ClientHello: неверная длина Extensions")?;
        while pos < extensions_end {
            let header_at = pos;
            let kind = take_u16(bytes, &mut pos, extensions_end)?;
            let length = take_u16(bytes, &mut pos, extensions_end)?;
            let data_at = pos;
            let end = pos
                .checked_add(length)
                .filter(|end| *end <= extensions_end)
                .ok_or("TLS extension выходит за границы ClientHello")?;
            let label = match kind {
                0 => "SNI",
                16 => "ALPN",
                43 => "Supported Versions",
                10 => "Supported Groups",
                51 => "Key Share",
                13 => "Signature Algorithms",
                21 => "Padding",
                _ => "TLS Extension",
            };
            fields.push(field(
                label,
                bytes,
                header_at,
                4 + length,
                format!("type {kind}, {length} bytes"),
            ));
            if kind == 0 {
                let mut sni_pos = data_at;
                let names_len = take_u16(bytes, &mut sni_pos, end)?;
                if sni_pos + names_len != end {
                    return Err("Некорректная длина списка SNI".into());
                }
                let names_end = end;
                while sni_pos < names_end {
                    let name_type = take_u8(bytes, &mut sni_pos, names_end)?;
                    let name_len = take_u16(bytes, &mut sni_pos, names_end)?;
                    let name_end = sni_pos
                        .checked_add(name_len)
                        .filter(|name_end| *name_end <= names_end)
                        .ok_or("Имя SNI обрезано")?;
                    if name_type == 0 {
                        let name = std::str::from_utf8(&bytes[sni_pos..name_end])
                            .map_err(|_| "SNI не является UTF-8")?;
                        if name.is_empty() || !name.is_ascii() {
                            return Err("SNI пустой или содержит не ASCII данные".into());
                        }
                        server_name = Some(name.to_owned());
                        fields.push(field(
                            "SNI hostname",
                            bytes,
                            sni_pos,
                            name_len,
                            name.to_owned(),
                        ));
                    }
                    sni_pos = name_end;
                }
            }
            pos = end;
        }
    }
    Ok(TlsClientHello {
        server_name,
        fields,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn strategy_analysis_keeps_order_maps_installed_functions_and_never_models_packets() {
        let catalog = [crate::runtime_profiles::LuaFunctionInfo {
            name: "multisplit".into(),
            file: "zapret-antidpi.lua".into(),
            docs: vec!["Разделение TCP payload".into()],
            arguments: vec!["pos — позиция разреза".into()],
            file_arguments: Vec::new(),
            standard_groups: vec!["general".into()],
            required: vec!["pos".into()],
        }];
        let result = analyze_strategy(
            "NFQWS_ARGS_CUSTOM='--filter-tcp=443 --filter-7=tls --lua-desync=multisplit:pos=host --lua-desync=unknown:ttl=3'",
            &catalog,
        ).unwrap();
        assert_eq!(result.step_count, 2);
        assert_eq!(result.steps[0].function, "multisplit");
        assert_eq!(result.steps[0].arguments[0].name, "pos");
        assert_eq!(result.steps[0].arguments[0].value.as_deref(), Some("host"));
        assert_eq!(result.steps[0].file.as_deref(), Some("zapret-antidpi.lua"));
        assert!(!result.steps[1].installed);
        assert_eq!(result.steps[1].evidence, "UNKNOWN");
        assert_eq!(result.steps[0].modeling, "Не моделируется");
        assert!(result.steps[0].unverified_arguments.is_empty());
        assert!(!result.lua_executed);
        assert_eq!(result.filters.len(), 2);
    }

    #[test]
    fn strategy_analysis_recognizes_each_comma_separated_documented_lua_argument() {
        let lua = "--[[\narg: nofake1, nofake2, nofake3 — fake controls\n]]\nfunction fake(ctx, desync)\n  return true\nend\n";
        let catalog = crate::runtime_profiles::parse_lua_catalog("zapret-antidpi.lua", lua);
        let result = analyze_strategy("--lua-desync=fake:nofake2=1:mystery=1", &catalog).unwrap();
        assert_eq!(result.steps[0].unverified_arguments, ["mystery"]);
    }

    #[test]
    fn strategy_analysis_accepts_documented_shared_and_file_level_arguments() {
        let lua = "-- arg: detect_host — hostname detector\n--[[\nstandard general:\n* repeats=2 - repeat count\n]]\n--[[\nstandard args: general. Common options.\n]]\nfunction fake(ctx, desync)\n  return true\nend\n";
        let functions = crate::runtime_profiles::parse_lua_catalog("zapret-auto.lua", lua);
        let groups = crate::runtime_profiles::parse_lua_standard_args(lua);
        let result = analyze_strategy_with_standard_groups(
            "--lua-desync=fake:repeats=2:detect_host=1:mystery=1",
            &functions,
            &groups,
        )
        .unwrap();
        assert_eq!(result.steps[0].unverified_arguments, ["mystery"]);
    }

    #[test]
    fn strategy_analysis_recognizes_circular_instance_tags_only_in_orchestrated_profiles() {
        let lua = "--[[\nEach orchestrated instance must have strategy=N arg. A final arg stops rotation.\n]]\nfunction circular(ctx, desync)\n  return true\nend\nfunction fake(ctx, desync)\n  return true\nend\n";
        let catalog = crate::runtime_profiles::parse_lua_catalog("zapret-auto.lua", lua);
        let tagged = analyze_strategy(
            "--lua-desync=fake:strategy=1:final=1 --lua-desync=circular",
            &catalog,
        )
        .unwrap();
        assert!(tagged.steps[0].unverified_arguments.is_empty());
        let unscoped = analyze_strategy("--lua-desync=fake:strategy=1:final=1", &catalog).unwrap();
        assert_eq!(
            unscoped.steps[0].unverified_arguments,
            ["strategy", "final"]
        );
    }

    #[test]
    fn strategy_analysis_rejects_empty_nul_and_oversized_input() {
        assert!(analyze_strategy("  ", &[]).is_err());
        assert!(analyze_strategy("--lua-desync=fake\0", &[]).is_err());
        assert!(matches!(
            analyze_strategy(&"x".repeat(MAX_STRATEGY_SOURCE_BYTES + 1), &[]),
            Err(DpiLabError::Limit(_))
        ));
    }

    #[test]
    fn strategy_analysis_preserves_profile_boundaries_filters_and_controls() {
        let analysis = analyze_strategy("--filter-tcp=443 --payload=tls_client_hello --lua-desync=fake --new=udp --filter-udp=443 --out-range=-n3 --lua-desync=quic", &[]).unwrap();
        assert_eq!(analysis.profiles.len(), 2);
        assert_eq!(analysis.steps[0].profile_index, 1);
        assert_eq!(analysis.steps[1].profile_index, 2);
        assert_eq!(analysis.profiles[0].filters, ["--filter-tcp=443"]);
        assert_eq!(analysis.profiles[1].filters, ["--filter-udp=443"]);
        assert_eq!(analysis.profiles[1].name.as_deref(), Some("udp"));
        assert_eq!(analysis.profiles[1].controls, ["--out-range=-n3"]);
    }

    #[test]
    fn strategy_config_uses_safe_assignment_scopes_and_ignores_commented_steps() {
        let source = "NFQWS_ARGS_CUSTOM='--filter-tcp=443 --lua-desync=first --new=second --filter-tcp=80 --lua-desync=second'\nNFQWS_ARGS_UDP='--filter-udp=443 --lua-desync=third'\n# --lua-desync=not_a_step\n";
        let analysis = analyze_strategy(source, &[]).unwrap();
        assert_eq!(analysis.steps.len(), 3);
        assert_eq!(analysis.profiles.len(), 3);
        assert_eq!(
            analysis
                .steps
                .iter()
                .map(|step| step.profile_index)
                .collect::<Vec<_>>(),
            [1, 2, 3]
        );
        assert_eq!(
            analysis.profiles[2].source.as_deref(),
            Some("NFQWS_ARGS_UDP")
        );
        assert_eq!(analysis.profiles[2].filters, ["--filter-udp=443"]);
        assert!(analyze_strategy("NFQWS_ARGS_CUSTOM=\"$(touch /tmp/never)\"", &[]).is_err());
    }

    fn pcap_with_frames(frames: &[Vec<u8>], big_endian: bool, nanos: bool) -> Vec<u8> {
        let mut bytes = Vec::new();
        let magic: [u8; 4] = match (big_endian, nanos) {
            (false, false) => [0xd4, 0xc3, 0xb2, 0xa1],
            (true, false) => [0xa1, 0xb2, 0xc3, 0xd4],
            (false, true) => [0x4d, 0x3c, 0xb2, 0xa1],
            (true, true) => [0xa1, 0xb2, 0x3c, 0x4d],
        };
        bytes.extend_from_slice(&magic);
        let mut global = Vec::new();
        push_u16(&mut global, 2, big_endian);
        push_u16(&mut global, 4, big_endian);
        push_u32(&mut global, 0, big_endian);
        push_u32(&mut global, 0, big_endian);
        push_u32(&mut global, 65_535, big_endian);
        push_u32(&mut global, 1, big_endian);
        bytes.extend(global);
        for (index, frame) in frames.iter().enumerate() {
            push_u32(&mut bytes, index as u32 + 1, big_endian);
            push_u32(
                &mut bytes,
                if nanos { 123_456_789 } else { 123_456 },
                big_endian,
            );
            push_u32(&mut bytes, frame.len() as u32, big_endian);
            push_u32(&mut bytes, frame.len() as u32, big_endian);
            bytes.extend_from_slice(frame);
        }
        bytes
    }

    fn push_u16(target: &mut Vec<u8>, value: u16, be: bool) {
        let bytes = if be {
            value.to_be_bytes()
        } else {
            value.to_le_bytes()
        };
        target.extend_from_slice(&bytes);
    }
    fn push_u32(target: &mut Vec<u8>, value: u32, be: bool) {
        let bytes = if be {
            value.to_be_bytes()
        } else {
            value.to_le_bytes()
        };
        target.extend_from_slice(&bytes);
    }

    fn tcp_frame(seq: u32, payload: &[u8]) -> Vec<u8> {
        let mut frame = vec![0u8; 14 + 20 + 20];
        frame[12..14].copy_from_slice(&0x0800u16.to_be_bytes());
        frame[14] = 0x45;
        frame[16..18].copy_from_slice(&((40 + payload.len()) as u16).to_be_bytes());
        frame[22] = 64;
        frame[23] = 6;
        frame[26..30].copy_from_slice(&[192, 0, 2, 10]);
        frame[30..34].copy_from_slice(&[198, 51, 100, 20]);
        frame[34..36].copy_from_slice(&12345u16.to_be_bytes());
        frame[36..38].copy_from_slice(&443u16.to_be_bytes());
        frame[38..42].copy_from_slice(&seq.to_be_bytes());
        frame[46] = 0x50;
        frame[47] = 0x18;
        frame.extend_from_slice(payload);
        frame
    }

    fn ipv6_tcp_frame(seq: u32, payload: &[u8], extensions: &[(u8, Vec<u8>)]) -> Vec<u8> {
        let extension_len: usize = extensions.iter().map(|(_, bytes)| bytes.len()).sum();
        let mut frame = vec![0u8; 14 + 40 + extension_len + 20];
        frame[12..14].copy_from_slice(&0x86ddu16.to_be_bytes());
        frame[14] = 0x60;
        frame[18..20].copy_from_slice(&((extension_len + 20 + payload.len()) as u16).to_be_bytes());
        frame[20] = extensions.first().map(|(kind, _)| *kind).unwrap_or(6);
        frame[21] = 64;
        frame[22..38].copy_from_slice(&Ipv6Addr::LOCALHOST.octets());
        frame[38..54].copy_from_slice(&"2001:db8::1".parse::<Ipv6Addr>().unwrap().octets());
        let mut offset = 54;
        for (_, bytes) in extensions {
            frame[offset..offset + bytes.len()].copy_from_slice(bytes);
            offset += bytes.len();
        }
        frame[offset..offset + 2].copy_from_slice(&12345u16.to_be_bytes());
        frame[offset + 2..offset + 4].copy_from_slice(&443u16.to_be_bytes());
        frame[offset + 4..offset + 8].copy_from_slice(&seq.to_be_bytes());
        frame[offset + 12] = 0x50;
        frame[offset + 13] = 0x18;
        frame.extend_from_slice(payload);
        frame
    }

    fn client_hello_sni(host: &str) -> Vec<u8> {
        let mut extensions = Vec::new();
        let mut sni = Vec::new();
        sni.extend_from_slice(&((host.len() + 3) as u16).to_be_bytes());
        sni.push(0);
        sni.extend_from_slice(&(host.len() as u16).to_be_bytes());
        sni.extend_from_slice(host.as_bytes());
        extensions.extend_from_slice(&0u16.to_be_bytes());
        extensions.extend_from_slice(&(sni.len() as u16).to_be_bytes());
        extensions.extend_from_slice(&sni);
        let mut body = vec![3, 3];
        body.extend_from_slice(&[7; 32]);
        body.push(0);
        body.extend_from_slice(&2u16.to_be_bytes());
        body.extend_from_slice(&[0x13, 1]);
        body.push(1);
        body.push(0);
        body.extend_from_slice(&(extensions.len() as u16).to_be_bytes());
        body.extend_from_slice(&extensions);
        let mut handshake = vec![1, 0, 0, 0];
        let body_len = body.len();
        handshake[1] = ((body_len >> 16) & 255) as u8;
        handshake[2] = ((body_len >> 8) & 255) as u8;
        handshake[3] = (body_len & 255) as u8;
        handshake.extend_from_slice(&body);
        let mut record = vec![22, 3, 1];
        record.extend_from_slice(&(handshake.len() as u16).to_be_bytes());
        record.extend_from_slice(&handshake);
        record
    }

    #[test]
    fn parses_classic_pcap_byte_order_and_timestamp_resolution() {
        for (be, nanos) in [(false, false), (true, true)] {
            let input = pcap_with_frames(&[tcp_frame(1, b"x")], be, nanos);
            let parsed = analyze_pcap("capture.pcap", &input).unwrap();
            assert_eq!(parsed.packets.len(), 1);
            assert_eq!(parsed.capture.packet_count, 1);
            assert_eq!(parsed.packets[0].timestamp_micros, 1_123_456);
        }
    }

    #[test]
    fn extracts_ethernet_ipv4_tcp_packet_and_keeps_capture_bytes() {
        let frame = tcp_frame(100, b"hello");
        let parsed = analyze_pcap(
            "capture.pcap",
            &pcap_with_frames(std::slice::from_ref(&frame), false, false),
        )
        .unwrap();
        assert_eq!(parsed.packets.len(), 1);
        assert_eq!(parsed.packets[0].source, "192.0.2.10");
        assert_eq!(parsed.packets[0].destination_port, Some(443));
        assert_eq!(parsed.packets[0].tcp_sequence, Some(100));
        assert_eq!(parsed.packets[0].payload_len, 5);
        assert_eq!(parsed.packets[0].frame_hex, hex::encode(frame));
        assert_eq!(parsed.packets[0].evidence, "CAPTURE OBSERVED");
    }

    #[test]
    fn linux_sll2_preserves_ipv4_ipv6_tcp_tls_and_network_order_in_both_pcap_orders() {
        for big_endian in [false, true] {
            let hello = client_hello_sni("example.com");
            for ethernet in [tcp_frame(100, &hello), ipv6_tcp_frame(100, &hello, &[])] {
                let mut frame = vec![0u8; 20];
                frame[0..2].copy_from_slice(&ethernet[12..14]);
                frame[4..8].copy_from_slice(&7u32.to_be_bytes());
                frame[8..10].copy_from_slice(&1u16.to_be_bytes());
                frame[10] = 4;
                frame[11] = 6;
                frame.extend_from_slice(&ethernet[14..]);
                let mut pcap = pcap_with_frames(std::slice::from_ref(&frame), big_endian, false);
                pcap[20..24].copy_from_slice(&if big_endian {
                    276u32.to_be_bytes()
                } else {
                    276u32.to_le_bytes()
                });
                let analysis = analyze_pcap("sll2.pcap", &pcap).unwrap();
                assert_eq!(analysis.capture.link_type, 276);
                assert_eq!(analysis.packets[0].tcp_sequence, Some(100));
                assert_eq!(analysis.packets[0].payload_len, hello.len());
                assert_eq!(analysis.packets[0].frame_hex, hex::encode(&frame));
                assert_eq!(
                    analysis.flows[0]
                        .tls_client_hello
                        .as_ref()
                        .unwrap()
                        .server_name
                        .as_deref(),
                    Some("example.com")
                );
            }
        }
    }

    #[test]
    fn linux_sll2_checks_header_bounds_and_skips_non_ip_protocols() {
        for length in 0..20 {
            assert_eq!(
                parse_frame(&vec![0; length], 276),
                Err("Linux SLL2-заголовок обрезан")
            );
        }
        let mut frame = vec![0u8; 20];
        frame[0..2].copy_from_slice(&0x0806u16.to_be_bytes());
        assert_eq!(parse_frame(&frame, 276), Ok(None));
        frame[0..2].copy_from_slice(&0x0800u16.to_be_bytes());
        assert!(parse_frame(&frame, 276).is_err());
    }

    #[test]
    fn extracts_ipv6_tcp_after_hop_by_hop_and_destination_options() {
        let extensions = [
            (0, vec![60, 0, 0, 0, 0, 0, 0, 0]),
            (60, vec![6, 0, 0, 0, 0, 0, 0, 0]),
        ];
        let frame = ipv6_tcp_frame(77, b"v6", &extensions);
        let parsed = analyze_pcap("ipv6.pcap", &pcap_with_frames(&[frame], false, false)).unwrap();

        assert_eq!(parsed.packets.len(), 1);
        assert_eq!(parsed.packets[0].source, "::1");
        assert_eq!(parsed.packets[0].destination, "2001:db8::1");
        assert_eq!(parsed.packets[0].tcp_sequence, Some(77));
        assert_eq!(parsed.packets[0].payload_len, 2);
    }

    #[test]
    fn skips_non_atomic_ipv6_fragments_and_rejects_truncated_extensions() {
        let fragment = [(44, vec![6, 0, 0, 1, 0, 0, 0, 0])];
        let parsed = analyze_pcap(
            "fragment.pcap",
            &pcap_with_frames(&[ipv6_tcp_frame(1, b"x", &fragment)], false, false),
        )
        .unwrap();
        assert_eq!(parsed.packets[0].protocol, "Другой");

        let malformed = [(0, vec![6, 255, 0, 0, 0, 0, 0, 0])];
        let parsed = analyze_pcap(
            "truncated-extension.pcap",
            &pcap_with_frames(&[ipv6_tcp_frame(1, b"x", &malformed)], false, false),
        )
        .unwrap();
        assert_eq!(parsed.packets[0].protocol, "Другой");
        assert!(parsed.warnings[0].contains("IPv6 extension"));
    }

    #[test]
    fn parses_atomic_ipv6_fragment_and_authentication_header_with_a_bounded_chain() {
        let atomic_fragment = [(44, vec![6, 0, 0, 0, 0, 0, 0, 0])];
        let ah = [(51, vec![6, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0])];
        for extensions in [&atomic_fragment[..], &ah[..]] {
            let parsed = analyze_pcap(
                "ipv6-extension.pcap",
                &pcap_with_frames(&[ipv6_tcp_frame(9, b"ok", extensions)], false, false),
            )
            .unwrap();
            assert_eq!(parsed.packets[0].protocol, "TCP");
            assert_eq!(parsed.packets[0].tcp_sequence, Some(9));
        }

        let mut excessive = vec![(60, vec![60, 0, 0, 0, 0, 0, 0, 0]); 9];
        excessive[8].1[0] = 6;
        let parsed = analyze_pcap(
            "ipv6-extension-limit.pcap",
            &pcap_with_frames(&[ipv6_tcp_frame(1, b"x", &excessive)], false, false),
        )
        .unwrap();
        assert_eq!(parsed.packets[0].protocol, "Другой");
        assert!(parsed.warnings[0].contains("лимит 8"));
    }

    #[test]
    fn arbitrary_bounded_frames_never_panic_in_link_and_ip_parsing() {
        let link_types = [1, 12, 101, 228, 229, 276];
        let mut state = 0x9e37_79b9u32;
        for case in 0..4096usize {
            let length = case % 320;
            let mut frame = vec![0u8; length];
            for byte in &mut frame {
                state ^= state << 13;
                state ^= state >> 17;
                state ^= state << 5;
                *byte = state as u8;
            }
            if frame.len() >= 14 && case % 4 == 0 {
                frame[12..14].copy_from_slice(&0x86ddu16.to_be_bytes());
                if frame.len() >= 54 {
                    frame[14] = 0x60;
                    let payload_len = (frame.len() - 54).min(u16::MAX as usize) as u16;
                    frame[18..20].copy_from_slice(&payload_len.to_be_bytes());
                    frame[20] = [0, 43, 44, 51, 60, 6][case % 6];
                }
            }
            let _ = parse_frame(&frame, link_types[case % link_types.len()]);
        }
    }

    #[test]
    fn rejects_malformed_capture_lengths_and_oversized_input() {
        let mut truncated = pcap_with_frames(&[tcp_frame(1, b"x")], false, false);
        truncated.pop();
        assert!(matches!(
            analyze_pcap("bad.pcap", &truncated),
            Err(DpiLabError::Malformed(_))
        ));
        assert!(matches!(
            analyze_pcap("large.pcap", &vec![0; MAX_PCAP_BYTES + 1]),
            Err(DpiLabError::Limit(_))
        ));
    }

    #[test]
    fn rejects_packet_count_above_limit_without_processing_the_rest() {
        let frames = (0..=MAX_CAPTURE_PACKETS)
            .map(|_| tcp_frame(100, b""))
            .collect::<Vec<_>>();
        assert!(matches!(
            analyze_pcap("many.pcap", &pcap_with_frames(&frames, false, false)),
            Err(DpiLabError::Limit(_))
        ));
    }

    #[test]
    fn reports_flows_above_limit_and_does_not_reconstruct_them() {
        let frames = (1..=MAX_TCP_FLOWS + 1)
            .map(|source| {
                let mut frame = tcp_frame(100, b"payload");
                frame[29] = source as u8;
                frame
            })
            .collect::<Vec<_>>();
        let parsed =
            analyze_pcap("many-flows.pcap", &pcap_with_frames(&frames, false, false)).unwrap();
        assert_eq!(parsed.flows.len(), MAX_TCP_FLOWS);
        assert!(
            parsed
                .warnings
                .iter()
                .any(|warning| warning.contains("сверх лимита"))
        );
    }

    #[test]
    fn reassembles_gaps_and_overlaps_with_first_seen_byte_provenance() {
        let frames = [
            tcp_frame(100, b"abcd"),
            tcp_frame(112, b"ij"),
            tcp_frame(102, b"XXefgh"),
        ];
        let parsed =
            analyze_pcap("overlap.pcap", &pcap_with_frames(&frames, false, false)).unwrap();
        let stream = &parsed.flows[0];
        assert_eq!(stream.reassembly_policy, "first-seen");
        assert_eq!(stream.data_ascii, "abcdefgh····ij");
        assert_eq!(stream.gap_bytes, 4);
        assert_eq!(stream.overlap_bytes, 2);
        assert_eq!(stream.conflicting_overlap_bytes, 2);
        assert_eq!(stream.byte_sources[2], Some(1));
        assert_eq!(stream.byte_sources[4], Some(3));
        assert!(stream.evidence.contains(&"SIMULATED"));
    }

    #[test]
    fn distinguishes_identical_retransmissions_from_conflicting_overlap() {
        let frames = [
            tcp_frame(100, b"abcd"),
            tcp_frame(100, b"abcd"),
            tcp_frame(102, b"cZ"),
        ];
        let result =
            analyze_pcap("overlap.pcap", &pcap_with_frames(&frames, false, false)).unwrap();
        let flow = &result.flows[0];
        assert_eq!(flow.data_ascii, "abcd");
        assert_eq!(flow.overlap_bytes, 6);
        assert_eq!(flow.conflicting_overlap_bytes, 1);
        let summary = result
            .summary
            .findings
            .iter()
            .find(|item| item.id == "reassembly")
            .unwrap();
        assert_eq!(summary.state, "not_confirmed");
        assert!(summary.detail.contains("различающимися байтами: 1"));
    }

    #[test]
    fn identical_retransmission_does_not_make_byte_reconstruction_ambiguous() {
        let frames = [tcp_frame(100, b"abcd"), tcp_frame(100, b"abcd")];
        let result =
            analyze_pcap("duplicate.pcap", &pcap_with_frames(&frames, false, false)).unwrap();
        assert_eq!(result.flows[0].conflicting_overlap_bytes, 0);
        assert_eq!(
            result
                .summary
                .findings
                .iter()
                .find(|item| item.id == "reassembly")
                .unwrap()
                .state,
            "confirmed"
        );
    }

    #[test]
    fn separates_reused_tcp_endpoints_without_splitting_retransmitted_syn() {
        let control = |sequence, flags| {
            let mut frame = tcp_frame(sequence, b"");
            frame[47] = flags;
            frame
        };
        let frames = [
            control(100, 2),
            control(100, 2),
            tcp_frame(101, b"first"),
            control(106, 17),
            control(900, 2),
            tcp_frame(901, b"second"),
        ];
        let result = analyze_pcap("reuse.pcap", &pcap_with_frames(&frames, false, false)).unwrap();
        assert_eq!(result.flows.len(), 2);
        assert_eq!(result.flows[0].data_ascii, "first");
        assert_eq!(result.flows[1].data_ascii, "second");
        assert_eq!(result.flows[0].packet_numbers, vec![1, 2, 3, 4]);
        assert_eq!(result.flows[1].packet_numbers, vec![5, 6]);
    }

    #[test]
    fn separates_partial_capture_before_a_new_syn() {
        let mut syn = tcp_frame(900, b"");
        syn[47] = 2;
        let frames = [tcp_frame(100, b"partial"), syn, tcp_frame(901, b"new")];
        let result =
            analyze_pcap("partial.pcap", &pcap_with_frames(&frames, false, false)).unwrap();
        assert_eq!(result.flows.len(), 2);
        assert_eq!(result.flows[0].data_ascii, "partial");
        assert_eq!(result.flows[1].data_ascii, "new");
    }

    #[test]
    fn new_epoch_applies_to_both_directions_of_the_connection() {
        let control = |sequence, flags| {
            let mut frame = tcp_frame(sequence, b"");
            frame[47] = flags;
            frame
        };
        let reverse = |sequence, payload: &[u8], flags| {
            let mut frame = tcp_frame(sequence, payload);
            frame[26..30].copy_from_slice(&[198, 51, 100, 20]);
            frame[30..34].copy_from_slice(&[192, 0, 2, 10]);
            frame[34..36].copy_from_slice(&443u16.to_be_bytes());
            frame[36..38].copy_from_slice(&12345u16.to_be_bytes());
            frame[47] = flags;
            frame
        };
        let frames = [
            control(100, 2),
            reverse(500, b"", 18),
            tcp_frame(101, b"one"),
            reverse(501, b"reply1", 24),
            reverse(507, b"", 17),
            control(900, 2),
            reverse(800, b"", 18),
            tcp_frame(901, b"two"),
            reverse(801, b"reply2", 24),
        ];
        let result = analyze_pcap("both.pcap", &pcap_with_frames(&frames, false, false)).unwrap();
        assert_eq!(result.flows.len(), 4);
        assert!(result.flows.iter().any(|flow| flow.data_ascii == "one"));
        assert!(result.flows.iter().any(|flow| flow.data_ascii == "two"));
        assert!(result.flows.iter().any(|flow| flow.data_ascii == "reply1"));
        assert!(result.flows.iter().any(|flow| flow.data_ascii == "reply2"));
    }

    #[test]
    fn fin_does_not_discard_remaining_data_from_opposite_direction() {
        let mut syn = tcp_frame(100, b"");
        syn[47] = 2;
        let mut fin = tcp_frame(101, b"");
        fin[47] = 17;
        let mut response = tcp_frame(500, b"last response");
        response[26..30].copy_from_slice(&[198, 51, 100, 20]);
        response[30..34].copy_from_slice(&[192, 0, 2, 10]);
        response[34..36].copy_from_slice(&443u16.to_be_bytes());
        response[36..38].copy_from_slice(&12345u16.to_be_bytes());
        let frames = [syn, fin, response];
        let result =
            analyze_pcap("half-close.pcap", &pcap_with_frames(&frames, false, false)).unwrap();
        assert_eq!(result.flows.len(), 2);
        assert!(
            result
                .flows
                .iter()
                .any(|flow| flow.data_ascii == "last response" && flow.packet_numbers == vec![3])
        );
    }

    #[test]
    fn first_seen_policy_follows_capture_arrival_order_for_overlaps() {
        let frames = [tcp_frame(102, b"XXefgh"), tcp_frame(100, b"abcd")];
        let parsed = analyze_pcap(
            "out-of-order.pcap",
            &pcap_with_frames(&frames, false, false),
        )
        .unwrap();
        let flow = &parsed.flows[0];
        assert_eq!(flow.data_ascii, "abXXefgh");
        assert_eq!(flow.overlap_bytes, 2);
        assert_eq!(flow.byte_sources[2], Some(1));
    }

    #[test]
    fn reassembles_tcp_sequence_wrap_and_reports_stream_start() {
        let frames = [tcp_frame(u32::MAX - 1, b"abcd"), tcp_frame(2, b"ef")];
        let parsed = analyze_pcap(
            "sequence-wrap.pcap",
            &pcap_with_frames(&frames, false, false),
        )
        .unwrap();
        let flow = &parsed.flows[0];
        assert_eq!(flow.sequence_start, u32::MAX - 1);
        assert_eq!(flow.data_ascii, "abcdef");
        assert_eq!(flow.gap_bytes, 0);
    }

    #[test]
    fn caps_reconstructed_stream_expansion_from_large_sequence_gaps() {
        let distant = 1 + MAX_REASSEMBLED_STREAM_BYTES as u32 + 100;
        let frames = [tcp_frame(1, b"a"), tcp_frame(distant, b"z")];
        let parsed =
            analyze_pcap("large-gap.pcap", &pcap_with_frames(&frames, false, false)).unwrap();
        let flow = &parsed.flows[0];
        assert_eq!(flow.byte_sources.len(), MAX_REASSEMBLED_STREAM_BYTES);
        assert!(flow.stream_truncated);
    }

    #[test]
    fn parses_client_hello_sni_and_rejects_truncated_extension_lengths() {
        let hello = client_hello_sni("example.com");
        let parsed = parse_client_hello(&hello).unwrap();
        assert_eq!(parsed.server_name.as_deref(), Some("example.com"));
        assert!(
            parsed
                .fields
                .iter()
                .any(|field| field.name == "Handshake Type"
                    && field.offset == 5
                    && field.length == 1)
        );
        assert!(
            parsed
                .fields
                .iter()
                .any(|field| field.name == "Handshake Length"
                    && field.offset == 6
                    && field.length == 3)
        );
        assert!(
            parsed
                .fields
                .iter()
                .any(|field| field.name == "SNI hostname" && field.length == 11)
        );
        let mut invalid = hello;
        invalid.pop();
        assert!(parse_client_hello(&invalid).is_err());

        let mut invalid_name_length = client_hello_sni("example.com");
        invalid_name_length[59..61].copy_from_slice(&u16::MAX.to_be_bytes());
        assert!(parse_client_hello(&invalid_name_length).is_err());
    }

    #[tokio::test]
    async fn saved_experiments_use_generated_ids_and_prune_old_entries() {
        let dir = tempfile::tempdir().unwrap();
        let capture = pcap_with_frames(&[], false, false);
        let saved = save_experiment(dir.path(), "../../auth.json", &capture)
            .await
            .unwrap();
        assert_eq!(saved.name, "auth.json");
        assert!(valid_experiment_id(&saved.id));
        assert_eq!(list_experiments(dir.path()).await.unwrap().len(), 1);
        let loaded = load_experiment(dir.path(), &saved.id).await.unwrap();
        assert_eq!(loaded.capture.packet_count, 0);
        assert!(
            load_experiment(dir.path(), "../../auth.json")
                .await
                .is_err()
        );

        for index in 0..MAX_SAVED_EXPERIMENTS {
            save_experiment(dir.path(), &format!("capture-{index}.pcap"), &capture)
                .await
                .unwrap();
        }
        assert_eq!(
            list_experiments(dir.path()).await.unwrap().len(),
            MAX_SAVED_EXPERIMENTS
        );
    }
}
