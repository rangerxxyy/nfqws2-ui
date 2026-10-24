use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, Query, State},
    http::{HeaderName, HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{Html, IntoResponse, Response},
    routing::{get, post},
};
use nfqws2_ui::config_document::{ConfigDocument, DiagnosticSeverity, validate_nfqws2_config};
use nfqws2_ui::freeze_diagnostic;
use nfqws2_ui::site_diagnostics::{self, DiagnosticStep};
use nfqws2_ui::site_trace;
use nfqws2_ui::strategy_picker::{self, StrategyMode, StrategyProtocol};
use nfqws2_ui::transfer_diagnostic;
mod snapshots;
use nfqws2_ui::{
    ServiceAction, ServiceState, discover_system_paths, known_config_paths, known_init_scripts,
    network_interface_names, parse_meminfo_kb, process_matches, service_action, service_action_at,
    service_status,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;
use std::{
    collections::{HashMap, HashSet},
    future::Future,
    io::Read as StdRead,
    net::{IpAddr, SocketAddr},
    path::{Path, PathBuf},
    pin::Pin,
    process::Stdio,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::{net::TcpListener, sync::Mutex};
use tower_http::{services::ServeDir, trace::TraceLayer};

const MAX_CONFIG_BYTES: usize = 512 * 1024;
const MAX_LIST_BYTES: usize = 512 * 1024;
const MAX_LOG_BYTES: usize = 256 * 1024;
const MAX_RELEASE_RESPONSE_BYTES: usize = 1024 * 1024;
const WRITE_REVISION_CONFLICT: &str = "REVISION_CONFLICT";
const GITHUB_RELEASES_API: &str =
    "https://api.github.com/repos/rangerxxyy/nfqws2-ui/releases?per_page=100";
const NFQWS2_RELEASES_API: &str =
    "https://api.github.com/repos/nfqws/nfqws2-keenetic/releases?per_page=100";
static UPDATE_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
const PANEL_SHELL_HTML: &str = r##"<!doctype html>
<html lang="ru" class="dark">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>nfqws2-UI</title>
  <link rel="icon" type="image/svg+xml" href="/favicon.svg">
  <link rel="stylesheet" href="/nfqws2-ui.css">
</head>
<body>
  <div id="root"></div>
  <script type="module" src="/nfqws2-ui.js"></script>
</body>
</html>"##;
const LIST_IDS: [&str; 5] = [
    "user.list",
    "auto.list",
    "exclude.list",
    "ipset.list",
    "ipset_exclude.list",
];

struct AppState {
    data_dir: PathBuf,
    port: u16,
    cpu_previous: Arc<Mutex<Option<(u64, u64)>>>,
    config_write_lock: Mutex<()>,
    config_override: Option<PathBuf>,
    update_lock: Mutex<()>,
    ui_update_status: Arc<std::sync::RwLock<UiUpdateStatus>>,
    site_diagnostic_lock: Arc<Mutex<()>>,
    monitoring_lock: Arc<Mutex<()>>,
    site_trace_controls: Mutex<HashMap<String, Arc<AtomicBool>>>,
    site_trace_runner: Arc<SiteTraceRunner>,
    strategy_pick_controls: Mutex<HashMap<String, Arc<AtomicBool>>>,
    transfer_diagnostic_controls: Mutex<HashMap<String, Arc<AtomicBool>>>,
    freeze_scan_controls: Mutex<HashMap<String, Arc<freeze_diagnostic::FreezeScanPause>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MonitoringStore {
    enabled: bool,
    interval_minutes: u64,
    last_auto_check: Option<u64>,
    sites: Vec<MonitoredSite>,
    telegram: TelegramSettings,
    #[serde(default)]
    telegram_notifications: TelegramNotificationStore,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TelegramNotificationStore {
    #[serde(default)]
    queue: Vec<TelegramQueuedMessage>,
    #[serde(default)]
    log: Vec<TelegramDeliveryLog>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TelegramQueuedMessage {
    timestamp: u64,
    text: String,
    #[serde(default)]
    tries: u32,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TelegramDeliveryLog {
    timestamp: u64,
    event_timestamp: u64,
    text: String,
    ok: bool,
    #[serde(default)]
    error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MonitoredSite {
    host: String,
    #[serde(default)]
    results: Vec<MonitoredResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MonitoredResult {
    checked_at: u64,
    status: String,
    summary: String,
    latency_ms: Option<u64>,
    steps: Vec<MonitoringStepRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MonitoringStepRecord {
    id: String,
    status: String,
    detail: String,
    duration_ms: Option<u64>,
    http_code: Option<u16>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TelegramSettings {
    #[serde(default)]
    token: String,
    #[serde(default)]
    chat_id: String,
    #[serde(default)]
    send_via: String,
    #[serde(default)]
    interface: String,
    #[serde(default)]
    proxy: String,
}

impl Default for TelegramSettings {
    fn default() -> Self {
        Self {
            token: String::new(),
            chat_id: String::new(),
            send_via: "direct".into(),
            interface: String::new(),
            proxy: String::new(),
        }
    }
}

impl Default for MonitoringStore {
    fn default() -> Self {
        Self {
            enabled: true,
            interval_minutes: 30,
            last_auto_check: None,
            sites: Vec::new(),
            telegram: TelegramSettings::default(),
            telegram_notifications: TelegramNotificationStore::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MonitoringStoreUpdate {
    enabled: bool,
    interval_minutes: u64,
    sites: Vec<MonitoredSite>,
}

#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct MonitoringRunRequest {
    hosts: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TelegramSettingsUpdate {
    token: Option<String>,
    chat_id: String,
    send_via: String,
    interface: String,
    proxy: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UiUpdateStatus {
    current_version: String,
    latest_version: Option<String>,
    update_available: bool,
    release_url: Option<String>,
    checked_at: Option<u64>,
}

impl Default for UiUpdateStatus {
    fn default() -> Self {
        Self {
            current_version: env!("CARGO_PKG_VERSION").into(),
            latest_version: None,
            update_available: false,
            release_url: None,
            checked_at: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SiteTraceOperation {
    id: String,
    state: String,
    stage: String,
    message: String,
    report: Option<site_trace::TraceReport>,
    updated_at: u64,
}

fn update_site_trace_progress(
    operation: &mut SiteTraceOperation,
    stage: &str,
    report: site_trace::TraceReport,
) {
    operation.stage = stage.chars().take(32).collect();
    operation.message = match stage {
        "resolve" => format!(
            "Определён адрес {}",
            report.address.as_deref().unwrap_or("")
        ),
        "direct" => "Завершена проверка напрямую; проверяется текущая стратегия nfqws2".into(),
        "route" => "Получен ответ через текущий маршрут; собирается журнал nfqws2 --debug".into(),
        _ => "Трассировка выполняется".into(),
    };
    operation.report = Some(report);
    operation.updated_at = now_unix_seconds();
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StrategyPickOperation {
    id: String,
    host: String,
    protocol: String,
    state: String,
    stage: String,
    current: Option<String>,
    done: usize,
    total: usize,
    results: Vec<strategy_picker::StrategyCandidateResult>,
    message: String,
    updated_at: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StrategyPickRequest {
    host: String,
    protocol: Option<String>,
    mode: Option<String>,
    repeats: Option<usize>,
    #[serde(default)]
    stop_after_first_confirmed: bool,
    #[serde(default)]
    include_community: bool,
    #[serde(default)]
    include_other_history: bool,
    #[serde(default)]
    refine_lighter: bool,
    custom_steps: Option<Vec<Vec<String>>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StrategyPickPreviewRequest {
    candidate_index: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StrategySourcePreviewRequest {
    source: String,
    operation_id: String,
    candidate_index: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StrategyApplyRequest {
    source: String,
    operation_id: String,
    candidate_index: usize,
    expected_revision: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct StrategyPickPreview {
    operation_id: String,
    candidate_index: usize,
    host: String,
    protocol: String,
    candidate: strategy_picker::StrategyCandidate,
    original_revision: String,
    original_content: String,
    proposed_content: String,
    summary: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransferDiagnosticOperation {
    id: String,
    host: String,
    state: String,
    stage: String,
    attempt: usize,
    message: String,
    result: Option<transfer_diagnostic::TransferComparison>,
    updated_at: u64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TransferDiagnosticRequest {
    host: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FreezeScanOperation {
    id: String,
    state: String,
    stage: String,
    current: Option<String>,
    done: usize,
    total: usize,
    results: Vec<freeze_diagnostic::FreezeNetworkResult>,
    message: String,
    updated_at: u64,
}

fn update_freeze_scan_progress(
    operation: &mut FreezeScanOperation,
    done: usize,
    total: usize,
    target: &str,
    completed_results: &[freeze_diagnostic::FreezeNetworkResult],
) {
    operation.stage = if target.contains(": проверка SNI ") {
        "strategy_search"
    } else {
        "baseline"
    }
    .into();
    operation.done = done.min(total);
    operation.total = total;
    operation.current = Some(target.chars().take(128).collect());
    operation.results = completed_results.to_vec();
    operation.message = if operation.stage == "strategy_search" {
        "Проверяется совместимое имя fake-SNI; требуется три последовательных успеха".into()
    } else {
        format!("Проверена сеть {} из {}", operation.done, total)
    };
    operation.updated_at = now_unix_seconds();
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FreezeScanRequest {
    #[serde(default)]
    extra_hosts: Vec<String>,
}

const MAX_SITE_TRACE_STATE_BYTES: usize = 512 * 1024;
const MAX_SITE_TRACE_OPERATIONS: usize = 12;
const MAX_STRATEGY_PICK_STATE_BYTES: usize = 512 * 1024;
const MAX_STRATEGY_PICK_OPERATIONS: usize = 12;
const MAX_TRANSFER_DIAGNOSTIC_STATE_BYTES: usize = 128 * 1024;
const MAX_FREEZE_SCAN_STATE_BYTES: usize = 2 * 1024 * 1024;
const MAX_FREEZE_SCAN_PROGRESS: usize = freeze_diagnostic::MAX_TARGETS * 2;

type SiteTraceRunner = dyn Fn(
        String,
        String,
        Vec<String>,
        String,
        Arc<AtomicBool>,
        usize,
        Option<bool>,
        Option<site_trace::TraceProgressCallback>,
    ) -> Pin<Box<dyn Future<Output = site_trace::TraceReport> + Send>>
    + Send
    + Sync;

#[allow(clippy::too_many_arguments)] // Keeps the injectable async runner boundary explicit.
fn router_site_trace_runner(
    host: String,
    proto: String,
    args: Vec<String>,
    binary: String,
    cancel: Arc<AtomicBool>,
    repeats: usize,
    known_baseline: Option<bool>,
    progress: Option<site_trace::TraceProgressCallback>,
) -> Pin<Box<dyn Future<Output = site_trace::TraceReport> + Send>> {
    Box::pin(async move {
        if let Some(baseline_ok) = known_baseline {
            site_trace::run_candidate_repeated(
                host,
                &proto,
                args,
                binary,
                baseline_ok,
                repeats,
                cancel,
            )
            .await
        } else if let Some(progress) = progress {
            site_trace::run_with_progress(host, &proto, args, binary, cancel, progress).await
        } else {
            site_trace::run(host, &proto, args, binary, cancel).await
        }
    })
}

fn strategy_probe_argv(binary: &str, profile_args: &[String]) -> Vec<String> {
    std::iter::once(binary.to_owned())
        .chain(profile_args.iter().cloned())
        .collect()
}

fn now_unix_seconds() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}

fn valid_site_trace_id(id: &str) -> bool {
    id.len() == 32 && id.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn site_trace_operation_path(directory: &Path, id: &str) -> PathBuf {
    directory.join(format!("{id}.json"))
}

async fn write_site_trace_operation(
    directory: &Path,
    operation: &SiteTraceOperation,
) -> Result<(), String> {
    if !valid_site_trace_id(&operation.id) {
        return Err("Недопустимый идентификатор операции трассировки".into());
    }
    let bytes = serde_json::to_vec(operation).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_SITE_TRACE_STATE_BYTES {
        return Err("Состояние трассировки превышает ограничение размера".into());
    }
    let destination = site_trace_operation_path(directory, &operation.id);
    let temporary = directory.join(format!("{}.{}.tmp", operation.id, new_update_id()));
    tokio::fs::write(&temporary, &bytes)
        .await
        .map_err(|e| format!("Не удалось сохранить статус трассировки: {e}"))?;
    #[cfg(windows)]
    if tokio::fs::try_exists(&destination).await.unwrap_or(false) {
        tokio::fs::remove_file(&destination)
            .await
            .map_err(|e| format!("Не удалось заменить статус трассировки: {e}"))?;
    }
    if let Err(error) = tokio::fs::rename(&temporary, &destination).await {
        let _ = tokio::fs::remove_file(&temporary).await;
        return Err(format!("Не удалось обновить статус трассировки: {error}"));
    }
    Ok(())
}

fn write_site_trace_operation_sync(
    directory: &Path,
    operation: &SiteTraceOperation,
) -> Result<(), String> {
    if !valid_site_trace_id(&operation.id) {
        return Err("Недопустимый идентификатор операции трассировки".into());
    }
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec(operation).map_err(|error| error.to_string())?;
    if bytes.len() > MAX_SITE_TRACE_STATE_BYTES {
        return Err("Состояние трассировки превышает ограничение размера".into());
    }
    let destination = site_trace_operation_path(directory, &operation.id);
    let temporary = directory.join(format!("{}.{}.tmp", operation.id, new_update_id()));
    std::fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    #[cfg(windows)]
    if destination.exists() {
        std::fs::remove_file(&destination).map_err(|error| error.to_string())?;
    }
    if let Err(error) = std::fs::rename(&temporary, &destination) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error.to_string());
    }
    Ok(())
}

async fn read_site_trace_operation(
    directory: &Path,
    id: &str,
) -> Result<Option<SiteTraceOperation>, String> {
    if !valid_site_trace_id(id) {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    let file = match tokio::fs::File::open(site_trace_operation_path(directory, id)).await {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Не удалось прочитать статус трассировки: {error}")),
    };
    file.take((MAX_SITE_TRACE_STATE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|e| format!("Не удалось прочитать статус трассировки: {e}"))?;
    if bytes.len() > MAX_SITE_TRACE_STATE_BYTES {
        return Err("Сохранённое состояние трассировки превышает лимит".into());
    }
    let operation: SiteTraceOperation = serde_json::from_slice(&bytes)
        .map_err(|_| "Повреждено состояние трассировки".to_string())?;
    let valid_state = matches!(
        operation.state.as_str(),
        "queued" | "running" | "stopping" | "complete" | "cancelled" | "failed"
    );
    let valid_report = operation.report.as_ref().is_none_or(|report| {
        report.trace_lines.len() <= site_trace::MAX_TRACE_LINES
            && report
                .trace_lines
                .iter()
                .all(|line| line.len() <= site_trace::MAX_TRACE_LINE_BYTES)
    });
    if operation.id != id || !valid_state || !valid_report {
        return Err("Недопустимое состояние трассировки".into());
    }
    Ok(Some(operation))
}

async fn prune_site_trace_operations(directory: &Path) {
    let Ok(mut entries) = tokio::fs::read_dir(directory).await else {
        return;
    };
    let mut completed = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let Some(id) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        if let Ok(Some(operation)) = read_site_trace_operation(directory, id).await
            && matches!(
                operation.state.as_str(),
                "complete" | "cancelled" | "failed"
            )
        {
            let updated = tokio::fs::metadata(&path)
                .await
                .and_then(|metadata| metadata.modified())
                .ok();
            completed.push((path, updated));
        }
    }
    if completed.len() <= MAX_SITE_TRACE_OPERATIONS {
        return;
    }
    let remove_count = completed.len() - MAX_SITE_TRACE_OPERATIONS;
    completed.sort_by_key(|(_, modified)| *modified);
    for (path, _) in completed.into_iter().take(remove_count) {
        let _ = tokio::fs::remove_file(path).await;
    }
}

fn strategy_pick_operation_path(directory: &Path, id: &str) -> PathBuf {
    directory.join(format!("{id}.json"))
}

fn write_strategy_pick_operation_sync(
    directory: &Path,
    operation: &StrategyPickOperation,
) -> Result<(), String> {
    if !valid_site_trace_id(&operation.id) {
        return Err("Недопустимый идентификатор автоподбора".into());
    }
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec(operation).map_err(|error| error.to_string())?;
    if bytes.len() > MAX_STRATEGY_PICK_STATE_BYTES {
        return Err("Состояние автоподбора превышает ограничение размера".into());
    }
    let destination = strategy_pick_operation_path(directory, &operation.id);
    let temporary = directory.join(format!("{}.{}.tmp", operation.id, new_update_id()));
    std::fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    #[cfg(windows)]
    if destination.exists() {
        std::fs::remove_file(&destination).map_err(|error| error.to_string())?;
    }
    if let Err(error) = std::fs::rename(&temporary, &destination) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error.to_string());
    }
    Ok(())
}

async fn read_strategy_pick_operation(
    directory: &Path,
    id: &str,
) -> Result<Option<StrategyPickOperation>, String> {
    if !valid_site_trace_id(id) {
        return Ok(None);
    }
    let file = match tokio::fs::File::open(strategy_pick_operation_path(directory, id)).await {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Не удалось прочитать статус автоподбора: {error}")),
    };
    let mut bytes = Vec::new();
    file.take((MAX_STRATEGY_PICK_STATE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|error| error.to_string())?;
    if bytes.len() > MAX_STRATEGY_PICK_STATE_BYTES {
        return Err("Сохранённое состояние автоподбора превышает лимит".into());
    }
    let operation: StrategyPickOperation = serde_json::from_slice(&bytes)
        .map_err(|_| "Повреждено состояние автоподбора".to_string())?;
    if operation.id != id
        || !matches!(
            operation.state.as_str(),
            "queued"
                | "running"
                | "stopping"
                | "complete"
                | "no_match"
                | "site_unblocked"
                | "cancelled"
                | "failed"
        )
        || operation.results.len() > strategy_picker::MAX_CANDIDATES
    {
        return Err("Недопустимое состояние автоподбора".into());
    }
    Ok(Some(operation))
}

async fn load_confirmed_strategy_candidates(
    directory: &Path,
    host: &str,
    protocol: &str,
    include_other_hosts: bool,
) -> Vec<strategy_picker::StrategyCandidate> {
    let Ok(mut entries) = tokio::fs::read_dir(directory).await else {
        return Vec::new();
    };
    let mut operations = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        if entry
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("json")
        {
            continue;
        }
        let Some(id) = entry
            .path()
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(str::to_owned)
        else {
            continue;
        };
        if let Ok(Some(operation)) = read_strategy_pick_operation(directory, &id).await
            && operation.protocol == protocol
            && matches!(
                operation.state.as_str(),
                "complete" | "no_match" | "cancelled" | "failed"
            )
        {
            operations.push(operation);
        }
    }
    operations.sort_by_key(|operation| {
        (
            operation.host != host,
            std::cmp::Reverse(operation.updated_at),
        )
    });
    let mut seen = std::collections::HashSet::new();
    let mut same_host = Vec::new();
    let mut other_hosts = Vec::new();
    for operation in operations {
        if operation.state == "site_unblocked" {
            continue;
        }
        let source_host = operation.host.clone();
        for result in operation
            .results
            .into_iter()
            .filter(|result| result.confirmed)
        {
            let candidate = result.candidate;
            if seen.insert(candidate.steps.clone()) {
                let history_candidate = strategy_picker::StrategyCandidate {
                    name: candidate.name,
                    source: if source_host == host {
                        "Ранее подтверждена для этого домена".into()
                    } else {
                        format!("История: раньше помогла {source_host}")
                    },
                    steps: candidate.steps,
                };
                if source_host == host {
                    if same_host.len() < 5 {
                        same_host.push(history_candidate);
                    }
                } else if include_other_hosts && other_hosts.len() < 5 {
                    other_hosts.push(history_candidate);
                }
            }
        }
    }
    same_host.extend(other_hosts);
    same_host
}

async fn load_confirmed_freeze_candidates(
    directory: &Path,
) -> Vec<strategy_picker::StrategyCandidate> {
    let Ok(mut entries) = tokio::fs::read_dir(directory).await else {
        return Vec::new();
    };
    let mut newest: Option<FreezeScanOperation> = None;
    while let Ok(Some(entry)) = entries.next_entry().await {
        if entry
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("json")
        {
            continue;
        }
        let Some(id) = entry
            .path()
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(str::to_owned)
        else {
            continue;
        };
        if let Ok(Some(operation)) = read_freeze_scan_operation(directory, &id).await
            && matches!(operation.state.as_str(), "complete" | "partial")
            && newest
                .as_ref()
                .is_none_or(|previous| operation.updated_at > previous.updated_at)
        {
            newest = Some(operation);
        }
    }
    newest.map_or_else(Vec::new, |operation| {
        freeze_diagnostic::confirmed_strategy_candidates(&operation.results)
    })
}

fn build_strategy_pick_preview(
    operation: &StrategyPickOperation,
    candidate_index: usize,
    original_content: &str,
    original_revision: String,
    functions: &HashSet<String>,
    blobs: &[String],
) -> Result<StrategyPickPreview, String> {
    if operation.state != "complete" {
        return Err("Применять можно только завершённый подбор".into());
    }
    let protocol = match operation.protocol.as_str() {
        "http" => StrategyProtocol::Http,
        "https" => StrategyProtocol::Https,
        _ => return Err("Применение этой версии протокола не поддерживается".into()),
    };
    let result = operation
        .results
        .get(candidate_index)
        .ok_or_else(|| "Индекс кандидата вне сохранённого результата подбора".to_string())?;
    if !result.confirmed || result.attempts < 2 || result.successes < 2 {
        return Err("Кандидат не прошёл повторное подтверждение; preview недоступен".into());
    }
    let profile = strategy_picker::build_domain_profile(
        &operation.host,
        protocol,
        &result.candidate.steps,
        functions,
        blobs,
    )?;
    let document = ConfigDocument::parse(original_content)?;
    let custom_args = document.literal_value("NFQWS_ARGS_CUSTOM")?;
    let selector = format!("--hostlist-domains={}", operation.host);
    if custom_args.split_whitespace().any(|arg| arg == selector) {
        return Err(
            "Для этого домена уже есть пользовательский профиль; сначала проверьте его вручную"
                .into(),
        );
    }
    let proposed_content = document.append_literal_words("NFQWS_ARGS_CUSTOM", &profile)?;
    let diagnostics = validate_nfqws2_config(&proposed_content)?;
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    {
        return Err("Сформированный профиль не прошёл статическую проверку конфигурации".into());
    }
    if proposed_content.len() > MAX_CONFIG_BYTES {
        return Err("Изменённая конфигурация превышает лимит размера".into());
    }
    Ok(StrategyPickPreview {
        operation_id: operation.id.clone(),
        candidate_index,
        host: operation.host.clone(),
        protocol: operation.protocol.clone(),
        candidate: result.candidate.clone(),
        original_revision,
        original_content: original_content.to_owned(),
        proposed_content,
        summary: "Будет добавлен отдельный доменный профиль в NFQWS_ARGS_CUSTOM. Применение и перезапуск пока не выполняются.".into(),
    })
}

fn strategy_dry_run_argv(
    operation: &StrategyPickOperation,
    candidate_index: usize,
    active_args: &[String],
    functions: &HashSet<String>,
    blobs: &[String],
) -> Result<Vec<String>, String> {
    let protocol = match operation.protocol.as_str() {
        "http" => StrategyProtocol::Http,
        "https" => StrategyProtocol::Https,
        _ => return Err("Для dry-run доступны только HTTP и HTTPS кандидаты".into()),
    };
    let candidate = operation
        .results
        .get(candidate_index)
        .filter(|result| result.confirmed && result.attempts >= 2 && result.successes >= 2)
        .ok_or_else(|| {
            "Dry-run разрешён только для повторно подтверждённого кандидата".to_string()
        })?;
    let profile_args = strategy_picker::candidate_argv(
        active_args,
        protocol,
        &candidate.candidate.steps,
        functions,
        blobs,
    )?;
    let mut argv = vec![active_args[0].clone()];
    let queue = active_args
        .iter()
        .skip(1)
        .enumerate()
        .find_map(|(index, argument)| {
            if argument == "--qnum" {
                active_args
                    .get(index + 2)
                    .map(|value| format!("--qnum={value}"))
            } else {
                argument
                    .strip_prefix("--qnum=")
                    .map(|value| format!("--qnum={value}"))
            }
        })
        .ok_or_else(|| {
            "Не найден --qnum работающего nfqws2; нельзя безопасно проверить профиль".to_owned()
        })?;
    argv.extend([queue]);
    argv.extend(profile_args);
    argv.insert(1, "--dry-run".into());
    Ok(argv)
}

async fn validate_strategy_dry_run(argv: &[String]) -> Result<String, String> {
    let Some(binary) = argv.first() else {
        return Err("Пустой argv для проверки nfqws2 --dry-run".into());
    };
    let output = tokio::time::timeout(
        std::time::Duration::from_secs(20),
        tokio::process::Command::new(binary)
            .args(&argv[1..])
            .kill_on_drop(true)
            .output(),
    )
    .await
    .map_err(|_| "nfqws2 --dry-run превысил лимит 20 секунд; конфигурация не изменена".to_string())?
    .map_err(|error| format!("Не удалось запустить nfqws2 --dry-run: {error}"))?;
    let detail = format!(
        "{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
    if !output.status.success() {
        let detail = detail.trim();
        return Err(if detail.is_empty() {
            format!(
                "nfqws2 --dry-run завершился с кодом {:?}; конфигурация не изменена",
                output.status.code()
            )
        } else {
            format!(
                "nfqws2 --dry-run отклонил профиль; конфигурация не изменена: {}",
                detail.chars().take(1500).collect::<String>()
            )
        });
    }
    Ok("nfqws2 --dry-run пройден".into())
}

async fn preview_strategy_pick(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(request): Json<StrategyPickPreviewRequest>,
) -> Response {
    if !valid_site_trace_id(&id) {
        return error(StatusCode::NOT_FOUND, "Операция автоподбора не найдена");
    }
    let operation_dir = state.data_dir.join("strategy-pick-operations");
    let operation = match read_strategy_pick_operation(&operation_dir, &id).await {
        Ok(Some(operation)) => operation,
        Ok(None) => return error(StatusCode::NOT_FOUND, "Операция автоподбора не найдена"),
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    };
    let Some((_, active_args)) = active_process_snapshot() else {
        return error(
            StatusCode::CONFLICT,
            "Preview остановлен: работающий nfqws2 не обнаружен однозначно",
        );
    };
    let Some(paths) = discover_system_paths() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Пути установленного nfqws2 не обнаружены однозначно",
        );
    };
    let functions = active_lua_functions(&active_args, &paths.lua_dir);
    let blobs = active_blob_names(&active_args);
    let Some(config_path) = config_path_for_state(&state) else {
        return error(
            StatusCode::NOT_FOUND,
            "Разрешённый конфигурационный файл не найден",
        );
    };
    let bytes = match tokio::fs::read(config_path).await {
        Ok(bytes) if bytes.len() <= MAX_CONFIG_BYTES => bytes,
        Ok(_) => {
            return error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Конфигурация превышает лимит 512 КБ",
            );
        }
        Err(io_error) => {
            return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string());
        }
    };
    let content = match String::from_utf8(bytes.clone()) {
        Ok(content) => content,
        Err(_) => {
            return error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Конфигурация не является UTF-8 текстом",
            );
        }
    };
    match build_strategy_pick_preview(
        &operation,
        request.candidate_index,
        &content,
        revision(&bytes),
        &functions,
        &blobs,
    ) {
        Ok(preview) => Json(preview).into_response(),
        Err(message) => error(StatusCode::UNPROCESSABLE_ENTITY, message),
    }
}

async fn preview_freeze_strategy(
    State(state): State<Arc<AppState>>,
    Json(request): Json<StrategySourcePreviewRequest>,
) -> Response {
    if request.source != "freezeScan" || !valid_site_trace_id(&request.operation_id) {
        return error(
            StatusCode::BAD_REQUEST,
            "Некорректный источник результата диагностики",
        );
    }
    let scan = match read_freeze_scan_operation(
        &state.data_dir.join("freeze-diagnostic-operations"),
        &request.operation_id,
    )
    .await
    {
        Ok(Some(operation)) => operation,
        Ok(None) => return error(StatusCode::NOT_FOUND, "Результат теста 16 КБ не найден"),
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    };
    let operation = match freeze_result_as_strategy_operation(&scan, request.candidate_index) {
        Ok(operation) => operation,
        Err(message) => return error(StatusCode::UNPROCESSABLE_ENTITY, message),
    };
    let Some((_, active_args)) = active_process_snapshot() else {
        return error(
            StatusCode::CONFLICT,
            "Preview остановлен: работающий nfqws2 не обнаружен однозначно",
        );
    };
    let Some(paths) = discover_system_paths() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Пути установленного nfqws2 не обнаружены однозначно",
        );
    };
    let functions = active_lua_functions(&active_args, &paths.lua_dir);
    let blobs = active_blob_names(&active_args);
    let Some(config_path) = config_path_for_state(&state) else {
        return error(
            StatusCode::NOT_FOUND,
            "Разрешённый конфигурационный файл не найден",
        );
    };
    let original = match tokio::fs::read(config_path).await {
        Ok(bytes) if bytes.len() <= MAX_CONFIG_BYTES => bytes,
        Ok(_) => {
            return error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Конфигурация превышает лимит 512 КБ",
            );
        }
        Err(io_error) => return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string()),
    };
    let content = match String::from_utf8(original.clone()) {
        Ok(content) => content,
        Err(_) => {
            return error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Конфигурация не является UTF-8 текстом",
            );
        }
    };
    match build_strategy_pick_preview(
        &operation,
        0,
        &content,
        revision(&original),
        &functions,
        &blobs,
    ) {
        Ok(preview) => Json(preview).into_response(),
        Err(message) => error(StatusCode::UNPROCESSABLE_ENTITY, message),
    }
}

fn freeze_result_as_strategy_operation(
    operation: &FreezeScanOperation,
    result_index: usize,
) -> Result<StrategyPickOperation, String> {
    if !matches!(
        operation.state.as_str(),
        "running" | "complete" | "partial" | "cancelled"
    ) {
        return Err(
            "Для применения нужен завершённый тест домена с сохранённым подтверждением".into(),
        );
    }
    let result = operation
        .results
        .get(result_index)
        .ok_or_else(|| "Индекс результата теста 16 КБ не найден".to_string())?;
    if result.target_kind != freeze_diagnostic::FreezeTargetKind::UserHost {
        return Err(
            "Стратегию из теста сети можно применять только после отдельного подбора для домена"
                .into(),
        );
    }
    let found = result
        .found_strategy
        .as_ref()
        .filter(|found| found.attempts >= 3 && found.successes >= 3 && found.reused_from.is_none())
        .ok_or_else(|| {
            "Для этого домена нет стратегии, подтверждённой тремя проверками".to_string()
        })?;
    Ok(StrategyPickOperation {
        id: operation.id.clone(),
        host: result.host.clone(),
        protocol: "https".into(),
        state: "complete".into(),
        stage: "complete".into(),
        current: None,
        done: 1,
        total: 1,
        results: vec![strategy_picker::StrategyCandidateResult {
            candidate: strategy_picker::StrategyCandidate {
                name: format!("Подтверждено тестом 16 КБ · {}", found.sni),
                source: "Проверено на введённом домене".into(),
                steps: vec![found.step.clone()],
            },
            attempts: found.attempts,
            successes: found.successes,
            confirmed: true,
            elapsed_ms: found.average_ms.unwrap_or_default(),
            detail: "Три последовательные проверки прошли на этом домене".into(),
        }],
        message: "Проверка 16 КБ завершена".into(),
        updated_at: operation.updated_at,
    })
}

async fn apply_confirmed_strategy(
    State(state): State<Arc<AppState>>,
    Json(request): Json<StrategyApplyRequest>,
) -> Response {
    if !valid_site_trace_id(&request.operation_id) {
        return error(StatusCode::NOT_FOUND, "Операция диагностики не найдена");
    }
    let operation = match request.source.as_str() {
        "strategyPick" => {
            match read_strategy_pick_operation(
                &state.data_dir.join("strategy-pick-operations"),
                &request.operation_id,
            )
            .await
            {
                Ok(Some(operation)) => operation,
                Ok(None) => return error(StatusCode::NOT_FOUND, "Результат автоподбора не найден"),
                Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
            }
        }
        "freezeScan" => {
            let scan = match read_freeze_scan_operation(
                &state.data_dir.join("freeze-diagnostic-operations"),
                &request.operation_id,
            )
            .await
            {
                Ok(Some(operation)) => operation,
                Ok(None) => return error(StatusCode::NOT_FOUND, "Результат теста 16 КБ не найден"),
                Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
            };
            match freeze_result_as_strategy_operation(&scan, request.candidate_index) {
                Ok(operation) => operation,
                Err(message) => return error(StatusCode::UNPROCESSABLE_ENTITY, message),
            }
        }
        _ => {
            return error(
                StatusCode::BAD_REQUEST,
                "Неизвестный источник подтверждённой стратегии",
            );
        }
    };
    let pause_control = if request.source == "freezeScan" && operation.state == "running" {
        let Some(control) = state
            .freeze_scan_controls
            .lock()
            .await
            .get(&request.operation_id)
            .cloned()
        else {
            return error(
                StatusCode::CONFLICT,
                "Проверка уже завершается; обновите результат и повторите применение",
            );
        };
        if let Err(message) = control.request_pause().await {
            let latest = read_freeze_scan_operation(
                &state.data_dir.join("freeze-diagnostic-operations"),
                &request.operation_id,
            )
            .await
            .ok()
            .flatten();
            if !latest.is_some_and(|scan| {
                matches!(scan.state.as_str(), "complete" | "partial" | "cancelled")
            }) {
                return error(StatusCode::CONFLICT, message);
            }
            None
        } else {
            Some(control)
        }
    } else {
        None
    };
    let _resume_scan = ResumeFreezeScanAfterApply(pause_control.clone());
    let _diagnostic_guard = if pause_control.is_some() {
        Some(state.site_diagnostic_lock.clone().lock_owned().await)
    } else {
        match state.site_diagnostic_lock.clone().try_lock_owned() {
            Ok(guard) => Some(guard),
            Err(_) => {
                return error(
                    StatusCode::CONFLICT,
                    "Сначала дождитесь завершения текущей сетевой диагностики",
                );
            }
        }
    };
    let Some((active_pid, active_args)) = active_process_snapshot() else {
        return error(
            StatusCode::CONFLICT,
            "Применение остановлено: работающий nfqws2 не обнаружен однозначно",
        );
    };
    let Some(paths) = discover_system_paths() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Пути установленного nfqws2 не обнаружены однозначно",
        );
    };
    let functions = active_lua_functions(&active_args, &paths.lua_dir);
    let blobs = active_blob_names(&active_args);
    let Some(config_path) = config_path_for_state(&state) else {
        return error(
            StatusCode::NOT_FOUND,
            "Разрешённый конфигурационный файл не найден",
        );
    };
    let original = match tokio::fs::read(&config_path).await {
        Ok(bytes) if bytes.len() <= MAX_CONFIG_BYTES => bytes,
        Ok(_) => {
            return error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Конфигурация превышает лимит 512 КБ",
            );
        }
        Err(io_error) => return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string()),
    };
    let content = match String::from_utf8(original.clone()) {
        Ok(content) => content,
        Err(_) => {
            return error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Конфигурация не является UTF-8 текстом",
            );
        }
    };
    let current_revision = revision(&original);
    if current_revision != request.expected_revision {
        return error(
            StatusCode::CONFLICT,
            "Конфигурация изменилась после предпросмотра; обновите preview и проверьте diff",
        );
    }
    let candidate_index = if request.source == "freezeScan" {
        0
    } else {
        request.candidate_index
    };
    let dry_run_argv = match strategy_dry_run_argv(
        &operation,
        candidate_index,
        &active_args,
        &functions,
        &blobs,
    ) {
        Ok(argv) => argv,
        Err(message) => return error(StatusCode::UNPROCESSABLE_ENTITY, message),
    };
    if let Err(message) = validate_strategy_dry_run(&dry_run_argv).await {
        return error(StatusCode::UNPROCESSABLE_ENTITY, message);
    }
    if !matches!(active_process_snapshot(), Some((pid, ref args)) if pid == active_pid && args == &active_args)
    {
        return error(
            StatusCode::CONFLICT,
            "Активный процесс nfqws2 изменился во время проверки; конфигурация не изменена",
        );
    }
    let preview = match build_strategy_pick_preview(
        &operation,
        candidate_index,
        &content,
        current_revision.clone(),
        &functions,
        &blobs,
    ) {
        Ok(preview) => preview,
        Err(message) => return error(StatusCode::UNPROCESSABLE_ENTITY, message),
    };
    let write = ConfigWrite {
        content: preview.proposed_content,
        revision: current_revision,
    };
    match apply_config_transaction(&state, write, || async {
        service_action(ServiceAction::Restart).await
    })
    .await
    {
        Ok(result) => Json(result).into_response(),
        Err(message) if message == WRITE_REVISION_CONFLICT => error(
            StatusCode::CONFLICT,
            "Конфигурация изменилась во время сохранения; повторите preview",
        ),
        Err(message) => error(StatusCode::BAD_GATEWAY, message),
    }
}

struct ResumeFreezeScanAfterApply(Option<Arc<freeze_diagnostic::FreezeScanPause>>);

impl Drop for ResumeFreezeScanAfterApply {
    fn drop(&mut self) {
        if let Some(control) = &self.0 {
            control.resume_after_apply();
        }
    }
}

fn prune_finished_json_operations(directory: &Path, limit: usize) {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return;
    };
    let mut finished = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let metadata = match entry.metadata() {
            Ok(metadata) if metadata.len() <= MAX_STRATEGY_PICK_STATE_BYTES as u64 => metadata,
            _ => {
                let _ = std::fs::remove_file(path);
                continue;
            }
        };
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        let Ok(value) = serde_json::from_slice::<serde_json::Value>(&bytes) else {
            let _ = std::fs::remove_file(path);
            continue;
        };
        let Some(state) = value.get("state").and_then(serde_json::Value::as_str) else {
            let _ = std::fs::remove_file(path);
            continue;
        };
        if matches!(state, "queued" | "running" | "stopping") {
            continue;
        }
        if !matches!(
            state,
            "complete" | "no_match" | "site_unblocked" | "cancelled" | "failed"
        ) {
            let _ = std::fs::remove_file(path);
            continue;
        }
        finished.push((path, metadata.modified().ok()));
    }
    if finished.len() <= limit {
        return;
    }
    finished.sort_by_key(|(_, modified)| *modified);
    let remove_count = finished.len().saturating_sub(limit);
    for (path, _) in finished.into_iter().take(remove_count) {
        let _ = std::fs::remove_file(path);
    }
}

fn active_lua_functions(args: &[String], lua_dir: &Path) -> HashSet<String> {
    let mut functions = HashSet::new();
    for (index, argument) in args.iter().enumerate() {
        let value = if let Some(value) = argument.strip_prefix("--lua-init=") {
            value
        } else if argument == "--lua-init" {
            let Some(value) = args.get(index + 1).map(String::as_str) else {
                continue;
            };
            value
        } else {
            continue;
        };
        let value = value.strip_prefix('@').unwrap_or(value);
        if value.is_empty() || value.len() > 4096 || value.contains(['\n', '\r', '\0']) {
            continue;
        }
        let requested = PathBuf::from(value);
        let mut candidates = vec![requested.clone()];
        if !requested.is_absolute() {
            candidates.push(lua_dir.join(&requested));
        }
        if requested
            .extension()
            .and_then(|extension| extension.to_str())
            == Some("lua")
        {
            let mut gz = requested.clone();
            gz.set_extension("lua.gz");
            candidates.push(gz.clone());
            if !gz.is_absolute() {
                candidates.push(lua_dir.join(gz.file_name().unwrap_or_default()));
            }
        }
        let Some(path) = candidates.into_iter().find(|path| path.is_file()) else {
            continue;
        };
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        if bytes.len() > 4 * 1024 * 1024 {
            continue;
        }
        let decoded = if path.extension().and_then(|extension| extension.to_str()) == Some("gz") {
            let mut output = Vec::new();
            if flate2::read::GzDecoder::new(bytes.as_slice())
                .take((4 * 1024 * 1024 + 1) as u64)
                .read_to_end(&mut output)
                .is_err()
                || output.len() > 4 * 1024 * 1024
            {
                continue;
            }
            output
        } else {
            bytes
        };
        functions.extend(strategy_picker::parse_lua_functions(
            &String::from_utf8_lossy(&decoded),
        ));
    }
    functions
}

fn active_blob_names(args: &[String]) -> Vec<String> {
    let mut names = vec![
        "fake_default_tls".into(),
        "fake_default_http".into(),
        "fake_default_quic".into(),
    ];
    for (index, argument) in args.iter().enumerate() {
        let value = argument.strip_prefix("--blob=").or_else(|| {
            (argument == "--blob")
                .then(|| args.get(index + 1).map(String::as_str))
                .flatten()
        });
        if let Some(value) = value
            && let Some((name, _)) = value.split_once(':')
            && name.len() <= 128
            && name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || b"_-./".contains(&byte))
        {
            names.push(name.to_string());
        }
    }
    names.sort();
    names.dedup();
    names
}

#[derive(Serialize)]
struct ApiError {
    error: String,
}

#[derive(Serialize)]
struct StatusResponse {
    service: ServiceInfo,
    system: SystemInfo,
    panel: PanelInfo,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PathInfo {
    path: String,
    exists: bool,
    readable: bool,
    writable: bool,
    used_by_active_config: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct InstallationResponse {
    platform: &'static str,
    installation_root: Option<String>,
    detected_binary: Option<PathInfo>,
    network_interfaces: Vec<String>,
    config: Option<PathInfo>,
    config_candidates: Vec<PathInfo>,
    init_script: Option<PathInfo>,
    init_script_candidates: Vec<PathInfo>,
    binary_candidates: Vec<PathInfo>,
    lists: Vec<PathInfo>,
    module_references: Vec<ModuleReference>,
}

const LUA_SCRIPT_NAMES: [&str; 6] = [
    "zapret-antidpi.lua",
    "zapret-auto.lua",
    "zapret-lib.lua",
    "zapret-obfs.lua",
    "zapret-pcap.lua",
    "zapret-tests.lua",
];

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LuaScriptInfo {
    name: &'static str,
    path: Option<String>,
    exists: bool,
    readable: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct LuaScriptContent {
    name: &'static str,
    path: String,
    compressed: bool,
    content: String,
    truncated: bool,
}

fn resolve_lua_script(lua_dir: &Path, name: &str) -> Option<(PathBuf, bool)> {
    if !LUA_SCRIPT_NAMES.contains(&name) || Path::new(name).file_name()?.to_str()? != name {
        return None;
    }
    let plain = lua_dir.join(name);
    let candidates = [
        (plain.clone(), false),
        (PathBuf::from(format!("{}.gz", plain.display())), true),
    ];
    let canonical_dir = lua_dir.canonicalize().ok()?;
    candidates.into_iter().find_map(|(path, compressed)| {
        let canonical = path.canonicalize().ok()?;
        (canonical.starts_with(&canonical_dir) && canonical.is_file())
            .then_some((canonical, compressed))
    })
}

fn read_lua_script(lua_dir: &Path, name: &str) -> std::io::Result<LuaScriptContent> {
    const MAX_SCRIPT_BYTES: u64 = 512 * 1024;
    let (path, compressed) = resolve_lua_script(lua_dir, name)
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "Lua-скрипт не найден"))?;
    let file = std::fs::File::open(&path)?;
    let (bytes, truncated) = if compressed {
        use std::io::Read;
        let mut decoder = flate2::read::GzDecoder::new(file);
        let mut bytes = Vec::new();
        decoder
            .by_ref()
            .take(MAX_SCRIPT_BYTES + 1)
            .read_to_end(&mut bytes)?;
        let truncated = bytes.len() as u64 > MAX_SCRIPT_BYTES;
        bytes.truncate(MAX_SCRIPT_BYTES as usize);
        (bytes, truncated)
    } else {
        use std::io::Read;
        let mut bytes = Vec::new();
        file.take(MAX_SCRIPT_BYTES + 1).read_to_end(&mut bytes)?;
        let truncated = bytes.len() as u64 > MAX_SCRIPT_BYTES;
        bytes.truncate(MAX_SCRIPT_BYTES as usize);
        (bytes, truncated)
    };
    let content = String::from_utf8_lossy(&bytes).into_owned();
    Ok(LuaScriptContent {
        name: LUA_SCRIPT_NAMES
            .iter()
            .find(|candidate| **candidate == name)
            .unwrap(),
        path: path.display().to_string(),
        compressed,
        content,
        truncated,
    })
}

async fn get_lua_scripts() -> Response {
    let Some(paths) = discover_system_paths() else {
        return Json(Vec::<LuaScriptInfo>::new()).into_response();
    };
    let scripts = LUA_SCRIPT_NAMES
        .iter()
        .map(|&name| {
            let resolved = resolve_lua_script(&paths.lua_dir, name);
            LuaScriptInfo {
                name,
                path: resolved
                    .as_ref()
                    .map(|(path, _)| path.display().to_string()),
                exists: resolved.is_some(),
                readable: resolved
                    .as_ref()
                    .is_some_and(|(path, _)| std::fs::File::open(path).is_ok()),
            }
        })
        .collect::<Vec<_>>();
    Json(scripts).into_response()
}

async fn get_lua_script(axum::extract::Path(name): axum::extract::Path<String>) -> Response {
    let Some(paths) = discover_system_paths() else {
        return error(StatusCode::NOT_FOUND, "Установка nfqws2 не обнаружена");
    };
    match read_lua_script(&paths.lua_dir, &name) {
        Ok(script) => Json(script).into_response(),
        Err(io_error) if io_error.kind() == std::io::ErrorKind::NotFound => error(
            StatusCode::NOT_FOUND,
            "Lua-скрипт не найден в каталоге установки",
        ),
        Err(io_error) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Не удалось прочитать Lua-скрипт: {io_error}"),
        ),
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeResponse {
    running: bool,
    pid: Option<u32>,
    source: &'static str,
    profiles: Vec<RuntimeProfile>,
    limitations: Vec<String>,
}

#[derive(Debug, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RuntimeProfile {
    index: usize,
    name: Option<String>,
    filters: Vec<String>,
    strategies: Vec<String>,
    payloads: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ModuleReference {
    kind: &'static str,
    argument: String,
    path: Option<String>,
    resolved_path: Option<String>,
    exists: bool,
    readable: bool,
    resolution: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ServiceInfo {
    state: ServiceState,
    detail: String,
    checked_at: u64,
    version: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SystemInfo {
    memory_total_bytes: Option<u64>,
    memory_used_bytes: Option<u64>,
    cpu_percent: Option<f64>,
    source: &'static str,
}

#[derive(Serialize)]
struct PanelInfo {
    version: &'static str,
    port: u16,
}

#[derive(Deserialize)]
struct ServiceRequest {
    action: String,
}

#[derive(Serialize)]
struct MessageResponse {
    message: String,
}

#[derive(Deserialize)]
struct ConfigQuery {
    file: String,
}

#[derive(Deserialize)]
struct ConfigWrite {
    content: String,
    revision: String,
}

#[derive(Deserialize)]
struct ConfigPreviewRequest {
    content: String,
    key: String,
    value: String,
}

#[derive(Deserialize)]
struct ConfigValidateRequest {
    content: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigValidationDiagnostic {
    severity: &'static str,
    code: String,
    message: String,
    line: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigValidationResponse {
    valid: bool,
    diagnostics: Vec<ConfigValidationDiagnostic>,
    checks: ConfigValidationChecks,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigValidationChecks {
    parser: &'static str,
    installed_binary: &'static str,
}

#[derive(Deserialize)]
struct SiteDiagnosticRequest {
    host: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct MonitoringRequest {
    hosts: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MonitoringHostResult {
    host: String,
    summary: String,
    current: MonitoringStep,
    steps: Vec<MonitoringStep>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MonitoringStep {
    id: &'static str,
    status: &'static str,
    detail: String,
    duration_ms: Option<u64>,
    http_code: Option<u16>,
}

#[derive(Deserialize)]
struct SiteTraceRequest {
    host: String,
    proto: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigResponse {
    file: String,
    content: String,
    revision: String,
}

#[derive(Debug, Serialize)]
struct ConfigApplyResponse {
    config: ConfigResponse,
    message: String,
}

#[derive(Deserialize)]
struct LogsQuery {
    source: String,
    tail: Option<usize>,
}

#[derive(Serialize)]
struct LogsResponse {
    content: String,
}

#[derive(Deserialize)]
struct UpdateInstallRequest {
    tag: String,
}

#[derive(Deserialize)]
struct CoreUpdateInstallRequest {
    tag: String,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateOperation {
    id: String,
    state: String,
    message: String,
}

#[derive(Deserialize)]
struct GithubReleaseRecord {
    tag_name: String,
    published_at: Option<String>,
    body: Option<String>,
    draft: bool,
    prerelease: bool,
    assets: Vec<GithubReleaseAsset>,
}

#[derive(Deserialize)]
struct GithubReleaseAsset {
    name: String,
    #[serde(default)]
    digest: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PanelRelease {
    tag: String,
    published_at: Option<String>,
    notes: String,
}

#[derive(Serialize, Clone)]
#[serde(rename_all = "camelCase")]
struct CoreRelease {
    tag: String,
    published_at: Option<String>,
    notes: String,
    asset: String,
    sha256: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupItem {
    id: String,
    created_at: u64,
    file: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SnapshotCreateRequest {
    #[serde(default)]
    note: String,
}

#[derive(Deserialize)]
struct SnapshotPinRequest {
    pinned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UiAppearanceSettings {
    theme: String,
    layout: String,
}

#[derive(Serialize)]
struct BackupsResponse {
    backups: Vec<BackupItem>,
}

#[derive(Serialize)]
struct ListsResponse {
    files: Vec<ListSummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListSummary {
    id: String,
    path: String,
    readable: bool,
    writable: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListResponse {
    id: String,
    path: String,
    content: String,
    revision: String,
}

#[derive(Deserialize)]
struct ListWrite {
    content: String,
    revision: String,
}

#[derive(Deserialize)]
struct ListDelete {
    revision: String,
}

fn error(status: StatusCode, message: impl Into<String>) -> Response {
    (
        status,
        Json(ApiError {
            error: message.into(),
        }),
    )
        .into_response()
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok":true,"service":"nfqws2-ui"}))
}

fn inspect_path(path: &Path, used_by_active_config: bool) -> PathInfo {
    let exists = path.is_file();
    PathInfo {
        path: path.display().to_string(),
        exists,
        readable: exists && std::fs::File::open(path).is_ok(),
        writable: exists && std::fs::OpenOptions::new().append(true).open(path).is_ok(),
        used_by_active_config,
    }
}

fn valid_release_tag(tag: &str) -> bool {
    let Some(version) = tag.strip_prefix('v') else {
        return false;
    };
    let parts = version.split('.').collect::<Vec<_>>();
    parts.len() == 3
        && parts.iter().all(|part| {
            !part.is_empty()
                && part.bytes().all(|byte| byte.is_ascii_digit())
                && (part == &"0" || !part.starts_with('0'))
        })
}

fn eligible_panel_releases(json: &str) -> Result<Vec<PanelRelease>, String> {
    if json.len() > MAX_RELEASE_RESPONSE_BYTES {
        return Err("Ответ GitHub Releases превышает лимит 1 МБ".into());
    }
    let records: Vec<GithubReleaseRecord> = serde_json::from_str(json)
        .map_err(|error| format!("Некорректный ответ GitHub Releases: {error}"))?;
    Ok(records
        .into_iter()
        .filter_map(|release| {
            if release.draft || release.prerelease || !valid_release_tag(&release.tag_name) {
                return None;
            }
            let asset = format!(
                "nfqws2-ui-{}-aarch64-unknown-linux-musl.tar.gz",
                release.tag_name
            );
            let checksum = format!("{asset}.sha256");
            if !release.assets.iter().any(|item| item.name == asset)
                || !release.assets.iter().any(|item| item.name == checksum)
            {
                return None;
            }
            let notes = release
                .body
                .unwrap_or_default()
                .chars()
                .take(24 * 1024)
                .collect();
            Some(PanelRelease {
                tag: release.tag_name,
                published_at: release.published_at,
                notes,
            })
        })
        .take(100)
        .collect())
}

fn numeric_release_version(version: &str) -> Option<(u64, u64, u64)> {
    let version = version.strip_prefix('v').unwrap_or(version);
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    parts.next().is_none().then_some((major, minor, patch))
}

fn ui_update_status(
    current_version: &str,
    releases: &[PanelRelease],
    checked_at: u64,
) -> UiUpdateStatus {
    let current = numeric_release_version(current_version);
    let latest = releases
        .iter()
        .filter_map(|release| {
            numeric_release_version(&release.tag).map(|version| (version, release))
        })
        .max_by_key(|(version, _)| *version);
    let latest_version = latest.map(|(_, release)| release.tag.clone());
    let update_available = match (current, latest) {
        (Some(current), Some((latest, _))) => latest > current,
        _ => false,
    };
    let release_url = if update_available {
        latest_version
            .as_ref()
            .map(|tag| format!("https://github.com/rangerxxyy/nfqws2-ui/releases/tag/{tag}"))
    } else {
        None
    };
    UiUpdateStatus {
        current_version: current_version.trim_start_matches('v').to_owned(),
        latest_version,
        update_available,
        release_url,
        checked_at: Some(checked_at),
    }
}

fn cache_ui_update_status(
    cache: &std::sync::RwLock<UiUpdateStatus>,
    releases: &[PanelRelease],
    checked_at: u64,
) -> UiUpdateStatus {
    let status = ui_update_status(env!("CARGO_PKG_VERSION"), releases, checked_at);
    if let Ok(mut cached) = cache.write() {
        *cached = status.clone();
    }
    status
}

fn start_ui_update_checker(state: Arc<AppState>) {
    tokio::spawn(async move {
        loop {
            let retry_after = match fetch_github_releases().await {
                Ok(releases) => {
                    cache_ui_update_status(&state.ui_update_status, &releases, now_unix_seconds());
                    std::time::Duration::from_secs(4 * 60 * 60)
                }
                Err(error) => {
                    tracing::warn!("Background UI update check failed: {error}");
                    std::time::Duration::from_secs(15 * 60)
                }
            };
            tokio::time::sleep(retry_after).await;
        }
    });
}

fn eligible_core_releases(json: &str, architecture: &str) -> Result<Vec<CoreRelease>, String> {
    if json.len() > MAX_RELEASE_RESPONSE_BYTES {
        return Err("Ответ GitHub Releases превысил лимит".into());
    }
    if architecture != "aarch64-3.10" {
        return Err(
            "Выбор версий nfqws2 пока поддерживается только на Entware aarch64-3.10".into(),
        );
    }
    let records: Vec<GithubReleaseRecord> = serde_json::from_str(json)
        .map_err(|error| format!("Некорректный список релизов nfqws2: {error}"))?;
    let mut releases = Vec::new();
    for record in records {
        if record.draft || record.prerelease || !valid_release_tag(&record.tag_name) {
            continue;
        }
        let version = record.tag_name.trim_start_matches('v');
        let arch_asset = format!("nfqws2-keenetic_{version}_{architecture}.ipk");
        let Some(sha256) = record
            .assets
            .iter()
            .find(|asset| asset.name == arch_asset)
            .and_then(|asset| asset.digest.as_deref())
            .and_then(valid_github_sha256)
        else {
            continue;
        };
        releases.push(CoreRelease {
            tag: record.tag_name,
            published_at: record.published_at,
            notes: record
                .body
                .unwrap_or_default()
                .chars()
                .take(24_000)
                .collect(),
            asset: arch_asset,
            sha256,
        });
    }
    Ok(releases)
}

fn valid_github_sha256(digest: &str) -> Option<String> {
    let value = digest.strip_prefix("sha256:")?;
    (value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit()))
        .then(|| value.to_ascii_lowercase())
}

fn installed_core_tag_from_output(output: &[u8]) -> Option<String> {
    String::from_utf8_lossy(output)
        .lines()
        .find_map(|line| line.strip_prefix("Version:").map(str::trim))
        .map(|version| version.split('-').next().unwrap_or(version))
        .filter(|version| valid_release_tag(&format!("v{version}")))
        .map(|version| format!("v{version}"))
}

async fn fetch_github_url(url: &'static str) -> Result<String, String> {
    let mut child = tokio::process::Command::new("/opt/bin/wget")
        .args(["-q", "-T", "10", "-O", "-", url])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .or_else(|_| {
            tokio::process::Command::new("wget")
                .args(["-q", "-T", "10", "-O", "-", url])
                .stdout(Stdio::piped())
                .stderr(Stdio::null())
                .spawn()
        })
        .map_err(|error| format!("Не удалось запустить wget: {error}"))?;
    let mut bytes = Vec::new();
    let stdout = child
        .stdout
        .take()
        .ok_or("Не удалось прочитать ответ GitHub")?;
    let read = tokio::time::timeout(
        std::time::Duration::from_secs(12),
        stdout
            .take((MAX_RELEASE_RESPONSE_BYTES + 1) as u64)
            .read_to_end(&mut bytes),
    )
    .await;
    if read.is_err() || bytes.len() > MAX_RELEASE_RESPONSE_BYTES {
        let _ = child.kill().await;
        return Err("Ответ GitHub Releases превысил лимит или время ожидания".into());
    }
    read.unwrap()
        .map_err(|error| format!("Ошибка чтения GitHub Releases: {error}"))?;
    let status = tokio::time::timeout(std::time::Duration::from_secs(2), child.wait())
        .await
        .map_err(|_| "GitHub Releases не завершил запрос вовремя".to_string())?
        .map_err(|error| format!("Ошибка wget: {error}"))?;
    if !status.success() {
        return Err("GitHub Releases недоступен".into());
    }
    String::from_utf8(bytes).map_err(|_| "Ответ GitHub Releases не UTF-8".into())
}

fn entware_architecture() -> Option<String> {
    let output = std::process::Command::new("/opt/bin/opkg")
        .arg("print-architecture")
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter_map(|line| {
            let mut fields = line.split_whitespace();
            (fields.next()? == "arch")
                .then(|| fields.next().map(str::to_string))
                .flatten()
        })
        .find(|arch| matches!(arch.as_str(), "aarch64-3.10" | "mips-3.4" | "mipsel-3.4"))
}

async fn get_core_releases() -> Response {
    let Some(architecture) = entware_architecture() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Не удалось определить поддерживаемую архитектуру Entware nfqws2",
        );
    };
    match fetch_github_url(NFQWS2_RELEASES_API).await {
        Ok(json) => match eligible_core_releases(&json, &architecture) {
            Ok(releases) => {
                let installed_tag = std::process::Command::new("/opt/bin/opkg")
                    .args(["status", "nfqws2-keenetic"])
                    .output()
                    .ok()
                    .and_then(|output| installed_core_tag_from_output(&output.stdout));
                Json(serde_json::json!({ "releases": releases, "installedTag": installed_tag }))
                    .into_response()
            }
            Err(message) => error(StatusCode::BAD_GATEWAY, message),
        },
        Err(message) => error(StatusCode::BAD_GATEWAY, message),
    }
}

async fn post_core_update_install(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CoreUpdateInstallRequest>,
) -> Response {
    if !valid_release_tag(&request.tag) {
        return error(StatusCode::BAD_REQUEST, "Недопустимый тег релиза nfqws2");
    }
    let _guard = state.update_lock.lock().await;
    let Some(architecture) = entware_architecture() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Обновление ядра доступно только в поддерживаемой Entware",
        );
    };
    let json = match fetch_github_url(NFQWS2_RELEASES_API).await {
        Ok(json) => json,
        Err(message) => return error(StatusCode::BAD_GATEWAY, message),
    };
    let releases = match eligible_core_releases(&json, &architecture) {
        Ok(releases) => releases,
        Err(message) => return error(StatusCode::BAD_GATEWAY, message),
    };
    let Some(selected) = releases.iter().find(|release| release.tag == request.tag) else {
        return error(
            StatusCode::NOT_FOUND,
            "Релиз nfqws2 без SHA-256 пакета для этой архитектуры",
        );
    };
    let current_version = match std::process::Command::new("/opt/bin/opkg")
        .args(["status", "nfqws2-keenetic"])
        .output()
    {
        Ok(output) => installed_core_tag_from_output(&output.stdout),
        Err(_) => None,
    };
    let Some(current_version) = current_version else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Не удалось определить установленную версию пакета nfqws2-keenetic",
        );
    };
    let Some(previous) = releases
        .iter()
        .find(|release| release.tag == current_version)
    else {
        return error(
            StatusCode::CONFLICT,
            "Текущая версия nfqws2 отсутствует в GitHub Releases; безопасный откат недоступен",
        );
    };
    if previous.tag == selected.tag {
        return error(
            StatusCode::CONFLICT,
            "Выбрана уже установленная версия nfqws2",
        );
    }
    let helper = Path::new("/opt/share/nfqws2-ui/current/scripts/update-core-release.sh");
    if !helper.is_file() {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Updater nfqws2 отсутствует; установи актуальную версию панели вручную",
        );
    }
    let operation_dir = state.data_dir.join("update-operations");
    if let Err(io_error) = tokio::fs::create_dir_all(&operation_dir).await {
        return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string());
    }
    let active = operation_dir.join("active");
    if let Ok(active_id) = tokio::fs::read_to_string(&active).await
        && let Ok(content) =
            tokio::fs::read_to_string(update_operation_path(&state, active_id.trim())).await
        && let Some(operation) = parse_update_operation(active_id.trim(), &content)
        && !matches!(
            operation.state.as_str(),
            "success" | "rolled_back" | "failed"
        )
    {
        return error(StatusCode::CONFLICT, "Другое обновление уже выполняется");
    }
    let id = new_update_id();
    let response = UpdateOperation {
        id: id.clone(),
        state: "downloading".into(),
        message: format!("Подготовка обновления nfqws2 до {}", selected.tag),
    };
    if let Err(io_error) = tokio::fs::write(
        update_operation_path(&state, &id),
        format!("downloading\n{}", response.message),
    )
    .await
    {
        return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string());
    }
    if let Err(io_error) = tokio::fs::write(&active, &id).await {
        return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string());
    }
    let child = tokio::process::Command::new("/bin/sh")
        .arg(helper)
        .arg(&selected.tag)
        .arg(&selected.asset)
        .arg(&selected.sha256)
        .arg(&previous.tag)
        .arg(&previous.asset)
        .arg(&previous.sha256)
        .arg(&id)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(false)
        .spawn();
    if let Err(io_error) = child {
        let _ = tokio::fs::remove_file(active).await;
        let _ = tokio::fs::remove_file(update_operation_path(&state, &id)).await;
        return error(
            StatusCode::BAD_GATEWAY,
            format!("Не удалось запустить updater nfqws2: {io_error}"),
        );
    }
    (StatusCode::ACCEPTED, Json(response)).into_response()
}

fn release_is_eligible(releases: &[PanelRelease], tag: &str) -> bool {
    releases.iter().any(|release| release.tag == tag)
}

async fn fetch_github_releases() -> Result<Vec<PanelRelease>, String> {
    let mut child = tokio::process::Command::new("wget")
        .args(["-q", "-T", "10", "-O", "-", GITHUB_RELEASES_API])
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .kill_on_drop(true)
        .spawn()
        .map_err(|error| format!("Не удалось запустить wget для GitHub Releases: {error}"))?;
    let Some(stdout) = child.stdout.take() else {
        let _ = child.kill().await;
        return Err("Не удалось прочитать ответ GitHub Releases".into());
    };
    let mut bytes = Vec::new();
    let read = tokio::time::timeout(
        std::time::Duration::from_secs(12),
        stdout
            .take((MAX_RELEASE_RESPONSE_BYTES + 1) as u64)
            .read_to_end(&mut bytes),
    )
    .await;
    if read.is_err() || bytes.len() > MAX_RELEASE_RESPONSE_BYTES {
        let _ = child.kill().await;
        return Err("Ответ GitHub Releases превысил лимит или время ожидания".into());
    }
    read.unwrap()
        .map_err(|error| format!("Ошибка чтения GitHub Releases: {error}"))?;
    let status = tokio::time::timeout(std::time::Duration::from_secs(2), child.wait())
        .await
        .map_err(|_| "GitHub Releases не завершил запрос вовремя".to_string())?
        .map_err(|error| format!("Ошибка wget: {error}"))?;
    if !status.success() {
        return Err("GitHub Releases недоступен; проверь подключение роутера к GitHub".into());
    }
    let json =
        std::str::from_utf8(&bytes).map_err(|_| "Ответ GitHub Releases не UTF-8".to_string())?;
    eligible_panel_releases(json)
}

fn update_operation_path(state: &AppState, id: &str) -> PathBuf {
    state
        .data_dir
        .join("update-operations")
        .join(format!("{id}.state"))
}

fn parse_update_operation(id: &str, content: &str) -> Option<UpdateOperation> {
    let (state, message) = content.split_once('\n')?;
    let state = match state.trim() {
        "downloading" | "verifying" | "installing" | "health_check" | "success" | "rolled_back"
        | "failed" => state.trim(),
        _ => return None,
    };
    Some(UpdateOperation {
        id: id.into(),
        state: state.into(),
        message: message.trim().into(),
    })
}

fn new_update_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos() as u64;
    let sequence = UPDATE_SEQUENCE.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    format!("{nanos:016x}{sequence:016x}")
}

async fn get_github_releases(State(state): State<Arc<AppState>>) -> Response {
    match fetch_github_releases().await {
        Ok(releases) => {
            let update_status =
                cache_ui_update_status(&state.ui_update_status, &releases, now_unix_seconds());
            Json(serde_json::json!({ "releases": releases, "updateStatus": update_status }))
                .into_response()
        }
        Err(message) => error(StatusCode::BAD_GATEWAY, message),
    }
}

async fn get_ui_update_status(State(state): State<Arc<AppState>>) -> Response {
    match state.ui_update_status.read() {
        Ok(status) => Json(status.clone()).into_response(),
        Err(_) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Статус обновления UI недоступен",
        ),
    }
}

async fn post_update_install(
    State(state): State<Arc<AppState>>,
    Json(request): Json<UpdateInstallRequest>,
) -> Response {
    if !valid_release_tag(&request.tag) {
        return error(StatusCode::BAD_REQUEST, "Недопустимый тег релиза");
    }
    let _guard = state.update_lock.lock().await;
    let releases = match fetch_github_releases().await {
        Ok(releases) => releases,
        Err(message) => return error(StatusCode::BAD_GATEWAY, message),
    };
    if !release_is_eligible(&releases, &request.tag) {
        return error(
            StatusCode::NOT_FOUND,
            "Релиз отсутствует или не содержит ARM64-пакет с SHA-256",
        );
    }
    let helper = Path::new("/opt/share/nfqws2-ui/current/scripts/update-release.sh");
    if !helper.is_file() {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Updater отсутствует; установи актуальную версию панели вручную",
        );
    }
    let operation_dir = state.data_dir.join("update-operations");
    if let Err(io_error) = tokio::fs::create_dir_all(&operation_dir).await {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Не удалось создать каталог статуса обновления: {io_error}"),
        );
    }
    let active = operation_dir.join("active");
    if let Ok(active_id) = tokio::fs::read_to_string(&active).await
        && let Ok(content) =
            tokio::fs::read_to_string(update_operation_path(&state, active_id.trim())).await
        && let Some(operation) = parse_update_operation(active_id.trim(), &content)
        && !matches!(
            operation.state.as_str(),
            "success" | "rolled_back" | "failed"
        )
    {
        return error(
            StatusCode::CONFLICT,
            "Установка другой версии уже выполняется",
        );
    }
    let id = new_update_id();
    let initial = format!("downloading\nПодготовка к загрузке {}", request.tag);
    if let Err(io_error) = tokio::fs::write(update_operation_path(&state, &id), initial).await {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Не удалось сохранить статус обновления: {io_error}"),
        );
    }
    if let Err(io_error) = tokio::fs::write(&active, &id).await {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Не удалось зарегистрировать обновление: {io_error}"),
        );
    }
    let child = tokio::process::Command::new("/bin/sh")
        .arg(helper)
        .arg(&request.tag)
        .arg(&id)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .kill_on_drop(false)
        .spawn();
    if let Err(io_error) = child {
        let _ = tokio::fs::remove_file(active).await;
        let _ = tokio::fs::remove_file(update_operation_path(&state, &id)).await;
        return error(
            StatusCode::BAD_GATEWAY,
            format!("Не удалось запустить безопасный updater: {io_error}"),
        );
    }
    let response = UpdateOperation {
        id,
        state: "downloading".into(),
        message: "Загрузка выбранного релиза началась".into(),
    };
    (StatusCode::ACCEPTED, Json(response)).into_response()
}

async fn get_update_operation(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    if id.len() != 32 || !id.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return error(StatusCode::NOT_FOUND, "Операция обновления не найдена");
    }
    match tokio::fs::read_to_string(update_operation_path(&state, &id)).await {
        Ok(content) => parse_update_operation(&id, &content)
            .map(|operation| Json(operation).into_response())
            .unwrap_or_else(|| {
                error(
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "Повреждён статус обновления",
                )
            }),
        Err(_) => error(StatusCode::NOT_FOUND, "Операция обновления не найдена"),
    }
}

fn module_references(config_text: &str) -> Vec<ModuleReference> {
    let mut references = Vec::new();
    for line in config_text.lines() {
        let line = strip_shell_comment(line);
        // nfqws init scripts commonly store a complete argument list in one
        // quoted shell assignment (for example NFQWS_ARGS="--lua-init=... --blob=...").
        // Inspect the literal arguments inside that outer assignment without
        // evaluating shell expansions or command substitutions.
        let line =
            line.split_once('=')
                .and_then(|(name, value)| {
                    let value = value.trim();
                    let quote = value.chars().next()?;
                    (matches!(quote, '\'' | '"') && value.ends_with(quote) && value.len() >= 2)
                        .then(|| {
                            format!(
                                "{} {}",
                                name.trim(),
                                &value[quote.len_utf8()..value.len() - quote.len_utf8()]
                            )
                        })
                })
                .unwrap_or_else(|| line.to_string());
        let mut word = String::new();
        let mut quote = None;
        let mut words = Vec::new();
        let flush = |word: &mut String, words: &mut Vec<String>| {
            if !word.is_empty() {
                words.push(std::mem::take(word));
            }
        };
        for character in line.chars() {
            match quote {
                Some(delimiter) if character == delimiter => quote = None,
                Some(_) => word.push(character),
                None if character == '\'' || character == '"' => quote = Some(character),
                None if character.is_whitespace() || character == ';' => {
                    flush(&mut word, &mut words)
                }
                None => word.push(character),
            }
        }
        flush(&mut word, &mut words);

        for word in words {
            let Some(flag_start) = word.find("--") else {
                continue;
            };
            let argument = &word[flag_start..];
            let (kind, raw_path) = if let Some(value) = argument.strip_prefix("--lua-init=@") {
                ("lua", value)
            } else if let Some(value) = argument.strip_prefix("--blob=") {
                let Some((_, path)) = value.rsplit_once('@') else {
                    continue;
                };
                ("blob", path)
            } else {
                continue;
            };
            if raw_path.is_empty()
                || references
                    .iter()
                    .any(|reference: &ModuleReference| reference.argument == argument)
            {
                continue;
            }

            let dynamic = raw_path.contains('$') || raw_path.starts_with('~');
            let path = (!dynamic).then(|| PathBuf::from(raw_path));
            let absolute = path.as_ref().is_some_and(|path| path.is_absolute());
            let mut resolved = path.as_ref().filter(|_| absolute).cloned();
            if let Some(candidate) = resolved.as_ref()
                && !candidate.is_file()
                && kind == "lua"
            {
                let mut gzip_name = candidate.as_os_str().to_os_string();
                gzip_name.push(".gz");
                let gzip_path = PathBuf::from(gzip_name);
                if gzip_path.is_file() {
                    resolved = Some(gzip_path);
                } else {
                    resolved = None;
                }
            }
            let exists = resolved
                .as_ref()
                .is_some_and(|candidate| candidate.is_file());
            let readable = resolved
                .as_ref()
                .is_some_and(|candidate| std::fs::File::open(candidate).is_ok());
            references.push(ModuleReference {
                kind,
                argument: argument.to_string(),
                path: path.map(|path| path.display().to_string()),
                resolved_path: resolved.map(|path| path.display().to_string()),
                exists,
                readable,
                resolution: if dynamic {
                    "dynamic"
                } else if absolute {
                    "absolute"
                } else {
                    "relative_unknown"
                },
            });
        }
    }
    references
}

async fn get_installation(State(state): State<Arc<AppState>>) -> Response {
    let discovered = discover_system_paths();
    let config = config_path_for_state(&state);
    let module_references = config
        .as_deref()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map(|text| module_references(&text))
        .unwrap_or_default();
    let config_candidates = known_config_paths()
        .iter()
        .map(|candidate| {
            let active = config
                .as_ref()
                .is_some_and(|path| candidate.canonicalize().ok() == path.canonicalize().ok());
            inspect_path(candidate, active)
        })
        .collect();
    let init_candidates = known_init_scripts();
    let init_script = discovered
        .as_ref()
        .map(|paths| paths.init_script.clone())
        .or_else(|| init_candidates.iter().find(|path| path.is_file()).cloned());
    let init_script_candidates = init_candidates
        .iter()
        .map(|candidate| inspect_path(candidate, false))
        .collect();
    let binary_candidates: Vec<PathBuf> = [
        PathBuf::from("/opt/usr/bin/nfqws2"),
        PathBuf::from("/usr/bin/nfqws2"),
    ]
    .into_iter()
    .collect();
    let binary_candidates = binary_candidates
        .iter()
        .map(|candidate| inspect_path(candidate, false))
        .collect();
    let active_arguments = active_process_arguments();
    let lists = config
        .as_ref()
        .map(|config| {
            LIST_IDS
                .iter()
                .filter_map(|id| {
                    let path = list_file_path(config, id)?;
                    let used = active_arguments
                        .as_deref()
                        .and_then(|args| list_reference_from_argv(args, id))
                        .filter(|reference| reference.is_absolute())
                        .and_then(|reference| reference.canonicalize().ok())
                        .is_some_and(|reference| reference == path);
                    Some(inspect_path(&path, used))
                })
                .collect()
        })
        .unwrap_or_default();
    let platform = discovered.as_ref().map_or("unknown", |paths| {
        if paths.root == Path::new("/opt") {
            "entware"
        } else {
            "openwrt"
        }
    });
    Json(InstallationResponse {
        platform,
        installation_root: discovered
            .as_ref()
            .map(|paths| paths.root.display().to_string()),
        detected_binary: discovered
            .as_ref()
            .map(|paths| inspect_path(&paths.binary, false)),
        network_interfaces: network_interface_names(Path::new("/sys/class/net")),
        config: config.as_deref().map(|path| inspect_path(path, true)),
        config_candidates,
        init_script: init_script.as_deref().map(|path| inspect_path(path, false)),
        init_script_candidates,
        binary_candidates,
        lists,
        module_references,
    })
    .into_response()
}

async fn security_headers(request: Request<Body>, next: Next) -> Response {
    let mut response = next.run(request).await;
    let headers = response.headers_mut();
    headers.insert(
        HeaderName::from_static("x-content-type-options"),
        HeaderValue::from_static("nosniff"),
    );
    headers.insert(
        HeaderName::from_static("x-frame-options"),
        HeaderValue::from_static("DENY"),
    );
    headers.insert(
        HeaderName::from_static("referrer-policy"),
        HeaderValue::from_static("no-referrer"),
    );
    headers.insert(
        HeaderName::from_static("cache-control"),
        HeaderValue::from_static("no-store"),
    );
    headers.insert(
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'"),
    );
    response
}

async fn get_status(State(state): State<Arc<AppState>>) -> Response {
    let (service_state, detail) = service_status().await;
    let core_version = match tokio::process::Command::new("/opt/bin/opkg")
        .args(["status", "nfqws2-keenetic"])
        .output()
        .await
    {
        Ok(output) => installed_core_tag_from_output(&output.stdout),
        Err(_) => None,
    };
    let meminfo = tokio::fs::read_to_string("/proc/meminfo").await.ok();
    let (memory_used_bytes, memory_total_bytes) = meminfo
        .as_deref()
        .and_then(memory_usage_bytes)
        .map_or((None, None), |(used, total)| (Some(used), Some(total)));
    let cpu_percent = cpu_sample(&state).await;
    let system = keenetic_system_stats().await.unwrap_or(SystemInfo {
        memory_total_bytes,
        memory_used_bytes,
        cpu_percent,
        source: "proc",
    });
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let checked_at = now;
    Json(StatusResponse {
        service: ServiceInfo {
            state: service_state,
            detail,
            checked_at,
            version: core_version,
        },
        system,
        panel: PanelInfo {
            version: env!("CARGO_PKG_VERSION"),
            port: state.port,
        },
    })
    .into_response()
}

fn keenetic_system_metrics(data: &serde_json::Value) -> Option<SystemInfo> {
    let total = data.get("memtotal")?.as_u64()?.checked_mul(1024)?;
    let used = data
        .get("memory")?
        .as_str()?
        .split_once('/')?
        .0
        .trim()
        .parse::<u64>()
        .ok()?
        .checked_mul(1024)?;
    let cpu = data.get("cpuload")?.as_f64()?;
    if total == 0 || used > total || !cpu.is_finite() || !(0.0..=100.0).contains(&cpu) {
        return None;
    }
    Some(SystemInfo {
        memory_total_bytes: Some(total),
        memory_used_bytes: Some(used),
        cpu_percent: Some(cpu),
        source: "keenetic-rci",
    })
}

async fn keenetic_system_stats() -> Option<SystemInfo> {
    // Fixed local firmware endpoint, as used by XKeen UI. No caller-supplied URL.
    // wget decodes HTTP framing; limit both time and JSON size, kill on cancellation.
    let request = async {
        let wget = if Path::new("/opt/bin/wget").is_file() {
            "/opt/bin/wget"
        } else {
            "wget"
        };
        let mut command = tokio::process::Command::new(wget);
        command
            .args(["-q", "-T", "2", "-O", "-"])
            .env("no_proxy", "127.0.0.1,localhost")
            .env("NO_PROXY", "127.0.0.1,localhost");
        // Reuse an existing firmware token without exposing it through our API.
        let token = std::env::var("NFQWS2_UI_RCI_TOKEN").ok().or_else(|| {
            let contents = std::fs::read("/opt/etc/xkeen/xkeen.json").ok()?;
            let config: serde_json::Value = serde_json::from_slice(&contents).ok()?;
            config
                .get("xkeen")?
                .get("rci_token")?
                .as_str()
                .map(str::to_owned)
        });
        if let Some(token) = token {
            if token.is_empty()
                || token.len() > 512
                || !token.bytes().all(|byte| (33..=126).contains(&byte))
            {
                return None;
            }
            command.arg(format!("--header=X-Ndma-Tkn: {token}"));
        }
        let mut child = command
            .arg("http://127.0.0.1:79/rci/show/system")
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .ok()?;
        let mut bytes = Vec::new();
        child
            .stdout
            .take()?
            .take(65537)
            .read_to_end(&mut bytes)
            .await
            .ok()?;
        if bytes.len() > 65536 || !child.wait().await.ok()?.success() {
            return None;
        }
        keenetic_system_metrics(&serde_json::from_slice(&bytes).ok()?)
    };
    tokio::time::timeout(std::time::Duration::from_millis(2500), request)
        .await
        .ok()
        .flatten()
}

fn parse_meminfo_kb_key(text: &str, key: &str) -> Option<u64> {
    text.lines().find_map(|line| {
        let (name, rest) = line.split_once(':')?;
        if name != key {
            return None;
        }
        let mut parts = rest.split_whitespace();
        let value = parts.next()?.parse::<u64>().ok()?;
        (parts.next() == Some("kB")).then_some(value)
    })
}

fn memory_usage_bytes(meminfo: &str) -> Option<(u64, u64)> {
    let total_kb = parse_meminfo_kb(meminfo)?;
    let available_kb = parse_meminfo_kb_key(meminfo, "MemAvailable").or_else(|| {
        let free = parse_meminfo_kb_key(meminfo, "MemFree")?;
        let buffers = parse_meminfo_kb_key(meminfo, "Buffers").unwrap_or(0);
        let cached = parse_meminfo_kb_key(meminfo, "Cached").unwrap_or(0);
        let shared = parse_meminfo_kb_key(meminfo, "Shmem").unwrap_or(0);
        Some(
            free.saturating_add(buffers)
                .saturating_add(cached)
                .saturating_sub(shared),
        )
    })?;
    let total_bytes = total_kb.saturating_mul(1024);
    let used_bytes = total_kb.saturating_sub(available_kb).saturating_mul(1024);
    Some((used_bytes, total_bytes))
}

fn parse_cpu_counters(text: &str) -> Option<(u64, u64)> {
    let cpu_line = text.lines().find(|line| line.starts_with("cpu "))?;
    let values: Vec<u64> = cpu_line
        .split_whitespace()
        .skip(1)
        .take(8)
        .map(str::parse)
        .collect::<Result<_, _>>()
        .ok()?;
    if values.len() < 4 {
        return None;
    }
    let total = values
        .iter()
        .fold(0u64, |total, value| total.saturating_add(*value));
    let idle = values[3].saturating_add(*values.get(4).unwrap_or(&0));
    Some((total, idle))
}

fn cpu_percent_between(previous: (u64, u64), current: (u64, u64)) -> Option<f64> {
    let delta_total = current.0.checked_sub(previous.0)?;
    let delta_idle = current.1.checked_sub(previous.1)?;
    (delta_total > 0).then(|| {
        let percent = 100.0 * (delta_total.saturating_sub(delta_idle) as f64 / delta_total as f64);
        ((percent * 10.0).round() / 10.0).clamp(0.0, 100.0)
    })
}

async fn cpu_sample(state: &AppState) -> Option<f64> {
    let text = tokio::fs::read_to_string("/proc/stat").await.ok()?;
    let current = parse_cpu_counters(&text)?;
    let mut previous = state.cpu_previous.lock().await;
    let result = previous.and_then(|old| cpu_percent_between(old, current));
    *previous = Some(current);
    result
}

async fn post_service(Json(request): Json<ServiceRequest>) -> Response {
    let action = match ServiceAction::try_from(request.action.as_str()) {
        Ok(action) => action,
        Err(()) => {
            return error(
                StatusCode::BAD_REQUEST,
                "Допустимые действия: start, stop, restart",
            );
        }
    };
    let result = if let Some(paths) = discover_system_paths() {
        service_action_at(action, &paths).await
    } else {
        Err("Не удалось однозначно обнаружить установленный nfqws2".to_string())
    };
    match result {
        Ok(message) => Json(MessageResponse { message }).into_response(),
        Err(message) => error(StatusCode::BAD_GATEWAY, message),
    }
}

fn config_path(name: &str) -> Option<PathBuf> {
    if name != "nfqws2.conf" {
        return None;
    }
    discover_system_paths().map(|paths| paths.config)
}

fn config_path_for_state(state: &AppState) -> Option<PathBuf> {
    state
        .config_override
        .as_ref()
        .filter(|path| path.is_file())
        .cloned()
        .or_else(|| config_path("nfqws2.conf"))
}

async fn create_ui_change_snapshot(
    state: &AppState,
    note: &str,
    sync_remote: bool,
) -> Result<(), String> {
    let Some(config) = config_path_for_state(state) else {
        return Ok(());
    };
    let _guard = state.config_write_lock.lock().await;
    let previous_id = snapshots::load_store(&state.data_dir)
        .items
        .last()
        .map(|item| item.id.clone());
    let snapshot =
        snapshots::create_snapshot(&state.data_dir, &config, "перед изменением", note, false)?;
    let created = previous_id.as_deref() != Some(snapshot.id.as_str());
    drop(_guard);
    if created && sync_remote {
        kick_remote_snapshot_push(&state.data_dir);
    }
    Ok(())
}

fn list_flag(id: &str) -> Option<&'static str> {
    match id {
        "user.list" => Some("--hostlist"),
        "auto.list" => Some("--hostlist-auto"),
        "exclude.list" => Some("--hostlist-exclude"),
        "ipset.list" => Some("--ipset"),
        "ipset_exclude.list" => Some("--ipset-exclude"),
        _ => None,
    }
}

fn strip_shell_comment(line: &str) -> &str {
    let mut quote = None;
    let mut escaped = false;
    for (index, character) in line.char_indices() {
        if escaped {
            escaped = false;
            continue;
        }
        if character == '\\' && quote != Some('\'') {
            escaped = true;
            continue;
        }
        if let Some(open) = quote {
            if character == open {
                quote = None;
            }
        } else if character == '\'' || character == '"' {
            quote = Some(character);
        } else if character == '#' && (index == 0 || line[..index].ends_with(char::is_whitespace)) {
            return &line[..index];
        }
    }
    line
}

fn configured_list_reference_for_flag(config_text: &str, flag: &str) -> Option<PathBuf> {
    let mut found = None;
    let mut unresolved = false;
    for line in config_text.lines() {
        let line = strip_shell_comment(line);
        let mut start_at = 0;
        while let Some(offset) = line[start_at..].find(flag) {
            let start = start_at + offset;
            let end = start + flag.len();
            let before_is_boundary =
                start == 0 || !line.as_bytes()[start - 1].is_ascii_alphanumeric();
            let after_is_boundary = line
                .as_bytes()
                .get(end)
                .is_none_or(|byte| !byte.is_ascii_alphanumeric() && *byte != b'-');
            if !before_is_boundary || !after_is_boundary {
                start_at = end;
                continue;
            }
            let mut cursor = end;
            while line
                .as_bytes()
                .get(cursor)
                .is_some_and(u8::is_ascii_whitespace)
            {
                cursor += 1;
            }
            if line.as_bytes().get(cursor) == Some(&b'=') {
                cursor += 1;
                while line
                    .as_bytes()
                    .get(cursor)
                    .is_some_and(u8::is_ascii_whitespace)
                {
                    cursor += 1;
                }
            } else if cursor == end {
                start_at = end;
                continue;
            }
            let bytes = line.as_bytes();
            let quote = bytes
                .get(cursor)
                .copied()
                .filter(|byte| *byte == b'\'' || *byte == b'"');
            if quote.is_some() {
                cursor += 1;
            }
            let value_start = cursor;
            while let Some(byte) = bytes.get(cursor) {
                if Some(*byte) == quote
                    || *byte == b'\''
                    || *byte == b'"'
                    || byte.is_ascii_whitespace()
                    || *byte == b'\\'
                {
                    break;
                }
                cursor += 1;
            }
            if cursor > value_start {
                let value = line[value_start..cursor].trim_start_matches('@');
                if value.contains('$') || value.starts_with('~') {
                    unresolved = true;
                } else {
                    let path = PathBuf::from(value);
                    if found.as_ref().is_some_and(|existing| existing != &path) {
                        return None;
                    }
                    found = Some(path);
                }
            }
            start_at = end;
        }
    }
    (!unresolved).then_some(found).flatten()
}

fn configured_list_reference(config_text: &str, id: &str) -> Option<PathBuf> {
    configured_list_reference_for_flag(config_text, list_flag(id)?)
}

fn list_reference_from_argv_for_flag(args: &[String], flag: &str) -> Option<PathBuf> {
    let mut found = None;
    let mut index = 0;
    while index < args.len() {
        let arg = args[index].as_str();
        let value = if let Some(value) = arg
            .strip_prefix(flag)
            .and_then(|value| value.strip_prefix('='))
        {
            Some(value)
        } else if arg == flag {
            args.get(index + 1).map(String::as_str)
        } else {
            None
        };
        if let Some(value) = value {
            let value = value.trim_start_matches('@');
            if value.is_empty() || value.contains(['$', '`']) || value.starts_with('~') {
                return None;
            }
            let path = PathBuf::from(value);
            if found.as_ref().is_some_and(|existing| existing != &path) {
                return None;
            }
            found = Some(path);
        }
        index += 1;
    }
    found
}

fn list_reference_from_argv(args: &[String], id: &str) -> Option<PathBuf> {
    list_reference_from_argv_for_flag(args, list_flag(id)?)
}

fn active_process_arguments() -> Option<Vec<String>> {
    active_process_snapshot().map(|(_, args)| args)
}

fn active_process_snapshot() -> Option<(u32, Vec<String>)> {
    let paths = discover_system_paths()?;
    let entries = std::fs::read_dir("/proc").ok()?;
    let expected = paths.binary.to_string_lossy().into_owned();
    let mut matches = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        if !name
            .to_string_lossy()
            .bytes()
            .all(|byte| byte.is_ascii_digit())
        {
            continue;
        }
        let Ok(pid) = name.to_string_lossy().parse::<u32>() else {
            continue;
        };
        let Ok(bytes) = std::fs::read(entry.path().join("cmdline")) else {
            continue;
        };
        if !process_matches(pid, &bytes, &[expected.as_str()]) {
            continue;
        }
        let args = bytes
            .split(|byte| *byte == 0)
            .filter(|arg| !arg.is_empty())
            .map(|arg| String::from_utf8_lossy(arg).into_owned())
            .collect::<Vec<_>>();
        if !args.is_empty() {
            matches.push((pid, args));
        }
    }
    (matches.len() == 1).then(|| matches.remove(0))
}

fn runtime_profiles(args: &[String]) -> Vec<RuntimeProfile> {
    let mut groups: Vec<Vec<&str>> = vec![Vec::new()];
    for arg in args.iter().skip(1).take(16_384) {
        if arg == "--new" || arg.starts_with("--new=") {
            if groups.len() >= 512 {
                break;
            }
            groups.push(Vec::new());
        } else {
            groups.last_mut().expect("initial group exists").push(arg);
        }
    }
    groups
        .into_iter()
        .filter(|group| {
            group.iter().any(|arg| {
                arg.starts_with("--filter-")
                    || arg.starts_with("--lua-desync=")
                    || arg.starts_with("--payload=")
            })
        })
        .enumerate()
        .map(|(index, group)| {
            let mut profile = RuntimeProfile {
                index: index + 1,
                name: None,
                filters: Vec::new(),
                strategies: Vec::new(),
                payloads: Vec::new(),
            };
            for arg in group {
                if let Some(name) = arg.strip_prefix("--name=") {
                    if name.len() <= 64
                        && !name.is_empty()
                        && name
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || b"-_.".contains(&byte))
                    {
                        profile.name = Some(name.to_owned());
                    }
                } else if arg.starts_with("--filter-") {
                    if profile.filters.len() < 32 && arg.len() <= 256 {
                        profile.filters.push(arg.to_owned());
                    }
                } else if let Some(strategy) = arg.strip_prefix("--lua-desync=") {
                    let name = strategy.split([':', '=']).next().unwrap_or_default();
                    if name.len() <= 64
                        && name
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || b"_-".contains(&byte))
                        && !name.is_empty()
                        && !profile.strategies.iter().any(|existing| existing == name)
                        && profile.strategies.len() < 64
                    {
                        profile.strategies.push(name.to_owned());
                    }
                } else if let Some(payload) = arg.strip_prefix("--payload=")
                    && profile.payloads.len() < 32
                    && payload.len() <= 128
                {
                    profile.payloads.push(payload.to_owned());
                }
            }
            profile
        })
        .collect()
}

async fn get_runtime() -> Response {
    match active_process_snapshot() {
        Some((pid, args)) => Json(RuntimeResponse {
            running: true,
            pid: Some(pid),
            source: "proc-cmdline",
            profiles: runtime_profiles(&args),
            limitations: vec!["Профили сняты с argv работающего процесса; это не доказывает успешную обработку трафика.".into()],
        }).into_response(),
        None => Json(RuntimeResponse {
            running: false,
            pid: None,
            source: "proc-cmdline",
            profiles: Vec::new(),
            limitations: vec!["Не найден единственный однозначно идентифицированный процесс nfqws2.".into()],
        }).into_response(),
    }
}

fn list_file_path(config: &Path, id: &str) -> Option<PathBuf> {
    let config = config.canonicalize().ok()?;
    let root = config.parent()?.canonicalize().ok()?;
    if !LIST_IDS.contains(&id) {
        if !safe_custom_list_id(id) {
            return None;
        }
        let lists_dir = root.join("lists");
        let lists_metadata = std::fs::symlink_metadata(&lists_dir).ok()?;
        if !lists_metadata.is_dir() || lists_metadata.file_type().is_symlink() {
            return None;
        }
        let mut same_name_count = 0;
        for entry in std::fs::read_dir(&lists_dir).ok()? {
            let entry = entry.ok()?;
            if entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.eq_ignore_ascii_case(id))
                && entry.file_type().ok()?.is_file()
            {
                same_name_count += 1;
            }
        }
        if same_name_count != 1 {
            return None;
        }
        let candidate = lists_dir.join(id);
        let metadata = std::fs::symlink_metadata(&candidate).ok()?;
        if !metadata.is_file() || metadata.file_type().is_symlink() {
            return None;
        }
        let path = candidate.canonicalize().ok()?;
        return (path.starts_with(&root) && path.is_file()).then_some(path);
    }
    let config_text = std::fs::read_to_string(&config).ok()?;
    let reference = active_process_arguments()
        .as_deref()
        .and_then(|args| list_reference_from_argv(args, id))
        .or_else(|| configured_list_reference(&config_text, id))
        // Like zemidala's inventory of LISTS_DIR: an existing package list
        // remains editable even when the selected mode does not reference it.
        // Never create it, and keep the same root/symlink restrictions below.
        .unwrap_or_else(|| PathBuf::from("lists").join(id));
    let candidate = if reference.is_absolute() {
        reference
    } else {
        root.join(reference)
    };
    let relative = candidate.strip_prefix(&root).ok()?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return None;
    }
    let mut cursor = root.clone();
    for component in relative.components() {
        cursor.push(component.as_os_str());
        if std::fs::symlink_metadata(&cursor)
            .ok()?
            .file_type()
            .is_symlink()
        {
            return None;
        }
    }
    let path = cursor.canonicalize().ok()?;
    (path.starts_with(&root) && path.is_file()).then_some(path)
}

fn safe_custom_list_id(id: &str) -> bool {
    id.len() <= 128
        && id.ends_with(".list")
        && !id.starts_with('.')
        && !id.contains("..")
        && id
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
        && !LIST_IDS
            .iter()
            .any(|standard| standard.eq_ignore_ascii_case(id))
}

fn list_reference_matches(config: &Path, id: &str, reference: &Path) -> bool {
    let Some(target) = list_file_path(config, id) else {
        return false;
    };
    let Some(root) = config
        .canonicalize()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
    else {
        return false;
    };
    let candidate = if reference.is_absolute() {
        reference.to_path_buf()
    } else {
        root.join(reference)
    };
    candidate
        .canonicalize()
        .ok()
        .is_some_and(|path| path == target)
}

fn custom_list_is_ipset(config: &Path, id: &str) -> bool {
    const IPSET_FLAGS: [&str; 2] = ["--ipset", "--ipset-exclude"];
    if let Some(args) = active_process_arguments()
        && IPSET_FLAGS.iter().any(|flag| {
            list_reference_from_argv_for_flag(&args, flag)
                .is_some_and(|reference| list_reference_matches(config, id, &reference))
        })
    {
        return true;
    }
    let Some(config_text) = std::fs::read_to_string(config).ok() else {
        return false;
    };
    IPSET_FLAGS.iter().any(|flag| {
        configured_list_reference_for_flag(&config_text, flag)
            .is_some_and(|reference| list_reference_matches(config, id, &reference))
    })
}

fn list_ids_for_config(config: &Path) -> Vec<String> {
    let mut ids = LIST_IDS
        .iter()
        .filter(|id| list_file_path(config, id).is_some())
        .map(|id| (*id).to_string())
        .collect::<Vec<_>>();
    let mut seen = ids
        .iter()
        .map(|id| id.to_ascii_lowercase())
        .collect::<HashSet<_>>();
    let root = match config
        .canonicalize()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
    {
        Some(root) => root,
        None => return ids,
    };
    let lists_dir = root.join("lists");
    let Ok(entries) = std::fs::read_dir(lists_dir) else {
        return ids;
    };
    let mut custom = entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let id = entry.file_name().into_string().ok()?;
            if !safe_custom_list_id(&id)
                || !entry.file_type().ok()?.is_file()
                || !seen.insert(id.to_ascii_lowercase())
                || list_file_path(config, &id).is_none()
            {
                return None;
            }
            Some(id)
        })
        .collect::<Vec<_>>();
    custom.sort_by_key(|id| id.to_ascii_lowercase());
    ids.extend(custom);
    ids
}

#[cfg(test)]
fn validate_list_content(id: &str, content: &str) -> Result<(), String> {
    validate_list_content_with_kind(id, content, id.starts_with("ipset"))
}

fn validate_list_content_with_kind(id: &str, content: &str, ip_list: bool) -> Result<(), String> {
    if !LIST_IDS.contains(&id) && !safe_custom_list_id(id) {
        return Err("Неизвестный список".into());
    }
    for (index, line) in content.lines().enumerate() {
        let value = line.trim();
        if value.is_empty() || value.starts_with('#') {
            continue;
        }
        let valid = if ip_list {
            match value.split_once('/') {
                None => value.parse::<IpAddr>().is_ok(),
                Some((address, prefix)) => {
                    let address_type = address.parse::<IpAddr>().ok();
                    let max = if address.contains(':') { 128 } else { 32 };
                    address_type.is_some()
                        && !prefix.is_empty()
                        && prefix.parse::<u8>().is_ok_and(|bits| bits <= max)
                }
            }
        } else {
            let domain = value.strip_prefix("*.").unwrap_or(value);
            let domain = domain.strip_suffix('.').unwrap_or(domain);
            domain.len() <= 253
                && !domain.is_empty()
                && !domain.ends_with('.')
                && domain.split('.').all(|label| {
                    !label.is_empty()
                        && label.len() <= 63
                        && label.as_bytes()[0].is_ascii_alphanumeric()
                        && label.as_bytes()[label.len() - 1].is_ascii_alphanumeric()
                        && label
                            .bytes()
                            .all(|byte| byte.is_ascii_alphanumeric() || byte == b'-')
                })
        };
        if !valid {
            let kind = if ip_list {
                "IP-адрес или CIDR"
            } else {
                "домен"
            };
            return Err(format!("Строка {}: ожидался корректный {kind}", index + 1));
        }
    }
    Ok(())
}

async fn diagnose_site(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SiteDiagnosticRequest>,
) -> Response {
    let host = match site_diagnostics::normalize_host(&request.host) {
        Ok(host) => host,
        Err(message) => return error(StatusCode::BAD_REQUEST, message),
    };
    let Ok(guard) = state.site_diagnostic_lock.clone().try_lock_owned() else {
        return error(
            StatusCode::CONFLICT,
            "Проверка сайта уже выполняется; дождитесь результата",
        );
    };
    let binary = discover_system_paths()
        .map(|paths| paths.binary.to_string_lossy().into_owned())
        .unwrap_or_default();
    // Keep isolation and cleanup alive even if the browser disconnects. Every
    // individual command/probe is bounded; do not cancel while rules are live.
    let mut report = match tokio::spawn(async move {
        let _guard = guard;
        site_diagnostics::diagnose(host, binary).await
    })
    .await
    {
        Ok(report) => report,
        Err(task_error) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Диагностика прервана: {task_error}"),
            );
        }
    };
    let mut matches = Vec::new();
    let mut unreadable = Vec::new();
    if let Some(config) = config_path_for_state(&state) {
        for id in LIST_IDS {
            let Some(path) = list_file_path(&config, id) else {
                continue;
            };
            let mut bytes = Vec::new();
            let read = match tokio::fs::File::open(&path).await {
                Ok(file) => file
                    .take((MAX_LIST_BYTES + 1) as u64)
                    .read_to_end(&mut bytes)
                    .await
                    .is_ok(),
                Err(_) => false,
            };
            if !read || bytes.len() > MAX_LIST_BYTES {
                unreadable.push(id);
                continue;
            }
            for line in String::from_utf8_lossy(&bytes).lines() {
                if site_diagnostics::list_entry_matches(
                    &report.host,
                    &report.addresses,
                    line,
                    id.starts_with("ipset"),
                ) {
                    matches.push(format!(
                        "{id}: {}",
                        line.chars().take(253).collect::<String>()
                    ));
                    break;
                }
            }
        }
        report.steps.push(DiagnosticStep {
            id: "lists".into(),
            status: if unreadable.is_empty() {
                "ok"
            } else {
                "warning"
            },
            detail: format!(
                "{}{}",
                if matches.is_empty() {
                    "Совпадений в прочитанных списках нет".into()
                } else {
                    matches.join("; ")
                },
                if unreadable.is_empty() {
                    String::new()
                } else {
                    format!(". Не удалось проверить: {}", unreadable.join(", "))
                }
            ),
            duration_ms: None,
            http_code: None,
        });
    } else {
        report.steps.push(DiagnosticStep {
            id: "lists".into(),
            status: "unavailable",
            detail: "Конфигурация nfqws2 не обнаружена; списки не проверены".into(),
            duration_ms: None,
            http_code: None,
        });
    }
    Json(report).into_response()
}

async fn check_monitoring_host(host: String) -> MonitoringHostResult {
    use tokio::net::lookup_host;
    use tokio::time::{Duration, Instant, timeout};

    let started = Instant::now();
    let addresses = match timeout(Duration::from_secs(3), lookup_host(format!("{host}:443"))).await
    {
        Ok(Ok(addresses)) => addresses
            .map(|address| address.ip())
            .filter_map(|ip| match ip {
                IpAddr::V4(ip) if monitor_probe_ipv4(ip) => Some(ip),
                _ => None,
            })
            .collect::<Vec<_>>(),
        _ => Vec::new(),
    };
    let dns_ms = started.elapsed().as_millis() as u64;
    let Some(ip) = addresses.first().copied() else {
        let dns_detail = "DNS не вернул публичный IPv4-адрес".to_string();
        let dns = MonitoringStep {
            id: "dns",
            status: "error",
            detail: dns_detail.clone(),
            duration_ms: Some(dns_ms),
            http_code: None,
        };
        let current = MonitoringStep {
            id: "current",
            status: "error",
            detail: dns_detail.clone(),
            duration_ms: Some(dns_ms),
            http_code: None,
        };
        return MonitoringHostResult {
            host,
            summary: dns_detail,
            current,
            steps: vec![dns],
        };
    };
    let dns = MonitoringStep {
        id: "dns",
        status: "ok",
        detail: format!(
            "Публичный IPv4: {}",
            addresses
                .iter()
                .map(ToString::to_string)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        duration_ms: Some(dns_ms),
        http_code: None,
    };
    let probe = site_diagnostics::monitor_probe(&host, ip).await;
    MonitoringHostResult {
        host,
        summary: probe.summary,
        current: MonitoringStep {
            id: "current",
            status: probe.current.status,
            detail: probe.current.detail,
            duration_ms: probe.current.duration_ms,
            http_code: probe.current.http_code,
        },
        steps: std::iter::once(dns)
            .chain(probe.steps.into_iter().map(|step| MonitoringStep {
                id: match step.id.as_str() {
                    "tcp" => "tcp",
                    "tls" => "tls",
                    "http" => "http",
                    "transfer" => "transfer",
                    _ => "current",
                },
                status: step.status,
                detail: step.detail,
                duration_ms: step.duration_ms,
                http_code: step.http_code,
            }))
            .collect(),
    }
}

fn monitor_probe_ipv4(ip: std::net::Ipv4Addr) -> bool {
    let value = u32::from(ip);
    site_diagnostics::public_ip(IpAddr::V4(ip)) || (0xc612_0000..=0xc613_ffff).contains(&value)
}

async fn monitor_sites(Json(request): Json<MonitoringRequest>) -> Response {
    if request.hosts.is_empty() || request.hosts.len() > 20 {
        return error(StatusCode::BAD_REQUEST, "Нужно передать от 1 до 20 доменов");
    }
    let hosts = match request
        .hosts
        .into_iter()
        .map(|host| site_diagnostics::normalize_host(&host))
        .collect::<Result<Vec<_>, _>>()
    {
        Ok(hosts) => hosts,
        Err(message) => return error(StatusCode::BAD_REQUEST, message),
    };
    let mut tasks = tokio::task::JoinSet::new();
    for host in hosts {
        tasks.spawn(check_monitoring_host(host));
    }
    let mut results = Vec::new();
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(report) => results.push(report),
            Err(join_error) => {
                return error(StatusCode::INTERNAL_SERVER_ERROR, join_error.to_string());
            }
        }
    }
    Json(results).into_response()
}

fn monitoring_store_path(data_dir: &Path) -> PathBuf {
    data_dir.join("monitoring.json")
}

async fn load_monitoring_store(data_dir: &Path) -> Result<MonitoringStore, String> {
    let path = monitoring_store_path(data_dir);
    match tokio::fs::read(&path).await {
        Ok(bytes) => serde_json::from_slice(&bytes)
            .map_err(|e| format!("Некорректный файл мониторинга: {e}")),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(MonitoringStore::default()),
        Err(e) => Err(format!("Не удалось прочитать настройки мониторинга: {e}")),
    }
}

async fn save_monitoring_store(data_dir: &Path, value: &MonitoringStore) -> Result<(), String> {
    tokio::fs::create_dir_all(data_dir)
        .await
        .map_err(|e| e.to_string())?;
    let path = monitoring_store_path(data_dir);
    let tmp = path.with_extension("json.tmp");
    let bytes = serde_json::to_vec(value).map_err(|e| e.to_string())?;
    tokio::fs::write(&tmp, bytes)
        .await
        .map_err(|e| e.to_string())?;
    #[cfg(unix)]
    tokio::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(0o600))
        .await
        .map_err(|e| e.to_string())?;
    tokio::fs::rename(&tmp, &path)
        .await
        .map_err(|e| e.to_string())
}

async fn get_monitoring_store(State(state): State<Arc<AppState>>) -> Response {
    match load_monitoring_store(&state.data_dir).await {
        Ok(store) => Json(serde_json::json!({"enabled":store.enabled,"intervalMinutes":store.interval_minutes,"lastAutoCheck":store.last_auto_check,"sites":store.sites})).into_response(),
        Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

async fn put_monitoring_store(
    State(state): State<Arc<AppState>>,
    Json(update): Json<MonitoringStoreUpdate>,
) -> Response {
    let _monitoring_guard = state.monitoring_lock.lock().await;
    if ![30, 60, 120].contains(&update.interval_minutes)
        && !(1..=1440).contains(&update.interval_minutes)
    {
        return error(
            StatusCode::BAD_REQUEST,
            "Интервал должен быть 30, 60, 120 минут или от 1 до 1440 минут",
        );
    }
    if update.sites.len() > 20 {
        return error(StatusCode::BAD_REQUEST, "Можно добавить не более 20 сайтов");
    }
    let mut normalized = Vec::new();
    for mut site in update.sites {
        site.host = match site_diagnostics::normalize_host(&site.host) {
            Ok(host) => host,
            Err(message) => return error(StatusCode::BAD_REQUEST, message),
        };
        site.results.truncate(96);
        if !normalized
            .iter()
            .any(|existing: &MonitoredSite| existing.host == site.host)
        {
            normalized.push(site);
        }
    }
    let mut store = match load_monitoring_store(&state.data_dir).await {
        Ok(value) => value,
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    };
    if let Err(message) =
        create_ui_change_snapshot(&state, "перед изменением мониторинга", true).await
    {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    store.enabled = update.enabled;
    store.interval_minutes = update.interval_minutes;
    store.sites = normalized;
    match save_monitoring_store(&state.data_dir, &store).await {
        Ok(()) => Json(serde_json::json!({"saved":true})).into_response(),
        Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

async fn get_telegram_settings(State(state): State<Arc<AppState>>) -> Response {
    match load_monitoring_store(&state.data_dir).await {
        Ok(store) => Json(serde_json::json!({"tokenSet":!store.telegram.token.is_empty(),"chatId":store.telegram.chat_id,"sendVia":store.telegram.send_via,"interface":store.telegram.interface,"proxy":store.telegram.proxy,"notifications":store.telegram_notifications})).into_response(),
        Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

async fn get_telegram_notifications(State(state): State<Arc<AppState>>) -> Response {
    match load_monitoring_store(&state.data_dir).await {
        Ok(store) => Json(store.telegram_notifications).into_response(),
        Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

async fn put_telegram_settings(
    State(state): State<Arc<AppState>>,
    Json(update): Json<TelegramSettingsUpdate>,
) -> Response {
    let _monitoring_guard = state.monitoring_lock.lock().await;
    if !["direct", "interface", "proxy"].contains(&update.send_via.as_str()) {
        return error(
            StatusCode::BAD_REQUEST,
            "Неизвестный способ отправки Telegram",
        );
    }
    if update.send_via == "proxy"
        && !update.proxy.starts_with("http://")
        && !update.proxy.starts_with("socks5h://")
    {
        return error(
            StatusCode::BAD_REQUEST,
            "Прокси должен начинаться с http:// или socks5h://",
        );
    }
    let mut store = match load_monitoring_store(&state.data_dir).await {
        Ok(value) => value,
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    };
    if let Err(message) =
        create_ui_change_snapshot(&state, "перед изменением уведомлений Telegram", true).await
    {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    if let Some(token) = update.token
        && !token.trim().is_empty()
    {
        store.telegram.token = token.trim().to_owned();
    }
    store.telegram.chat_id = update.chat_id.trim().to_owned();
    store.telegram.send_via = update.send_via;
    store.telegram.interface = update.interface;
    store.telegram.proxy = update.proxy;
    match save_monitoring_store(&state.data_dir, &store).await {
        Ok(()) => {
            Json(serde_json::json!({"saved":true,"tokenSet":!store.telegram.token.is_empty()}))
                .into_response()
        }
        Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

#[derive(Debug)]
struct TelegramApiError {
    message: String,
    retryable: bool,
    error_code: Option<u64>,
}

async fn telegram_interface_resolve() -> Option<std::net::Ipv4Addr> {
    let system_has_public_address = tokio::time::timeout(
        std::time::Duration::from_secs(3),
        tokio::net::lookup_host("api.telegram.org:443"),
    )
    .await
    .ok()
    .and_then(Result::ok)
    .is_some_and(|addresses| {
        addresses
            .map(|address| address.ip())
            .any(|ip| matches!(ip, IpAddr::V4(ip) if site_diagnostics::public_ip(IpAddr::V4(ip))))
    });
    if system_has_public_address {
        return None;
    }
    let addresses = site_diagnostics::public_ipv4_references("api.telegram.org").await;
    addresses.into_iter().find_map(|ip| match ip {
        IpAddr::V4(ip) => Some(ip),
        IpAddr::V6(_) => None,
    })
}

async fn telegram_api(
    settings: &TelegramSettings,
    method: &str,
    form: &[(&str, &str)],
) -> Result<serde_json::Value, TelegramApiError> {
    if settings.token.trim().is_empty() {
        return Err(TelegramApiError {
            message: "Сначала укажите токен бота".into(),
            retryable: false,
            error_code: None,
        });
    }
    let url = format!(
        "https://api.telegram.org/bot{}/{method}",
        settings.token.trim()
    );
    let mut cmd = tokio::process::Command::new(if Path::new("/opt/bin/curl").is_file() {
        "/opt/bin/curl"
    } else {
        "curl"
    });
    cmd.args([
        "--disable",
        "--silent",
        "--show-error",
        "--connect-timeout",
        "6",
        "--max-time",
        "10",
        "--request",
        "POST",
    ]);
    match settings.send_via.as_str() {
        "interface" if !settings.interface.is_empty() => {
            cmd.arg("--interface").arg(&settings.interface);
            if let Some(ip) = telegram_interface_resolve().await {
                cmd.arg("--resolve")
                    .arg(format!("api.telegram.org:443:{ip}"));
            }
        }
        "proxy" if !settings.proxy.is_empty() => {
            cmd.arg("--proxy").arg(&settings.proxy);
        }
        "proxy" => {
            return Err(TelegramApiError {
                message: "Укажите адрес прокси".into(),
                retryable: false,
                error_code: None,
            });
        }
        _ => {}
    }
    for (key, value) in form {
        cmd.arg("--data-urlencode").arg(format!("{key}={value}"));
    }
    cmd.arg("--write-out")
        .arg("\nNFQWS2_TELEGRAM_HTTP_CODE:%{http_code}")
        .arg(url)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let output = cmd.output().await.map_err(|_| TelegramApiError {
        message: "Не удалось запустить curl для Telegram".into(),
        retryable: true,
        error_code: None,
    })?;
    let redact = |value: &str| {
        let value = value.replace(settings.token.trim(), "[токен скрыт]");
        if settings.proxy.is_empty() {
            value
        } else {
            value.replace(&settings.proxy, "[прокси скрыт]")
        }
    };
    if !output.status.success() {
        let code = output
            .status
            .code()
            .map_or_else(|| "неизвестен".to_string(), |code| code.to_string());
        let detail = String::from_utf8_lossy(&output.stderr);
        let detail = redact(detail.trim());
        let detail = detail.chars().take(180).collect::<String>();
        return Err(TelegramApiError {
            message: if detail.is_empty() {
                format!("Не удалось связаться с Telegram (curl, код {code})")
            } else {
                format!("Не удалось связаться с Telegram (curl, код {code}): {detail}")
            },
            retryable: true,
            error_code: None,
        });
    }
    let response = String::from_utf8_lossy(&output.stdout);
    let (body, http_code) = response.rsplit_once("\nNFQWS2_TELEGRAM_HTTP_CODE:").map_or(
        (response.as_ref(), "неизвестен"),
        |(body, code)| (body, code.trim()),
    );
    let value: serde_json::Value = serde_json::from_str(body.trim()).map_err(|_| {
        let preview = redact(body.trim()).chars().take(180).collect::<String>();
        let message = if preview.is_empty() {
            format!("Telegram API не вернул JSON (HTTP {http_code}); проверьте маршрут соединения")
        } else {
            format!("Telegram API не вернул JSON (HTTP {http_code}); ответ: {preview}")
        };
        TelegramApiError {
            message,
            retryable: true,
            error_code: None,
        }
    })?;
    if value.get("ok").and_then(|v| v.as_bool()) != Some(true) {
        let code = value.get("error_code").and_then(|v| v.as_u64());
        return Err(TelegramApiError {
            message: value
                .get("description")
                .and_then(|v| v.as_str())
                .unwrap_or("Telegram отклонил запрос")
                .to_owned(),
            retryable: code == Some(429),
            error_code: code,
        });
    }
    Ok(value)
}

const TELEGRAM_NOTIFICATION_TTL_SECS: u64 = 24 * 60 * 60;
const TELEGRAM_NOTIFICATION_LOG_LIMIT: usize = 30;

fn queue_telegram_notification(store: &mut MonitoringStore, text: String, timestamp: u64) {
    store
        .telegram_notifications
        .queue
        .push(TelegramQueuedMessage {
            timestamp,
            text,
            tries: 0,
            error: None,
        });
}

fn format_telegram_event_time(timestamp: u64) -> String {
    let seconds = i64::try_from(timestamp).unwrap_or(i64::MAX);
    let days = seconds.div_euclid(86_400);
    let day_seconds = seconds.rem_euclid(86_400);
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 }.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    let hour = day_seconds / 3_600;
    let minute = day_seconds % 3_600 / 60;
    format!("{day:02}.{month:02}.{year:04} {hour:02}:{minute:02} UTC")
}

async fn flush_telegram_notifications_unlocked(data_dir: &Path) -> Result<(), String> {
    let mut store = load_monitoring_store(data_dir).await?;
    if store.telegram_notifications.queue.is_empty() {
        return Ok(());
    }
    let now = now_unix_seconds();
    while let Some(message) = store.telegram_notifications.queue.first().cloned() {
        if now.saturating_sub(message.timestamp) > TELEGRAM_NOTIFICATION_TTL_SECS {
            store.telegram_notifications.queue.remove(0);
            store.telegram_notifications.log.insert(
                0,
                TelegramDeliveryLog {
                    timestamp: now,
                    event_timestamp: message.timestamp,
                    text: message.text,
                    ok: false,
                    error: Some(format!(
                        "Не доставлено за сутки: {}",
                        message.error.unwrap_or_else(|| "нет связи".into())
                    )),
                },
            );
            continue;
        }
        if store.telegram.chat_id.trim().is_empty() || store.telegram.token.trim().is_empty() {
            break;
        }
        let late = now.saturating_sub(message.timestamp) > 120;
        let text = if late {
            format!(
                "{}\n(событие в {}, сообщение задержалось: Telegram был недоступен)",
                message.text,
                format_telegram_event_time(message.timestamp)
            )
        } else {
            message.text.clone()
        };
        match telegram_api(
            &store.telegram,
            "sendMessage",
            &[("chat_id", &store.telegram.chat_id), ("text", &text)],
        )
        .await
        {
            Ok(_) => {
                store.telegram_notifications.queue.remove(0);
                store.telegram_notifications.log.insert(
                    0,
                    TelegramDeliveryLog {
                        timestamp: now,
                        event_timestamp: message.timestamp,
                        text: message.text,
                        ok: true,
                        error: None,
                    },
                );
            }
            Err(failure) if failure.retryable => {
                if let Some(queued) = store.telegram_notifications.queue.first_mut() {
                    queued.tries = queued.tries.saturating_add(1);
                    queued.error = Some(failure.message);
                }
                break;
            }
            Err(failure) => {
                store.telegram_notifications.queue.remove(0);
                store.telegram_notifications.log.insert(
                    0,
                    TelegramDeliveryLog {
                        timestamp: now,
                        event_timestamp: message.timestamp,
                        text: message.text,
                        ok: false,
                        error: Some(failure.message),
                    },
                );
            }
        }
    }
    store
        .telegram_notifications
        .log
        .truncate(TELEGRAM_NOTIFICATION_LOG_LIMIT);
    save_monitoring_store(data_dir, &store).await
}

async fn telegram_test(State(state): State<Arc<AppState>>) -> Response {
    let store = match load_monitoring_store(&state.data_dir).await {
        Ok(value) => value,
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    };
    if store.telegram.chat_id.trim().is_empty() {
        return error(StatusCode::BAD_REQUEST, "Сначала укажите ID чата");
    }
    match telegram_api(
        &store.telegram,
        "sendMessage",
        &[
            ("chat_id", &store.telegram.chat_id),
            (
                "text",
                "Проверочное сообщение nfqws2-ui: уведомления Telegram настроены.",
            ),
        ],
    )
    .await
    {
        Ok(_) => Json(serde_json::json!({"sent":true})).into_response(),
        Err(failure) => error(StatusCode::BAD_GATEWAY, failure.message),
    }
}

async fn telegram_find_chats(State(state): State<Arc<AppState>>) -> Response {
    let store = match load_monitoring_store(&state.data_dir).await {
        Ok(value) => value,
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    };
    match telegram_api(&store.telegram, "getUpdates", &[]).await {
        Ok(value) => {
            let mut chats = Vec::<serde_json::Value>::new();
            if let Some(updates) = value.get("result").and_then(|v| v.as_array()) {
                for update in updates {
                    for key in [
                        "message",
                        "edited_message",
                        "channel_post",
                        "edited_channel_post",
                        "my_chat_member",
                        "chat_member",
                        "chat_join_request",
                    ] {
                        if let Some(chat) = update.get(key).and_then(|v| v.get("chat"))
                            && let Some(id) = chat.get("id")
                        {
                            let name = chat
                                .get("title")
                                .or_else(|| chat.get("first_name"))
                                .and_then(|value| value.as_str())
                                .unwrap_or_default();
                            let item = serde_json::json!({"id":id.to_string().trim_matches('"'),"title":name,"type":chat.get("type").cloned().unwrap_or_default(),"username":chat.get("username").cloned().unwrap_or_default()});
                            if !chats.contains(&item) {
                                chats.push(item);
                            }
                        }
                    }
                }
            }
            Json(serde_json::json!({"chats":chats})).into_response()
        }
        Err(failure) => {
            let message = match failure.error_code {
                Some(401 | 404) => "Telegram не принял токен — проверьте, что он скопирован у @BotFather целиком.".to_owned(),
                Some(409) if failure.message.to_ascii_lowercase().contains("webhook") => "У этого бота включён webhook, поэтому Telegram не отдаёт сообщения через getUpdates. Создайте отдельного бота для уведомлений или укажите ID чата вручную.".to_owned(),
                Some(409) => "Этого бота сейчас опрашивает другая программа. Создайте отдельного бота для уведомлений или укажите ID чата вручную.".to_owned(),
                _ => format!("Telegram: {}", failure.message),
            };
            error(StatusCode::BAD_GATEWAY, message)
        }
    }
}

async fn check_saved_sites(
    state: &AppState,
    force: bool,
    only_hosts: Option<Vec<String>>,
) -> Result<Vec<MonitoringHostResult>, String> {
    let _monitoring_guard = state.monitoring_lock.lock().await;
    let data_dir = state.data_dir.as_path();
    let mut store = load_monitoring_store(data_dir).await?;
    if !force && (!store.enabled || store.sites.is_empty()) {
        return Ok(Vec::new());
    }
    let now = now_unix_seconds();
    if !force
        && store
            .last_auto_check
            .is_some_and(|last| now.saturating_sub(last) < store.interval_minutes * 60)
    {
        return Ok(Vec::new());
    }
    let configured_hosts = store
        .sites
        .iter()
        .map(|site| site.host.clone())
        .collect::<Vec<_>>();
    let sites = configured_hosts
        .iter()
        .filter(|host| {
            only_hosts
                .as_ref()
                .is_none_or(|selected| selected.contains(host))
        })
        .cloned()
        .collect::<Vec<_>>();
    if sites.is_empty() {
        return Ok(Vec::new());
    }
    if only_hosts.is_none() {
        store.last_auto_check = Some(now);
        save_monitoring_store(data_dir, &store).await?;
    }
    let mut tasks = tokio::task::JoinSet::new();
    for host in sites {
        tasks.spawn(check_monitoring_host(host));
    }
    let mut reports = Vec::new();
    while let Some(result) = tasks.join_next().await {
        reports.push(result.map_err(|e| e.to_string())?);
    }
    let mut updated = load_monitoring_store(data_dir).await?;
    let mut notification_messages = Vec::new();
    for report in &reports {
        if let Some(site) = updated
            .sites
            .iter_mut()
            .find(|site| site.host == report.host)
        {
            let previous = site.results.first().map(|r| r.status.clone());
            let status = report.current.status.to_owned();
            if previous.as_ref().is_some_and(|old| old != &status)
                && !updated.telegram.chat_id.is_empty()
            {
                let text = if status == "ok" {
                    format!("✅ {} снова открывается: {}", report.host, report.summary)
                } else {
                    format!(
                        "❌ {} перестал открываться: {}",
                        report.host, report.summary
                    )
                };
                notification_messages.push(text);
            }
            site.results.insert(
                0,
                MonitoredResult {
                    checked_at: now,
                    status,
                    summary: report.summary.clone(),
                    latency_ms: report.current.duration_ms,
                    steps: report
                        .steps
                        .iter()
                        .map(|step| MonitoringStepRecord {
                            id: step.id.to_owned(),
                            status: step.status.to_owned(),
                            detail: step.detail.clone(),
                            duration_ms: step.duration_ms,
                            http_code: step.http_code,
                        })
                        .collect(),
                },
            );
            site.results.truncate(96);
        }
    }
    for text in notification_messages {
        queue_telegram_notification(&mut updated, text, now);
    }
    save_monitoring_store(data_dir, &updated).await?;
    flush_telegram_notifications_unlocked(data_dir).await?;
    Ok(reports)
}

async fn run_saved_monitoring(
    State(state): State<Arc<AppState>>,
    request: Option<Json<MonitoringRunRequest>>,
) -> Response {
    let hosts = request.and_then(|Json(request)| request.hosts);
    match check_saved_sites(&state, true, hosts).await {
        Ok(results) => Json(results).into_response(),
        Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

fn start_monitoring_scheduler(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            let flush_result = {
                let _monitoring_guard = state.monitoring_lock.lock().await;
                flush_telegram_notifications_unlocked(&state.data_dir).await
            };
            if let Err(message) = flush_result {
                tracing::warn!("telegram notification queue: {message}");
            }
            if let Err(message) = check_saved_sites(&state, false, None).await {
                tracing::warn!("monitoring scheduler: {message}");
            }
        }
    });
}

async fn trace_site(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SiteTraceRequest>,
) -> Response {
    let host = match site_diagnostics::normalize_host(&request.host) {
        Ok(host) => host,
        Err(message) => return error(StatusCode::BAD_REQUEST, message),
    };
    let proto = request.proto.as_deref().unwrap_or("https");
    if !["http", "https"].contains(&proto) {
        return error(
            StatusCode::BAD_REQUEST,
            "Для трассировки доступны только HTTP и HTTPS; QUIC проверяется отдельно",
        );
    }
    let Ok(guard) = state.site_diagnostic_lock.clone().try_lock_owned() else {
        return error(
            StatusCode::CONFLICT,
            "Проверка сайта или трассировка уже выполняется; повторите позже",
        );
    };
    let operation_dir = state.data_dir.join("site-trace-operations");
    if let Err(io_error) = tokio::fs::create_dir_all(&operation_dir).await {
        return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string());
    }
    prune_finished_json_operations(
        &operation_dir,
        MAX_STRATEGY_PICK_OPERATIONS.saturating_sub(1),
    );
    #[cfg(unix)]
    if let Ok(metadata) = tokio::fs::metadata(&operation_dir).await {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = metadata.permissions();
        permissions.set_mode(0o700);
        if let Err(io_error) = tokio::fs::set_permissions(&operation_dir, permissions).await {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Не удалось защитить каталог операций трассировки: {io_error}"),
            );
        }
    }
    prune_site_trace_operations(&operation_dir).await;
    let id = new_update_id();
    let control = Arc::new(AtomicBool::new(false));
    let operation = SiteTraceOperation {
        id: id.clone(),
        state: "queued".into(),
        stage: "queued".into(),
        message: "Трассировка поставлена в очередь".into(),
        report: None,
        updated_at: now_unix_seconds(),
    };
    if let Err(message) = write_site_trace_operation(&operation_dir, &operation).await {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    state
        .site_trace_controls
        .lock()
        .await
        .insert(id.clone(), control.clone());
    prune_site_trace_operations(&operation_dir).await;

    let task_state = state.clone();
    let task_id = id.clone();
    let task_host = host.clone();
    let task_proto = proto.to_owned();
    tokio::spawn(async move {
        let _guard = guard;
        let operation_dir = task_state.data_dir.join("site-trace-operations");
        let mut running = SiteTraceOperation {
            id: task_id.clone(),
            state: "running".into(),
            stage: "router_probe".into(),
            message: "Выполняется проверка с роутера".into(),
            report: None,
            updated_at: now_unix_seconds(),
        };
        let _ = write_site_trace_operation(&operation_dir, &running).await;
        let partial_operation = Arc::new(std::sync::Mutex::new(running.clone()));
        let progress_operation = partial_operation.clone();
        let progress_directory = operation_dir.clone();
        let progress: site_trace::TraceProgressCallback = Arc::new(move |stage, report| {
            if let Ok(mut operation) = progress_operation.lock() {
                update_site_trace_progress(&mut operation, stage, report);
                let _ = write_site_trace_operation_sync(&progress_directory, &operation);
            }
        });
        let binary = if let Some(paths) = discover_system_paths() {
            paths.binary.to_string_lossy().into_owned()
        } else {
            String::new()
        };
        let args = active_process_snapshot()
            .map(|(_, args)| args)
            .unwrap_or_default();
        let report = (task_state.site_trace_runner)(
            task_host,
            task_proto,
            args,
            binary,
            control.clone(),
            1,
            None,
            Some(progress),
        )
        .await;
        if let Ok(partial) = partial_operation.lock() {
            running = partial.clone();
        }
        let cancelled = control.load(Ordering::Acquire);
        running.state = if cancelled { "cancelled" } else { "complete" }.into();
        running.stage = running.state.clone();
        running.message = if cancelled {
            "Трассировка остановлена; очистка завершена".into()
        } else {
            report.summary.clone()
        };
        running.report = Some(report);
        running.updated_at = now_unix_seconds();
        let _ = write_site_trace_operation(&operation_dir, &running).await;
        task_state.site_trace_controls.lock().await.remove(&task_id);
    });
    (StatusCode::ACCEPTED, Json(operation)).into_response()
}

async fn start_strategy_pick(
    State(state): State<Arc<AppState>>,
    Json(request): Json<StrategyPickRequest>,
) -> Response {
    let repeats = request.repeats.unwrap_or(3);
    if !(2..=5).contains(&repeats) {
        return error(
            StatusCode::BAD_REQUEST,
            "Число повторов должно быть от 2 до 5",
        );
    }
    let host = match site_diagnostics::normalize_host(&request.host) {
        Ok(host) => host,
        Err(message) => return error(StatusCode::BAD_REQUEST, message),
    };
    let protocol = match request.protocol.as_deref().unwrap_or("https") {
        "http" => StrategyProtocol::Http,
        "https" => StrategyProtocol::Https,
        "quic" => {
            return error(
                StatusCode::NOT_IMPLEMENTED,
                "Перебор стратегий QUIC пока не поддерживается: доступна только отдельная проверка QUIC Initial, без проверки через тестовый nfqws2",
            );
        }
        _ => {
            return error(
                StatusCode::BAD_REQUEST,
                "Протокол должен быть http, https или quic",
            );
        }
    };
    let custom_mode = request.mode.as_deref() == Some("custom");
    let mode = match request.mode.as_deref().unwrap_or("quick") {
        "quick" => Some(StrategyMode::Quick),
        "extended" => Some(StrategyMode::Extended),
        "full" => Some(StrategyMode::Full),
        "custom" => Some(StrategyMode::Quick),
        _ => return error(StatusCode::BAD_REQUEST, "Неизвестный режим подбора"),
    };
    if custom_mode && request.custom_steps.is_none() {
        return error(
            StatusCode::BAD_REQUEST,
            "Для режима собственной стратегии укажите от 1 до 10 вариантов --lua-desync=…",
        );
    }
    let Ok(guard) = state.site_diagnostic_lock.clone().try_lock_owned() else {
        return error(
            StatusCode::CONFLICT,
            "Сейчас выполняется другая сетевая диагностика или трассировка",
        );
    };
    let Some((_, active_args)) = active_process_snapshot() else {
        return error(
            StatusCode::CONFLICT,
            "Подбор остановлен: не найден однозначно работающий nfqws2",
        );
    };
    let Some(paths) = discover_system_paths() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Пути установленного nfqws2 не обнаружены однозначно",
        );
    };
    let functions = active_lua_functions(&active_args, &paths.lua_dir);
    if functions.is_empty() {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Не удалось прочитать Lua-функции, подключённые к работающему nfqws2",
        );
    }
    let blobs = active_blob_names(&active_args);
    let configured = strategy_picker::configured_candidates(&active_args, protocol);
    let mut plan = strategy_picker::build_plan(
        protocol,
        mode.unwrap_or(StrategyMode::Quick),
        &configured,
        &functions,
        &blobs,
    );
    if custom_mode {
        let Some(steps) = request.custom_steps.as_deref() else {
            return error(
                StatusCode::BAD_REQUEST,
                "Для собственной стратегии укажите от 1 до 10 вариантов --lua-desync=…",
            );
        };
        if let Err(message) =
            strategy_picker::prepend_custom_candidates(&mut plan, steps, &functions, &blobs)
        {
            return error(StatusCode::UNPROCESSABLE_ENTITY, message);
        }
        strategy_picker::retain_custom_candidates(&mut plan);
    }
    {
        let history = load_confirmed_strategy_candidates(
            &state.data_dir.join("strategy-pick-operations"),
            &host,
            match protocol {
                StrategyProtocol::Http => "http",
                StrategyProtocol::Https => "https",
                StrategyProtocol::Quic => "quic",
            },
            request.include_other_history && !custom_mode,
        )
        .await;
        if !custom_mode {
            strategy_picker::insert_reusable_candidates(&mut plan, history, &functions, &blobs);
        }
    }
    let freeze_candidates = {
        load_confirmed_freeze_candidates(&state.data_dir.join("freeze-diagnostic-operations")).await
    };
    if plan.candidates.is_empty() && freeze_candidates.is_empty() && !request.include_community {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Нет совместимых стратегий для выбранного протокола в активном конфиге и подключённых Lua-файлах",
        );
    }
    let operation_dir = state.data_dir.join("strategy-pick-operations");
    if let Err(io_error) = tokio::fs::create_dir_all(&operation_dir).await {
        return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string());
    }
    prune_finished_json_operations(
        &operation_dir,
        MAX_STRATEGY_PICK_OPERATIONS.saturating_sub(1),
    );
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(io_error) =
            tokio::fs::set_permissions(&operation_dir, std::fs::Permissions::from_mode(0o700)).await
        {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Не удалось защитить каталог автоподбора: {io_error}"),
            );
        }
    }
    let id = new_update_id();
    let control = Arc::new(AtomicBool::new(false));
    let mut operation = StrategyPickOperation {
        id: id.clone(),
        host: host.clone(),
        protocol: match protocol {
            StrategyProtocol::Http => "http",
            StrategyProtocol::Https => "https",
            StrategyProtocol::Quic => "quic",
        }
        .into(),
        state: "queued".into(),
        stage: "queued".into(),
        current: None,
        done: 0,
        total: plan.candidates.len(),
        results: Vec::new(),
        message: "Подбор поставлен в очередь; основной конфиг не изменяется".into(),
        updated_at: now_unix_seconds(),
    };
    if let Err(message) = write_strategy_pick_operation_sync(&operation_dir, &operation) {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    let accepted_operation = operation.clone();
    state
        .strategy_pick_controls
        .lock()
        .await
        .insert(id.clone(), control.clone());
    let task_state = state.clone();
    let task_id = id.clone();
    let task_host = host.clone();
    let task_proto = operation.protocol.clone();
    let task_repeats = repeats;
    let stop_after_first_confirmed = request.stop_after_first_confirmed;
    let refine_lighter = request.refine_lighter;
    let include_community = request.include_community && !custom_mode;
    let community_cache_dir = state.data_dir.join("community-strategy-cache");
    let include_catalog_sources = !custom_mode;
    let binary = paths.binary.to_string_lossy().into_owned();
    let active_args_for_task = active_args.clone();
    let functions_for_task = functions.clone();
    let blobs_for_task = blobs.clone();
    let runner = state.site_trace_runner.clone();
    let probe_cancel = control.clone();
    tokio::spawn(async move {
        let _guard = guard;
        let operation_dir = task_state.data_dir.join("strategy-pick-operations");
        operation.state = "running".into();
        operation.stage = "baseline".into();
        operation.message =
            "Сначала выполняется прямой запрос, затем проверяются выбранные стратегии".into();
        operation.updated_at = now_unix_seconds();
        let shared = Arc::new(std::sync::Mutex::new(operation));
        if let Ok(current) = shared.lock() {
            let _ = write_strategy_pick_operation_sync(&operation_dir, &current);
        }
        let baseline_report = runner(
            task_host.clone(),
            task_proto.clone(),
            Vec::new(),
            binary.clone(),
            control.clone(),
            1,
            None,
            None,
        )
        .await;
        let baseline_ok = baseline_report.without_nfqws2.ok;
        if control.load(Ordering::Acquire) {
            if let Ok(mut current) = shared.lock() {
                current.state = "cancelled".into();
                current.stage = "baseline".into();
                current.message = "Подбор отменён во время прямой проверки".into();
                current.updated_at = now_unix_seconds();
                let _ = write_strategy_pick_operation_sync(&operation_dir, &current);
            }
            task_state
                .strategy_pick_controls
                .lock()
                .await
                .remove(&task_id);
            return;
        };
        if let Ok(mut current) = shared.lock() {
            current.stage = "baseline".into();
            current.message = match baseline_ok {
                Some(true) => format!(
                    "Прямая проверка получила ответ ({}); продолжается проверка выбранных стратегий",
                    baseline_report.without_nfqws2.reason
                ),
                Some(false) => {
                    "Прямая проверка не открыла сайт; собираются совместимые стратегии".into()
                }
                None => format!(
                    "Прямой запрос не дал однозначного результата ({}); продолжается изолированная проверка выбранных стратегий",
                    baseline_report.without_nfqws2.reason
                ),
            };
            current.updated_at = now_unix_seconds();
            let _ = write_strategy_pick_operation_sync(&operation_dir, &current);
        }
        let mut community_notice = String::new();
        if include_community && !control.load(Ordering::SeqCst) {
            match nfqws2_ui::community_strategies::discover_candidates(
                &task_host,
                protocol,
                &functions,
                &blobs,
                &community_cache_dir,
            )
            .await
            {
                Ok(discovery) => {
                    let count = discovery.candidates.len();
                    strategy_picker::insert_reusable_candidates(
                        &mut plan,
                        discovery.candidates,
                        &functions,
                        &blobs,
                    );
                    community_notice = if count == 0 {
                        format!(
                            "Каталог сообщества для провайдера AS{} не содержит совместимых вариантов",
                            discovery.provider_asn
                        )
                    } else {
                        format!(
                            "Добавлено вариантов сообщества: {count} · провайдер AS{}",
                            discovery.provider_asn
                        )
                    };
                }
                Err(message) => {
                    community_notice = format!(
                        "Каталог сообщества недоступен ({message}); продолжается обычный подбор"
                    )
                }
            }
        }
        if include_catalog_sources {
            strategy_picker::insert_reusable_candidates(
                &mut plan,
                freeze_candidates,
                &functions,
                &blobs,
            );
        }
        if let Ok(mut current) = shared.lock() {
            current.total = plan.candidates.len();
            current.stage = "testing".into();
            current.message = if matches!(mode, Some(StrategyMode::Full)) {
                format!(
                    "Приоритетные варианты проверяются по очереди, полный каталог — безопасными пакетами с последовательным подтверждением; отмена доступна в любой момент{}",
                    if community_notice.is_empty() {
                        String::new()
                    } else {
                        format!(" · {community_notice}")
                    }
                )
            } else if community_notice.is_empty() {
                "Проверяются реальные запросы через отдельный тестовый nfqws2".into()
            } else {
                format!("{community_notice}; каждый вариант проверяется на этом роутере")
            };
            current.updated_at = now_unix_seconds();
            let _ = write_strategy_pick_operation_sync(&operation_dir, &current);
        }
        let progress_state = shared.clone();
        let progress_dir = operation_dir.clone();
        let batch_active_args = active_args_for_task.clone();
        let batch_functions = functions_for_task.clone();
        let batch_blobs = blobs_for_task.clone();
        let batch_host = task_host.clone();
        let batch_binary = binary.clone();
        let batch_cancel = probe_cancel.clone();
        let batch_protocol = protocol;
        let probe_candidate = move |candidate: &strategy_picker::StrategyCandidate, repeats| {
            let steps = candidate.steps.clone();
            let candidate_argv = strategy_picker::candidate_argv(
                &active_args_for_task,
                protocol,
                &steps,
                &functions_for_task,
                &blobs_for_task,
            );
            let host = task_host.clone();
            let proto = task_proto.clone();
            let binary = binary.clone();
            let runner = runner.clone();
            let cancel = probe_cancel.clone();
            async move {
                let args = match candidate_argv {
                    Ok(args) => strategy_probe_argv(&binary, &args),
                    Err(message) => {
                        return vec![strategy_picker::StrategyProbeResult {
                            baseline_ok: None,
                            candidate_ok: None,
                            detail: message,
                        }];
                    }
                };
                let report = runner(
                    host,
                    proto,
                    args,
                    binary,
                    cancel,
                    repeats,
                    baseline_ok,
                    None,
                )
                .await;
                let phases = if report.candidate_attempts.is_empty() {
                    vec![report.with_nfqws2.clone()]
                } else {
                    report.candidate_attempts
                };
                phases
                    .into_iter()
                    .map(|phase| strategy_picker::StrategyProbeResult {
                        baseline_ok,
                        candidate_ok: (phase.state == "complete").then_some(phase.ok).flatten(),
                        detail: format!("{}; {}", report.summary, phase.reason),
                    })
                    .collect::<Vec<_>>()
            }
        };
        let batch_runner = move |candidates: Vec<strategy_picker::StrategyCandidate>| {
            let active_args = batch_active_args.clone();
            let functions = batch_functions.clone();
            let blobs = batch_blobs.clone();
            let host = batch_host.clone();
            let binary = batch_binary.clone();
            let cancel = batch_cancel.clone();
            async move {
                site_trace::run_candidate_batch(
                    site_trace::CandidateBatchRuntime {
                        host,
                        protocol: batch_protocol,
                        active_args,
                        binary,
                        functions,
                        blobs,
                        cancel,
                    },
                    &candidates,
                )
                .await
            }
        };
        let update_progress = move |done, total, current: &str| {
            if let Ok(mut operation) = progress_state.lock() {
                operation.current = Some(current.to_owned());
                operation.done = done;
                operation.total = total;
                operation.stage = "testing".into();
                operation.message = format!("Проверяется стратегия: {current}");
                operation.updated_at = now_unix_seconds();
                let _ = write_strategy_pick_operation_sync(&progress_dir, &operation);
            }
        };
        let mut plan_result = if mode == Some(StrategyMode::Full) {
            strategy_picker::run_full_candidates_with_batches(
                plan,
                control.clone(),
                strategy_picker::FullBatchOptions {
                    repeats: task_repeats,
                    stop_after_first_confirmed,
                    max_batch_size: nfqws2_ui::probe_lanes::MAX_BATCH_LANES,
                },
                batch_runner,
                &probe_candidate,
                update_progress,
            )
            .await
        } else {
            strategy_picker::run_candidates_with_compaction(
                plan,
                control.clone(),
                task_repeats,
                stop_after_first_confirmed,
                false,
                &probe_candidate,
                update_progress,
            )
            .await
        };
        let already_confirmed = plan_result.results.iter().any(|result| result.confirmed);
        if !custom_mode
            && !matches!(
                plan_result.state.as_str(),
                "cancelled" | "failed" | "site_unblocked"
            )
            && (plan_result.state == "no_match" || (refine_lighter && already_confirmed))
        {
            let initial_checked = plan_result.checked;
            let initial_total = plan_result.total;
            let progress_state = shared.clone();
            let progress_dir = operation_dir.clone();
            let refinement = strategy_picker::refine_candidates(
                strategy_picker::StrategyRefinementConfig {
                    candidates: &plan_result.refinement_bases,
                    protocol,
                    repeats: task_repeats,
                    already_confirmed,
                    force: refine_lighter,
                    cancel: control.clone(),
                    available_functions: &functions,
                    available_blobs: &blobs,
                },
                &probe_candidate,
                move |done, total, current| {
                    if let Ok(mut operation) = progress_state.lock() {
                        operation.current = Some(current.to_owned());
                        operation.done = initial_checked.saturating_add(done);
                        operation.total = initial_total.saturating_add(total);
                        operation.stage = "refining".into();
                        operation.message = format!("Уточняется вариант: {current}");
                        operation.updated_at = now_unix_seconds();
                        let _ = write_strategy_pick_operation_sync(&progress_dir, &operation);
                    }
                },
            )
            .await;
            plan_result.checked = initial_checked.saturating_add(refinement.checked);
            plan_result.total = initial_total.saturating_add(refinement.total);
            plan_result.results.extend(refinement.results);
            if refinement.state == "cancelled" {
                plan_result.state = "cancelled".into();
            } else if refinement.state == "partial" {
                plan_result.state = "partial".into();
            } else if plan_result.results.iter().any(|result| result.confirmed) {
                plan_result.state = "complete".into();
            }
            if refinement.message != "Уточнение не требуется" {
                plan_result.message = format!("{} · {}", plan_result.message, refinement.message);
            }
        }
        if let Ok(mut operation) = shared.lock() {
            operation.state = plan_result.state;
            operation.stage = operation.state.clone();
            operation.done = plan_result.checked;
            operation.total = plan_result.total;
            operation.results = plan_result.results;
            operation.current = None;
            operation.message = if community_notice.is_empty() {
                plan_result.message
            } else {
                format!("{} · {community_notice}", plan_result.message)
            };
            operation.updated_at = now_unix_seconds();
            let _ = write_strategy_pick_operation_sync(&operation_dir, &operation);
        }
        task_state
            .strategy_pick_controls
            .lock()
            .await
            .remove(&task_id);
    });
    (StatusCode::ACCEPTED, Json(accepted_operation)).into_response()
}

async fn get_strategy_pick_operation(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    let directory = state.data_dir.join("strategy-pick-operations");
    match read_strategy_pick_operation(&directory, &id).await {
        Ok(Some(mut operation)) => {
            if matches!(operation.state.as_str(), "queued" | "running" | "stopping")
                && !state.strategy_pick_controls.lock().await.contains_key(&id)
            {
                operation.state = "failed".into();
                operation.stage = "interrupted".into();
                operation.current = None;
                operation.message =
                    "Фоновая задача прервалась при перезапуске backend; запустите новую проверку"
                        .into();
                operation.updated_at = now_unix_seconds();
                let _ = write_strategy_pick_operation_sync(&directory, &operation);
            }
            Json(operation).into_response()
        }
        Ok(None) => error(StatusCode::NOT_FOUND, "Операция автоподбора не найдена"),
        Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

async fn get_strategy_pick_history(State(state): State<Arc<AppState>>) -> Response {
    let directory = state.data_dir.join("strategy-pick-operations");
    if !directory.exists() {
        return Json(Vec::<StrategyPickOperation>::new()).into_response();
    }
    prune_finished_json_operations(&directory, MAX_STRATEGY_PICK_OPERATIONS.saturating_sub(1));
    let entries = match tokio::fs::read_dir(&directory).await {
        Ok(entries) => entries,
        Err(io_error) => return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string()),
    };
    let mut operations = Vec::new();
    let mut entries = entries;
    loop {
        let entry = match entries.next_entry().await {
            Ok(Some(entry)) => entry,
            Ok(None) => break,
            Err(io_error) => {
                return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string());
            }
        };
        if entry
            .path()
            .extension()
            .and_then(|extension| extension.to_str())
            != Some("json")
        {
            continue;
        }
        let Some(id) = entry
            .path()
            .file_stem()
            .and_then(|stem| stem.to_str())
            .map(str::to_owned)
        else {
            continue;
        };
        let Ok(Some(operation)) = read_strategy_pick_operation(&directory, &id).await else {
            continue;
        };
        if matches!(
            operation.state.as_str(),
            "complete" | "no_match" | "site_unblocked" | "cancelled" | "failed"
        ) {
            operations.push(operation);
        }
    }
    operations.sort_by_key(|operation| std::cmp::Reverse(operation.updated_at));
    operations.truncate(MAX_STRATEGY_PICK_OPERATIONS);
    Json(operations).into_response()
}

async fn stop_strategy_pick_operation(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    let directory = state.data_dir.join("strategy-pick-operations");
    let Some(mut operation) = (match read_strategy_pick_operation(&directory, &id).await {
        Ok(value) => value,
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }) else {
        return error(StatusCode::NOT_FOUND, "Операция автоподбора не найдена");
    };
    if !matches!(operation.state.as_str(), "queued" | "running" | "stopping") {
        return Json(operation).into_response();
    }
    let controls = state.strategy_pick_controls.lock().await;
    let Some(control) = controls.get(&id) else {
        return error(StatusCode::CONFLICT, "Операция уже завершается");
    };
    control.store(true, Ordering::Release);
    operation.state = "stopping".into();
    operation.stage = "cleanup".into();
    operation.message =
        "Остановка запрошена; тестовый процесс и временные правила завершаются".into();
    operation.updated_at = now_unix_seconds();
    drop(controls);
    if let Err(message) = write_strategy_pick_operation_sync(&directory, &operation) {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    (StatusCode::ACCEPTED, Json(operation)).into_response()
}

fn transfer_operation_path(directory: &Path, id: &str) -> PathBuf {
    directory.join(format!("{id}.json"))
}

fn write_transfer_operation(
    directory: &Path,
    operation: &TransferDiagnosticOperation,
) -> Result<(), String> {
    if !valid_site_trace_id(&operation.id) {
        return Err("Недопустимый идентификатор диагностики передачи".into());
    }
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec(operation).map_err(|error| error.to_string())?;
    if bytes.len() > MAX_TRANSFER_DIAGNOSTIC_STATE_BYTES {
        return Err("Состояние диагностики передачи превышает ограничение размера".into());
    }
    let destination = transfer_operation_path(directory, &operation.id);
    let temporary = directory.join(format!("{}.{}.tmp", operation.id, new_update_id()));
    std::fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    #[cfg(windows)]
    if destination.exists() {
        std::fs::remove_file(&destination).map_err(|error| error.to_string())?;
    }
    if let Err(error) = std::fs::rename(&temporary, &destination) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error.to_string());
    }
    Ok(())
}

async fn read_transfer_operation(
    directory: &Path,
    id: &str,
) -> Result<Option<TransferDiagnosticOperation>, String> {
    if !valid_site_trace_id(id) {
        return Ok(None);
    }
    let file = match tokio::fs::File::open(transfer_operation_path(directory, id)).await {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Не удалось прочитать статус диагностики: {error}")),
    };
    let mut bytes = Vec::new();
    file.take((MAX_TRANSFER_DIAGNOSTIC_STATE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|error| error.to_string())?;
    if bytes.len() > MAX_TRANSFER_DIAGNOSTIC_STATE_BYTES {
        return Err("Сохранённое состояние диагностики превышает лимит".into());
    }
    let operation: TransferDiagnosticOperation = serde_json::from_slice(&bytes)
        .map_err(|_| "Повреждено состояние диагностики передачи".to_string())?;
    if operation.id != id
        || !matches!(
            operation.state.as_str(),
            "queued" | "running" | "stopping" | "complete" | "partial" | "cancelled" | "failed"
        )
        || operation.attempt > 2
    {
        return Err("Недопустимое состояние диагностики передачи".into());
    }
    Ok(Some(operation))
}

fn freeze_scan_operation_path(directory: &Path, id: &str) -> PathBuf {
    directory.join(format!("{id}.json"))
}

fn write_freeze_scan_operation(
    directory: &Path,
    operation: &FreezeScanOperation,
) -> Result<(), String> {
    if !valid_site_trace_id(&operation.id) {
        return Err("Недопустимый идентификатор диагностики сетей".into());
    }
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec(operation).map_err(|error| error.to_string())?;
    if bytes.len() > MAX_FREEZE_SCAN_STATE_BYTES {
        return Err("Результат диагностики сетей превышает лимит размера".into());
    }
    let destination = freeze_scan_operation_path(directory, &operation.id);
    let temporary = directory.join(format!("{}.{}.tmp", operation.id, new_update_id()));
    std::fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    #[cfg(windows)]
    if destination.exists() {
        std::fs::remove_file(&destination).map_err(|error| error.to_string())?;
    }
    if let Err(error) = std::fs::rename(&temporary, &destination) {
        let _ = std::fs::remove_file(&temporary);
        return Err(error.to_string());
    }
    Ok(())
}

async fn read_freeze_scan_operation(
    directory: &Path,
    id: &str,
) -> Result<Option<FreezeScanOperation>, String> {
    if !valid_site_trace_id(id) {
        return Ok(None);
    }
    let file = match tokio::fs::File::open(freeze_scan_operation_path(directory, id)).await {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => {
            return Err(format!(
                "Не удалось прочитать статус проверки сетей: {error}"
            ));
        }
    };
    let max_bytes = MAX_FREEZE_SCAN_STATE_BYTES;
    let mut bytes = Vec::new();
    file.take((max_bytes + 1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|error| error.to_string())?;
    if bytes.len() > max_bytes {
        return Err("Сохранённое состояние проверки сетей превышает лимит".into());
    }
    let operation: FreezeScanOperation = serde_json::from_slice(&bytes)
        .map_err(|_| "Повреждено состояние проверки сетей".to_string())?;
    if operation.id != id
        || !matches!(
            operation.state.as_str(),
            "queued" | "running" | "stopping" | "complete" | "partial" | "cancelled" | "failed"
        )
        || operation.done > operation.total
        || operation.total > MAX_FREEZE_SCAN_PROGRESS
        || operation.results.len() > freeze_diagnostic::MAX_TARGETS
    {
        return Err("Недопустимое состояние проверки сетей".into());
    }
    Ok(Some(operation))
}

async fn start_freeze_scan(
    State(state): State<Arc<AppState>>,
    Json(request): Json<FreezeScanRequest>,
) -> Response {
    if let Err(message) = freeze_diagnostic::build_targets(&request.extra_hosts) {
        return error(StatusCode::BAD_REQUEST, message);
    }
    let Some((_, active_args)) = active_process_snapshot() else {
        return error(
            StatusCode::CONFLICT,
            "Проверка сетей остановлена: не найден однозначно работающий nfqws2",
        );
    };
    let Some(paths) = discover_system_paths() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Пути установленного nfqws2 не обнаружены однозначно",
        );
    };
    let functions = active_lua_functions(&active_args, &paths.lua_dir);
    let blobs = active_blob_names(&active_args);
    if functions.is_empty() {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Не удалось прочитать Lua-функции активного nfqws2; перебор остановлен до запуска",
        );
    }
    let Ok(guard) = state.site_diagnostic_lock.clone().try_lock_owned() else {
        return error(
            StatusCode::CONFLICT,
            "Сейчас выполняется другая сетевая диагностика или трассировка",
        );
    };
    let directory = state.data_dir.join("freeze-diagnostic-operations");
    if let Err(io_error) = tokio::fs::create_dir_all(&directory).await {
        return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string());
    }
    prune_finished_json_operations(&directory, 11);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(io_error) =
            tokio::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).await
        {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Не удалось защитить каталог результатов: {io_error}"),
            );
        }
    }
    let id = new_update_id();
    let cancel = Arc::new(AtomicBool::new(false));
    let pause = Arc::new(freeze_diagnostic::FreezeScanPause::new(
        state.site_diagnostic_lock.clone(),
        guard,
        cancel,
    ));
    let operation = FreezeScanOperation {
        id: id.clone(),
        state: "queued".into(),
        stage: "queued".into(),
        current: None,
        done: 0,
        total: freeze_diagnostic::build_targets(&request.extra_hosts)
            .map_or(0, |targets| targets.len()),
        results: Vec::new(),
        message: "Проверка сетей поставлена в очередь".into(),
        updated_at: now_unix_seconds(),
    };
    if let Err(message) = write_freeze_scan_operation(&directory, &operation) {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    state
        .freeze_scan_controls
        .lock()
        .await
        .insert(id.clone(), pause.clone());
    let accepted = operation.clone();
    let task_state = state.clone();
    let task_id = id;
    let binary = paths.binary.to_string_lossy().into_owned();
    tokio::spawn(async move {
        let directory = task_state.data_dir.join("freeze-diagnostic-operations");
        let mut operation = operation;
        operation.state = "running".into();
        operation.stage = "baseline".into();
        operation.message = "Проверяются контрольные сети напрямую и через текущий маршрут".into();
        let shared = Arc::new(std::sync::Mutex::new(operation));
        if let Ok(value) = shared.lock() {
            let _ = write_freeze_scan_operation(&directory, &value);
        }
        let progress = shared.clone();
        let progress_directory = directory.clone();
        let cancel = pause.cancel_handle();
        let scan_args = active_args;
        let scan_functions = functions;
        let scan_blobs = blobs;
        let result = freeze_diagnostic::scan_network_baselines(
            &request.extra_hosts,
            cancel.clone(),
            scan_args,
            binary,
            scan_functions,
            scan_blobs,
            pause.clone(),
            move |done, total, target, completed_results| {
                if let Ok(mut value) = progress.lock() {
                    update_freeze_scan_progress(&mut value, done, total, target, completed_results);
                    let _ = write_freeze_scan_operation(&progress_directory, &value);
                }
            },
        )
        .await;
        if let Ok(mut value) = shared.lock() {
            value.updated_at = now_unix_seconds();
            match result {
                Ok(outcome) if cancel.load(Ordering::Acquire) => {
                    value.state = "cancelled".into();
                    value.stage = "cleanup".into();
                    value.done = outcome.done.min(outcome.total);
                    value.total = outcome.total;
                    value.current = None;
                    value.results = outcome.results;
                    value.message = "Проверка остановлена; завершённые результаты сохранены, временные правила очищены".into();
                }
                Ok(outcome) => {
                    value.state = if outcome.limited {
                        "partial"
                    } else {
                        "complete"
                    }
                    .into();
                    value.stage = value.state.clone();
                    value.done = outcome.done.min(outcome.total);
                    value.total = outcome.total;
                    value.current = None;
                    value.results = outcome.results;
                    let matches = value
                        .results
                        .iter()
                        .filter(|result| result.found_strategy.is_some())
                        .count();
                    let affected = value
                        .results
                        .iter()
                        .filter(|result| result.freeze_detected)
                        .count();
                    value.message = if outcome.limited {
                        format!(
                            "Проверка частично завершена по лимиту времени: подтверждено имён для {matches} из {affected} сетей; непроверенные сети не считаются успешными, конфигурация не менялась"
                        )
                    } else if matches > 0 {
                        format!(
                            "Проверка завершена: подтверждено имён для {matches} из {affected} сетей; конфигурация не менялась"
                        )
                    } else if affected > 0 {
                        format!(
                            "Проверка завершена: обрыв возможен в {affected} сетях, подтверждённая стратегия не найдена; конфигурация не менялась"
                        )
                    } else {
                        "Проверка завершена: воспроизводимого обрыва 16–20 КБ и необходимости подбора не обнаружено".into()
                    };
                }
                Err(message) if cancel.load(Ordering::Acquire) || message.contains("отмен") => {
                    value.state = "cancelled".into();
                    value.stage = "cleanup".into();
                    value.current = None;
                    value.message = "Проверка отменена; временные правила очищены".into();
                }
                Err(message) => {
                    value.state = "failed".into();
                    value.stage = "failed".into();
                    value.current = None;
                    value.message = message;
                }
            }
            let _ = write_freeze_scan_operation(&directory, &value);
        }
        pause.finish_scan().await;
        task_state
            .freeze_scan_controls
            .lock()
            .await
            .remove(&task_id);
    });
    (StatusCode::ACCEPTED, Json(accepted)).into_response()
}

async fn get_freeze_scan(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    let directory = state.data_dir.join("freeze-diagnostic-operations");
    match read_freeze_scan_operation(&directory, &id).await {
        Ok(Some(mut operation)) => {
            if matches!(operation.state.as_str(), "queued" | "running" | "stopping")
                && !state.freeze_scan_controls.lock().await.contains_key(&id)
            {
                operation.state = "failed".into();
                operation.stage = "interrupted".into();
                operation.message =
                    "Проверка прервалась при перезапуске backend; повторите запуск".into();
                operation.updated_at = now_unix_seconds();
                let _ = write_freeze_scan_operation(&directory, &operation);
            }
            Json(operation).into_response()
        }
        Ok(None) => error(StatusCode::NOT_FOUND, "Операция проверки сетей не найдена"),
        Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

async fn cancel_freeze_scan(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    let directory = state.data_dir.join("freeze-diagnostic-operations");
    let Some(mut operation) = (match read_freeze_scan_operation(&directory, &id).await {
        Ok(value) => value,
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }) else {
        return error(StatusCode::NOT_FOUND, "Операция проверки сетей не найдена");
    };
    if !matches!(operation.state.as_str(), "queued" | "running" | "stopping") {
        return error(StatusCode::CONFLICT, "Проверка уже завершена");
    }
    let controls = state.freeze_scan_controls.lock().await;
    let Some(control) = controls.get(&id) else {
        return error(StatusCode::CONFLICT, "Операция уже завершается");
    };
    control.cancel();
    operation.state = "stopping".into();
    operation.stage = "cleanup".into();
    operation.message =
        "Остановка запрошена; текущий запрос завершится с очисткой временных правил".into();
    operation.updated_at = now_unix_seconds();
    drop(controls);
    if let Err(message) = write_freeze_scan_operation(&directory, &operation) {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    (StatusCode::ACCEPTED, Json(operation)).into_response()
}

async fn start_transfer_diagnostic(
    State(state): State<Arc<AppState>>,
    Json(request): Json<TransferDiagnosticRequest>,
) -> Response {
    let host = match site_diagnostics::normalize_host(&request.host) {
        Ok(host) => host,
        Err(message) => return error(StatusCode::BAD_REQUEST, message),
    };
    let Ok(guard) = state.site_diagnostic_lock.clone().try_lock_owned() else {
        return error(
            StatusCode::CONFLICT,
            "Сейчас выполняется другая сетевая диагностика или трассировка",
        );
    };
    let directory = state.data_dir.join("transfer-diagnostic-operations");
    if let Err(io_error) = tokio::fs::create_dir_all(&directory).await {
        return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string());
    }
    prune_finished_json_operations(&directory, 11);
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(io_error) =
            tokio::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).await
        {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Не удалось защитить каталог результатов: {io_error}"),
            );
        }
    }
    let id = new_update_id();
    let control = Arc::new(AtomicBool::new(false));
    let mut operation = TransferDiagnosticOperation {
        id: id.clone(),
        host: host.clone(),
        state: "queued".into(),
        stage: "queued".into(),
        attempt: 0,
        message: "Диагностика поставлена в очередь".into(),
        result: None,
        updated_at: now_unix_seconds(),
    };
    if let Err(message) = write_transfer_operation(&directory, &operation) {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    let accepted = operation.clone();
    state
        .transfer_diagnostic_controls
        .lock()
        .await
        .insert(id.clone(), control.clone());
    let task_state = state.clone();
    let task_id = id;
    tokio::spawn(async move {
        let _guard = guard;
        let directory = task_state.data_dir.join("transfer-diagnostic-operations");
        operation.state = "running".into();
        operation.stage = "direct".into();
        operation.message =
            "Сначала дважды измеряется прямое HTTPS-соединение, затем текущий маршрут".into();
        operation.updated_at = now_unix_seconds();
        let shared = Arc::new(std::sync::Mutex::new(operation));
        if let Ok(current) = shared.lock() {
            let _ = write_transfer_operation(&directory, &current);
        }
        let progress = shared.clone();
        let progress_directory = directory.clone();
        let cancel = control.clone();
        let result = transfer_diagnostic::compare_transfer(host, control, move |stage, attempt| {
            if let Ok(mut operation) = progress.lock() {
                operation.stage = stage.into();
                operation.attempt = attempt;
                operation.message = if stage == "direct" {
                    "Проверяется прямое соединение"
                } else {
                    "Проверяется соединение через текущую конфигурацию"
                }
                .into();
                operation.updated_at = now_unix_seconds();
                let _ = write_transfer_operation(&progress_directory, &operation);
            }
        })
        .await;
        if let Ok(mut operation) = shared.lock() {
            operation.updated_at = now_unix_seconds();
            match result {
                Ok(result) => {
                    operation.state = "complete".into();
                    operation.stage = "complete".into();
                    operation.message = "Измерения завершены; причина сетевого поведения не устанавливается автоматически".into();
                    operation.result = Some(result);
                }
                Err(message) if cancel.load(Ordering::Acquire) || message.contains("отмен") => {
                    operation.state = "cancelled".into();
                    operation.stage = "cleanup".into();
                    operation.message = "Проверка отменена; временные правила очищены".into();
                }
                Err(message) => {
                    operation.state = "failed".into();
                    operation.stage = "failed".into();
                    operation.message = message;
                }
            }
            let _ = write_transfer_operation(&directory, &operation);
        }
        task_state
            .transfer_diagnostic_controls
            .lock()
            .await
            .remove(&task_id);
    });
    (StatusCode::ACCEPTED, Json(accepted)).into_response()
}

async fn get_transfer_diagnostic_operation(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    let directory = state.data_dir.join("transfer-diagnostic-operations");
    match read_transfer_operation(&directory, &id).await {
        Ok(Some(mut operation)) => {
            if matches!(operation.state.as_str(), "queued" | "running" | "stopping")
                && !state
                    .transfer_diagnostic_controls
                    .lock()
                    .await
                    .contains_key(&id)
            {
                operation.state = "failed".into();
                operation.stage = "interrupted".into();
                operation.message =
                    "Фоновая задача прервалась при перезапуске backend; повторите диагностику"
                        .into();
                operation.updated_at = now_unix_seconds();
                let _ = write_transfer_operation(&directory, &operation);
            }
            Json(operation).into_response()
        }
        Ok(None) => error(StatusCode::NOT_FOUND, "Операция диагностики не найдена"),
        Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

async fn cancel_transfer_diagnostic(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    let directory = state.data_dir.join("transfer-diagnostic-operations");
    let Some(mut operation) = (match read_transfer_operation(&directory, &id).await {
        Ok(value) => value,
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }) else {
        return error(StatusCode::NOT_FOUND, "Операция диагностики не найдена");
    };
    if !matches!(operation.state.as_str(), "queued" | "running" | "stopping") {
        return Json(operation).into_response();
    }
    let controls = state.transfer_diagnostic_controls.lock().await;
    let Some(control) = controls.get(&id) else {
        return error(StatusCode::CONFLICT, "Операция уже завершается");
    };
    control.store(true, Ordering::Release);
    operation.state = "stopping".into();
    operation.stage = "cleanup".into();
    operation.message =
        "Остановка запрошена; активный запрос завершится в пределах тайм-аута".into();
    operation.updated_at = now_unix_seconds();
    drop(controls);
    if let Err(message) = write_transfer_operation(&directory, &operation) {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    (StatusCode::ACCEPTED, Json(operation)).into_response()
}

async fn get_site_trace_operation(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    if !valid_site_trace_id(&id) {
        return error(StatusCode::NOT_FOUND, "Операция трассировки не найдена");
    }
    match read_site_trace_operation(&state.data_dir.join("site-trace-operations"), &id).await {
        Ok(Some(operation)) => Json(operation).into_response(),
        Ok(None) => error(StatusCode::NOT_FOUND, "Операция трассировки не найдена"),
        Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

async fn stop_site_trace_operation(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    if !valid_site_trace_id(&id) {
        return error(StatusCode::NOT_FOUND, "Операция трассировки не найдена");
    }
    let operation_dir = state.data_dir.join("site-trace-operations");
    let Some(mut operation) = (match read_site_trace_operation(&operation_dir, &id).await {
        Ok(operation) => operation,
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }) else {
        return error(StatusCode::NOT_FOUND, "Операция трассировки не найдена");
    };
    if !matches!(operation.state.as_str(), "queued" | "running" | "stopping") {
        return (StatusCode::OK, Json(operation)).into_response();
    }
    let controls = state.site_trace_controls.lock().await;
    let Some(control) = controls.get(&id) else {
        return error(StatusCode::CONFLICT, "Операция уже завершается");
    };
    control.store(true, Ordering::Release);
    operation.state = "stopping".into();
    operation.stage = "cleanup".into();
    operation.message = "Остановка запрошена; завершаются дочерние процессы и очистка".into();
    operation.updated_at = now_unix_seconds();
    drop(controls);
    if let Err(message) = write_site_trace_operation(&operation_dir, &operation).await {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    (StatusCode::ACCEPTED, Json(operation)).into_response()
}

async fn preview_config(Json(request): Json<ConfigPreviewRequest>) -> Response {
    if request.content.len() > MAX_CONFIG_BYTES || request.value.len() > 4096 {
        return error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Черновик или значение превышает лимит размера",
        );
    }
    if ![
        "ISP_INTERFACE",
        "NFQWS_EXTRA_ARGS",
        "NFQWS_BASE_ARGS",
        "NFQWS_ARGS",
        "NFQWS_ARGS_QUIC",
        "NFQWS_ARGS_UDP",
        "NFQWS_ARGS_IPSET",
        "NFQWS_ARGS_CUSTOM",
        "NFQUEUE_NUM",
        "POLICY_NAME",
        "IPV6_ENABLED",
        "LOG_LEVEL",
    ]
    .contains(&request.key.as_str())
    {
        return error(
            StatusCode::BAD_REQUEST,
            "Эта переменная доступна только в raw-редакторе",
        );
    }
    let document = match ConfigDocument::parse(&request.content) {
        Ok(document) => document,
        Err(message) => return error(StatusCode::UNPROCESSABLE_ENTITY, message),
    };
    let editable_argument_groups = [
        "NFQWS_BASE_ARGS",
        "NFQWS_ARGS",
        "NFQWS_ARGS_QUIC",
        "NFQWS_ARGS_UDP",
        "NFQWS_ARGS_IPSET",
        "NFQWS_ARGS_CUSTOM",
    ];
    let result = if request.key == "NFQWS_EXTRA_ARGS"
        && ["$MODE_LIST", "$MODE_AUTO", "$MODE_ALL"].contains(&request.value.as_str())
    {
        document.set_mode(&request.value)
    } else if request.key == "NFQWS_EXTRA_ARGS"
        || editable_argument_groups.contains(&request.key.as_str())
    {
        document.set_literal_or_append(&request.key, &request.value)
    } else {
        document.set_literal(&request.key, &request.value)
    };
    match result {
        Ok(content) if content.len() <= MAX_CONFIG_BYTES => Json(serde_json::json!({
            "content": content,
            "changedVariable": request.key,
        }))
        .into_response(),
        Ok(_) => error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Изменённый черновик превышает лимит размера",
        ),
        Err(message) => error(StatusCode::UNPROCESSABLE_ENTITY, message),
    }
}

async fn validate_config(Json(request): Json<ConfigValidateRequest>) -> Response {
    if request.content.len() > MAX_CONFIG_BYTES {
        return error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Конфигурация превышает лимит 512 КБ",
        );
    }
    let (diagnostics, parser_status) = match validate_nfqws2_config(&request.content) {
        Ok(diagnostics) => (
            diagnostics
                .into_iter()
                .map(|diagnostic| ConfigValidationDiagnostic {
                    severity: match diagnostic.severity {
                        DiagnosticSeverity::Error => "error",
                        DiagnosticSeverity::Warning => "warning",
                    },
                    code: diagnostic.code.into(),
                    message: diagnostic.message,
                    line: diagnostic.line,
                })
                .collect::<Vec<_>>(),
            "passed",
        ),
        Err(message) => {
            let line = message
                .strip_prefix("Строка ")
                .and_then(|rest| rest.split(':').next())
                .and_then(|value| value.parse::<usize>().ok());
            (
                vec![ConfigValidationDiagnostic {
                    severity: "error",
                    code: "unsupported_shell_syntax".into(),
                    message,
                    line,
                }],
                "failed",
            )
        }
    };
    let valid = !diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == "error");
    let response = ConfigValidationResponse {
        valid,
        diagnostics,
        checks: ConfigValidationChecks {
            parser: parser_status,
            installed_binary: "not_run",
        },
    };
    Json(response).into_response()
}

fn revision(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

async fn get_config(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ConfigQuery>,
) -> Response {
    if query.file != "nfqws2.conf" {
        return error(
            StatusCode::NOT_FOUND,
            "Разрешённый конфигурационный файл не найден",
        );
    }
    let Some(path) = config_path_for_state(&state) else {
        return error(
            StatusCode::NOT_FOUND,
            "Разрешённый конфигурационный файл не найден",
        );
    };
    match tokio::fs::read(&path).await {
        Ok(bytes) if bytes.len() <= MAX_CONFIG_BYTES => Json(ConfigResponse {
            file: path.display().to_string(),
            content: String::from_utf8_lossy(&bytes).into_owned(),
            revision: revision(&bytes),
        })
        .into_response(),
        Ok(_) => error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Конфигурация превышает лимит 512 КБ",
        ),
        Err(io_error) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Не удалось прочитать конфигурацию: {io_error}"),
        ),
    }
}

async fn get_lists(State(state): State<Arc<AppState>>) -> Response {
    let Some(config) = config_path_for_state(&state) else {
        return error(StatusCode::NOT_FOUND, "Конфигурация nfqws2 не найдена");
    };
    let files = list_ids_for_config(&config)
        .into_iter()
        .filter_map(|id| {
            let path = list_file_path(&config, &id)?;
            Some(ListSummary {
                id,
                path: path.display().to_string(),
                readable: std::fs::File::open(&path).is_ok(),
                writable: std::fs::OpenOptions::new().append(true).open(&path).is_ok(),
            })
        })
        .collect();
    Json(ListsResponse { files }).into_response()
}

async fn get_list(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    let Some(config) = config_path_for_state(&state) else {
        return error(StatusCode::NOT_FOUND, "Конфигурация nfqws2 не найдена");
    };
    let Some(path) = list_file_path(&config, &id) else {
        return error(
            StatusCode::NOT_FOUND,
            "Разрешённый существующий список не найден",
        );
    };
    match tokio::fs::read(&path).await {
        Ok(bytes) if bytes.len() <= MAX_LIST_BYTES => Json(ListResponse {
            id,
            path: path.display().to_string(),
            content: String::from_utf8_lossy(&bytes).into_owned(),
            revision: revision(&bytes),
        })
        .into_response(),
        Ok(_) => error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Список превышает лимит 512 КБ",
        ),
        Err(io_error) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Не удалось прочитать список: {io_error}"),
        ),
    }
}

async fn put_list(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(write): Json<ListWrite>,
) -> Response {
    if write.content.len() > MAX_LIST_BYTES {
        return error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Список превышает лимит 512 КБ",
        );
    }
    if write.content.as_bytes().contains(&0) {
        return error(
            StatusCode::BAD_REQUEST,
            "Список содержит запрещённый NUL-байт",
        );
    }
    let Some(config) = config_path_for_state(&state) else {
        return error(StatusCode::NOT_FOUND, "Конфигурация nfqws2 не найдена");
    };
    let ip_list = id.starts_with("ipset") || custom_list_is_ipset(&config, &id);
    if let Err(message) = validate_list_content_with_kind(&id, &write.content, ip_list) {
        return error(StatusCode::BAD_REQUEST, message);
    }
    let _write_guard = state.config_write_lock.lock().await;
    let Some(path) = list_file_path(&config, &id) else {
        return error(
            StatusCode::NOT_FOUND,
            "Разрешённый существующий список не найден",
        );
    };
    let original = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(io_error) => return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string()),
    };
    if revision(&original) != write.revision {
        return error(
            StatusCode::CONFLICT,
            "Список изменился после загрузки; обновите страницу и проверьте изменения",
        );
    }
    if let Err(message) = backup_and_write_named(
        &state.data_dir,
        &path,
        &format!("list-{id}"),
        MAX_LIST_BYTES,
        write.content.as_bytes(),
        Some(&write.revision),
    )
    .await
    {
        return match message {
            BackupWriteError::Conflict => error(
                StatusCode::CONFLICT,
                "Список изменился во время сохранения; обновите его и проверьте изменения",
            ),
            BackupWriteError::Failed(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
        };
    }
    let saved = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(io_error) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Список записан, но повторное чтение не удалось: {io_error}"),
            );
        }
    };
    if saved != write.content.as_bytes() {
        return error(
            StatusCode::CONFLICT,
            "Список изменился параллельно с записью. Новое содержимое не перезаписывалось; перечитайте файл и проверьте изменения",
        );
    }
    Json(ListResponse {
        id,
        path: path.display().to_string(),
        content: String::from_utf8_lossy(&saved).into_owned(),
        revision: revision(&saved),
    })
    .into_response()
}

async fn put_config(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ConfigQuery>,
    Json(write): Json<ConfigWrite>,
) -> Response {
    if write.content.len() > MAX_CONFIG_BYTES {
        return error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Конфигурация превышает лимит 512 КБ",
        );
    }
    if write.content.as_bytes().contains(&0) {
        return error(
            StatusCode::BAD_REQUEST,
            "Конфигурация содержит запрещённый NUL-байт",
        );
    }
    let _write_guard = state.config_write_lock.lock().await;
    if query.file != "nfqws2.conf" {
        return error(
            StatusCode::NOT_FOUND,
            "Разрешённый конфигурационный файл не найден",
        );
    }
    let Some(path) = config_path_for_state(&state) else {
        return error(
            StatusCode::NOT_FOUND,
            "Разрешённый конфигурационный файл не найден",
        );
    };
    let original = match tokio::fs::read(&path).await {
        Ok(data) => data,
        Err(io_error) => return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string()),
    };
    if revision(&original) != write.revision {
        return error(
            StatusCode::CONFLICT,
            "Файл изменился после загрузки; обновите страницу и проверьте diff",
        );
    }
    if let Err(message) = backup_and_write_named(
        &state.data_dir,
        &path,
        "nfqws2.conf",
        MAX_CONFIG_BYTES,
        write.content.as_bytes(),
        Some(&write.revision),
    )
    .await
    {
        return match message {
            BackupWriteError::Conflict => error(
                StatusCode::CONFLICT,
                "Конфигурация изменилась во время сохранения; обновите её и проверьте diff",
            ),
            BackupWriteError::Failed(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
        };
    }
    let saved = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(io_error) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Конфиг записан, но повторное чтение не удалось: {io_error}"),
            );
        }
    };
    if saved != write.content.as_bytes() {
        return error(
            StatusCode::CONFLICT,
            "Конфигурация изменилась параллельно с записью. Перечитайте файл и проверьте изменения",
        );
    }
    Json(ConfigResponse {
        file: path.display().to_string(),
        content: String::from_utf8_lossy(&saved).into_owned(),
        revision: revision(&saved),
    })
    .into_response()
}

async fn apply_config_transaction<F, Fut>(
    state: &AppState,
    write: ConfigWrite,
    mut restart: F,
) -> Result<ConfigApplyResponse, String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<String, String>>,
{
    if write.content.len() > MAX_CONFIG_BYTES || write.content.as_bytes().contains(&0) {
        return Err("Конфигурация превышает лимит или содержит запрещённый NUL-байт".into());
    }
    let _write_guard = state.config_write_lock.lock().await;
    let path = config_path_for_state(state)
        .ok_or_else(|| "Разрешённый конфигурационный файл не найден".to_string())?;
    let original = tokio::fs::read(&path)
        .await
        .map_err(|error| format!("Не удалось прочитать конфигурацию: {error}"))?;
    if revision(&original) != write.revision {
        return Err("Файл изменился после загрузки; обновите страницу и проверьте diff".into());
    }
    let candidate_revision = revision(write.content.as_bytes());
    if let Err(write_error) = backup_and_write_named(
        &state.data_dir,
        &path,
        "nfqws2.conf",
        MAX_CONFIG_BYTES,
        write.content.as_bytes(),
        Some(&write.revision),
    )
    .await
    {
        if matches!(write_error, BackupWriteError::Conflict) {
            return Err(WRITE_REVISION_CONFLICT.into());
        }
        let BackupWriteError::Failed(write_error) = write_error else {
            unreachable!()
        };
        let rollback =
            restore_config_content(&state.data_dir, &path, &original, &candidate_revision).await;
        return Err(match rollback {
            Ok(()) => format!(
                "Не удалось записать новую конфигурацию: {write_error}. Исходная версия восстановлена."
            ),
            Err(rollback_error) => format!(
                "Не удалось записать новую конфигурацию: {write_error}. Откат тоже не подтверждён: {rollback_error}"
            ),
        });
    }
    let saved = match tokio::fs::read(&path).await {
        Ok(saved) => saved,
        Err(error) => {
            let rollback =
                restore_config_content(&state.data_dir, &path, &original, &candidate_revision)
                    .await;
            return Err(match rollback {
                Ok(()) => format!(
                    "Конфиг записан, но повторное чтение не удалось: {error}. Исходная версия восстановлена."
                ),
                Err(rollback_error) => format!(
                    "Конфиг записан, повторное чтение не удалось: {error}. Откат тоже не подтверждён: {rollback_error}"
                ),
            });
        }
    };
    if saved != write.content.as_bytes() {
        let rollback =
            restore_config_content(&state.data_dir, &path, &original, &candidate_revision).await;
        return Err(match rollback {
            Ok(()) => "Повторное чтение конфига не совпало с отправленным содержимым; исходная версия восстановлена.".into(),
            Err(error) => format!("Повторное чтение конфига не совпало; откат не подтверждён: {error}"),
        });
    }
    match restart().await {
        Ok(message) => Ok(ConfigApplyResponse {
            config: ConfigResponse {
                file: path.display().to_string(),
                revision: revision(&saved),
                content: String::from_utf8_lossy(&saved).into_owned(),
            },
            message,
        }),
        Err(apply_error) => {
            if let Err(rollback_error) =
                restore_config_content(&state.data_dir, &path, &original, &candidate_revision).await
            {
                return Err(format!(
                    "Применение не удалось: {apply_error}. Не удалось восстановить старую конфигурацию: {rollback_error}"
                ));
            }
            match restart().await {
                Ok(restore_message) => Err(format!(
                    "Применение не удалось: {apply_error}. Исходный конфиг восстановлен и служба перезапущена: {restore_message}"
                )),
                Err(restore_error) => Err(format!(
                    "Применение не удалось: {apply_error}. Исходный конфиг восстановлен, но повторный запуск службы не подтверждён: {restore_error}"
                )),
            }
        }
    }
}

async fn delete_custom_list(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(request): Json<ListDelete>,
) -> Response {
    if !safe_custom_list_id(&id) {
        return error(
            StatusCode::FORBIDDEN,
            "Удалять можно только пользовательские .list файлы",
        );
    }
    let Some(config) = config_path_for_state(&state) else {
        return error(StatusCode::NOT_FOUND, "Конфигурация nfqws2 не найдена");
    };
    let _write_guard = state.config_write_lock.lock().await;
    let Some(path) = list_file_path(&config, &id) else {
        return error(StatusCode::NOT_FOUND, "Пользовательский список не найден");
    };
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(io_error) if io_error.kind() == std::io::ErrorKind::NotFound => {
            return error(StatusCode::NOT_FOUND, "Пользовательский список не найден");
        }
        Err(io_error) => return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string()),
    };
    if revision(&bytes) != request.revision {
        return error(
            StatusCode::CONFLICT,
            "Список изменился после загрузки; обновите список перед удалением",
        );
    }
    if let Err(backup_error) = backup_and_write_named(
        &state.data_dir,
        &path,
        &format!("list-{id}"),
        MAX_LIST_BYTES,
        &bytes,
        Some(&request.revision),
    )
    .await
    {
        return match backup_error {
            BackupWriteError::Conflict => error(
                StatusCode::CONFLICT,
                "Список изменился во время удаления; обновите его перед повторной попыткой",
            ),
            BackupWriteError::Failed(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
        };
    }
    match tokio::fs::remove_file(&path).await {
        Ok(()) => Json(MessageResponse {
            message: format!("Пользовательский список {id} удалён"),
        })
        .into_response(),
        Err(io_error) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Не удалось удалить список: {io_error}"),
        ),
    }
}

async fn restore_config_content(
    data_dir: &Path,
    path: &Path,
    original: &[u8],
    expected_current_revision: &str,
) -> Result<(), String> {
    let current = tokio::fs::read(path)
        .await
        .map_err(|error| format!("не удалось прочитать конфиг перед откатом: {error}"))?;
    if current == original {
        return Ok(());
    }
    if revision(&current) != expected_current_revision {
        return Err(
            "конфиг изменился после применения; откат остановлен, чтобы не затереть новые данные"
                .into(),
        );
    }
    backup_and_write_named(
        data_dir,
        path,
        "nfqws2.conf",
        MAX_CONFIG_BYTES,
        original,
        Some(expected_current_revision),
    )
    .await
    .map_err(|error| error.to_string())?;
    let restored = tokio::fs::read(path)
        .await
        .map_err(|error| format!("не удалось повторно прочитать конфигурацию: {error}"))?;
    if restored != original {
        return Err(
            "повторное чтение не совпало с исходным содержимым; резервная копия сохранена".into(),
        );
    }
    Ok(())
}

async fn apply_config(
    State(state): State<Arc<AppState>>,
    Json(write): Json<ConfigWrite>,
) -> Response {
    match apply_config_transaction(&state, write, || async {
        service_action(ServiceAction::Restart).await
    })
    .await
    {
        Ok(result) => Json(result).into_response(),
        Err(message) if message == WRITE_REVISION_CONFLICT => error(
            StatusCode::CONFLICT,
            "Конфигурация изменилась во время сохранения; повторите загрузку",
        ),
        Err(message) => error(StatusCode::BAD_GATEWAY, message),
    }
}

async fn backup_and_write(
    data_dir: &Path,
    config: &Path,
    new_content: &[u8],
) -> Result<(), String> {
    backup_and_write_named(
        data_dir,
        config,
        "nfqws2.conf",
        MAX_CONFIG_BYTES,
        new_content,
        None,
    )
    .await
    .map_err(|error| error.to_string())
}

#[derive(Debug)]
enum BackupWriteError {
    Conflict,
    Failed(String),
}

impl std::fmt::Display for BackupWriteError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Conflict => formatter.write_str("Файл изменился перед атомарной заменой"),
            Self::Failed(message) => formatter.write_str(message),
        }
    }
}

impl From<String> for BackupWriteError {
    fn from(message: String) -> Self {
        Self::Failed(message)
    }
}

impl From<&str> for BackupWriteError {
    fn from(message: &str) -> Self {
        Self::Failed(message.to_owned())
    }
}

async fn backup_and_write_named(
    data_dir: &Path,
    source: &Path,
    backup_name: &str,
    max_bytes: usize,
    new_content: &[u8],
    expected_revision: Option<&str>,
) -> Result<(), BackupWriteError> {
    if new_content.len() > max_bytes {
        return Err(format!("Новый файл превышает лимит {} КБ", max_bytes / 1024).into());
    }
    let old = tokio::fs::read(source)
        .await
        .map_err(|error| error.to_string())?;
    if expected_revision.is_some_and(|expected| revision(&old) != expected) {
        return Err(BackupWriteError::Conflict);
    }
    if old.len() > max_bytes {
        return Err(format!(
            "Текущий файл превышает лимит {} КБ; изменение отменено",
            max_bytes / 1024
        )
        .into());
    }
    let snapshot_config = if backup_name == "nfqws2.conf" {
        Some(source.to_path_buf())
    } else {
        source
            .parent()
            .and_then(Path::parent)
            .map(|root| root.join("nfqws2.conf"))
            .filter(|path| path.is_file())
            .or_else(|| config_path("nfqws2.conf"))
    };
    if let Some(config) = snapshot_config {
        snapshots::create_snapshot(data_dir, &config, "перед изменением", backup_name, false)
            .map_err(|message| {
                format!("Не удалось сохранить полный снимок перед изменением: {message}")
            })?;
        kick_remote_snapshot_push(data_dir);
    }
    if backup_name.is_empty()
        || backup_name.len() > 64
        || !backup_name
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
    {
        return Err("Недопустимое имя резервной копии".into());
    }
    let backup_dir = data_dir.join("backups");
    tokio::fs::create_dir_all(&backup_dir)
        .await
        .map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&backup_dir, std::fs::Permissions::from_mode(0o700))
            .await
            .map_err(|error| error.to_string())?;
    }
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let stamp = format!("{}.{:09}", now.as_secs(), now.subsec_nanos());
    let backup = backup_dir.join(format!("{backup_name}.{stamp}.bak"));
    let mut backup_file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&backup)
        .await
        .map_err(|error| format!("Не удалось создать backup: {error}"))?;
    backup_file
        .write_all(&old)
        .await
        .map_err(|error| format!("Не удалось записать backup: {error}"))?;
    backup_file
        .sync_all()
        .await
        .map_err(|error| format!("Не удалось синхронизировать backup: {error}"))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        tokio::fs::set_permissions(&backup, std::fs::Permissions::from_mode(0o600))
            .await
            .map_err(|error| error.to_string())?;
    }
    #[cfg(unix)]
    let meta = tokio::fs::metadata(source)
        .await
        .map_err(|error| error.to_string())?;
    let tmp = source.with_extension(format!("tmp.{stamp}"));
    let mut tmp_file = tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&tmp)
        .await
        .map_err(|error| error.to_string())?;
    tmp_file
        .write_all(new_content)
        .await
        .map_err(|error| error.to_string())?;
    tmp_file
        .sync_all()
        .await
        .map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        use std::os::unix::ffi::OsStrExt;
        use std::os::unix::fs::{MetadataExt, PermissionsExt};
        let temporary_meta = tokio::fs::metadata(&tmp)
            .await
            .map_err(|error| error.to_string())?;
        if temporary_meta.uid() != meta.uid() || temporary_meta.gid() != meta.gid() {
            let path = std::ffi::CString::new(tmp.as_os_str().as_bytes())
                .map_err(|_| "Путь временного файла содержит NUL-байт".to_string())?;
            // The configured file's owner is preserved across atomic replacement.
            if unsafe { libc::chown(path.as_ptr(), meta.uid(), meta.gid()) } != 0 {
                let error = std::io::Error::last_os_error();
                let _ = tokio::fs::remove_file(&tmp).await;
                return Err(
                    format!("Не удалось сохранить владельца/группу конфигурации: {error}").into(),
                );
            }
        }
        let mode = meta.permissions().mode();
        if let Err(error) =
            tokio::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(mode)).await
        {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(error.to_string().into());
        }
    }
    drop(tmp_file);
    if let Some(expected_revision) = expected_revision {
        let latest = tokio::fs::read(source).await.map_err(|error| {
            format!("Не удалось повторно проверить файл перед заменой: {error}")
        })?;
        if revision(&latest) != expected_revision {
            let _ = tokio::fs::remove_file(&tmp).await;
            return Err(BackupWriteError::Conflict);
        }
    }
    if let Err(error) = tokio::fs::rename(&tmp, source).await {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(
            format!("Не удалось атомарно заменить конфиг; backup сохранён: {error}").into(),
        );
    }
    #[cfg(unix)]
    {
        let directory = tokio::fs::File::open(source.parent().unwrap_or_else(|| Path::new("/")))
            .await
            .map_err(|error| {
                format!("Конфиг заменён, но не удалось открыть каталог для sync: {error}")
            })?;
        directory
            .sync_all()
            .await
            .map_err(|error| format!("Конфиг заменён, но не удалось sync каталог: {error}"))?;
    }
    Ok(())
}

async fn get_backups(State(state): State<Arc<AppState>>) -> Response {
    let directory = state.data_dir.join("backups");
    let mut backups = Vec::new();
    let mut entries = match tokio::fs::read_dir(directory).await {
        Ok(entries) => entries,
        Err(_) => return Json(BackupsResponse { backups }).into_response(),
    };
    while let Ok(Some(entry)) = entries.next_entry().await {
        let file_name = entry.file_name().to_string_lossy().into_owned();
        if file_name.starts_with("nfqws2.conf.") && file_name.ends_with(".bak") {
            let id = file_name.clone();
            let created_at = file_name
                .trim_start_matches("nfqws2.conf.")
                .trim_end_matches(".bak")
                .split('.')
                .next()
                .unwrap_or_default()
                .parse::<u64>()
                .unwrap_or_default();
            backups.push(BackupItem {
                id,
                created_at,
                file: "nfqws2.conf".into(),
            });
        }
    }
    backups.sort_by(|a, b| b.id.cmp(&a.id));
    Json(BackupsResponse { backups }).into_response()
}

async fn get_snapshots(State(state): State<Arc<AppState>>) -> Response {
    Json(snapshots::list_snapshots(&state.data_dir)).into_response()
}

async fn create_snapshot_api(
    State(state): State<Arc<AppState>>,
    request: Option<Json<SnapshotCreateRequest>>,
) -> Response {
    let note = request.map(|Json(value)| value.note).unwrap_or_default();
    let Some(config) = config_path_for_state(&state) else {
        return error(StatusCode::NOT_FOUND, "Конфигурация nfqws2 не найдена");
    };
    let _guard = state.config_write_lock.lock().await;
    match snapshots::create_snapshot(&state.data_dir, &config, "вручную", &note, true) {
        Ok(item) => {
            kick_remote_snapshot_push(&state.data_dir);
            Json(item).into_response()
        }
        Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

async fn update_snapshot_settings(
    State(state): State<Arc<AppState>>,
    Json(settings): Json<snapshots::SnapshotSettings>,
) -> Response {
    if let Err(message) = validate_snapshot_remote_settings(&settings.remote) {
        return error(StatusCode::BAD_REQUEST, message);
    }
    if let Err(message) =
        create_ui_change_snapshot(&state, "перед изменением хранения снимков", false).await
    {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    match snapshots::save_settings(&state.data_dir, &settings) {
        Ok(()) => {
            if let Err(message) = snapshots::apply_retention(&state.data_dir) {
                return error(StatusCode::INTERNAL_SERVER_ERROR, message);
            }
            Json(serde_json::json!({"saved": true})).into_response()
        }
        Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

async fn pin_snapshot(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
    Json(request): Json<SnapshotPinRequest>,
) -> Response {
    if !snapshots::valid_snapshot_id(&id) {
        return error(StatusCode::BAD_REQUEST, "Неверный ID снимка");
    }
    let mut store = snapshots::load_store(&state.data_dir);
    let Some(item) = store.items.iter_mut().find(|item| item.id == id) else {
        return error(StatusCode::NOT_FOUND, "Снимок не найден");
    };
    if request.pinned && !item.local {
        return error(
            StatusCode::CONFLICT,
            "Снимок находится только на NAS; закрепите его на роутере после восстановления",
        );
    }
    item.pinned = request.pinned;
    match snapshots::save_store(&state.data_dir, &store) {
        Ok(()) => Json(serde_json::json!({"saved": true})).into_response(),
        Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

async fn download_snapshot(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    match snapshot_archive_available(&state.data_dir, &id).await {
        Ok(bytes) => snapshot_download_response(bytes, &format!("nfqws2-snapshot-{id}.tar.gz")),
        Err(message) => error(StatusCode::NOT_FOUND, message),
    }
}

async fn snapshot_archive_available(data_dir: &Path, id: &str) -> Result<Vec<u8>, String> {
    if !snapshots::valid_snapshot_id(id) {
        return Err("Неверный ID резервной копии".into());
    }
    if let Ok(bytes) = snapshots::archive_bytes(data_dir, id) {
        return Ok(bytes);
    }
    let store = snapshots::load_store(data_dir);
    if !store.items.iter().any(|item| item.id == id && item.remote) {
        return Err("Архив отсутствует на роутере и не отмечен как копия на NAS".into());
    }
    let path = snapshots::snapshot_dir(data_dir).join(format!(".remote-{id}.tmp"));
    let _ = tokio::fs::remove_file(&path).await;
    remote_call(data_dir, "get", &format!("{id}.tar.gz"), None, Some(&path)).await?;
    #[cfg(unix)]
    tokio::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
        .await
        .map_err(|e| e.to_string())?;
    let bytes = tokio::fs::read(&path)
        .await
        .map_err(|e| format!("Не удалось прочитать архив с NAS: {e}"))?;
    let _ = tokio::fs::remove_file(&path).await;
    if bytes.is_empty() || bytes.len() > snapshots::MAX_ARCHIVE_BYTES {
        return Err("Архив с NAS пустой или превышает 64 МиБ".into());
    }
    Ok(bytes)
}

async fn download_current_snapshot(State(state): State<Arc<AppState>>) -> Response {
    let Some(config) = config_path_for_state(&state) else {
        return error(StatusCode::NOT_FOUND, "Конфигурация nfqws2 не найдена");
    };
    let _guard = state.config_write_lock.lock().await;
    match snapshots::create_snapshot(
        &state.data_dir,
        &config,
        "вручную",
        "архив для скачивания",
        true,
    )
    .and_then(|item| {
        snapshots::archive_bytes(&state.data_dir, &item.id).map(|bytes| (item.id, bytes))
    }) {
        Ok((id, bytes)) => {
            kick_remote_snapshot_push(&state.data_dir);
            snapshot_download_response(bytes, &format!("nfqws2-snapshot-{id}.tar.gz"))
        }
        Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

fn snapshot_download_response(bytes: Vec<u8>, filename: &str) -> Response {
    let mut response = Response::new(Body::from(bytes));
    *response.status_mut() = StatusCode::OK;
    response.headers_mut().insert(
        axum::http::header::CONTENT_TYPE,
        HeaderValue::from_static("application/gzip"),
    );
    let disposition = format!("attachment; filename=\"{filename}\"");
    match HeaderValue::from_str(&disposition) {
        Ok(value) => {
            response
                .headers_mut()
                .insert(axum::http::header::CONTENT_DISPOSITION, value);
        }
        Err(_) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Не удалось сформировать имя архива",
            );
        }
    }
    response.headers_mut().insert(
        axum::http::header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    );
    response
}

async fn restore_snapshot_api(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    let Some(config) = config_path_for_state(&state) else {
        return error(StatusCode::NOT_FOUND, "Конфигурация nfqws2 не найдена");
    };
    let _guard = state.config_write_lock.lock().await;
    let archive = match snapshot_archive_available(&state.data_dir, &id).await {
        Ok(bytes) => bytes,
        Err(message) => return error(StatusCode::NOT_FOUND, message),
    };
    if let Err(message) = snapshots::validate_archive(&archive) {
        return error(StatusCode::BAD_REQUEST, message);
    }
    if let Err(message) = snapshots::create_snapshot(
        &state.data_dir,
        &config,
        "перед восстановлением",
        &format!("перед снимком {id}"),
        true,
    ) {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Не удалось сохранить текущее состояние: {message}"),
        );
    }
    match snapshots::validate_and_restore(&state.data_dir, &config, &archive) {
        Ok(changed) => {
            kick_remote_snapshot_push(&state.data_dir);
            Json(serde_json::json!({"changed": changed, "message": format!("Восстановлено файлов: {}. Перезапустите nfqws2, чтобы применить конфигурацию.", changed.len())})).into_response()
        }
        Err(message) => error(StatusCode::BAD_REQUEST, message),
    }
}

async fn restore_uploaded_snapshot(State(state): State<Arc<AppState>>, body: Bytes) -> Response {
    let Some(config) = config_path_for_state(&state) else {
        return error(StatusCode::NOT_FOUND, "Конфигурация nfqws2 не найдена");
    };
    let _guard = state.config_write_lock.lock().await;
    if let Err(message) = snapshots::validate_archive(&body) {
        return error(StatusCode::BAD_REQUEST, message);
    }
    if let Err(message) = snapshots::create_snapshot(
        &state.data_dir,
        &config,
        "перед восстановлением",
        "перед архивом с устройства",
        true,
    ) {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Не удалось сохранить текущее состояние: {message}"),
        );
    }
    match snapshots::validate_and_restore(&state.data_dir, &config, &body) {
        Ok(changed) => {
            kick_remote_snapshot_push(&state.data_dir);
            Json(serde_json::json!({"changed": changed, "message": format!("Восстановлено файлов: {}. Перезапустите nfqws2, чтобы применить конфигурацию.", changed.len())})).into_response()
        }
        Err(message) => error(StatusCode::BAD_REQUEST, message),
    }
}

async fn create_snapshot_key(State(state): State<Arc<AppState>>) -> Response {
    match create_router_snapshot_key(&state.data_dir).await {
        Ok(public_key) => Json(serde_json::json!({"publicKey": public_key})).into_response(),
        Err(message) => error(StatusCode::BAD_GATEWAY, message),
    }
}

async fn test_snapshot_remote(State(state): State<Arc<AppState>>) -> Response {
    match remote_call(&state.data_dir, "ping", "", None, None).await {
        Ok(output) if output.trim() == "ok" => {
            match remote_call(&state.data_dir, "list", "", None, None).await {
                Ok(output) => {
                    record_snapshot_remote_success(&state.data_dir).await;
                    Json(serde_json::json!({"ok": true, "count": output.lines().filter(|line| !line.trim().is_empty()).count()})).into_response()
                }
                Err(message) => error(StatusCode::BAD_GATEWAY, message),
            }
        }
        Ok(_) => error(
            StatusCode::BAD_GATEWAY,
            "NAS ответил неожиданным результатом",
        ),
        Err(message) => error(StatusCode::BAD_GATEWAY, message),
    }
}

async fn record_snapshot_remote_success(data_dir: &Path) {
    let mut state: serde_json::Value = tokio::fs::read(snapshots::remote_state_path(data_dir))
        .await
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    state["ok"] = serde_json::json!(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );
    state["error"] = serde_json::Value::Null;
    state["errorAt"] = serde_json::Value::Null;
    let _ = tokio::fs::write(
        snapshots::remote_state_path(data_dir),
        serde_json::to_vec(&state).unwrap_or_default(),
    )
    .await;
}

async fn push_snapshot_remote(State(state): State<Arc<AppState>>) -> Response {
    match push_remote_snapshots(&state.data_dir).await {
        Ok(sent) => Json(serde_json::json!({"sent": sent})).into_response(),
        Err(message) => error(StatusCode::BAD_GATEWAY, message),
    }
}

async fn get_ui_appearance(State(state): State<Arc<AppState>>) -> Response {
    let path = state.data_dir.join("appearance.json");
    match tokio::fs::read(path).await {
        Ok(bytes) => match serde_json::from_slice::<UiAppearanceSettings>(&bytes) {
            Ok(value) => Json(Some(value)).into_response(),
            Err(_) => Json(Option::<UiAppearanceSettings>::None).into_response(),
        },
        Err(io_error) if io_error.kind() == std::io::ErrorKind::NotFound => {
            Json(Option::<UiAppearanceSettings>::None).into_response()
        }
        Err(io_error) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Не удалось прочитать оформление UI: {io_error}"),
        ),
    }
}

async fn put_ui_appearance(
    State(state): State<Arc<AppState>>,
    Json(settings): Json<UiAppearanceSettings>,
) -> Response {
    if !matches!(settings.theme.as_str(), "dark" | "light" | "neon")
        || !matches!(settings.layout.as_str(), "standard" | "compact")
    {
        return error(StatusCode::BAD_REQUEST, "Недопустимые параметры оформления");
    }
    if let Err(io_error) = tokio::fs::create_dir_all(&state.data_dir).await {
        return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string());
    }
    let path = state.data_dir.join("appearance.json");
    if path.is_file()
        && let Err(message) =
            create_ui_change_snapshot(&state, "перед изменением оформления UI", true).await
    {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    let temp = path.with_extension("json.tmp");
    let bytes = match serde_json::to_vec(&settings) {
        Ok(bytes) => bytes,
        Err(serialize_error) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                serialize_error.to_string(),
            );
        }
    };
    if let Err(io_error) = tokio::fs::write(&temp, bytes).await {
        return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string());
    }
    #[cfg(unix)]
    if let Err(io_error) =
        tokio::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o600)).await
    {
        return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string());
    }
    match tokio::fs::rename(&temp, &path).await {
        Ok(()) => Json(serde_json::json!({"saved": true})).into_response(),
        Err(io_error) => error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string()),
    }
}

fn validate_snapshot_remote_settings(
    remote: &snapshots::SnapshotRemoteSettings,
) -> Result<(), String> {
    if remote.host.len() > 253
        || !remote
            .host
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'-' | b':'))
    {
        return Err("Адрес NAS должен содержать только имя хоста или IP-адрес".into());
    }
    if remote.user.len() > 64
        || !remote
            .user
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'))
    {
        return Err("Имя пользователя NAS содержит недопустимые символы".into());
    }
    if !remote.dir.is_empty()
        && (!remote.dir.starts_with('/')
            || remote.dir.len() > 512
            || remote.dir.split('/').any(|part| part == "..")
            || !remote
                .dir
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'/' | b'.' | b'_' | b'-')))
    {
        return Err("Папка NAS должна быть абсолютным путём без .. и специальных символов".into());
    }
    if remote.enabled && (remote.host.is_empty() || remote.user.is_empty() || remote.dir.is_empty())
    {
        return Err("Для отправки на NAS заполните адрес, пользователя и папку".into());
    }
    Ok(())
}

fn snapshot_ssh_tool() -> Option<&'static str> {
    ["dbclient", "ssh"].into_iter().find(|tool| {
        std::process::Command::new("sh")
            .args(["-c", &format!("command -v {tool}")])
            .output()
            .is_ok_and(|result| result.status.success())
    })
}

fn snapshot_ssh_is_dropbear(tool: &str) -> bool {
    if tool == "dbclient" {
        return true;
    }
    std::process::Command::new("sh")
        .args(["-c", "command -v ssh"])
        .output()
        .ok()
        .filter(|output| output.status.success())
        .and_then(|output| std::fs::read_link(String::from_utf8_lossy(&output.stdout).trim()).ok())
        .is_some_and(|target| {
            target
                .to_string_lossy()
                .to_ascii_lowercase()
                .contains("dropbear")
        })
}

fn normalize_snapshot_public_key(raw: &str) -> Option<String> {
    let mut fields = raw.split_whitespace();
    let kind = fields.next()?;
    let encoded = fields.next()?;
    if !matches!(kind, "ssh-ed25519" | "ssh-rsa")
        || encoded.is_empty()
        || !encoded
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'+' | b'/' | b'='))
    {
        return None;
    }
    Some(format!("{kind} {encoded} nfqws2-ui"))
}

async fn create_router_snapshot_key(data_dir: &Path) -> Result<String, String> {
    let tool = snapshot_ssh_tool().ok_or("На роутере не найден ssh-клиент (dbclient или ssh)")?;
    let dropbear = snapshot_ssh_is_dropbear(tool);
    let key = snapshots::remote_key_path(data_dir);
    if !key.is_file() {
        tokio::fs::create_dir_all(snapshots::snapshot_dir(data_dir))
            .await
            .map_err(|e| e.to_string())?;
        let output = if !dropbear {
            std::process::Command::new("ssh-keygen")
                .args(["-q", "-t", "ed25519", "-N", "", "-C", "nfqws2-ui", "-f"])
                .arg(&key)
                .output()
        } else {
            std::process::Command::new("dropbearkey")
                .args(["-t", "ed25519", "-f"])
                .arg(&key)
                .output()
        }
        .map_err(|e| format!("Не удалось запустить генератор ключа: {e}"))?;
        if !output.status.success() {
            return Err(format!(
                "Не удалось создать ключ: {}",
                String::from_utf8_lossy(&output.stderr)
            ));
        }
    }
    #[cfg(unix)]
    tokio::fs::set_permissions(&key, std::fs::Permissions::from_mode(0o600))
        .await
        .map_err(|e| e.to_string())?;
    let pub_path = snapshots::remote_pub_path(data_dir);
    let raw_public = if pub_path.is_file() {
        tokio::fs::read_to_string(&pub_path)
            .await
            .unwrap_or_default()
            .trim()
            .to_owned()
    } else {
        let output = if dropbear {
            std::process::Command::new("dropbearkey")
                .args(["-y", "-f"])
                .arg(&key)
                .output()
        } else {
            std::process::Command::new("ssh-keygen")
                .args(["-y", "-f"])
                .arg(&key)
                .output()
        }
        .map_err(|e| e.to_string())?;
        String::from_utf8_lossy(&output.stdout)
            .lines()
            .find(|line| line.starts_with("ssh-"))
            .map(|line| {
                format!(
                    "{} nfqws2-ui",
                    line.split_whitespace()
                        .take(2)
                        .collect::<Vec<_>>()
                        .join(" ")
                )
            })
            .unwrap_or_default()
    };
    let public = normalize_snapshot_public_key(&raw_public)
        .ok_or("Не удалось прочитать открытый ключ роутера")?;
    tokio::fs::write(pub_path, format!("{public}\n"))
        .await
        .map_err(|e| e.to_string())?;
    Ok(public)
}

async fn remote_call(
    data_dir: &Path,
    op: &str,
    name: &str,
    input: Option<&Path>,
    output_path: Option<&Path>,
) -> Result<String, String> {
    let settings = snapshots::load_settings(data_dir).remote;
    validate_snapshot_remote_settings(&settings)?;
    if settings.host.is_empty() || settings.user.is_empty() || settings.dir.is_empty() {
        return Err("Сначала заполните и сохраните адрес, пользователя и папку NAS".into());
    }
    let tool = snapshot_ssh_tool().ok_or("На роутере нет ssh-клиента")?;
    let key = snapshots::remote_key_path(data_dir);
    if !key.is_file() {
        return Err("Сначала создайте ключ роутера".into());
    }
    if !matches!(op, "ping" | "list" | "put" | "get")
        || (!name.is_empty()
            && name != "index.json"
            && !snapshots::valid_snapshot_id(name.trim_end_matches(".tar.gz")))
    {
        return Err("Недопустимая команда или имя файла для NAS".into());
    }
    let target = format!("{}@{}", settings.user, settings.host);
    let remote_command = if name.is_empty() {
        op.to_owned()
    } else {
        format!("{op} {name}")
    };
    let mut command = tokio::process::Command::new(tool);
    if tool == "ssh" && !snapshot_ssh_is_dropbear(tool) {
        command
            .args(["-i"])
            .arg(&key)
            .args([
                "-p",
                &settings.port.to_string(),
                "-o",
                "BatchMode=yes",
                "-o",
                "StrictHostKeyChecking=accept-new",
                "-o",
            ])
            .arg(format!(
                "UserKnownHostsFile={}",
                snapshots::known_hosts_path(data_dir).display()
            ))
            .args(["-o", "ConnectTimeout=10"])
            .arg(&target)
            .arg(&remote_command);
    } else {
        command
            .args(["-y", "-i"])
            .arg(&key)
            .args(["-p", &settings.port.to_string()])
            .arg(&target)
            .arg(&remote_command);
    }
    command
        .env("HOME", data_dir)
        .env("PATH", "/opt/bin:/opt/sbin:/usr/bin:/bin:/usr/sbin:/sbin")
        .kill_on_drop(true);
    if let Some(path) = input {
        command.stdin(std::process::Stdio::from(
            std::fs::File::open(path).map_err(|e| e.to_string())?,
        ));
    } else {
        command.stdin(std::process::Stdio::null());
    }
    if let Some(path) = output_path {
        command.stdout(std::process::Stdio::from(
            std::fs::File::create(path).map_err(|e| e.to_string())?,
        ));
    }
    let output = tokio::time::timeout(std::time::Duration::from_secs(120), command.output())
        .await
        .map_err(|_| "Нет ответа от NAS за 120 секунд".to_owned())?
        .map_err(|e| format!("Не удалось запустить ssh: {e}"))?;
    if !output.status.success() {
        let message = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        return Err(if message.is_empty() {
            "NAS отклонил запрос".into()
        } else {
            message
        });
    }
    if output_path.is_some() {
        return Ok(String::new());
    }
    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

struct SnapshotPushLock(PathBuf);
impl Drop for SnapshotPushLock {
    fn drop(&mut self) {
        let _ = std::fs::remove_file(&self.0);
    }
}

async fn push_remote_snapshots(data_dir: &Path) -> Result<usize, String> {
    match push_remote_snapshots_inner(data_dir).await {
        Ok(sent) => Ok(sent),
        Err(message) => {
            let mut state: serde_json::Value =
                tokio::fs::read(snapshots::remote_state_path(data_dir))
                    .await
                    .ok()
                    .and_then(|bytes| serde_json::from_slice(&bytes).ok())
                    .unwrap_or_default();
            state["error"] = serde_json::json!(message);
            state["errorAt"] = serde_json::json!(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs()
            );
            let _ = tokio::fs::write(
                snapshots::remote_state_path(data_dir),
                serde_json::to_vec(&state).unwrap_or_default(),
            )
            .await;
            Err(message)
        }
    }
}

fn kick_remote_snapshot_push(data_dir: &Path) {
    if !snapshots::load_settings(data_dir).remote.enabled {
        return;
    }
    let directory = data_dir.to_path_buf();
    tokio::spawn(async move {
        let _ = push_remote_snapshots(&directory).await;
    });
}

async fn push_remote_snapshots_inner(data_dir: &Path) -> Result<usize, String> {
    let settings = snapshots::load_settings(data_dir);
    if !settings.remote.enabled {
        return Err("Отправка на NAS выключена".into());
    }
    let lock_path = snapshots::snapshot_dir(data_dir).join(".push.lock");
    tokio::fs::create_dir_all(snapshots::snapshot_dir(data_dir))
        .await
        .map_err(|e| e.to_string())?;
    let lock_file = match std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&lock_path)
    {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            #[cfg(unix)]
            {
                let owner = std::fs::read_to_string(&lock_path)
                    .ok()
                    .and_then(|value| value.trim().parse::<u32>().ok());
                if owner.is_none()
                    || owner.is_some_and(|pid| Path::new(&format!("/proc/{pid}")).exists())
                {
                    return Err("Отправка снимков на NAS уже выполняется".into());
                }
                let _ = std::fs::remove_file(&lock_path);
                std::fs::OpenOptions::new()
                    .write(true)
                    .create_new(true)
                    .open(&lock_path)
                    .map_err(|e| format!("Не удалось заблокировать отправку на NAS: {e}"))?
            }
            #[cfg(not(unix))]
            {
                return Err("Отправка снимков на NAS уже выполняется".into());
            }
        }
        Err(error) => return Err(format!("Не удалось заблокировать отправку на NAS: {error}")),
    };
    use std::io::Write as _;
    let mut lock_file = lock_file;
    let _ = writeln!(lock_file, "{}", std::process::id());
    drop(lock_file);
    let _lock = SnapshotPushLock(lock_path);
    let mut store = snapshots::load_store(data_dir);
    let mut sent = 0;
    let pending: Vec<String> = store
        .items
        .iter()
        .filter(|item| item.local && !item.remote)
        .map(|item| item.id.clone())
        .collect();
    for id in pending {
        let file = snapshots::snapshot_dir(data_dir).join(format!("{id}.tar.gz"));
        if let Err(error) =
            remote_call(data_dir, "put", &format!("{id}.tar.gz"), Some(&file), None).await
            && !error.contains("уже существует")
        {
            return Err(error);
        }
        if let Some(item) = store.items.iter_mut().find(|item| item.id == id) {
            item.remote = true;
        }
        sent += 1;
        snapshots::save_store(data_dir, &store)?;
    }
    let index_path = snapshots::index_path(data_dir);
    snapshots::apply_retention(data_dir)?;
    store = snapshots::load_store(data_dir);
    snapshots::save_store(data_dir, &store)?;
    remote_call(data_dir, "put", "index.json", Some(&index_path), None).await?;
    let mut state: serde_json::Value = tokio::fs::read(snapshots::remote_state_path(data_dir))
        .await
        .ok()
        .and_then(|bytes| serde_json::from_slice(&bytes).ok())
        .unwrap_or_default();
    state["ok"] = serde_json::json!(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
    );
    state["error"] = serde_json::Value::Null;
    tokio::fs::write(
        snapshots::remote_state_path(data_dir),
        serde_json::to_vec(&state).unwrap_or_default(),
    )
    .await
    .map_err(|e| e.to_string())?;
    Ok(sent)
}

fn start_snapshot_scheduler(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            if !snapshots::is_due_daily(&state.data_dir) {
                continue;
            }
            let Some(config) = config_path_for_state(&state) else {
                continue;
            };
            let guard = state.config_write_lock.lock().await;
            match snapshots::create_snapshot(&state.data_dir, &config, "ежедневный", "", false)
            {
                Ok(_) => {
                    snapshots::mark_daily_snapshot(&state.data_dir);
                    drop(guard);
                    kick_remote_snapshot_push(&state.data_dir);
                }
                Err(error) => eprintln!("daily snapshot failed: {error}"),
            }
        }
    });
}

async fn create_backup(State(state): State<Arc<AppState>>) -> Response {
    let Some(config) = config_path_for_state(&state) else {
        return error(StatusCode::NOT_FOUND, "Файл конфигурации не найден");
    };
    let _write_guard = state.config_write_lock.lock().await;
    let content = match tokio::fs::read(&config).await {
        Ok(content) if content.len() <= MAX_CONFIG_BYTES => content,
        Ok(_) => {
            return error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Конфигурация превышает лимит",
            );
        }
        Err(io_error) => return error(StatusCode::NOT_FOUND, io_error.to_string()),
    };
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let stamp = format!("{}.{:09}", now.as_secs(), now.subsec_nanos());
    let id = format!("nfqws2.conf.{stamp}.bak");
    let directory = state.data_dir.join("backups");
    if let Err(io_error) = tokio::fs::create_dir_all(&directory).await {
        return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(io_error) =
            tokio::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700)).await
        {
            return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string());
        }
    }
    let backup_path = directory.join(&id);
    let mut backup = match tokio::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&backup_path)
        .await
    {
        Ok(file) => file,
        Err(io_error) => return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string()),
    };
    if let Err(io_error) = backup.write_all(&content).await {
        let _ = tokio::fs::remove_file(&backup_path).await;
        return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string());
    }
    if let Err(io_error) = backup.sync_all().await {
        let _ = tokio::fs::remove_file(&backup_path).await;
        return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string());
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(io_error) =
            tokio::fs::set_permissions(&backup_path, std::fs::Permissions::from_mode(0o600)).await
        {
            let _ = tokio::fs::remove_file(&backup_path).await;
            return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string());
        }
    }
    (
        StatusCode::CREATED,
        Json(BackupItem {
            id,
            created_at: now.as_secs(),
            file: "nfqws2.conf".into(),
        }),
    )
        .into_response()
}

async fn restore_backup(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    if id.len() > 96
        || !id.starts_with("nfqws2.conf.")
        || !id.ends_with(".bak")
        || !id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_')
    {
        return error(StatusCode::BAD_REQUEST, "Неверный ID резервной копии");
    }
    let backup = state.data_dir.join("backups").join(id);
    let Some(config) = config_path_for_state(&state) else {
        return error(StatusCode::NOT_FOUND, "Файл конфигурации не найден");
    };
    let content = match tokio::fs::read(&backup).await {
        Ok(content) if content.len() <= MAX_CONFIG_BYTES => content,
        Ok(_) => {
            return error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Резервная копия превышает лимит",
            );
        }
        Err(io_error) => return error(StatusCode::NOT_FOUND, io_error.to_string()),
    };
    let _write_guard = state.config_write_lock.lock().await;
    if let Err(message) = backup_and_write(&state.data_dir, &config, &content).await {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    Json(MessageResponse {
        message: "Конфигурация восстановлена; перед этим создана backup текущего файла".into(),
    })
    .into_response()
}

async fn read_bounded_tail(path: &Path, max_bytes: usize) -> std::io::Result<Vec<u8>> {
    let mut file = tokio::fs::File::open(path).await?;
    let file_len = file.seek(std::io::SeekFrom::End(0)).await?;
    let offset = file_len.saturating_sub(max_bytes as u64);
    file.seek(std::io::SeekFrom::Start(offset)).await?;

    let mut tail = Vec::with_capacity((file_len - offset) as usize);
    file.take(max_bytes as u64).read_to_end(&mut tail).await?;
    Ok(tail)
}

fn configured_log_path(config_text: &str, config_file: &Path) -> Option<PathBuf> {
    let mut found: Option<String> = None;
    for line in config_text.lines() {
        let line = strip_shell_comment(line).trim();
        let Some((name, raw_value)) = line.split_once('=') else {
            continue;
        };
        if name.trim() != "LOG_DEBUG_PATH" {
            continue;
        }
        let mut value = raw_value.trim();
        if value.len() >= 2 {
            let bytes = value.as_bytes();
            if (bytes[0] == b'\'' && bytes[value.len() - 1] == b'\'')
                || (bytes[0] == b'"' && bytes[value.len() - 1] == b'"')
            {
                value = &value[1..value.len() - 1];
            }
        }
        let value = value.trim().strip_prefix('@').unwrap_or(value.trim());
        if value.is_empty()
            || value
                .chars()
                .any(|character| matches!(character, '$' | '`' | ';' | '\0' | '\n' | '\r'))
        {
            return None;
        }
        if found.as_deref().is_some_and(|previous| previous != value) {
            return None;
        }
        found = Some(value.to_string());
    }
    let value = found?;
    let path = PathBuf::from(value);
    Some(if path.is_absolute() {
        path
    } else {
        config_file.parent()?.join(path)
    })
}

async fn get_logs(State(state): State<Arc<AppState>>, Query(query): Query<LogsQuery>) -> Response {
    if query.source != "nfqws2" {
        return error(StatusCode::BAD_REQUEST, "Неизвестный источник журнала");
    }
    let config = config_path_for_state(&state);
    let config_text = config
        .as_deref()
        .and_then(|path| std::fs::read_to_string(path).ok());
    let discovered = discover_system_paths();
    let log_dir = discovered
        .as_ref()
        .map(|paths| paths.log_dir.clone())
        .or_else(|| {
            config
                .as_deref()?
                .ancestors()
                .nth(3)
                .map(|root| root.join("var/log"))
        });
    let Some(log_dir) = log_dir else {
        return Json(LogsResponse {
            content: "Каталог журналов nfqws2 не обнаружен".into(),
        })
        .into_response();
    };
    let mut candidates = Vec::new();
    if let (Some(text), Some(config_path)) = (config_text.as_deref(), config.as_deref())
        && let Some(path) = configured_log_path(text, config_path)
    {
        candidates.push(path);
    }
    candidates
        .extend(["nfqws2.log", "nfqws.log", "nfqws2-debug.log"].map(|name| log_dir.join(name)));
    let canonical_log_dir = log_dir.canonicalize().ok();
    let Some(path) = candidates.into_iter().find(|candidate| {
        candidate.is_file()
            && candidate.canonicalize().ok().is_some_and(|canonical| {
                canonical_log_dir
                    .as_ref()
                    .is_some_and(|root| canonical.starts_with(root))
            })
    }) else {
        return Json(LogsResponse {
            content: "Журнал nfqws2 не найден".into(),
        })
        .into_response();
    };
    let bytes = match read_bounded_tail(&path, MAX_LOG_BYTES).await {
        Ok(bytes) => bytes,
        Err(io_error) => return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string()),
    };
    let mut content = String::from_utf8_lossy(&bytes).into_owned();
    if let Some(lines) = query.tail
        && lines > 0
        && lines < 5000
    {
        content = content
            .lines()
            .rev()
            .take(lines)
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect::<Vec<_>>()
            .join("\n");
    }
    Json(LogsResponse { content }).into_response()
}

async fn bind_port() -> Result<(TcpListener, u16), String> {
    for port in 1002..=1100 {
        match TcpListener::bind(SocketAddr::from(([0, 0, 0, 0], port))).await {
            Ok(listener) => return Ok((listener, port)),
            Err(error) if error.kind() == std::io::ErrorKind::AddrInUse => continue,
            Err(error) => return Err(format!("Не удалось открыть порт {port}: {error}")),
        }
    }
    Err("Порты 1002–1100 заняты".into())
}

fn api_router(state: Arc<AppState>, web_root: PathBuf) -> Router {
    Router::new()
        .route("/", get(panel_shell))
        .route("/api/health", get(health))
        .route("/api/nfqws2/installation", get(get_installation))
        .route("/api/nfqws2/scripts", get(get_lua_scripts))
        .route("/api/nfqws2/scripts/{name}", get(get_lua_script))
        .route("/api/status", get(get_status))
        .route("/api/nfqws2/runtime", get(get_runtime))
        .route("/api/service", post(post_service))
        .route("/api/config", get(get_config).put(put_config))
        .route("/api/nfqws2/config/preview", post(preview_config))
        .route("/api/nfqws2/config/validate", post(validate_config))
        .route("/api/nfqws2/diagnostics/site", post(diagnose_site))
        .route("/api/nfqws2/diagnostics/monitor", post(monitor_sites))
        .route(
            "/api/nfqws2/monitoring",
            get(get_monitoring_store).put(put_monitoring_store),
        )
        .route("/api/nfqws2/monitoring/run", post(run_saved_monitoring))
        .route(
            "/api/nfqws2/telegram",
            get(get_telegram_settings).put(put_telegram_settings),
        )
        .route(
            "/api/nfqws2/telegram/notifications",
            get(get_telegram_notifications),
        )
        .route("/api/nfqws2/telegram/test", post(telegram_test))
        .route("/api/nfqws2/telegram/find-chats", post(telegram_find_chats))
        .route(
            "/api/nfqws2/diagnostics/transfer",
            post(start_transfer_diagnostic),
        )
        .route(
            "/api/nfqws2/diagnostics/transfer/operations/{id}",
            get(get_transfer_diagnostic_operation),
        )
        .route(
            "/api/nfqws2/diagnostics/transfer/operations/{id}/cancel",
            post(cancel_transfer_diagnostic),
        )
        .route(
            "/api/nfqws2/diagnostics/freeze-scan",
            post(start_freeze_scan),
        )
        .route(
            "/api/nfqws2/diagnostics/freeze-scan/operations/{id}",
            get(get_freeze_scan),
        )
        .route(
            "/api/nfqws2/diagnostics/freeze-scan/operations/{id}/cancel",
            post(cancel_freeze_scan),
        )
        .route("/api/nfqws2/diagnostics/trace", post(trace_site))
        .route("/api/nfqws2/strategies/pick", post(start_strategy_pick))
        .route(
            "/api/nfqws2/strategies/pick/history",
            get(get_strategy_pick_history),
        )
        .route(
            "/api/nfqws2/strategies/pick/operations/{id}",
            get(get_strategy_pick_operation),
        )
        .route(
            "/api/nfqws2/strategies/pick/operations/{id}/cancel",
            post(stop_strategy_pick_operation),
        )
        .route(
            "/api/nfqws2/strategies/pick/operations/{id}/preview",
            post(preview_strategy_pick),
        )
        .route(
            "/api/nfqws2/strategies/preview",
            post(preview_freeze_strategy),
        )
        .route(
            "/api/nfqws2/strategies/apply",
            post(apply_confirmed_strategy),
        )
        .route(
            "/api/nfqws2/diagnostics/trace/operations/{id}",
            get(get_site_trace_operation),
        )
        .route(
            "/api/nfqws2/diagnostics/trace/operations/{id}/stop",
            post(stop_site_trace_operation),
        )
        .route("/api/nfqws2/apply", post(apply_config))
        .route("/api/nfqws2/lists", get(get_lists))
        .route(
            "/api/nfqws2/lists/{id}",
            get(get_list).put(put_list).delete(delete_custom_list),
        )
        .route("/api/backups", get(get_backups).post(create_backup))
        .route("/api/backups/{id}/restore", post(restore_backup))
        .route(
            "/api/snapshots",
            get(get_snapshots).post(create_snapshot_api),
        )
        .route(
            "/api/snapshots/settings",
            axum::routing::put(update_snapshot_settings),
        )
        .route(
            "/api/snapshots/current/archive",
            get(download_current_snapshot),
        )
        .route(
            "/api/snapshots/restore-file",
            post(restore_uploaded_snapshot)
                .layer(DefaultBodyLimit::max(snapshots::MAX_ARCHIVE_BYTES)),
        )
        .route("/api/snapshots/remote/key", post(create_snapshot_key))
        .route("/api/snapshots/remote/test", post(test_snapshot_remote))
        .route("/api/snapshots/remote/push", post(push_snapshot_remote))
        .route(
            "/api/ui/appearance",
            get(get_ui_appearance).put(put_ui_appearance),
        )
        .route("/api/snapshots/{id}/archive", get(download_snapshot))
        .route("/api/snapshots/{id}/restore", post(restore_snapshot_api))
        .route("/api/snapshots/{id}/pin", post(pin_snapshot))
        .route("/api/logs", get(get_logs))
        .route("/api/updates/releases", get(get_github_releases))
        .route("/api/updates/status", get(get_ui_update_status))
        .route("/api/updates/install", post(post_update_install))
        .route("/api/nfqws2/updates/releases", get(get_core_releases))
        .route(
            "/api/nfqws2/updates/install",
            post(post_core_update_install),
        )
        .route("/api/updates/operations/{id}", get(get_update_operation))
        .with_state(state)
        .fallback_service(ServeDir::new(web_root).append_index_html_on_directories(true))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(security_headers))
}

async fn panel_shell() -> Html<&'static str> {
    Html(PANEL_SHELL_HTML)
}

#[tokio::main]
async fn main() {
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("nfqws2-ui {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let data_dir = std::env::var_os("NFQWS2_UI_DATA_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/opt/var/lib/nfqws2-ui"));
    let (listener, port) = match bind_port().await {
        Ok(value) => value,
        Err(error) => {
            eprintln!("nfqws2-ui: {error}");
            std::process::exit(1);
        }
    };
    let port_file = std::env::var_os("NFQWS2_UI_PORT_FILE")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/opt/var/run/nfqws2-ui.port"));
    if let Some(parent) = port_file.parent()
        && let Err(error) = std::fs::create_dir_all(parent)
    {
        eprintln!("nfqws2-ui: cannot create port-file directory: {error}");
        std::process::exit(1);
    }
    let port_tmp = port_file.with_extension(format!("port.{}.tmp", std::process::id()));
    if let Err(error) = std::fs::write(&port_tmp, port.to_string())
        .and_then(|()| std::fs::rename(&port_tmp, &port_file))
    {
        eprintln!("nfqws2-ui: cannot persist selected port: {error}");
        std::process::exit(1);
    }
    let state = Arc::new(AppState {
        data_dir,
        port,
        cpu_previous: Arc::new(Mutex::new(None)),
        config_write_lock: Mutex::new(()),
        config_override: None,
        update_lock: Mutex::new(()),
        ui_update_status: Arc::new(std::sync::RwLock::new(UiUpdateStatus::default())),
        site_diagnostic_lock: Arc::new(Mutex::new(())),
        monitoring_lock: Arc::new(Mutex::new(())),
        site_trace_controls: Mutex::new(HashMap::new()),
        site_trace_runner: Arc::new(router_site_trace_runner),
        strategy_pick_controls: Mutex::new(HashMap::new()),
        transfer_diagnostic_controls: Mutex::new(HashMap::new()),
        freeze_scan_controls: Mutex::new(HashMap::new()),
    });
    start_ui_update_checker(state.clone());
    start_monitoring_scheduler(state.clone());
    start_snapshot_scheduler(state.clone());
    let web_root = std::env::var_os("NFQWS2_UI_WEB_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/opt/share/nfqws2-ui/current/public"));
    let app = api_router(state, web_root);
    tracing::info!("nfqws2-ui listening on 0.0.0.0:{port}");
    if let Err(error) = axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await
    {
        eprintln!("nfqws2-ui server error: {error}");
    }
}

async fn shutdown_signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        let Ok(mut interrupt) = signal(SignalKind::interrupt()) else {
            let _ = tokio::signal::ctrl_c().await;
            return;
        };
        let Ok(mut terminate) = signal(SignalKind::terminate()) else {
            let _ = tokio::signal::ctrl_c().await;
            return;
        };
        tokio::select! {
            _ = interrupt.recv() => {},
            _ = terminate.recv() => {},
        }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    #[test]
    fn monitoring_store_keeps_a_defaulted_telegram_delivery_queue_for_existing_installations() {
        let old_store = serde_json::json!({
            "enabled": true,
            "intervalMinutes": 30,
            "lastAutoCheck": null,
            "sites": [],
            "telegram": {}
        });
        let parsed: MonitoringStore = serde_json::from_value(old_store).unwrap();
        let serialized = serde_json::to_value(parsed).unwrap();

        assert!(serialized.get("telegramNotifications").is_some());
        assert_eq!(format_telegram_event_time(0), "01.01.1970 00:00 UTC");
    }

    #[test]
    fn monitoring_accepts_podkop_fake_ipv4_range_but_rejects_other_reserved_addresses() {
        assert!(monitor_probe_ipv4("198.18.0.0".parse().unwrap()));
        assert!(monitor_probe_ipv4("198.19.255.255".parse().unwrap()));
        assert!(!monitor_probe_ipv4("198.51.100.1".parse().unwrap()));
        assert!(!monitor_probe_ipv4("127.0.0.1".parse().unwrap()));
    }

    #[test]
    fn ui_update_status_selects_newest_semver_and_only_announces_newer_version() {
        let releases = vec![
            PanelRelease {
                tag: "v0.3.8".into(),
                published_at: None,
                notes: String::new(),
            },
            PanelRelease {
                tag: "v0.3.10".into(),
                published_at: None,
                notes: String::new(),
            },
            PanelRelease {
                tag: "v0.3.9".into(),
                published_at: None,
                notes: String::new(),
            },
        ];
        let update = ui_update_status("0.3.9", &releases, 123);
        assert_eq!(update.latest_version.as_deref(), Some("v0.3.10"));
        assert!(update.update_available);
        assert_eq!(update.checked_at, Some(123));
        assert_eq!(
            update.release_url.as_deref(),
            Some("https://github.com/rangerxxyy/nfqws2-ui/releases/tag/v0.3.10")
        );

        let current = ui_update_status("0.3.10", &releases, 456);
        assert!(!current.update_available);
        let newer_current = ui_update_status("0.3.11", &releases, 789);
        assert!(!newer_current.update_available);
    }

    #[test]
    fn manual_ui_release_check_refreshes_the_background_notification_cache() {
        let cache = std::sync::RwLock::new(UiUpdateStatus::default());
        let releases = vec![PanelRelease {
            tag: "v0.3.7".into(),
            published_at: None,
            notes: String::new(),
        }];

        let checked = cache_ui_update_status(&cache, &releases, 456);
        let background_status = cache.read().unwrap().clone();

        assert_eq!(checked.latest_version.as_deref(), Some("v0.3.7"));
        assert_eq!(checked.checked_at, Some(456));
        assert_eq!(background_status.latest_version, checked.latest_version);
        assert_eq!(background_status.release_url, checked.release_url);
        assert_eq!(background_status.checked_at, Some(456));
    }

    #[test]
    fn ui_update_status_uses_only_releases_with_the_existing_arm64_package_gate() {
        let json = r#"[
          {"tag_name":"v0.3.11","draft":false,"prerelease":false,"assets":[]},
          {"tag_name":"v0.3.10","draft":false,"prerelease":false,"assets":[
            {"name":"nfqws2-ui-v0.3.10-aarch64-unknown-linux-musl.tar.gz"},
            {"name":"nfqws2-ui-v0.3.10-aarch64-unknown-linux-musl.tar.gz.sha256"}
          ]}
        ]"#;
        let eligible = eligible_panel_releases(json).unwrap();
        let status = ui_update_status("0.3.9", &eligible, 123);
        assert_eq!(status.latest_version.as_deref(), Some("v0.3.10"));
        assert!(status.update_available);
    }

    fn test_state(data_dir: PathBuf) -> Arc<AppState> {
        test_state_with_config(data_dir, None)
    }

    fn test_state_with_config(
        data_dir: PathBuf,
        config_override: Option<PathBuf>,
    ) -> Arc<AppState> {
        Arc::new(AppState {
            data_dir,
            port: 1002,
            cpu_previous: Arc::new(Mutex::new(None)),
            config_write_lock: Mutex::new(()),
            config_override,
            update_lock: Mutex::new(()),
            ui_update_status: Arc::new(std::sync::RwLock::new(UiUpdateStatus::default())),
            site_diagnostic_lock: Arc::new(Mutex::new(())),
            monitoring_lock: Arc::new(Mutex::new(())),
            site_trace_controls: Mutex::new(HashMap::new()),
            site_trace_runner: Arc::new(router_site_trace_runner),
            strategy_pick_controls: Mutex::new(HashMap::new()),
            transfer_diagnostic_controls: Mutex::new(HashMap::new()),
            freeze_scan_controls: Mutex::new(HashMap::new()),
        })
    }

    fn confirmed_freeze_scan(kind: freeze_diagnostic::FreezeTargetKind) -> FreezeScanOperation {
        FreezeScanOperation {
            id: "0123456789abcdef0123456789abcdef".into(),
            state: "complete".into(),
            stage: "complete".into(),
            current: None,
            done: 1,
            total: 1,
            results: vec![freeze_diagnostic::FreezeNetworkResult {
                label: "site.example".into(),
                host: "site.example".into(),
                path: "/".into(),
                target_kind: kind,
                address: Some(std::net::Ipv4Addr::new(203, 0, 113, 15)),
                asn_hint: None,
                asn: freeze_diagnostic::AsnVerification::Unavailable,
                direct: freeze_diagnostic::FreezeMeasurement {
                    classification: "PossibleFreeze".into(),
                    ok: false,
                    bytes_received: Some(18_000),
                    replies: None,
                    elapsed_ms: 4000,
                    reason: "repeatable boundary timeout".into(),
                },
                current: freeze_diagnostic::FreezeMeasurement {
                    classification: "PossibleFreeze".into(),
                    ok: false,
                    bytes_received: Some(18_000),
                    replies: None,
                    elapsed_ms: 4000,
                    reason: "repeatable boundary timeout".into(),
                },
                freeze_detected: true,
                candidate_total: 1,
                tried_candidates: 1,
                found_strategy: Some(freeze_diagnostic::FreezeStrategyMatch {
                    sni: "ya.ru".into(),
                    step: "--lua-desync=hostfakesplit:repeats=4:tcp_ts=-600000:tcp_md5:host=ya.ru"
                        .into(),
                    attempts: 3,
                    successes: 3,
                    average_ms: Some(100),
                    reused_from: None,
                }),
                candidate_attempts: Vec::new(),
            }],
            message: "done".into(),
            updated_at: 1,
        }
    }

    #[test]
    fn freeze_scan_application_is_limited_to_confirmed_user_domains() {
        let scan = confirmed_freeze_scan(freeze_diagnostic::FreezeTargetKind::UserHost);
        let candidate = freeze_result_as_strategy_operation(&scan, 0).unwrap();
        assert_eq!(candidate.host, "site.example");
        assert_eq!(candidate.results[0].successes, 3);
        let mut confirmed_without_baseline_flag = scan.clone();
        confirmed_without_baseline_flag.results[0].freeze_detected = false;
        assert!(freeze_result_as_strategy_operation(&confirmed_without_baseline_flag, 0).is_ok());
        assert!(
            freeze_result_as_strategy_operation(
                &confirmed_freeze_scan(freeze_diagnostic::FreezeTargetKind::PinnedAddress),
                0
            )
            .is_err()
        );
        let mut partial = scan;
        partial.state = "partial".into();
        assert!(freeze_result_as_strategy_operation(&partial, 0).is_ok());
        let mut reused = confirmed_freeze_scan(freeze_diagnostic::FreezeTargetKind::UserHost);
        reused.results[0]
            .found_strategy
            .as_mut()
            .unwrap()
            .reused_from = Some("other.example".into());
        assert!(freeze_result_as_strategy_operation(&reused, 0).is_err());
    }

    #[test]
    fn freeze_scan_application_can_use_a_persisted_confirmation_while_scan_runs() {
        let mut scan = confirmed_freeze_scan(freeze_diagnostic::FreezeTargetKind::UserHost);
        scan.state = "running".into();
        scan.stage = "strategy_search".into();
        scan.done = 21;
        scan.total = 34;
        assert_eq!(
            freeze_result_as_strategy_operation(&scan, 0).unwrap().host,
            "site.example"
        );
        scan.state = "partial".into();
        assert_eq!(
            freeze_result_as_strategy_operation(&scan, 0).unwrap().host,
            "site.example"
        );
    }

    #[test]
    fn freeze_scan_preview_scopes_confirmed_strategy_to_the_entered_domain() {
        let operation = freeze_result_as_strategy_operation(
            &confirmed_freeze_scan(freeze_diagnostic::FreezeTargetKind::UserHost),
            0,
        )
        .unwrap();
        let preview = build_strategy_pick_preview(
            &operation,
            0,
            "NFQWS_ARGS_CUSTOM=\"\"\n",
            "revision-1".into(),
            &HashSet::from(["hostfakesplit".to_string()]),
            &[],
        )
        .unwrap();
        assert!(
            preview
                .proposed_content
                .contains("--hostlist-domains=site.example")
        );
        assert!(preview.proposed_content.contains("hostfakesplit:repeats=4"));
        assert_eq!(preview.host, "site.example");
    }

    fn test_state_with_trace_runner(
        data_dir: PathBuf,
        runner: Arc<SiteTraceRunner>,
    ) -> Arc<AppState> {
        Arc::new(AppState {
            data_dir,
            port: 1002,
            cpu_previous: Arc::new(Mutex::new(None)),
            config_write_lock: Mutex::new(()),
            config_override: None,
            update_lock: Mutex::new(()),
            ui_update_status: Arc::new(std::sync::RwLock::new(UiUpdateStatus::default())),
            site_diagnostic_lock: Arc::new(Mutex::new(())),
            monitoring_lock: Arc::new(Mutex::new(())),
            site_trace_controls: Mutex::new(HashMap::new()),
            site_trace_runner: runner,
            strategy_pick_controls: Mutex::new(HashMap::new()),
            transfer_diagnostic_controls: Mutex::new(HashMap::new()),
            freeze_scan_controls: Mutex::new(HashMap::new()),
        })
    }

    #[test]
    fn isolated_strategy_probe_keeps_first_profile_filter_after_trace_argv_cleanup() {
        let candidate_args = vec![
            "--filter-tcp=443".to_string(),
            "--filter-l7=tls".to_string(),
            "--payload=tls_client_hello".to_string(),
            "--lua-desync=hostfakesplit:host=ya.ru".to_string(),
        ];
        let argv = strategy_probe_argv("/opt/usr/bin/nfqws2", &candidate_args);
        let result = site_trace::trace_argv(&argv, 301, Path::new("/tmp/trace.log"));
        assert_eq!(result.first().map(String::as_str), Some("--filter-tcp=443"));
        assert!(result.contains(&"--filter-l7=tls".into()));
        assert!(result.contains(&"--payload=tls_client_hello".into()));
        assert!(result.contains(&"--lua-desync=hostfakesplit:host=ya.ru".into()));
    }

    fn confirmed_strategy_pick(
        state: &str,
        protocol: &str,
        confirmed: bool,
    ) -> StrategyPickOperation {
        StrategyPickOperation {
            id: "0123456789abcdef0123456789abcdef".into(),
            host: "example.com".into(),
            protocol: protocol.into(),
            state: state.into(),
            stage: state.into(),
            current: None,
            done: 1,
            total: 1,
            results: vec![strategy_picker::StrategyCandidateResult {
                candidate: strategy_picker::StrategyCandidate {
                    name: "multisplit".into(),
                    source: "verified test fixture".into(),
                    steps: vec!["--lua-desync=multisplit:pos=1".into()],
                },
                attempts: 2,
                successes: if confirmed { 2 } else { 1 },
                confirmed,
                elapsed_ms: 10,
                detail: "two target probes".into(),
            }],
            message: state.into(),
            updated_at: now_unix_seconds(),
        }
    }

    #[tokio::test]
    async fn picker_reuses_only_confirmed_candidates_for_the_same_host_and_protocol() {
        let dir = tempfile::tempdir().unwrap();
        let mut saved = confirmed_strategy_pick("complete", "https", true);
        write_strategy_pick_operation_sync(dir.path(), &saved).unwrap();
        saved.id = "1123456789abcdef0123456789abcdef".into();
        saved.host = "other.example".into();
        saved.results[0].candidate.steps = vec!["--lua-desync=multisplit:pos=2".into()];
        write_strategy_pick_operation_sync(dir.path(), &saved).unwrap();
        saved.id = "2123456789abcdef0123456789abcdef".into();
        saved.host = "example.com".into();
        saved.protocol = "http".into();
        write_strategy_pick_operation_sync(dir.path(), &saved).unwrap();
        saved.id = "3123456789abcdef0123456789abcdef".into();
        saved.protocol = "https".into();
        saved.results[0].confirmed = false;
        write_strategy_pick_operation_sync(dir.path(), &saved).unwrap();

        let candidates =
            load_confirmed_strategy_candidates(dir.path(), "example.com", "https", false).await;
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].source, "Ранее подтверждена для этого домена");
        let candidates =
            load_confirmed_strategy_candidates(dir.path(), "example.com", "https", true).await;
        assert_eq!(candidates.len(), 2);
        assert_eq!(
            candidates[1].source,
            "История: раньше помогла other.example"
        );
    }

    #[test]
    fn strategy_preview_uses_only_the_confirmed_persisted_candidate_and_preserves_config() {
        let original = "MODE_LIST='--hostlist=lists/user.list'\nNFQWS_ARGS_CUSTOM=\"--new --filter-tcp=80\" # custom\n";
        let functions = HashSet::from(["multisplit".to_string()]);
        let preview = build_strategy_pick_preview(
            &confirmed_strategy_pick("complete", "https", true),
            0,
            original,
            "rev-1".into(),
            &functions,
            &[],
        )
        .unwrap();
        assert_eq!(preview.host, "example.com");
        assert_eq!(preview.protocol, "https");
        assert_eq!(preview.original_revision, "rev-1");
        assert!(
            preview
                .proposed_content
                .contains("MODE_LIST='--hostlist=lists/user.list'")
        );
        assert!(preview.proposed_content.contains("# custom"));
        assert!(
            preview
                .proposed_content
                .contains("--hostlist-domains=example.com")
        );
        assert!(
            preview
                .proposed_content
                .starts_with(original.split("NFQWS_ARGS_CUSTOM").next().unwrap())
        );
    }

    #[test]
    fn strategy_dry_run_uses_only_server_persisted_confirmed_candidate() {
        let operation = confirmed_strategy_pick("complete", "https", true);
        let active_args = vec![
            "/opt/usr/bin/nfqws2".into(),
            "--qnum=300".into(),
            "--lua-init=@/opt/etc/nfqws2/lua/zapret-lib.lua".into(),
            "--new".into(),
            "--filter-tcp=443".into(),
            "--lua-desync=old:pos=1".into(),
        ];
        let argv = strategy_dry_run_argv(
            &operation,
            0,
            &active_args,
            &HashSet::from(["multisplit".to_string()]),
            &[],
        )
        .unwrap();
        assert_eq!(argv[0], "/opt/usr/bin/nfqws2");
        assert_eq!(argv[1], "--dry-run");
        assert!(argv.iter().any(|argument| argument == "--qnum=300"));
        assert!(
            argv.iter()
                .any(|argument| argument == "--lua-desync=multisplit:pos=1")
        );
        assert!(
            !argv
                .iter()
                .any(|argument| argument == "--lua-desync=old:pos=1")
        );
        let mut unconfirmed = operation;
        unconfirmed.results[0].confirmed = false;
        assert!(
            strategy_dry_run_argv(
                &unconfirmed,
                0,
                &active_args,
                &HashSet::from(["multisplit".to_string()]),
                &[]
            )
            .is_err()
        );
    }

    #[test]
    fn strategy_preview_rejects_incomplete_unconfirmed_quic_and_out_of_range_results() {
        let original = "NFQWS_ARGS_CUSTOM=\"\"\n";
        let functions = HashSet::from(["multisplit".to_string()]);
        let revision = "rev".to_owned();
        for (operation, index) in [
            (confirmed_strategy_pick("cancelled", "https", true), 0),
            (confirmed_strategy_pick("complete", "https", false), 0),
            (confirmed_strategy_pick("complete", "quic", true), 0),
            (confirmed_strategy_pick("complete", "https", true), 1),
        ] {
            assert!(
                build_strategy_pick_preview(
                    &operation,
                    index,
                    original,
                    revision.clone(),
                    &functions,
                    &[],
                )
                .is_err()
            );
        }
    }

    #[test]
    fn operation_pruning_keeps_running_jobs_and_bounds_completed_history() {
        let dir = tempfile::tempdir().unwrap();
        for (index, state) in ["complete", "no_match", "cancelled", "failed", "running"]
            .into_iter()
            .enumerate()
        {
            std::fs::write(
                dir.path().join(format!("{index:032x}.json")),
                serde_json::json!({"state": state}).to_string(),
            )
            .unwrap();
        }
        std::fs::write(dir.path().join("corrupt.json"), "{").unwrap();
        std::fs::write(
            dir.path().join("oversized.json"),
            vec![b' '; MAX_STRATEGY_PICK_STATE_BYTES + 1],
        )
        .unwrap();
        std::fs::write(
            dir.path().join("unknown.json"),
            serde_json::json!({"state": "unexpected"}).to_string(),
        )
        .unwrap();

        prune_finished_json_operations(dir.path(), 2);

        let remaining = std::fs::read_dir(dir.path())
            .unwrap()
            .filter_map(Result::ok)
            .map(|entry| {
                let bytes = std::fs::read(entry.path()).unwrap();
                serde_json::from_slice::<serde_json::Value>(&bytes).unwrap()["state"]
                    .as_str()
                    .unwrap()
                    .to_string()
            })
            .collect::<Vec<_>>();
        assert_eq!(remaining.len(), 3);
        assert_eq!(
            remaining.iter().filter(|state| *state == "running").count(),
            1
        );
        assert_eq!(
            remaining.iter().filter(|state| *state != "running").count(),
            2
        );
        assert!(!dir.path().join("corrupt.json").exists());
        assert!(!dir.path().join("oversized.json").exists());
        assert!(!dir.path().join("unknown.json").exists());
    }

    async fn body_json(response: Response) -> serde_json::Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[test]
    fn runtime_inventory_uses_only_active_argv_and_splits_at_new_profiles() {
        let args = [
            "/opt/usr/bin/nfqws2",
            "--user=nobody",
            "--qnum=300",
            "--filter-tcp=443,80",
            "--payload=tls_client_hello",
            "--lua-desync=fake:blob=secret-ish-name",
            "--lua-desync=multisplit:pos=midsld",
            "--new",
            "--name=QUIC",
            "--filter-udp=443",
            "--payload=quic_initial",
            "--lua-desync=fake:blob=quic_initial",
        ]
        .map(str::to_owned);
        let profiles = runtime_profiles(&args);
        assert_eq!(profiles.len(), 2);
        assert_eq!(profiles[0].index, 1);
        assert_eq!(profiles[0].name, None);
        assert_eq!(profiles[0].filters, ["--filter-tcp=443,80"]);
        assert_eq!(profiles[0].strategies, ["fake", "multisplit"]);
        assert_eq!(profiles[0].payloads, ["tls_client_hello"]);
        assert_eq!(profiles[1].name.as_deref(), Some("QUIC"));
        assert_eq!(profiles[1].filters, ["--filter-udp=443"]);
        assert_eq!(profiles[1].strategies, ["fake"]);
        assert_eq!(profiles[1].payloads, ["quic_initial"]);
        let json = serde_json::to_string(&profiles).unwrap();
        assert!(!json.contains("secret-ish-name"));
    }

    #[test]
    fn runtime_inventory_ignores_empty_groups_and_non_profile_arguments() {
        let args = [
            "/opt/usr/bin/nfqws2",
            "--user=nobody",
            "--new",
            "--new",
            "--filter-tcp=443",
        ]
        .map(str::to_owned);
        let profiles = runtime_profiles(&args);
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].filters, ["--filter-tcp=443"]);
    }

    #[tokio::test]
    async fn log_tail_reads_at_most_the_requested_bytes() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("nfqws2.log");
        let contents = vec![b'x'; MAX_LOG_BYTES * 4];
        tokio::fs::write(&path, &contents).await.unwrap();

        let tail = read_bounded_tail(&path, 1024).await.unwrap();

        assert_eq!(tail.len(), 1024);
        assert_eq!(tail, contents[contents.len() - 1024..]);
    }

    #[test]
    fn log_path_uses_literal_debug_path_from_active_config() {
        let config = Path::new("/opt/etc/nfqws2/nfqws2.conf");
        assert_eq!(
            configured_log_path(
                "LOG_DEBUG_PATH=\"@/opt/var/log/nfqws2-debug.log\"\n",
                config,
            ),
            Some(PathBuf::from("/opt/var/log/nfqws2-debug.log"))
        );
    }

    #[test]
    fn log_path_does_not_evaluate_dynamic_shell_values() {
        assert_eq!(
            configured_log_path(
                "LOG_DEBUG_PATH=\"$DATA_DIR/nfqws2.log\"\n",
                Path::new("/opt/etc/nfqws2/nfqws2.conf"),
            ),
            None
        );
    }

    #[test]
    fn process_argument_list_parser_accepts_equals_and_separate_values() {
        assert_eq!(
            list_reference_from_argv(
                &[
                    "nfqws2".into(),
                    "--hostlist".into(),
                    "lists/user.list".into()
                ],
                "user.list"
            ),
            Some(PathBuf::from("lists/user.list"))
        );
        assert_eq!(
            list_reference_from_argv(
                &["nfqws2".into(), "--hostlist-auto=lists/auto.list".into()],
                "auto.list"
            ),
            Some(PathBuf::from("lists/auto.list"))
        );
    }

    #[test]
    fn process_argument_list_parser_rejects_conflicting_or_dynamic_paths() {
        assert_eq!(
            list_reference_from_argv(
                &[
                    "nfqws2".into(),
                    "--hostlist=a.list".into(),
                    "--hostlist=b.list".into()
                ],
                "user.list"
            ),
            None
        );
        assert_eq!(
            list_reference_from_argv(&["nfqws2".into(), "--hostlist=$LIST".into()], "user.list"),
            None
        );
    }

    #[test]
    fn release_catalog_keeps_only_stable_releases_with_package_and_checksum() {
        let data = serde_json::json!([
            {"tag_name":"v1.2.3","published_at":"2026-10-22T00:00:00Z","body":"<script>alert(1)</script>","draft":false,"prerelease":false,"assets":[{"name":"nfqws2-ui-v1.2.3-aarch64-unknown-linux-musl.tar.gz"},{"name":"nfqws2-ui-v1.2.3-aarch64-unknown-linux-musl.tar.gz.sha256"}]},
            {"tag_name":"v1.2.4-rc1","draft":false,"prerelease":true,"assets":[{"name":"nfqws2-ui-v1.2.4-rc1-aarch64-unknown-linux-musl.tar.gz"},{"name":"nfqws2-ui-v1.2.4-rc1-aarch64-unknown-linux-musl.tar.gz.sha256"}]},
            {"tag_name":"v1.2.2","draft":false,"prerelease":false,"assets":[{"name":"nfqws2-ui-v1.2.2-aarch64-unknown-linux-musl.tar.gz"}]}
        ]);

        let releases = eligible_panel_releases(&data.to_string()).unwrap();

        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].tag, "v1.2.3");
        assert_eq!(releases[0].notes, "<script>alert(1)</script>");
    }

    #[test]
    fn release_catalog_rejects_invalid_or_oversized_metadata() {
        assert!(eligible_panel_releases("not json").is_err());
        assert!(eligible_panel_releases(&" ".repeat(MAX_RELEASE_RESPONSE_BYTES + 1)).is_err());
    }

    #[test]
    fn core_release_catalog_requires_stable_aarch64_package_and_github_digest() {
        let digest = "a".repeat(64);
        let data = serde_json::json!([
            {"tag_name":"v1.3.2","published_at":"2026-10-21T11:04:30Z","body":"nfqws2 release","draft":false,"prerelease":false,"assets":[
                {"name":"nfqws2-keenetic_1.3.2_aarch64-3.10.ipk","digest":format!("sha256:{digest}")},
                {"name":"nfqws2-keenetic_1.3.2_all_entware.ipk","digest":format!("sha256:{digest}")}
            ]},
            {"tag_name":"v1.3.3-rc1","draft":false,"prerelease":true,"assets":[]},
            {"tag_name":"v1.3.1","draft":false,"prerelease":false,"assets":[{"name":"nfqws2-keenetic_1.3.1_aarch64-3.10.ipk"}]}
        ]);
        let releases = eligible_core_releases(&data.to_string(), "aarch64-3.10").unwrap();
        assert_eq!(releases.len(), 1);
        assert_eq!(releases[0].tag, "v1.3.2");
        assert_eq!(releases[0].asset, "nfqws2-keenetic_1.3.2_aarch64-3.10.ipk");
        assert_eq!(releases[0].sha256, digest);
        assert!(eligible_core_releases(&data.to_string(), "mips-3.4").is_err());
    }

    #[test]
    fn installed_core_version_comes_from_opkg_metadata_only() {
        assert_eq!(
            installed_core_tag_from_output(b"Package: nfqws2-keenetic\nVersion: 1.3.2-1\n"),
            Some("v1.3.2".into())
        );
        assert_eq!(
            installed_core_tag_from_output(b"Package: nfqws2-keenetic\nStatus: not-installed\n"),
            None
        );
        assert_eq!(
            installed_core_tag_from_output(b"Version: 1.3.2; touch /tmp/nope\n"),
            None
        );
    }

    #[tokio::test]
    async fn status_is_available_without_a_login() {
        let dir = tempfile::tempdir().unwrap();
        let app = api_router(test_state(dir.path().to_path_buf()), dir.path().join("web"));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers().get("x-frame-options").unwrap(), "DENY");
        assert_eq!(
            response.headers().get("x-content-type-options").unwrap(),
            "nosniff"
        );
    }

    #[tokio::test]
    async fn ui_update_status_is_readable_without_triggering_an_update() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state(dir.path().join("data"));
        *state.ui_update_status.write().unwrap() = UiUpdateStatus {
            current_version: "0.3.4".into(),
            latest_version: Some("v0.3.5".into()),
            update_available: true,
            release_url: Some("https://github.com/rangerxxyy/nfqws2-ui/releases/tag/v0.3.5".into()),
            checked_at: Some(123),
        };
        let app = api_router(state, dir.path().join("web"));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/updates/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let bytes = to_bytes(response.into_body(), MAX_CONFIG_BYTES)
            .await
            .unwrap();
        let payload: UiUpdateStatus = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(payload.latest_version.as_deref(), Some("v0.3.5"));
        assert!(payload.update_available);
        assert_eq!(payload.checked_at, Some(123));
    }

    #[tokio::test]
    async fn root_serves_the_typescript_bundle_shell_without_an_html_file() {
        let dir = tempfile::tempdir().unwrap();
        let web_root = dir.path().join("web");
        tokio::fs::create_dir_all(&web_root).await.unwrap();
        tokio::fs::write(
            web_root.join("nfqws2-ui.js"),
            "window.__panel_bundle = true;",
        )
        .await
        .unwrap();
        tokio::fs::write(web_root.join("nfqws2-ui.css"), "body { color: white; }")
            .await
            .unwrap();
        let app = api_router(test_state(dir.path().to_path_buf()), web_root);
        let response = app
            .clone()
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(response.headers()["cache-control"], "no-store");
        let html = String::from_utf8(
            to_bytes(response.into_body(), usize::MAX)
                .await
                .unwrap()
                .to_vec(),
        )
        .unwrap();
        assert!(html.contains("id=\"root\""));
        assert!(html.contains("/nfqws2-ui.js"));
        assert!(html.contains("/nfqws2-ui.css"));
        assert!(!html.contains("tabContent-conf"));

        let bundle = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/nfqws2-ui.js")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(bundle.status(), StatusCode::OK);
        assert_eq!(
            String::from_utf8(
                to_bytes(bundle.into_body(), usize::MAX)
                    .await
                    .unwrap()
                    .to_vec()
            )
            .unwrap(),
            "window.__panel_bundle = true;"
        );

        let stylesheet = app
            .oneshot(
                Request::builder()
                    .uri("/nfqws2-ui.css")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(stylesheet.status(), StatusCode::OK);
        assert_eq!(
            String::from_utf8(
                to_bytes(stylesheet.into_body(), usize::MAX)
                    .await
                    .unwrap()
                    .to_vec()
            )
            .unwrap(),
            "body { color: white; }"
        );
    }

    #[tokio::test]
    async fn runtime_endpoint_reports_process_snapshot_without_returning_raw_argv() {
        let dir = tempfile::tempdir().unwrap();
        let app = api_router(
            test_state(dir.path().to_path_buf()),
            dir.path().to_path_buf(),
        );
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/nfqws2/runtime")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert!(
            body.get("running")
                .and_then(serde_json::Value::as_bool)
                .is_some()
        );
        assert_eq!(body["source"], "proc-cmdline");
        assert!(body["profiles"].is_array());
        assert!(body.get("arguments").is_none());
    }

    #[tokio::test]
    async fn config_read_and_write_use_the_selected_installation_path() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("custom/nfqws2.conf");
        tokio::fs::create_dir_all(config.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::write(&config, "NFQWS_ARGS=--hostlist=lists/domains.list\n")
            .await
            .unwrap();
        let state = test_state_with_config(dir.path().join("state"), Some(config.clone()));
        let app = api_router(state, dir.path().join("web"));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/config?file=nfqws2.conf")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let loaded = body_json(response).await;
        let revision = loaded["revision"].as_str().unwrap().to_string();
        assert_eq!(
            loaded["content"],
            "NFQWS_ARGS=--hostlist=lists/domains.list\n"
        );

        let saved = app.oneshot(
            Request::builder()
                .method("PUT")
                .uri("/api/config?file=nfqws2.conf")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({ "content": "NFQWS_ARGS=--hostlist=lists/changed.list\n", "revision": revision }).to_string()))
                .unwrap(),
        ).await.unwrap();
        assert_eq!(saved.status(), StatusCode::OK);
        assert_eq!(
            tokio::fs::read_to_string(config).await.unwrap(),
            "NFQWS_ARGS=--hostlist=lists/changed.list\n"
        );
    }

    #[tokio::test]
    async fn config_apply_rolls_back_and_restarts_when_new_config_fails() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let original = b"NFQWS_ARGS=--filter-tcp=443\n";
        let replacement = "NFQWS_ARGS=--unsupported-option\n";
        tokio::fs::write(&config, original).await.unwrap();
        let state = test_state_with_config(dir.path().join("state"), Some(config.clone()));
        let restart_count = Arc::new(AtomicUsize::new(0));
        let restart_counter = restart_count.clone();

        let result = apply_config_transaction(
            &state,
            ConfigWrite {
                content: replacement.into(),
                revision: revision(original),
            },
            move || {
                let attempt = restart_counter.fetch_add(1, Ordering::SeqCst);
                async move {
                    if attempt == 0 {
                        Err("fake invalid-config restart failure".to_string())
                    } else {
                        Ok("old config service healthy".to_string())
                    }
                }
            },
        )
        .await;

        let error = result.unwrap_err();
        assert!(error.contains("Исходный конфиг восстановлен и служба перезапущена"));
        assert_eq!(restart_count.load(Ordering::SeqCst), 2);
        assert_eq!(tokio::fs::read(config).await.unwrap(), original);
    }

    #[tokio::test]
    async fn config_apply_returns_success_only_after_restart_callback_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let original = b"NFQWS_ARGS=--filter-tcp=443\n";
        let replacement = "NFQWS_ARGS=--filter-tcp=80,443\n";
        tokio::fs::write(&config, original).await.unwrap();
        let state = test_state_with_config(dir.path().join("state"), Some(config.clone()));

        let result = apply_config_transaction(
            &state,
            ConfigWrite {
                content: replacement.into(),
                revision: revision(original),
            },
            || async { Ok("restart confirmed".to_string()) },
        )
        .await
        .unwrap();

        assert_eq!(result.config.content, replacement);
        assert_eq!(result.message, "restart confirmed");
        assert_eq!(
            tokio::fs::read(config).await.unwrap(),
            replacement.as_bytes()
        );
    }

    #[tokio::test]
    async fn config_apply_rejects_stale_revision_without_calling_service() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let original = b"NFQWS_ARGS=--filter-tcp=443\n";
        tokio::fs::write(&config, original).await.unwrap();
        let state = test_state_with_config(dir.path().join("state"), Some(config.clone()));
        let called = Arc::new(AtomicBool::new(false));
        let called_by_restart = called.clone();

        let result = apply_config_transaction(
            &state,
            ConfigWrite {
                content: "NFQWS_ARGS=--filter-tcp=80\n".into(),
                revision: "stale".into(),
            },
            move || {
                called_by_restart.store(true, Ordering::SeqCst);
                async { Ok("unexpected restart".to_string()) }
            },
        )
        .await;

        assert!(result.unwrap_err().contains("изменился после загрузки"));
        assert!(!called.load(Ordering::SeqCst));
        assert_eq!(tokio::fs::read(config).await.unwrap(), original);
    }

    #[tokio::test]
    async fn installation_endpoint_reports_real_paths_and_configured_lists() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path().join("etc/nfqws2");
        let config = root.join("nfqws2.conf");
        let list = root.join("custom/domains.list");
        let lua = root.join("lua/custom.lua");
        let blob = root.join("blobs/tls.bin");
        tokio::fs::create_dir_all(list.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::create_dir_all(lua.parent().unwrap())
            .await
            .unwrap();
        tokio::fs::create_dir_all(blob.parent().unwrap())
            .await
            .unwrap();
        let config_text = format!(
            "NFQWS_ARGS=\"--hostlist=custom/domains.list --lua-init=@{} --blob=tls_clienthello:@{}\"\n",
            lua.display(),
            blob.display(),
        );
        tokio::fs::write(&config, config_text).await.unwrap();
        tokio::fs::write(&list, "example.org\n").await.unwrap();
        tokio::fs::write(&lua, "-- module\n").await.unwrap();
        tokio::fs::write(&blob, b"blob").await.unwrap();
        let state = test_state_with_config(dir.path().join("state"), Some(config.clone()));
        let app = api_router(state, dir.path().join("web"));

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/nfqws2/installation")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let report = body_json(response).await;
        assert_eq!(report["platform"], "unknown");
        assert!(report["networkInterfaces"].is_array());
        assert_eq!(report["config"]["path"], config.display().to_string());
        assert_eq!(report["config"]["exists"], true);
        assert_eq!(report["config"]["usedByActiveConfig"], true);
        assert_eq!(report["lists"].as_array().unwrap().len(), 1);
        assert_eq!(
            report["lists"][0]["path"],
            list.canonicalize().unwrap().display().to_string()
        );
        assert_eq!(report["lists"][0]["usedByActiveConfig"], false);
        assert_eq!(report["lists"][0]["readable"], true);
        assert_eq!(report["moduleReferences"].as_array().unwrap().len(), 2);
        assert_eq!(report["moduleReferences"][0]["kind"], "lua");
        assert_eq!(
            report["moduleReferences"][0]["resolvedPath"],
            lua.display().to_string()
        );
        assert_eq!(report["moduleReferences"][0]["readable"], true);
        assert_eq!(report["moduleReferences"][1]["kind"], "blob");
        assert_eq!(report["moduleReferences"][1]["exists"], true);
    }

    #[test]
    fn module_reference_discovery_checks_literal_files_without_evaluating_shell() {
        let dir = tempfile::tempdir().unwrap();
        let lua = dir.path().join("lua/custom.lua");
        let lua_gzip = PathBuf::from(format!("{}.gz", lua.display()));
        let blob = dir.path().join("blobs/tls.bin");
        std::fs::create_dir_all(lua.parent().unwrap()).unwrap();
        std::fs::create_dir_all(blob.parent().unwrap()).unwrap();
        std::fs::write(&lua_gzip, b"compressed").unwrap();
        std::fs::write(&blob, b"blob").unwrap();
        let config = format!(
            "NFQWS_ARGS=\"--lua-init=@{} --blob=tls_clienthello:+1@{}\"\nMODE_X=\"--lua-init=@$DYNAMIC_LUA\"\nNFQWS_EXTRA_ARGS=\"--blob=relative:@blobs/other.bin\"\n# --lua-init=@/commented/out.lua\nNFQWS_ARGS_CUSTOM=\"--lua-init=print(1) --blob=inline:0x1234\"\n",
            lua.display(),
            blob.display(),
        );

        let references = module_references(&config);

        assert_eq!(references.len(), 4);
        assert_eq!(references[0].kind, "lua");
        assert_eq!(references[0].path.as_deref(), Some(lua.to_str().unwrap()));
        assert_eq!(
            references[0].resolved_path.as_deref(),
            Some(lua_gzip.to_str().unwrap())
        );
        assert!(references[0].exists);
        assert!(references[0].readable);
        assert_eq!(references[1].kind, "blob");
        assert_eq!(references[1].path.as_deref(), Some(blob.to_str().unwrap()));
        assert!(references[1].exists);
        assert_eq!(references[2].resolution, "dynamic");
        assert_eq!(references[2].resolved_path, None);
        assert!(!references[2].exists);
        assert_eq!(references[3].resolution, "relative_unknown");
        assert_eq!(references[3].resolved_path, None);
        assert!(!references[3].exists);
    }

    #[test]
    fn active_module_catalog_handles_separate_lua_and_blob_arguments() {
        use flate2::{Compression, write::GzEncoder};

        let dir = tempfile::tempdir().unwrap();
        let lua_dir = dir.path().join("lua");
        std::fs::create_dir_all(&lua_dir).unwrap();
        let lua_path = lua_dir.join("custom.lua.gz");
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        std::io::Write::write_all(&mut encoder, b"function custom_fake(ctx, desync)\n").unwrap();
        std::fs::write(&lua_path, encoder.finish().unwrap()).unwrap();
        let args = vec![
            "nfqws2".into(),
            "--lua-init".into(),
            "@custom.lua".into(),
            "--blob".into(),
            "tls_clienthello:@/opt/share/tls.bin".into(),
        ];
        let functions = active_lua_functions(&args, &lua_dir);
        let blobs = active_blob_names(&args);
        assert!(functions.contains("custom_fake"));
        assert!(blobs.contains(&"tls_clienthello".into()));
    }

    #[tokio::test]
    async fn create_backup_endpoint_snapshots_the_config_without_rewriting_it() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let original = b"NFQWS_ARGS=--hostlist=lists/user.list\n";
        tokio::fs::write(&config, original).await.unwrap();
        let state = test_state_with_config(dir.path().join("state"), Some(config.clone()));
        let app = api_router(state, dir.path().join("web"));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/backups")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let backup = body_json(response).await;
        assert_eq!(backup["file"], "nfqws2.conf");
        let id = backup["id"].as_str().unwrap();
        assert!(id.starts_with("nfqws2.conf."));
        assert_eq!(tokio::fs::read(config).await.unwrap(), original);
        assert_eq!(
            tokio::fs::read(dir.path().join("state/backups").join(id))
                .await
                .unwrap(),
            original
        );
    }

    #[tokio::test]
    async fn restore_backup_restores_selected_config_and_keeps_pre_restore_copy() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let original = b"NFQWS_ARGS=--hostlist=lists/original.list\n";
        let changed = b"NFQWS_ARGS=--hostlist=lists/changed.list\n";
        tokio::fs::write(&config, original).await.unwrap();
        let state = test_state_with_config(dir.path().join("state"), Some(config.clone()));
        let app = api_router(state, dir.path().join("web"));

        let created = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/backups")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(created.status(), StatusCode::CREATED);
        let backup = body_json(created).await;
        let id = backup["id"].as_str().unwrap();
        tokio::fs::write(&config, changed).await.unwrap();

        let restored = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/backups/{id}/restore"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(restored.status(), StatusCode::OK);
        assert_eq!(tokio::fs::read(&config).await.unwrap(), original);
        let backups_dir = dir.path().join("state/backups");
        let mut entries = tokio::fs::read_dir(backups_dir).await.unwrap();
        let mut saved_changed_version = false;
        while let Some(entry) = entries.next_entry().await.unwrap() {
            if entry.file_name() != id {
                saved_changed_version |= tokio::fs::read(entry.path()).await.unwrap() == changed;
            }
        }
        assert!(
            saved_changed_version,
            "pre-restore config must be retained as a backup"
        );
    }

    #[tokio::test]
    async fn site_diagnosis_rejects_local_or_injected_targets_before_any_network_probe() {
        let dir = tempfile::tempdir().unwrap();
        let app = api_router(test_state(dir.path().join("data")), dir.path().join("web"));
        for host in [
            "127.0.0.1",
            "router.local",
            "x.com;reboot",
            "http://user:pass@x.com",
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/nfqws2/diagnostics/site")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::json!({"host":host}).to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            assert!(body_json(response).await["error"].is_string());
        }
    }

    #[tokio::test]
    async fn site_trace_rejects_local_or_injected_targets_before_firewall_access() {
        let dir = tempfile::tempdir().unwrap();
        let app = api_router(test_state(dir.path().join("data")), dir.path().join("web"));
        for host in [
            "127.0.0.1",
            "router.local",
            "x.com;reboot",
            "http://user:pass@x.com",
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/nfqws2/diagnostics/trace")
                        .header("content-type", "application/json")
                        .body(Body::from(serde_json::json!({"host":host}).to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            assert!(body_json(response).await["error"].is_string());
        }
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/nfqws2/diagnostics/trace")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"host":"example.com","proto":"quic"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(
            body_json(response).await["error"]
                .as_str()
                .unwrap()
                .contains("HTTP и HTTPS")
        );
    }

    #[tokio::test]
    async fn site_trace_invalid_request_does_not_create_operation_storage() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let app = api_router(test_state(data.clone()), dir.path().join("web"));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/nfqws2/diagnostics/trace")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"host":"127.0.0.1"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(!data.join("site-trace-operations").exists());
    }

    #[tokio::test]
    async fn site_trace_unknown_operation_status_and_stop_are_not_found() {
        let dir = tempfile::tempdir().unwrap();
        let app = api_router(test_state(dir.path().join("data")), dir.path().join("web"));
        for (method, uri) in [
            (
                "GET",
                "/api/nfqws2/diagnostics/trace/operations/0123456789abcdef0123456789abcdef",
            ),
            (
                "POST",
                "/api/nfqws2/diagnostics/trace/operations/0123456789abcdef0123456789abcdef/stop",
            ),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(uri)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
            assert!(body_json(response).await["error"].is_string());
        }
    }

    #[tokio::test]
    async fn transfer_diagnostic_rejects_invalid_targets_and_unknown_operations() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let app = api_router(test_state(data.clone()), dir.path().join("web"));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/nfqws2/diagnostics/transfer")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"host":"127.0.0.1"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(!data.join("transfer-diagnostic-operations").exists());
        for uri in [
            "/api/nfqws2/diagnostics/transfer/operations/0123456789abcdef0123456789abcdef",
            "/api/nfqws2/diagnostics/transfer/operations/0123456789abcdef0123456789abcdef/cancel",
        ] {
            let method = if uri.ends_with("/cancel") {
                "POST"
            } else {
                "GET"
            };
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method(method)
                        .uri(uri)
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }
    }

    #[tokio::test]
    async fn freeze_scan_rejects_invalid_hosts_without_scheduling_router_probes() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let app = api_router(test_state(data.clone()), dir.path().join("web"));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/nfqws2/diagnostics/freeze-scan")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"extraHosts":["127.0.0.1"]}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(!data.join("freeze-diagnostic-operations").exists());
    }

    #[tokio::test]
    async fn freeze_scan_status_reads_a_bounded_persisted_operation() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let directory = data.join("freeze-diagnostic-operations");
        let id = "abcdef0123456789abcdef0123456789";
        write_freeze_scan_operation(
            &directory,
            &FreezeScanOperation {
                id: id.into(),
                state: "partial".into(),
                stage: "partial".into(),
                current: None,
                done: 19,
                total: 21,
                results: Vec::new(),
                message: "done".into(),
                updated_at: 12,
            },
        )
        .unwrap();
        let app = api_router(test_state(data), dir.path().join("web"));
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/nfqws2/diagnostics/freeze-scan/operations/{id}"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let operation = body_json(response).await;
        assert_eq!(operation["done"], 19);
        assert_eq!(operation["total"], 21);
        assert_eq!(operation["state"], "partial");
    }

    #[test]
    fn freeze_scan_progress_persists_each_completed_target_row() {
        let mut operation = FreezeScanOperation {
            id: "abcdef0123456789abcdef0123456789".into(),
            state: "running".into(),
            stage: "baseline".into(),
            current: None,
            done: 0,
            total: 20,
            results: Vec::new(),
            message: String::new(),
            updated_at: 0,
        };
        let completed = freeze_diagnostic::FreezeNetworkResult {
            label: "Hetzner".into(),
            host: "hetzner.com".into(),
            path: "/large.bin".into(),
            target_kind: freeze_diagnostic::FreezeTargetKind::HostReference,
            address: Some(std::net::Ipv4Addr::new(91, 98, 156, 82)),
            asn_hint: Some(24940),
            asn: freeze_diagnostic::AsnVerification::Verified { asn: 24940 },
            direct: freeze_diagnostic::FreezeMeasurement {
                classification: "Complete".into(),
                ok: true,
                bytes_received: Some(65_536),
                replies: None,
                elapsed_ms: 75,
                reason: "HTTP 206".into(),
            },
            current: freeze_diagnostic::FreezeMeasurement {
                classification: "Complete".into(),
                ok: true,
                bytes_received: Some(65_536),
                replies: None,
                elapsed_ms: 80,
                reason: "HTTP 206".into(),
            },
            freeze_detected: false,
            candidate_total: 0,
            tried_candidates: 0,
            found_strategy: None,
            candidate_attempts: Vec::new(),
        };

        update_freeze_scan_progress(
            &mut operation,
            1,
            20,
            "Hetzner",
            std::slice::from_ref(&completed),
        );

        assert_eq!(operation.done, 1);
        assert_eq!(operation.results.len(), 1);
        assert_eq!(operation.results[0].host, "hetzner.com");
        assert_eq!(operation.results[0].address, completed.address);
        assert_eq!(operation.results[0].asn, completed.asn);
    }

    #[tokio::test]
    async fn picker_reuses_confirmed_strategy_from_latest_completed_freeze_scan() {
        let dir = tempfile::tempdir().unwrap();
        let directory = dir.path().join("freeze-diagnostic-operations");
        let result = |label: &str, step: &str| freeze_diagnostic::FreezeNetworkResult {
            label: label.into(),
            host: format!("{label}.example"),
            path: "/large.bin".into(),
            target_kind: freeze_diagnostic::FreezeTargetKind::PinnedAddress,
            address: Some(std::net::Ipv4Addr::new(203, 0, 113, 1)),
            asn_hint: None,
            asn: freeze_diagnostic::AsnVerification::Unavailable,
            direct: freeze_diagnostic::FreezeMeasurement {
                classification: "PossibleFreeze".into(),
                ok: false,
                bytes_received: Some(18_000),
                replies: None,
                elapsed_ms: 4_000,
                reason: "повторяемый тайм-аут после начала ответа".into(),
            },
            current: freeze_diagnostic::FreezeMeasurement {
                classification: "PossibleFreeze".into(),
                ok: false,
                bytes_received: Some(18_000),
                replies: None,
                elapsed_ms: 4_000,
                reason: "повторяемый тайм-аут после начала ответа".into(),
            },
            freeze_detected: true,
            candidate_total: 1,
            tried_candidates: 1,
            found_strategy: Some(freeze_diagnostic::FreezeStrategyMatch {
                sni: "ya.ru".into(),
                step: step.into(),
                attempts: 3,
                successes: 3,
                average_ms: Some(120),
                reused_from: None,
            }),
            candidate_attempts: Vec::new(),
        };
        let save = |id: &str, state: &str, updated_at, label: &str, step: &str| {
            write_freeze_scan_operation(
                &directory,
                &FreezeScanOperation {
                    id: id.into(),
                    state: state.into(),
                    stage: state.into(),
                    current: None,
                    done: 1,
                    total: 1,
                    results: vec![result(label, step)],
                    message: state.into(),
                    updated_at,
                },
            )
            .unwrap();
        };
        save(
            "11111111111111111111111111111111",
            "complete",
            10,
            "older",
            "--lua-desync=hostfakesplit:host=old.example",
        );
        save(
            "22222222222222222222222222222222",
            "partial",
            20,
            "newer",
            "--lua-desync=hostfakesplit:host=new.example",
        );
        save(
            "33333333333333333333333333333333",
            "running",
            30,
            "active",
            "--lua-desync=hostfakesplit:host=active.example",
        );
        let candidates = load_confirmed_freeze_candidates(&directory).await;
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].name, "Обрыв на 16 КБ · ya.ru · newer");
        assert_eq!(
            candidates[0].steps,
            ["--lua-desync=hostfakesplit:host=new.example"]
        );
    }

    #[tokio::test]
    async fn strategy_pick_cancel_persists_stopping_and_signals_owned_task() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let operation_dir = data.join("strategy-pick-operations");
        let id = "0123456789abcdef0123456789abcdef";
        let operation = StrategyPickOperation {
            id: id.into(),
            host: "example.com".into(),
            protocol: "https".into(),
            state: "running".into(),
            stage: "testing".into(),
            current: Some("multisplit".into()),
            done: 0,
            total: 1,
            results: Vec::new(),
            message: "running".into(),
            updated_at: now_unix_seconds(),
        };
        write_strategy_pick_operation_sync(&operation_dir, &operation).unwrap();
        let state = test_state(data);
        let cancel = Arc::new(AtomicBool::new(false));
        state
            .strategy_pick_controls
            .lock()
            .await
            .insert(id.into(), cancel.clone());
        let app = api_router(state, dir.path().join("web"));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/nfqws2/strategies/pick/operations/{id}/cancel"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
        assert!(cancel.load(Ordering::Acquire));
        let saved = read_strategy_pick_operation(&operation_dir, id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(saved.state, "stopping");
        assert_eq!(saved.stage, "cleanup");
    }

    #[tokio::test]
    async fn transfer_diagnostic_cancel_persists_stopping_and_signals_owned_task() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let operation_dir = data.join("transfer-diagnostic-operations");
        let id = "abcdef0123456789abcdef0123456789";
        write_transfer_operation(
            &operation_dir,
            &TransferDiagnosticOperation {
                id: id.into(),
                host: "example.com".into(),
                state: "running".into(),
                stage: "direct".into(),
                attempt: 1,
                message: "running".into(),
                result: None,
                updated_at: now_unix_seconds(),
            },
        )
        .unwrap();
        let state = test_state(data);
        let cancel = Arc::new(AtomicBool::new(false));
        state
            .transfer_diagnostic_controls
            .lock()
            .await
            .insert(id.into(), cancel.clone());
        let app = api_router(state, dir.path().join("web"));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/nfqws2/diagnostics/transfer/operations/{id}/cancel"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
        assert!(cancel.load(Ordering::Acquire));
        let saved = read_transfer_operation(&operation_dir, id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(saved.state, "stopping");
        assert_eq!(saved.stage, "cleanup");
    }

    #[tokio::test]
    async fn strategy_pick_history_returns_bounded_terminal_operations_newest_first() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let operation_dir = data.join("strategy-pick-operations");
        for (index, state, host, updated_at) in [
            (1, "complete", "older.example", 10),
            (2, "no_match", "newer.example", 20),
            (3, "running", "active.example", 30),
        ] {
            write_strategy_pick_operation_sync(
                &operation_dir,
                &StrategyPickOperation {
                    id: format!("{index:032x}"),
                    host: host.into(),
                    protocol: "https".into(),
                    state: state.into(),
                    stage: state.into(),
                    current: None,
                    done: 0,
                    total: 1,
                    results: Vec::new(),
                    message: state.into(),
                    updated_at,
                },
            )
            .unwrap();
        }
        let app = api_router(test_state(data), dir.path().join("web"));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/nfqws2/strategies/pick/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let history = body_json(response).await;
        assert_eq!(history.as_array().unwrap().len(), 2);
        assert_eq!(history[0]["host"], "newer.example");
        assert_eq!(history[1]["host"], "older.example");
    }

    #[tokio::test]
    async fn strategy_picker_refuses_to_schedule_without_a_verified_live_nfqws2_process() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let app = api_router(test_state(data.clone()), dir.path().join("web"));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/nfqws2/strategies/pick")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"host":"example.com","protocol":"https","mode":"quick"})
                            .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert!(!data.join("strategy-pick-operations").exists());
    }

    #[tokio::test]
    async fn strategy_picker_rejects_repeat_counts_outside_the_bounded_range() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let app = api_router(test_state(data.clone()), dir.path().join("web"));
        for repeats in [1, 6] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/nfqws2/strategies/pick")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            serde_json::json!({"host":"example.com","repeats":repeats}).to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        }
        assert!(!data.join("strategy-pick-operations").exists());
    }

    #[tokio::test]
    async fn full_mode_is_accepted_and_unknown_modes_and_quic_runs_are_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let app = api_router(test_state(data.clone()), dir.path().join("web"));
        for (mode, status) in [
            ("full", StatusCode::CONFLICT),
            ("not-a-mode", StatusCode::BAD_REQUEST),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/nfqws2/strategies/pick")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            serde_json::json!({"host":"example.com","mode":mode}).to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), status);
        }
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/nfqws2/strategies/pick")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"host":"example.com","mode":"custom"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/nfqws2/strategies/pick")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"host":"example.com","protocol":"quic"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
        assert!(!data.join("strategy-pick-operations").exists());
    }

    #[tokio::test]
    async fn transfer_diagnostic_refuses_to_run_while_another_site_operation_owns_the_lock() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let state = test_state(data.clone());
        let _guard = state.site_diagnostic_lock.clone().lock_owned().await;
        let app = api_router(state, dir.path().join("web"));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/nfqws2/diagnostics/transfer")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"host":"example.com"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert!(!data.join("transfer-diagnostic-operations").exists());
    }

    #[tokio::test]
    async fn persisted_running_transfer_is_marked_interrupted_after_backend_restart() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let directory = data.join("transfer-diagnostic-operations");
        let id = "0123456789abcdef0123456789abcdef";
        write_transfer_operation(
            &directory,
            &TransferDiagnosticOperation {
                id: id.into(),
                host: "example.com".into(),
                state: "running".into(),
                stage: "direct".into(),
                attempt: 1,
                message: "Проверяется".into(),
                result: None,
                updated_at: 123,
            },
        )
        .unwrap();
        let app = api_router(test_state(data), dir.path().join("web"));
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/nfqws2/diagnostics/transfer/operations/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["state"], "failed");
        assert_eq!(body["stage"], "interrupted");
    }

    #[tokio::test]
    async fn site_trace_rejects_a_concurrent_diagnostic_before_creating_an_operation() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let state = test_state(data.clone());
        let _guard = state.site_diagnostic_lock.clone().lock_owned().await;
        let app = api_router(state, dir.path().join("web"));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/nfqws2/diagnostics/trace")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"host":"example.com"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CONFLICT);
        assert!(!data.join("site-trace-operations").exists());
    }

    #[tokio::test]
    async fn config_preview_preserves_draft_and_never_writes_the_device_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let original = "# сохраняем\nISP_INTERFACE=\"wan\n eth2\" # comment\nUNKNOWN='keep'\n";
        tokio::fs::write(&config, original).await.unwrap();
        let app = api_router(
            test_state_with_config(dir.path().join("state"), Some(config.clone())),
            dir.path().join("web"),
        );
        let response = app.clone().oneshot(Request::builder().method("POST")
            .uri("/api/nfqws2/config/preview").header("content-type", "application/json")
            .body(Body::from(serde_json::json!({"content": original, "key": "ISP_INTERFACE", "value": "eth3"}).to_string())).unwrap()).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let result = body_json(response).await;
        assert_eq!(
            result["content"],
            original.replace("\"wan\n eth2\"", "\"eth3\"")
        );
        let missing_group = app.clone().oneshot(Request::builder().method("POST")
            .uri("/api/nfqws2/config/preview").header("content-type", "application/json")
            .body(Body::from(serde_json::json!({"content": original, "key": "NFQWS_ARGS_UDP", "value": "--filter-udp=443 --payload=quic_initial"}).to_string())).unwrap()).await.unwrap();
        assert_eq!(missing_group.status(), StatusCode::OK);
        assert!(
            body_json(missing_group).await["content"]
                .as_str()
                .unwrap()
                .contains("NFQWS_ARGS_UDP=\"--filter-udp=443 --payload=quic_initial\"")
        );
        assert_eq!(tokio::fs::read_to_string(&config).await.unwrap(), original);
        for (content, key, value, expected) in [
            (
                "ISP_INTERFACE=wan\nISP_INTERFACE=eth0\n",
                "ISP_INTERFACE",
                "eth3",
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            ("UNKNOWN=one\n", "UNKNOWN", "two", StatusCode::BAD_REQUEST),
            (
                "NFQWS_EXTRA_ARGS='$MODE_LIST'\n",
                "NFQWS_EXTRA_ARGS",
                "$MODE_AUTO",
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
            (
                "ISP_INTERFACE=$(id)\n",
                "ISP_INTERFACE",
                "eth3",
                StatusCode::UNPROCESSABLE_ENTITY,
            ),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/nfqws2/config/preview")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            serde_json::json!({"content": content, "key": key, "value": value})
                                .to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), expected);
            assert_eq!(tokio::fs::read_to_string(&config).await.unwrap(), original);
        }
    }

    #[tokio::test]
    async fn config_validation_endpoint_reports_static_errors_without_writing_or_restarting() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("nfqws2.conf");
        tokio::fs::write(&config_path, "NFQWS_ARGS=\"--filter-tcp=443\"\n")
            .await
            .unwrap();
        let app = api_router(
            test_state_with_config(dir.path().to_path_buf(), Some(config_path.clone())),
            dir.path().into(),
        );
        let response = app
            .oneshot(
                Request::post("/api/nfqws2/config/validate")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        r#"{"content":"NFQWS_ARGS=\"--filter-tcp=443 --new\"\\n"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let report = body_json(response).await;
        assert_eq!(report["valid"], false);
        assert_eq!(report["checks"]["parser"], "passed");
        assert_eq!(report["checks"]["installedBinary"], "not_run");
        assert_eq!(
            report["diagnostics"][0]["code"],
            "profile_delimiter_location"
        );
        assert_eq!(
            tokio::fs::read_to_string(&config_path).await.unwrap(),
            "NFQWS_ARGS=\"--filter-tcp=443\"\n"
        );
        assert!(!dir.path().join("backups").exists());
    }

    #[tokio::test]
    async fn package_user_list_without_config_reference_can_be_read_saved_and_reread() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let list_dir = dir.path().join("lists");
        tokio::fs::create_dir_all(&list_dir).await.unwrap();
        tokio::fs::write(&config, "NFQWS_EXTRA_ARGS=\"$MODE_ALL\"\n")
            .await
            .unwrap();
        tokio::fs::write(list_dir.join("user.list"), "example.org\n")
            .await
            .unwrap();
        let state = test_state_with_config(dir.path().join("state"), Some(config));
        let app = api_router(state, dir.path().join("web"));
        let loaded = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/nfqws2/lists/user.list")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(loaded.status(), StatusCode::OK);
        let loaded = body_json(loaded).await;
        assert_eq!(loaded["content"], "example.org\n");
        let revision = loaded["revision"].as_str().unwrap();
        let invalid = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/nfqws2/lists/user.list")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"content":"bad domain..\n","revision":"{revision}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            tokio::fs::read_to_string(list_dir.join("user.list"))
                .await
                .unwrap(),
            "example.org\n"
        );
        let save = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/nfqws2/lists/user.list")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"content":"example.org\nnew.example.org\n","revision":"{revision}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(save.status(), StatusCode::OK);
        let saved = body_json(save).await;
        assert_eq!(saved["content"], "example.org\nnew.example.org\n");
        assert_eq!(
            tokio::fs::read_to_string(list_dir.join("user.list"))
                .await
                .unwrap(),
            "example.org\nnew.example.org\n"
        );
        let mut backups = tokio::fs::read_dir(dir.path().join("state/backups"))
            .await
            .unwrap();
        assert!(backups.next_entry().await.unwrap().is_some());

        let conflict = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/nfqws2/lists/user.list")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"content":"stale.example.org\n","revision":"{revision}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(conflict.status(), StatusCode::CONFLICT);
        assert_eq!(
            tokio::fs::read_to_string(list_dir.join("user.list"))
                .await
                .unwrap(),
            "example.org\nnew.example.org\n"
        );
    }

    #[tokio::test]
    async fn list_index_returns_every_existing_file_referenced_by_active_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let list_dir = dir.path().join("lists");
        tokio::fs::create_dir_all(&list_dir).await.unwrap();
        tokio::fs::write(
            &config,
            "NFQWS_ARGS=--hostlist=lists/user.list --hostlist-auto=lists/auto.list --hostlist-exclude=lists/exclude.list --ipset=lists/ipset.list --ipset-exclude=lists/ipset_exclude.list\n",
        )
        .await
        .unwrap();
        for id in LIST_IDS {
            tokio::fs::write(list_dir.join(id), "sample\n")
                .await
                .unwrap();
        }
        let app = api_router(
            test_state_with_config(dir.path().join("state"), Some(config)),
            dir.path().join("web"),
        );
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/nfqws2/lists")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let report = body_json(response).await;
        let ids = report["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|file| file["id"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(ids, LIST_IDS);
    }

    #[tokio::test]
    async fn list_index_includes_custom_list_files_from_the_lists_directory() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let list_dir = dir.path().join("lists");
        tokio::fs::create_dir_all(&list_dir).await.unwrap();
        tokio::fs::write(&config, "NFQWS_EXTRA_ARGS=--hostlist=lists/user.list\n")
            .await
            .unwrap();
        tokio::fs::write(list_dir.join("user.list"), "example.org\n")
            .await
            .unwrap();
        tokio::fs::write(list_dir.join("google.list"), "google.com\n")
            .await
            .unwrap();
        tokio::fs::create_dir_all(list_dir.join("nested"))
            .await
            .unwrap();
        tokio::fs::write(list_dir.join("nested/ignored.list"), "ignored.example\n")
            .await
            .unwrap();
        tokio::fs::write(list_dir.join("notes.txt"), "ignored.example\n")
            .await
            .unwrap();

        let app = api_router(
            test_state_with_config(dir.path().join("state"), Some(config)),
            dir.path().join("web"),
        );
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/nfqws2/lists")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        let report = body_json(response).await;
        let ids = report["files"]
            .as_array()
            .unwrap()
            .iter()
            .map(|file| file["id"].as_str().unwrap())
            .collect::<Vec<_>>();
        assert!(ids.contains(&"google.list"));
        assert!(!ids.contains(&"nested/ignored.list"));
        assert!(!ids.contains(&"notes.txt"));
    }

    #[tokio::test]
    async fn custom_list_can_be_read_and_saved_with_revision_protection() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let list_dir = dir.path().join("lists");
        tokio::fs::create_dir_all(&list_dir).await.unwrap();
        tokio::fs::write(&config, "NFQWS_EXTRA_ARGS=--hostlist=lists/user.list\n")
            .await
            .unwrap();
        let custom_path = list_dir.join("google.list");
        tokio::fs::write(&custom_path, "google.com\n")
            .await
            .unwrap();
        let app = api_router(
            test_state_with_config(dir.path().join("state"), Some(config)),
            dir.path().join("web"),
        );

        let loaded = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/nfqws2/lists/google.list")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(loaded.status(), StatusCode::OK);
        let loaded = body_json(loaded).await;
        assert_eq!(loaded["id"], "google.list");
        assert_eq!(loaded["content"], "google.com\n");
        let revision = loaded["revision"].as_str().unwrap();

        let saved = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/nfqws2/lists/google.list")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"content":"google.com\nwww.google.com\n","revision":"{revision}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(saved.status(), StatusCode::OK);
        assert_eq!(
            tokio::fs::read_to_string(custom_path).await.unwrap(),
            "google.com\nwww.google.com\n"
        );
    }

    #[tokio::test]
    async fn custom_list_delete_is_revision_checked_and_rejects_standard_lists() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let list_dir = dir.path().join("lists");
        tokio::fs::create_dir_all(&list_dir).await.unwrap();
        tokio::fs::write(&config, "NFQWS_EXTRA_ARGS=$MODE_LIST\n")
            .await
            .unwrap();
        tokio::fs::write(list_dir.join("google.list"), "google.com\n")
            .await
            .unwrap();
        tokio::fs::write(list_dir.join("user.list"), "example.com\n")
            .await
            .unwrap();
        let app = api_router(
            test_state_with_config(dir.path().join("state"), Some(config)),
            dir.path().join("web"),
        );
        let loaded = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/nfqws2/lists/google.list")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let revision = body_json(loaded).await["revision"]
            .as_str()
            .unwrap()
            .to_string();
        let stale = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/nfqws2/lists/google.list")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"revision":"stale"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(stale.status(), StatusCode::CONFLICT);
        assert!(list_dir.join("google.list").exists());
        let standard = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/nfqws2/lists/user.list")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"revision":"{revision}"}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(standard.status(), StatusCode::FORBIDDEN);
        assert!(list_dir.join("user.list").exists());
        let deleted = app
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/nfqws2/lists/google.list")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(r#"{{"revision":"{revision}"}}"#)))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(deleted.status(), StatusCode::OK);
        assert!(!list_dir.join("google.list").exists());
        let backup_dir = dir.path().join("state").join("backups");
        let backups = std::fs::read_dir(backup_dir).unwrap().count();
        assert_eq!(backups, 1);
    }

    #[tokio::test]
    async fn monitoring_batch_rejects_invalid_hosts_and_more_than_twenty_sites() {
        let app = api_router(
            test_state(PathBuf::from("/tmp/nfqws2-ui-test")),
            PathBuf::from("/tmp/web"),
        );
        let invalid = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/nfqws2/diagnostics/monitor")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"hosts":["192.168.1.1"]}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(invalid.status(), StatusCode::BAD_REQUEST);
        let hosts = (0..21)
            .map(|index| format!("site{index}.example"))
            .collect::<Vec<_>>();
        let too_many = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/nfqws2/diagnostics/monitor")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "hosts": hosts }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(too_many.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn custom_list_validation_uses_its_configured_ipset_reference() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let list_dir = dir.path().join("lists");
        tokio::fs::create_dir_all(&list_dir).await.unwrap();
        tokio::fs::write(&config, "NFQWS_ARGS_IPSET=--ipset=lists/google.list\n")
            .await
            .unwrap();
        let custom_path = list_dir.join("google.list");
        tokio::fs::write(&custom_path, "192.0.2.1\n").await.unwrap();
        let app = api_router(
            test_state_with_config(dir.path().join("state"), Some(config)),
            dir.path().join("web"),
        );
        let loaded = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/nfqws2/lists/google.list")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let revision = body_json(loaded).await["revision"]
            .as_str()
            .unwrap()
            .to_string();
        let saved = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/nfqws2/lists/google.list")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"content":"google.com\n","revision":"{revision}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(saved.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            tokio::fs::read_to_string(custom_path).await.unwrap(),
            "192.0.2.1\n"
        );
    }

    #[test]
    fn list_validation_keeps_comments_and_accepts_domain_and_cidr_formats() {
        assert!(
            validate_list_content(
                "user.list",
                "# keep this note\nexample.org\n*.cdn.example.org\nexample.net.\n"
            )
            .is_ok()
        );
        assert!(validate_list_content("ipset.list", "192.0.2.1\n2001:db8::/32\n").is_ok());
        assert!(validate_list_content("user.list", "bad domain.example\n").is_err());
        assert!(validate_list_content("ipset.list", "192.0.2.0/33\n").is_err());
        assert!(validate_list_content("ipset.list", "192.0.2.0/\n").is_err());
        assert!(validate_list_content("../outside", "example.org\n").is_err());
    }

    #[test]
    fn package_lists_are_available_even_when_mode_all_does_not_reference_them() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let lists = dir.path().join("lists");
        std::fs::create_dir_all(&lists).unwrap();
        std::fs::write(&config, "NFQWS_EXTRA_ARGS=\"$MODE_ALL\"\n").unwrap();
        for id in LIST_IDS {
            let path = lists.join(id);
            std::fs::write(&path, "# existing package file\n").unwrap();
            assert_eq!(
                list_file_path(&config, id),
                Some(path.canonicalize().unwrap())
            );
        }
        std::fs::remove_file(lists.join("user.list")).unwrap();
        assert!(list_file_path(&config, "user.list").is_none());
        assert!(list_file_path(&config, "../nfqws2.conf").is_none());
    }

    #[test]
    fn keenetic_metrics_use_rci_memory_capacity_without_rounding_or_invention() {
        let metrics = keenetic_system_metrics(&serde_json::json!({
            "memtotal": 524288, "memory": "131072/524288", "cpuload": 17
        }))
        .unwrap();
        assert_eq!(metrics.memory_total_bytes, Some(512 * 1024 * 1024));
        assert_eq!(metrics.memory_used_bytes, Some(128 * 1024 * 1024));
        assert_eq!(metrics.cpu_percent, Some(17.0));
        assert_eq!(metrics.source, "keenetic-rci");
        assert!(keenetic_system_metrics(&serde_json::json!({"memtotal": 0})).is_none());
        assert!(
            keenetic_system_metrics(&serde_json::json!({
                "memtotal": 524288, "memory": "999999/524288", "cpuload": 17
            }))
            .is_none()
        );
        assert!(
            keenetic_system_metrics(&serde_json::json!({
                "memtotal": 524288, "memory": "131072/524288", "cpuload": 101
            }))
            .is_none()
        );
    }

    #[test]
    fn list_path_only_resolves_existing_allowlisted_files_referenced_by_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let lists = dir.path().join("lists");
        std::fs::create_dir_all(&lists).unwrap();
        std::fs::write(&config, "NFQWS_ARGS=--hostlist=lists/user.list\n").unwrap();
        std::fs::write(lists.join("user.list"), "example.org\n").unwrap();

        assert_eq!(
            list_file_path(&config, "user.list"),
            Some(lists.join("user.list").canonicalize().unwrap())
        );
        assert!(list_file_path(&config, "missing.list").is_none());
        assert!(list_file_path(&config, "../nfqws2.conf").is_none());
    }

    #[test]
    fn list_resolver_uses_active_config_reference_not_a_guessed_filename() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let custom = dir.path().join("custom");
        std::fs::create_dir_all(&custom).unwrap();
        std::fs::write(
            &config,
            "NFQWS_ARGS=\"--hostlist=custom/domains.txt\" # --hostlist=lists/user.list\n",
        )
        .unwrap();
        std::fs::write(custom.join("domains.txt"), "example.org\n").unwrap();
        std::fs::create_dir_all(dir.path().join("lists")).unwrap();
        std::fs::write(dir.path().join("lists/user.list"), "stale.example.org\n").unwrap();

        assert_eq!(
            list_file_path(&config, "user.list"),
            Some(custom.join("domains.txt").canonicalize().unwrap())
        );
    }

    #[test]
    fn standard_nfqws2_mode_list_reference_resolves_user_list() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let list_dir = dir.path().join("lists");
        std::fs::create_dir_all(&list_dir).unwrap();
        let user_list = list_dir.join("user.list");
        std::fs::write(
            &config,
            "MODE_LIST=\"--hostlist=lists/user.list\"\nMODE_ALL=\"--hostlist-exclude=lists/exclude.list\"\nMODE_AUTO=\"$MODE_LIST --hostlist-auto=lists/auto.list $MODE_ALL\"\nNFQWS_EXTRA_ARGS=\"$MODE_AUTO\"\n",
        )
        .unwrap();
        std::fs::write(&user_list, "example.org\n").unwrap();
        std::fs::write(list_dir.join("exclude.list"), "excluded.example.org\n").unwrap();
        std::fs::write(list_dir.join("auto.list"), "automatic.example.org\n").unwrap();

        assert_eq!(
            list_file_path(&config, "user.list"),
            Some(user_list.canonicalize().unwrap())
        );
    }

    #[test]
    fn list_reference_parser_handles_known_flags_and_ignores_shell_variables() {
        let config = "# --hostlist=ignored.list\nNFQWS_ARGS='--hostlist-auto=lists/auto.list --ipset=/opt/etc/nfqws2/lists/addresses.list'\nNFQWS_ARGS_CUSTOM=--hostlist=${USER_LIST}\n";
        assert_eq!(
            configured_list_reference(config, "auto.list"),
            Some(PathBuf::from("lists/auto.list"))
        );
        assert_eq!(
            configured_list_reference(config, "ipset.list"),
            Some(PathBuf::from("/opt/etc/nfqws2/lists/addresses.list"))
        );
        assert_eq!(configured_list_reference(config, "exclude.list"), None);
        assert_eq!(configured_list_reference(config, "user.list"), None);
        assert_eq!(configured_list_reference(config, "unknown.list"), None);
        assert_eq!(
            configured_list_reference(
                "NFQWS_ARGS=--hostlist=lists/one.list\nNFQWS_ARGS_QUIC=--hostlist=lists/two.list\n",
                "user.list"
            ),
            None
        );
    }

    #[cfg(unix)]
    #[test]
    fn list_path_rejects_symlink_references_outside_the_config_directory() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let lists = dir.path().join("lists");
        std::fs::create_dir_all(&lists).unwrap();
        std::fs::write(&config, "NFQWS_ARGS=--hostlist=lists/user.list\n").unwrap();
        let outside = dir.path().join("outside.list");
        std::fs::write(&outside, "secret.example.org\n").unwrap();
        std::os::unix::fs::symlink(&outside, lists.join("user.list")).unwrap();

        assert!(list_file_path(&config, "user.list").is_none());
    }

    #[tokio::test]
    async fn login_endpoints_are_removed_and_service_actions_remain_allowlisted() {
        let dir = tempfile::tempdir().unwrap();
        let app = api_router(test_state(dir.path().to_path_buf()), dir.path().join("web"));
        let removed_login = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/login")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(removed_login.status(), StatusCode::METHOD_NOT_ALLOWED);
        let invalid_action = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/service")
                    .header("content-type", "application/json")
                    .body(Body::from(r#"{"action":"restart;reboot"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(invalid_action.status(), StatusCode::BAD_REQUEST);
    }

    #[tokio::test]
    async fn config_write_creates_private_backup_before_replacing_file() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        tokio::fs::write(&config, "original config\n")
            .await
            .unwrap();
        backup_and_write(dir.path(), &config, b"updated config\n")
            .await
            .unwrap();
        assert_eq!(
            tokio::fs::read_to_string(&config).await.unwrap(),
            "updated config\n"
        );
        let mut entries = tokio::fs::read_dir(dir.path().join("backups"))
            .await
            .unwrap();
        let backup_path = entries.next_entry().await.unwrap().unwrap().path();
        assert_eq!(
            tokio::fs::read_to_string(backup_path).await.unwrap(),
            "original config\n"
        );
    }

    #[tokio::test]
    async fn atomic_write_rejects_stale_revision_before_creating_backup() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        tokio::fs::write(&config, b"loaded version\n")
            .await
            .unwrap();
        let loaded_revision = revision(b"loaded version\n");
        tokio::fs::write(&config, b"external concurrent edit\n")
            .await
            .unwrap();

        let result = backup_and_write_named(
            dir.path(),
            &config,
            "nfqws2.conf",
            MAX_CONFIG_BYTES,
            b"browser draft\n",
            Some(&loaded_revision),
        )
        .await;

        assert!(matches!(result, Err(BackupWriteError::Conflict)));
        assert_eq!(
            tokio::fs::read(&config).await.unwrap(),
            b"external concurrent edit\n"
        );
        assert!(!dir.path().join("backups").exists());
    }

    #[tokio::test]
    async fn rollback_does_not_overwrite_a_newer_external_config() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        tokio::fs::write(&config, b"external newer config\n")
            .await
            .unwrap();

        let result = restore_config_content(
            dir.path(),
            &config,
            b"previous config\n",
            &revision(b"candidate config\n"),
        )
        .await;

        assert!(result.is_err());
        assert_eq!(
            tokio::fs::read(&config).await.unwrap(),
            b"external newer config\n"
        );
        assert!(!dir.path().join("backups").exists());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn atomic_config_write_preserves_unix_owner_group_and_mode() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        if unsafe { libc::geteuid() } != 0 {
            return;
        }
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        tokio::fs::write(&config, b"original\n").await.unwrap();
        tokio::fs::set_permissions(&config, std::fs::Permissions::from_mode(0o640))
            .await
            .unwrap();
        let path = std::ffi::CString::new(config.as_os_str().as_encoded_bytes()).unwrap();
        assert_eq!(unsafe { libc::chown(path.as_ptr(), 65534, 65534) }, 0);
        let before = tokio::fs::metadata(&config).await.unwrap();

        backup_and_write(dir.path(), &config, b"replacement\n")
            .await
            .unwrap();
        let after = tokio::fs::metadata(&config).await.unwrap();

        assert_eq!(after.uid(), before.uid());
        assert_eq!(after.gid(), before.gid());
        assert_eq!(
            after.permissions().mode() & 0o7777,
            before.permissions().mode() & 0o7777
        );
    }

    #[test]
    fn cpu_parser_excludes_guest_time_and_percent_uses_counter_deltas() {
        let before = parse_cpu_counters("cpu 100 10 20 800 30 5 5 10 40 2\n").unwrap();
        let after = parse_cpu_counters("cpu 150 10 25 840 35 5 5 10 100 8\n").unwrap();
        assert_eq!(before, (980, 830));
        assert_eq!(after, (1080, 875));
        assert_eq!(cpu_percent_between(before, after), Some(55.0));
    }

    #[test]
    fn cpu_parser_returns_no_value_for_malformed_or_reset_counters() {
        assert_eq!(parse_cpu_counters("cpu bad data"), None);
        assert_eq!(parse_cpu_counters("cpu 100 10 bad 800 30 5 5 10"), None);
        assert_eq!(cpu_percent_between((100, 80), (90, 70)), None);
    }

    #[test]
    fn memory_metrics_prefer_kernel_memavailable_for_used_memory() {
        let (used, total) = memory_usage_bytes(
            "MemTotal: 1000 kB\nMemFree: 100 kB\nMemAvailable: 700 kB\nBuffers: 40 kB\nCached: 120 kB\n",
        )
        .unwrap();
        assert_eq!(total, 1000 * 1024);
        assert_eq!(used, 300 * 1024);
    }

    #[test]
    fn memory_metrics_estimate_reclaimable_cache_on_older_kernels() {
        let (used, total) = memory_usage_bytes(
            "MemTotal: 1000 kB\nMemFree: 100 kB\nBuffers: 50 kB\nCached: 200 kB\nShmem: 20 kB\n",
        )
        .unwrap();
        assert_eq!(total, 1000 * 1024);
        assert_eq!(used, 670 * 1024);
    }

    #[tokio::test]
    async fn config_write_refuses_oversized_input_without_touching_source() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        tokio::fs::write(&config, b"original\n").await.unwrap();
        let oversized = vec![b'x'; MAX_CONFIG_BYTES + 1];

        let result = backup_and_write(dir.path(), &config, &oversized).await;

        assert!(result.is_err());
        assert_eq!(tokio::fs::read(&config).await.unwrap(), b"original\n");
        assert!(!dir.path().join("backups").exists());
    }

    #[tokio::test]
    async fn site_trace_operation_storage_is_bounded_and_uses_camel_case_fields() {
        let dir = tempfile::tempdir().unwrap();
        let operation = SiteTraceOperation {
            id: "0123456789abcdef0123456789abcdef".into(),
            state: "running".into(),
            stage: "router_probe".into(),
            message: "Проверяется сайт".into(),
            report: None,
            updated_at: 42,
        };
        write_site_trace_operation(dir.path(), &operation)
            .await
            .unwrap();
        let json: serde_json::Value = serde_json::from_slice(
            &tokio::fs::read(site_trace_operation_path(dir.path(), &operation.id))
                .await
                .unwrap(),
        )
        .unwrap();
        assert_eq!(json["updatedAt"], 42);
        assert_eq!(json["stage"], "router_probe");
        assert_eq!(
            read_site_trace_operation(dir.path(), &operation.id)
                .await
                .unwrap()
                .unwrap()
                .state,
            "running"
        );
        let mut updated = operation.clone();
        updated.stage = "cleanup".into();
        updated.updated_at = 43;
        write_site_trace_operation(dir.path(), &updated)
            .await
            .unwrap();
        assert_eq!(
            read_site_trace_operation(dir.path(), &operation.id)
                .await
                .unwrap()
                .unwrap()
                .stage,
            "cleanup"
        );
        assert!(
            write_site_trace_operation(
                dir.path(),
                &SiteTraceOperation {
                    message: "x".repeat(MAX_SITE_TRACE_STATE_BYTES),
                    ..operation
                }
            )
            .await
            .is_err()
        );
    }

    #[test]
    fn site_trace_progress_keeps_the_resolved_ip_and_completed_direct_phase() {
        let mut operation = SiteTraceOperation {
            id: "0123456789abcdef0123456789abcdef".into(),
            state: "running".into(),
            stage: "router_probe".into(),
            message: "Выполняется проверка".into(),
            report: None,
            updated_at: 1,
        };
        let report = site_trace::TraceReport {
            host: "example.com".into(),
            address: Some("93.184.216.34".into()),
            without_nfqws2: site_trace::TracePhase {
                state: "complete".into(),
                ok: Some(true),
                code: Some(200),
                duration_ms: Some(80),
                diagnostic_classification: None,
                reason: "HTTP 200".into(),
            },
            with_nfqws2: site_trace::TracePhase::unavailable("Проверка маршрута ещё выполняется"),
            trace_lines: Vec::new(),
            trace_truncated: false,
            trace_hostname: None,
            trace_profile: None,
            trace_actions: Vec::new(),
            trace_warnings: Vec::new(),
            candidate_attempts: Vec::new(),
            summary: "Проверка выполняется".into(),
        };

        update_site_trace_progress(&mut operation, "direct", report);

        assert_eq!(operation.stage, "direct");
        let partial = operation.report.unwrap();
        assert_eq!(partial.address.as_deref(), Some("93.184.216.34"));
        assert_eq!(partial.without_nfqws2.code, Some(200));
        assert_eq!(partial.with_nfqws2.state, "unavailable");
    }

    #[tokio::test]
    async fn site_trace_operation_reader_rejects_malformed_state_and_oversized_files() {
        let dir = tempfile::tempdir().unwrap();
        let id = "0123456789abcdef0123456789abcdef";
        let path = site_trace_operation_path(dir.path(), id);
        tokio::fs::write(
            &path,
            br#"{"id":"0123456789abcdef0123456789abcdef","state":"invented"}"#,
        )
        .await
        .unwrap();
        assert!(read_site_trace_operation(dir.path(), id).await.is_err());
        tokio::fs::write(&path, vec![b'x'; MAX_SITE_TRACE_STATE_BYTES + 1])
            .await
            .unwrap();
        assert!(read_site_trace_operation(dir.path(), id).await.is_err());
        assert!(
            read_site_trace_operation(dir.path(), "../unknown")
                .await
                .unwrap()
                .is_none()
        );
    }

    #[tokio::test]
    async fn site_trace_status_route_returns_persisted_camel_case_operation() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let operation_dir = data.join("site-trace-operations");
        tokio::fs::create_dir_all(&operation_dir).await.unwrap();
        let id = "0123456789abcdef0123456789abcdef";
        write_site_trace_operation(
            &operation_dir,
            &SiteTraceOperation {
                id: id.into(),
                state: "running".into(),
                stage: "router_probe".into(),
                message: "Проверяется роутером".into(),
                report: None,
                updated_at: 123,
            },
        )
        .await
        .unwrap();
        let app = api_router(test_state(data), dir.path().join("web"));
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/api/nfqws2/diagnostics/trace/operations/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = body_json(response).await;
        assert_eq!(body["id"], id);
        assert_eq!(body["state"], "running");
        assert_eq!(body["stage"], "router_probe");
        assert_eq!(body["updatedAt"], 123);
    }

    #[tokio::test]
    async fn site_trace_start_returns_accepted_then_persists_runner_result() {
        let dir = tempfile::tempdir().unwrap();
        let runner: Arc<SiteTraceRunner> =
            Arc::new(|host, _proto, _, _, _, _repeats, _baseline, progress| {
                Box::pin(async move {
                    let partial = site_trace::TraceReport {
                        host: host.clone(),
                        address: Some("93.184.216.34".into()),
                        without_nfqws2: site_trace::TracePhase::unavailable(
                            "Прямой запрос выполняется",
                        ),
                        with_nfqws2: site_trace::TracePhase::unavailable("Маршрут ещё не проверен"),
                        trace_lines: Vec::new(),
                        trace_truncated: false,
                        trace_hostname: None,
                        trace_profile: None,
                        trace_actions: Vec::new(),
                        trace_warnings: Vec::new(),
                        candidate_attempts: Vec::new(),
                        summary: "Определён IP".into(),
                    };
                    if let Some(progress) = progress {
                        progress("resolve", partial);
                        tokio::time::sleep(std::time::Duration::from_millis(80)).await;
                    }
                    site_trace::TraceReport {
                        host,
                        address: Some("93.184.216.34".into()),
                        without_nfqws2: site_trace::TracePhase {
                            state: "complete".into(),
                            ok: Some(true),
                            code: Some(200),
                            duration_ms: Some(12),
                            diagnostic_classification: None,
                            reason: "HTTP 200".into(),
                        },
                        with_nfqws2: site_trace::TracePhase::unavailable("fake runner"),
                        trace_lines: vec!["test log".into()],
                        trace_truncated: false,
                        trace_hostname: None,
                        trace_profile: None,
                        trace_actions: Vec::new(),
                        trace_warnings: Vec::new(),
                        candidate_attempts: Vec::new(),
                        summary: "Тестовый результат".into(),
                    }
                })
            });
        let app = api_router(
            test_state_with_trace_runner(dir.path().join("data"), runner),
            dir.path().join("web"),
        );
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/nfqws2/diagnostics/trace")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"host":"example.com"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let started = body_json(response).await;
        let id = started["id"].as_str().unwrap().to_owned();
        assert_eq!(started["state"], "queued");
        assert!(started["report"].is_null());
        let mut operation = serde_json::Value::Null;
        let mut saw_partial = false;
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        while tokio::time::Instant::now() < deadline {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .uri(format!("/api/nfqws2/diagnostics/trace/operations/{id}"))
                        .body(Body::empty())
                        .unwrap(),
                )
                .await
                .unwrap();
            operation = body_json(response).await;
            if operation["stage"] == "resolve" && operation["report"]["address"] == "93.184.216.34"
            {
                saw_partial = true;
            }
            if operation["state"] == "complete" {
                break;
            }
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        assert!(
            saw_partial,
            "resolved address should be persisted before trace completion"
        );
        assert_eq!(operation["state"], "complete");
        assert_eq!(operation["report"]["host"], "example.com");
        assert_eq!(operation["report"]["withoutNfqws2"]["code"], 200);
        assert_eq!(operation["report"]["traceLines"][0], "test log");
    }

    #[tokio::test]
    async fn site_trace_stop_route_sets_cancellation_and_persists_stopping_state() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let operation_dir = data.join("site-trace-operations");
        tokio::fs::create_dir_all(&operation_dir).await.unwrap();
        let id = "0123456789abcdef0123456789abcdef";
        write_site_trace_operation(
            &operation_dir,
            &SiteTraceOperation {
                id: id.into(),
                state: "running".into(),
                stage: "router_probe".into(),
                message: "Проверяется".into(),
                report: None,
                updated_at: 123,
            },
        )
        .await
        .unwrap();
        let state = test_state(data.clone());
        let cancelled = Arc::new(AtomicBool::new(false));
        state
            .site_trace_controls
            .lock()
            .await
            .insert(id.into(), cancelled.clone());
        let app = api_router(state, dir.path().join("web"));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!(
                        "/api/nfqws2/diagnostics/trace/operations/{id}/stop"
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::ACCEPTED);
        let body = body_json(response).await;
        assert_eq!(body["state"], "stopping");
        assert_eq!(body["stage"], "cleanup");
        assert!(cancelled.load(Ordering::Acquire));
        let persisted = read_site_trace_operation(&operation_dir, id)
            .await
            .unwrap()
            .unwrap();
        assert_eq!(persisted.state, "stopping");
    }

    #[test]
    fn lua_script_reader_resolves_only_allowlisted_names_under_lua_directory() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("zapret-lib.lua"), "return true\n").unwrap();

        let script = read_lua_script(dir.path(), "zapret-lib.lua").unwrap();
        assert_eq!(script.content, "return true\n");
        assert!(!script.compressed);
        assert!(!script.truncated);
        assert!(resolve_lua_script(dir.path(), "../../etc/passwd").is_none());
        assert!(resolve_lua_script(dir.path(), "custom.lua").is_none());
        assert!(read_lua_script(dir.path(), "zapret-auto.lua").is_err());
    }

    #[test]
    fn lua_script_reader_decompresses_gzip_and_caps_preview_size() {
        use flate2::{Compression, write::GzEncoder};
        use std::io::Write;

        let dir = tempfile::tempdir().unwrap();
        let original = "-- source\n".repeat(70_000);
        let mut encoder = GzEncoder::new(Vec::new(), Compression::default());
        encoder.write_all(original.as_bytes()).unwrap();
        std::fs::write(
            dir.path().join("zapret-auto.lua.gz"),
            encoder.finish().unwrap(),
        )
        .unwrap();

        let script = read_lua_script(dir.path(), "zapret-auto.lua").unwrap();
        assert!(script.compressed);
        assert!(script.truncated);
        assert_eq!(script.content.len(), 512 * 1024);
        assert!(original.starts_with(&script.content));
    }
}
