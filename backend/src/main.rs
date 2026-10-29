use axum::{
    Json, Router,
    body::{Body, Bytes},
    extract::{DefaultBodyLimit, Path as AxumPath, Query, State},
    http::{HeaderName, HeaderValue, Request, StatusCode},
    middleware::{self, Next},
    response::{
        Html, IntoResponse, Response,
        sse::{Event, KeepAlive, Sse},
    },
    routing::{get, post},
};
use nfqws2_ui::change_history;
use nfqws2_ui::community_strategies::{
    self, CommunityResponse, CommunityViewItem, ProviderSettings,
};
use nfqws2_ui::config_document::{ConfigDocument, DiagnosticSeverity, validate_nfqws2_config};
use nfqws2_ui::config_format::format_nfqws_config;
use nfqws2_ui::dpi_lab;
use nfqws2_ui::freeze_diagnostic;
use nfqws2_ui::freeze_profile_join;
use nfqws2_ui::interception_status::{self, InterceptionStatus};
use nfqws2_ui::managed_asn_lists::{self, ManagedAsnList, ManagedAsnListStore};
use nfqws2_ui::monitor_auto::{
    self, AutoStrategyEvent, AutoStrategyOffer, AutoStrategySettings, AutoStrategyStore,
};
use nfqws2_ui::site_diagnostics::{self, DiagnosticProgress, DiagnosticReport, DiagnosticStep};
use nfqws2_ui::site_trace;
use nfqws2_ui::strategy_picker::{self, StrategyMode, StrategyProtocol};
use nfqws2_ui::transfer_diagnostic;
mod package_leftovers;
mod snapshots;
use nfqws2_ui::{
    ServiceAction, ServiceState, default_network_interfaces, discover_system_paths,
    known_config_paths, known_init_scripts, network_interface_names, parse_meminfo_kb,
    process_matches, service_action, service_action_at, service_status,
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
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use tokio::io::{AsyncReadExt, AsyncSeekExt, AsyncWriteExt};
use tokio::{net::TcpListener, sync::Mutex};
use tower_http::{services::ServeDir, trace::TraceLayer};
mod auth;

const MAX_CONFIG_BYTES: usize = 512 * 1024;
const MAX_LIST_BYTES: usize = 512 * 1024;
const MAX_LOG_BYTES: usize = 256 * 1024;
const MAX_RELEASE_RESPONSE_BYTES: usize = 1024 * 1024;
const UI_RELEASE_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(2 * 60);
const UI_RELEASE_ERROR_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(10 * 60);
const CORE_RELEASE_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(2 * 60);
const CORE_RELEASE_ERROR_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(10 * 60);
const CONFIG_APPLY_MONITOR_BASELINE_TTL: std::time::Duration =
    std::time::Duration::from_secs(4 * 60 * 60);
const WRITE_REVISION_CONFLICT: &str = "REVISION_CONFLICT";
const GITHUB_RELEASES_API: &str =
    "https://api.github.com/repos/rangerxxyy/nfqws2-ui/releases?per_page=10";
const NFQWS2_RELEASES_API: &str =
    "https://api.github.com/repos/nfqws/nfqws2-keenetic/releases?per_page=10";
const GITHUB_API_FALLBACKS: [&str; 2] = ["https://gh-proxy.com/", "https://ghfast.top/"];
static UPDATE_SEQUENCE: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

fn github_wget_candidates() -> [&'static str; 2] {
    ["/opt/bin/wget", "wget"]
}

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
    auth: Arc<auth::AuthManager>,
    port: u16,
    cpu_previous: Arc<Mutex<Option<(u64, u64)>>>,
    config_write_lock: Mutex<()>,
    config_override: Option<PathBuf>,
    update_lock: Mutex<()>,
    ui_release_cache: Mutex<Option<UiReleaseCacheEntry>>,
    core_release_cache: Mutex<Option<CoreReleaseCacheEntry>>,
    installed_core_version_cache: Mutex<InstalledCoreVersionCache>,
    keenetic_system_stats_cache: Mutex<KeeneticSystemStatsCache>,
    interception_status_cache: Mutex<InterceptionStatusCache>,
    ui_update_status: Arc<std::sync::RwLock<UiUpdateStatus>>,
    site_diagnostic_lock: Arc<Mutex<()>>,
    dpi_lab_lock: Mutex<()>,
    managed_asn_refresh_lock: Mutex<()>,
    monitoring_lock: Arc<Mutex<()>>,
    auto_strategy_lock: Arc<Mutex<()>>,
    site_trace_controls: Mutex<HashMap<String, Arc<AtomicBool>>>,
    site_trace_runner: Arc<SiteTraceRunner>,
    foreign_config_controls: Mutex<HashMap<String, Arc<AtomicBool>>>,
    strategy_pick_controls: Mutex<HashMap<String, Arc<AtomicBool>>>,
    transfer_diagnostic_controls: Mutex<HashMap<String, Arc<AtomicBool>>>,
    freeze_scan_controls: Mutex<HashMap<String, Arc<freeze_diagnostic::FreezeScanPause>>>,
    nfqws_cli_catalog_cache: Mutex<Option<NfqwsCliCatalogCacheEntry>>,
    nfqws_lua_catalog_cache: Mutex<Option<NfqwsLuaCatalogCacheEntry>>,
    #[cfg(test)]
    change_history_fail_path: Mutex<Option<PathBuf>>,
}

#[derive(Default)]
struct InterceptionStatusCache {
    checked_at: Option<Instant>,
    visible: Option<InterceptionStatus>,
    missing_since: Option<Instant>,
}

const INSTALLED_CORE_VERSION_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(5);

#[derive(Default)]
struct InstalledCoreVersionCache {
    checked_at: Option<Instant>,
    version: Option<String>,
}

impl InstalledCoreVersionCache {
    fn get(&self, now: Instant) -> Option<Option<String>> {
        self.checked_at
            .filter(|checked_at| {
                now.saturating_duration_since(*checked_at) < INSTALLED_CORE_VERSION_CACHE_TTL
            })
            .map(|_| self.version.clone())
    }

    fn store(&mut self, version: Option<String>, checked_at: Instant) {
        self.checked_at = Some(checked_at);
        self.version = version;
    }
}

const KEENETIC_SYSTEM_STATS_CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(5);

#[derive(Default)]
struct KeeneticSystemStatsCache {
    checked_at: Option<Instant>,
    stats: Option<SystemInfo>,
}

impl KeeneticSystemStatsCache {
    fn get(&self, now: Instant) -> Option<Option<SystemInfo>> {
        self.checked_at
            .filter(|checked_at| keenetic_system_stats_cache_is_fresh(Some(*checked_at), now))
            .map(|_| self.stats.clone())
    }

    fn store(&mut self, stats: Option<SystemInfo>, checked_at: Instant) {
        self.checked_at = Some(checked_at);
        self.stats = stats;
    }
}

fn keenetic_system_stats_cache_is_fresh(checked_at: Option<Instant>, now: Instant) -> bool {
    checked_at.is_some_and(|checked_at| {
        now.saturating_duration_since(checked_at) < KEENETIC_SYSTEM_STATS_CACHE_TTL
    })
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
struct AutoStrategySettingsUpdate {
    enabled: bool,
    apply_automatically: bool,
    failures_before_pick: usize,
    cooldown_hours: u64,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForeignConfigCheckOperation {
    id: String,
    state: String,
    stage: String,
    current: Option<String>,
    done: usize,
    total: usize,
    hosts: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    podkop_hosts: Vec<String>,
    comparisons: Vec<ForeignConfigComparison>,
    message: String,
    updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForeignConfigComparison {
    name: String,
    source: String,
    sites: Vec<ForeignConfigSiteResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForeignConfigSiteResult {
    host: String,
    current_ok: Option<bool>,
    current_detail: String,
    #[serde(default)]
    current_attempts: usize,
    #[serde(default)]
    current_successes: usize,
    #[serde(default)]
    current_average_ms: Option<u64>,
    candidate_ok: Option<bool>,
    candidate_detail: String,
    #[serde(default)]
    candidate_attempts: usize,
    #[serde(default)]
    candidate_successes: usize,
    #[serde(default)]
    candidate_average_ms: Option<u64>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForeignConfigCheckRequest {
    configs: Vec<ForeignConfigInput>,
    hosts: Vec<String>,
    repeats: Option<usize>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ForeignConfigInput {
    name: String,
    source: Option<String>,
    content: Option<String>,
    url: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SupportReportRequest {
    #[serde(default = "default_true")]
    hide_site_names: bool,
}

struct SupportReportData {
    platform: String,
    architecture: String,
    service_state: String,
    core_version: Option<String>,
    memory_used_bytes: Option<u64>,
    memory_total_bytes: Option<u64>,
    cpu_percent: Option<f64>,
    interception_status: String,
    running_profiles: usize,
    monitored_sites: Vec<String>,
    monitoring_enabled: bool,
    config_errors: usize,
    config_warnings: usize,
    lists_count: usize,
}

fn format_support_report(data: &SupportReportData) -> String {
    let sites = if data.monitored_sites.is_empty() {
        "Сайты мониторинга: не настроены".to_owned()
    } else {
        format!("Сайты мониторинга: {}", data.monitored_sites.join(", "))
    };
    let core_version = data.core_version.as_deref().unwrap_or("не определена");
    let memory = match (data.memory_used_bytes, data.memory_total_bytes) {
        (Some(used), Some(total)) => format!(
            "{} / {} MiB",
            used.saturating_add(512 * 1024) / (1024 * 1024),
            total.saturating_add(512 * 1024) / (1024 * 1024)
        ),
        _ => "нет данных".to_owned(),
    };
    let cpu = data
        .cpu_percent
        .filter(|value| value.is_finite() && (0.0..=100.0).contains(value))
        .map(|value| format!("{value:.1}%"))
        .unwrap_or_else(|| "нет данных".to_owned());
    format!(
        "Отчёт для помощи · nfqws2-UI {}\nПлатформа: {}\nАрхитектура: {}\nПанель: {}\nСлужба nfqws2: {}\nВерсия nfqws2: {}\nПамять: {}\nЗагрузка CPU: {}\nПравила перехвата: {}\nАктивных профилей: {}\nСтатическая проверка конфига: ошибок {}, предупреждений {}\nСписков: {}\nМониторинг: {}\n{}\n\nСодержимое конфига и списков, токен Telegram и подробный вывод команд в отчёт не включены. Сетевая проверка не запускалась.",
        env!("CARGO_PKG_VERSION"),
        data.platform,
        data.architecture,
        env!("CARGO_PKG_VERSION"),
        data.service_state,
        core_version,
        memory,
        cpu,
        data.interception_status,
        data.running_profiles,
        data.config_errors,
        data.config_warnings,
        data.lists_count,
        if data.monitoring_enabled {
            "включён"
        } else {
            "выключен"
        },
        sites,
    )
}

fn support_site_labels(sites: &[MonitoredSite], hide_site_names: bool) -> Vec<String> {
    sites
        .iter()
        .enumerate()
        .map(|(index, site)| {
            if hide_site_names {
                format!("сайт-{}", index + 1)
            } else {
                site.host
                    .chars()
                    .filter(|character| !character.is_control())
                    .take(253)
                    .collect()
            }
        })
        .collect()
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
    #[serde(default)]
    hosts: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    podkop_hosts: Vec<String>,
    #[serde(default)]
    multi_site_results: Vec<MultiSiteCandidateResult>,
    #[serde(default)]
    multi_site_baseline: Vec<MultiSiteBaselineResult>,
    message: String,
    updated_at: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MultiSiteDomainResult {
    host: String,
    eligible: bool,
    baseline_reason: String,
    attempts: usize,
    successes: usize,
    elapsed_ms: u64,
    confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MultiSiteCandidateResult {
    candidate: strategy_picker::StrategyCandidate,
    opened_domains: usize,
    domains: Vec<MultiSiteDomainResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MultiSiteBaselineResult {
    host: String,
    eligible: bool,
    reason: String,
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
    include_other_history: bool,
    #[serde(default)]
    refine_lighter: bool,
    #[serde(default)]
    include_community: bool,
    custom_steps: Option<Vec<Vec<String>>>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommunityQuery {
    #[serde(default)]
    detect: bool,
    #[serde(default)]
    auto_detect: bool,
    #[serde(default)]
    force: bool,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommunityTargetAsnQuery {
    host: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CommunityProviderWrite {
    asn: Option<u32>,
    name: String,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MultiStrategyPickRequest {
    hosts: Vec<String>,
    protocol: Option<String>,
    repeats: Option<usize>,
    #[serde(default)]
    include_other_history: bool,
    #[serde(default)]
    include_community: bool,
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

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FreezeProfileJoinRequest {
    operation_id: String,
    result_index: usize,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct FreezeProfileJoinApplyRequest {
    operation_id: String,
    result_index: usize,
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct FreezeProfileJoinPreview {
    operation_id: String,
    result_index: usize,
    asn: u32,
    profile_index: usize,
    profile_name: String,
    list_id: String,
    prefix_count: usize,
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
    #[serde(default)]
    probe_short_uploads: bool,
}

const MAX_SITE_TRACE_STATE_BYTES: usize = 512 * 1024;
const MAX_SITE_TRACE_OPERATIONS: usize = 12;
const MAX_STRATEGY_PICK_STATE_BYTES: usize = 512 * 1024;
const MAX_STRATEGY_PICK_OPERATIONS: usize = 12;
const MAX_TRANSFER_DIAGNOSTIC_STATE_BYTES: usize = 128 * 1024;
const MAX_FREEZE_SCAN_STATE_BYTES: usize = 2 * 1024 * 1024;
const MAX_FREEZE_SCAN_PROGRESS: usize = freeze_diagnostic::MAX_TARGETS * 2;
const MAX_FOREIGN_CONFIG_BYTES: usize = 64 * 1024;
const MAX_FOREIGN_CONFIGS: usize = 10;
const MAX_FOREIGN_CONFIG_HOSTS: usize = 15;
const DEFAULT_FOREIGN_CONFIG_CONFIRMATION_REPEATS: usize = 3;
const MIN_FOREIGN_CONFIG_CONFIRMATION_REPEATS: usize = 1;
const MAX_FOREIGN_CONFIG_CONFIRMATION_REPEATS: usize = 5;
const MIN_FOREIGN_CONFIG_AVAILABLE_MB: u64 = 20;
const MAX_FOREIGN_AUTO_LIST_BYTES: u64 = 4 * 1024 * 1024;

fn normalize_foreign_config_repeats(requested: Option<usize>) -> usize {
    requested
        .unwrap_or(DEFAULT_FOREIGN_CONFIG_CONFIRMATION_REPEATS)
        .clamp(
            MIN_FOREIGN_CONFIG_CONFIRMATION_REPEATS,
            MAX_FOREIGN_CONFIG_CONFIRMATION_REPEATS,
        )
}

fn validate_foreign_config_counts(configs: usize, hosts: usize) -> Result<(), &'static str> {
    if configs == 0 || configs > MAX_FOREIGN_CONFIGS {
        return Err("Добавьте от одного до десяти конфигов для сравнения");
    }
    if hosts == 0 || hosts > MAX_FOREIGN_CONFIG_HOSTS {
        return Err("Укажите от одного до пятнадцати сайтов или публичных IPv4");
    }
    Ok(())
}

fn foreign_config_candidate_is_applicable(
    state: &str,
    comparison: &ForeignConfigComparison,
) -> bool {
    state == "complete"
        && !comparison.sites.is_empty()
        && comparison
            .sites
            .iter()
            .all(|site| site.candidate_ok == Some(true))
}

fn foreign_config_should_retry(
    outcome: Option<bool>,
    completed_attempt: usize,
    repeats: usize,
) -> bool {
    outcome == Some(true) && completed_attempt + 1 < repeats
}

fn append_foreign_config_assignment(
    source: &str,
    name: &str,
    raw_value: &str,
) -> Result<String, String> {
    if source.contains('\0') || raw_value.contains(['\0', '\n', '\r']) {
        return Err("Невозможно безопасно добавить значения по умолчанию для конфигурации".into());
    }
    let separator = if source.is_empty() || source.ends_with('\n') {
        ""
    } else {
        "\n"
    };
    Ok(format!("{source}{separator}{name}={raw_value}\n"))
}

fn insert_foreign_config_assignments_before_extra(source: &str, assignments: &str) -> String {
    let offset = source
        .split_inclusive('\n')
        .scan(0usize, |offset, line| {
            let start = *offset;
            *offset += line.len();
            Some((start, line))
        })
        .find_map(|(start, line)| {
            let mut statement = line.trim_start();
            if let Some(rest) = statement.strip_prefix("export")
                && rest.starts_with([' ', '\t'])
            {
                statement = rest.trim_start();
            }
            statement.starts_with("NFQWS_EXTRA_ARGS=").then_some(start)
        });
    if let Some(offset) = offset {
        format!("{}{}{}", &source[..offset], assignments, &source[offset..])
    } else if source.is_empty() || source.ends_with('\n') {
        format!("{source}{assignments}")
    } else {
        format!("{source}\n{assignments}")
    }
}

fn foreign_config_install_root(lists_directory: &Path) -> &'static str {
    let path = lists_directory.to_string_lossy();
    if path == "/etc/nfqws2/lists" || path.starts_with("/etc/nfqws2/lists/") {
        ""
    } else {
        "/opt"
    }
}

fn foreign_config_with_zemidala_defaults(
    source: &str,
    lua_directory: &Path,
    blobs_directory: &Path,
    lists_directory: &Path,
) -> Result<String, String> {
    let document = ConfigDocument::parse(source)?;
    let mut adapted = source.to_owned();
    let install_root = foreign_config_install_root(lists_directory);
    if !document.contains_assignment("NFQWS_BASE_ARGS") {
        let mut defaults = Vec::new();
        for module in ["zapret-lib.lua", "zapret-antidpi.lua", "zapret-auto.lua"] {
            let source_path = format!("{install_root}/etc/nfqws2/lua/{module}");
            if resolve_foreign_lua_path(&source_path, lua_directory).is_ok() {
                defaults.push(format!("--lua-init=@{source_path}"));
            }
        }
        for blob in ["quic_initial", "tls_clienthello"] {
            let source_path = format!("{install_root}/etc/nfqws2/blobs/{blob}.bin");
            if resolve_foreign_blob_path(&source_path, blobs_directory).is_ok() {
                defaults.push(format!("--blob={blob}:@{source_path}"));
            }
        }
        let value = defaults.join(" ");
        adapted =
            append_foreign_config_assignment(&adapted, "NFQWS_BASE_ARGS", &format!("\"{value}\""))?;
    }

    let mut missing_modes = String::new();
    for name in ["MODE_LIST", "MODE_ALL", "MODE_AUTO"] {
        let current = ConfigDocument::parse(&adapted)?;
        if !current.contains_assignment(name) {
            let raw_value = match name {
                "MODE_LIST" => format!("\"--hostlist={install_root}/etc/nfqws2/lists/user.list\""),
                "MODE_ALL" => {
                    format!("\"--hostlist-exclude={install_root}/etc/nfqws2/lists/exclude.list\"")
                }
                "MODE_AUTO" => format!(
                    "\"$MODE_LIST --hostlist-auto={install_root}/etc/nfqws2/lists/auto.list --hostlist-auto-debug={install_root}/var/log/nfqws2.log $MODE_ALL\""
                ),
                _ => unreachable!("only stock modes are listed"),
            };
            missing_modes.push_str(&format!("{name}={raw_value}\n"));
        }
    }
    if !missing_modes.is_empty() {
        adapted = insert_foreign_config_assignments_before_extra(&adapted, &missing_modes);
    }
    let current = ConfigDocument::parse(&adapted)?;
    if !current.contains_assignment("NFQWS_EXTRA_ARGS") {
        adapted = append_foreign_config_assignment(&adapted, "NFQWS_EXTRA_ARGS", "\"$MODE_LIST\"")?;
    }
    Ok(adapted)
}

fn foreign_config_dry_run_argv(
    binary: &str,
    source: &str,
    args: &[String],
) -> Result<Vec<String>, String> {
    let document = ConfigDocument::parse(source)?;
    let value = |name: &str, fallback: &str| -> Result<String, String> {
        if document.contains_assignment(name) {
            document.literal_or_mode_value(name)
        } else {
            Ok(fallback.to_owned())
        }
    };
    let queue = value("NFQUEUE_NUM", "300")?;
    let queue = if queue.is_empty() { "300" } else { &queue };
    if !queue.bytes().all(|byte| byte.is_ascii_digit()) || queue.parse::<u16>().is_err() {
        return Err("NFQUEUE_NUM должен быть числом от 0 до 65535".into());
    }
    let user = value("USER", "nobody")?;
    let user = if user.is_empty() { "nobody" } else { &user };
    if user.is_empty()
        || !user
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.-".contains(&byte))
    {
        return Err("USER содержит неподдерживаемые символы".into());
    }
    let base_args = if document.contains_assignment("NFQWS_BASE_ARGS") {
        document.literal_or_mode_value("NFQWS_BASE_ARGS")?
    } else {
        String::new()
    };
    let mut argv = vec![
        binary.to_owned(),
        "--dry-run".into(),
        format!("--user={user}"),
        format!("--qnum={queue}"),
    ];
    if !base_args
        .split_whitespace()
        .any(|argument| argument.starts_with("--fastpath-workaround="))
    {
        argv.push("--fastpath-workaround=auto".into());
    }
    argv.extend(args.iter().cloned());
    Ok(argv)
}

fn foreign_config_memory_warning(meminfo: &str) -> Option<String> {
    let available_kb = parse_meminfo_kb_key(meminfo, "MemAvailable")?;
    let available_mb = available_kb / 1024;
    (available_mb < MIN_FOREIGN_CONFIG_AVAILABLE_MB).then(|| {
        format!(
            "Недостаточно свободной памяти ({available_mb} МБ): проверочный nfqws2 не запущен; требуется не менее {MIN_FOREIGN_CONFIG_AVAILABLE_MB} МБ"
        )
    })
}
const MAX_FOREIGN_CONFIG_OPERATIONS: usize = 8;
const MAX_FOREIGN_CONFIG_STATE_BYTES: usize = 128 * 1024;

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

struct MultiSiteProbeRequest<'a> {
    hosts: &'a [String],
    protocol: String,
    args: Vec<String>,
    binary: String,
    control: Arc<AtomicBool>,
    repeats: usize,
    known_baseline: Option<bool>,
}

async fn run_multi_site_probes(
    runner: Arc<SiteTraceRunner>,
    request: MultiSiteProbeRequest<'_>,
) -> Vec<(String, Vec<site_trace::TracePhase>)> {
    let permits = Arc::new(tokio::sync::Semaphore::new(
        nfqws2_ui::probe_lanes::MAX_BATCH_LANES,
    ));
    let mut tasks = tokio::task::JoinSet::new();
    for host in request.hosts {
        let host = host.clone();
        let protocol = request.protocol.clone();
        let args = request.args.clone();
        let binary = request.binary.clone();
        let control = request.control.clone();
        let repeats = request.repeats;
        let known_baseline = request.known_baseline;
        let runner = runner.clone();
        let permits = permits.clone();
        tasks.spawn(async move {
            let _permit = permits.acquire_owned().await.ok()?;
            let report = runner(
                host.clone(),
                protocol,
                args,
                binary,
                control,
                repeats,
                known_baseline,
                None,
            )
            .await;
            let phases = if report.candidate_attempts.is_empty() {
                vec![report.with_nfqws2]
            } else {
                report.candidate_attempts
            };
            Some((host, phases))
        });
    }
    let mut results = Vec::with_capacity(request.hosts.len());
    while let Some(joined) = tasks.join_next().await {
        if let Ok(Some(result)) = joined {
            results.push(result);
        }
    }
    results.sort_by_key(|(host, _)| {
        request
            .hosts
            .iter()
            .position(|expected| expected == host)
            .unwrap_or(usize::MAX)
    });
    results
}

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
    if !matches!(
        operation.state.as_str(),
        "complete" | "partial" | "cancelled" | "failed"
    ) {
        return Err("Применять можно только подтверждённый результат завершённого или остановленного подбора".into());
    }
    let protocol = match operation.protocol.as_str() {
        "http" => StrategyProtocol::Http,
        "https" => StrategyProtocol::Https,
        "quic" => StrategyProtocol::Quic,
        _ => return Err("Применение этой версии протокола не поддерживается".into()),
    };
    let result = operation
        .results
        .get(candidate_index)
        .ok_or_else(|| "Индекс кандидата вне сохранённого результата подбора".to_string())?;
    if !result.confirmed || result.attempts < 2 || result.successes < 2 {
        return Err("Кандидат не прошёл повторное подтверждение; preview недоступен".into());
    }
    if operation.hosts.len() > 1 {
        return build_multi_site_strategy_preview(
            operation,
            candidate_index,
            original_content,
            original_revision,
            functions,
            blobs,
        );
    }
    if protocol == StrategyProtocol::Quic {
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
        if custom_args
            .split_whitespace()
            .any(|argument| argument == selector)
        {
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
            return Err(
                "Сформированный QUIC-профиль не прошёл статическую проверку конфигурации".into(),
            );
        }
        if proposed_content.len() > MAX_CONFIG_BYTES {
            return Err("Изменённая конфигурация превышает лимит размера".into());
        }
        return Ok(StrategyPickPreview {
            operation_id: operation.id.clone(),
            candidate_index,
            host: operation.host.clone(),
            protocol: operation.protocol.clone(),
            candidate: result.candidate.clone(),
            original_revision,
            original_content: original_content.to_owned(),
            proposed_content,
            summary: "Будет добавлен отдельный QUIC-профиль UDP 443 в NFQWS_ARGS_CUSTOM только для выбранного домена. Остальные сайты и TCP-профили не меняются.".into(),
        });
    }
    let _validated_profile = strategy_picker::build_domain_profile(
        &operation.host,
        protocol,
        &result.candidate.steps,
        functions,
        blobs,
    )?;
    let document = ConfigDocument::parse(original_content)?;
    let current_args = document.literal_value("NFQWS_ARGS")?;
    let next_args = insert_first_strategy_steps(&current_args, &result.candidate.steps)?;
    let proposed_content = document.set_literal_or_append("NFQWS_ARGS", &next_args)?;
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
        summary: "Подтверждённая стратегия станет Strategy #1 в NFQWS_ARGS; существующие номера сдвинутся вниз. Это общая группа правил для всех сайтов. NFQWS_BASE_ARGS не меняется.".into(),
    })
}

fn build_multi_site_strategy_preview(
    operation: &StrategyPickOperation,
    candidate_index: usize,
    original_content: &str,
    original_revision: String,
    functions: &HashSet<String>,
    blobs: &[String],
) -> Result<StrategyPickPreview, String> {
    let protocol = match operation.protocol.as_str() {
        "http" => StrategyProtocol::Http,
        "https" => StrategyProtocol::Https,
        _ => return Err("Применение этой версии протокола не поддерживается".into()),
    };
    let strategy = operation
        .results
        .get(candidate_index)
        .ok_or_else(|| "Индекс кандидата вне сохранённого результата подбора".to_string())?;
    let candidate = operation
        .multi_site_results
        .get(candidate_index)
        .ok_or_else(|| "Для кандидата отсутствуют результаты по доменам".to_string())?;
    if !strategy.confirmed || candidate.candidate != strategy.candidate {
        return Err("Общий профиль не подтверждён или результаты доменов не совпадают".into());
    }
    let confirmed_hosts = candidate
        .domains
        .iter()
        .filter(|domain| {
            domain.eligible
                && domain.confirmed
                && domain.attempts >= 2
                && domain.successes == domain.attempts
        })
        .map(|domain| domain.host.as_str())
        .collect::<Vec<_>>();
    if confirmed_hosts.len() < 2 {
        return Err(
            "Для применения нужны минимум два домена, подтверждённых всеми повторами".into(),
        );
    }
    let document = ConfigDocument::parse(original_content)?;
    let custom_args = document.literal_value("NFQWS_ARGS_CUSTOM")?;
    let mut profiles = Vec::with_capacity(confirmed_hosts.len());
    for host in &confirmed_hosts {
        let profile = strategy_picker::build_domain_profile(
            host,
            protocol,
            &strategy.candidate.steps,
            functions,
            blobs,
        )?;
        let selector = format!("--hostlist-domains={host}");
        if custom_args
            .split_whitespace()
            .any(|argument| argument == selector)
        {
            return Err(format!(
                "Для домена {host} уже задан пользовательский профиль; сначала проверьте его вручную"
            ));
        }
        profiles.push(profile);
    }
    let proposed_content =
        document.append_literal_words("NFQWS_ARGS_CUSTOM", &profiles.join(" "))?;
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
        host: confirmed_hosts.join(", "),
        protocol: operation.protocol.clone(),
        candidate: strategy.candidate.clone(),
        original_revision,
        original_content: original_content.to_owned(),
        proposed_content,
        summary: format!(
            "Общий профиль добавлен в NFQWS_ARGS_CUSTOM только для {} доменов, на которых он прошёл все повторы. NFQWS_BASE_ARGS и остальные сайты не меняются.",
            confirmed_hosts.len()
        ),
    })
}

fn build_freeze_strategy_preview(
    operation: &StrategyPickOperation,
    candidate_index: usize,
    original_content: &str,
    original_revision: String,
    functions: &HashSet<String>,
    blobs: &[String],
) -> Result<StrategyPickPreview, String> {
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
    let selector = if let Ok(ip) = operation.host.parse::<std::net::Ipv4Addr>() {
        format!("--ipset-ip={ip}")
    } else {
        format!("--hostlist-domains={}", operation.host)
    };
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

fn build_freeze_profile_join_preview(
    operation: &FreezeScanOperation,
    result_index: usize,
    original_content: &str,
    list_contents: &HashMap<String, String>,
    prefixes: &[String],
) -> Result<FreezeProfileJoinPreview, String> {
    if !matches!(operation.state.as_str(), "complete" | "partial") {
        return Err("Завершите проверку 16 КБ перед добавлением сети в профиль".into());
    }
    let result = operation
        .results
        .get(result_index)
        .ok_or_else(|| "Проверенная сеть не найдена в сохранённом результате".to_owned())?;
    if result.target_kind != freeze_diagnostic::FreezeTargetKind::UserHost
        || !result.freeze_detected
    {
        return Err("Добавлять в существующий профиль можно только проверенную пользовательскую сеть с найденным обрывом".into());
    }
    let freeze_diagnostic::AsnVerification::Verified { asn } = result.asn else {
        return Err("ASN сети не подтверждён RIPEstat".into());
    };
    let address = result
        .address
        .map(IpAddr::V4)
        .filter(|address| site_diagnostics::public_ip(*address))
        .ok_or_else(|| "Проверенная сеть не содержит публичный IPv4-адрес".to_owned())?;
    let found = result
        .found_strategy
        .as_ref()
        .filter(|strategy| {
            strategy.attempts == 3 && strategy.successes == 3 && strategy.reused_from.is_none()
        })
        .ok_or_else(|| "Стратегия сети не прошла все три подтверждающие проверки".to_owned())?;
    if !freeze_profile_join::address_is_in_prefixes(address, prefixes) {
        return Err(format!(
            "Проверенный адрес {address} не входит в текущие префиксы AS{asn}"
        ));
    }
    let Some(profile_index) = freeze_profile_join::find_joinable_profile(
        original_content,
        &result.host,
        address,
        &found.step,
        list_contents,
    )?
    else {
        return Err(
            "Не найден один однозначный IP-only профиль с той же стратегией и проверенным адресом"
                .into(),
        );
    };
    let list_id = format!("ipset_as{asn}.list");
    let proposed_content =
        freeze_profile_join::build_joined_config(original_content, profile_index, &list_id)?;
    if proposed_content.len() > MAX_CONFIG_BYTES {
        return Err("Изменённая конфигурация превышает лимит размера".into());
    }
    let diagnostics = validate_nfqws2_config(&proposed_content)?;
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    {
        return Err("Изменённая конфигурация не прошла статическую проверку".into());
    }
    let profiles = nfqws2_ui::runtime_profiles::from_config(
        original_content,
        &revision(original_content.as_bytes()),
    )?;
    let profile_name = profiles
        .profiles
        .iter()
        .find(|profile| profile.index == profile_index && profile.source == "NFQWS_ARGS_CUSTOM")
        .map(|profile| profile.name.clone())
        .ok_or_else(|| "Профиль изменился при построении предпросмотра".to_owned())?;
    Ok(FreezeProfileJoinPreview {
        operation_id: operation.id.clone(),
        result_index,
        asn,
        profile_index,
        profile_name: profile_name.clone(),
        list_id: list_id.clone(),
        prefix_count: prefixes.len(),
        original_revision: revision(original_content.as_bytes()),
        original_content: original_content.to_owned(),
        proposed_content,
        summary: format!(
            "AS{asn}: {} префиксов будут записаны в {list_id} и добавлены в профиль #{profile_index} {profile_name}.",
            prefixes.len()
        ),
    })
}

async fn fetch_ripe_announced_prefixes(asn: u32) -> Result<Vec<String>, String> {
    if asn == 0 {
        return Err("Некорректный номер AS".into());
    }
    let url = format!("https://stat.ripe.net/data/announced-prefixes/data.json?resource=AS{asn}");
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
    .chain(std::iter::once(url))
    .collect::<Vec<_>>();
    let (status, body) =
        site_diagnostics::capture(site_diagnostics::curl_program(), &args, 24, 1024 * 1024 + 1)
            .await?;
    if status != 0 {
        return Err("RIPEstat не смог вернуть анонсируемые префиксы AS".into());
    }
    freeze_profile_join::parse_announced_prefixes(&body)
}

async fn read_profile_list_contents(
    config: &str,
    lists_directory: &Path,
) -> Result<HashMap<String, String>, String> {
    let profiles = nfqws2_ui::runtime_profiles::from_config(config, "freeze-join")?;
    let mut contents = HashMap::new();
    for profile in profiles
        .profiles
        .iter()
        .filter(|profile| profile.source == "NFQWS_ARGS_CUSTOM")
    {
        for reference in freeze_profile_join::profile_list_references(&profile.args)? {
            let Ok(path) = resolve_foreign_list_path(&reference, lists_directory) else {
                continue;
            };
            let Ok(file) = tokio::fs::File::open(path).await else {
                continue;
            };
            let mut bytes = Vec::new();
            if file
                .take((MAX_LIST_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .await
                .is_err()
                || bytes.len() > MAX_LIST_BYTES
            {
                continue;
            }
            if let Ok(content) = String::from_utf8(bytes) {
                contents.insert(reference, content);
            }
        }
    }
    Ok(contents)
}

fn insert_first_strategy_steps(current: &str, steps: &[String]) -> Result<String, String> {
    if steps.is_empty()
        || steps
            .iter()
            .any(|step| !step.starts_with("--lua-desync=") || step.chars().any(char::is_whitespace))
    {
        return Err("Разрешены только параметры --lua-desync без пробелов".into());
    }
    if current.contains(['$', '`', '\\', ';', '|', '&', '\0', '\'', '"', '#']) {
        return Err("NFQWS_ARGS содержит shell-синтаксис; безопасное применение недоступно".into());
    }
    let tokens = current.split_whitespace().collect::<Vec<_>>();
    let existing = tokens
        .iter()
        .filter(|token| token.starts_with("--lua-desync="))
        .map(|token| {
            if token.starts_with("--lua-desync=circular:") || *token == "--lua-desync=circular" {
                return Ok((*token).to_string());
            }
            let mut parts = token.split(':').map(str::to_string).collect::<Vec<_>>();
            let strategy_indexes = parts
                .iter()
                .enumerate()
                .filter_map(|(index, part)| {
                    part.strip_prefix("strategy=")
                        .map(|value| (index, value.to_string()))
                })
                .collect::<Vec<_>>();
            if strategy_indexes.len() > 1 {
                return Err("В NFQWS_ARGS у шага указано несколько номеров стратегии".into());
            }
            if let Some((index, value)) = strategy_indexes.first() {
                let number = value
                    .parse::<u32>()
                    .map_err(|_| "В NFQWS_ARGS найден некорректный номер стратегии")?;
                parts[*index] = format!("strategy={}", number.saturating_add(1));
            } else {
                parts.push("strategy=2".into());
            }
            Ok(parts.join(":"))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let first = steps
        .iter()
        .map(|step| {
            let mut parts = step
                .split(':')
                .map(str::to_string)
                .filter(|part| !part.starts_with("strategy="))
                .collect::<Vec<_>>();
            parts.push("strategy=1".into());
            parts.join(":")
        })
        .collect::<Vec<_>>();
    let mut output = Vec::with_capacity(tokens.len() + first.len());
    let mut inserted = false;
    let mut existing_index = 0;
    for token in tokens {
        if !inserted
            && token.starts_with("--lua-desync=")
            && token != "--lua-desync=circular"
            && !token.starts_with("--lua-desync=circular:")
        {
            output.extend(first.iter().cloned());
            inserted = true;
        }
        if token.starts_with("--lua-desync=") {
            output.push(
                existing
                    .get(existing_index)
                    .cloned()
                    .unwrap_or_else(|| token.to_string()),
            );
            existing_index += 1;
        } else {
            output.push(token.to_string());
        }
    }
    if !inserted {
        output.extend(first);
    }
    Ok(output.join("\n"))
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
        "quic" => StrategyProtocol::Quic,
        _ => return Err("Для dry-run доступны только HTTP, HTTPS и QUIC кандидаты".into()),
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

async fn validate_profile_config_dry_run(
    source: &str,
    paths: &nfqws2_ui::Nfqws2Paths,
) -> Result<String, String> {
    let directory =
        std::env::temp_dir().join(format!("nfqws2-ui-profile-check-{}", new_update_id()));
    let mut cleanup = ForeignAutoListCleanup {
        path: directory.clone(),
        armed: false,
    };
    let mut auto_lists = Vec::new();
    let arguments = external_config_args(
        source,
        &directory.join("auto.list"),
        &paths.lua_dir,
        &paths.blobs_dir,
        &paths.lists_dir,
        &mut auto_lists,
        false,
    )?;
    prepare_foreign_auto_lists(&directory, &auto_lists, &mut cleanup).await?;
    let argv = foreign_config_dry_run_argv(&paths.binary.to_string_lossy(), source, &arguments)?;
    validate_strategy_dry_run(&argv).await
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
    match build_freeze_strategy_preview(
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

async fn preview_freeze_profile_join(
    State(state): State<Arc<AppState>>,
    Json(request): Json<FreezeProfileJoinRequest>,
) -> Response {
    if !valid_site_trace_id(&request.operation_id) {
        return error(StatusCode::NOT_FOUND, "Результат теста 16 КБ не найден");
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
    if active_process_snapshot().is_none() {
        return error(
            StatusCode::CONFLICT,
            "Не найден однозначно работающий nfqws2 для проверки профиля",
        );
    }
    let Some(result) = scan.results.get(request.result_index) else {
        return error(
            StatusCode::NOT_FOUND,
            "Проверенная сеть не найдена в результате сканирования",
        );
    };
    let freeze_diagnostic::AsnVerification::Verified { asn } = result.asn else {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "ASN сети не подтверждён RIPEstat",
        );
    };
    let Some(paths) = discover_system_paths() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Пути установленного nfqws2 не обнаружены однозначно",
        );
    };
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
    let content = match String::from_utf8(original) {
        Ok(content) => content,
        Err(_) => {
            return error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Конфигурация не является UTF-8 текстом",
            );
        }
    };
    let lists = match read_profile_list_contents(&content, &paths.lists_dir).await {
        Ok(lists) => lists,
        Err(message) => return error(StatusCode::UNPROCESSABLE_ENTITY, message),
    };
    let prefixes = match fetch_ripe_announced_prefixes(asn).await {
        Ok(prefixes) => prefixes,
        Err(message) => return error(StatusCode::BAD_GATEWAY, message),
    };
    match build_freeze_profile_join_preview(
        &scan,
        request.result_index,
        &content,
        &lists,
        &prefixes,
    ) {
        Ok(preview) => {
            match freeze_profile_join::asn_list_exists(&paths.lists_dir, &preview.list_id) {
                Ok(false) => Json(preview).into_response(),
                Ok(true) => error(
                    StatusCode::CONFLICT,
                    "ASN-список с таким именем уже существует; он не будет перезаписан",
                ),
                Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
            }
        }
        Err(message) => error(StatusCode::UNPROCESSABLE_ENTITY, message),
    }
}

async fn apply_freeze_profile_join(
    State(state): State<Arc<AppState>>,
    Json(request): Json<FreezeProfileJoinApplyRequest>,
) -> Response {
    if !valid_site_trace_id(&request.operation_id) {
        return error(StatusCode::NOT_FOUND, "Результат теста 16 КБ не найден");
    }
    let Ok(_diagnostic_guard) = state.site_diagnostic_lock.clone().try_lock_owned() else {
        return error(
            StatusCode::CONFLICT,
            "Сначала дождитесь завершения текущей сетевой диагностики",
        );
    };
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
    let Some((active_pid, active_args)) = active_process_snapshot() else {
        return error(
            StatusCode::CONFLICT,
            "Не найден однозначно работающий nfqws2 для изменения профиля",
        );
    };
    let Some(result) = scan.results.get(request.result_index) else {
        return error(
            StatusCode::NOT_FOUND,
            "Проверенная сеть не найдена в результате сканирования",
        );
    };
    let freeze_diagnostic::AsnVerification::Verified { asn } = result.asn else {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "ASN сети не подтверждён RIPEstat",
        );
    };
    let Some(paths) = discover_system_paths() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Пути установленного nfqws2 не обнаружены однозначно",
        );
    };
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
    if revision(&original) != request.expected_revision {
        return error(
            StatusCode::CONFLICT,
            "Конфигурация изменилась после предпросмотра; откройте preview снова",
        );
    }
    let lists = match read_profile_list_contents(&content, &paths.lists_dir).await {
        Ok(lists) => lists,
        Err(message) => return error(StatusCode::UNPROCESSABLE_ENTITY, message),
    };
    let prefixes = match fetch_ripe_announced_prefixes(asn).await {
        Ok(prefixes) => prefixes,
        Err(message) => return error(StatusCode::BAD_GATEWAY, message),
    };
    let preview = match build_freeze_profile_join_preview(
        &scan,
        request.result_index,
        &content,
        &lists,
        &prefixes,
    ) {
        Ok(preview) => preview,
        Err(message) => return error(StatusCode::UNPROCESSABLE_ENTITY, message),
    };
    if preview.original_revision != request.expected_revision {
        return error(
            StatusCode::CONFLICT,
            "Конфигурация изменилась после предпросмотра; откройте preview снова",
        );
    }
    match freeze_profile_join::asn_list_exists(&paths.lists_dir, &preview.list_id) {
        Ok(false) => {}
        Ok(true) => {
            return error(
                StatusCode::CONFLICT,
                "ASN-список с таким именем уже существует; он не будет перезаписан",
            );
        }
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
    let list_content = match managed_asn_lists::build_list_content(asn, &prefixes) {
        Ok(content) => content,
        Err(message) => return error(StatusCode::UNPROCESSABLE_ENTITY, message),
    };
    if let Err(message) = validate_list_content_with_kind(&preview.list_id, &list_content, true) {
        return error(StatusCode::UNPROCESSABLE_ENTITY, message);
    }
    let list_path = match freeze_profile_join::create_list_exclusive(
        &paths.lists_dir,
        &preview.list_id,
        &list_content,
    ) {
        Ok(path) => path,
        Err(message) => return error(StatusCode::CONFLICT, message),
    };
    if let Err(message) = validate_profile_config_dry_run(&preview.proposed_content, &paths).await {
        let message = cleanup_freeze_join_list(&list_path, message).await;
        return error(StatusCode::UNPROCESSABLE_ENTITY, message);
    }
    if !matches!(active_process_snapshot(), Some((pid, ref args)) if pid == active_pid && args == &active_args)
    {
        let cleanup_note = cleanup_freeze_join_list(&list_path, String::new()).await;
        let cleanup_note = if cleanup_note.is_empty() {
            String::new()
        } else {
            format!(". {}", cleanup_note.trim_start_matches(". "))
        };
        return error(
            StatusCode::CONFLICT,
            format!(
                "Активный процесс nfqws2 изменился во время проверки; конфигурация не изменена{cleanup_note}"
            ),
        );
    }
    let monitoring = match load_monitoring_store(&state.data_dir).await {
        Ok(monitoring) => monitoring,
        Err(message) => {
            let message = cleanup_freeze_join_list(&list_path, message).await;
            return error(StatusCode::INTERNAL_SERVER_ERROR, message);
        }
    };
    let hosts = strategy_apply_verification_hosts(&result.host, &monitoring);
    let list_bytes = match tokio::fs::read(&list_path).await {
        Ok(bytes) => bytes,
        Err(io_error) => {
            let message = cleanup_freeze_join_list(&list_path, io_error.to_string()).await;
            return error(StatusCode::INTERNAL_SERVER_ERROR, message);
        }
    };
    let feed = ManagedAsnList::new(asn, &preview.list_id, prefixes.len(), &list_bytes);
    // Keep the scheduler from pruning this just-registered feed before the config transaction
    // makes its list reference active; release the lock only after apply/rollback finishes.
    let _managed_refresh_guard = state.managed_asn_refresh_lock.lock().await;
    let mut managed_store = ManagedAsnListStore::load(&state.data_dir);
    managed_store.register(feed);
    if let Err(message) = managed_store.save(&state.data_dir) {
        let message = cleanup_freeze_join_list(&list_path, message).await;
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    let write = ConfigWrite {
        content: preview.proposed_content,
        revision: preview.original_revision,
    };
    let monitor_policy_route = resolve_monitoring_policy_route(&state).await;
    match apply_freeze_join_transaction(
        &state,
        write,
        &list_path,
        &state.data_dir,
        asn,
        || async { service_action(ServiceAction::Restart).await },
        || {
            let monitor_policy_route = monitor_policy_route.clone();
            let hosts = hosts.clone();
            async move {
                verify_strategy_hosts_with(hosts, move |host| {
                    let monitor_policy_route = monitor_policy_route.clone();
                    async move {
                        let report = check_monitoring_host(host, monitor_policy_route).await;
                        if report.current.status == "ok" {
                            Ok(())
                        } else {
                            Err(report.summary)
                        }
                    }
                })
                .await
            }
        },
    )
    .await
    {
        Ok(applied) => Json(applied).into_response(),
        Err(message) => {
            if message.contains(WRITE_REVISION_CONFLICT)
                || message.contains("Файл изменился после загрузки")
            {
                error(
                    StatusCode::CONFLICT,
                    message.replace(WRITE_REVISION_CONFLICT, "Конфигурация изменилась во время применения; проверьте конфиг и список перед повтором"),
                )
            } else {
                error(StatusCode::BAD_GATEWAY, message)
            }
        }
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
        hosts: Vec::new(),
        podkop_hosts: Vec::new(),
        multi_site_results: Vec::new(),
        multi_site_baseline: Vec::new(),
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
    let preview = match if request.source == "freezeScan" {
        build_freeze_strategy_preview(
            &operation,
            candidate_index,
            &content,
            current_revision.clone(),
            &functions,
            &blobs,
        )
    } else {
        build_strategy_pick_preview(
            &operation,
            candidate_index,
            &content,
            current_revision.clone(),
            &functions,
            &blobs,
        )
    } {
        Ok(preview) => preview,
        Err(message) => return error(StatusCode::UNPROCESSABLE_ENTITY, message),
    };
    let write = ConfigWrite {
        content: preview.proposed_content,
        revision: current_revision,
    };
    let monitoring = match load_monitoring_store(&state.data_dir).await {
        Ok(monitoring) => monitoring,
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    };
    let verify_hosts = strategy_apply_verification_hosts_for_protocol(
        &operation.host,
        &operation.protocol,
        &monitoring,
    );
    let verify_quic = operation.protocol == "quic";
    let verification_binary = paths.binary.to_string_lossy().into_owned();
    let monitor_policy_route = resolve_monitoring_policy_route(&state).await;
    match apply_config_transaction_with_verification(
        &state,
        write,
        || async { service_action(ServiceAction::Restart).await },
        || {
            let monitor_policy_route = monitor_policy_route.clone();
            let verify_hosts = verify_hosts.clone();
            let verification_binary = verification_binary.clone();
            async move {
                verify_strategy_hosts_with(verify_hosts, move |host| {
                    let monitor_policy_route = monitor_policy_route.clone();
                    let verification_binary = verification_binary.clone();
                    async move {
                        if verify_quic {
                            let addresses = site_diagnostics::public_ipv4_references(&host).await;
                            let packet =
                                nfqws2_ui::quic_probe::generate(&verification_binary, &host)
                                    .await?;
                            let mut last_detail =
                                "QUIC Initial не получил подтверждённого ответа".to_owned();
                            for address in addresses {
                                let IpAddr::V4(address) = address else {
                                    continue;
                                };
                                if !site_diagnostics::public_ip(IpAddr::V4(address)) {
                                    continue;
                                }
                                let result =
                                    nfqws2_ui::quic_probe::probe(&packet, address, true).await;
                                if result.status == "ok" {
                                    return Ok(());
                                }
                                last_detail = result.detail;
                            }
                            return Err(format!(
                                "Проверка QUIC для {host} не прошла: {last_detail}"
                            ));
                        }
                        let report = check_monitoring_host(host, monitor_policy_route).await;
                        if report.current.status == "ok" {
                            Ok(())
                        } else {
                            Err(report.summary)
                        }
                    }
                })
                .await
            }
        },
    )
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
    interception: Option<InterceptionStatus>,
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
    default_network_interfaces: Vec<String>,
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

#[derive(Clone, Serialize)]
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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PendingConfigApplyJournal {
    config_path: String,
    original_content: Vec<u8>,
    candidate_revision: String,
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

#[derive(Debug, Clone)]
enum MonitoringPolicyRoute {
    Standard,
    Marked(String),
    Unavailable(String),
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
struct LogClearRequest {
    source: String,
}

#[derive(Deserialize)]
struct UpdateInstallRequest {
    tag: String,
}

#[derive(Deserialize, Default)]
struct UiReleaseQuery {
    #[serde(default)]
    force: bool,
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

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct PanelRelease {
    tag: String,
    published_at: Option<String>,
    notes: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct UiReleaseCacheEntry {
    checked_at: u64,
    releases: Vec<PanelRelease>,
    #[serde(default)]
    last_error_at: Option<u64>,
    #[serde(default)]
    last_error: Option<String>,
}

fn ui_release_cache_is_fresh(entry: &UiReleaseCacheEntry, now: u64) -> bool {
    if entry
        .last_error_at
        .is_some_and(|checked| now.saturating_sub(checked) < UI_RELEASE_ERROR_CACHE_TTL.as_secs())
    {
        return true;
    }
    entry.checked_at > 0 && now.saturating_sub(entry.checked_at) < UI_RELEASE_CACHE_TTL.as_secs()
}

fn ui_release_check_retry_after(
    checked_at: u64,
    warning: Option<&str>,
    now: u64,
) -> std::time::Duration {
    if warning.is_some() {
        return UI_RELEASE_ERROR_CACHE_TTL;
    }
    let next_check = checked_at.saturating_add(UI_RELEASE_CACHE_TTL.as_secs());
    std::time::Duration::from_secs(next_check.saturating_sub(now).max(1))
}

fn ui_release_cache_path(data_dir: &Path) -> PathBuf {
    data_dir.join("ui-releases-cache.json")
}

async fn read_ui_release_cache(path: &Path) -> Option<UiReleaseCacheEntry> {
    let metadata = tokio::fs::metadata(path).await.ok()?;
    if metadata.len() > MAX_RELEASE_RESPONSE_BYTES as u64 {
        return None;
    }
    let bytes = tokio::fs::read(path).await.ok()?;
    serde_json::from_slice(&bytes).ok()
}

async fn write_ui_release_cache(path: &Path, entry: &UiReleaseCacheEntry) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Не удалось определить каталог кэша релизов UI".to_owned())?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|error| format!("Не удалось создать каталог кэша релизов UI: {error}"))?;
    let bytes = serde_json::to_vec(entry).map_err(|error| error.to_string())?;
    if bytes.len() > MAX_RELEASE_RESPONSE_BYTES {
        return Err("Кэш релизов UI превышает ограничение размера".into());
    }
    let temporary = path.with_extension(format!("{}.tmp", new_update_id()));
    tokio::fs::write(&temporary, bytes)
        .await
        .map_err(|error| format!("Не удалось записать кэш релизов UI: {error}"))?;
    #[cfg(unix)]
    if let Err(error) =
        tokio::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o600)).await
    {
        let _ = tokio::fs::remove_file(&temporary).await;
        return Err(format!("Не удалось защитить кэш релизов UI: {error}"));
    }
    #[cfg(windows)]
    if tokio::fs::try_exists(path).await.unwrap_or(false)
        && let Err(error) = tokio::fs::remove_file(path).await
    {
        let _ = tokio::fs::remove_file(&temporary).await;
        return Err(format!("Не удалось заменить кэш релизов UI: {error}"));
    }
    if let Err(error) = tokio::fs::rename(&temporary, path).await {
        let _ = tokio::fs::remove_file(&temporary).await;
        return Err(format!("Не удалось обновить кэш релизов UI: {error}"));
    }
    Ok(())
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct CoreRelease {
    tag: String,
    published_at: Option<String>,
    notes: String,
    asset: String,
    sha256: String,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
struct CoreReleaseCacheEntry {
    checked_at: u64,
    architecture: String,
    releases: Vec<CoreRelease>,
    #[serde(default)]
    last_error_at: Option<u64>,
}

#[derive(Deserialize, Default)]
struct CoreReleaseQuery {
    #[serde(default)]
    force: bool,
}

fn core_release_cache_is_fresh(
    entry: &CoreReleaseCacheEntry,
    architecture: &str,
    now: u64,
) -> bool {
    if entry.architecture != architecture {
        return false;
    }
    if entry
        .last_error_at
        .is_some_and(|checked| now.saturating_sub(checked) < CORE_RELEASE_ERROR_CACHE_TTL.as_secs())
    {
        return true;
    }
    entry.checked_at > 0 && now.saturating_sub(entry.checked_at) < CORE_RELEASE_CACHE_TTL.as_secs()
}

fn should_use_core_release_cache(
    entry: &CoreReleaseCacheEntry,
    architecture: &str,
    now: u64,
    force: bool,
) -> bool {
    !force && core_release_cache_is_fresh(entry, architecture, now)
}

fn core_release_cache_path(data_dir: &Path) -> PathBuf {
    data_dir.join("core-releases-cache.json")
}

async fn read_core_release_cache(path: &Path) -> Option<CoreReleaseCacheEntry> {
    let metadata = tokio::fs::metadata(path).await.ok()?;
    if metadata.len() > MAX_RELEASE_RESPONSE_BYTES as u64 {
        return None;
    }
    let bytes = tokio::fs::read(path).await.ok()?;
    serde_json::from_slice(&bytes).ok()
}

async fn write_core_release_cache(
    path: &Path,
    entry: &CoreReleaseCacheEntry,
) -> Result<(), String> {
    let parent = path
        .parent()
        .ok_or_else(|| "Не удалось определить каталог кэша релизов nfqws2".to_owned())?;
    tokio::fs::create_dir_all(parent)
        .await
        .map_err(|error| format!("Не удалось создать каталог кэша релизов nfqws2: {error}"))?;
    let bytes = serde_json::to_vec(entry).map_err(|error| error.to_string())?;
    if bytes.len() > MAX_RELEASE_RESPONSE_BYTES {
        return Err("Кэш релизов nfqws2 превышает ограничение размера".into());
    }
    let temporary = path.with_extension(format!("{}.tmp", new_update_id()));
    tokio::fs::write(&temporary, bytes)
        .await
        .map_err(|error| format!("Не удалось записать кэш релизов nfqws2: {error}"))?;
    #[cfg(unix)]
    if let Err(error) =
        tokio::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o600)).await
    {
        let _ = tokio::fs::remove_file(&temporary).await;
        return Err(format!("Не удалось защитить кэш релизов nfqws2: {error}"));
    }
    #[cfg(windows)]
    if tokio::fs::try_exists(path).await.unwrap_or(false)
        && let Err(error) = tokio::fs::remove_file(path).await
    {
        let _ = tokio::fs::remove_file(&temporary).await;
        return Err(format!("Не удалось заменить кэш релизов nfqws2: {error}"));
    }
    if let Err(error) = tokio::fs::rename(&temporary, path).await {
        let _ = tokio::fs::remove_file(&temporary).await;
        return Err(format!("Не удалось обновить кэш релизов nfqws2: {error}"));
    }
    Ok(())
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct CoreUpdateStatus {
    latest_tag: Option<String>,
    installed_tag: Option<String>,
    update_available: bool,
    checked_at: u64,
}

async fn load_cached_core_releases(
    state: &AppState,
    architecture: &str,
    force: bool,
) -> Result<(Vec<CoreRelease>, u64, Option<String>), String> {
    let mut cache = state.core_release_cache.lock().await;
    if cache.is_none() {
        *cache = read_core_release_cache(&core_release_cache_path(&state.data_dir)).await;
    }
    let now = now_unix_seconds();
    if let Some(entry) = cache
        .as_ref()
        .filter(|entry| should_use_core_release_cache(entry, architecture, now, force))
    {
        let warning = entry
            .last_error_at
            .filter(|checked| now.saturating_sub(*checked) < CORE_RELEASE_ERROR_CACHE_TTL.as_secs())
            .map(|_| "GitHub недоступен; показан сохранённый список релизов".to_owned());
        return Ok((entry.releases.clone(), entry.checked_at, warning));
    }
    let stale = cache
        .as_ref()
        .filter(|entry| entry.architecture == architecture)
        .map(|entry| (entry.releases.clone(), entry.checked_at));
    match fetch_github_url(NFQWS2_RELEASES_API).await {
        Ok(json) => match eligible_core_releases(&json, architecture) {
            Ok(releases) => {
                let checked_at = now_unix_seconds();
                let entry = CoreReleaseCacheEntry {
                    checked_at,
                    architecture: architecture.to_owned(),
                    releases: releases.clone(),
                    last_error_at: None,
                };
                *cache = Some(entry.clone());
                if let Err(error) =
                    write_core_release_cache(&core_release_cache_path(&state.data_dir), &entry)
                        .await
                {
                    tracing::warn!("cannot persist nfqws2 release cache: {error}");
                }
                Ok((releases, checked_at, None))
            }
            Err(message) => Err(message),
        },
        Err(message) => {
            let mut entry = stale
                .as_ref()
                .map(|(releases, checked_at)| CoreReleaseCacheEntry {
                    checked_at: *checked_at,
                    architecture: architecture.to_owned(),
                    releases: releases.clone(),
                    last_error_at: None,
                })
                .or_else(|| {
                    cache
                        .as_ref()
                        .cloned()
                        .filter(|entry| entry.architecture == architecture && entry.checked_at == 0)
                })
                .unwrap_or_else(|| CoreReleaseCacheEntry {
                    checked_at: 0,
                    architecture: architecture.to_owned(),
                    releases: Vec::new(),
                    last_error_at: None,
                });
            entry.last_error_at = Some(now_unix_seconds());
            *cache = Some(entry.clone());
            if let Err(error) =
                write_core_release_cache(&core_release_cache_path(&state.data_dir), &entry).await
            {
                tracing::warn!("cannot persist nfqws2 release-check backoff: {error}");
            }
            if entry.checked_at > 0 {
                Ok((entry.releases, entry.checked_at, Some(message)))
            } else {
                Err(message)
            }
        }
    }
}

fn core_update_available(latest: Option<&str>, installed: Option<&str>) -> bool {
    let normalize = |tag: &str| {
        tag.trim()
            .strip_prefix('v')
            .unwrap_or(tag.trim())
            .to_owned()
    };
    match (latest, installed) {
        (Some(latest), Some(installed)) => normalize(latest) != normalize(installed),
        _ => false,
    }
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeProfilesWrite {
    profiles: Vec<Vec<String>>,
    revision: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct RuntimeProfileSettingsWrite {
    variables: std::collections::HashMap<String, Vec<String>>,
    revision: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct UiAppearanceSettings {
    theme: String,
    layout: String,
    #[serde(default = "default_accent")]
    accent: String,
    #[serde(default = "default_status_bar")]
    status_bar: String,
    #[serde(default = "default_true")]
    remind_windows_tcp_timestamps: bool,
    #[serde(default = "default_true")]
    show_config_warnings: bool,
}

fn default_accent() -> String {
    "blue".into()
}

fn default_status_bar() -> String {
    "standard".into()
}

fn default_true() -> bool {
    true
}

#[derive(Serialize)]
struct BackupsResponse {
    backups: Vec<BackupItem>,
}

#[derive(Serialize)]
struct ListsResponse {
    files: Vec<ListSummary>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackageLeftoversDeleteRequest {
    names: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct PackageLeftoverCompareRequest {
    name: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct PackageLeftoversResponse {
    items: Vec<package_leftovers::PackageLeftover>,
    total_bytes: u64,
    deleted: Option<usize>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ListSummary {
    id: String,
    path: String,
    readable: bool,
    writable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    managed_asn: Option<ManagedAsnListSummary>,
    usage_known: bool,
    used_by: Vec<ListProfileUsage>,
    validation_required: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ListProfileUsage {
    profile_index: usize,
    profile_name: String,
    role: &'static str,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ManagedAsnListSummary {
    asn: u32,
    prefix_count: usize,
    last_attempt_date: Option<String>,
    last_success_date: Option<String>,
    last_error: Option<String>,
    paused: bool,
    pending: bool,
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

fn valid_panel_release_tag(tag: &str) -> bool {
    let Some(version) = tag.strip_prefix('v') else {
        return false;
    };
    let base = version.split_once("-r").map_or(version, |(base, _)| base);
    if !valid_release_tag(&format!("v{base}")) {
        return false;
    }
    match version.split_once("-r") {
        None => true,
        Some((_, revision)) => {
            !revision.is_empty()
                && !revision.starts_with('0')
                && revision.bytes().all(|byte| byte.is_ascii_digit())
        }
    }
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
            if release.draft || release.prerelease || !valid_panel_release_tag(&release.tag_name) {
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

fn numeric_release_version(version: &str) -> Option<(u64, u64, u64, u64)> {
    let version = version.strip_prefix('v').unwrap_or(version);
    let (version, revision) = match version.split_once("-r") {
        Some((version, revision)) => (version, revision.parse().ok()?),
        None => (version, 0),
    };
    let mut parts = version.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next()?.parse().ok()?;
    parts
        .next()
        .is_none()
        .then_some((major, minor, patch, revision))
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
            let retry_after = match load_cached_ui_releases(&state, false).await {
                Ok((releases, checked_at, warning)) => {
                    cache_ui_update_status(&state.ui_update_status, &releases, checked_at);
                    ui_release_check_retry_after(checked_at, warning.as_deref(), now_unix_seconds())
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

async fn spawn_github_wget(url: &str) -> Result<tokio::process::Child, std::io::Error> {
    spawn_github_wget_with_candidates(url, &github_wget_candidates()).await
}

async fn spawn_github_wget_with_candidates(
    url: &str,
    candidates: &[&str],
) -> Result<tokio::process::Child, std::io::Error> {
    let mut last_error = None;
    for executable in candidates {
        match tokio::process::Command::new(executable)
            .args(["-q", "-T", "15", "-O", "-", url])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
        {
            Ok(child) => return Ok(child),
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "no GitHub download client candidates configured",
        )
    }))
}

fn github_release_api_candidates(url: &str) -> Vec<String> {
    if !url.starts_with("https://api.github.com/repos/") {
        return vec![url.to_owned()];
    }
    std::iter::once(url.to_owned())
        .chain(
            GITHUB_API_FALLBACKS
                .iter()
                .map(|proxy| format!("{proxy}{url}")),
        )
        .collect()
}

async fn fetch_github_url(url: &'static str) -> Result<String, String> {
    let mut last_error = None;
    for (index, candidate) in github_release_api_candidates(url).iter().enumerate() {
        match fetch_github_url_once(candidate).await {
            Ok(json) => {
                if serde_json::from_str::<Vec<serde_json::Value>>(&json).is_err() {
                    last_error = Some("GitHub Releases вернул некорректный JSON".to_owned());
                    continue;
                }
                if index > 0 {
                    tracing::warn!("GitHub Releases API responded through fallback route {index}");
                }
                return Ok(json);
            }
            Err(error) => last_error = Some(error),
        }
    }
    Err(last_error.unwrap_or_else(|| "GitHub Releases недоступен".into()))
}

async fn fetch_github_url_once(url: &str) -> Result<String, String> {
    let mut child = spawn_github_wget(url)
        .await
        .map_err(|error| format!("Не удалось запустить wget: {error}"))?;
    let mut bytes = Vec::new();
    let stdout = child
        .stdout
        .take()
        .ok_or("Не удалось прочитать ответ GitHub")?;
    let read = tokio::time::timeout(
        std::time::Duration::from_secs(18),
        stdout
            .take((MAX_RELEASE_RESPONSE_BYTES + 1) as u64)
            .read_to_end(&mut bytes),
    )
    .await;
    if read.is_err() {
        let _ = child.kill().await;
        return Err("Истекло время ожидания ответа GitHub Releases".into());
    }
    if bytes.len() > MAX_RELEASE_RESPONSE_BYTES {
        let _ = child.kill().await;
        return Err("Ответ GitHub Releases превысил лимит 1 МБ".into());
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

async fn get_core_releases(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CoreReleaseQuery>,
) -> Response {
    let Some(architecture) = entware_architecture() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Не удалось определить поддерживаемую архитектуру Entware nfqws2",
        );
    };
    match load_cached_core_releases(&state, &architecture, query.force).await {
        Ok((releases, _, warning)) => {
            let installed_tag = std::process::Command::new("/opt/bin/opkg")
                .args(["status", "nfqws2-keenetic"])
                .output()
                .ok()
                .and_then(|output| installed_core_tag_from_output(&output.stdout));
            Json(serde_json::json!({ "releases": releases, "installedTag": installed_tag, "warning": warning }))
                .into_response()
        }
        Err(message) => error(StatusCode::BAD_GATEWAY, message),
    }
}

async fn get_core_update_status(State(state): State<Arc<AppState>>) -> Response {
    let Some(architecture) = entware_architecture() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Не удалось определить поддерживаемую архитектуру Entware nfqws2",
        );
    };
    let (releases, checked_at, _) =
        match load_cached_core_releases(&state, &architecture, false).await {
            Ok(value) => value,
            Err(message) => return error(StatusCode::BAD_GATEWAY, message),
        };
    let installed_tag = std::process::Command::new("/opt/bin/opkg")
        .args(["status", "nfqws2-keenetic"])
        .output()
        .ok()
        .and_then(|output| installed_core_tag_from_output(&output.stdout));
    let latest_tag = releases.first().map(|release| release.tag.clone());
    let update_available = core_update_available(latest_tag.as_deref(), installed_tag.as_deref());
    Json(CoreUpdateStatus {
        latest_tag,
        installed_tag,
        update_available,
        checked_at,
    })
    .into_response()
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
    let json = fetch_github_url(GITHUB_RELEASES_API).await?;
    eligible_panel_releases(&json)
}

async fn load_cached_ui_releases(
    state: &AppState,
    force: bool,
) -> Result<(Vec<PanelRelease>, u64, Option<String>), String> {
    let mut cache = state.ui_release_cache.lock().await;
    if cache.is_none() {
        *cache = read_ui_release_cache(&ui_release_cache_path(&state.data_dir)).await;
    }
    let now = now_unix_seconds();
    if !force
        && let Some(entry) = cache
            .as_ref()
            .filter(|entry| ui_release_cache_is_fresh(entry, now))
    {
        if entry.releases.is_empty() && entry.last_error.is_some() {
            return Err(entry
                .last_error
                .clone()
                .unwrap_or_else(|| "В GitHub Releases нет подходящих версий UI".into()));
        }
        return Ok((
            entry.releases.clone(),
            entry.checked_at,
            entry.last_error.clone(),
        ));
    }

    match fetch_github_releases().await {
        Ok(releases) => {
            let entry = UiReleaseCacheEntry {
                checked_at: now_unix_seconds(),
                releases: releases.clone(),
                last_error_at: None,
                last_error: None,
            };
            *cache = Some(entry.clone());
            if let Err(error) =
                write_ui_release_cache(&ui_release_cache_path(&state.data_dir), &entry).await
            {
                tracing::warn!("cannot persist UI release cache: {error}");
            }
            Ok((releases, entry.checked_at, None))
        }
        Err(message) => {
            let mut entry = cache.clone().unwrap_or_else(|| UiReleaseCacheEntry {
                checked_at: 0,
                releases: Vec::new(),
                last_error_at: None,
                last_error: None,
            });
            entry.last_error_at = Some(now_unix_seconds());
            entry.last_error = Some(message.clone());
            *cache = Some(entry.clone());
            if let Err(error) =
                write_ui_release_cache(&ui_release_cache_path(&state.data_dir), &entry).await
            {
                tracing::warn!("cannot persist UI release-check backoff: {error}");
            }
            if entry.releases.is_empty() {
                Err(message)
            } else {
                Ok((entry.releases, entry.checked_at, Some(message)))
            }
        }
    }
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

async fn get_github_releases(
    State(state): State<Arc<AppState>>,
    Query(query): Query<UiReleaseQuery>,
) -> Response {
    match load_cached_ui_releases(&state, query.force).await {
        Ok((releases, _, warning)) => {
            let update_status =
                cache_ui_update_status(&state.ui_update_status, &releases, now_unix_seconds());
            Json(serde_json::json!({ "releases": releases, "updateStatus": update_status, "warning": warning })).into_response()
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
    if !valid_panel_release_tag(&request.tag) {
        return error(StatusCode::BAD_REQUEST, "Недопустимый тег релиза");
    }
    let _guard = state.update_lock.lock().await;
    let releases = match load_cached_ui_releases(&state, false).await {
        Ok((releases, _, _)) => releases,
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
        default_network_interfaces: default_network_interfaces(
            &std::fs::read_to_string("/proc/net/route").unwrap_or_default(),
            &std::fs::read_to_string("/proc/net/ipv6_route").unwrap_or_default(),
        ),
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
    let interception = cached_interception_status(&state, service_state).await;
    let core_version = installed_core_version(&state).await;
    let system = current_system_info(&state).await;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    Json(StatusResponse {
        service: ServiceInfo {
            state: service_state,
            detail,
            checked_at: now,
            version: core_version,
        },
        system,
        panel: PanelInfo {
            version: env!("CARGO_PKG_VERSION"),
            port: state.port,
        },
        interception,
    })
    .into_response()
}

async fn current_system_info(state: &AppState) -> SystemInfo {
    let meminfo = tokio::fs::read_to_string("/proc/meminfo").await.ok();
    let (memory_used_bytes, memory_total_bytes) = meminfo
        .as_deref()
        .and_then(memory_usage_bytes)
        .map_or((None, None), |(used, total)| (Some(used), Some(total)));
    let cpu_percent = cpu_sample(state).await;
    cached_keenetic_system_stats(state)
        .await
        .unwrap_or(SystemInfo {
            memory_total_bytes,
            memory_used_bytes,
            cpu_percent,
            source: "proc",
        })
}

async fn installed_core_version(state: &AppState) -> Option<String> {
    let mut cache = state.installed_core_version_cache.lock().await;
    if let Some(version) = cache.get(Instant::now()) {
        return version;
    }
    let version = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        tokio::process::Command::new("/opt/bin/opkg")
            .args(["status", "nfqws2-keenetic"])
            .kill_on_drop(true)
            .output(),
    )
    .await
    .ok()
    .and_then(Result::ok)
    .and_then(|output| installed_core_tag_from_output(&output.stdout));
    cache.store(version.clone(), Instant::now());
    version
}

async fn cached_interception_status(
    state: &AppState,
    service_state: ServiceState,
) -> Option<InterceptionStatus> {
    const CACHE_TTL: std::time::Duration = std::time::Duration::from_secs(10);
    let mut cache = state.interception_status_cache.lock().await;
    if service_state != ServiceState::Running {
        *cache = InterceptionStatusCache::default();
        return None;
    }
    let now = Instant::now();
    if cache
        .checked_at
        .is_some_and(|checked_at| now.duration_since(checked_at) < CACHE_TTL)
    {
        return cache.visible.clone();
    }
    let observed = interception_status::read_iptables_status().await;
    let observed_at = Instant::now();
    let (visible, missing_since) = interception_status::visible_interception_status(
        observed,
        cache.missing_since,
        observed_at,
    );
    *cache = InterceptionStatusCache {
        checked_at: Some(observed_at),
        visible: visible.clone(),
        missing_since,
    };
    visible
}

async fn build_support_report(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SupportReportRequest>,
) -> Response {
    let platform = discover_system_paths()
        .map(|paths| {
            if paths.root == Path::new("/opt") {
                "Keenetic / Entware"
            } else {
                "OpenWrt"
            }
        })
        .unwrap_or("не определена")
        .to_owned();
    let service_state_value = service_status().await.0;
    let service_state = match service_state_value {
        ServiceState::Running => "запущена",
        ServiceState::Stopped => "остановлена",
        ServiceState::Error => "ошибка",
        ServiceState::Unknown => "неизвестно",
    }
    .to_owned();
    let core_version = installed_core_version(&state).await;
    let system = current_system_info(&state).await;
    let interception = cached_interception_status(&state, service_state_value).await;
    let interception_status = match interception {
        Some(status) if status.missing_hooks.is_empty() => "цепочки подключены".to_owned(),
        Some(status) => format!("нет переходов: {}", status.missing_hooks.join(", ")),
        None if service_state_value != ServiceState::Running => {
            "проверка пропущена: служба не запущена".to_owned()
        }
        None => {
            "не подтверждено: проверка недоступна или правила недавно перестраивались".to_owned()
        }
    };
    let running_profiles = active_process_snapshot()
        .map(|(_, args)| runtime_profiles(&args).len())
        .unwrap_or(0);

    let (config_errors, config_warnings, lists_count) =
        if let Some(config_path) = config_path_for_state(&state) {
            let content = match tokio::fs::read_to_string(&config_path).await {
                Ok(content) => content,
                Err(_) => {
                    return error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Не удалось прочитать конфигурацию для диагностики",
                    );
                }
            };
            let diagnostics = match validate_nfqws2_config(&content) {
                Ok(diagnostics) => diagnostics,
                Err(_) => {
                    return error(
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Не удалось проверить конфигурацию для диагностики",
                    );
                }
            };
            (
                diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
                    .count(),
                diagnostics
                    .iter()
                    .filter(|diagnostic| diagnostic.severity == DiagnosticSeverity::Warning)
                    .count(),
                list_ids_for_config(&config_path).len(),
            )
        } else {
            (0, 0, 0)
        };

    let monitoring = match load_monitoring_store(&state.data_dir).await {
        Ok(monitoring) => monitoring,
        Err(_) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "Не удалось прочитать сводку мониторинга",
            );
        }
    };
    let monitored_sites = support_site_labels(&monitoring.sites, request.hide_site_names);
    let report = format_support_report(&SupportReportData {
        platform,
        architecture: std::env::consts::ARCH.to_owned(),
        service_state,
        core_version,
        memory_used_bytes: system.memory_used_bytes,
        memory_total_bytes: system.memory_total_bytes,
        cpu_percent: system.cpu_percent,
        interception_status,
        running_profiles,
        monitored_sites,
        monitoring_enabled: monitoring.enabled,
        config_errors,
        config_warnings,
        lists_count,
    });
    Json(serde_json::json!({ "text": report })).into_response()
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

async fn cached_keenetic_system_stats(state: &AppState) -> Option<SystemInfo> {
    let mut cache = state.keenetic_system_stats_cache.lock().await;
    if let Some(stats) = cache.get(Instant::now()) {
        return stats;
    }
    let stats = keenetic_system_stats().await;
    cache.store(stats.clone(), Instant::now());
    stats
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
    if !snapshots::load_settings(&state.data_dir).auto_enabled {
        return Ok(());
    }
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
    let (references, ambiguous) = configured_list_references_for_flag(config_text, flag);
    (!ambiguous && references.len() <= 1)
        .then(|| references.into_iter().next())
        .flatten()
}

fn configured_list_references_for_flag(config_text: &str, flag: &str) -> (Vec<PathBuf>, bool) {
    let mut references = Vec::new();
    let mut ambiguous = false;
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
                ambiguous = true;
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
                if value.contains(['$', '`']) || value.starts_with(['~', '-']) {
                    ambiguous = true;
                } else {
                    let path = PathBuf::from(value);
                    if !references.contains(&path) {
                        references.push(path);
                    }
                }
            } else {
                ambiguous = true;
            }
            start_at = end;
        }
    }
    (references, ambiguous)
}

fn argv_has_ambiguous_list_reference(args: &[String], flag: &str) -> bool {
    for (index, argument) in args.iter().take(16_384).enumerate() {
        let value = if let Some(value) = argument
            .strip_prefix(flag)
            .and_then(|suffix| suffix.strip_prefix('='))
        {
            Some(value)
        } else if argument == flag {
            Some(args.get(index + 1).map(String::as_str).unwrap_or_default())
        } else {
            None
        };
        let Some(value) = value else {
            continue;
        };
        if value.is_empty() || value.contains(['$', '`', '\0']) || value.starts_with(['~', '-']) {
            return true;
        }
    }
    false
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

fn runtime_list_references(args: &[String]) -> Vec<(PathBuf, bool)> {
    const FLAGS: [(&str, bool); 5] = [
        ("--hostlist", false),
        ("--hostlist-auto", false),
        ("--hostlist-exclude", false),
        ("--ipset", true),
        ("--ipset-exclude", true),
    ];
    let mut references = Vec::new();
    for (index, arg) in args.iter().take(16_384).enumerate() {
        for (flag, is_ipset) in FLAGS {
            let value = arg
                .strip_prefix(flag)
                .and_then(|suffix| suffix.strip_prefix('='))
                .or_else(|| {
                    (arg == flag)
                        .then(|| args.get(index + 1).map(String::as_str))
                        .flatten()
                });
            let Some(value) = value.map(|value| value.trim_start_matches('@')) else {
                continue;
            };
            if value.is_empty()
                || value.len() > 4096
                || value.contains(['$', '`', '\0'])
                || value.starts_with('~')
            {
                continue;
            }
            let reference = (PathBuf::from(value), is_ipset);
            if !references.contains(&reference) {
                references.push(reference);
                if references.len() >= 512 {
                    return references;
                }
            }
        }
    }
    references
}

fn resolve_runtime_list_references(config: &Path, args: &[String]) -> Vec<(PathBuf, bool)> {
    let Some(config) = config.canonicalize().ok() else {
        return Vec::new();
    };
    let Some(root) = config.parent() else {
        return Vec::new();
    };
    let mut resolved = Vec::new();
    for (reference, is_ipset) in runtime_list_references(args) {
        let candidate = if reference.is_absolute() {
            reference
        } else {
            root.join(reference)
        };
        let Ok(relative) = candidate.strip_prefix(root) else {
            continue;
        };
        if relative.as_os_str().is_empty()
            || relative
                .components()
                .any(|component| !matches!(component, std::path::Component::Normal(_)))
        {
            continue;
        }
        let mut cursor = root.to_path_buf();
        let mut safe = true;
        for component in relative.components() {
            cursor.push(component.as_os_str());
            match std::fs::symlink_metadata(&cursor) {
                Ok(metadata) if !metadata.file_type().is_symlink() => {}
                _ => {
                    safe = false;
                    break;
                }
            }
        }
        if !safe {
            continue;
        }
        let Some(path) = cursor
            .canonicalize()
            .ok()
            .filter(|path| path.starts_with(root) && path.is_file())
        else {
            continue;
        };
        let entry = (path, is_ipset);
        if !resolved.contains(&entry) {
            resolved.push(entry);
        }
    }
    resolved
}

fn diagnostic_list_candidates(config: &Path, args: &[String]) -> Vec<(String, PathBuf, bool)> {
    let Some(root) = config
        .canonicalize()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf))
    else {
        return Vec::new();
    };
    // A site's diagnosis must reflect rules used by the running nfqws2
    // process. Files merely present in `lists/` may belong to another tool
    // and must not create a false profile match.
    let standard_paths = LIST_IDS
        .iter()
        .filter_map(|id| list_file_path(config, id).map(|path| (path, (*id).to_string())))
        .collect::<HashMap<_, _>>();
    let mut candidates = Vec::new();
    for (path, is_ipset) in resolve_runtime_list_references(config, args) {
        if candidates.iter().any(|(_, existing, _)| existing == &path) {
            continue;
        }
        let label = standard_paths.get(&path).cloned().unwrap_or_else(|| {
            path.strip_prefix(&root)
                .map(|relative| relative.to_string_lossy().replace('\\', "/"))
                .unwrap_or_else(|_| "активный список".into())
        });
        candidates.push((label, path, is_ipset));
    }
    candidates
}

async fn read_matching_runtime_list_entries(
    candidates: &[(String, PathBuf, bool)],
    host: &str,
    addresses: &[IpAddr],
) -> (Vec<String>, Vec<String>) {
    let mut matches = Vec::new();
    let mut unreadable = Vec::new();
    for (id, path, is_ipset) in candidates {
        let mut bytes = Vec::new();
        let read = match tokio::fs::File::open(path).await {
            Ok(file) => file
                .take((MAX_LIST_BYTES + 1) as u64)
                .read_to_end(&mut bytes)
                .await
                .is_ok(),
            Err(_) => false,
        };
        if !read || bytes.len() > MAX_LIST_BYTES {
            unreadable.push(id.clone());
            continue;
        }
        for line in String::from_utf8_lossy(&bytes).lines() {
            if site_diagnostics::list_entry_matches(host, addresses, line, *is_ipset) {
                matches.push(format!(
                    "{id}: {}",
                    line.chars().take(253).collect::<String>()
                ));
                break;
            }
        }
    }
    (matches, unreadable)
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

fn list_reference_matches_path(config: &Path, list_path: &Path, reference: &Path) -> bool {
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
    let target = list_path
        .canonicalize()
        .unwrap_or_else(|_| list_path.to_path_buf());
    candidate.canonicalize().is_ok_and(|path| path == target)
}

fn list_profile_usage(
    config: &Path,
    list_path: &Path,
    profiles: &[nfqws2_ui::runtime_profiles::RuntimeProfile],
) -> Vec<ListProfileUsage> {
    const FLAGS: [(&str, &str); 5] = [
        ("--hostlist", "include"),
        ("--hostlist-auto", "auto"),
        ("--hostlist-exclude", "exclude"),
        ("--ipset", "include"),
        ("--ipset-exclude", "exclude"),
    ];
    let mut usages = Vec::new();
    for profile in profiles {
        for (index, argument) in profile.args.iter().take(16_384).enumerate() {
            for (flag, role) in FLAGS {
                let reference = argument
                    .strip_prefix(flag)
                    .and_then(|suffix| suffix.strip_prefix('='))
                    .or_else(|| {
                        (argument == flag)
                            .then(|| profile.args.get(index + 1).map(String::as_str))
                            .flatten()
                    });
                let Some(reference) = reference.map(|value| value.trim_start_matches('@')) else {
                    continue;
                };
                if reference.is_empty()
                    || reference.len() > 4096
                    || reference.contains(['$', '`', '\0'])
                    || reference.starts_with('~')
                {
                    continue;
                }
                if list_reference_matches_path(config, list_path, Path::new(reference)) {
                    let usage = ListProfileUsage {
                        profile_index: profile.index,
                        profile_name: profile.name.clone(),
                        role,
                    };
                    if !usages.contains(&usage) {
                        usages.push(usage);
                    }
                }
            }
        }
    }
    usages.sort_by_key(|usage| (usage.profile_index, usage.role));
    usages
}

fn list_requires_nfqws_validation(
    config: &Path,
    list_path: &Path,
    id: &str,
    config_source: &str,
    active_args: Option<&[String]>,
) -> bool {
    const HOSTLIST_FLAGS: [&str; 3] = ["--hostlist", "--hostlist-auto", "--hostlist-exclude"];
    const IPSET_FLAGS: [&str; 2] = ["--ipset", "--ipset-exclude"];
    const ALL_FLAGS: [&str; 5] = [
        "--hostlist",
        "--hostlist-auto",
        "--hostlist-exclude",
        "--ipset",
        "--ipset-exclude",
    ];
    let known_list = LIST_IDS.contains(&id);
    let is_ipset = id.starts_with("ipset");
    let flags: &[&str] = if !known_list {
        &ALL_FLAGS
    } else if is_ipset {
        &IPSET_FLAGS
    } else {
        &HOSTLIST_FLAGS
    };
    for flag in flags {
        let (references, ambiguous) = configured_list_references_for_flag(config_source, flag);
        if ambiguous
            || references
                .iter()
                .any(|reference| list_reference_matches_path(config, list_path, reference))
        {
            return true;
        }
    }
    if let Some(args) = active_args {
        if flags
            .iter()
            .any(|flag| argv_has_ambiguous_list_reference(args, flag))
        {
            return true;
        }
        return runtime_list_references(args)
            .iter()
            .any(|(reference, _)| list_reference_matches_path(config, list_path, reference));
    }
    false
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
    let host = match site_diagnostics::normalize_site_target(&request.host) {
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
    append_diagnostic_list_check(&state, &mut report).await;
    Json(report).into_response()
}

async fn append_diagnostic_list_check(state: &AppState, report: &mut DiagnosticReport) {
    if let Some(config) = config_path_for_state(state) {
        let lists_started = Instant::now();
        let active_args = active_process_arguments().unwrap_or_default();
        let candidates = diagnostic_list_candidates(&config, &active_args);
        let no_active_lists = candidates.is_empty();
        let (matches, unreadable) =
            read_matching_runtime_list_entries(&candidates, &report.host, &report.addresses).await;
        let mut lists_step = DiagnosticStep {
            id: "lists".into(),
            status: if unreadable.is_empty() {
                "ok"
            } else {
                "warning"
            },
            detail: format!(
                "{}{}",
                if no_active_lists {
                    "В nfqws2 нет подключённых списков для сверки".into()
                } else if matches.is_empty() {
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
        };
        lists_step.record_elapsed(lists_started);
        report.steps.push(lists_step);
    } else {
        report.steps.push(DiagnosticStep {
            id: "lists".into(),
            status: "unavailable",
            detail: "Конфигурация nfqws2 не обнаружена; списки не проверены".into(),
            duration_ms: None,
            http_code: None,
        });
    }
}

async fn diagnose_site_stream(
    State(state): State<Arc<AppState>>,
    Json(request): Json<SiteDiagnosticRequest>,
) -> Response {
    let host = match site_diagnostics::normalize_site_target(&request.host) {
        Ok(host) => host,
        Err(message) => return error(StatusCode::BAD_REQUEST, message),
    };
    // A browser-side AbortController closes the previous SSE stream, but the
    // detached diagnostic deliberately remains alive until temporary firewall
    // rules have been removed. Queue the replacement instead of returning 409.
    let guard = state.site_diagnostic_lock.clone().lock_owned().await;
    let binary = discover_system_paths()
        .map(|paths| paths.binary.to_string_lossy().into_owned())
        .unwrap_or_default();
    let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<Event>();
    tokio::spawn(async move {
        let _guard = guard;
        let progress_tx = tx.clone();
        let mut report = site_diagnostics::diagnose_with_progress(
            host,
            binary,
            move |progress: DiagnosticProgress| {
                let event = serde_json::json!({"type":"progress", "progress":progress});
                let _ = progress_tx.send(Event::default().data(event.to_string()));
            },
        )
        .await;
        append_diagnostic_list_check(&state, &mut report).await;
        let event = serde_json::json!({"type":"complete", "report":report});
        let _ = tx.send(Event::default().data(event.to_string()));
    });
    let events = futures_util::stream::unfold(rx, |mut receiver| async move {
        receiver
            .recv()
            .await
            .map(|event| (Ok::<Event, std::convert::Infallible>(event), receiver))
    });
    Sse::new(events)
        .keep_alive(KeepAlive::default())
        .into_response()
}

async fn resolve_monitoring_policy_route(state: &AppState) -> MonitoringPolicyRoute {
    let Some(path) = config_path_for_state(state) else {
        return MonitoringPolicyRoute::Standard;
    };
    let config = match tokio::fs::read_to_string(&path).await {
        Ok(config) => config,
        Err(error) => {
            return MonitoringPolicyRoute::Unavailable(format!(
                "Не удалось прочитать конфигурацию для определения POLICY_NAME: {error}"
            ));
        }
    };
    let document = match ConfigDocument::parse(&config) {
        Ok(document) => document,
        Err(error) => {
            let policy_assignment_present = config.lines().any(|line| {
                let line = line.trim_start();
                !line.starts_with('#') && line.starts_with("POLICY_NAME=")
            });
            return if policy_assignment_present {
                MonitoringPolicyRoute::Unavailable(format!(
                    "Не удалось безопасно разобрать POLICY_NAME: {error}"
                ))
            } else {
                MonitoringPolicyRoute::Standard
            };
        }
    };
    if !document.contains_assignment("POLICY_NAME") {
        return MonitoringPolicyRoute::Standard;
    }
    let policy_name = match document.literal_value("POLICY_NAME") {
        Ok(value) => value,
        Err(error) => return MonitoringPolicyRoute::Unavailable(error),
    };
    if policy_name.trim().is_empty() {
        return MonitoringPolicyRoute::Standard;
    }
    match site_trace::read_policy_route_mark().await {
        Ok(Some(mark)) => MonitoringPolicyRoute::Marked(mark),
        // Match Zemidala: POLICY_NAME alone does not prove that the active
        // nfqws2 rules are in policy-only mode. Service marks such as
        // 0x40000000/0x40000000 are intentionally ignored by policy_route_mark.
        Ok(None) => MonitoringPolicyRoute::Standard,
        Err(error) => MonitoringPolicyRoute::Unavailable(format!(
            "Не удалось прочитать активную метку политики: {error}"
        )),
    }
}

async fn run_monitor_probe(
    host: &str,
    ip: std::net::Ipv4Addr,
    route: &MonitoringPolicyRoute,
) -> site_diagnostics::MonitorProbeResult {
    match route {
        MonitoringPolicyRoute::Standard => site_diagnostics::monitor_probe(host, ip).await,
        MonitoringPolicyRoute::Marked(mark) => {
            site_diagnostics::monitor_probe_with_policy(host, ip, mark).await
        }
        MonitoringPolicyRoute::Unavailable(detail) => {
            site_diagnostics::unavailable_monitor_result(detail.clone())
        }
    }
}

async fn check_monitoring_host(
    host: String,
    policy_route: MonitoringPolicyRoute,
) -> MonitoringHostResult {
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
    let mut probe = run_monitor_probe(&host, ip, &policy_route).await;
    if site_diagnostics::monitor_probe_needs_fallback(probe.current.status)
        && let Some(fallback_ip) = addresses.get(1).copied()
    {
        let fallback = run_monitor_probe(&host, fallback_ip, &policy_route).await;
        if !site_diagnostics::monitor_probe_needs_fallback(fallback.current.status) {
            probe = fallback;
        }
    }
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

fn is_podkop_fake_ipv4(ip: std::net::Ipv4Addr) -> bool {
    u32::from(ip) & 0xfffe_0000 == 0xc612_0000
}

fn output_contains_podkop_fake_ip(output: &[u8]) -> bool {
    String::from_utf8_lossy(output)
        .split(|character: char| !character.is_ascii_digit() && character != '.')
        .filter_map(|candidate| candidate.parse::<std::net::Ipv4Addr>().ok())
        .any(is_podkop_fake_ipv4)
}

async fn detect_podkop_hosts(hosts: &[String]) -> Vec<String> {
    if !Path::new("/etc/init.d/podkop").is_file() {
        return Vec::new();
    }
    let mut tasks = tokio::task::JoinSet::new();
    let mut detected = HashSet::new();
    for host in hosts {
        if let Ok(ip) = host.parse::<std::net::Ipv4Addr>() {
            if is_podkop_fake_ipv4(ip) {
                detected.insert(host.clone());
            }
            continue;
        }
        let host = host.clone();
        tasks.spawn(async move {
            let lookup = async {
                let output = match tokio::process::Command::new("nslookup")
                    .args([host.as_str(), "127.0.0.42"])
                    .stdout(Stdio::piped())
                    .stderr(Stdio::null())
                    .kill_on_drop(true)
                    .output()
                    .await
                {
                    Ok(output) => output,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                        tokio::process::Command::new("busybox")
                            .args(["nslookup", host.as_str(), "127.0.0.42"])
                            .stdout(Stdio::piped())
                            .stderr(Stdio::null())
                            .kill_on_drop(true)
                            .output()
                            .await
                            .ok()?
                    }
                    Err(_) => return None,
                };
                Some(output)
            };
            let is_fake = tokio::time::timeout(std::time::Duration::from_secs(2), lookup)
                .await
                .ok()
                .flatten()
                .filter(|output| output.status.success())
                .is_some_and(|output| output_contains_podkop_fake_ip(&output.stdout));
            (host, is_fake)
        });
    }
    while let Some(result) = tasks.join_next().await {
        if let Ok((host, true)) = result {
            detected.insert(host);
        }
    }
    hosts
        .iter()
        .filter(|host| detected.contains(*host))
        .cloned()
        .collect()
}

async fn monitor_sites(
    State(state): State<Arc<AppState>>,
    Json(request): Json<MonitoringRequest>,
) -> Response {
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
    let policy_route = resolve_monitoring_policy_route(&state).await;
    match run_bounded_monitor_checks(hosts, move |host| {
        check_monitoring_host(host, policy_route.clone())
    })
    .await
    {
        Ok(results) => Json(results).into_response(),
        Err(join_error) => error(StatusCode::INTERNAL_SERVER_ERROR, join_error.to_string()),
    }
}

const MAX_CONCURRENT_MONITOR_HOSTS: usize = 3;

async fn run_bounded_monitor_checks<F, Fut>(
    hosts: Vec<String>,
    check: F,
) -> Result<Vec<MonitoringHostResult>, tokio::task::JoinError>
where
    F: Fn(String) -> Fut,
    Fut: std::future::Future<Output = MonitoringHostResult> + Send + 'static,
{
    let mut hosts = hosts.into_iter();
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..MAX_CONCURRENT_MONITOR_HOSTS {
        let Some(host) = hosts.next() else { break };
        tasks.spawn(check(host));
    }
    let mut results = Vec::new();
    while let Some(result) = tasks.join_next().await {
        results.push(result?);
        if let Some(host) = hosts.next() {
            tasks.spawn(check(host));
        }
    }
    Ok(results)
}

fn monitoring_store_path(data_dir: &Path) -> PathBuf {
    data_dir.join("monitoring.json")
}

async fn load_monitoring_store(data_dir: &Path) -> Result<MonitoringStore, String> {
    let path = monitoring_store_path(data_dir);
    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            let mut store: MonitoringStore = serde_json::from_slice(&bytes)
                .map_err(|e| format!("Некорректный файл мониторинга: {e}"))?;
            store.interval_minutes = store.interval_minutes.clamp(10, 1440);
            Ok(store)
        }
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

fn auto_strategy_store_path(data_dir: &Path) -> PathBuf {
    data_dir.join("monitoring-auto-strategy.json")
}

async fn load_auto_strategy_store(data_dir: &Path) -> Result<AutoStrategyStore, String> {
    let path = auto_strategy_store_path(data_dir);
    match tokio::fs::read(&path).await {
        Ok(bytes) if bytes.len() <= 128 * 1024 => {
            let mut store: AutoStrategyStore = serde_json::from_slice(&bytes)
                .map_err(|_| "Повреждены настройки автоподбора".to_string())?;
            store.settings = monitor_auto::normalize_settings(store.settings);
            Ok(store)
        }
        Ok(_) => Err("Состояние автоподбора превышает лимит 128 КиБ".into()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(AutoStrategyStore::default())
        }
        Err(error) => Err(format!(
            "Не удалось прочитать настройки автоподбора: {error}"
        )),
    }
}

async fn save_auto_strategy_store(
    data_dir: &Path,
    store: &AutoStrategyStore,
) -> Result<(), String> {
    tokio::fs::create_dir_all(data_dir)
        .await
        .map_err(|error| error.to_string())?;
    let path = auto_strategy_store_path(data_dir);
    let temporary = path.with_extension("json.tmp");
    let mut bounded = store.clone();
    bounded.log.truncate(80);
    bounded.offers.truncate(20);
    let bytes = serde_json::to_vec(&bounded).map_err(|error| error.to_string())?;
    if bytes.len() > 128 * 1024 {
        return Err("Состояние автоподбора превышает лимит 128 КиБ".into());
    }
    tokio::fs::write(&temporary, bytes)
        .await
        .map_err(|error| error.to_string())?;
    #[cfg(unix)]
    tokio::fs::set_permissions(&temporary, std::fs::Permissions::from_mode(0o600))
        .await
        .map_err(|error| error.to_string())?;
    tokio::fs::rename(&temporary, &path)
        .await
        .map_err(|error| error.to_string())
}

async fn get_auto_strategy(State(state): State<Arc<AppState>>) -> Response {
    match load_auto_strategy_store(&state.data_dir).await {
        Ok(store) => Json(store).into_response(),
        Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

async fn put_auto_strategy(
    State(state): State<Arc<AppState>>,
    Json(update): Json<AutoStrategySettingsUpdate>,
) -> Response {
    let _guard = state.auto_strategy_lock.lock().await;
    let mut store = match load_auto_strategy_store(&state.data_dir).await {
        Ok(store) => store,
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    };
    store.settings = monitor_auto::normalize_settings(AutoStrategySettings {
        enabled: update.enabled,
        apply_automatically: update.apply_automatically,
        failures_before_pick: update.failures_before_pick,
        cooldown_hours: update.cooldown_hours,
    });
    match save_auto_strategy_store(&state.data_dir, &store).await {
        Ok(()) => Json(store).into_response(),
        Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

async fn put_monitoring_store(
    State(state): State<Arc<AppState>>,
    Json(update): Json<MonitoringStoreUpdate>,
) -> Response {
    let _monitoring_guard = state.monitoring_lock.lock().await;
    if !monitoring_interval_valid(update.interval_minutes) {
        return error(
            StatusCode::BAD_REQUEST,
            "Интервал должен быть 30, 60, 120 минут или от 10 до 1440 минут",
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

fn monitoring_interval_valid(minutes: u64) -> bool {
    [30, 60, 120].contains(&minutes) || (10..=1440).contains(&minutes)
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

fn record_monitoring_completion(
    store: &mut MonitoringStore,
    is_full_cycle: bool,
    completed_at: u64,
) {
    if is_full_cycle {
        store.last_auto_check = Some(completed_at);
    }
}

fn queue_monitoring_changes(store: &mut MonitoringStore, messages: Vec<String>, timestamp: u64) {
    if messages.is_empty() || store.telegram.chat_id.is_empty() {
        return;
    }
    queue_telegram_notification(
        store,
        format!("nfqws2 на роутере:\n{}", messages.join("\n")),
        timestamp,
    );
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
    let store = load_monitoring_store(data_dir).await?;
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
    let policy_route = resolve_monitoring_policy_route(state).await;
    let reports = run_bounded_monitor_checks(sites, move |host| {
        check_monitoring_host(host, policy_route.clone())
    })
    .await
    .map_err(|error| error.to_string())?;
    let mut updated = load_monitoring_store(data_dir).await?;
    record_monitoring_completion(&mut updated, only_hosts.is_none(), now_unix_seconds());
    let mut notification_messages = Vec::new();
    for report in &reports {
        if let Some(site) = updated
            .sites
            .iter_mut()
            .find(|site| site.host == report.host)
        {
            let status = report.current.status.to_owned();
            let previous = site.results.first().map(|result| result.status.clone());
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
    queue_monitoring_changes(&mut updated, notification_messages, now);
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
            if let Err(message) = auto_strategy_tick(&state).await {
                tracing::warn!("monitoring auto strategy: {message}");
            }
        }
    });
}

fn auto_strategy_event(
    store: &mut AutoStrategyStore,
    host: &str,
    kind: &str,
    message: impl Into<String>,
) {
    store.log.insert(
        0,
        AutoStrategyEvent {
            host: host.to_owned(),
            kind: kind.to_owned(),
            message: message.into(),
            timestamp: now_unix_seconds(),
        },
    );
    store.log.truncate(80);
}

async fn auto_strategy_tick(state: &Arc<AppState>) -> Result<(), String> {
    let _guard = state.auto_strategy_lock.lock().await;
    let mut store = load_auto_strategy_store(&state.data_dir).await?;
    if !store.settings.enabled {
        if let Some(id) = store.active_operation_id.take() {
            if let Some(control) = state.strategy_pick_controls.lock().await.get(&id) {
                control.store(true, Ordering::Release);
            }
            let host = store.active_host.take().unwrap_or_default();
            auto_strategy_event(
                &mut store,
                &host,
                "cancelled",
                "Автоподбор отключён; незавершённая проверка остановлена, конфигурация не менялась.",
            );
            save_auto_strategy_store(&state.data_dir, &store).await?;
        }
        return Ok(());
    }
    let now = now_unix_seconds();
    if let Some(operation_id) = store.active_operation_id.clone() {
        let Some(operation) = read_strategy_pick_operation(
            &state.data_dir.join("strategy-pick-operations"),
            &operation_id,
        )
        .await?
        else {
            store.active_operation_id = None;
            store.active_host = None;
            auto_strategy_event(
                &mut store,
                "",
                "error",
                "Результат автоподбора потерян; операция снята с ожидания.",
            );
            return save_auto_strategy_store(&state.data_dir, &store).await;
        };
        if matches!(operation.state.as_str(), "queued" | "running" | "stopping") {
            return Ok(());
        }
        let host = store
            .active_host
            .clone()
            .unwrap_or_else(|| operation.host.clone());
        let monitor_store = load_monitoring_store(&state.data_dir).await?;
        let target_still_monitored =
            monitor_store.enabled && monitor_store.sites.iter().any(|site| site.host == host);
        if !target_still_monitored {
            auto_strategy_event(
                &mut store,
                &host,
                "cancelled",
                "Сайт удалён из мониторинга или мониторинг выключен; найденный профиль не применяется.",
            );
            store.active_operation_id = None;
            store.active_host = None;
            return save_auto_strategy_store(&state.data_dir, &store).await;
        }
        let healthy_again = monitor_store
            .sites
            .iter()
            .find(|site| site.host == host)
            .and_then(|site| site.results.first())
            .is_some_and(|result| result.status == "ok");
        if healthy_again || operation.state == "site_unblocked" {
            auto_strategy_event(
                &mut store,
                &host,
                "recovered",
                "Сайт снова отвечает; результат не применяется.",
            );
        } else if let Some((candidate_index, result)) =
            operation.results.iter().enumerate().find(|(_, result)| {
                result.confirmed && result.attempts >= 2 && result.successes >= 2
            })
        {
            let offer = AutoStrategyOffer {
                host: host.clone(),
                operation_id: operation.id.clone(),
                candidate_index,
                strategy_name: result.candidate.name.clone(),
                steps: result.candidate.steps.clone(),
                created_at: now,
            };
            store.offers.insert(0, offer);
            store.offers.truncate(20);
            if store.settings.apply_automatically {
                match apply_auto_strategy_candidate(state, &operation, candidate_index).await {
                    Ok(updated_existing_profile) => auto_strategy_event(
                        &mut store,
                        &host,
                        "applied",
                        if updated_existing_profile {
                            format!(
                                "Подтверждённая стратегия «{}» установлена в существующий профиль домена; перезапуск и контрольные сайты прошли проверку.",
                                result.candidate.name
                            )
                        } else {
                            format!(
                                "Подтверждённая стратегия «{}» добавлена отдельным профилем домена; перезапуск и контрольные сайты прошли проверку.",
                                result.candidate.name
                            )
                        },
                    ),
                    Err(message) => auto_strategy_event(
                        &mut store,
                        &host,
                        "rollback",
                        format!("Применение не завершено: {message}"),
                    ),
                }
            } else {
                auto_strategy_event(
                    &mut store,
                    &host,
                    "offer",
                    format!(
                        "Найдена подтверждённая стратегия «{}». Основной конфиг не менялся; результат доступен во вкладке автоподбора.",
                        result.candidate.name
                    ),
                );
            }
        } else {
            auto_strategy_event(
                &mut store,
                &host,
                "no-match",
                "Подбор завершён, но подтверждённой стратегии не найдено. Конфигурация не менялась.",
            );
        }
        store.active_operation_id = None;
        store.active_host = None;
        return save_auto_strategy_store(&state.data_dir, &store).await;
    }

    let monitoring = load_monitoring_store(&state.data_dir).await?;
    if !monitoring.enabled || monitoring.sites.is_empty() {
        return Ok(());
    }
    let monitored_hosts = monitoring
        .sites
        .iter()
        .map(|site| site.host.as_str())
        .collect::<HashSet<_>>();
    store
        .last_attempt_at
        .retain(|host, _| monitored_hosts.contains(host.as_str()));
    store
        .retry_after
        .retain(|host, _| monitored_hosts.contains(host.as_str()));
    let mut picked = None;
    for site in &monitoring.sites {
        if store
            .retry_after
            .get(&site.host)
            .is_some_and(|retry| now < *retry)
        {
            continue;
        }
        let statuses = site
            .results
            .iter()
            .map(|result| result.status.as_str())
            .collect::<Vec<_>>();
        let readiness = monitor_auto::readiness(
            &store.settings,
            &statuses,
            store.last_attempt_at.get(&site.host).copied(),
            now,
        );
        if readiness != monitor_auto::AutoPickReadiness::Ready {
            continue;
        }
        let latest = site.results.first().expect("ready site has a result");
        if monitor_auto::is_infrastructure_failure(&latest.summary) {
            store
                .retry_after
                .insert(site.host.clone(), now.saturating_add(300));
            auto_strategy_event(
                &mut store,
                &site.host,
                "skipped",
                "Пропущено: похоже на проблему DNS или маршрута, а не на необходимость менять стратегию.",
            );
            continue;
        }
        picked = Some(site.host.clone());
        break;
    }
    let Some(host) = picked else {
        save_auto_strategy_store(&state.data_dir, &store).await?;
        return Ok(());
    };
    if active_process_snapshot().is_none() {
        store
            .retry_after
            .insert(host.clone(), now.saturating_add(300));
        auto_strategy_event(
            &mut store,
            &host,
            "skipped",
            "Подбор пропущен: работающий процесс nfqws2 не обнаружен однозначно.",
        );
        return save_auto_strategy_store(&state.data_dir, &store).await;
    }
    if detect_podkop_hosts(std::slice::from_ref(&host))
        .await
        .contains(&host)
    {
        store
            .retry_after
            .insert(host.clone(), now.saturating_add(300));
        auto_strategy_event(
            &mut store,
            &host,
            "skipped",
            "Пропущено: домен направляется через Podkop; автоподбор nfqws2 не будет менять его маршрут.",
        );
        return save_auto_strategy_store(&state.data_dir, &store).await;
    }
    let response = start_strategy_pick(
        State(state.clone()),
        Json(StrategyPickRequest {
            host: host.clone(),
            protocol: Some("https".into()),
            mode: Some("quick".into()),
            repeats: Some(3),
            stop_after_first_confirmed: false,
            include_other_history: true,
            refine_lighter: false,
            include_community: true,
            custom_steps: None,
        }),
    )
    .await;
    if !response.status().is_success() {
        let body = axum::body::to_bytes(response.into_body(), 16 * 1024)
            .await
            .unwrap_or_default();
        let reason = serde_json::from_slice::<serde_json::Value>(&body)
            .ok()
            .and_then(|value| {
                value
                    .get("error")
                    .and_then(serde_json::Value::as_str)
                    .map(str::to_owned)
            })
            .unwrap_or_else(|| {
                "Сейчас нельзя запустить диагностику; повтор будет после паузы.".into()
            });
        store
            .retry_after
            .insert(host.clone(), now.saturating_add(300));
        auto_strategy_event(
            &mut store,
            &host,
            "deferred",
            format!("Автоподбор отложен: {reason}"),
        );
        return save_auto_strategy_store(&state.data_dir, &store).await;
    }
    let body = axum::body::to_bytes(response.into_body(), 32 * 1024)
        .await
        .map_err(|error| error.to_string())?;
    let value: serde_json::Value = serde_json::from_slice(&body)
        .map_err(|error| format!("Не удалось прочитать ответ автоподбора: {error}"))?;
    if let Some(id) = value.get("id").and_then(serde_json::Value::as_str) {
        store.last_attempt_at.insert(host.clone(), now);
        store.retry_after.remove(&host);
        store.active_operation_id = Some(id.to_owned());
        store.active_host = Some(host.clone());
        let failure_threshold = store.settings.failures_before_pick;
        auto_strategy_event(
            &mut store,
            &host,
            "started",
            format!(
                "После {failure_threshold} ошибок подряд запущен ограниченный подбор HTTPS-стратегии; рабочий конфиг не меняется."
            ),
        );
    } else {
        store
            .retry_after
            .insert(host.clone(), now.saturating_add(300));
        auto_strategy_event(
            &mut store,
            &host,
            "deferred",
            "Подбор не принят системой диагностики; конфигурация не менялась.",
        );
    }
    save_auto_strategy_store(&state.data_dir, &store).await
}

async fn apply_auto_strategy_candidate(
    state: &AppState,
    operation: &StrategyPickOperation,
    index: usize,
) -> Result<bool, String> {
    if operation.protocol != "https" {
        return Err("Автоматическое применение разрешено только для HTTPS-профиля".into());
    }
    let result = operation
        .results
        .get(index)
        .ok_or_else(|| "Не найден выбранный результат".to_string())?;
    if !result.confirmed || result.attempts < 2 || result.successes < 2 {
        return Err("Результат не прошёл повторное подтверждение".into());
    }
    let Some((pid, active_args)) = active_process_snapshot() else {
        return Err("Работающий nfqws2 изменился; профиль не применён".into());
    };
    let paths = discover_system_paths()
        .ok_or_else(|| "Не удалось определить установленные файлы nfqws2".to_string())?;
    let functions = active_lua_functions(&active_args, &paths.lua_dir);
    let blobs = active_blob_names(&active_args);
    let profile = strategy_picker::build_domain_profile(
        &operation.host,
        StrategyProtocol::Https,
        &result.candidate.steps,
        &functions,
        &blobs,
    )?;
    let config_path = config_path_for_state(state)
        .ok_or_else(|| "Разрешённый конфиг nfqws2 не найден".to_string())?;
    let original = tokio::fs::read(&config_path)
        .await
        .map_err(|error| error.to_string())?;
    let original_content = String::from_utf8(original.clone())
        .map_err(|_| "Конфигурация не является корректным UTF-8".to_string())?;
    let (proposed, updated_existing_profile) =
        build_auto_strategy_config(&original_content, &profile)?;
    if validate_nfqws2_config(&proposed)?
        .iter()
        .any(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    {
        return Err("Новая конфигурация не прошла статическую проверку".into());
    }
    validate_profile_config_dry_run(&proposed, &paths).await?;
    if active_process_snapshot().is_none_or(|(current_pid, _)| current_pid != pid) {
        return Err("Процесс nfqws2 изменился; профиль не применён".into());
    }
    let monitoring = load_monitoring_store(&state.data_dir).await?;
    let hosts = strategy_apply_verification_hosts(&operation.host, &monitoring);
    let route = resolve_monitoring_policy_route(state).await;
    apply_config_transaction_with_verification(
        state,
        ConfigWrite {
            content: proposed,
            revision: revision(&original),
        },
        || async { service_action(ServiceAction::Restart).await },
        || {
            let hosts = hosts.clone();
            let route = route.clone();
            async move {
                verify_strategy_hosts_with(hosts, move |host| {
                    let route = route.clone();
                    async move {
                        let report = check_monitoring_host(host, route).await;
                        if report.current.status == "ok" {
                            Ok(())
                        } else {
                            Err(report.summary)
                        }
                    }
                })
                .await
            }
        },
    )
    .await
    .map(|_| updated_existing_profile)
}

fn build_auto_strategy_config(
    original_content: &str,
    profile: &str,
) -> Result<(String, bool), String> {
    let document = ConfigDocument::parse(original_content)?;
    let custom_args = document.literal_value("NFQWS_ARGS_CUSTOM")?;
    let mut profiles = nfqws2_ui::runtime_profiles::split_custom_profile_args(&custom_args)?;
    profiles.retain(|existing| !existing.is_empty());
    let candidate_profiles = nfqws2_ui::runtime_profiles::split_custom_profile_args(profile)?;
    let candidate = candidate_profiles
        .get(1)
        .filter(|_| candidate_profiles.len() == 2)
        .ok_or_else(|| "Автоподбор сформировал некорректный профиль".to_string())?;
    let selector = candidate
        .iter()
        .find(|argument| {
            argument.starts_with("--hostlist-domains=") || argument.starts_with("--ipset-ip=")
        })
        .ok_or_else(|| "В профиле автоподбора отсутствует точный доменный селектор".to_string())?;
    let candidate_steps = candidate
        .iter()
        .filter(|argument| argument.starts_with("--lua-desync="))
        .cloned()
        .collect::<Vec<_>>();
    if candidate_steps.is_empty() {
        return Err("Подтверждённый профиль не содержит Lua-стратегии".into());
    }

    let exact_matches = profiles
        .iter()
        .enumerate()
        .filter(|(_, profile)| profile.iter().any(|argument| argument == selector))
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    let updated_existing_profile = match exact_matches.as_slice() {
        [index] => {
            let target = profiles
                .get_mut(*index)
                .ok_or_else(|| "Точный профиль домена изменился".to_string())?;
            target.retain(|argument| !argument.starts_with("--lua-desync="));
            target.extend(candidate_steps);
            true
        }
        [] => {
            profiles.insert(0, candidate.clone());
            false
        }
        _ => {
            return Err(
                "Найдено несколько точных профилей домена; автоматическое изменение отменено"
                    .into(),
            );
        }
    };
    let encoded = nfqws2_ui::runtime_profiles::encode_custom_profile_args(&profiles)?;
    Ok((
        document.set_literal_all("NFQWS_ARGS_CUSTOM", &encoded)?,
        updated_existing_profile,
    ))
}

fn foreign_config_operation_path(directory: &Path, id: &str) -> PathBuf {
    directory.join(format!("{id}.json"))
}

fn foreign_config_candidate_path(directory: &Path, id: &str, index: usize) -> PathBuf {
    directory.join(format!("{id}.candidate-{index}"))
}

fn write_foreign_config_operation_sync(
    directory: &Path,
    operation: &ForeignConfigCheckOperation,
) -> Result<(), String> {
    if !valid_site_trace_id(&operation.id) {
        return Err("Недопустимый идентификатор проверки конфигурации".into());
    }
    std::fs::create_dir_all(directory).map_err(|error| error.to_string())?;
    let bytes = serde_json::to_vec(operation).map_err(|error| error.to_string())?;
    if bytes.len() > MAX_FOREIGN_CONFIG_STATE_BYTES {
        return Err("Результаты проверки конфигурации превышают лимит".into());
    }
    let destination = foreign_config_operation_path(directory, &operation.id);
    let temporary = directory.join(format!("{}.{}.tmp", operation.id, new_update_id()));
    std::fs::write(&temporary, bytes).map_err(|error| error.to_string())?;
    #[cfg(unix)]
    {
        let permissions = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(&temporary, permissions).map_err(|error| error.to_string())?;
    }
    std::fs::rename(&temporary, &destination).map_err(|error| error.to_string())
}

async fn read_foreign_config_operation(
    directory: &Path,
    id: &str,
) -> Result<Option<ForeignConfigCheckOperation>, String> {
    if !valid_site_trace_id(id) {
        return Ok(None);
    }
    let mut bytes = Vec::new();
    let file = match tokio::fs::File::open(foreign_config_operation_path(directory, id)).await {
        Ok(file) => file,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(format!("Не удалось прочитать результат проверки: {error}")),
    };
    file.take((MAX_FOREIGN_CONFIG_STATE_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|error| format!("Не удалось прочитать результат проверки: {error}"))?;
    if bytes.len() > MAX_FOREIGN_CONFIG_STATE_BYTES {
        return Err("Сохранённый результат проверки превышает лимит".into());
    }
    let operation: ForeignConfigCheckOperation = serde_json::from_slice(&bytes)
        .map_err(|_| "Повреждён результат проверки конфигурации".to_string())?;
    let valid_state = matches!(
        operation.state.as_str(),
        "queued" | "running" | "stopping" | "complete" | "cancelled" | "failed"
    );
    if operation.id != id
        || !valid_state
        || operation.hosts.len() > MAX_FOREIGN_CONFIG_HOSTS
        || operation.comparisons.len() > MAX_FOREIGN_CONFIGS
        || operation
            .comparisons
            .iter()
            .any(|comparison| comparison.sites.len() != operation.hosts.len())
    {
        return Err("Недопустимое состояние проверки конфигурации".into());
    }
    Ok(Some(operation))
}

async fn prune_foreign_config_operations(directory: &Path) {
    let Ok(mut entries) = tokio::fs::read_dir(directory).await else {
        return;
    };
    let mut finished = Vec::new();
    while let Ok(Some(entry)) = entries.next_entry().await {
        let path = entry.path();
        if path.extension().and_then(|value| value.to_str()) != Some("json") {
            continue;
        }
        let Some(id) = path.file_stem().and_then(|value| value.to_str()) else {
            continue;
        };
        if let Ok(Some(operation)) = read_foreign_config_operation(directory, id).await
            && matches!(
                operation.state.as_str(),
                "complete" | "cancelled" | "failed"
            )
        {
            finished.push((operation.updated_at, path));
        }
    }
    finished.sort_by_key(|item| std::cmp::Reverse(item.0));
    for (_, path) in finished.into_iter().skip(MAX_FOREIGN_CONFIG_OPERATIONS) {
        if let Some(id) = path.file_stem().and_then(|value| value.to_str()) {
            for index in 0..MAX_FOREIGN_CONFIGS {
                let _ = tokio::fs::remove_file(foreign_config_candidate_path(directory, id, index))
                    .await;
            }
        }
        let _ = tokio::fs::remove_file(path).await;
    }
}

fn external_config_test_args(
    source: &str,
    isolated_auto_list: &Path,
    lua_directory: &Path,
    blobs_directory: &Path,
    lists_directory: &Path,
    auto_list_sources: &mut Vec<(PathBuf, Option<PathBuf>)>,
) -> Result<Vec<String>, String> {
    external_config_args(
        source,
        isolated_auto_list,
        lua_directory,
        blobs_directory,
        lists_directory,
        auto_list_sources,
        true,
    )
}

fn external_config_args(
    source: &str,
    isolated_auto_list: &Path,
    lua_directory: &Path,
    blobs_directory: &Path,
    lists_directory: &Path,
    auto_list_sources: &mut Vec<(PathBuf, Option<PathBuf>)>,
    require_http_filter: bool,
) -> Result<Vec<String>, String> {
    let diagnostics = validate_nfqws2_config(source)?;
    if let Some(diagnostic) = diagnostics
        .iter()
        .find(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
    {
        return Err(format!("{}: {}", diagnostic.code, diagnostic.message));
    }
    let document = ConfigDocument::parse(source)?;
    let mut auto_list_index = 0usize;
    let mut map_arguments = |name: &str, value: &str| -> Result<Vec<String>, String> {
        let mut mapped = Vec::new();
        for argument in value.split_whitespace() {
            if argument.starts_with("--hostlist-auto-debug=") {
                // Debug output is not needed for a connectivity comparison and
                // may otherwise write into a path supplied by the foreign file.
                continue;
            } else if argument == "--hostlist-auto-debug" {
                return Err(format!(
                    "{name}: --hostlist-auto-debug должен быть записан как --hostlist-auto-debug=путь"
                ));
            } else if argument == "--hostlist-auto" {
                return Err(format!(
                    "{name}: --hostlist-auto должен быть записан как --hostlist-auto=путь"
                ));
            } else if [
                "--hostlist",
                "--hostlist-exclude",
                "--ipset",
                "--ipset-exclude",
            ]
            .contains(&argument)
            {
                return Err(format!(
                    "{name}: {argument} должен быть записан как {argument}=путь"
                ));
            } else if let Some(path) = argument.strip_prefix("--hostlist-auto=") {
                let destination = if auto_list_index == 0 {
                    isolated_auto_list.to_path_buf()
                } else {
                    isolated_auto_list.with_file_name(format!(
                        "{}-{auto_list_index}.list",
                        isolated_auto_list
                            .file_stem()
                            .and_then(|stem| stem.to_str())
                            .unwrap_or("auto")
                    ))
                };
                let source = resolve_foreign_list_path(path, lists_directory).ok();
                auto_list_sources.push((destination.clone(), source));
                auto_list_index += 1;
                mapped.push(format!("--hostlist-auto={}", destination.display()));
            } else if let Some(argument) = rewrite_foreign_list_argument(argument, lists_directory)
                .map_err(|message| format!("{name}: {message}"))?
            {
                mapped.push(argument);
            } else if let Some(path) = argument.strip_prefix("--lua-init=") {
                let Some(path) = path.strip_prefix('@') else {
                    return Err(format!(
                        "{name}: --lua-init должен ссылаться на локальный файл через @путь"
                    ));
                };
                let path = resolve_foreign_lua_path(path, lua_directory)
                    .map_err(|message| format!("{name}: {message}"))?;
                mapped.push(format!("--lua-init=@{}", path.display()));
            } else if let Some(argument) = rewrite_foreign_blob_argument(argument, blobs_directory)
                .map_err(|message| format!("{name}: {message}"))?
            {
                mapped.push(argument);
            } else {
                mapped.push(argument.to_owned());
            }
        }
        Ok(mapped)
    };
    let read_group = |name: &str| -> Result<Vec<String>, String> {
        if !document.contains_assignment(name) {
            return Ok(Vec::new());
        }
        let value = document
            .literal_or_mode_value(name)
            .map_err(|message| format!("{name}: {message}"))?;
        Ok(value.split_whitespace().map(str::to_owned).collect())
    };

    // Match Zemidala's expectedProfiles()/cfgPrepare() order. BASE is global;
    // all other argument variables become explicit nfqws2 profiles separated by --new.
    let mut base_args =
        map_arguments("NFQWS_BASE_ARGS", &read_group("NFQWS_BASE_ARGS")?.join(" "))?;
    base_args.retain(|argument| {
        ![
            "--qnum",
            "--daemon",
            "--pidfile",
            "--user",
            "--uid",
            "--debug",
        ]
        .iter()
        .any(|name| argument == name || argument.starts_with(&format!("{name}=")))
    });
    let mut profiles: Vec<Vec<String>> = Vec::new();
    let custom = map_arguments(
        "NFQWS_ARGS_CUSTOM",
        &read_group("NFQWS_ARGS_CUSTOM")?.join(" "),
    )?;
    if !custom.is_empty() {
        let mut custom_profile = Vec::new();
        for argument in custom {
            if argument == "--new" || argument.starts_with("--new=") {
                profiles.push(std::mem::take(&mut custom_profile));
                if let Some(name) = argument
                    .strip_prefix("--new=")
                    .filter(|name| !name.is_empty())
                {
                    custom_profile.push(format!("--name={name}"));
                }
            } else {
                custom_profile.push(argument);
            }
        }
        profiles.push(custom_profile);
    }

    let udp = map_arguments("NFQWS_ARGS_UDP", &read_group("NFQWS_ARGS_UDP")?.join(" "))?;
    if !udp.is_empty() {
        profiles.push(udp);
    }
    let quic = map_arguments("NFQWS_ARGS_QUIC", &read_group("NFQWS_ARGS_QUIC")?.join(" "))?;
    let ipset = map_arguments(
        "NFQWS_ARGS_IPSET",
        &read_group("NFQWS_ARGS_IPSET")?.join(" "),
    )?;
    let extra = map_arguments(
        "NFQWS_EXTRA_ARGS",
        &read_group("NFQWS_EXTRA_ARGS")?.join(" "),
    )?;
    if !quic.is_empty() {
        if !ipset.is_empty() {
            let mut profile = quic.clone();
            profile.extend(ipset.iter().cloned());
            profile.push("--ipset-ip=0.0.0.0".into());
            profiles.push(profile);
        }
        let mut profile = quic;
        profile.extend(extra.iter().cloned());
        profiles.push(profile);
    }
    let tcp = map_arguments("NFQWS_ARGS", &read_group("NFQWS_ARGS")?.join(" "))?;
    if require_http_filter && !document.contains_assignment("NFQWS_ARGS") {
        return Err("В конфигурации отсутствует NFQWS_ARGS".into());
    }
    if require_http_filter
        && !tcp
            .iter()
            .any(|argument| argument.starts_with("--filter-tcp="))
    {
        return Err("В NFQWS_ARGS не найден --filter-tcp для проверки HTTPS/HTTP".into());
    }
    if !ipset.is_empty() {
        let mut profile = tcp.clone();
        profile.extend(ipset);
        profile.push("--ipset-ip=0.0.0.0".into());
        profiles.push(profile);
    }
    let mut profile = tcp;
    profile.extend(extra);
    profiles.push(profile);

    let has_module = |argument: &str, module: &str| {
        argument
            .strip_prefix("--lua-init=@")
            .and_then(|path| Path::new(path).file_name())
            .and_then(|name| name.to_str())
            .is_some_and(|name| name.strip_suffix(".gz").unwrap_or(name) == module)
    };
    let has_zapret_lib = base_args
        .iter()
        .any(|argument| has_module(argument, "zapret-lib.lua"));
    if profiles
        .iter()
        .flatten()
        .any(|argument| argument.starts_with("--lua-desync="))
        && !has_zapret_lib
    {
        for module in ["zapret-lib.lua", "zapret-antidpi.lua", "zapret-auto.lua"] {
            if let Ok(path) =
                resolve_foreign_lua_path(&format!("/opt/etc/nfqws2/lua/{module}"), lua_directory)
                && !base_args
                    .iter()
                    .any(|argument| has_module(argument, module))
            {
                base_args.push(format!("--lua-init=@{}", path.display()));
            }
        }
    }

    let mut arguments = base_args;
    for (index, profile) in profiles.into_iter().enumerate() {
        if index > 0 {
            arguments.push("--new".into());
        }
        arguments.extend(profile);
    }
    Ok(arguments)
}

fn resolve_foreign_list_path(value: &str, lists_directory: &Path) -> Result<PathBuf, String> {
    const SOURCE_PREFIXES: [&str; 6] = [
        "/opt/etc/nfqws2/lists/",
        "/etc/nfqws2/lists/",
        "/opt/etc/nfqws/lists/",
        "/etc/nfqws/lists/",
        "/opt/zapret2/ipset/",
        "/opt/zapret/ipset/",
    ];
    if value.is_empty() || value.contains(['\\', '\0']) {
        return Err("Путь к списку пустой или содержит недопустимые символы".into());
    }
    let value = value.trim_start_matches('@');
    let relative = if value.starts_with('/') {
        if let Ok(relative) = Path::new(value).strip_prefix(lists_directory) {
            relative.to_path_buf()
        } else if let Some(relative) = SOURCE_PREFIXES
            .iter()
            .find_map(|prefix| value.strip_prefix(prefix))
        {
            PathBuf::from(relative)
        } else {
            return Err(format!(
                "путь списка «{value}» не относится к поддерживаемой раскладке nfqws2/zapret"
            ));
        }
    } else {
        Path::new(value)
            .strip_prefix("lists")
            .unwrap_or_else(|_| Path::new(value))
            .to_path_buf()
    };
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(format!("путь списка «{value}» выходит за каталог списков"));
    }
    let root = lists_directory
        .canonicalize()
        .map_err(|error| format!("каталог списков не найден: {error}"))?;
    let candidate = root.join(&relative);
    let mut cursor = root.clone();
    for component in relative.components() {
        cursor.push(component.as_os_str());
        let metadata = std::fs::symlink_metadata(&cursor)
            .map_err(|_| format!("файл списка «{value}» отсутствует в установленном каталоге"))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "путь списка «{value}» содержит символическую ссылку"
            ));
        }
    }
    let metadata = std::fs::metadata(&candidate)
        .map_err(|_| format!("файл списка «{value}» отсутствует в установленном каталоге"))?;
    let canonical = candidate
        .canonicalize()
        .map_err(|_| format!("не удалось проверить путь списка «{value}»"))?;
    if !metadata.is_file() || !canonical.starts_with(&root) {
        return Err(format!("путь списка «{value}» не является обычным файлом"));
    }
    Ok(canonical)
}

fn rewrite_foreign_list_argument(
    argument: &str,
    lists_directory: &Path,
) -> Result<Option<String>, String> {
    for flag in [
        "--hostlist",
        "--hostlist-exclude",
        "--ipset",
        "--ipset-exclude",
    ] {
        let Some(path) = argument.strip_prefix(&format!("{flag}=")) else {
            continue;
        };
        return resolve_foreign_list_path(path, lists_directory)
            .map(|path| Some(format!("{flag}={}", path.display())));
    }
    Ok(None)
}

fn resolve_foreign_blob_path(value: &str, blobs_directory: &Path) -> Result<PathBuf, String> {
    const SOURCE_PREFIXES: [&str; 6] = [
        "/opt/etc/nfqws2/blobs/",
        "/etc/nfqws2/blobs/",
        "/opt/etc/nfqws/blobs/",
        "/etc/nfqws/blobs/",
        "/opt/zapret2/files/fake/",
        "/opt/zapret/files/fake/",
    ];
    if value.is_empty() || value.contains(['\\', '\0']) {
        return Err("Путь к блобу пустой или содержит недопустимые символы".into());
    }
    let value = value.trim_start_matches('@');
    let relative = if value.starts_with('/') {
        if let Ok(relative) = Path::new(value).strip_prefix(blobs_directory) {
            relative.to_path_buf()
        } else if let Some(relative) = SOURCE_PREFIXES
            .iter()
            .find_map(|prefix| value.strip_prefix(prefix))
        {
            PathBuf::from(relative)
        } else {
            return Err(format!(
                "путь блоба «{value}» не относится к поддерживаемой раскладке nfqws2/zapret"
            ));
        }
    } else {
        Path::new(value)
            .strip_prefix("blobs")
            .unwrap_or_else(|_| Path::new(value))
            .to_path_buf()
    };
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(format!("путь блоба «{value}» выходит за каталог блобов"));
    }
    let root = blobs_directory
        .canonicalize()
        .map_err(|error| format!("каталог блобов не найден: {error}"))?;
    let candidate = root.join(&relative);
    let mut cursor = root.clone();
    for component in relative.components() {
        cursor.push(component.as_os_str());
        let metadata = std::fs::symlink_metadata(&cursor)
            .map_err(|_| format!("файл блоба «{value}» отсутствует в установленном каталоге"))?;
        if metadata.file_type().is_symlink() {
            return Err(format!(
                "путь блоба «{value}» содержит символическую ссылку"
            ));
        }
    }
    let metadata = std::fs::metadata(&candidate)
        .map_err(|_| format!("файл блоба «{value}» отсутствует в установленном каталоге"))?;
    let canonical = candidate
        .canonicalize()
        .map_err(|_| format!("не удалось проверить путь блоба «{value}»"))?;
    if !metadata.is_file() || !canonical.starts_with(&root) {
        return Err(format!("путь блоба «{value}» не является обычным файлом"));
    }
    Ok(canonical)
}

fn rewrite_foreign_blob_argument(
    argument: &str,
    blobs_directory: &Path,
) -> Result<Option<String>, String> {
    let Some(value) = argument.strip_prefix("--blob=") else {
        return Ok(None);
    };
    let Some((name, declaration)) = value.split_once(':') else {
        return Ok(Some(argument.to_owned()));
    };
    let Some((offset, path)) = declaration.split_once('@') else {
        // Inline hexadecimal blobs are part of the config itself and must not
        // be replaced with a same-named blob from the active installation.
        return Ok(Some(argument.to_owned()));
    };
    if !offset.is_empty()
        && !offset
            .strip_prefix('+')
            .unwrap_or(offset)
            .bytes()
            .all(|byte| byte.is_ascii_digit())
    {
        return Err(format!(
            "неподдерживаемый формат смещения блоба «{offset}@»"
        ));
    }
    let path = resolve_foreign_blob_path(path, blobs_directory)?;
    Ok(Some(format!("--blob={name}:{offset}@{}", path.display())))
}

fn resolve_foreign_lua_path(value: &str, lua_directory: &Path) -> Result<PathBuf, String> {
    const SOURCE_PREFIXES: [&str; 6] = [
        "/opt/etc/nfqws2/lua/",
        "/etc/nfqws2/lua/",
        "/opt/etc/nfqws/lua/",
        "/etc/nfqws/lua/",
        "/opt/zapret2/lua/",
        "/opt/zapret/lua/",
    ];
    if value.is_empty() || value.contains(['\\', '\0']) {
        return Err("Путь к Lua-файлу пустой или содержит недопустимые символы".into());
    }
    let value = value.trim_start_matches('@');
    let relative = if value.starts_with('/') {
        if let Ok(relative) = Path::new(value).strip_prefix(lua_directory) {
            relative.to_path_buf()
        } else if let Some(relative) = SOURCE_PREFIXES
            .iter()
            .find_map(|prefix| value.strip_prefix(prefix))
        {
            PathBuf::from(relative)
        } else {
            return Err(format!(
                "путь Lua-файла «{value}» не относится к поддерживаемой раскладке nfqws2/zapret"
            ));
        }
    } else {
        Path::new(value)
            .strip_prefix("lua")
            .unwrap_or_else(|_| Path::new(value))
            .to_path_buf()
    };
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, std::path::Component::Normal(_)))
    {
        return Err(format!("путь Lua-файла «{value}» выходит за каталог Lua"));
    }
    let root = lua_directory
        .canonicalize()
        .map_err(|error| format!("каталог Lua не найден: {error}"))?;
    let mut candidates = vec![relative.clone()];
    if relative
        .extension()
        .is_some_and(|extension| extension == "lua")
    {
        candidates.push(relative.with_extension("lua.gz"));
    }
    for relative in candidates {
        let candidate = root.join(&relative);
        let mut cursor = root.clone();
        let mut symlink_found = false;
        let mut missing = false;
        for component in relative.components() {
            cursor.push(component.as_os_str());
            match std::fs::symlink_metadata(&cursor) {
                Ok(metadata) if metadata.file_type().is_symlink() => {
                    symlink_found = true;
                    break;
                }
                Ok(_) => {}
                Err(_) => {
                    missing = true;
                    break;
                }
            }
        }
        if symlink_found {
            return Err(format!(
                "путь Lua-файла «{value}» содержит символическую ссылку"
            ));
        }
        if missing {
            continue;
        }
        let metadata = std::fs::metadata(&candidate)
            .map_err(|_| format!("не удалось проверить Lua-файл «{value}»"))?;
        let canonical = candidate
            .canonicalize()
            .map_err(|_| format!("не удалось проверить Lua-файл «{value}»"))?;
        if metadata.is_file() && canonical.starts_with(&root) {
            return Ok(canonical);
        }
    }
    Err(format!(
        "Lua-файл «{value}» отсутствует в установленном каталоге"
    ))
}

struct ForeignAutoListCleanup {
    path: PathBuf,
    armed: bool,
}

impl Drop for ForeignAutoListCleanup {
    fn drop(&mut self) {
        if self.armed {
            let _ = std::fs::remove_dir_all(&self.path);
        }
    }
}

async fn prepare_foreign_auto_lists(
    directory: &Path,
    auto_lists: &[(PathBuf, Option<PathBuf>)],
    cleanup: &mut ForeignAutoListCleanup,
) -> Result<(), String> {
    if auto_lists.is_empty() {
        return Ok(());
    }
    // Exclusive creation prevents a pre-created symlink in the shared temp
    // directory from redirecting writes outside this operation's scratch area.
    #[cfg(unix)]
    {
        use std::os::unix::fs::DirBuilderExt;
        let mut builder = std::fs::DirBuilder::new();
        builder.mode(0o700);
        builder
            .create(directory)
            .map_err(|error| format!("Не удалось создать временные списки проверки: {error}"))?;
    }
    #[cfg(not(unix))]
    tokio::fs::create_dir(directory)
        .await
        .map_err(|error| format!("Не удалось создать временные списки проверки: {error}"))?;
    cleanup.armed = true;
    for (destination, source) in auto_lists {
        let contents = if let Some(source) = source {
            match tokio::fs::metadata(source).await {
                Ok(metadata) if metadata.len() <= MAX_FOREIGN_AUTO_LIST_BYTES => {
                    let file = tokio::fs::File::open(source).await.map_err(|error| {
                        format!("Не удалось открыть автосписок проверяемого конфига: {error}")
                    })?;
                    let mut contents = Vec::new();
                    file.take(MAX_FOREIGN_AUTO_LIST_BYTES + 1)
                        .read_to_end(&mut contents)
                        .await
                        .map_err(|error| {
                            format!("Не удалось прочитать автосписок проверяемого конфига: {error}")
                        })?;
                    if contents.len() as u64 > MAX_FOREIGN_AUTO_LIST_BYTES {
                        return Err(format!(
                            "Автосписок проверяемого конфига превышает лимит {} МиБ",
                            MAX_FOREIGN_AUTO_LIST_BYTES / (1024 * 1024)
                        ));
                    }
                    Some(contents)
                }
                Ok(_) => {
                    return Err(format!(
                        "Автосписок проверяемого конфига превышает лимит {} МиБ",
                        MAX_FOREIGN_AUTO_LIST_BYTES / (1024 * 1024)
                    ));
                }
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
                Err(error) => {
                    return Err(format!(
                        "Не удалось проверить автосписок проверяемого конфига: {error}"
                    ));
                }
            }
        } else {
            None
        };
        #[cfg(unix)]
        {
            let mut file = tokio::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&destination)
                .await
                .map_err(|error| format!("Не удалось создать временный автосписок: {error}"))?;
            file.write_all(contents.as_deref().unwrap_or_default())
                .await
                .map_err(|error| format!("Не удалось записать временный автосписок: {error}"))?;
        }
        #[cfg(not(unix))]
        tokio::fs::write(destination, contents.unwrap_or_default())
            .await
            .map_err(|error| format!("Не удалось создать временный автосписок: {error}"))?;
    }
    Ok(())
}

fn parse_public_https_config_url(value: &str) -> Result<(String, String), String> {
    if value.len() > 2048 || !value.starts_with("https://") {
        return Err("Ссылка должна начинаться с https://".into());
    }
    let authority = value[8..].split(['/', '?', '#']).next().unwrap_or_default();
    if authority.is_empty() || authority.contains('@') || authority.contains(['\\', '\r', '\n']) {
        return Err("Ссылка содержит недопустимый адрес или учётные данные".into());
    }
    let host = if let Some((host, port)) = authority.rsplit_once(':') {
        if port != "443" || host.contains(':') {
            return Err("Для загрузки конфигурации разрешён только HTTPS-порт 443".into());
        }
        host
    } else {
        authority
    };
    // Keep the URL host byte-for-byte aligned with the host passed to curl's
    // --resolve pin. Curl treats a trailing-dot FQDN as a distinct URL host;
    // normalizing it before pinning could otherwise let curl resolve it again.
    if host.ends_with('.') {
        return Err("В ссылке не допускается точка в конце доменного имени".into());
    }
    let host = site_diagnostics::normalize_host(host)?;
    if host != "github.com" {
        return Ok((host, value.to_owned()));
    }

    // Match Zemidala's convenience for GitHub blob links, but keep the
    // stricter HTTPS-only, public-DNS-pinned fetch used by this backend.
    let path_and_suffix = &value[8 + authority.len()..];
    let suffix_offset = path_and_suffix
        .find(['?', '#'])
        .unwrap_or(path_and_suffix.len());
    let (path, suffix) = path_and_suffix.split_at(suffix_offset);
    let segments = path
        .strip_prefix('/')
        .unwrap_or_default()
        .splitn(4, '/')
        .collect::<Vec<_>>();
    if segments.len() != 4
        || segments[0].is_empty()
        || segments[1].is_empty()
        || segments[2] != "blob"
        || !segments[3].contains('/')
    {
        return Err(
            "Для GitHub укажите ссылку на файл в формате /owner/repo/blob/ветка/файл".into(),
        );
    }
    let raw_path = format!("{}/{}/{}", segments[0], segments[1], segments[3]);
    Ok((
        "raw.githubusercontent.com".into(),
        format!("https://raw.githubusercontent.com/{raw_path}{suffix}"),
    ))
}

async fn fetch_external_config(url: &str) -> Result<String, String> {
    let (host, url) = parse_public_https_config_url(url)?;
    let address = site_diagnostics::public_ipv4_references(&host)
        .await
        .into_iter()
        .find(|address| site_diagnostics::public_ip(*address))
        .ok_or_else(|| "Ссылка не разрешилась в публичный IPv4-адрес".to_string())?;
    let args = vec![
        "--disable".into(),
        "--silent".into(),
        "--show-error".into(),
        "--fail".into(),
        "--noproxy".into(),
        "*".into(),
        "--proto".into(),
        "=https".into(),
        "--connect-timeout".into(),
        "3".into(),
        "--max-time".into(),
        "8".into(),
        "--max-filesize".into(),
        MAX_FOREIGN_CONFIG_BYTES.to_string(),
        "--max-redirs".into(),
        "0".into(),
        "--resolve".into(),
        format!("{host}:443:{address}"),
        url,
    ];
    let (status, bytes) = site_diagnostics::capture(
        site_diagnostics::curl_program(),
        &args,
        9,
        MAX_FOREIGN_CONFIG_BYTES,
    )
    .await?;
    if status != 0 {
        return Err(format!(
            "Загрузка конфигурации завершилась с кодом {status}"
        ));
    }
    String::from_utf8(bytes).map_err(|_| "Загруженный файл не является UTF-8 текстом".into())
}

#[derive(serde::Deserialize)]
struct DpiLabCaptureQuery {
    name: Option<String>,
}

fn dpi_lab_error_response(error: String) -> Response {
    let status = if error.contains("лимит") || error.contains("больше 5000") {
        StatusCode::PAYLOAD_TOO_LARGE
    } else if error.contains("не удалось") || error.contains("Не удалось") {
        StatusCode::INTERNAL_SERVER_ERROR
    } else {
        StatusCode::BAD_REQUEST
    };
    (status, Json(serde_json::json!({"error": error}))).into_response()
}

async fn analyze_dpi_capture(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DpiLabCaptureQuery>,
    body: Bytes,
) -> Response {
    let name = query.name.unwrap_or_else(|| "capture.pcap".into());
    let _guard = state.dpi_lab_lock.lock().await;
    match dpi_lab::analyze_pcap(&name, &body) {
        Ok(analysis) => Json(analysis).into_response(),
        Err(error) => dpi_lab_error_response(error.to_string()),
    }
}

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct DpiLabStrategyRequest {
    source: String,
}

async fn analyze_dpi_strategy(
    State(state): State<Arc<AppState>>,
    Json(request): Json<DpiLabStrategyRequest>,
) -> Response {
    if request.source.len() > dpi_lab::MAX_STRATEGY_SOURCE_BYTES {
        return error(
            StatusCode::PAYLOAD_TOO_LARGE,
            format!(
                "Текст стратегии превышает лимит {} КиБ",
                dpi_lab::MAX_STRATEGY_SOURCE_BYTES / 1024
            ),
        );
    }
    let Some(paths) = discover_system_paths() else {
        return error(
            StatusCode::NOT_FOUND,
            "Установка nfqws2 не обнаружена; каталог Lua-функций недоступен",
        );
    };
    let catalog = load_cached_nfqws_lua_catalog(&state, &paths.lua_dir, false).await;
    match dpi_lab::analyze_strategy_with_standard_groups(
        &request.source,
        &catalog.functions,
        &catalog.standard_groups,
    ) {
        Ok(analysis) => Json(analysis).into_response(),
        Err(error) => dpi_lab_error_response(error.to_string()),
    }
}

async fn save_dpi_experiment(
    State(state): State<Arc<AppState>>,
    Query(query): Query<DpiLabCaptureQuery>,
    body: Bytes,
) -> Response {
    let name = query.name.unwrap_or_else(|| "capture.pcap".into());
    let _guard = state.dpi_lab_lock.lock().await;
    match dpi_lab::save_experiment(&state.data_dir, &name, &body).await {
        Ok(experiment) => (StatusCode::CREATED, Json(experiment)).into_response(),
        Err(error) => dpi_lab_error_response(error),
    }
}

async fn list_dpi_experiments(State(state): State<Arc<AppState>>) -> Response {
    let _guard = state.dpi_lab_lock.lock().await;
    match dpi_lab::list_experiments(&state.data_dir).await {
        Ok(items) => Json(serde_json::json!({"items": items})).into_response(),
        Err(error) => dpi_lab_error_response(error),
    }
}

async fn get_dpi_experiment(
    State(state): State<Arc<AppState>>,
    AxumPath(id): AxumPath<String>,
) -> Response {
    let _guard = state.dpi_lab_lock.lock().await;
    match dpi_lab::load_experiment(&state.data_dir, &id).await {
        Ok(analysis) => Json(analysis).into_response(),
        Err(error) if error.contains("не найден") => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": error})),
        )
            .into_response(),
        Err(error) => dpi_lab_error_response(error),
    }
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

async fn start_foreign_config_check(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ForeignConfigCheckRequest>,
) -> Response {
    let repeats = normalize_foreign_config_repeats(request.repeats);
    if request.configs.is_empty() || request.configs.len() > MAX_FOREIGN_CONFIGS {
        return error(
            StatusCode::BAD_REQUEST,
            "Добавьте от одного до десяти конфигов для сравнения",
        );
    }
    let mut hosts = Vec::new();
    for input in &request.hosts {
        let host = match site_diagnostics::normalize_site_target(input) {
            Ok(host) => host,
            Err(message) => return error(StatusCode::BAD_REQUEST, message),
        };
        if !hosts.contains(&host) {
            hosts.push(host);
        }
    }
    if let Err(message) = validate_foreign_config_counts(request.configs.len(), hosts.len()) {
        return error(StatusCode::BAD_REQUEST, message);
    }
    let Some((_, active_args)) = active_process_snapshot() else {
        return error(
            StatusCode::CONFLICT,
            "Не найден однозначно работающий nfqws2 для сравнения",
        );
    };
    let Some(paths) = discover_system_paths() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Пути установленного nfqws2 не обнаружены однозначно",
        );
    };
    let binary = paths.binary.to_string_lossy().into_owned();
    let id = new_update_id();
    let auto_list_directory = std::env::temp_dir().join(format!("nfqws2-ui-foreign-config-{id}"));
    let mut auto_list_cleanup = ForeignAutoListCleanup {
        path: auto_list_directory.clone(),
        armed: false,
    };
    let mut prepared = Vec::new();
    let mut prepared_auto_lists = Vec::new();
    let mut total_bytes = 0usize;
    for (index, input) in request.configs.into_iter().enumerate() {
        let is_url = input.url.is_some();
        let content = match (input.content, input.url) {
            (Some(content), None) if !content.trim().is_empty() => content,
            (None, Some(url)) => match fetch_external_config(&url).await {
                Ok(content) => content,
                Err(message) => return error(StatusCode::UNPROCESSABLE_ENTITY, message),
            },
            _ => {
                return error(
                    StatusCode::BAD_REQUEST,
                    "Для каждого конфига укажите содержимое или HTTPS-ссылку",
                );
            }
        };
        if content.len() > MAX_FOREIGN_CONFIG_BYTES {
            return error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Каждый конфиг ограничен 64 КиБ",
            );
        }
        total_bytes = total_bytes.saturating_add(content.len());
        if total_bytes > MAX_FOREIGN_CONFIG_BYTES * MAX_FOREIGN_CONFIGS {
            return error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Суммарный размер конфигов превышает лимит",
            );
        }
        let isolated_auto_list = auto_list_directory.join(format!("config-{index}.list"));
        let adapted_content = match foreign_config_with_zemidala_defaults(
            &content,
            &paths.lua_dir,
            &paths.blobs_dir,
            &paths.lists_dir,
        ) {
            Ok(content) => content,
            Err(message) => return error(StatusCode::UNPROCESSABLE_ENTITY, message),
        };
        let args = match external_config_test_args(
            &adapted_content,
            &isolated_auto_list,
            &paths.lua_dir,
            &paths.blobs_dir,
            &paths.lists_dir,
            &mut prepared_auto_lists,
        ) {
            Ok(args) => args,
            Err(message) => return error(StatusCode::UNPROCESSABLE_ENTITY, message),
        };
        let dry_run = match foreign_config_dry_run_argv(&binary, &adapted_content, &args) {
            Ok(argv) => argv,
            Err(message) => return error(StatusCode::UNPROCESSABLE_ENTITY, message),
        };
        let name = input.name.trim().chars().take(48).collect::<String>();
        let name = if name.is_empty() {
            format!("Конфиг {}", index + 1)
        } else {
            name
        };
        let source = if input.source.as_deref() == Some("Файл") {
            "Файл"
        } else if input.source.as_deref() == Some("Ссылка") || is_url {
            "Ссылка"
        } else {
            "Текст"
        }
        .to_owned();
        prepared.push((name, source, args, dry_run, adapted_content));
    }
    let Ok(guard) = state.site_diagnostic_lock.clone().try_lock_owned() else {
        return error(
            StatusCode::CONFLICT,
            "Сейчас выполняется другая сетевая диагностика; дождитесь её завершения",
        );
    };
    if let Err(message) = prepare_foreign_auto_lists(
        &auto_list_directory,
        &prepared_auto_lists,
        &mut auto_list_cleanup,
    )
    .await
    {
        return error(StatusCode::UNPROCESSABLE_ENTITY, message);
    }
    let directory = state.data_dir.join("foreign-config-check-operations");
    prune_foreign_config_operations(&directory).await;
    let control = Arc::new(AtomicBool::new(false));
    let comparisons = prepared
        .iter()
        .map(|(name, source, _, _, _)| ForeignConfigComparison {
            name: name.clone(),
            source: source.clone(),
            sites: hosts
                .iter()
                .map(|host| ForeignConfigSiteResult {
                    host: host.clone(),
                    current_ok: None,
                    current_detail: "Ожидает проверки".into(),
                    current_attempts: 0,
                    current_successes: 0,
                    current_average_ms: None,
                    candidate_ok: None,
                    candidate_detail: "Ожидает проверки".into(),
                    candidate_attempts: 0,
                    candidate_successes: 0,
                    candidate_average_ms: None,
                })
                .collect(),
        })
        .collect();
    let operation = ForeignConfigCheckOperation {
        id: id.clone(),
        state: "queued".into(),
        stage: "queued".into(),
        current: None,
        done: 0,
        total: hosts.len().saturating_mul(prepared.len() + 1),
        podkop_hosts: detect_podkop_hosts(&hosts).await,
        hosts,
        comparisons,
        message: "Проверка конфигов поставлена в очередь; активная конфигурация не меняется".into(),
        updated_at: now_unix_seconds(),
    };
    if let Err(message) = write_foreign_config_operation_sync(&directory, &operation) {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    for (index, (_, _, _, _, content)) in prepared.iter().enumerate() {
        let candidate_path = foreign_config_candidate_path(&directory, &id, index);
        if let Err(write_error) = std::fs::write(&candidate_path, content.as_bytes()) {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Не удалось сохранить проверенный конфиг: {write_error}"),
            );
        }
        #[cfg(unix)]
        if let Err(permission_error) =
            std::fs::set_permissions(&candidate_path, std::fs::Permissions::from_mode(0o600))
        {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Не удалось защитить проверенный конфиг: {permission_error}"),
            );
        }
    }
    state
        .foreign_config_controls
        .lock()
        .await
        .insert(id.clone(), control.clone());
    let task_state = state.clone();
    let task_id = id.clone();
    let task_active_args = active_args;
    let worker_operation = operation.clone();
    tokio::spawn(async move {
        let _auto_list_cleanup = auto_list_cleanup;
        let _guard = guard;
        let operation_dir = task_state.data_dir.join("foreign-config-check-operations");
        let progress = Arc::new(std::sync::Mutex::new(worker_operation));
        if let Ok(mut operation) = progress.lock() {
            operation.state = "running".into();
            operation.stage = "validation".into();
            operation.message = "Проверяется безопасность и совместимость конфигов".into();
            operation.updated_at = now_unix_seconds();
            let _ = write_foreign_config_operation_sync(&operation_dir, &operation);
        }
        for (name, _, _, dry_run, _) in &prepared {
            if let Err(message) = validate_strategy_dry_run(dry_run).await {
                if let Ok(mut operation) = progress.lock() {
                    operation.state = "failed".into();
                    operation.stage = "failed".into();
                    operation.message = format!("«{name}»: {message}");
                    operation.updated_at = now_unix_seconds();
                    let _ = write_foreign_config_operation_sync(&operation_dir, &operation);
                }
                task_state
                    .foreign_config_controls
                    .lock()
                    .await
                    .remove(&task_id);
                return;
            }
        }
        let meminfo = tokio::fs::read_to_string("/proc/meminfo")
            .await
            .unwrap_or_default();
        if let Some(message) = foreign_config_memory_warning(&meminfo) {
            if let Ok(mut operation) = progress.lock() {
                operation.state = "failed".into();
                operation.stage = "resource_guard".into();
                operation.current = None;
                operation.message = message;
                operation.updated_at = now_unix_seconds();
                let _ = write_foreign_config_operation_sync(&operation_dir, &operation);
            }
            task_state
                .foreign_config_controls
                .lock()
                .await
                .remove(&task_id);
            return;
        }
        let runner = task_state.site_trace_runner.clone();
        let mut stopped = false;
        let total_hosts = progress
            .lock()
            .ok()
            .map(|operation| operation.hosts.len())
            .unwrap_or(0);
        for host_index in 0..total_hosts {
            if control.load(Ordering::Acquire) {
                stopped = true;
                break;
            }
            let (host, active_args) = match progress.lock() {
                Ok(operation) => (
                    operation.hosts[host_index].clone(),
                    task_active_args.clone(),
                ),
                Err(_) => break,
            };
            if let Ok(mut operation) = progress.lock() {
                operation.stage = "active".into();
                operation.current = Some(format!("Текущий конфиг · {host}"));
                operation.message = format!("Проверяется установленный конфиг: {host}");
                operation.updated_at = now_unix_seconds();
                let _ = write_foreign_config_operation_sync(&operation_dir, &operation);
            }
            let baseline = runner(
                host.clone(),
                "https".into(),
                active_args,
                binary.clone(),
                control.clone(),
                1,
                None,
                None,
            )
            .await;
            let direct_ok = baseline.without_nfqws2.ok;
            if let Ok(mut operation) = progress.lock() {
                for comparison in &mut operation.comparisons {
                    if let Some(site) = comparison.sites.get_mut(host_index) {
                        site.current_ok = baseline.with_nfqws2.ok;
                        site.current_detail = baseline.with_nfqws2.reason.clone();
                        site.current_attempts = 1;
                        site.current_successes = usize::from(baseline.with_nfqws2.ok == Some(true));
                        site.current_average_ms = baseline.with_nfqws2.duration_ms;
                    }
                }
                operation.done = operation.done.saturating_add(1);
                operation.updated_at = now_unix_seconds();
                let _ = write_foreign_config_operation_sync(&operation_dir, &operation);
            }
            for (config_index, (_, _, args, _, _)) in prepared.iter().enumerate() {
                if control.load(Ordering::Acquire) {
                    stopped = true;
                    break;
                }
                if let Ok(mut operation) = progress.lock() {
                    operation.stage = "candidate".into();
                    operation.current = Some(format!(
                        "{} · {host}",
                        operation.comparisons[config_index].name
                    ));
                    operation.message = format!(
                        "Проверяется конфиг «{}»: {host}",
                        operation.comparisons[config_index].name
                    );
                    operation.updated_at = now_unix_seconds();
                    let _ = write_foreign_config_operation_sync(&operation_dir, &operation);
                }
                let mut reports = Vec::with_capacity(repeats);
                for attempt in 0..repeats {
                    if control.load(Ordering::Acquire) {
                        stopped = true;
                        break;
                    }
                    if let Ok(mut operation) = progress.lock() {
                        operation.current = Some(format!(
                            "{} · {host} · проверка {}/{}",
                            operation.comparisons[config_index].name,
                            attempt + 1,
                            repeats
                        ));
                        operation.updated_at = now_unix_seconds();
                        let _ = write_foreign_config_operation_sync(&operation_dir, &operation);
                    }
                    let report = runner(
                        host.clone(),
                        "https".into(),
                        args.clone(),
                        binary.clone(),
                        control.clone(),
                        1,
                        Some(direct_ok.unwrap_or(false)),
                        None,
                    )
                    .await;
                    let outcome = report.with_nfqws2.ok;
                    reports.push(report);
                    // Match Zemidala's fail-fast confirmation: one failed
                    // attempt rules out this config for the site.
                    if !foreign_config_should_retry(outcome, attempt, repeats) {
                        break;
                    }
                }
                if stopped {
                    break;
                }
                let successes = reports
                    .iter()
                    .filter(|report| report.with_nfqws2.ok == Some(true))
                    .count();
                let successful_durations = reports
                    .iter()
                    .filter(|report| report.with_nfqws2.ok == Some(true))
                    .filter_map(|report| report.with_nfqws2.duration_ms)
                    .collect::<Vec<_>>();
                let average_ms = (!successful_durations.is_empty()).then(|| {
                    (successful_durations.iter().sum::<u64>()
                        + successful_durations.len() as u64 / 2)
                        / successful_durations.len() as u64
                });
                let confirmed = successes == repeats;
                let last_report = reports.last();
                let candidate_detail = format!(
                    "{successes}/{} успешных · {}",
                    repeats,
                    last_report
                        .map(|report| report.with_nfqws2.reason.as_str())
                        .unwrap_or("проверка отменена")
                );
                if let Ok(mut operation) = progress.lock() {
                    if let Some(site) = operation.comparisons[config_index]
                        .sites
                        .get_mut(host_index)
                    {
                        site.candidate_ok = Some(confirmed);
                        site.candidate_detail = candidate_detail;
                        site.candidate_attempts = reports.len();
                        site.candidate_successes = successes;
                        site.candidate_average_ms = average_ms;
                    }
                    operation.done = operation.done.saturating_add(1);
                    operation.updated_at = now_unix_seconds();
                    let _ = write_foreign_config_operation_sync(&operation_dir, &operation);
                }
            }
            if stopped {
                break;
            }
        }
        if let Ok(mut operation) = progress.lock() {
            operation.state = if stopped || control.load(Ordering::Acquire) {
                "cancelled"
            } else {
                "complete"
            }
            .into();
            operation.stage = operation.state.clone();
            operation.current = None;
            operation.message = if operation.state == "cancelled" {
                "Проверка остановлена; завершённые сравнения сохранены".into()
            } else {
                "Проверка конфигураций завершена; активный конфиг не изменялся".into()
            };
            operation.updated_at = now_unix_seconds();
            let _ = write_foreign_config_operation_sync(&operation_dir, &operation);
        }
        task_state
            .foreign_config_controls
            .lock()
            .await
            .remove(&task_id);
    });
    (StatusCode::ACCEPTED, Json(operation)).into_response()
}

async fn get_foreign_config_check(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    let directory = state.data_dir.join("foreign-config-check-operations");
    match read_foreign_config_operation(&directory, &id).await {
        Ok(Some(mut operation)) => {
            if matches!(operation.state.as_str(), "queued" | "running" | "stopping")
                && !state.foreign_config_controls.lock().await.contains_key(&id)
            {
                operation.state = "failed".into();
                operation.stage = "failed".into();
                operation.current = None;
                operation.message =
                    "Процесс проверки завершился; сохранённые результаты доступны".into();
                operation.updated_at = now_unix_seconds();
                let _ = write_foreign_config_operation_sync(&directory, &operation);
            }
            Json(operation).into_response()
        }
        Ok(None) => error(StatusCode::NOT_FOUND, "Проверка конфигурации не найдена"),
        Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

async fn apply_foreign_config_candidate(
    State(state): State<Arc<AppState>>,
    axum::extract::Path((id, index)): axum::extract::Path<(String, usize)>,
) -> Response {
    if !valid_site_trace_id(&id) || index >= MAX_FOREIGN_CONFIGS {
        return error(StatusCode::NOT_FOUND, "Проверенный конфиг не найден");
    }
    let directory = state.data_dir.join("foreign-config-check-operations");
    let operation = match read_foreign_config_operation(&directory, &id).await {
        Ok(Some(operation)) => operation,
        Ok(None) => return error(StatusCode::NOT_FOUND, "Проверка конфигурации не найдена"),
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    };
    let Some(comparison) = operation.comparisons.get(index) else {
        return error(StatusCode::NOT_FOUND, "Проверенный конфиг не найден");
    };
    if !foreign_config_candidate_is_applicable(&operation.state, comparison) {
        return error(
            StatusCode::CONFLICT,
            "Применить можно только конфигурацию, успешно прошедшую все проверки",
        );
    }
    let candidate_path = foreign_config_candidate_path(&directory, &id, index);
    let content = match tokio::fs::read_to_string(&candidate_path).await {
        Ok(content) if content.len() <= MAX_CONFIG_BYTES => content,
        Ok(_) => {
            return error(
                StatusCode::PAYLOAD_TOO_LARGE,
                "Проверенная конфигурация превышает лимит",
            );
        }
        Err(read_error) if read_error.kind() == std::io::ErrorKind::NotFound => {
            return error(StatusCode::GONE, "Файл проверенной конфигурации уже удалён");
        }
        Err(read_error) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Не удалось прочитать проверенную конфигурацию: {read_error}"),
            );
        }
    };
    let Some(path) = config_path_for_state(&state) else {
        return error(
            StatusCode::NOT_FOUND,
            "Разрешённый конфигурационный файл не найден",
        );
    };
    let original = match tokio::fs::read(&path).await {
        Ok(original) => original,
        Err(read_error) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Не удалось прочитать текущую конфигурацию: {read_error}"),
            );
        }
    };
    let write = ConfigWrite {
        content,
        revision: revision(&original),
    };
    match apply_config_transaction(&state, write, || async {
        service_action(ServiceAction::Restart).await
    })
    .await
    {
        Ok(result) => Json(result).into_response(),
        Err(message) if message == WRITE_REVISION_CONFLICT => error(
            StatusCode::CONFLICT,
            "Конфигурация изменилась во время применения; повторите попытку",
        ),
        Err(message) => error(StatusCode::BAD_GATEWAY, message),
    }
}

async fn cancel_foreign_config_check(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    if !valid_site_trace_id(&id) {
        return error(StatusCode::NOT_FOUND, "Проверка конфигурации не найдена");
    }
    let directory = state.data_dir.join("foreign-config-check-operations");
    let Some(mut operation) = (match read_foreign_config_operation(&directory, &id).await {
        Ok(operation) => operation,
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }) else {
        return error(StatusCode::NOT_FOUND, "Проверка конфигурации не найдена");
    };
    if !matches!(operation.state.as_str(), "queued" | "running" | "stopping") {
        return Json(operation).into_response();
    }
    let controls = state.foreign_config_controls.lock().await;
    if let Some(control) = controls.get(&id) {
        control.store(true, Ordering::Release);
        operation.state = "stopping".into();
        operation.stage = "stopping".into();
        operation.message = "Остановка после безопасной очистки тестового процесса".into();
        operation.updated_at = now_unix_seconds();
        if let Err(message) = write_foreign_config_operation_sync(&directory, &operation) {
            return error(StatusCode::INTERNAL_SERVER_ERROR, message);
        }
    }
    Json(operation).into_response()
}

fn picker_mode_uses_catalog_candidates(custom_mode: bool) -> bool {
    !custom_mode
}

fn normalize_multi_strategy_hosts(hosts: &[String]) -> Result<Vec<String>, String> {
    if !(2..=15).contains(&hosts.len()) {
        return Err("Укажите от 2 до 15 доменов".into());
    }
    let mut normalized = Vec::with_capacity(hosts.len());
    for raw in hosts {
        let host = site_diagnostics::normalize_site_target(raw)?;
        if host.parse::<std::net::IpAddr>().is_ok() {
            return Err("В многосайтовом подборе указываются домены, не IP-адреса".into());
        }
        if normalized.iter().any(|existing| existing == &host) {
            return Err(format!("Домен «{host}» указан несколько раз"));
        }
        normalized.push(host);
    }
    Ok(normalized)
}

async fn start_multi_strategy_pick(
    State(state): State<Arc<AppState>>,
    Json(request): Json<MultiStrategyPickRequest>,
) -> Response {
    let hosts = match normalize_multi_strategy_hosts(&request.hosts) {
        Ok(hosts) => hosts,
        Err(message) => return error(StatusCode::BAD_REQUEST, message),
    };
    let repeats = request.repeats.unwrap_or(3);
    if !(2..=5).contains(&repeats) {
        return error(
            StatusCode::BAD_REQUEST,
            "Число повторов должно быть от 2 до 5",
        );
    }
    let protocol = match request.protocol.as_deref().unwrap_or("https") {
        "http" => StrategyProtocol::Http,
        "https" => StrategyProtocol::Https,
        _ => {
            return error(
                StatusCode::BAD_REQUEST,
                "Протокол должен быть http или https",
            );
        }
    };
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
            "Не удалось прочитать Lua-функции работающего nfqws2",
        );
    }
    let blobs = active_blob_names(&active_args);
    let configured = strategy_picker::configured_candidates(&active_args, protocol);
    let mut plan = strategy_picker::build_plan(
        protocol,
        StrategyMode::Quick,
        &configured,
        &functions,
        &blobs,
    );
    if request.include_other_history {
        let protocol_name = match protocol {
            StrategyProtocol::Http => "http",
            StrategyProtocol::Https => "https",
            StrategyProtocol::Quic => unreachable!(),
        };
        let history = load_confirmed_strategy_candidates(
            &state.data_dir.join("strategy-pick-operations"),
            &hosts[0],
            protocol_name,
            true,
        )
        .await;
        strategy_picker::insert_reusable_candidates(&mut plan, history, &functions, &blobs);
    }
    if request.include_community
        && let Some(asn) = community_strategies::read_provider_settings(&state.data_dir).asn
        && let Ok((items, _, _)) = community_strategies::load(&state.data_dir, asn, false).await
    {
        let protocol_name = match protocol {
            StrategyProtocol::Http => "http",
            StrategyProtocol::Https => "tls",
            StrategyProtocol::Quic => unreachable!(),
        };
        if let Some(host) = hosts.first() {
            let candidates = community_strategies::picker_candidates_for_host(
                items.clone(),
                host,
                protocol_name,
                now_unix_seconds(),
            )
            .await;
            strategy_picker::insert_reusable_candidates(&mut plan, candidates, &functions, &blobs);
        }
    }
    plan.candidates.truncate(128);
    if plan.candidates.is_empty() {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            "Не найдено совместимых стратегий для проверки",
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
    let id = new_update_id();
    let control = Arc::new(AtomicBool::new(false));
    let proto = match protocol {
        StrategyProtocol::Http => "http",
        StrategyProtocol::Https => "https",
        StrategyProtocol::Quic => unreachable!(),
    }
    .to_owned();
    let operation = StrategyPickOperation {
        id: id.clone(),
        host: hosts.join(", "),
        protocol: proto.clone(),
        state: "queued".into(),
        stage: "queued".into(),
        current: None,
        done: 0,
        total: plan.candidates.len(),
        results: Vec::new(),
        hosts: hosts.clone(),
        podkop_hosts: detect_podkop_hosts(&hosts).await,
        multi_site_results: Vec::new(),
        multi_site_baseline: Vec::new(),
        message: "Подбор общего профиля поставлен в очередь; исходный конфиг не меняется".into(),
        updated_at: now_unix_seconds(),
    };
    if let Err(message) = write_strategy_pick_operation_sync(&operation_dir, &operation) {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    state
        .strategy_pick_controls
        .lock()
        .await
        .insert(id.clone(), control.clone());
    let accepted = operation.clone();
    let task_state = state.clone();
    let task_id = id;
    let runner = state.site_trace_runner.clone();
    let binary = paths.binary.to_string_lossy().into_owned();
    let task_args = active_args.clone();
    let task_functions = functions.clone();
    let task_blobs = blobs.clone();
    let task_hosts = hosts.clone();
    let task_protocol = protocol;
    tokio::spawn(async move {
        let _guard = guard;
        run_multi_strategy_pick(
            task_state,
            task_id,
            operation,
            plan.candidates,
            task_hosts,
            proto,
            task_protocol,
            repeats,
            runner,
            binary,
            task_args,
            task_functions,
            task_blobs,
            control,
        )
        .await;
    });
    (StatusCode::ACCEPTED, Json(accepted)).into_response()
}

#[allow(clippy::too_many_arguments)]
async fn run_multi_strategy_pick(
    state: Arc<AppState>,
    id: String,
    mut operation: StrategyPickOperation,
    candidates: Vec<strategy_picker::StrategyCandidate>,
    hosts: Vec<String>,
    proto: String,
    protocol: StrategyProtocol,
    repeats: usize,
    runner: Arc<SiteTraceRunner>,
    binary: String,
    active_args: Vec<String>,
    functions: HashSet<String>,
    blobs: Vec<String>,
    control: Arc<AtomicBool>,
) {
    let directory = state.data_dir.join("strategy-pick-operations");
    operation.state = "running".into();
    operation.stage = "baseline".into();
    operation.updated_at = now_unix_seconds();
    let _ = write_strategy_pick_operation_sync(&directory, &operation);
    let mut eligible_hosts = Vec::new();
    for host in &hosts {
        if control.load(Ordering::Acquire) {
            break;
        }
        let report = runner(
            host.clone(),
            proto.clone(),
            Vec::new(),
            binary.clone(),
            control.clone(),
            1,
            None,
            None,
        )
        .await;
        let eligible = report.without_nfqws2.ok == Some(false);
        let reason = if eligible {
            "Прямой запрос не открыл домен".into()
        } else if report.without_nfqws2.ok == Some(true) {
            format!("Домен доступен напрямую ({})", report.without_nfqws2.reason)
        } else {
            format!(
                "Базовая проверка не дала результата ({})",
                report.without_nfqws2.reason
            )
        };
        if eligible {
            eligible_hosts.push(host.clone());
        }
        operation.multi_site_baseline.push(MultiSiteBaselineResult {
            host: host.clone(),
            eligible,
            reason,
        });
        operation.current = Some(host.clone());
        operation.message = format!(
            "Прямая проверка доменов: {}/{}",
            operation.multi_site_baseline.len(),
            hosts.len()
        );
        operation.updated_at = now_unix_seconds();
        let _ = write_strategy_pick_operation_sync(&directory, &operation);
    }
    if control.load(Ordering::Acquire) || eligible_hosts.len() < 2 {
        operation.state = if control.load(Ordering::Acquire) {
            "cancelled"
        } else {
            "site_unblocked"
        }
        .into();
        operation.stage = "baseline".into();
        operation.current = None;
        operation.message = if control.load(Ordering::Acquire) {
            "Подбор отменён во время прямой проверки".into()
        } else {
            "Менее двух доменов требуют обхода; общий профиль не подбирался".into()
        };
        operation.updated_at = now_unix_seconds();
        let _ = write_strategy_pick_operation_sync(&directory, &operation);
        state.strategy_pick_controls.lock().await.remove(&id);
        return;
    }
    let candidate_plan = strategy_picker::StrategyPlan {
        protocol,
        candidates,
    };
    let mut candidates = candidate_plan.candidates;
    candidates.truncate(128);
    let total = candidates.len();
    operation.total = total;
    let mut ranked = Vec::<(
        usize,
        strategy_picker::StrategyCandidate,
        Vec<MultiSiteDomainResult>,
    )>::new();
    for (index, candidate) in candidates.into_iter().enumerate() {
        if control.load(Ordering::Acquire) {
            break;
        }
        let args = match strategy_picker::candidate_argv(
            &active_args,
            protocol,
            &candidate.steps,
            &functions,
            &blobs,
        ) {
            Ok(args) => strategy_probe_argv(&binary, &args),
            Err(_) => continue,
        };
        let probes = run_multi_site_probes(
            runner.clone(),
            MultiSiteProbeRequest {
                hosts: &eligible_hosts,
                protocol: proto.clone(),
                args,
                binary: binary.clone(),
                control: control.clone(),
                repeats: 1,
                known_baseline: Some(false),
            },
        )
        .await;
        let mut domains = Vec::new();
        let mut opened = 0usize;
        for (host, phases) in probes {
            let successes = phases
                .iter()
                .filter(|phase| phase.ok == Some(true) && phase.state == "complete")
                .count();
            opened += usize::from(successes > 0);
            domains.push(MultiSiteDomainResult {
                host,
                eligible: true,
                baseline_reason: String::new(),
                attempts: phases.len(),
                successes,
                elapsed_ms: phases
                    .iter()
                    .map(|phase| phase.duration_ms.unwrap_or_default())
                    .sum(),
                confirmed: false,
            });
        }
        ranked.push((opened, candidate.clone(), domains));
        operation.current = Some(candidate.name);
        operation.done = index + 1;
        operation.stage = "testing".into();
        operation.message = format!("Первичный проход: {}/{} стратегий", index + 1, total);
        operation.updated_at = now_unix_seconds();
        operation.multi_site_results = ranked
            .iter()
            .map(|(n, c, d)| MultiSiteCandidateResult {
                candidate: c.clone(),
                opened_domains: *n,
                domains: d.clone(),
            })
            .collect();
        operation.results = ranked
            .iter()
            .map(
                |(_, candidate, domains)| strategy_picker::StrategyCandidateResult {
                    candidate: candidate.clone(),
                    attempts: domains.iter().map(|domain| domain.attempts).sum(),
                    successes: domains.iter().map(|domain| domain.successes).sum(),
                    confirmed: false,
                    elapsed_ms: domains.iter().map(|domain| domain.elapsed_ms).sum(),
                    detail: format!("Первичная проверка на {} доменах", domains.len()),
                },
            )
            .collect();
        let _ = write_strategy_pick_operation_sync(&directory, &operation);
    }
    ranked.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    for (_, candidate, domains) in ranked.iter_mut().take(5) {
        if control.load(Ordering::Acquire) {
            break;
        }
        let args = strategy_picker::candidate_argv(
            &active_args,
            protocol,
            &candidate.steps,
            &functions,
            &blobs,
        )
        .map(|args| strategy_probe_argv(&binary, &args))
        .unwrap_or_default();
        let hosts = domains
            .iter()
            .map(|domain| domain.host.clone())
            .collect::<Vec<_>>();
        let confirmations = run_multi_site_probes(
            runner.clone(),
            MultiSiteProbeRequest {
                hosts: &hosts,
                protocol: proto.clone(),
                args,
                binary: binary.clone(),
                control: control.clone(),
                repeats,
                known_baseline: Some(false),
            },
        )
        .await;
        for (domain, (_, phases)) in domains.iter_mut().zip(confirmations) {
            domain.attempts = 0;
            domain.successes = 0;
            domain.elapsed_ms = 0;
            domain.attempts = phases.len();
            domain.successes = phases
                .iter()
                .filter(|phase| phase.ok == Some(true) && phase.state == "complete")
                .count();
            domain.elapsed_ms = phases
                .iter()
                .map(|phase| phase.duration_ms.unwrap_or_default())
                .sum();
            domain.confirmed = !control.load(Ordering::Acquire)
                && domain.attempts == repeats
                && domain.successes == repeats;
        }
    }
    ranked.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    operation.multi_site_results = ranked
        .iter()
        .map(|(n, c, d)| MultiSiteCandidateResult {
            candidate: c.clone(),
            opened_domains: *n,
            domains: d.clone(),
        })
        .collect();
    operation.results = ranked
        .iter()
        .map(|(_, candidate, domains)| {
            let attempts = domains.iter().map(|d| d.attempts).sum();
            let successes = domains.iter().map(|d| d.successes).sum();
            let confirmed = !domains.is_empty() && domains.iter().all(|d| d.confirmed);
            strategy_picker::StrategyCandidateResult {
                candidate: candidate.clone(),
                attempts,
                successes,
                confirmed,
                elapsed_ms: domains.iter().map(|d| d.elapsed_ms).sum(),
                detail: format!("Подтверждение по {} доменам", domains.len()),
            }
        })
        .collect();
    operation.state = if control.load(Ordering::Acquire) {
        "cancelled"
    } else if operation.results.iter().any(|r| r.confirmed) {
        "complete"
    } else if operation
        .multi_site_results
        .iter()
        .any(|r| r.opened_domains > 0)
    {
        "partial"
    } else {
        "no_match"
    }
    .into();
    operation.stage = operation.state.clone();
    operation.current = None;
    operation.message = if operation.results.iter().any(|r| r.confirmed) {
        "Найден общий профиль, подтверждённый на всех выбранных доменах".into()
    } else if operation.state == "partial" {
        "Есть частичные результаты; общего подтверждённого профиля нет".into()
    } else if operation.state == "cancelled" {
        "Подбор отменён; результаты сохранены".into()
    } else {
        "Общий профиль не прошёл проверку на выбранных доменах".into()
    };
    operation.updated_at = now_unix_seconds();
    let _ = write_strategy_pick_operation_sync(&directory, &operation);
    state.strategy_pick_controls.lock().await.remove(&id);
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
    let host = match site_diagnostics::normalize_site_target(&request.host) {
        Ok(host) => host,
        Err(message) => return error(StatusCode::BAD_REQUEST, message),
    };
    let protocol = match request.protocol.as_deref().unwrap_or("https") {
        "http" => StrategyProtocol::Http,
        "https" => StrategyProtocol::Https,
        "quic" => StrategyProtocol::Quic,
        _ => {
            return error(
                StatusCode::BAD_REQUEST,
                "Протокол должен быть http, https или quic",
            );
        }
    };
    if protocol == StrategyProtocol::Quic && host.parse::<std::net::IpAddr>().is_ok() {
        return error(
            StatusCode::BAD_REQUEST,
            "Для QUIC-подбора укажите домен: проверка QUIC Initial требует TLS SNI",
        );
    }
    let custom_mode = request.mode.as_deref() == Some("custom");
    let mut mode = match request.mode.as_deref().unwrap_or("quick") {
        "quick" => Some(StrategyMode::Quick),
        "extended" => Some(StrategyMode::Extended),
        "full" => Some(StrategyMode::Full),
        "custom" => Some(StrategyMode::Quick),
        _ => return error(StatusCode::BAD_REQUEST, "Неизвестный режим подбора"),
    };
    if protocol == StrategyProtocol::Quic {
        // QUIC only uses the small sequential candidate set. In particular,
        // never route it into the HTTP full-catalog batch runner.
        mode = Some(StrategyMode::Quick);
    }
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
    if request.include_community
        && protocol != StrategyProtocol::Quic
        && !custom_mode
        && let Some(asn) = community_strategies::read_provider_settings(&state.data_dir).asn
        && let Ok((items, _, _)) = community_strategies::load(&state.data_dir, asn, false).await
    {
        let protocol_name = match protocol {
            StrategyProtocol::Http => "http",
            _ => "tls",
        };
        let candidates = community_strategies::picker_candidates_for_host(
            items,
            &host,
            protocol_name,
            now_unix_seconds(),
        )
        .await;
        strategy_picker::insert_reusable_candidates(&mut plan, candidates, &functions, &blobs);
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
    let include_catalog_sources =
        protocol != StrategyProtocol::Quic && picker_mode_uses_catalog_candidates(custom_mode);
    let freeze_candidates = if include_catalog_sources {
        load_confirmed_freeze_candidates(&state.data_dir.join("freeze-diagnostic-operations")).await
    } else {
        Vec::new()
    };
    if plan.candidates.is_empty() && freeze_candidates.is_empty() {
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
        hosts: Vec::new(),
        podkop_hosts: detect_podkop_hosts(std::slice::from_ref(&host)).await,
        multi_site_results: Vec::new(),
        multi_site_baseline: Vec::new(),
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
        if quic_baseline_is_already_reachable(protocol, baseline_ok) {
            if let Ok(mut current) = shared.lock() {
                current.state = "site_unblocked".into();
                current.stage = "baseline".into();
                current.current = None;
                current.done = 0;
                current.total = 0;
                current.message = format!(
                    "QUIC Initial уже получил прямой ответ ({}); стратегии не перебирались",
                    baseline_report.without_nfqws2.reason
                );
                current.updated_at = now_unix_seconds();
                let _ = write_strategy_pick_operation_sync(&operation_dir, &current);
            }
            task_state
                .strategy_pick_controls
                .lock()
                .await
                .remove(&task_id);
            return;
        }
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
                "Приоритетные варианты проверяются по очереди, полный каталог — безопасными пакетами с последовательным подтверждением; отмена доступна в любой момент".into()
            } else {
                "Проверяются реальные запросы через отдельный тестовый nfqws2".into()
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
        let result_state = shared.clone();
        let result_dir = operation_dir.clone();
        let mut publish_result = move |candidate: &strategy_picker::StrategyCandidateResult| {
            if let Ok(mut operation) = result_state.lock() {
                let existing = operation.results.iter().position(|row| {
                    row.candidate.name == candidate.candidate.name
                        && row.candidate.source == candidate.candidate.source
                        && row.candidate.steps == candidate.candidate.steps
                });
                if let Some(index) = existing {
                    operation.results[index] = candidate.clone();
                } else if operation.results.len() < 256 {
                    operation.results.push(candidate.clone());
                } else if candidate.confirmed
                    && let Some(index) = operation.results.iter().rposition(|row| !row.confirmed)
                {
                    operation.results[index] = candidate.clone();
                } else {
                    return;
                }
                operation.updated_at = now_unix_seconds();
                let _ = write_strategy_pick_operation_sync(&result_dir, &operation);
            }
        };
        let mut plan_result = if strategy_uses_full_batch_runner(protocol, mode) {
            strategy_picker::run_full_candidates_with_batches_and_results(
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
                &mut publish_result,
            )
            .await
        } else {
            strategy_picker::run_candidates_with_compaction_and_results(
                plan,
                control.clone(),
                task_repeats,
                stop_after_first_confirmed,
                false,
                &probe_candidate,
                update_progress,
                &mut publish_result,
            )
            .await
        };
        let already_confirmed = plan_result.results.iter().any(|result| result.confirmed);
        if protocol != StrategyProtocol::Quic
            && !custom_mode
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
            operation.message = plan_result.message;
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

fn quic_baseline_is_already_reachable(
    protocol: StrategyProtocol,
    baseline_ok: Option<bool>,
) -> bool {
    protocol == StrategyProtocol::Quic && baseline_ok == Some(true)
}

fn strategy_uses_full_batch_runner(protocol: StrategyProtocol, mode: Option<StrategyMode>) -> bool {
    protocol != StrategyProtocol::Quic && mode == Some(StrategyMode::Full)
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

async fn clear_strategy_pick_history(State(state): State<Arc<AppState>>) -> Response {
    let directory = state.data_dir.join("strategy-pick-operations");
    let mut entries = match tokio::fs::read_dir(&directory).await {
        Ok(entries) => entries,
        Err(io_error) if io_error.kind() == std::io::ErrorKind::NotFound => {
            return Json(serde_json::json!({ "removed": 0 })).into_response();
        }
        Err(io_error) => return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string()),
    };
    let mut removed = 0usize;
    loop {
        let entry = match entries.next_entry().await {
            Ok(Some(entry)) => entry,
            Ok(None) => break,
            Err(io_error) => return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string()),
        };
        let path = entry.path();
        if path.extension().and_then(|extension| extension.to_str()) != Some("json") {
            continue;
        }
        let Some(id) = path.file_stem().and_then(|stem| stem.to_str()) else {
            continue;
        };
        let operation = match read_strategy_pick_operation(&directory, id).await {
            Ok(Some(operation)) => operation,
            Ok(None) => continue,
            Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
        };
        if matches!(
            operation.state.as_str(),
            "complete" | "no_match" | "site_unblocked" | "cancelled" | "failed"
        ) {
            if let Err(io_error) = tokio::fs::remove_file(path).await {
                return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string());
            }
            removed += 1;
        }
    }
    Json(serde_json::json!({ "removed": removed })).into_response()
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

fn freeze_in_config_unknown(reason: &str) -> serde_json::Value {
    serde_json::json!({ "state": "unknown", "profiles": [], "route": null, "why": reason })
}

fn freeze_in_config_status(
    host: &str,
    address: Option<std::net::Ipv4Addr>,
    found_step: &str,
    profiles: &[nfqws2_ui::runtime_profiles::RuntimeProfile],
    list_contents: &HashMap<String, String>,
) -> serde_json::Value {
    let Some(found_step) = freeze_profile_join::normalize_strategy_step(found_step) else {
        return freeze_in_config_unknown(
            "Найденный шаг нельзя безопасно сопоставить с конфигурацией",
        );
    };
    let matching = profiles
        .iter()
        .filter(|profile| {
            profile.steps.iter().any(|step| {
                freeze_profile_join::normalize_strategy_step(step).as_ref() == Some(&found_step)
            })
        })
        .collect::<Vec<_>>();
    let matching_profiles = matching
        .iter()
        .map(|profile| serde_json::json!({ "index": profile.index, "name": profile.name }))
        .collect::<Vec<_>>();
    if matching.is_empty() {
        return serde_json::json!({ "state": "not_present", "profiles": [], "route": null, "why": null });
    }
    let Some(address) = address else {
        return serde_json::json!({
            "state": "unknown", "profiles": matching_profiles, "route": null,
            "why": "У результата проверки нет подтверждённого IP-адреса",
        });
    };
    let route_index = match freeze_profile_join::first_matching_profile(
        profiles,
        host,
        IpAddr::V4(address),
        list_contents,
    ) {
        Ok(index) => index,
        Err(reason) => {
            return serde_json::json!({
                "state": "unknown", "profiles": matching_profiles, "route": null, "why": reason,
            });
        }
    };
    let route = route_index.and_then(|index| {
        profiles
            .iter()
            .find(|profile| profile.index == index)
            .map(|profile| serde_json::json!({ "index": profile.index, "name": profile.name }))
    });
    let working =
        route_index.is_some_and(|index| matching.iter().any(|profile| profile.index == index));
    let excluded_by = if working {
        None
    } else {
        matching.iter().find_map(|profile| {
            freeze_profile_join::profile_exclusion_reason(
                profile,
                host,
                IpAddr::V4(address),
                list_contents,
            )
            .map(|reason| (profile, reason))
        })
    };
    let why = if working {
        serde_json::Value::Null
    } else if let Some((profile, reason)) = excluded_by {
        serde_json::Value::String(format!(
            "Профиль #{} «{}» исключает адрес через {reason}",
            profile.index, profile.name
        ))
    } else if let Some(route) = &route {
        serde_json::Value::String(format!(
            "Адрес сейчас обрабатывает профиль #{} «{}», найденный шаг находится в другом профиле",
            route["index"].as_u64().unwrap_or_default(),
            route["name"].as_str().unwrap_or_default()
        ))
    } else {
        serde_json::Value::String("Ни один профиль сейчас не совпадает с этим адресом".into())
    };
    serde_json::json!({
        "state": if working { "working" } else { "elsewhere" },
        "profiles": matching_profiles,
        "route": route,
        "why": why,
    })
}

async fn read_freeze_scan_profile_context(
    config_path: &Path,
) -> Result<
    (
        Vec<nfqws2_ui::runtime_profiles::RuntimeProfile>,
        HashMap<String, String>,
    ),
    String,
> {
    const MAX_CONTEXT_LISTS: usize = 128;
    const MAX_CONTEXT_LIST_BYTES: usize = 4 * 1024 * 1024;
    let mut bytes = Vec::new();
    tokio::fs::File::open(config_path)
        .await
        .map_err(|error| format!("не удалось прочитать текущую конфигурацию: {error}"))?
        .take((MAX_CONFIG_BYTES + 1) as u64)
        .read_to_end(&mut bytes)
        .await
        .map_err(|error| format!("не удалось прочитать текущую конфигурацию: {error}"))?;
    if bytes.len() > MAX_CONFIG_BYTES {
        return Err("текущая конфигурация превышает лимит 512 КиБ".into());
    }
    let source = std::str::from_utf8(&bytes)
        .map_err(|_| "текущая конфигурация не является UTF-8 текстом".to_owned())?;
    let parsed = nfqws2_ui::runtime_profiles::from_config(source, &revision(&bytes))
        .map_err(|message| format!("не удалось разобрать профили: {message}"))?;
    let lists_directory = config_path
        .parent()
        .map(|parent| parent.join("lists"))
        .ok_or_else(|| "не удалось определить каталог списков".to_owned())?;
    let mut list_contents = HashMap::new();
    let mut total_bytes = 0_usize;
    for profile in &parsed.profiles {
        for reference in freeze_profile_join::profile_list_references(&profile.args)
            .map_err(|message| format!("не удалось разобрать списки профиля: {message}"))?
        {
            if list_contents.contains_key(&reference) {
                continue;
            }
            if list_contents.len() >= MAX_CONTEXT_LISTS {
                return Err("слишком много списков для безопасной проверки маршрута".into());
            }
            let path = resolve_foreign_list_path(&reference, &lists_directory)?;
            let mut content = Vec::new();
            let file = tokio::fs::File::open(path)
                .await
                .map_err(|error| format!("список профиля недоступен: {error}"))?;
            file.take((MAX_LIST_BYTES + 1) as u64)
                .read_to_end(&mut content)
                .await
                .map_err(|error| format!("не удалось прочитать список профиля: {error}"))?;
            if content.len() > MAX_LIST_BYTES {
                return Err("список профиля превышает лимит 512 КиБ".into());
            }
            total_bytes = total_bytes.saturating_add(content.len());
            if total_bytes > MAX_CONTEXT_LIST_BYTES {
                return Err("суммарный размер списков для проверки превышает 4 МиБ".into());
            }
            let content = String::from_utf8(content)
                .map_err(|_| "список профиля не является UTF-8 текстом".to_owned())?;
            list_contents.insert(reference, content);
        }
    }
    Ok((parsed.profiles, list_contents))
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
        let probe_short_uploads = request.probe_short_uploads;
        let result = freeze_diagnostic::scan_network_baselines(
            &request.extra_hosts,
            probe_short_uploads,
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
            if operation
                .results
                .iter()
                .any(|result| result.found_strategy.is_some())
            {
                let context = match config_path_for_state(&state) {
                    Some(path) => read_freeze_scan_profile_context(&path).await,
                    None => Err("текущая конфигурация nfqws2 не найдена".into()),
                };
                let mut response = match serde_json::to_value(&operation) {
                    Ok(value) => value,
                    Err(message) => {
                        return error(StatusCode::INTERNAL_SERVER_ERROR, message.to_string());
                    }
                };
                if let Some(results) = response["results"].as_array_mut() {
                    for (index, result) in operation.results.iter().enumerate() {
                        let Some(found) = result.found_strategy.as_ref() else {
                            continue;
                        };
                        let context_status = match &context {
                            Ok((profiles, lists)) => freeze_in_config_status(
                                &result.host,
                                result.address,
                                &found.step,
                                profiles,
                                lists,
                            ),
                            Err(reason) => freeze_in_config_unknown(reason),
                        };
                        if let Some(value) = results.get_mut(index) {
                            value["inConf"] = context_status;
                        }
                    }
                }
                Json(response).into_response()
            } else {
                Json(operation).into_response()
            }
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

async fn format_config(Json(request): Json<ConfigValidateRequest>) -> Response {
    if request.content.len() > MAX_CONFIG_BYTES {
        return error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Конфигурация превышает лимит 512 КБ",
        );
    }
    match format_nfqws_config(&request.content) {
        Ok(content) => Json(serde_json::json!({ "content": content })).into_response(),
        Err(message) => error(StatusCode::UNPROCESSABLE_ENTITY, message),
    }
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
    let config_bytes = std::fs::read(&config).ok();
    let config_source = config_bytes
        .as_deref()
        .and_then(|bytes| std::str::from_utf8(bytes).ok());
    let active_args = active_process_arguments();
    let profile_inventory = config_bytes.as_deref().and_then(|bytes| {
        let content = std::str::from_utf8(bytes).ok()?;
        nfqws2_ui::runtime_profiles::from_config(content, &revision(bytes)).ok()
    });
    let managed = ManagedAsnListStore::load(&state.data_dir);
    let files = list_ids_for_config(&config)
        .into_iter()
        .filter_map(|id| {
            let path = list_file_path(&config, &id)?;
            let validation_required = config_source.is_none_or(|source| {
                list_requires_nfqws_validation(&config, &path, &id, source, active_args.as_deref())
            });
            let managed_asn = managed
                .entries
                .iter()
                .find(|entry| entry.list_id == id)
                .map(|entry| ManagedAsnListSummary {
                    asn: entry.asn,
                    prefix_count: entry.prefix_count,
                    last_attempt_date: entry.last_attempt_date.clone(),
                    last_success_date: entry.last_success_date.clone(),
                    last_error: entry.last_error.clone(),
                    paused: entry.paused,
                    pending: entry.pending_update.is_some(),
                });
            Some(ListSummary {
                id,
                path: path.display().to_string(),
                readable: std::fs::File::open(&path).is_ok(),
                writable: std::fs::OpenOptions::new().append(true).open(&path).is_ok(),
                managed_asn,
                usage_known: profile_inventory.is_some(),
                used_by: profile_inventory
                    .as_ref()
                    .map(|inventory| list_profile_usage(&config, &path, &inventory.profiles))
                    .unwrap_or_default(),
                validation_required,
            })
        })
        .collect();
    Json(ListsResponse { files }).into_response()
}

async fn get_package_leftovers(State(state): State<Arc<AppState>>) -> Response {
    let Some(config) = config_path_for_state(&state) else {
        return error(StatusCode::NOT_FOUND, "Конфигурация nfqws2 не найдена");
    };
    let Some(config_dir) = config.parent() else {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Не удалось определить каталог конфигурации",
        );
    };
    let lists_dir = config_dir.join("lists");
    match package_leftovers::list_package_leftovers(&config, &lists_dir) {
        Ok(items) => {
            let total_bytes = items.iter().map(|item| item.size_bytes).sum();
            Json(PackageLeftoversResponse {
                items,
                total_bytes,
                deleted: None,
            })
            .into_response()
        }
        Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

async fn delete_package_leftovers(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PackageLeftoversDeleteRequest>,
) -> Response {
    if request.names.is_empty() || request.names.len() > 100 {
        return error(
            StatusCode::BAD_REQUEST,
            "Выберите от 1 до 100 файлов для удаления",
        );
    }
    let Some(config) = config_path_for_state(&state) else {
        return error(StatusCode::NOT_FOUND, "Конфигурация nfqws2 не найдена");
    };
    let Some(config_dir) = config.parent() else {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Не удалось определить каталог конфигурации",
        );
    };
    let lists_dir = config_dir.join("lists");
    match package_leftovers::delete_package_leftovers(&config, &lists_dir, &request.names) {
        Ok(deleted) => match package_leftovers::list_package_leftovers(&config, &lists_dir) {
            Ok(items) => {
                let total_bytes = items.iter().map(|item| item.size_bytes).sum();
                Json(PackageLeftoversResponse {
                    items,
                    total_bytes,
                    deleted: Some(deleted),
                })
                .into_response()
            }
            Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
        },
        Err(message) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
    }
}

async fn compare_package_leftover(
    State(state): State<Arc<AppState>>,
    Json(request): Json<PackageLeftoverCompareRequest>,
) -> Response {
    let Some(config) = config_path_for_state(&state) else {
        return error(StatusCode::NOT_FOUND, "Конфигурация nfqws2 не найдена");
    };
    let Some(config_dir) = config.parent() else {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Не удалось определить каталог конфигурации",
        );
    };
    match package_leftovers::compare_package_leftover(
        &config,
        &config_dir.join("lists"),
        &request.name,
    ) {
        Ok(comparison) => Json(comparison).into_response(),
        Err(message) => error(StatusCode::CONFLICT, message),
    }
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
    let _write_guard = state.config_write_lock.lock().await;
    let Some(path) = list_file_path(&config, &id) else {
        return error(
            StatusCode::NOT_FOUND,
            "Разрешённый существующий список не найден",
        );
    };
    let config_source = match tokio::fs::read_to_string(&config).await {
        Ok(content) => content,
        Err(read_error) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Не удалось проверить ссылки конфигурации: {read_error}"),
            );
        }
    };
    if list_requires_nfqws_validation(
        &config,
        &path,
        &id,
        &config_source,
        active_process_arguments().as_deref(),
    ) && let Err(message) = validate_list_content_with_kind(&id, &write.content, ip_list)
    {
        return error(StatusCode::BAD_REQUEST, message);
    }
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
    restart: F,
) -> Result<ConfigApplyResponse, String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<String, String>>,
{
    apply_config_transaction_with_verification(state, write, restart, || async { Ok(()) }).await
}

async fn apply_config_transaction_with_verification<F, Fut, V, VFut>(
    state: &AppState,
    write: ConfigWrite,
    restart: F,
    verify: V,
) -> Result<ConfigApplyResponse, String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<String, String>>,
    V: FnMut() -> VFut,
    VFut: std::future::Future<Output = Result<(), String>>,
{
    let _write_guard = state.config_write_lock.lock().await;
    apply_config_transaction_with_verification_locked(state, write, restart, verify).await
}

async fn apply_config_transaction_with_verification_locked<F, Fut, V, VFut>(
    state: &AppState,
    write: ConfigWrite,
    mut restart: F,
    mut verify: V,
) -> Result<ConfigApplyResponse, String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<String, String>>,
    V: FnMut() -> VFut,
    VFut: std::future::Future<Output = Result<(), String>>,
{
    if write.content.len() > MAX_CONFIG_BYTES || write.content.as_bytes().contains(&0) {
        return Err("Конфигурация превышает лимит или содержит запрещённый NUL-байт".into());
    }
    let path = config_path_for_state(state)
        .ok_or_else(|| "Разрешённый конфигурационный файл не найден".to_string())?;
    let original = tokio::fs::read(&path)
        .await
        .map_err(|error| format!("Не удалось прочитать конфигурацию: {error}"))?;
    if revision(&original) != write.revision {
        return Err("Файл изменился после загрузки; обновите страницу и проверьте diff".into());
    }
    let candidate_revision = revision(write.content.as_bytes());
    let journal_path = pending_config_apply_path(&state.data_dir);
    if tokio::fs::try_exists(&journal_path)
        .await
        .map_err(|error| format!("Не удалось проверить журнал применения: {error}"))?
    {
        return Err(
            "Предыдущее применение конфига требует восстановления; перезапустите панель".into(),
        );
    }
    write_pending_config_apply(
        &state.data_dir,
        &PendingConfigApplyJournal {
            config_path: path.to_string_lossy().into_owned(),
            original_content: original.clone(),
            candidate_revision: candidate_revision.clone(),
        },
    )
    .await?;
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
        return match rollback {
            Ok(()) => {
                clear_pending_config_apply(&state.data_dir).await?;
                Err(format!(
                    "Не удалось записать новую конфигурацию: {write_error}. Исходная версия восстановлена."
                ))
            }
            Err(rollback_error) => Err(format!(
                "Не удалось записать новую конфигурацию: {write_error}. Откат тоже не подтверждён: {rollback_error}"
            )),
        };
    }
    let saved = match tokio::fs::read(&path).await {
        Ok(saved) => saved,
        Err(error) => {
            let rollback =
                restore_config_content(&state.data_dir, &path, &original, &candidate_revision)
                    .await;
            return match rollback {
                Ok(()) => {
                    clear_pending_config_apply(&state.data_dir).await?;
                    Err(format!(
                        "Конфиг записан, но повторное чтение не удалось: {error}. Исходная версия восстановлена."
                    ))
                }
                Err(rollback_error) => Err(format!(
                    "Конфиг записан, повторное чтение не удалось: {error}. Откат тоже не подтверждён: {rollback_error}"
                )),
            };
        }
    };
    if saved != write.content.as_bytes() {
        let rollback =
            restore_config_content(&state.data_dir, &path, &original, &candidate_revision).await;
        return match rollback {
            Ok(()) => {
                clear_pending_config_apply(&state.data_dir).await?;
                Err("Повторное чтение конфига не совпало с отправленным содержимым; исходная версия восстановлена.".into())
            }
            Err(error) => Err(format!(
                "Повторное чтение конфига не совпало; откат не подтверждён: {error}"
            )),
        };
    }
    let apply_result = match restart().await {
        Ok(message) => verify()
            .await
            .map(|()| message)
            .map_err(|error| format!("Проверка после перезапуска не пройдена: {error}")),
        Err(error) => Err(error),
    };
    match apply_result {
        Ok(message) => {
            let response = ConfigApplyResponse {
                config: ConfigResponse {
                    file: path.display().to_string(),
                    revision: revision(&saved),
                    content: String::from_utf8_lossy(&saved).into_owned(),
                },
                message,
            };
            clear_pending_config_apply(&state.data_dir).await?;
            Ok(response)
        }
        Err(apply_error) => {
            if let Err(rollback_error) =
                restore_config_content(&state.data_dir, &path, &original, &candidate_revision).await
            {
                return Err(format!(
                    "Применение не удалось: {apply_error}. Не удалось восстановить старую конфигурацию: {rollback_error}"
                ));
            }
            match restart().await {
                Ok(restore_message) => {
                    clear_pending_config_apply(&state.data_dir).await?;
                    Err(format!(
                        "Применение не удалось: {apply_error}. Исходный конфиг восстановлен и служба перезапущена: {restore_message}"
                    ))
                }
                Err(restore_error) => Err(format!(
                    "Применение не удалось: {apply_error}. Исходный конфиг восстановлен, но повторный запуск службы не подтверждён: {restore_error}"
                )),
            }
        }
    }
}

async fn cleanup_freeze_join_list(path: &Path, message: String) -> String {
    match tokio::fs::remove_file(path).await {
        Ok(()) => message,
        Err(cleanup_error) => {
            format!("{message}. Созданный ASN-список не удалось удалить: {cleanup_error}")
        }
    }
}

async fn cleanup_freeze_join_managed_list(
    path: &Path,
    data_dir: &Path,
    asn: u32,
    message: String,
) -> String {
    let mut message = cleanup_freeze_join_list(path, message).await;
    let mut store = ManagedAsnListStore::load(data_dir);
    store.remove(asn);
    if let Err(error) = store.save(data_dir) {
        if !message.is_empty() {
            message.push_str(". ");
        }
        message.push_str(&format!(
            "Не удалось удалить регистрацию ASN-списка: {error}"
        ));
    }
    message
}

async fn apply_freeze_join_transaction<F, Fut, V, VFut>(
    state: &AppState,
    write: ConfigWrite,
    list_path: &Path,
    data_dir: &Path,
    asn: u32,
    restart: F,
    verify: V,
) -> Result<ConfigApplyResponse, String>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<String, String>>,
    V: FnMut() -> VFut,
    VFut: std::future::Future<Output = Result<(), String>>,
{
    match apply_config_transaction_with_verification(state, write, restart, verify).await {
        Ok(applied) => Ok(applied),
        Err(message) => {
            Err(cleanup_freeze_join_managed_list(list_path, data_dir, asn, message).await)
        }
    }
}

fn strategy_apply_verification_hosts(target: &str, monitoring: &MonitoringStore) -> Vec<String> {
    let mut hosts = vec![target.to_owned()];
    for site in &monitoring.sites {
        if hosts.len() > 20 {
            break;
        }
        if site.host != target
            && site
                .results
                .first()
                .is_some_and(|result| result.status == "ok")
            && !hosts.contains(&site.host)
        {
            hosts.push(site.host.clone());
        }
    }
    hosts
}

fn strategy_apply_verification_hosts_for_protocol(
    target: &str,
    protocol: &str,
    monitoring: &MonitoringStore,
) -> Vec<String> {
    if protocol == "quic" {
        vec![target.to_owned()]
    } else {
        strategy_apply_verification_hosts(target, monitoring)
    }
}

fn config_apply_verification_hosts(monitoring: &MonitoringStore, now: u64) -> Vec<String> {
    monitoring
        .sites
        .iter()
        .filter(|site| {
            site.results.first().is_some_and(|result| {
                result.status == "ok"
                    && result.checked_at <= now
                    && now.saturating_sub(result.checked_at)
                        < CONFIG_APPLY_MONITOR_BASELINE_TTL.as_secs()
            })
        })
        .filter_map(|site| site_diagnostics::normalize_host(&site.host).ok())
        .fold(Vec::new(), |mut hosts, host| {
            if hosts.len() < 20 && !hosts.contains(&host) {
                hosts.push(host);
            }
            hosts
        })
}

async fn verify_strategy_hosts_with<F, Fut>(hosts: Vec<String>, probe: F) -> Result<(), String>
where
    F: Fn(String) -> Fut + Clone + Send + Sync + 'static,
    Fut: std::future::Future<Output = Result<(), String>> + Send + 'static,
{
    let mut tasks = tokio::task::JoinSet::new();
    let network_slots = Arc::new(tokio::sync::Semaphore::new(3));
    for host in hosts {
        let probe = probe.clone();
        let network_slots = network_slots.clone();
        tasks.spawn(async move {
            let first = {
                let _permit = network_slots
                    .clone()
                    .acquire_owned()
                    .await
                    .map_err(|error| error.to_string())?;
                probe(host.clone()).await
            };
            if first.is_ok() {
                return Ok::<(), String>(());
            }
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
            let _permit = network_slots
                .acquire_owned()
                .await
                .map_err(|error| error.to_string())?;
            probe(host.clone())
                .await
                .map_err(|message| format!("{host}: {message}"))
        });
    }

    let mut failures = Vec::new();
    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(())) => {}
            Ok(Err(message)) => failures.push(message),
            Err(error) => failures.push(format!("проверка завершилась с ошибкой: {error}")),
        }
    }
    if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
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
    let config_bytes = match tokio::fs::read(&config).await {
        Ok(bytes) => bytes,
        Err(io_error) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Не удалось проверить связи списка с профилями: {io_error}"),
            );
        }
    };
    let config_source = match std::str::from_utf8(&config_bytes) {
        Ok(source) => source,
        Err(_) => {
            return error(
                StatusCode::CONFLICT,
                "Не удалось безопасно разобрать конфигурацию; список не удалён",
            );
        }
    };
    let profiles = match nfqws2_ui::runtime_profiles::from_config(
        config_source,
        &revision(&config_bytes),
    ) {
        Ok(profiles) => profiles,
        Err(_) => {
            return error(
                StatusCode::CONFLICT,
                "Не удалось безопасно определить использование списка в профилях; список не удалён",
            );
        }
    };
    let usages = list_profile_usage(&config, &path, &profiles.profiles);
    if !usages.is_empty() {
        let names = usages
            .iter()
            .map(|usage| format!("#{} {}", usage.profile_index, usage.profile_name))
            .collect::<HashSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(", ");
        return error(
            StatusCode::CONFLICT,
            format!("Список используется профилями {names}. Сначала отключите его от них."),
        );
    }
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
        Ok(()) => {
            if let Err(message) = change_history::scan(
                &state.data_dir,
                &config,
                "интерфейс",
                &format!("Удаление списка {id}"),
            ) {
                tracing::warn!("Список удалён, но журнал изменений не обновлён: {message}");
            }
            Json(MessageResponse {
                message: format!("Пользовательский список {id} удалён"),
            })
            .into_response()
        }
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

fn pending_config_apply_path(data_dir: &Path) -> PathBuf {
    data_dir.join("pending-config-apply.json")
}

async fn write_pending_config_apply(
    data_dir: &Path,
    journal: &PendingConfigApplyJournal,
) -> Result<(), String> {
    tokio::fs::create_dir_all(data_dir)
        .await
        .map_err(|error| format!("не удалось подготовить журнал применения: {error}"))?;
    let path = pending_config_apply_path(data_dir);
    let temporary = path.with_extension(format!(
        "json.{}.{}.tmp",
        std::process::id(),
        uuid::Uuid::new_v4()
    ));
    let bytes = serde_json::to_vec(journal)
        .map_err(|error| format!("не удалось сериализовать журнал применения: {error}"))?;
    #[cfg(unix)]
    {
        let mut file = tokio::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(&temporary)
            .await
            .map_err(|error| format!("не удалось создать журнал применения: {error}"))?;
        file.write_all(&bytes)
            .await
            .map_err(|error| format!("не удалось записать журнал применения: {error}"))?;
        file.sync_all()
            .await
            .map_err(|error| format!("не удалось сохранить журнал применения на диск: {error}"))?;
    }
    #[cfg(not(unix))]
    tokio::fs::write(&temporary, bytes)
        .await
        .map_err(|error| format!("не удалось записать журнал применения: {error}"))?;
    tokio::fs::rename(&temporary, &path)
        .await
        .map_err(|error| format!("не удалось сохранить журнал применения: {error}"))
}

async fn clear_pending_config_apply(data_dir: &Path) -> Result<(), String> {
    match tokio::fs::remove_file(pending_config_apply_path(data_dir)).await {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(format!("не удалось удалить журнал применения: {error}")),
    }
}

async fn recover_pending_config_apply_with<F, Fut>(
    state: &AppState,
    restart: F,
) -> Result<(), String>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = Result<String, String>>,
{
    let path = pending_config_apply_path(&state.data_dir);
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(format!("не удалось прочитать журнал применения: {error}")),
    };
    let journal: PendingConfigApplyJournal = serde_json::from_slice(&bytes)
        .map_err(|error| format!("журнал применения повреждён: {error}"))?;
    let config = config_path_for_state(state).ok_or_else(|| {
        "разрешённый конфигурационный файл не найден при восстановлении".to_string()
    })?;
    if journal.config_path != config.to_string_lossy() {
        return Err("путь в журнале применения не совпадает с разрешённым конфигом".into());
    }
    let current = tokio::fs::read(&config)
        .await
        .map_err(|error| format!("не удалось прочитать конфиг для восстановления: {error}"))?;
    if revision(&current) == journal.candidate_revision {
        restore_config_content(
            &state.data_dir,
            &config,
            &journal.original_content,
            &journal.candidate_revision,
        )
        .await?;
    } else if current != journal.original_content {
        return Err(
            "конфиг отличается и от исходного, и от проверяемого; восстановление остановлено"
                .into(),
        );
    }
    restart().await.map_err(|error| {
        format!("не удалось перезапустить службу после восстановления: {error}")
    })?;
    clear_pending_config_apply(&state.data_dir).await
}

async fn recover_pending_config_apply(state: &AppState) -> Result<(), String> {
    recover_pending_config_apply_with(state, || async {
        service_action(ServiceAction::Restart).await
    })
    .await
}

async fn apply_config(
    State(state): State<Arc<AppState>>,
    Json(write): Json<ConfigWrite>,
) -> Response {
    let _monitoring_guard = state.monitoring_lock.lock().await;
    let monitoring = match load_monitoring_store(&state.data_dir).await {
        Ok(monitoring) => monitoring,
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    };
    let hosts = config_apply_verification_hosts(&monitoring, now_unix_seconds());
    let policy_route = resolve_monitoring_policy_route(&state).await;
    let result = apply_config_transaction_with_verification(
        &state,
        write,
        || async { service_action(ServiceAction::Restart).await },
        || {
            let hosts = hosts.clone();
            let policy_route = policy_route.clone();
            async move {
                verify_strategy_hosts_with(hosts, move |host| {
                    let policy_route = policy_route.clone();
                    async move {
                        let report = check_monitoring_host(host, policy_route).await;
                        if report.current.status == "ok" {
                            Ok(())
                        } else {
                            Err(report.summary)
                        }
                    }
                })
                .await
            }
        },
    )
    .await;
    match result {
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
    let history_config = snapshot_config.clone();
    if let Some(config) = &history_config {
        let (source, note) = external_change_source(config);
        change_history::scan(data_dir, config, source, note).map_err(|message| {
            BackupWriteError::Failed(format!(
                "Не удалось сверить историю перед изменением: {message}"
            ))
        })?;
    }
    if let Some(config) =
        snapshot_config.filter(|_| snapshots::load_settings(data_dir).auto_enabled)
    {
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
    if let Some(config) = history_config
        && let Err(error) = change_history::scan(
            data_dir,
            &config,
            if backup_name.starts_with("history-") {
                "откат"
            } else {
                "интерфейс"
            },
            if backup_name.starts_with("history-") {
                "Откат записи истории"
            } else {
                backup_name
            },
        )
    {
        tracing::warn!("Файл записан, но не удалось обновить журнал изменений: {error}");
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

async fn get_change_history(State(state): State<Arc<AppState>>) -> Response {
    let Some(config) = config_path_for_state(&state) else {
        return error(StatusCode::NOT_FOUND, "Конфигурация nfqws2 не найдена");
    };
    let _guard = state.config_write_lock.lock().await;
    let store = snapshots::load_store(&state.data_dir);
    let last_snapshot = store
        .items
        .last()
        .map(|item| item.id.as_str())
        .unwrap_or_default();
    let (source, note) = external_change_source(&config);
    if let Err(message) = change_history::scan(&state.data_dir, &config, source, note) {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    Json(match change_history::list(&state.data_dir, 300) {
        Ok(history) => serde_json::json!({ "items": history.items, "initialAt": history.initial_at, "canRestoreInitial": history.can_restore_initial, "canUndoLast": history.can_undo_last, "lastSnapshot": last_snapshot }),
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    })
    .into_response()
}

async fn get_community_strategies(
    State(state): State<Arc<AppState>>,
    Query(query): Query<CommunityQuery>,
) -> Response {
    let mut provider = community_strategies::read_provider_settings(&state.data_dir);
    let mut detection_error = None;
    if community_strategies::provider_detection_needed(
        provider.asn,
        query.detect,
        query.auto_detect,
    ) {
        match community_strategies::detect_provider().await {
            Ok(detected) => {
                provider = detected;
                if let Err(message) =
                    community_strategies::write_provider_settings(&state.data_dir, &provider)
                {
                    return error(StatusCode::INTERNAL_SERVER_ERROR, message);
                }
            }
            Err(message) => detection_error = Some(message),
        }
    }
    let (provider_result, index) = if let Some(asn) = provider.asn {
        let (provider_result, index) = tokio::join!(
            community_strategies::load(&state.data_dir, asn, query.force),
            community_strategies::load_index(&state.data_dir, query.force),
        );
        (Some(provider_result), index)
    } else {
        (
            None,
            community_strategies::load_index(&state.data_dir, query.force).await,
        )
    };
    let (items, fetched_at, error_message) = match provider_result {
        Some(Ok((items, fetched_at, load_error))) => (items, Some(fetched_at), load_error),
        Some(Err(message)) => (Vec::new(), None, Some(message)),
        None => (Vec::new(), None, None),
    };
    let now = now_unix_seconds();
    let mut items: Vec<_> = items
        .into_iter()
        .map(|item| {
            let skipped = item.fails >= 2 && item.fails > item.reports;
            let fresh = community_strategies::is_fresh(&item, now);
            let name = item
                .steps
                .iter()
                .filter_map(|step| step.strip_prefix("--lua-desync="))
                .map(|step| step.split(':').next().unwrap_or(step))
                .collect::<Vec<_>>()
                .join(" + ");
            CommunityViewItem {
                item,
                name,
                fresh,
                skipped,
            }
        })
        .collect();
    items.sort_by(|left, right| {
        (
            left.skipped,
            !left.fresh,
            std::cmp::Reverse(left.item.reports.saturating_sub(left.item.fails)),
            std::cmp::Reverse(&left.item.last),
        )
            .cmp(&(
                right.skipped,
                !right.fresh,
                std::cmp::Reverse(right.item.reports.saturating_sub(right.item.fails)),
                std::cmp::Reverse(&right.item.last),
            ))
    });
    Json(CommunityResponse {
        asn: provider.asn,
        provider: provider.name,
        fetched_at,
        error: detection_error.or(error_message),
        total: index.map(|entry| entry.0),
        providers: index.map(|entry| entry.1),
        items,
        repo: community_strategies::COMMUNITY_REPO.to_owned(),
        page: format!(
            "{}/blob/main/STRATEGIES.md",
            community_strategies::COMMUNITY_REPO
        ),
        mine: provider.asn.map(|asn| {
            format!(
                "{}/blob/main/providers/AS{asn}.md",
                community_strategies::COMMUNITY_REPO
            )
        }),
    })
    .into_response()
}

async fn get_community_target_asn(Query(query): Query<CommunityTargetAsnQuery>) -> Response {
    Json(serde_json::json!({
        "asn": community_strategies::target_asn_for_host(&query.host).await
    }))
    .into_response()
}

async fn set_community_provider(
    State(state): State<Arc<AppState>>,
    Json(request): Json<CommunityProviderWrite>,
) -> Response {
    let provider = ProviderSettings {
        asn: request.asn,
        name: request.name.trim().to_owned(),
    };
    if provider.asn.is_some_and(|asn| asn == 0) {
        return error(StatusCode::BAD_REQUEST, "Номер ASN должен быть больше нуля");
    }
    if let Err(message) = community_strategies::write_provider_settings(&state.data_dir, &provider)
    {
        return error(StatusCode::BAD_REQUEST, message);
    }
    Json(provider).into_response()
}

fn external_change_source(config: &Path) -> (&'static str, &'static str) {
    let parent = config.parent().unwrap_or_else(|| Path::new("/"));
    let updated = std::fs::metadata(config)
        .and_then(|metadata| metadata.modified())
        .ok();
    for candidate in ["nfqws2.conf-opkg", "nfqws2.conf.apk-new"] {
        if let (Some(changed), Ok(package)) = (
            updated,
            std::fs::metadata(parent.join(candidate)).and_then(|metadata| metadata.modified()),
        ) && changed
            .duration_since(package)
            .or_else(|_| package.duration_since(changed))
            .is_ok_and(|duration| duration.as_secs() <= 30)
        {
            return (
                "обновление пакета",
                "Обнаружено изменение рядом с шаблоном пакета",
            );
        }
    }
    (
        "вне интерфейса",
        "Изменение обнаружено при проверке истории",
    )
}

async fn get_change_history_diff(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    match change_history::diff(&state.data_dir, &id) {
        Ok(diff) => Json(diff).into_response(),
        Err(message) => error(StatusCode::NOT_FOUND, message),
    }
}

async fn revert_change_history(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    revert_change_history_id(&state, &id, "Откат изменения").await
}

async fn undo_last_change(State(state): State<Arc<AppState>>) -> Response {
    let Some(config) = config_path_for_state(&state).and_then(|path| path.canonicalize().ok())
    else {
        return error(StatusCode::NOT_FOUND, "Конфигурация nfqws2 не найдена");
    };
    let _write_guard = state.config_write_lock.lock().await;
    let (source, note) = external_change_source(&config);
    if let Err(message) = change_history::scan(&state.data_dir, &config, source, note) {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    let batch = match change_history::undoable_batch(&state.data_dir) {
        Ok(batch) => batch,
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    };
    let Some(mut batch) = batch else {
        return error(
            StatusCode::CONFLICT,
            "Нет изменения, которое можно отменить",
        );
    };
    let current_config_source = match tokio::fs::read_to_string(&config).await {
        Ok(content) => content,
        Err(read_error) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Не удалось проверить конфигурацию перед откатом: {read_error}"),
            );
        }
    };
    let mut target_config_source = None;
    for item in batch.iter().filter(|item| item.file == "nfqws2.conf") {
        let (entry, previous) = match change_history::version_before(&state.data_dir, &item.id) {
            Ok(value) => value,
            Err(message) => return error(StatusCode::NOT_FOUND, message),
        };
        if Path::new(&entry.path) == config {
            target_config_source = previous;
            break;
        }
    }
    let active_args = active_process_arguments();
    // Check the entire operation before changing anything, so an intervening
    // edit cannot leave a multi-file undo half-applied.
    for entry in &batch {
        let current = match tokio::fs::read(&entry.path).await {
            Ok(bytes) => Some(hex::encode(Sha256::digest(bytes))),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(io_error) => {
                return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string());
            }
        };
        if current != entry.to {
            return error(
                StatusCode::CONFLICT,
                "Файл изменился после этой операции; отмена остановлена. Обновите историю и проверьте diff.",
            );
        }
    }
    // Resolve and validate every historical version before writing any file.
    // A bad older list must not leave earlier files in the same batch reverted.
    for item in &batch {
        let (entry, previous) = match change_history::version_before(&state.data_dir, &item.id) {
            Ok(value) => value,
            Err(message) => return error(StatusCode::NOT_FOUND, message),
        };
        let path = PathBuf::from(&entry.path);
        if path == config {
            let Some(content) = previous else {
                return error(StatusCode::CONFLICT, "Нельзя удалить конфигурацию nfqws2");
            };
            let diagnostics = match validate_nfqws2_config(&content) {
                Ok(diagnostics) => diagnostics,
                Err(message) => return error(StatusCode::BAD_REQUEST, message),
            };
            if let Some(diagnostic) = diagnostics
                .iter()
                .find(|diagnostic| diagnostic.severity == DiagnosticSeverity::Error)
            {
                return error(
                    StatusCode::BAD_REQUEST,
                    format!(
                        "Предыдущая версия конфигурации не проходит проверку: {}",
                        diagnostic.message
                    ),
                );
            }
        } else {
            let valid_path = path.extension().and_then(|extension| extension.to_str())
                == Some("list")
                && path.parent().is_some_and(|parent| {
                    parent
                        == config
                            .parent()
                            .unwrap_or_else(|| Path::new("/"))
                            .join("lists")
                });
            if !valid_path {
                return error(
                    StatusCode::FORBIDDEN,
                    "Запись истории выходит за разрешённые файлы",
                );
            }
            if let Some(content) = previous {
                if content.len() > MAX_LIST_BYTES {
                    return error(
                        StatusCode::PAYLOAD_TOO_LARGE,
                        "Предыдущая версия списка превышает лимит 512 КБ",
                    );
                }
                let requires_validation = !LIST_IDS.contains(&entry.file.as_str())
                    || list_requires_nfqws_validation(
                        &config,
                        &path,
                        &entry.file,
                        &current_config_source,
                        active_args.as_deref(),
                    )
                    || target_config_source.as_deref().is_some_and(|target| {
                        list_requires_nfqws_validation(
                            &config,
                            &path,
                            &entry.file,
                            target,
                            active_args.as_deref(),
                        )
                    });
                if requires_validation {
                    let kind = entry.file.starts_with("ipset");
                    if let Err(message) =
                        validate_list_content_with_kind(&entry.file, &content, kind)
                    {
                        return error(
                            StatusCode::BAD_REQUEST,
                            format!("Предыдущая версия списка не проходит проверку: {message}"),
                        );
                    }
                }
            }
        }
    }
    // Restore lists first and the config last, so the single service restart
    // caused by a config rollback sees the complete restored operation.
    batch.sort_by_key(|entry| entry.file == "nfqws2.conf");
    let batch_id = batch[0].batch.clone();
    let files: Vec<_> = batch.iter().map(|entry| entry.file.clone()).collect();
    let snapshot = match snapshots::create_snapshot(
        &state.data_dir,
        &config,
        "перед восстановлением",
        &format!("страховочная копия перед отменой {batch_id}"),
        true,
    ) {
        Ok(snapshot) => snapshot,
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    };
    let mut config_reverted = false;
    for entry in &batch {
        let response =
            revert_change_history_id_inner(&state, &entry.id, "Отмена последнего изменения", true)
                .await;
        if !response.status().is_success() {
            let failure_status = response.status();
            let failure_body = axum::body::to_bytes(response.into_body(), 64 * 1024)
                .await
                .map(|bytes| String::from_utf8_lossy(&bytes).into_owned())
                .unwrap_or_else(|body_error| {
                    format!("Не удалось прочитать ошибку отката: {body_error}")
                });
            return restore_failed_undo_batch(
                &state,
                &config,
                &snapshot.id,
                config_reverted,
                failure_status,
                failure_body,
            )
            .await;
        }
        config_reverted |= entry.file == "nfqws2.conf";
    }
    let paths: Vec<_> = batch
        .iter()
        .map(|entry| PathBuf::from(&entry.path))
        .collect();
    if let Err(message) = change_history::mark_undo_of_batch(&state.data_dir, &paths, &batch_id) {
        return restore_failed_undo_batch(
            &state,
            &config,
            &snapshot.id,
            config_reverted,
            StatusCode::INTERNAL_SERVER_ERROR,
            message,
        )
        .await;
    }
    Json(serde_json::json!({
        "ok": true,
        "message": format!("Отменено изменение в {} {}", files.len(), if files.len() == 1 { "файле" } else { "файлах" }),
        "files": files,
    }))
    .into_response()
}

async fn restore_failed_undo_batch(
    state: &AppState,
    config: &Path,
    snapshot_id: &str,
    restart_config: bool,
    failure_status: StatusCode,
    failure_body: String,
) -> Response {
    let archive = match snapshots::archive_bytes(&state.data_dir, snapshot_id) {
        Ok(archive) => archive,
        Err(message) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("{failure_body}; не удалось прочитать страховочную копию: {message}"),
            );
        }
    };
    let restored = match snapshots::validate_and_restore(&state.data_dir, config, &archive) {
        Ok(files) => files,
        Err(message) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("{failure_body}; не удалось восстановить страховочную копию: {message}"),
            );
        }
    };
    if restart_config && let Err(message) = service_action(ServiceAction::Restart).await {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "{failure_body}; файлы восстановлены, но перезапуск службы не удался: {message}"
            ),
        );
    }
    if let Err(message) = change_history::scan(
        &state.data_dir,
        config,
        "откат",
        "восстановление после ошибки групповой отмены",
    ) {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!(
                "{failure_body}; файлы восстановлены ({}), но не удалось обновить историю: {message}",
                restored.len()
            ),
        );
    }
    error(failure_status, failure_body)
}

async fn revert_change_history_id(state: &AppState, id: &str, note: &str) -> Response {
    revert_change_history_id_inner(state, id, note, false).await
}

async fn revert_change_history_id_inner(
    state: &AppState,
    id: &str,
    note: &str,
    lock_held: bool,
) -> Response {
    let Some(config) = config_path_for_state(state) else {
        return error(StatusCode::NOT_FOUND, "Конфигурация nfqws2 не найдена");
    };
    let (entry, previous) = match change_history::version_before(&state.data_dir, id) {
        Ok(value) => value,
        Err(message) => return error(StatusCode::NOT_FOUND, message),
    };
    let config = match config.canonicalize() {
        Ok(path) => path,
        Err(io_error) => return error(StatusCode::NOT_FOUND, io_error.to_string()),
    };
    let path = PathBuf::from(&entry.path);
    let valid = path == config
        || (path.extension().and_then(|extension| extension.to_str()) == Some("list")
            && path.parent().is_some_and(|parent| {
                parent
                    == config
                        .parent()
                        .unwrap_or_else(|| Path::new("/"))
                        .join("lists")
            }));
    if !valid {
        return error(
            StatusCode::FORBIDDEN,
            "Запись истории выходит за разрешённые файлы",
        );
    }
    if path == config {
        let current = match tokio::fs::read(&path).await {
            Ok(bytes) => Some(bytes),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(io_error) => return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string()),
        };
        if current
            .as_deref()
            .map(|bytes| hex::encode(Sha256::digest(bytes)))
            != entry.to
        {
            return error(
                StatusCode::CONFLICT,
                "Файл изменился после этой записи; откат остановлен. Обновите историю и проверьте diff.",
            );
        }
        let Some(previous) = previous else {
            return error(StatusCode::CONFLICT, "Нельзя удалить конфигурацию nfqws2");
        };
        let write = ConfigWrite {
            content: previous,
            revision: revision(current.as_deref().unwrap_or_default()),
        };
        let result = if lock_held {
            apply_config_transaction_with_verification_locked(
                state,
                write,
                || async { service_action(ServiceAction::Restart).await },
                || async { Ok(()) },
            )
            .await
        } else {
            apply_config_transaction(state, write, || async {
                service_action(ServiceAction::Restart).await
            })
            .await
        };
        return match result {
            Ok(result) => {
                if let Err(message) =
                    change_history::mark_latest(&state.data_dir, &path, "откат", note)
                {
                    tracing::warn!(
                        "Конфигурация откатилась, но не удалось подписать запись истории: {message}"
                    );
                }
                Json(serde_json::json!({"ok": true, "message": result.message})).into_response()
            }
            Err(message) => error(StatusCode::BAD_GATEWAY, message),
        };
    }
    #[cfg(test)]
    if state.change_history_fail_path.lock().await.as_ref() == Some(&path) {
        return error(
            StatusCode::INTERNAL_SERVER_ERROR,
            "Тестовый сбой отката файла истории",
        );
    }
    let _guard = if lock_held {
        None
    } else {
        Some(state.config_write_lock.lock().await)
    };
    let current = match tokio::fs::read(&path).await {
        Ok(bytes) => Some(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
        Err(io_error) => return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string()),
    };
    if current
        .as_deref()
        .map(|bytes| hex::encode(Sha256::digest(bytes)))
        != entry.to
    {
        return error(
            StatusCode::CONFLICT,
            "Файл изменился после этой записи; откат остановлен. Обновите историю и проверьте diff.",
        );
    }
    if !lock_held
        && let Err(message) = snapshots::create_snapshot(
            &state.data_dir,
            &config,
            "перед восстановлением",
            &format!("перед откатом {id}"),
            true,
        )
    {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    match previous {
        Some(content) => {
            if content.len() > MAX_LIST_BYTES {
                return error(
                    StatusCode::PAYLOAD_TOO_LARGE,
                    "Предыдущая версия списка превышает лимит 512 КБ",
                );
            }
            let config_source = tokio::fs::read_to_string(&config).await.ok();
            let requires_validation = !LIST_IDS.contains(&entry.file.as_str())
                || config_source.as_deref().is_none_or(|source| {
                    list_requires_nfqws_validation(
                        &config,
                        &path,
                        &entry.file,
                        source,
                        active_process_arguments().as_deref(),
                    )
                });
            if requires_validation {
                let kind = entry.file.starts_with("ipset");
                if let Err(message) = validate_list_content_with_kind(&entry.file, &content, kind) {
                    return error(
                        StatusCode::BAD_REQUEST,
                        format!("Предыдущая версия списка не проходит проверку: {message}"),
                    );
                }
            }
            if let Err(message) = backup_and_write_named(
                &state.data_dir,
                &path,
                &format!("history-{}", id),
                MAX_LIST_BYTES,
                content.as_bytes(),
                current.as_deref().map(revision).as_deref(),
            )
            .await
            {
                return error(StatusCode::INTERNAL_SERVER_ERROR, message.to_string());
            }
        }
        None => {
            if let Err(io_error) = tokio::fs::remove_file(&path).await {
                return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string());
            }
        }
    }
    if let Err(message) = change_history::scan(&state.data_dir, &config, "откат", note) {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    Json(serde_json::json!({"ok": true, "message": format!("{} · {}", note, entry.file)}))
        .into_response()
}

async fn restore_initial_history(State(state): State<Arc<AppState>>) -> Response {
    let Some(config) = config_path_for_state(&state) else {
        return error(StatusCode::NOT_FOUND, "Конфигурация nfqws2 не найдена");
    };
    let config = match config.canonicalize() {
        Ok(path) => path,
        Err(io_error) => return error(StatusCode::NOT_FOUND, io_error.to_string()),
    };
    let _guard = state.config_write_lock.lock().await;
    let (source, note) = external_change_source(&config);
    if let Err(message) = change_history::scan(&state.data_dir, &config, source, note) {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    let archive = match change_history::initial_archive(&state.data_dir, &config) {
        Ok(archive) => archive,
        Err(message) => return error(StatusCode::CONFLICT, message),
    };
    let before_restore = match snapshots::create_snapshot(
        &state.data_dir,
        &config,
        "перед восстановлением",
        "перед возвратом исходного состояния истории",
        true,
    ) {
        Ok(snapshot) => snapshot,
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    };
    let before_restore_archive = match snapshots::archive_bytes(&state.data_dir, &before_restore.id)
    {
        Ok(archive) => archive,
        Err(message) => return error(StatusCode::INTERNAL_SERVER_ERROR, message),
    };
    let changed = match snapshots::validate_and_restore(&state.data_dir, &config, &archive) {
        Ok(changed) => changed,
        Err(message) => return error(StatusCode::BAD_REQUEST, message),
    };
    // Zemidala intentionally leaves lists created after history started: the
    // restored config stops referencing them, while user-owned files are kept.
    // Only files present in the baseline archive are restored above.
    let restart_message = if discover_system_paths()
        .is_some_and(|paths| paths.config.canonicalize().ok().as_ref() == Some(&config))
    {
        match service_action(ServiceAction::Restart).await {
            Ok(message) => message,
            Err(restart_error) => {
                let rollback = snapshots::validate_and_restore(
                    &state.data_dir,
                    &config,
                    &before_restore_archive,
                );
                let rollback_restart = service_action(ServiceAction::Restart).await;
                let detail = match (rollback, rollback_restart) {
                    (Ok(_), Ok(_)) => {
                        "Исходное состояние восстановлено; служба перезапущена.".to_owned()
                    }
                    (Err(restore_error), restart) => format!(
                        "Не удалось восстановить исходное состояние ({restore_error}); перезапуск службы: {}",
                        restart.err().unwrap_or_else(|| "не требовался".into())
                    ),
                    (Ok(_), Err(rollback_error)) => format!(
                        "Файлы исходного состояния восстановлены, но перезапуск отката не удался: {rollback_error}"
                    ),
                };
                return error(
                    StatusCode::BAD_GATEWAY,
                    format!("Перезапуск после восстановления не удался: {restart_error}. {detail}"),
                );
            }
        }
    } else {
        "Установленная служба не обнаружена".to_owned()
    };
    if let Err(message) = change_history::scan(
        &state.data_dir,
        &config,
        "откат",
        "Вернуть как при установке",
    ) {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    Json(serde_json::json!({"changed": changed, "message": format!("Исходное состояние восстановлено: {} файлов. {}", changed.len(), restart_message)})).into_response()
}

async fn delete_snapshot_api(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    match snapshots::delete_local_snapshot(&state.data_dir, &id) {
        Ok(()) => Json(serde_json::json!({ "deleted": true })).into_response(),
        Err(message) => error(StatusCode::CONFLICT, message),
    }
}

async fn get_profiles(State(state): State<Arc<AppState>>) -> Response {
    let Some(path) = config_path_for_state(&state) else {
        return error(StatusCode::NOT_FOUND, "Конфигурация nfqws2 не найдена");
    };
    match tokio::fs::read(&path).await {
        Ok(bytes) if bytes.len() <= MAX_CONFIG_BYTES => {
            let content = match std::str::from_utf8(&bytes) {
                Ok(content) => content,
                Err(_) => {
                    return error(
                        StatusCode::UNPROCESSABLE_ENTITY,
                        "Конфигурация содержит некорректный UTF-8; исправьте файл в raw-редакторе",
                    );
                }
            };
            match nfqws2_ui::runtime_profiles::from_config(content, &revision(&bytes)) {
                Ok(profiles) => Json(profiles).into_response(),
                Err(message) => error(StatusCode::UNPROCESSABLE_ENTITY, message),
            }
        }
        Ok(_) => error(
            StatusCode::PAYLOAD_TOO_LARGE,
            "Конфигурация превышает лимит 512 КиБ",
        ),
        Err(io_error) => error(
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Не удалось прочитать конфигурацию: {io_error}"),
        ),
    }
}

fn build_runtime_profile_settings_config(
    original: &str,
    request: RuntimeProfileSettingsWrite,
) -> Result<String, String> {
    if revision(original.as_bytes()) != request.revision {
        return Err(WRITE_REVISION_CONFLICT.into());
    }
    const ALLOWED: &[&str] = &[
        "NFQWS_ARGS_UDP",
        "NFQWS_ARGS_QUIC",
        "NFQWS_ARGS_IPSET",
        "NFQWS_ARGS",
        "NFQWS_EXTRA_ARGS",
    ];
    if request.variables.is_empty() || request.variables.len() > ALLOWED.len() {
        return Err("В запросе настроек профиля нет допустимых параметров".into());
    }
    let original_document = ConfigDocument::parse(original)?;
    nfqws2_ui::runtime_profiles::from_config(original, &request.revision)?;
    let mut proposed = original.to_owned();
    for (name, args) in request.variables {
        if !ALLOWED.contains(&name.as_str()) {
            return Err(format!(
                "Изменение группы {name} через редактор профиля запрещено"
            ));
        }
        if name == "NFQWS_EXTRA_ARGS"
            && original_document
                .referenced_mode("NFQWS_EXTRA_ARGS")
                .is_some()
        {
            return Err("NFQWS_EXTRA_ARGS ссылается на MODE_LIST, MODE_AUTO или MODE_ALL; измените режим в общих настройках, чтобы сохранить связь со списками".into());
        }
        if args.len() > 8192
            || args.iter().any(|arg| {
                arg.is_empty()
                    || arg.len() > 4096
                    || arg.split_whitespace().count() != 1
                    || arg.chars().any(|c| {
                        c.is_control()
                            || matches!(c, '\'' | '"' | '`' | '$' | '\\' | ';' | '&' | '|')
                    })
            })
        {
            return Err(format!("Группа {name} содержит недопустимый параметр"));
        }
        proposed = ConfigDocument::parse(&proposed)?.set_literal_all(&name, &args.join("\n"))?;
        if proposed.len() > MAX_CONFIG_BYTES {
            return Err("Изменённая конфигурация превышает лимит размера".into());
        }
    }
    let diagnostics = validate_nfqws2_config(&proposed)?;
    if let Some(diagnostic) = diagnostics
        .into_iter()
        .find(|item| item.severity == DiagnosticSeverity::Error)
    {
        return Err(format!("{}: {}", diagnostic.code, diagnostic.message));
    }
    Ok(proposed)
}

async fn save_runtime_profile_settings(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RuntimeProfileSettingsWrite>,
) -> Response {
    let Some(path) = config_path_for_state(&state) else {
        return error(StatusCode::NOT_FOUND, "Конфигурация nfqws2 не найдена");
    };
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(err) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Не удалось прочитать конфигурацию: {err}"),
            );
        }
    };
    let original = match std::str::from_utf8(&bytes) {
        Ok(value) => value,
        Err(_) => {
            return error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Конфигурация содержит некорректный UTF-8",
            );
        }
    };
    let proposed = match build_runtime_profile_settings_config(original, request) {
        Ok(value) => value,
        Err(message) if message == WRITE_REVISION_CONFLICT => {
            return error(
                StatusCode::CONFLICT,
                "Конфигурация изменилась; обновите список профилей перед сохранением",
            );
        }
        Err(message) => return error(StatusCode::UNPROCESSABLE_ENTITY, message),
    };
    let Some(paths) = discover_system_paths() else {
        return error(
            StatusCode::NOT_FOUND,
            "Установка nfqws2 не обнаружена; настройки профиля не сохранены",
        );
    };
    if let Err(message) = validate_profile_config_dry_run(&proposed, &paths).await {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("nfqws2 --dry-run отклонил конфигурацию; изменений нет: {message}"),
        );
    }
    match apply_config_transaction(
        &state,
        ConfigWrite {
            content: proposed,
            revision: revision(&bytes),
        },
        || async { service_action(ServiceAction::Restart).await },
    )
    .await
    {
        Ok(result) => Json(result).into_response(),
        Err(message) if message == WRITE_REVISION_CONFLICT => error(
            StatusCode::CONFLICT,
            "Конфигурация изменилась во время сохранения профиля",
        ),
        Err(message) => error(StatusCode::BAD_GATEWAY, message),
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct NfqwsCliOption {
    name: String,
    syntax: String,
    description: String,
    global: bool,
    takes_value: bool,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct NfqwsCliCatalog {
    options: Vec<NfqwsCliOption>,
    l7: Vec<String>,
    payloads: Vec<String>,
    full: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NfqwsCliCatalogCacheKey {
    binary: PathBuf,
    modified: Option<SystemTime>,
    length: u64,
}

#[derive(Debug, Clone)]
struct NfqwsCliCatalogCacheEntry {
    key: NfqwsCliCatalogCacheKey,
    catalog: NfqwsCliCatalog,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NfqwsLuaCatalogCacheKey {
    directory: PathBuf,
    files: Vec<(PathBuf, Option<SystemTime>, u64)>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct NfqwsLuaCatalog {
    functions: Vec<nfqws2_ui::runtime_profiles::LuaFunctionInfo>,
    standard_groups: Vec<nfqws2_ui::runtime_profiles::LuaStandardGroup>,
}

#[derive(Debug, Clone)]
struct NfqwsLuaCatalogCacheEntry {
    key: NfqwsLuaCatalogCacheKey,
    catalog: NfqwsLuaCatalog,
}

fn nfqws_lua_catalog_cache_key(directory: &Path) -> Option<NfqwsLuaCatalogCacheKey> {
    let directory = directory.canonicalize().ok()?;
    let mut files = std::fs::read_dir(&directory)
        .ok()?
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| {
            let path = entry.path();
            let extension = path.extension()?.to_str()?;
            if extension != "lua" && extension != "gz" {
                return None;
            }
            let metadata = std::fs::metadata(&path).ok()?;
            Some((path, metadata.modified().ok(), metadata.len()))
        })
        .collect::<Vec<_>>();
    files.sort_by(|left, right| left.0.cmp(&right.0));
    Some(NfqwsLuaCatalogCacheKey { directory, files })
}

fn reusable_nfqws_lua_catalog(
    cached: Option<&NfqwsLuaCatalogCacheEntry>,
    key: &NfqwsLuaCatalogCacheKey,
    force_refresh: bool,
) -> Option<NfqwsLuaCatalog> {
    if force_refresh {
        return None;
    }
    cached
        .filter(|entry| entry.key == *key)
        .map(|entry| entry.catalog.clone())
}

fn nfqws_cli_catalog_cache_key(binary: &Path) -> Option<NfqwsCliCatalogCacheKey> {
    let binary = binary.canonicalize().ok()?;
    let metadata = std::fs::metadata(&binary).ok()?;
    Some(NfqwsCliCatalogCacheKey {
        binary,
        modified: metadata.modified().ok(),
        length: metadata.len(),
    })
}

fn reusable_nfqws_cli_catalog(
    cached: Option<&NfqwsCliCatalogCacheEntry>,
    key: &NfqwsCliCatalogCacheKey,
    force_refresh: bool,
) -> Option<NfqwsCliCatalog> {
    if force_refresh {
        return None;
    }
    cached
        .filter(|entry| entry.key == *key && entry.catalog.full)
        .map(|entry| entry.catalog.clone())
}

fn parse_nfqws_cli_catalog(help: &str) -> NfqwsCliCatalog {
    let mut catalog = NfqwsCliCatalog::default();
    let mut section = "GLOBAL".to_owned();
    let mut logical_options = Vec::<(String, String)>::new();
    for raw_line in help.lines() {
        let line = strip_terminal_controls(raw_line);
        let trimmed = line.trim();
        if trimmed.ends_with(':')
            && trimmed
                .chars()
                .all(|ch| ch.is_ascii_uppercase() || ch == ' ' || ch == '-' || ch == ':')
        {
            section = trimmed.trim_end_matches(':').to_owned();
            continue;
        }
        if trimmed.starts_with("--") {
            logical_options.push((trimmed.to_owned(), section.clone()));
            continue;
        }
        // Some nfqws2 builds wrap help descriptions at the terminal width.
        // Join continuation lines so value lists (notably --payload/--filter-l7)
        // are complete even when the command inherits a narrow terminal.
        if !trimmed.is_empty()
            && let Some((previous, _)) = logical_options.last_mut()
        {
            previous.push(' ');
            previous.push_str(trimmed);
        }
    }
    for (line, section) in logical_options {
        let Some(option) = line.strip_prefix("--") else {
            continue;
        };
        let name_end = option
            .find(|ch: char| !(ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-'))
            .unwrap_or(option.len());
        if name_end == 0 {
            continue;
        }
        let name = &option[..name_end];
        let rest = &option[name_end..];
        let syntax_end = rest
            .find(|ch: char| ch.is_ascii_whitespace() || ch == ';')
            .unwrap_or(rest.len());
        let suffix = &rest[..syntax_end];
        let description = rest[syntax_end..]
            .trim()
            .trim_start_matches(';')
            .trim()
            .to_owned();
        let takes_value = !suffix.is_empty() && !suffix.starts_with('[');
        if let Some(values) = description
            .rsplit_once(':')
            .map(|(_, values)| values.trim())
        {
            let values = values
                .split_ascii_whitespace()
                .map(str::to_owned)
                .collect::<Vec<_>>();
            if name == "filter-l7" {
                catalog.l7 = values;
            } else if name == "payload" {
                catalog.payloads = values;
            }
        }
        catalog.options.push(NfqwsCliOption {
            name: name.to_owned(),
            syntax: format!("--{name}{suffix}"),
            global: matches!(section.as_str(), "GLOBAL" | "DESYNC ENGINE INIT")
                || description.contains("global parameter"),
            description,
            takes_value,
        });
    }
    const REQUIRED_OPTIONS: [&str; 7] = [
        "qnum",
        "blob",
        "lua-init",
        "new",
        "filter-tcp",
        "payload",
        "lua-desync",
    ];
    catalog.full = REQUIRED_OPTIONS.iter().all(|required| {
        catalog
            .options
            .iter()
            .any(|option| option.name == *required)
    });
    catalog
}

async fn run_nfqws_cli_help(binary: &Path, timeout: std::time::Duration) -> NfqwsCliCatalog {
    match tokio::time::timeout(
        timeout,
        tokio::process::Command::new(binary)
            .arg("--help")
            .kill_on_drop(true)
            .output(),
    )
    .await
    {
        Ok(Ok(output)) => parse_nfqws_cli_catalog(&format!(
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        )),
        _ => NfqwsCliCatalog::default(),
    }
}

async fn load_nfqws_cli_catalog(binary: &Path) -> NfqwsCliCatalog {
    let first = run_nfqws_cli_help(binary, std::time::Duration::from_secs(5)).await;
    if first.full {
        return first;
    }
    // Match Zemidala's fallback: --help may be truncated on a busy router,
    // so retry once with a longer deadline before exposing the catalog.
    let retry = run_nfqws_cli_help(binary, std::time::Duration::from_secs(15)).await;
    if retry.full || retry.options.len() >= first.options.len() {
        retry
    } else {
        first
    }
}

async fn load_cached_nfqws_cli_catalog(
    state: &AppState,
    binary: &Path,
    force_refresh: bool,
) -> NfqwsCliCatalog {
    let Some(key) = nfqws_cli_catalog_cache_key(binary) else {
        return load_nfqws_cli_catalog(binary).await;
    };
    let mut cache = state.nfqws_cli_catalog_cache.lock().await;
    if let Some(catalog) = reusable_nfqws_cli_catalog(cache.as_ref(), &key, force_refresh) {
        return catalog;
    }
    let catalog = load_nfqws_cli_catalog(binary).await;
    if catalog.full {
        *cache = Some(NfqwsCliCatalogCacheEntry {
            key,
            catalog: catalog.clone(),
        });
    } else if cache.as_ref().is_some_and(|entry| entry.key != key) {
        // Never keep a complete catalog for an older/replaced binary.
        *cache = None;
    }
    catalog
}

fn load_nfqws_lua_catalog(directory: &Path) -> NfqwsLuaCatalog {
    let Ok(entries) = std::fs::read_dir(directory) else {
        return NfqwsLuaCatalog {
            functions: Vec::new(),
            standard_groups: Vec::new(),
        };
    };
    let mut entries = entries.filter_map(Result::ok).collect::<Vec<_>>();
    entries.sort_by_key(|entry| entry.file_name());
    let mut catalog = std::collections::BTreeMap::new();
    let mut standard_groups = std::collections::BTreeMap::new();
    let mut total_bytes = 0usize;
    for entry in entries {
        if !entry.file_type().is_ok_and(|file_type| file_type.is_file()) {
            continue;
        }
        let path = entry.path();
        let Some(extension) = path.extension().and_then(|value| value.to_str()) else {
            continue;
        };
        if extension != "lua" && extension != "gz" {
            continue;
        }
        let Ok(bytes) = std::fs::read(&path) else {
            continue;
        };
        if bytes.len() > 4 * 1024 * 1024
            || total_bytes.saturating_add(bytes.len()) > 16 * 1024 * 1024
        {
            continue;
        }
        total_bytes += bytes.len();
        let decoded = if extension == "gz" {
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
        let source = String::from_utf8_lossy(&decoded);
        let file = path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .trim_end_matches(".gz")
            .to_string();
        for group in nfqws2_ui::runtime_profiles::parse_lua_standard_args(&source) {
            let entry = standard_groups
                .entry(group.name.clone())
                .or_insert_with(|| nfqws2_ui::runtime_profiles::LuaStandardGroup {
                    name: group.name,
                    arguments: Vec::new(),
                });
            for argument in group.arguments {
                if !entry.arguments.contains(&argument) {
                    entry.arguments.push(argument);
                }
            }
        }
        for function in nfqws2_ui::runtime_profiles::parse_lua_catalog(&file, &source) {
            catalog.entry(function.name.clone()).or_insert(function);
        }
    }
    NfqwsLuaCatalog {
        functions: catalog.into_values().collect(),
        standard_groups: standard_groups.into_values().collect(),
    }
}

async fn load_cached_nfqws_lua_catalog(
    state: &AppState,
    directory: &Path,
    force_refresh: bool,
) -> NfqwsLuaCatalog {
    let Some(key) = nfqws_lua_catalog_cache_key(directory) else {
        return load_nfqws_lua_catalog(directory);
    };
    let mut cache = state.nfqws_lua_catalog_cache.lock().await;
    if let Some(catalog) = reusable_nfqws_lua_catalog(cache.as_ref(), &key, force_refresh) {
        return catalog;
    }
    let catalog = load_nfqws_lua_catalog(directory);
    *cache = Some(NfqwsLuaCatalogCacheEntry {
        key,
        catalog: catalog.clone(),
    });
    catalog
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ProfileCatalogQuery {
    #[serde(default)]
    force_refresh: bool,
}

fn strip_terminal_controls(value: &str) -> String {
    let mut output = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(ch) = chars.next() {
        if ch == '\u{1b}' {
            if chars.next_if_eq(&'[').is_some() {
                for code in chars.by_ref() {
                    if ('@'..='~').contains(&code) {
                        break;
                    }
                }
            } else {
                let _ = chars.next();
            }
        } else if ch == '\t' {
            output.push(' ');
        } else if !ch.is_control() {
            output.push(ch);
        }
    }
    output
}

async fn get_profile_lua_catalog(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ProfileCatalogQuery>,
) -> Response {
    let Some(paths) = discover_system_paths() else {
        return error(StatusCode::NOT_FOUND, "Установка nfqws2 не обнаружена");
    };
    let cli_catalog =
        load_cached_nfqws_cli_catalog(&state, &paths.binary, query.force_refresh).await;
    let lua_catalog =
        load_cached_nfqws_lua_catalog(&state, &paths.lua_dir, query.force_refresh).await;
    Json(serde_json::json!({ "functions": lua_catalog.functions, "standardGroups": lua_catalog.standard_groups, "cli": cli_catalog })).into_response()
}

fn build_runtime_profiles_config(
    original: &str,
    request: RuntimeProfilesWrite,
) -> Result<String, String> {
    if revision(original.as_bytes()) != request.revision {
        return Err(WRITE_REVISION_CONFLICT.into());
    }
    let encoded = nfqws2_ui::runtime_profiles::encode_custom_profile_args(&request.profiles)?;
    nfqws2_ui::runtime_profiles::from_config(original, &request.revision)?;
    let document = ConfigDocument::parse(original)?;
    let proposed = document.set_literal_all("NFQWS_ARGS_CUSTOM", &encoded)?;
    let diagnostics = validate_nfqws2_config(&proposed)?;
    if let Some(diagnostic) = diagnostics
        .into_iter()
        .find(|item| item.severity == DiagnosticSeverity::Error)
    {
        return Err(format!("{}: {}", diagnostic.code, diagnostic.message));
    }
    Ok(proposed)
}

async fn save_runtime_profiles(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RuntimeProfilesWrite>,
) -> Response {
    let Some(path) = config_path_for_state(&state) else {
        return error(StatusCode::NOT_FOUND, "Конфигурация nfqws2 не найдена");
    };
    let bytes = match tokio::fs::read(&path).await {
        Ok(bytes) => bytes,
        Err(io_error) => {
            return error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Не удалось прочитать конфигурацию: {io_error}"),
            );
        }
    };
    let original = match std::str::from_utf8(&bytes) {
        Ok(original) => original,
        Err(_) => {
            return error(
                StatusCode::UNPROCESSABLE_ENTITY,
                "Конфигурация содержит некорректный UTF-8; сохранение профилей остановлено",
            );
        }
    };
    let proposed = match build_runtime_profiles_config(original, request) {
        Ok(content) => content,
        Err(message) if message == WRITE_REVISION_CONFLICT => {
            return error(
                StatusCode::CONFLICT,
                "Конфигурация изменилась; обновите список профилей перед сохранением",
            );
        }
        Err(message) => return error(StatusCode::UNPROCESSABLE_ENTITY, message),
    };
    let Some(paths) = discover_system_paths() else {
        return error(
            StatusCode::NOT_FOUND,
            "Установка nfqws2 не обнаружена; профили не сохранены",
        );
    };
    if let Err(message) = validate_profile_config_dry_run(&proposed, &paths).await {
        return error(
            StatusCode::UNPROCESSABLE_ENTITY,
            format!("nfqws2 --dry-run отклонил конфигурацию; изменений нет: {message}"),
        );
    }
    let current_revision = revision(&bytes);
    match apply_config_transaction(
        &state,
        ConfigWrite {
            content: proposed,
            revision: current_revision,
        },
        || async { service_action(ServiceAction::Restart).await },
    )
    .await
    {
        Ok(result) => Json(result).into_response(),
        Err(message) if message == WRITE_REVISION_CONFLICT => error(
            StatusCode::CONFLICT,
            "Конфигурация изменилась во время сохранения профиля",
        ),
        Err(message) => error(StatusCode::BAD_GATEWAY, message),
    }
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
        || !matches!(
            settings.layout.as_str(),
            "standard" | "compact" | "dense" | "wide" | "right" | "top" | "tiles"
        )
        || !matches!(
            settings.accent.as_str(),
            "blue" | "cyan" | "emerald" | "amber" | "rose" | "violet"
        )
        || !matches!(
            settings.status_bar.as_str(),
            "standard" | "minimal" | "compact" | "stacked" | "reverse"
        )
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

fn start_change_history_scheduler(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(600));
        loop {
            interval.tick().await;
            let Some(config) = config_path_for_state(&state) else {
                continue;
            };
            let _guard = state.config_write_lock.lock().await;
            let (source, note) = external_change_source(&config);
            if let Err(error) = change_history::scan(&state.data_dir, &config, source, note) {
                tracing::warn!("Не удалось проверить историю изменений: {error}");
            }
        }
    });
}

fn managed_asn_local_date() -> Option<String> {
    #[cfg(unix)]
    let output = std::process::Command::new("date")
        .arg("+%Y-%m-%d")
        .output()
        .ok()?;
    #[cfg(windows)]
    let output = std::process::Command::new("powershell.exe")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "(Get-Date).ToString('yyyy-MM-dd',[Globalization.CultureInfo]::InvariantCulture)",
        ])
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let date = String::from_utf8_lossy(&output.stdout).trim().to_owned();
    valid_iso_local_date(&date).then_some(date)
}

fn valid_iso_local_date(date: &str) -> bool {
    let bytes = date.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes
            .iter()
            .enumerate()
            .all(|(index, byte)| matches!(index, 4 | 7) || byte.is_ascii_digit())
}

fn config_uses_managed_asn_list(
    config: &str,
    list_id: &str,
    lists_directory: &Path,
) -> Option<bool> {
    let list_path = lists_directory.join(list_id).canonicalize().ok()?;
    let parsed = nfqws2_ui::runtime_profiles::from_config(config, "managed-asn-refresh").ok()?;
    for profile in parsed
        .profiles
        .iter()
        .filter(|profile| profile.source == "NFQWS_ARGS_CUSTOM")
    {
        let references = freeze_profile_join::profile_list_references(&profile.args).ok()?;
        for reference in references {
            let Ok(reference_path) = resolve_foreign_list_path(&reference, lists_directory) else {
                continue;
            };
            if reference_path == list_path {
                return Some(true);
            }
        }
    }
    Some(false)
}

fn reload_active_nfqw2_lists() -> Result<(), String> {
    #[cfg(unix)]
    {
        let Some((pid, args)) = active_process_snapshot() else {
            // The list is already on disk and nfqws2 will read it on its next start.
            return Ok(());
        };
        if !matches!(active_process_snapshot(), Some((current_pid, ref current_args)) if current_pid == pid && current_args == &args)
        {
            return Err("Активный процесс nfqws2 изменился перед перечитыванием списков".into());
        }
        let pid = libc::pid_t::try_from(pid)
            .map_err(|_| "Недопустимый PID nfqws2 для перечитывания списков".to_owned())?;
        // zapret2/nfqws2 reloads hostlists and ipsets on SIGHUP; no service restart is needed.
        let result = unsafe { libc::kill(pid, libc::SIGHUP) };
        if result != 0 {
            return Err(format!(
                "Не удалось отправить SIGHUP процессу nfqws2: {}",
                std::io::Error::last_os_error()
            ));
        }
        Ok(())
    }
    #[cfg(not(unix))]
    {
        // The router backend is Unix-only; desktop test runs have no nfqws2 process to signal.
        Ok(())
    }
}

async fn refresh_managed_asn_lists(state: &AppState) {
    let Some(local_date) = managed_asn_local_date() else {
        tracing::warn!("managed ASN list refresh skipped: local date is unavailable");
        return;
    };
    let Ok(_refresh_guard) = state.managed_asn_refresh_lock.try_lock() else {
        return;
    };
    let Some(paths) = discover_system_paths() else {
        return;
    };
    let Some(config_path) = config_path_for_state(state) else {
        return;
    };
    let Ok(config_bytes) = tokio::fs::read(config_path).await else {
        return;
    };
    let Ok(config) = String::from_utf8(config_bytes) else {
        return;
    };
    let mut store = ManagedAsnListStore::load(&state.data_dir);
    let entries = store.entries.clone();
    for mut feed in entries {
        if feed.asn == 0 || !freeze_profile_join::valid_asn_list_id(&feed.list_id) {
            store.remove(feed.asn);
            if let Err(message) = store.save(&state.data_dir) {
                tracing::warn!("managed ASN registry cleanup failed: {message}");
            }
            continue;
        }
        let list_path = paths.lists_dir.join(&feed.list_id);
        match std::fs::symlink_metadata(&list_path) {
            Ok(metadata) if metadata.file_type().is_file() => {}
            Ok(_) => {
                feed.paused = true;
                feed.last_error =
                    Some("Путь управляемого ASN-списка больше не является обычным файлом".into());
                store.register(feed);
                if let Err(message) = store.save(&state.data_dir) {
                    tracing::warn!("managed ASN state save failed: {message}");
                }
                continue;
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                store.remove(feed.asn);
                if let Err(message) = store.save(&state.data_dir) {
                    tracing::warn!("managed ASN registry cleanup failed: {message}");
                }
                continue;
            }
            Err(error) => {
                tracing::warn!("cannot inspect managed ASN list {}: {error}", feed.list_id);
                continue;
            }
        }
        let Some(referenced) =
            config_uses_managed_asn_list(&config, &feed.list_id, &paths.lists_dir)
        else {
            continue;
        };
        if !referenced {
            store.remove(feed.asn);
            if let Err(message) = store.save(&state.data_dir) {
                tracing::warn!("managed ASN registry cleanup failed: {message}");
            }
            continue;
        }
        if !feed.is_due(&local_date) {
            continue;
        }
        let result = managed_asn_lists::refresh_managed_list_with(
            &mut feed,
            &mut store,
            &state.data_dir,
            &list_path,
            &local_date,
            |asn| async move { fetch_ripe_announced_prefixes(asn).await },
            || async { reload_active_nfqw2_lists() },
        )
        .await;
        if let Err(message) = result {
            tracing::warn!("managed ASN list AS{} refresh failed: {message}", feed.asn);
        } else {
            tracing::info!("managed ASN list AS{} refreshed", feed.asn);
        }
    }
}

fn start_managed_asn_scheduler(state: Arc<AppState>) {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(30));
        loop {
            interval.tick().await;
            refresh_managed_asn_lists(&state).await;
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

fn resolve_log_path(state: &AppState, source: &str) -> Result<PathBuf, String> {
    if !["nfqws2-ui", "nfqws2"].contains(&source) {
        return Err("Неизвестный источник журнала".into());
    }
    let config = config_path_for_state(state);
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
        })
        .ok_or_else(|| "Каталог журналов nfqws2 не обнаружен".to_owned())?;
    let mut candidates = Vec::new();
    if source == "nfqws2-ui" {
        candidates.push(log_dir.join("nfqws2-ui.log"));
    } else {
        if let (Some(text), Some(config_path)) = (config_text.as_deref(), config.as_deref())
            && let Some(path) = configured_log_path(text, config_path)
        {
            candidates.push(path);
        }
        candidates
            .extend(["nfqws2.log", "nfqws.log", "nfqws2-debug.log"].map(|name| log_dir.join(name)));
    }
    let canonical_log_dir = log_dir
        .canonicalize()
        .map_err(|_| "Каталог журналов не найден".to_owned())?;
    candidates
        .into_iter()
        .find_map(|candidate| safe_log_candidate(&canonical_log_dir, &candidate))
        .ok_or_else(|| format!("Журнал {source} не найден"))
}

fn safe_log_candidate(log_dir: &Path, candidate: &Path) -> Option<PathBuf> {
    let metadata = std::fs::symlink_metadata(candidate).ok()?;
    if metadata.file_type().is_symlink() || !metadata.is_file() {
        return None;
    }
    let canonical_root = log_dir.canonicalize().ok()?;
    let canonical_candidate = candidate.canonicalize().ok()?;
    canonical_candidate
        .starts_with(canonical_root)
        .then_some(canonical_candidate)
}

async fn get_logs(State(state): State<Arc<AppState>>, Query(query): Query<LogsQuery>) -> Response {
    let path = match resolve_log_path(&state, &query.source) {
        Ok(path) => path,
        Err(message) if message == "Неизвестный источник журнала" => {
            return error(StatusCode::BAD_REQUEST, message);
        }
        Err(message) => return Json(LogsResponse { content: message }).into_response(),
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

async fn clear_log(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LogClearRequest>,
) -> Response {
    let path = match resolve_log_path(&state, &request.source) {
        Ok(path) => path,
        Err(message) if message == "Неизвестный источник журнала" => {
            return error(StatusCode::BAD_REQUEST, message);
        }
        Err(message) => return error(StatusCode::NOT_FOUND, message),
    };
    let result = tokio::task::spawn_blocking(move || {
        let mut options = std::fs::OpenOptions::new();
        options.write(true).truncate(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.custom_flags(libc::O_NOFOLLOW | libc::O_CLOEXEC);
        }
        let file = options.open(&path).map_err(|error| error.to_string())?;
        if !file
            .metadata()
            .map_err(|error| error.to_string())?
            .is_file()
        {
            return Err("Источник журнала не является обычным файлом".to_owned());
        }
        Ok::<(), String>(())
    })
    .await;
    match result {
        Ok(Ok(())) => Json(serde_json::json!({ "cleared": true })).into_response(),
        Ok(Err(message)) => error(StatusCode::INTERNAL_SERVER_ERROR, message),
        Err(join_error) => error(StatusCode::INTERNAL_SERVER_ERROR, join_error.to_string()),
    }
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
    let auth_state = state.auth.clone();
    Router::new()
        .merge(
            Router::new()
                .route("/api/auth/status", get(auth::AuthManager::status))
                .route("/api/auth/login", post(auth::AuthManager::login))
                .route("/api/auth/logout", post(auth::AuthManager::logout))
                .route(
                    "/api/auth/settings",
                    axum::routing::put(auth::AuthManager::update_settings),
                )
                .with_state(auth_state.clone()),
        )
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
        .route("/api/nfqws2/config/format", post(format_config))
        .route("/api/nfqws2/config/validate", post(validate_config))
        .route("/api/nfqws2/diagnostics/site", post(diagnose_site))
        .route(
            "/api/nfqws2/dpi-lab/strategy/analyze",
            post(analyze_dpi_strategy).layer(DefaultBodyLimit::max(
                dpi_lab::MAX_STRATEGY_SOURCE_BYTES * 6 + 1024,
            )),
        )
        .route(
            "/api/nfqws2/dpi-lab/analyze",
            post(analyze_dpi_capture).layer(DefaultBodyLimit::max(dpi_lab::MAX_PCAP_BYTES)),
        )
        .route(
            "/api/nfqws2/dpi-lab/experiments",
            get(list_dpi_experiments)
                .post(save_dpi_experiment)
                .layer(DefaultBodyLimit::max(dpi_lab::MAX_PCAP_BYTES)),
        )
        .route(
            "/api/nfqws2/dpi-lab/experiments/{id}",
            get(get_dpi_experiment),
        )
        .route(
            "/api/nfqws2/diagnostics/site/stream",
            post(diagnose_site_stream),
        )
        .route(
            "/api/nfqws2/diagnostics/support-report",
            post(build_support_report),
        )
        .route(
            "/api/nfqws2/diagnostics/config-check",
            post(start_foreign_config_check),
        )
        .route(
            "/api/nfqws2/diagnostics/config-check/operations/{id}",
            get(get_foreign_config_check),
        )
        .route(
            "/api/nfqws2/diagnostics/config-check/operations/{id}/cancel",
            post(cancel_foreign_config_check),
        )
        .route(
            "/api/nfqws2/diagnostics/config-check/operations/{id}/configs/{index}/apply",
            post(apply_foreign_config_candidate),
        )
        .route("/api/nfqws2/diagnostics/monitor", post(monitor_sites))
        .route(
            "/api/nfqws2/monitoring",
            get(get_monitoring_store).put(put_monitoring_store),
        )
        .route("/api/nfqws2/monitoring/run", post(run_saved_monitoring))
        .route(
            "/api/nfqws2/monitoring/auto-strategy",
            get(get_auto_strategy).put(put_auto_strategy),
        )
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
        .route("/api/community-strategies", get(get_community_strategies))
        .route(
            "/api/community-strategies/target-asn",
            get(get_community_target_asn),
        )
        .route(
            "/api/community-strategies/provider",
            post(set_community_provider),
        )
        .route("/api/nfqws2/strategies/pick", post(start_strategy_pick))
        .route(
            "/api/nfqws2/strategies/pick/multi",
            post(start_multi_strategy_pick),
        )
        .route(
            "/api/nfqws2/strategies/pick/history",
            get(get_strategy_pick_history).delete(clear_strategy_pick_history),
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
            "/api/nfqws2/diagnostics/freeze-scan/join-preview",
            post(preview_freeze_profile_join),
        )
        .route(
            "/api/nfqws2/diagnostics/freeze-scan/join",
            post(apply_freeze_profile_join),
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
            "/api/nfqws2/package-leftovers",
            get(get_package_leftovers).post(delete_package_leftovers),
        )
        .route(
            "/api/nfqws2/package-leftovers/compare",
            post(compare_package_leftover),
        )
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
        .route("/api/change-history", get(get_change_history))
        .route("/api/change-history/undo-last", post(undo_last_change))
        .route("/api/change-history/initial", post(restore_initial_history))
        .route(
            "/api/change-history/{id}/diff",
            get(get_change_history_diff),
        )
        .route(
            "/api/change-history/{id}/revert",
            post(revert_change_history),
        )
        .route(
            "/api/profiles",
            get(get_profiles).put(save_runtime_profiles),
        )
        .route("/api/profiles/catalog", get(get_profile_lua_catalog))
        .route(
            "/api/profiles/settings",
            axum::routing::put(save_runtime_profile_settings),
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
        .route(
            "/api/snapshots/{id}",
            axum::routing::delete(delete_snapshot_api),
        )
        .route("/api/logs", get(get_logs))
        .route("/api/logs/clear", post(clear_log))
        .route("/api/updates/releases", get(get_github_releases))
        .route("/api/updates/status", get(get_ui_update_status))
        .route("/api/updates/install", post(post_update_install))
        .route("/api/nfqws2/updates/releases", get(get_core_releases))
        .route("/api/nfqws2/updates/status", get(get_core_update_status))
        .route(
            "/api/nfqws2/updates/install",
            post(post_core_update_install),
        )
        .route("/api/updates/operations/{id}", get(get_update_operation))
        .with_state(state)
        .fallback_service(ServeDir::new(web_root).append_index_html_on_directories(true))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn_with_state(
            auth_state,
            auth::AuthManager::middleware,
        ))
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
        auth: Arc::new(auth::AuthManager::new(data_dir.clone())),
        data_dir,
        port,
        cpu_previous: Arc::new(Mutex::new(None)),
        config_write_lock: Mutex::new(()),
        config_override: None,
        update_lock: Mutex::new(()),
        ui_release_cache: Mutex::new(None),
        core_release_cache: Mutex::new(None),
        installed_core_version_cache: Mutex::new(InstalledCoreVersionCache::default()),
        keenetic_system_stats_cache: Mutex::new(KeeneticSystemStatsCache::default()),
        interception_status_cache: Mutex::new(InterceptionStatusCache::default()),
        ui_update_status: Arc::new(std::sync::RwLock::new(UiUpdateStatus::default())),
        site_diagnostic_lock: Arc::new(Mutex::new(())),
        dpi_lab_lock: Mutex::new(()),
        managed_asn_refresh_lock: Mutex::new(()),
        monitoring_lock: Arc::new(Mutex::new(())),
        auto_strategy_lock: Arc::new(Mutex::new(())),
        site_trace_controls: Mutex::new(HashMap::new()),
        site_trace_runner: Arc::new(router_site_trace_runner),
        foreign_config_controls: Mutex::new(HashMap::new()),
        strategy_pick_controls: Mutex::new(HashMap::new()),
        transfer_diagnostic_controls: Mutex::new(HashMap::new()),
        freeze_scan_controls: Mutex::new(HashMap::new()),
        nfqws_cli_catalog_cache: Mutex::new(None),
        nfqws_lua_catalog_cache: Mutex::new(None),
        #[cfg(test)]
        change_history_fail_path: Mutex::new(None),
    });
    if let Err(error) = recover_pending_config_apply(&state).await {
        eprintln!("nfqws2-ui: не удалось восстановить прерванное применение конфига: {error}");
        let _ = tokio::fs::remove_file(&port_file).await;
        return;
    }
    start_ui_update_checker(state.clone());
    start_monitoring_scheduler(state.clone());
    start_snapshot_scheduler(state.clone());
    start_change_history_scheduler(state.clone());
    start_managed_asn_scheduler(state.clone());
    let web_root = std::env::var_os("NFQWS2_UI_WEB_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/opt/share/nfqws2-ui/current/public"));
    let app = api_router(state, web_root);
    tracing::info!("nfqws2-ui listening on 0.0.0.0:{port}");
    if let Err(error) = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
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
    #[tokio::test]
    async fn monitoring_checks_are_bounded_to_three_concurrent_hosts() {
        use std::sync::{
            Arc,
            atomic::{AtomicUsize, Ordering},
        };

        let active = Arc::new(AtomicUsize::new(0));
        let peak = Arc::new(AtomicUsize::new(0));
        let results = super::run_bounded_monitor_checks(
            (0..10)
                .map(|index| format!("site-{index}.example"))
                .collect(),
            {
                let active = active.clone();
                let peak = peak.clone();
                move |host| {
                    let active = active.clone();
                    let peak = peak.clone();
                    async move {
                        let running = active.fetch_add(1, Ordering::SeqCst) + 1;
                        peak.fetch_max(running, Ordering::SeqCst);
                        tokio::time::sleep(std::time::Duration::from_millis(5)).await;
                        active.fetch_sub(1, Ordering::SeqCst);
                        super::MonitoringHostResult {
                            host,
                            summary: "ok".into(),
                            current: super::MonitoringStep {
                                id: "current",
                                status: "ok",
                                detail: "ok".into(),
                                duration_ms: Some(1),
                                http_code: Some(200),
                            },
                            steps: Vec::new(),
                        }
                    }
                }
            },
        )
        .await
        .unwrap();

        assert_eq!(results.len(), 10);
        assert!(peak.load(Ordering::SeqCst) <= 3);
        assert!(peak.load(Ordering::SeqCst) > 1);
    }

    #[test]
    fn freeze_scan_upload_probe_is_opt_in_and_backward_compatible() {
        let disabled: super::FreezeScanRequest =
            serde_json::from_str(r#"{"extraHosts":["example.com"]}"#).unwrap();
        assert!(!disabled.probe_short_uploads);
        let enabled: super::FreezeScanRequest =
            serde_json::from_str(r#"{"extraHosts":["example.com"],"probeShortUploads":true}"#)
                .unwrap();
        assert!(enabled.probe_short_uploads);
    }

    use super::*;

    #[test]
    fn keenetic_system_stats_cache_expires_after_five_seconds() {
        let checked_at = Instant::now();
        let mut cache = KeeneticSystemStatsCache::default();

        cache.store(
            Some(SystemInfo {
                memory_total_bytes: Some(1024),
                memory_used_bytes: Some(512),
                cpu_percent: Some(25.0),
                source: "keenetic-rci",
            }),
            checked_at,
        );

        assert!(keenetic_system_stats_cache_is_fresh(
            Some(checked_at),
            checked_at + std::time::Duration::from_secs(4)
        ));
        assert!(!keenetic_system_stats_cache_is_fresh(
            Some(checked_at),
            checked_at + std::time::Duration::from_secs(5)
        ));
        assert!(!keenetic_system_stats_cache_is_fresh(None, checked_at));
        assert_eq!(
            cache
                .get(checked_at + std::time::Duration::from_secs(4))
                .flatten()
                .map(|stats| stats.source),
            Some("keenetic-rci")
        );

        cache.store(None, checked_at);
        assert!(
            cache
                .get(checked_at + std::time::Duration::from_secs(4))
                .is_some_and(|stats| stats.is_none())
        );
        assert!(
            cache
                .get(checked_at + std::time::Duration::from_secs(5))
                .is_none()
        );
    }

    #[test]
    fn nfqws_help_catalog_parses_runtime_protocols_payloads_and_argument_scope() {
        let catalog = parse_nfqws_cli_catalog(
            "GLOBAL:\n  --daemon\n  --qnum=<n>\nMULTI-STRATEGY:\n  --filter-l7=proto[,proto] ; L6-L7 protocol filter : all unknown known http tls dtls quic wireguard dht discord stun xmpp dns mtproto bt utp_bt\nLUA PACKET PASS MODE:\n  --payload=type[,type] ; set payload types following LUA functions should process : all unknown empty known ipv4 ipv6 icmp http_req http_reply tls_client_hello tls_server_hello dtls_client_hello dtls_server_hello quic_initial wireguard_initiation wireguard_response wireguard_cookie wireguard_keepalive wireguard_data dht discord_ip_discovery stun xmpp_stream xmpp_starttls xmpp_proceed xmpp_features dns_query dns_response mtproto_initial bt_handshake utp_bt_handshake\n  --hostlist=<file> ; read domains\n",
        );
        assert_eq!(
            catalog.l7,
            [
                "all",
                "unknown",
                "known",
                "http",
                "tls",
                "dtls",
                "quic",
                "wireguard",
                "dht",
                "discord",
                "stun",
                "xmpp",
                "dns",
                "mtproto",
                "bt",
                "utp_bt"
            ]
        );
        assert_eq!(
            catalog.payloads,
            [
                "all",
                "unknown",
                "empty",
                "known",
                "ipv4",
                "ipv6",
                "icmp",
                "http_req",
                "http_reply",
                "tls_client_hello",
                "tls_server_hello",
                "dtls_client_hello",
                "dtls_server_hello",
                "quic_initial",
                "wireguard_initiation",
                "wireguard_response",
                "wireguard_cookie",
                "wireguard_keepalive",
                "wireguard_data",
                "dht",
                "discord_ip_discovery",
                "stun",
                "xmpp_stream",
                "xmpp_starttls",
                "xmpp_proceed",
                "xmpp_features",
                "dns_query",
                "dns_response",
                "mtproto_initial",
                "bt_handshake",
                "utp_bt_handshake"
            ]
        );
        assert!(
            catalog
                .options
                .iter()
                .find(|option| option.name == "qnum")
                .unwrap()
                .global
        );
        assert!(
            !catalog
                .options
                .iter()
                .find(|option| option.name == "hostlist")
                .unwrap()
                .global
        );
        assert!(
            !catalog
                .options
                .iter()
                .find(|option| option.name == "daemon")
                .unwrap()
                .takes_value
        );
    }

    #[test]
    fn nfqws_cli_catalog_cache_only_reuses_full_entries_for_the_same_binary() {
        let dir = tempfile::tempdir().unwrap();
        let binary = dir.path().join("nfqws2");
        std::fs::write(&binary, b"binary-v1").unwrap();
        let key = super::nfqws_cli_catalog_cache_key(&binary).unwrap();
        let full = super::NfqwsCliCatalog {
            options: vec![super::NfqwsCliOption {
                name: "lua-desync".into(),
                syntax: "--lua-desync=<func>".into(),
                description: "Lua desync".into(),
                global: false,
                takes_value: true,
            }],
            l7: vec!["tls".into()],
            payloads: vec!["tls_client_hello".into()],
            full: true,
        };
        let entry = super::NfqwsCliCatalogCacheEntry {
            key: key.clone(),
            catalog: full.clone(),
        };
        assert_eq!(
            super::reusable_nfqws_cli_catalog(Some(&entry), &key, false),
            Some(full.clone())
        );
        assert_eq!(
            super::reusable_nfqws_cli_catalog(Some(&entry), &key, true),
            None
        );

        std::fs::write(&binary, b"replacement-binary-with-different-length").unwrap();
        let changed_key = super::nfqws_cli_catalog_cache_key(&binary).unwrap();
        assert_ne!(changed_key, key);
        assert_eq!(
            super::reusable_nfqws_cli_catalog(Some(&entry), &changed_key, false),
            None
        );

        let partial = super::NfqwsCliCatalog {
            full: false,
            ..full
        };
        let partial_entry = super::NfqwsCliCatalogCacheEntry {
            key: changed_key.clone(),
            catalog: partial,
        };
        assert_eq!(
            super::reusable_nfqws_cli_catalog(Some(&partial_entry), &changed_key, false),
            None
        );
    }

    #[test]
    fn nfqws_lua_catalog_cache_tracks_script_files_and_can_be_refreshed() {
        let dir = tempfile::tempdir().unwrap();
        let lua = dir.path().join("host.lua");
        std::fs::write(&lua, b"function host(ctx, desync) end\n").unwrap();
        let key = super::nfqws_lua_catalog_cache_key(dir.path()).unwrap();
        let catalog = super::NfqwsLuaCatalog {
            functions: Vec::new(),
            standard_groups: Vec::new(),
        };
        let entry = super::NfqwsLuaCatalogCacheEntry {
            key: key.clone(),
            catalog: catalog.clone(),
        };
        assert_eq!(
            super::reusable_nfqws_lua_catalog(Some(&entry), &key, false),
            Some(catalog)
        );
        assert_eq!(
            super::reusable_nfqws_lua_catalog(Some(&entry), &key, true),
            None
        );

        std::fs::write(dir.path().join("notes.txt"), b"not a Lua dependency").unwrap();
        let unchanged_key = super::nfqws_lua_catalog_cache_key(dir.path()).unwrap();
        assert_eq!(unchanged_key, key);

        std::fs::write(&lua, b"function host(ctx, desync) end\n-- updated\n").unwrap();
        let changed_key = super::nfqws_lua_catalog_cache_key(dir.path()).unwrap();
        assert_ne!(changed_key, key);
        assert_eq!(
            super::reusable_nfqws_lua_catalog(Some(&entry), &changed_key, false),
            None
        );
    }

    #[test]
    fn nfqws_help_catalog_marks_partial_and_complete_help_output() {
        let partial = parse_nfqws_cli_catalog(
            "GLOBAL:\n  --qnum=<n>\nMULTI-STRATEGY:\n  --filter-l7=proto ; protocols : tls http\nLUA PACKET PASS MODE:\n  --payload=type ; payloads : known tls_client_hello\n",
        );
        assert!(!partial.full);

        let complete = parse_nfqws_cli_catalog(
            "GLOBAL:\n  --qnum=<n>\n  --lua-init=<code>\n  --blob=<name>:<file>\nMULTI-STRATEGY:\n  --new\n  --filter-tcp=<ports>\n  --filter-l7=proto ; protocols : tls http\n  --lua-desync=<fn>\nLUA PACKET PASS MODE:\n  --payload=type ; payloads : known tls_client_hello\n",
        );
        assert!(complete.full);

        // Match Zemidala's completeness rule: required option definitions prove
        // that help is complete; optional value lists may be absent on variants.
        let complete_without_value_lists = parse_nfqws_cli_catalog(
            "GLOBAL:\n  --qnum=<n>\n  --lua-init=<code>\n  --blob=<name>:<file>\nMULTI-STRATEGY:\n  --new\n  --filter-tcp=<ports>\n  --lua-desync=<fn>\nLUA PACKET PASS MODE:\n  --payload=type\n",
        );
        assert!(complete_without_value_lists.full);
    }

    #[test]
    fn log_path_resolution_rejects_symlinked_candidates() {
        let root = tempfile::tempdir().unwrap();
        let log_dir = root.path().join("log");
        std::fs::create_dir(&log_dir).unwrap();
        let actual = log_dir.join("actual.log");
        std::fs::write(&actual, "log\n").unwrap();
        assert_eq!(
            safe_log_candidate(&log_dir, &actual),
            Some(actual.canonicalize().unwrap())
        );

        #[cfg(unix)]
        {
            let alias = log_dir.join("nfqws2-ui.log");
            std::os::unix::fs::symlink(&actual, &alias).unwrap();
            assert_eq!(safe_log_candidate(&log_dir, &alias), None);
        }
    }

    #[test]
    fn runtime_profile_dry_run_arguments_accept_udp_only_configurations() {
        let directory = tempfile::tempdir().unwrap();
        let lists = directory.path().join("lists");
        let blobs = directory.path().join("blobs");
        let lua = directory.path().join("lua");
        std::fs::create_dir(&lists).unwrap();
        std::fs::create_dir(&blobs).unwrap();
        std::fs::create_dir(&lua).unwrap();
        let source = "NFQWS_ARGS_CUSTOM=\"--filter-udp=443 --payload=quic_initial\"\n";
        let mut auto_lists = Vec::new();
        let arguments = external_config_args(
            source,
            &directory.path().join("auto.list"),
            &lua,
            &blobs,
            &lists,
            &mut auto_lists,
            false,
        )
        .unwrap();
        assert!(arguments.contains(&"--filter-udp=443".to_owned()));
        assert!(arguments.contains(&"--payload=quic_initial".to_owned()));
        assert!(
            external_config_test_args(
                source,
                &directory.path().join("auto.list"),
                &lua,
                &blobs,
                &lists,
                &mut Vec::new(),
            )
            .is_err()
        );
    }

    #[test]
    fn runtime_profile_edit_changes_only_custom_arguments_and_preserves_other_config() {
        let original = concat!(
            "MODE_LIST='--hostlist=lists/user.list' # keep mode\n",
            "NFQWS_ARGS_CUSTOM=\"--filter-tcp=443\" # keep base comment\n",
            "NFQWS_ARGS_CUSTOM=\"$NFQWS_ARGS_CUSTOM --new --filter-tcp=80\" # keep append comment\n",
            "NFQWS_ARGS=\"--filter-tcp=80,443 --lua-desync=split\" # managed\n",
        );
        let request = RuntimeProfilesWrite {
            profiles: vec![
                vec!["--filter-tcp=443".into()],
                vec!["--filter-tcp=80".into(), "--lua-desync=fake".into()],
            ],
            revision: revision(original.as_bytes()),
        };
        let updated = build_runtime_profiles_config(original, request).unwrap();

        assert!(updated.contains("MODE_LIST='--hostlist=lists/user.list' # keep mode"));
        assert!(updated.contains("# keep base comment"));
        assert!(updated.contains("# keep append comment"));
        assert!(
            updated.contains("NFQWS_ARGS=\"--filter-tcp=80,443 --lua-desync=split\" # managed")
        );
        let document = ConfigDocument::parse(&updated).unwrap();
        assert_eq!(
            document.literal_or_mode_value("NFQWS_ARGS_CUSTOM").unwrap(),
            "--filter-tcp=443\n--new\n--filter-tcp=80\n--lua-desync=fake"
        );
    }

    #[test]
    fn runtime_profile_edit_rejects_stale_revisions_and_shell_fragments() {
        let original = "NFQWS_ARGS_CUSTOM=\"--filter-tcp=443\"\n";
        let stale = RuntimeProfilesWrite {
            profiles: vec![vec!["--filter-tcp=80".into()]],
            revision: "stale".into(),
        };
        assert_eq!(
            build_runtime_profiles_config(original, stale).unwrap_err(),
            WRITE_REVISION_CONFLICT
        );
        let unsafe_args = RuntimeProfilesWrite {
            profiles: vec![vec!["--filter-tcp=443;reboot".into()]],
            revision: revision(original.as_bytes()),
        };
        assert!(build_runtime_profiles_config(original, unsafe_args).is_err());
    }

    #[test]
    fn managed_profile_settings_update_only_allowlisted_source_groups() {
        let original = "NFQWS_ARGS_UDP=\"--filter-udp=443 --payload=quic_initial\"\nNFQWS_ARGS=\"--filter-tcp=80,443\" # shared HTTP base\n";
        let mut variables = std::collections::HashMap::new();
        variables.insert(
            "NFQWS_ARGS_UDP".to_owned(),
            vec![
                "--filter-udp=443".to_owned(),
                "--lua-desync=fake".to_owned(),
            ],
        );
        let updated = build_runtime_profile_settings_config(
            original,
            RuntimeProfileSettingsWrite {
                variables,
                revision: revision(original.as_bytes()),
            },
        )
        .unwrap();
        assert!(updated.contains("NFQWS_ARGS_UDP=\"--filter-udp=443\n--lua-desync=fake\""));
        assert!(updated.contains("NFQWS_ARGS=\"--filter-tcp=80,443\" # shared HTTP base"));

        let mut previously_missing_group = std::collections::HashMap::new();
        previously_missing_group.insert(
            "NFQWS_ARGS_QUIC".to_owned(),
            vec![
                "--filter-udp=443".to_owned(),
                "--payload=quic_initial".to_owned(),
            ],
        );
        let updated = build_runtime_profile_settings_config(
            original,
            RuntimeProfileSettingsWrite {
                variables: previously_missing_group,
                revision: revision(original.as_bytes()),
            },
        )
        .unwrap();
        assert!(updated.contains("NFQWS_ARGS_QUIC=\"--filter-udp=443\n--payload=quic_initial\""));

        let mut forbidden = std::collections::HashMap::new();
        forbidden.insert("IPV6_ENABLED".to_owned(), vec!["1".to_owned()]);
        assert!(
            build_runtime_profile_settings_config(
                original,
                RuntimeProfileSettingsWrite {
                    variables: forbidden,
                    revision: revision(original.as_bytes()),
                }
            )
            .is_err()
        );

        let mode_config = "MODE_AUTO=\"--hostlist-auto=lists/auto.list\"\nNFQWS_ARGS=\"--filter-tcp=443\"\nNFQWS_EXTRA_ARGS=\"$MODE_AUTO\"\n";
        let mut mode_variables = std::collections::HashMap::new();
        mode_variables.insert(
            "NFQWS_EXTRA_ARGS".to_owned(),
            vec!["--hostlist-auto=lists/auto.list".to_owned()],
        );
        assert!(
            build_runtime_profile_settings_config(
                mode_config,
                RuntimeProfileSettingsWrite {
                    variables: mode_variables,
                    revision: revision(mode_config.as_bytes()),
                }
            )
            .unwrap_err()
            .contains("чтобы сохранить связь со списками")
        );
    }

    #[test]
    fn managed_profile_settings_can_update_every_constructor_source_in_one_transaction() {
        let original = "";
        let mut variables = std::collections::HashMap::new();
        variables.insert(
            "NFQWS_ARGS_UDP".to_owned(),
            vec![
                "--filter-udp=443".to_owned(),
                "--payload=quic_initial".to_owned(),
            ],
        );
        variables.insert(
            "NFQWS_ARGS_QUIC".to_owned(),
            vec![
                "--filter-udp=443".to_owned(),
                "--payload=quic_initial".to_owned(),
                "--lua-desync=fake".to_owned(),
            ],
        );
        variables.insert(
            "NFQWS_ARGS_IPSET".to_owned(),
            vec!["--ipset=lists/ipset.list".to_owned()],
        );
        variables.insert(
            "NFQWS_ARGS".to_owned(),
            vec![
                "--filter-tcp=80,443".to_owned(),
                "--filter-l7=tls".to_owned(),
                "--lua-desync=fake".to_owned(),
            ],
        );
        variables.insert(
            "NFQWS_EXTRA_ARGS".to_owned(),
            vec!["--hostlist=lists/user.list".to_owned()],
        );

        let updated = build_runtime_profile_settings_config(
            original,
            RuntimeProfileSettingsWrite {
                variables,
                revision: revision(original.as_bytes()),
            },
        )
        .unwrap();
        let document = ConfigDocument::parse(&updated).unwrap();
        for (source, expected) in [
            ("NFQWS_ARGS_UDP", "--filter-udp=443\n--payload=quic_initial"),
            (
                "NFQWS_ARGS_QUIC",
                "--filter-udp=443\n--payload=quic_initial\n--lua-desync=fake",
            ),
            ("NFQWS_ARGS_IPSET", "--ipset=lists/ipset.list"),
            (
                "NFQWS_ARGS",
                "--filter-tcp=80,443\n--filter-l7=tls\n--lua-desync=fake",
            ),
            ("NFQWS_EXTRA_ARGS", "--hostlist=lists/user.list"),
        ] {
            assert_eq!(
                document.literal_or_mode_value(source).unwrap(),
                expected,
                "{source}"
            );
        }
        let profiles = nfqws2_ui::runtime_profiles::from_config(&updated, "rev").unwrap();
        assert_eq!(
            profiles
                .profiles
                .iter()
                .filter(|profile| !profile.editable)
                .count(),
            5
        );
        assert_eq!(profiles.profiles[0].components[0].source, "NFQWS_ARGS_UDP");
        assert_eq!(profiles.profiles[1].components[0].source, "NFQWS_ARGS_QUIC");
        assert_eq!(profiles.profiles[2].components[0].source, "NFQWS_ARGS_QUIC");
        assert_eq!(profiles.profiles[3].components[0].source, "NFQWS_ARGS");
        assert_eq!(profiles.profiles[4].components[0].source, "NFQWS_ARGS");
    }

    #[tokio::test]
    async fn profile_api_lists_all_groups_and_only_custom_are_marked_editable() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        std::fs::write(
            &config,
            "MODE_LIST='--hostlist=lists/user.list'\nNFQWS_ARGS_CUSTOM=\"--filter-tcp=443 --lua-desync=fake --new=video --filter-tcp=80 --lua-desync=split\"\nNFQWS_ARGS_UDP=\"--filter-udp=443 --payload=quic_initial\"\nNFQWS_ARGS=\"--filter-tcp=80,443 --hostlist=lists/user.list\"\nNFQWS_EXTRA_ARGS=\"$MODE_LIST\"\n",
        )
        .unwrap();
        let app = api_router(
            test_state_with_config(dir.path().join("data"), Some(config)),
            dir.path().join("web"),
        );
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/profiles")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        let result: serde_json::Value = serde_json::from_slice(&body).unwrap();
        let profiles = result["profiles"].as_array().unwrap();
        assert_eq!(profiles.len(), 7);
        assert_eq!(profiles[0]["name"], "Свой профиль 1");
        assert_eq!(profiles[1]["source"], "NFQWS_ARGS_CUSTOM");
        assert_eq!(profiles[2]["source"], "NFQWS_ARGS_UDP");
        assert_eq!(profiles[3]["name"], "QUIC по IP");
        assert_eq!(profiles[4]["name"], "QUIC по сайтам");
        assert_eq!(profiles[5]["name"], "HTTP/HTTPS по IP");
        assert_eq!(profiles[6]["name"], "HTTP/HTTPS по сайтам");
        assert_eq!(profiles[0]["editable"], true);
        assert_eq!(profiles[2]["editable"], false);
        assert_eq!(profiles[3]["components"][0]["source"], "NFQWS_ARGS_QUIC");
        assert_eq!(profiles[3]["components"][0]["editable"], true);
        assert_eq!(profiles[4]["components"][1]["source"], "NFQWS_EXTRA_ARGS");
        assert_eq!(profiles[4]["components"][1]["editable"], false);
        assert!(
            result["revision"]
                .as_str()
                .is_some_and(|value| !value.is_empty())
        );
    }

    #[test]
    fn support_report_masks_site_names_by_default_and_sanitizes_visible_names() {
        let sites = vec![
            MonitoredSite {
                host: "example.com".into(),
                results: Vec::new(),
            },
            MonitoredSite {
                host: "second.example\nInjected".into(),
                results: Vec::new(),
            },
        ];

        assert_eq!(support_site_labels(&sites, true), ["сайт-1", "сайт-2"]);
        assert_eq!(
            support_site_labels(&sites, false),
            ["example.com", "second.exampleInjected"]
        );
    }

    #[test]
    fn support_report_summarizes_router_resources_core_and_interception() {
        let report = format_support_report(&SupportReportData {
            platform: "Keenetic / Entware".into(),
            architecture: "aarch64".into(),
            service_state: "запущена".into(),
            core_version: Some("1.3.2".into()),
            memory_used_bytes: Some(256 * 1024 * 1024),
            memory_total_bytes: Some(512 * 1024 * 1024),
            cpu_percent: Some(12.5),
            interception_status: "цепочки подключены".into(),
            running_profiles: 3,
            monitored_sites: vec!["сайт-1".into()],
            monitoring_enabled: true,
            config_errors: 0,
            config_warnings: 2,
            lists_count: 4,
        });

        assert!(report.contains("Архитектура: aarch64"));
        assert!(report.contains("Версия nfqws2: 1.3.2"));
        assert!(report.contains("Память: 256 / 512 MiB"));
        assert!(report.contains("Загрузка CPU: 12.5%"));
        assert!(report.contains("Правила перехвата: цепочки подключены"));
    }

    #[test]
    fn foreign_config_request_limits_match_zemidala() {
        assert!(validate_foreign_config_counts(1, 1).is_ok());
        assert!(validate_foreign_config_counts(10, 15).is_ok());
        assert!(validate_foreign_config_counts(11, 1).is_err());
        assert!(validate_foreign_config_counts(1, 16).is_err());
        assert!(validate_foreign_config_counts(0, 1).is_err());
        assert!(validate_foreign_config_counts(1, 0).is_err());
    }

    #[test]
    fn foreign_config_apply_requires_complete_success_for_every_site() {
        let successful = ForeignConfigComparison {
            name: "Проверенный конфиг".into(),
            source: "Файл".into(),
            sites: vec![
                ForeignConfigSiteResult {
                    host: "one.example".into(),
                    current_ok: Some(false),
                    current_detail: String::new(),
                    current_attempts: 1,
                    current_successes: 0,
                    current_average_ms: Some(210),
                    candidate_ok: Some(true),
                    candidate_detail: String::new(),
                    candidate_attempts: 3,
                    candidate_successes: 3,
                    candidate_average_ms: Some(95),
                },
                ForeignConfigSiteResult {
                    host: "two.example".into(),
                    current_ok: Some(true),
                    current_detail: String::new(),
                    current_attempts: 1,
                    current_successes: 1,
                    current_average_ms: Some(120),
                    candidate_ok: Some(true),
                    candidate_detail: String::new(),
                    candidate_attempts: 3,
                    candidate_successes: 3,
                    candidate_average_ms: Some(110),
                },
            ],
        };
        assert!(foreign_config_candidate_is_applicable(
            "complete",
            &successful
        ));
        assert!(!foreign_config_candidate_is_applicable(
            "cancelled",
            &successful
        ));

        let mut incomplete = successful.clone();
        incomplete.sites[1].candidate_ok = None;
        assert!(!foreign_config_candidate_is_applicable(
            "complete",
            &incomplete
        ));

        let mut failed = successful.clone();
        failed.sites[1].candidate_ok = Some(false);
        assert!(!foreign_config_candidate_is_applicable("complete", &failed));
    }

    #[test]
    fn foreign_config_repeats_stop_after_three_successes_or_the_first_failure() {
        assert!(foreign_config_should_retry(Some(true), 0, 3));
        assert!(foreign_config_should_retry(Some(true), 1, 3));
        assert!(!foreign_config_should_retry(Some(true), 2, 3));
        assert!(!foreign_config_should_retry(Some(true), 0, 1));
        assert!(!foreign_config_should_retry(Some(false), 0, 3));
        assert!(!foreign_config_should_retry(None, 0, 3));
        assert_eq!(normalize_foreign_config_repeats(None), 3);
        assert_eq!(normalize_foreign_config_repeats(Some(1)), 1);
        assert_eq!(normalize_foreign_config_repeats(Some(5)), 5);
        assert_eq!(normalize_foreign_config_repeats(Some(0)), 1);
        assert_eq!(normalize_foreign_config_repeats(Some(99)), 5);
    }

    #[test]
    fn foreign_config_site_metrics_are_serialized_and_legacy_operations_still_load() {
        let site = ForeignConfigSiteResult {
            host: "example.org".into(),
            current_ok: Some(true),
            current_detail: "HTTP 200".into(),
            current_attempts: 1,
            current_successes: 1,
            current_average_ms: Some(125),
            candidate_ok: Some(true),
            candidate_detail: "3/3 успешных".into(),
            candidate_attempts: 3,
            candidate_successes: 3,
            candidate_average_ms: Some(98),
        };
        let value = serde_json::to_value(&site).unwrap();
        assert_eq!(value["candidateAttempts"], 3);
        assert_eq!(value["candidateSuccesses"], 3);
        assert_eq!(value["candidateAverageMs"], 98);
        assert_eq!(value["currentAverageMs"], 125);

        let legacy: ForeignConfigSiteResult = serde_json::from_value(serde_json::json!({
            "host": "legacy.example",
            "currentOk": true,
            "currentDetail": "HTTP 200",
            "candidateOk": true,
            "candidateDetail": "3/3 успешных"
        }))
        .unwrap();
        assert_eq!(legacy.candidate_attempts, 0);
        assert_eq!(legacy.candidate_average_ms, None);
    }

    #[test]
    fn foreign_config_memory_guard_matches_the_twenty_megabyte_upstream_floor() {
        assert!(foreign_config_memory_warning("MemAvailable: 19456 kB\n").is_some());
        assert!(foreign_config_memory_warning("MemAvailable: 20479 kB\n").is_some());
        assert!(foreign_config_memory_warning("MemAvailable: 20480 kB\n").is_none());
        assert!(foreign_config_memory_warning("MemFree: 8192 kB\n").is_none());
    }

    #[test]
    fn foreign_config_parser_resolves_installed_lua_paths_and_active_list_paths() {
        let directory = tempfile::tempdir().unwrap();
        let lists = directory.path().join("lists");
        let blobs = directory.path().join("blobs");
        let lua = directory.path().join("lua");
        std::fs::create_dir(&lists).unwrap();
        std::fs::create_dir(&blobs).unwrap();
        std::fs::create_dir(&lua).unwrap();
        std::fs::write(lists.join("user.list"), "example.org\n").unwrap();
        std::fs::write(lists.join("auto.list"), "auto.example.org\n").unwrap();
        std::fs::write(blobs.join("tls_clienthello.bin"), b"tls hello").unwrap();
        std::fs::write(lua.join("zapret-lib.lua"), "-- installed library\n").unwrap();
        let source = "NFQWS_BASE_ARGS=\"--lua-init=@/opt/etc/nfqws2/lua/zapret-lib.lua --blob=tls_clienthello:@/opt/etc/nfqws2/blobs/tls_clienthello.bin --hostlist-auto=/opt/zapret/ipset/auto.list --hostlist-auto-debug=/var/log/auto.log\"\nNFQWS_ARGS=\"--filter-tcp=80,443 --hostlist=/opt/zapret/ipset/user.list --payload=tls_client_hello --lua-desync=fake\"\nNFQWS_ARGS_CUSTOM=\"--new --filter-tcp=443 --lua-desync=multisplit:pos=1\"\n";
        let isolated_auto_list = Path::new("/tmp/config-check/auto.list");
        let mut auto_lists = Vec::new();
        let args = external_config_test_args(
            source,
            isolated_auto_list,
            &lua,
            &blobs,
            &lists,
            &mut auto_lists,
        )
        .unwrap();
        assert_eq!(args.len(), 11, "{args:#?}");
        assert!(args.contains(&"--new".to_string()));
        assert!(args.contains(&format!(
            "--lua-init=@{}",
            lua.join("zapret-lib.lua").canonicalize().unwrap().display()
        )));
        assert!(args.contains(&format!(
            "--blob=tls_clienthello:@{}",
            blobs.join("tls_clienthello.bin").canonicalize().unwrap().display()
        )));
        assert!(args.contains(&format!("--hostlist-auto={}", isolated_auto_list.display())));
        assert_eq!(auto_lists.len(), 1);
        assert_eq!(
            auto_lists[0].1.as_deref(),
            Some(lists.join("auto.list").canonicalize().unwrap().as_path())
        );
        assert!(args.contains(&format!(
            "--hostlist={}",
            lists.join("user.list").canonicalize().unwrap().display()
        )));
        assert!(
            !args
                .iter()
                .any(|argument| argument.starts_with("--hostlist-auto-debug="))
        );
        assert!(
            args.iter()
                .any(|argument| argument == "--filter-tcp=80,443")
        );

        let mode_reference = "MODE_LIST=\"--hostlist=lists/user.list\"\nNFQWS_ARGS=\"--filter-tcp=443\"\nNFQWS_EXTRA_ARGS=\"$MODE_LIST\"\n";
        let mut auto_lists = Vec::new();
        let args = external_config_test_args(
            mode_reference,
            Path::new("/tmp/config-check/mode-auto.list"),
            &lua,
            &blobs,
            &lists,
            &mut auto_lists,
        )
        .unwrap();
        assert!(args.iter().any(|argument| argument
            == &format!(
                "--hostlist={}",
                lists.join("user.list").canonicalize().unwrap().display()
            )));
    }

    #[test]
    fn foreign_config_parser_resolves_nested_zemidala_modes_and_isolates_auto_list() {
        let directory = tempfile::tempdir().unwrap();
        let lists = directory.path().join("lists");
        let blobs = directory.path().join("blobs");
        let lua = directory.path().join("lua");
        std::fs::create_dir(&lists).unwrap();
        std::fs::create_dir(&blobs).unwrap();
        std::fs::create_dir(&lua).unwrap();
        std::fs::write(lists.join("user.list"), "example.org\n").unwrap();
        std::fs::write(lists.join("exclude.list"), "excluded.example.org\n").unwrap();
        std::fs::write(lists.join("auto.list"), "learned.example.org\n").unwrap();
        let source = concat!(
            "MODE_LIST=\"--hostlist=lists/user.list\"\n",
            "MODE_ALL=\"--hostlist-exclude=lists/exclude.list\"\n",
            "MODE_AUTO=\"$MODE_LIST --hostlist-auto=lists/auto.list $MODE_ALL\"\n",
            "NFQWS_EXTRA_ARGS=\"$MODE_AUTO\"\n",
            "NFQWS_ARGS=\"--filter-tcp=443 --payload=tls_client_hello --lua-desync=fake\"\n",
        );
        let isolated_auto_list = Path::new("/tmp/config-check/nested-mode-auto.list");
        let mut auto_lists = Vec::new();
        let args = external_config_test_args(
            source,
            isolated_auto_list,
            &lua,
            &blobs,
            &lists,
            &mut auto_lists,
        )
        .unwrap();

        assert!(args.contains(&format!(
            "--hostlist={}",
            lists.join("user.list").canonicalize().unwrap().display()
        )));
        assert!(args.contains(&format!(
            "--hostlist-exclude={}",
            lists.join("exclude.list").canonicalize().unwrap().display()
        )));
        assert!(args.contains(&format!("--hostlist-auto={}", isolated_auto_list.display())));
        assert_eq!(auto_lists.len(), 1);
        assert_eq!(
            auto_lists[0].1.as_deref(),
            Some(lists.join("auto.list").canonicalize().unwrap().as_path())
        );
    }

    #[test]
    fn foreign_config_parser_preserves_empty_custom_profile_boundaries_like_zemidala() {
        let directory = tempfile::tempdir().unwrap();
        let lists = directory.path().join("lists");
        let blobs = directory.path().join("blobs");
        let lua = directory.path().join("lua");
        std::fs::create_dir(&lists).unwrap();
        std::fs::create_dir(&blobs).unwrap();
        std::fs::create_dir(&lua).unwrap();
        let source = concat!(
            "NFQWS_ARGS_CUSTOM=\"--new --filter-tcp=80 --lua-desync=first --new --new=video --filter-tcp=443 --lua-desync=second --new= --new\"\n",
            "NFQWS_ARGS=\"--filter-tcp=443 --lua-desync=base\"\n",
        );
        let args = external_config_test_args(
            source,
            Path::new("/tmp/config-check/empty-profile.list"),
            &lua,
            &blobs,
            &lists,
            &mut Vec::new(),
        )
        .unwrap();
        let profiles = args
            .split(|argument| argument == "--new")
            .map(|profile| profile.join(" "))
            .collect::<Vec<_>>();
        assert_eq!(profiles.len(), 7, "{profiles:#?}");
        assert!(profiles[0].is_empty());
        assert!(profiles[1].contains("--filter-tcp=80 --lua-desync=first"));
        assert!(profiles[2].is_empty());
        assert!(profiles[3].contains("--name=video"));
        assert!(profiles[3].contains("--lua-desync=second"));
        assert!(profiles[4].is_empty());
        assert!(profiles[5].is_empty());
        assert!(profiles[6].contains("--filter-tcp=443 --lua-desync=base"));
    }

    #[test]
    fn foreign_config_defaults_use_the_detected_entware_or_openwrt_root() {
        assert_eq!(
            foreign_config_install_root(Path::new("/opt/etc/nfqws2/lists")),
            "/opt"
        );
        assert_eq!(
            foreign_config_install_root(Path::new("/etc/nfqws2/lists")),
            ""
        );
        assert_eq!(
            foreign_config_install_root(Path::new("C:/temp/lists")),
            "/opt"
        );
    }

    #[test]
    fn foreign_config_defaults_missing_variables_like_zemidala() {
        let directory = tempfile::tempdir().unwrap();
        let lists = directory.path().join("lists");
        let blobs = directory.path().join("blobs");
        let lua = directory.path().join("lua");
        std::fs::create_dir(&lists).unwrap();
        std::fs::create_dir(&blobs).unwrap();
        std::fs::create_dir(&lua).unwrap();
        for file in ["user.list", "exclude.list", "auto.list"] {
            std::fs::write(lists.join(file), "example.com\n").unwrap();
        }
        for file in ["zapret-lib.lua", "zapret-antidpi.lua", "zapret-auto.lua"] {
            std::fs::write(lua.join(file), "-- fixture\n").unwrap();
        }
        for file in ["quic_initial.bin", "tls_clienthello.bin"] {
            std::fs::write(blobs.join(file), b"blob").unwrap();
        }
        let source =
            "NFQWS_ARGS=\"--filter-tcp=80,443 --payload=tls_client_hello --lua-desync=fake\"\n";
        let adapted = foreign_config_with_zemidala_defaults(source, &lua, &blobs, &lists)
            .expect("stock defaults should be safely added");
        assert!(adapted.contains("MODE_LIST=\"--hostlist=/opt/etc/nfqws2/lists/user.list\""));
        assert!(
            adapted.contains("MODE_ALL=\"--hostlist-exclude=/opt/etc/nfqws2/lists/exclude.list\"")
        );
        assert!(
            adapted
                .contains("MODE_AUTO=\"$MODE_LIST --hostlist-auto=/opt/etc/nfqws2/lists/auto.list")
        );
        assert!(adapted.contains("NFQWS_EXTRA_ARGS=\"$MODE_LIST\""));
        assert!(
            adapted.contains("NFQWS_BASE_ARGS=\"--lua-init=@/opt/etc/nfqws2/lua/zapret-lib.lua")
        );
        assert!(adapted.contains("--blob=quic_initial:@/opt/etc/nfqws2/blobs/quic_initial.bin"));
        assert!(
            adapted.contains("--blob=tls_clienthello:@/opt/etc/nfqws2/blobs/tls_clienthello.bin")
        );

        let openwrt = foreign_config_with_zemidala_defaults(
            source,
            &lua,
            &blobs,
            Path::new("/etc/nfqws2/lists"),
        )
        .unwrap();
        assert!(openwrt.contains("MODE_LIST=\"--hostlist=/etc/nfqws2/lists/user.list\""));
        assert!(openwrt.contains("MODE_ALL=\"--hostlist-exclude=/etc/nfqws2/lists/exclude.list\""));
        assert!(openwrt.contains("--hostlist-auto=/etc/nfqws2/lists/auto.list"));
        assert!(openwrt.contains("--hostlist-auto-debug=/var/log/nfqws2.log"));
        assert!(openwrt.contains("NFQWS_BASE_ARGS=\"--lua-init=@/etc/nfqws2/lua/zapret-lib.lua"));

        let args = external_config_test_args(
            &adapted,
            &directory.path().join("isolated-auto.list"),
            &lua,
            &blobs,
            &lists,
            &mut Vec::new(),
        )
        .unwrap();
        assert!(
            args.iter()
                .any(|argument| argument.starts_with("--hostlist="))
        );
        assert!(
            args.iter()
                .any(|argument| argument.starts_with("--blob=quic_initial:@"))
        );
        assert!(
            args.iter()
                .any(|argument| argument.starts_with("--blob=tls_clienthello:@"))
        );

        let auto_source = concat!(
            "NFQWS_ARGS=\"--filter-tcp=80,443 --payload=tls_client_hello --lua-desync=fake\"\n",
            "NFQWS_EXTRA_ARGS=\"$MODE_AUTO\"\n",
        );
        let auto_adapted =
            foreign_config_with_zemidala_defaults(auto_source, &lua, &blobs, &lists).unwrap();
        let auto_args = external_config_test_args(
            &auto_adapted,
            &directory.path().join("isolated-auto-mode.list"),
            &lua,
            &blobs,
            &lists,
            &mut Vec::new(),
        )
        .unwrap();
        assert!(
            auto_args
                .iter()
                .any(|argument| argument.starts_with("--hostlist-auto="))
        );

        let explicitly_empty = concat!(
            "NFQWS_BASE_ARGS=\"\"\n",
            "NFQWS_ARGS=\"--filter-tcp=443\"\n",
            "NFQWS_EXTRA_ARGS=\"\"\n",
        );
        let unchanged =
            foreign_config_with_zemidala_defaults(explicitly_empty, &lua, &blobs, &lists).unwrap();
        assert!(unchanged.contains("MODE_LIST="));
        assert!(!unchanged.contains("--lua-init=@/opt/etc/nfqws2/lua/"));
        assert!(unchanged.contains("NFQWS_BASE_ARGS=\"\""));
        assert!(unchanged.contains("NFQWS_EXTRA_ARGS=\"\""));
    }

    #[test]
    fn foreign_config_dry_run_adds_required_queue_user_and_fastpath_defaults() {
        let args = vec![
            "--filter-tcp=443".to_string(),
            "--lua-desync=fake".to_string(),
        ];
        let argv =
            foreign_config_dry_run_argv("/usr/bin/nfqws2", "NFQUEUE_NUM=317\nUSER=daemon\n", &args)
                .unwrap();
        assert_eq!(argv[0], "/usr/bin/nfqws2");
        assert!(argv.contains(&"--dry-run".into()));
        assert!(argv.contains(&"--user=daemon".into()));
        assert!(argv.contains(&"--qnum=317".into()));
        assert!(argv.contains(&"--fastpath-workaround=auto".into()));
        assert!(argv.ends_with(&args));

        let defaulted = foreign_config_dry_run_argv("nfqws2", "", &args).unwrap();
        assert!(defaulted.contains(&"--user=nobody".into()));
        assert!(defaulted.contains(&"--qnum=300".into()));
        assert!(defaulted.contains(&"--fastpath-workaround=auto".into()));

        let empty_values =
            foreign_config_dry_run_argv("nfqws2", "NFQUEUE_NUM=\"\"\nUSER=\"\"\n", &args).unwrap();
        assert!(empty_values.contains(&"--user=nobody".into()));
        assert!(empty_values.contains(&"--qnum=300".into()));
        assert!(
            foreign_config_dry_run_argv("nfqws2", "NFQUEUE_NUM=not-a-number\n", &args).is_err()
        );
        assert!(foreign_config_dry_run_argv("nfqws2", "USER=\"nobody --bad\"\n", &args).is_err());

        let profile_fastpath = vec!["--fastpath-workaround=none".to_string()];
        let profile_only = foreign_config_dry_run_argv("nfqws2", "", &profile_fastpath).unwrap();
        assert!(profile_only.contains(&"--fastpath-workaround=auto".into()));

        let base_fastpath = foreign_config_dry_run_argv(
            "nfqws2",
            "NFQWS_BASE_ARGS=\"--fastpath-workaround=none\"\n",
            &args,
        )
        .unwrap();
        assert!(!base_fastpath.contains(&"--fastpath-workaround=auto".into()));
    }

    #[test]
    fn foreign_config_parser_builds_zemidala_runtime_profiles_for_all_argument_groups() {
        let directory = tempfile::tempdir().unwrap();
        let lists = directory.path().join("lists");
        let lua = directory.path().join("lua");
        std::fs::create_dir(&lists).unwrap();
        std::fs::create_dir(&lua).unwrap();
        std::fs::write(lua.join("safe.lua"), "-- safe module\n").unwrap();
        let source = concat!(
            "NFQWS_BASE_ARGS=\"--qnum=17 --daemon --debug=/tmp/foreign.log --lua-init=@/opt/etc/nfqws2/lua/safe.lua\"\n",
            "NFQWS_ARGS_CUSTOM=\"--filter-tcp=443 --lua-desync=custom\"\n",
            "NFQWS_ARGS_CUSTOM=\"$NFQWS_ARGS_CUSTOM --new=video --filter-tcp=80 --lua-desync=custom2\"\n",
            "NFQWS_ARGS_UDP=\"--filter-udp=443 --payload=quic_initial\"\n",
            "NFQWS_ARGS_QUIC=\"--filter-udp=443 --payload=quic_initial --lua-desync=quic\"\n",
            "NFQWS_ARGS_IPSET=\"--ipset=lists/ips.txt\"\n",
            "NFQWS_EXTRA_ARGS=\"--lua-desync=extra\"\n",
            "NFQWS_ARGS=\"--filter-tcp=443 --lua-desync=tcp\"\n",
        );
        std::fs::write(lists.join("ips.txt"), "192.0.2.1\n").unwrap();
        let mut auto_lists = Vec::new();
        let args = external_config_test_args(
            source,
            Path::new("/tmp/config-check/auto.list"),
            &lua,
            &lists,
            &lists,
            &mut auto_lists,
        )
        .unwrap();

        let profiles = args
            .split(|argument| argument == "--new")
            .map(|profile| profile.join(" "))
            .collect::<Vec<_>>();
        assert_eq!(profiles.len(), 7, "{profiles:#?}");
        assert!(profiles[0].contains("--filter-tcp=443 --lua-desync=custom"));
        assert!(profiles[1].contains("--name=video"));
        assert!(profiles[1].contains("--filter-tcp=80"));
        assert!(profiles[2].contains("--filter-udp=443 --payload=quic_initial"));
        assert!(profiles[3].contains("--lua-desync=quic"));
        assert!(profiles[3].contains("--ipset-ip=0.0.0.0"));
        assert!(profiles[4].contains("--lua-desync=quic --lua-desync=extra"));
        assert!(profiles[5].contains("--lua-desync=tcp"));
        assert!(profiles[5].contains("--ipset-ip=0.0.0.0"));
        assert!(profiles[6].contains("--lua-desync=tcp --lua-desync=extra"));
        assert!(
            profiles
                .iter()
                .all(|profile| !profile.contains("--qnum=17"))
        );
        assert!(profiles.iter().all(|profile| !profile.contains("--daemon")));
        assert!(profiles.iter().all(|profile| !profile.contains("--debug=")));
    }

    #[test]
    fn foreign_config_parser_maps_installed_lua_and_adds_available_defaults() {
        let directory = tempfile::tempdir().unwrap();
        let lists = directory.path().join("lists");
        let blobs = directory.path().join("blobs");
        let lua = directory.path().join("lua");
        std::fs::create_dir(&lists).unwrap();
        std::fs::create_dir(&blobs).unwrap();
        std::fs::create_dir(&lua).unwrap();
        std::fs::write(lua.join("custom.lua"), "-- local custom module\n").unwrap();
        std::fs::write(lua.join("zapret-lib.lua"), "-- lib\n").unwrap();
        std::fs::write(lua.join("zapret-antidpi.lua.gz"), b"gzip content").unwrap();
        std::fs::write(lua.join("zapret-auto.lua"), "-- auto\n").unwrap();

        let source = concat!(
            "NFQWS_BASE_ARGS=\"--lua-init=@/opt/zapret2/lua/custom.lua\"\n",
            "NFQWS_ARGS=\"--filter-tcp=443 --payload=tls_client_hello --lua-desync=fake\"\n",
        );
        let args = external_config_test_args(
            source,
            Path::new("/tmp/config-check/auto.list"),
            &lua,
            &blobs,
            &lists,
            &mut Vec::new(),
        )
        .unwrap();
        assert!(args.contains(&format!(
            "--lua-init=@{}",
            lua.join("custom.lua").canonicalize().unwrap().display()
        )));
        for module in ["zapret-lib.lua", "zapret-antidpi.lua.gz", "zapret-auto.lua"] {
            assert!(args.contains(&format!(
                "--lua-init=@{}",
                lua.join(module).canonicalize().unwrap().display()
            )));
        }

        let missing = concat!(
            "NFQWS_BASE_ARGS=\"--lua-init=@/etc/passwd\"\n",
            "NFQWS_ARGS=\"--filter-tcp=443\"\n",
        );
        assert!(
            external_config_test_args(
                missing,
                Path::new("/tmp/config-check/auto.list"),
                &lua,
                &blobs,
                &lists,
                &mut Vec::new(),
            )
            .is_err()
        );
    }

    #[test]
    fn foreign_config_parser_rejects_unmapped_blob_files_and_keeps_inline_blobs() {
        let directory = tempfile::tempdir().unwrap();
        let lists = directory.path().join("lists");
        let blobs = directory.path().join("blobs");
        let lua = directory.path().join("lua");
        std::fs::create_dir(&lists).unwrap();
        std::fs::create_dir(&blobs).unwrap();
        std::fs::create_dir(&lua).unwrap();
        std::fs::write(blobs.join("tls.bin"), b"tls blob").unwrap();
        let unknown_file = concat!(
            "NFQWS_BASE_ARGS=\"--blob=foreign:@/etc/shadow\"\n",
            "NFQWS_ARGS=\"--filter-tcp=443 --payload=tls_client_hello --lua-desync=fake:blob=foreign\"\n",
        );
        assert!(
            external_config_test_args(
                unknown_file,
                Path::new("/tmp/config-check/auto.list"),
                &lua,
                &blobs,
                &lists,
                &mut Vec::new(),
            )
            .is_err()
        );

        let inline = concat!(
            "NFQWS_BASE_ARGS=\"--blob=inline:0x1234\"\n",
            "NFQWS_ARGS=\"--filter-tcp=443 --payload=tls_client_hello --lua-desync=fake:blob=inline\"\n",
        );
        let args = external_config_test_args(
            inline,
            Path::new("/tmp/config-check/auto.list"),
            &lua,
            &blobs,
            &lists,
            &mut Vec::new(),
        )
        .unwrap();
        assert!(
            args.iter()
                .any(|argument| argument == "--blob=inline:0x1234")
        );

        let offset = concat!(
            "NFQWS_BASE_ARGS=\"--blob=custom:16@/opt/zapret2/files/fake/tls.bin\"\n",
            "NFQWS_ARGS=\"--filter-tcp=443 --payload=tls_client_hello --lua-desync=fake:blob=custom\"\n",
        );
        let args = external_config_test_args(
            offset,
            Path::new("/tmp/config-check/auto.list"),
            &lua,
            &blobs,
            &lists,
            &mut Vec::new(),
        )
        .unwrap();
        assert!(args.iter().any(|argument| {
            argument
                == &format!(
                    "--blob=custom:16@{}",
                    blobs.join("tls.bin").canonicalize().unwrap().display()
                )
        }));
    }

    #[test]
    fn foreign_config_parser_rejects_dynamic_values_new_lua_files_and_missing_tcp_filter() {
        let directory = tempfile::tempdir().unwrap();
        let lists = directory.path().join("lists");
        let lua = directory.path().join("lua");
        std::fs::create_dir(&lua).unwrap();
        std::fs::create_dir(&lists).unwrap();
        for source in [
            "NFQWS_ARGS=\"--filter-tcp=443\"\nNFQWS_EXTRA_ARGS=\"$MODE_LIST\"\n",
            "NFQWS_BASE_ARGS=\"--lua-init=@/untrusted.lua\"\nNFQWS_ARGS=\"--filter-tcp=443\"\n",
            "NFQWS_ARGS=\"--filter-udp=443\"\n",
            "NFQWS_ARGS=\"--filter-tcp=443 --new\"\n",
        ] {
            assert!(
                external_config_test_args(
                    source,
                    Path::new("/tmp/config-check/auto.list"),
                    &lua,
                    &lists,
                    &lists,
                    &mut Vec::new()
                )
                .is_err(),
                "{source}"
            );
        }
    }

    #[test]
    fn foreign_config_list_paths_map_only_to_existing_files_in_the_installed_lists_directory() {
        let directory = tempfile::tempdir().unwrap();
        let lists = directory.path().join("lists");
        std::fs::create_dir(&lists).unwrap();
        let active_list = lists.join("user.list");
        std::fs::write(&active_list, "example.org\n").unwrap();

        for prefix in [
            "/opt/etc/nfqws2/lists/",
            "/etc/nfqws2/lists/",
            "/opt/etc/nfqws/lists/",
            "/etc/nfqws/lists/",
            "/opt/zapret2/ipset/",
            "/opt/zapret/ipset/",
        ] {
            assert_eq!(
                resolve_foreign_list_path(&format!("{prefix}user.list"), &lists).unwrap(),
                active_list.canonicalize().unwrap(),
                "{prefix}"
            );
        }
        assert!(resolve_foreign_list_path("/opt/zapret/ipset/missing.list", &lists).is_err());
        assert!(resolve_foreign_list_path("/tmp/user.list", &lists).is_err());
        assert!(resolve_foreign_list_path("../outside.list", &lists).is_err());
        #[cfg(unix)]
        {
            let outside = directory.path().join("outside.list");
            std::fs::write(&outside, "outside.example\n").unwrap();
            std::os::unix::fs::symlink(&outside, lists.join("linked.list")).unwrap();
            assert!(resolve_foreign_list_path("/opt/zapret/ipset/linked.list", &lists).is_err());
        }
    }

    #[tokio::test]
    async fn foreign_auto_list_is_seeded_into_private_temporary_copy() {
        let id = new_update_id();
        let directory = std::env::temp_dir().join(format!("nfqws2-ui-test-auto-{id}"));
        let source_dir = tempfile::tempdir().unwrap();
        let lists_dir = source_dir.path().join("lists");
        std::fs::create_dir(&lists_dir).unwrap();
        let source = lists_dir.join("auto.list");
        tokio::fs::write(&source, "example.org\n").await.unwrap();
        let auto_lists = vec![
            (directory.join("config-0.list"), Some(source)),
            (directory.join("config-1.list"), None),
        ];
        let mut cleanup = ForeignAutoListCleanup {
            path: directory.clone(),
            armed: false,
        };

        prepare_foreign_auto_lists(&directory, &auto_lists, &mut cleanup)
            .await
            .unwrap();
        assert_eq!(
            tokio::fs::read_to_string(directory.join("config-0.list"))
                .await
                .unwrap(),
            "example.org\n"
        );
        assert_eq!(
            tokio::fs::read_to_string(directory.join("config-1.list"))
                .await
                .unwrap(),
            ""
        );
        drop(cleanup);
        assert!(!directory.exists());
    }

    #[test]
    fn external_config_url_is_https_public_domain_without_credentials_or_custom_ports() {
        assert_eq!(
            parse_public_https_config_url(
                "https://raw.githubusercontent.com/user/repo/main/nfqws2.conf"
            )
            .unwrap(),
            (
                "raw.githubusercontent.com".into(),
                "https://raw.githubusercontent.com/user/repo/main/nfqws2.conf".into()
            )
        );
        assert_eq!(
            parse_public_https_config_url(
                "https://github.com/zemidala/nfqws2-ui/blob/main/nfqws2.conf"
            )
            .unwrap(),
            (
                "raw.githubusercontent.com".into(),
                "https://raw.githubusercontent.com/zemidala/nfqws2-ui/main/nfqws2.conf".into()
            )
        );
        for url in [
            "http://example.com/config.conf",
            "https://user:pass@example.com/config.conf",
            "https://example.com:8443/config.conf",
            "https://example.com./config.conf",
            "https://router.local/config.conf",
            "https://127.0.0.1/config.conf",
        ] {
            assert!(parse_public_https_config_url(url).is_err(), "{url}");
        }
        assert!(parse_public_https_config_url("https://github.com/user/repo").is_err());
    }

    #[test]
    fn insert_first_strategy_steps_preserves_shared_circular_and_shifts_each_number() {
        let steps = vec!["--lua-desync=new:pos=1".to_string()];
        let changed = insert_first_strategy_steps(
            "--filter-tcp=80,443 --payload=tls_client_hello --lua-desync=circular:fails=2 --lua-desync=old-a:strategy=1 --lua-desync=old-b:strategy=2",
            &steps,
        )
        .unwrap();
        assert_eq!(
            changed,
            "--filter-tcp=80,443\n--payload=tls_client_hello\n--lua-desync=circular:fails=2\n--lua-desync=new:pos=1:strategy=1\n--lua-desync=old-a:strategy=2\n--lua-desync=old-b:strategy=3"
        );
    }

    #[test]
    fn insert_first_strategy_steps_numbers_legacy_steps_and_rejects_ambiguous_input() {
        let steps = vec!["--lua-desync=new:pos=1".to_string()];
        assert_eq!(
            insert_first_strategy_steps("--lua-desync=old:pos=2", &steps).unwrap(),
            "--lua-desync=new:pos=1:strategy=1\n--lua-desync=old:pos=2:strategy=2"
        );
        assert!(insert_first_strategy_steps("--lua-desync=old:strategy=nope", &steps).is_err());
        assert!(insert_first_strategy_steps("--lua-desync=old:$MODE", &steps).is_err());
    }
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use tower::ServiceExt;

    #[test]
    fn appearance_settings_default_new_reminder_for_existing_router_files() {
        let settings: UiAppearanceSettings = serde_json::from_value(serde_json::json!({
            "theme": "dark",
            "layout": "standard"
        }))
        .unwrap();
        assert!(settings.remind_windows_tcp_timestamps);
        assert!(settings.show_config_warnings);
        let serialized = serde_json::to_value(settings).unwrap();
        assert_eq!(serialized["remindWindowsTcpTimestamps"], true);
        assert_eq!(serialized["accent"], "blue");
        assert_eq!(serialized["statusBar"], "standard");
        assert_eq!(serialized["showConfigWarnings"], true);
        assert!(serialized.get("simplifiedNavigation").is_none());
    }

    #[test]
    fn appearance_settings_keep_new_router_layout_and_palette_choices() {
        let settings: UiAppearanceSettings = serde_json::from_value(serde_json::json!({
            "theme": "light",
            "layout": "simplified",
            "accent": "cyan",
            "statusBar": "stacked",
            "remindWindowsTcpTimestamps": false,
            "showConfigWarnings": false,
            "simplifiedNavigation": true
        }))
        .unwrap();
        assert_eq!(settings.layout, "simplified");
        assert_eq!(settings.accent, "cyan");
        assert_eq!(settings.status_bar, "stacked");
        assert!(!settings.remind_windows_tcp_timestamps);
        assert!(!settings.show_config_warnings);
        let serialized = serde_json::to_value(settings).unwrap();
        assert!(serialized.get("simplifiedNavigation").is_none());
    }

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

    #[tokio::test]
    async fn monitoring_store_migrates_legacy_intervals_to_the_safe_range() {
        let dir = tempfile::tempdir().unwrap();
        let path = monitoring_store_path(dir.path());
        tokio::fs::write(
            path,
            serde_json::json!({
                "enabled": true,
                "intervalMinutes": 1,
                "lastAutoCheck": null,
                "sites": [],
                "telegram": {}
            })
            .to_string(),
        )
        .await
        .unwrap();

        let store = load_monitoring_store(dir.path()).await.unwrap();
        assert_eq!(store.interval_minutes, 10);
    }

    #[tokio::test]
    async fn telegram_transport_and_conditional_fields_survive_store_reload() {
        let dir = tempfile::tempdir().unwrap();
        let mut store = MonitoringStore::default();
        store.telegram.send_via = "proxy".into();
        store.telegram.interface = "eth0".into();
        store.telegram.proxy = "socks5h://127.0.0.1:1080".into();
        save_monitoring_store(dir.path(), &store).await.unwrap();

        let reloaded = load_monitoring_store(dir.path()).await.unwrap();
        assert_eq!(reloaded.telegram.send_via, "proxy");
        assert_eq!(reloaded.telegram.interface, "eth0");
        assert_eq!(reloaded.telegram.proxy, "socks5h://127.0.0.1:1080");
    }

    #[test]
    fn monitoring_accepts_podkop_fake_ipv4_range_but_rejects_other_reserved_addresses() {
        assert!(monitor_probe_ipv4("198.18.0.0".parse().unwrap()));
        assert!(monitor_probe_ipv4("198.19.255.255".parse().unwrap()));
        assert!(!monitor_probe_ipv4("198.51.100.1".parse().unwrap()));
        assert!(!monitor_probe_ipv4("127.0.0.1".parse().unwrap()));
    }

    #[test]
    fn podkop_detection_matches_only_the_198_18_15_fake_ip_range() {
        assert!(is_podkop_fake_ipv4("198.18.0.0".parse().unwrap()));
        assert!(is_podkop_fake_ipv4("198.18.255.255".parse().unwrap()));
        assert!(is_podkop_fake_ipv4("198.19.0.0".parse().unwrap()));
        assert!(is_podkop_fake_ipv4("198.19.255.255".parse().unwrap()));
        assert!(!is_podkop_fake_ipv4("198.17.255.255".parse().unwrap()));
        assert!(!is_podkop_fake_ipv4("198.20.0.0".parse().unwrap()));
        assert!(!is_podkop_fake_ipv4("198.51.100.1".parse().unwrap()));
        assert!(output_contains_podkop_fake_ip(
            b"Server: 127.0.0.42\nName: example.org\nAddress: 198.18.1.2\n"
        ));
        assert!(!output_contains_podkop_fake_ip(
            b"Server: 127.0.0.42\nName: example.org\nAddress: 203.0.113.2\n"
        ));
    }

    #[test]
    fn monitoring_interval_has_a_ten_minute_floor_like_zemidala() {
        assert!(!monitoring_interval_valid(1));
        assert!(!monitoring_interval_valid(9));
        assert!(monitoring_interval_valid(10));
        assert!(monitoring_interval_valid(30));
        assert!(monitoring_interval_valid(1440));
        assert!(!monitoring_interval_valid(1441));
    }

    #[test]
    fn monitoring_timestamp_is_committed_after_full_checks_but_not_per_site_checks() {
        let mut store = MonitoringStore {
            last_auto_check: Some(10),
            ..MonitoringStore::default()
        };

        record_monitoring_completion(&mut store, false, 20);
        assert_eq!(store.last_auto_check, Some(10));
        record_monitoring_completion(&mut store, true, 30);
        assert_eq!(store.last_auto_check, Some(30));
    }

    #[test]
    fn monitoring_groups_changes_from_one_cycle_into_one_telegram_message() {
        let mut store = MonitoringStore::default();
        store.telegram.chat_id = "123".into();
        queue_monitoring_changes(
            &mut store,
            vec![
                "❌ one.example перестал открываться".into(),
                "✅ two.example снова открывается".into(),
            ],
            42,
        );

        assert_eq!(store.telegram_notifications.queue.len(), 1);
        assert_eq!(
            store.telegram_notifications.queue[0].text,
            "nfqws2 на роутере:\n❌ one.example перестал открываться\n✅ two.example снова открывается"
        );
        assert_eq!(store.telegram_notifications.queue[0].timestamp, 42);
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
    fn panel_revision_release_is_available_after_its_base_version() {
        let releases = vec![
            PanelRelease {
                tag: "v1.3.5".into(),
                published_at: None,
                notes: String::new(),
            },
            PanelRelease {
                tag: "v1.3.5-r1".into(),
                published_at: None,
                notes: String::new(),
            },
        ];

        let update = ui_update_status("1.3.5", &releases, 123);
        assert_eq!(update.latest_version.as_deref(), Some("v1.3.5-r1"));
        assert!(update.update_available);
        assert!(!ui_update_status("1.3.5-r1", &releases, 456).update_available);
        assert!(valid_panel_release_tag("v1.3.5-r1"));
        assert!(!valid_panel_release_tag("1.3.6"));
        assert!(!valid_release_tag("v1.3.5-r1"));
        assert!(!valid_panel_release_tag("v1.3.5-r0"));
        assert!(!valid_panel_release_tag("v1.3.5-r01"));

        let stable_release = [PanelRelease {
            tag: "v1.3.6".into(),
            published_at: None,
            notes: String::new(),
        }];
        let update = ui_update_status("1.3.5-r1", &stable_release, 789);
        assert_eq!(update.latest_version.as_deref(), Some("v1.3.6"));
        assert!(update.update_available);
    }

    #[test]
    fn github_release_requests_try_entware_wget_before_path_wget() {
        assert_eq!(github_wget_candidates(), ["/opt/bin/wget", "wget"]);
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
            auth: Arc::new(auth::AuthManager::new(data_dir.clone())),
            data_dir,
            port: 1002,
            cpu_previous: Arc::new(Mutex::new(None)),
            config_write_lock: Mutex::new(()),
            config_override,
            update_lock: Mutex::new(()),
            ui_release_cache: Mutex::new(None),
            core_release_cache: Mutex::new(None),
            installed_core_version_cache: Mutex::new(InstalledCoreVersionCache::default()),
            keenetic_system_stats_cache: Mutex::new(KeeneticSystemStatsCache::default()),
            interception_status_cache: Mutex::new(InterceptionStatusCache::default()),
            ui_update_status: Arc::new(std::sync::RwLock::new(UiUpdateStatus::default())),
            site_diagnostic_lock: Arc::new(Mutex::new(())),
            dpi_lab_lock: Mutex::new(()),
            managed_asn_refresh_lock: Mutex::new(()),
            monitoring_lock: Arc::new(Mutex::new(())),
            auto_strategy_lock: Arc::new(Mutex::new(())),
            site_trace_controls: Mutex::new(HashMap::new()),
            site_trace_runner: Arc::new(router_site_trace_runner),
            foreign_config_controls: Mutex::new(HashMap::new()),
            strategy_pick_controls: Mutex::new(HashMap::new()),
            transfer_diagnostic_controls: Mutex::new(HashMap::new()),
            freeze_scan_controls: Mutex::new(HashMap::new()),
            nfqws_cli_catalog_cache: Mutex::new(None),
            nfqws_lua_catalog_cache: Mutex::new(None),
            change_history_fail_path: Mutex::new(None),
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
        let preview = build_freeze_strategy_preview(
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

    #[test]
    fn freeze_join_preview_requires_verified_asn_and_builds_scoped_config_diff() {
        let mut scan = confirmed_freeze_scan(freeze_diagnostic::FreezeTargetKind::UserHost);
        scan.results[0].address = Some("8.8.8.8".parse().unwrap());
        scan.results[0].asn = freeze_diagnostic::AsnVerification::Verified { asn: 15169 };
        let config = "NFQWS_ARGS_CUSTOM=\"--filter-tcp=443 --filter-l7=tls --payload=tls_client_hello --ipset=lists/ipset.list --lua-desync=hostfakesplit:repeats=4:tcp_ts=-600000:tcp_md5:host=ya.ru\"\nNFQWS_ARGS_UDP=\"\"\nNFQWS_ARGS_QUIC=\"\"\nNFQWS_ARGS_IPSET=\"\"\nNFQWS_ARGS=\"\"\nNFQWS_EXTRA_ARGS=\"\"\n";
        let lists = HashMap::from([("lists/ipset.list".to_owned(), "8.8.8.0/24\n".to_owned())]);
        let prefixes = vec!["8.8.8.0/24".to_owned()];
        let preview =
            build_freeze_profile_join_preview(&scan, 0, config, &lists, &prefixes).unwrap();
        assert_eq!(preview.asn, 15169);
        assert_eq!(preview.profile_index, 1);
        assert_eq!(preview.list_id, "ipset_as15169.list");
        assert!(
            preview
                .proposed_content
                .contains("--ipset=lists/ipset_as15169.list")
        );
        assert!(preview.original_content.contains("NFQWS_ARGS_CUSTOM"));

        scan.results[0].asn = freeze_diagnostic::AsnVerification::Unavailable;
        assert!(build_freeze_profile_join_preview(&scan, 0, config, &lists, &prefixes).is_err());

        scan.results[0].asn = freeze_diagnostic::AsnVerification::Verified { asn: 15169 };
        scan.results[0].found_strategy.as_mut().unwrap().reused_from =
            Some("another.example".into());
        assert!(build_freeze_profile_join_preview(&scan, 0, config, &lists, &prefixes).is_err());
    }

    #[tokio::test]
    async fn freeze_join_routes_reject_missing_scan_without_creating_config_or_list_files() {
        let dir = tempfile::tempdir().unwrap();
        let operation_id = "0123456789abcdef0123456789abcdef";
        let app = api_router(test_state(dir.path().to_path_buf()), dir.path().join("web"));
        for (uri, body) in [
            (
                "/api/nfqws2/diagnostics/freeze-scan/join-preview",
                serde_json::json!({"operationId": operation_id, "resultIndex": 0}),
            ),
            (
                "/api/nfqws2/diagnostics/freeze-scan/join",
                serde_json::json!({"operationId": operation_id, "resultIndex": 0, "expectedRevision": "stale"}),
            ),
        ] {
            let response = app
                .clone()
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri(uri)
                        .header("content-type", "application/json")
                        .body(Body::from(body.to_string()))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::NOT_FOUND);
        }
        assert!(!dir.path().join("freeze-diagnostic-operations").exists());
        assert!(!dir.path().join("nfqws2.conf").exists());
        assert!(!dir.path().join("lists/ipset_as15169.list").exists());
    }

    fn test_state_with_trace_runner(
        data_dir: PathBuf,
        runner: Arc<SiteTraceRunner>,
    ) -> Arc<AppState> {
        Arc::new(AppState {
            auth: Arc::new(auth::AuthManager::new(data_dir.clone())),
            data_dir,
            port: 1002,
            cpu_previous: Arc::new(Mutex::new(None)),
            config_write_lock: Mutex::new(()),
            config_override: None,
            update_lock: Mutex::new(()),
            ui_release_cache: Mutex::new(None),
            core_release_cache: Mutex::new(None),
            installed_core_version_cache: Mutex::new(InstalledCoreVersionCache::default()),
            keenetic_system_stats_cache: Mutex::new(KeeneticSystemStatsCache::default()),
            interception_status_cache: Mutex::new(InterceptionStatusCache::default()),
            ui_update_status: Arc::new(std::sync::RwLock::new(UiUpdateStatus::default())),
            site_diagnostic_lock: Arc::new(Mutex::new(())),
            dpi_lab_lock: Mutex::new(()),
            managed_asn_refresh_lock: Mutex::new(()),
            monitoring_lock: Arc::new(Mutex::new(())),
            auto_strategy_lock: Arc::new(Mutex::new(())),
            site_trace_controls: Mutex::new(HashMap::new()),
            site_trace_runner: runner,
            foreign_config_controls: Mutex::new(HashMap::new()),
            strategy_pick_controls: Mutex::new(HashMap::new()),
            transfer_diagnostic_controls: Mutex::new(HashMap::new()),
            freeze_scan_controls: Mutex::new(HashMap::new()),
            nfqws_cli_catalog_cache: Mutex::new(None),
            nfqws_lua_catalog_cache: Mutex::new(None),
            change_history_fail_path: Mutex::new(None),
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

    #[test]
    fn multi_strategy_hosts_require_two_to_fifteen_unique_domains() {
        let hosts = vec![" Example.com ".to_string(), "example.org".to_string()];
        assert_eq!(
            normalize_multi_strategy_hosts(&hosts).unwrap(),
            vec!["example.com", "example.org"]
        );
        assert!(normalize_multi_strategy_hosts(&["one.example".into()]).is_err());
        assert!(normalize_multi_strategy_hosts(&vec!["example.com".into(); 16]).is_err());
        assert!(
            normalize_multi_strategy_hosts(&["example.com".into(), "EXAMPLE.COM".into()]).is_err()
        );
        assert!(normalize_multi_strategy_hosts(&["1.1.1.1".into(), "example.com".into()]).is_err());
    }

    #[tokio::test]
    async fn multi_strategy_route_rejects_invalid_host_sets_before_scheduling() {
        let invalid_sets = vec![
            vec!["one.example".to_string()],
            vec!["example.com".to_string(); 16],
            vec!["example.com".to_string(), "EXAMPLE.COM".to_string()],
            vec!["1.1.1.1".to_string(), "example.com".to_string()],
        ];
        for hosts in invalid_sets {
            let dir = tempfile::tempdir().unwrap();
            let app = api_router(test_state(dir.path().join("data")), dir.path().join("web"));
            let response = app
                .oneshot(
                    Request::builder()
                        .method("POST")
                        .uri("/api/nfqws2/strategies/pick/multi")
                        .header("content-type", "application/json")
                        .body(Body::from(
                            serde_json::json!({"hosts": hosts, "protocol": "https"}).to_string(),
                        ))
                        .unwrap(),
                )
                .await
                .unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST);
            assert!(!dir.path().join("data/strategy-pick-operations").exists());
        }
        let dir = tempfile::tempdir().unwrap();
        let app = api_router(test_state(dir.path().join("data")), dir.path().join("web"));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/nfqws2/strategies/pick/multi")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"hosts": ["one.example", "two.example"], "protocol": "quic"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::BAD_REQUEST);
        assert!(!dir.path().join("data/strategy-pick-operations").exists());
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
            hosts: Vec::new(),
            podkop_hosts: Vec::new(),
            multi_site_results: Vec::new(),
            multi_site_baseline: Vec::new(),
            message: state.into(),
            updated_at: now_unix_seconds(),
        }
    }

    #[test]
    fn strategy_pick_history_without_podkop_field_remains_readable() {
        let mut legacy =
            serde_json::to_value(confirmed_strategy_pick("complete", "https", true)).unwrap();
        legacy.as_object_mut().unwrap().remove("podkopHosts");
        let restored: StrategyPickOperation = serde_json::from_value(legacy).unwrap();
        assert!(restored.podkop_hosts.is_empty());
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
        let original = "MODE_LIST='--hostlist=lists/user.list'\nNFQWS_BASE_ARGS=\"--lua-init=@/opt/etc/nfqws2/lua/zapret-lib.lua\"\nNFQWS_ARGS=\"--filter-tcp=80 --lua-desync=old:strategy=1\"\nNFQWS_ARGS_CUSTOM=\"--new --filter-tcp=80\" # custom\n";
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
                .contains("NFQWS_BASE_ARGS=\"--lua-init=@/opt/etc/nfqws2/lua/zapret-lib.lua\"")
        );
        assert!(preview.proposed_content.contains(
            "--filter-tcp=80\n--lua-desync=multisplit:pos=1:strategy=1\n--lua-desync=old:strategy=2"
        ));
        assert!(
            !preview
                .proposed_content
                .contains("--hostlist-domains=example.com")
        );
    }

    #[test]
    fn multi_site_preview_scopes_the_shared_candidate_only_to_confirmed_domains() {
        let mut operation = confirmed_strategy_pick("complete", "https", true);
        operation.host = "one.example, two.example, open.example".into();
        operation.hosts = vec![
            "one.example".into(),
            "two.example".into(),
            "open.example".into(),
        ];
        operation.multi_site_baseline = vec![
            MultiSiteBaselineResult {
                host: "one.example".into(),
                eligible: true,
                reason: "blocked".into(),
            },
            MultiSiteBaselineResult {
                host: "two.example".into(),
                eligible: true,
                reason: "blocked".into(),
            },
            MultiSiteBaselineResult {
                host: "open.example".into(),
                eligible: false,
                reason: "direct".into(),
            },
        ];
        operation.multi_site_results = vec![MultiSiteCandidateResult {
            candidate: operation.results[0].candidate.clone(),
            opened_domains: 2,
            domains: vec![
                MultiSiteDomainResult {
                    host: "one.example".into(),
                    eligible: true,
                    baseline_reason: String::new(),
                    attempts: 3,
                    successes: 3,
                    elapsed_ms: 110,
                    confirmed: true,
                },
                MultiSiteDomainResult {
                    host: "two.example".into(),
                    eligible: true,
                    baseline_reason: String::new(),
                    attempts: 3,
                    successes: 3,
                    elapsed_ms: 120,
                    confirmed: true,
                },
            ],
        }];
        let preview = build_strategy_pick_preview(
            &operation,
            0,
            "NFQWS_ARGS=\"--filter-tcp=443\"\nNFQWS_ARGS_CUSTOM=\"--new --filter-tcp=80\"\n",
            "rev-multi".into(),
            &HashSet::from(["multisplit".to_string()]),
            &[],
        )
        .unwrap();
        assert!(
            preview
                .proposed_content
                .contains("--hostlist-domains=one.example")
        );
        assert!(
            preview
                .proposed_content
                .contains("--hostlist-domains=two.example")
        );
        assert!(
            !preview
                .proposed_content
                .contains("--hostlist-domains=open.example")
        );
        assert!(preview.proposed_content.contains("NFQWS_ARGS_CUSTOM"));
    }

    #[test]
    fn strategy_preview_for_ipv4_keeps_shared_config_and_adds_first_strategy() {
        let mut operation = confirmed_strategy_pick("complete", "https", true);
        operation.host = "8.8.8.8".into();
        let preview = build_strategy_pick_preview(
            &operation,
            0,
            "NFQWS_BASE_ARGS=\"--blob=tls_clienthello:@/opt/etc/nfqws2/blobs/tls_clienthello.bin\"\nNFQWS_ARGS=\"--filter-tcp=443\"\n",
            "rev-ip".into(),
            &HashSet::from(["multisplit".to_string()]),
            &[],
        )
        .unwrap();
        assert!(
            preview
                .proposed_content
                .contains("--lua-desync=multisplit:pos=1:strategy=1")
        );
        assert!(!preview.proposed_content.contains("--hostlist-domains"));
        assert!(preview.proposed_content.contains(
            "NFQWS_BASE_ARGS=\"--blob=tls_clienthello:@/opt/etc/nfqws2/blobs/tls_clienthello.bin\""
        ));
        assert_eq!(preview.host, "8.8.8.8");
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
    fn strategy_dry_run_builds_a_udp_quic_candidate_profile() {
        let operation = confirmed_strategy_pick("complete", "quic", true);
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
        assert!(argv.iter().any(|argument| argument == "--filter-udp=443"));
        assert!(argv.iter().any(|argument| argument == "--filter-l7=quic"));
        assert!(
            argv.iter()
                .any(|argument| argument == "--payload=quic_initial")
        );
        assert!(argv.iter().any(|argument| argument == "--qnum=300"));
        assert!(!argv.iter().any(|argument| argument == "--filter-tcp=443"));
    }

    #[test]
    fn strategy_preview_rejects_incomplete_unconfirmed_and_out_of_range_results() {
        let original = "NFQWS_ARGS_CUSTOM=\"\"\n";
        let functions = HashSet::from(["multisplit".to_string()]);
        let revision = "rev".to_owned();
        for (operation, index) in [
            (confirmed_strategy_pick("complete", "https", false), 0),
            (confirmed_strategy_pick("running", "https", true), 0),
            (confirmed_strategy_pick("complete", "quic", false), 0),
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
    fn automatic_strategy_replaces_lua_steps_in_the_exact_existing_domain_profile() {
        let source = "NFQWS_ARGS_CUSTOM=\"--new --filter-tcp=443 --filter-l7=tls --hostlist-domains=example.com --payload=tls_client_hello --lua-desync=fake:strategy=1\"\n";
        let candidate = "--new --filter-tcp=443 --filter-l7=tls --hostlist-domains=example.com --payload=tls_client_hello --lua-desync=multisplit:pos=1";

        let (updated, replaced) = super::build_auto_strategy_config(source, candidate).unwrap();
        assert!(replaced);
        assert!(updated.contains("--hostlist-domains=example.com"));
        assert!(updated.contains("--lua-desync=multisplit:pos=1"));
        assert!(!updated.contains("--lua-desync=fake:strategy=1"));
        assert!(updated.contains("--payload=tls_client_hello"));
    }

    #[test]
    fn automatic_strategy_inserts_site_profile_before_existing_profiles() {
        let source = "NFQWS_ARGS_CUSTOM=\"--new --filter-tcp=443 --filter-l7=tls --lua-desync=fake:strategy=1\"\n";
        let candidate = "--new --filter-tcp=443 --filter-l7=tls --hostlist-domains=example.com --payload=tls_client_hello --lua-desync=multisplit:pos=1";

        let (updated, replaced) = super::build_auto_strategy_config(source, candidate).unwrap();
        assert!(!replaced);
        let profiles = nfqws2_ui::runtime_profiles::split_custom_profile_args(
            &super::ConfigDocument::parse(&updated)
                .unwrap()
                .literal_value("NFQWS_ARGS_CUSTOM")
                .unwrap(),
        )
        .unwrap();
        assert!(profiles[0].contains(&"--hostlist-domains=example.com".into()));
        assert!(profiles[1].contains(&"--lua-desync=fake:strategy=1".into()));
    }

    #[test]
    fn strategy_preview_applies_confirmed_quic_only_to_the_selected_domain() {
        let preview = build_strategy_pick_preview(
            &confirmed_strategy_pick("complete", "quic", true),
            0,
            "NFQWS_ARGS=\"--filter-tcp=443 --lua-desync=old:strategy=1\"\nNFQWS_ARGS_CUSTOM=\"\"\n",
            "rev-quic".into(),
            &HashSet::from(["multisplit".to_string()]),
            &[],
        )
        .unwrap();

        assert!(preview.proposed_content.contains("--filter-udp=443"));
        assert!(preview.proposed_content.contains("--filter-l7=quic"));
        assert!(preview.proposed_content.contains("--payload=quic_initial"));
        assert!(
            preview
                .proposed_content
                .contains("--hostlist-domains=example.com")
        );
        assert!(
            preview
                .proposed_content
                .contains("NFQWS_ARGS=\"--filter-tcp=443 --lua-desync=old:strategy=1\"")
        );
        assert!(preview.summary.contains("только для выбранного домена"));

        let mut duplicate = confirmed_strategy_pick("complete", "quic", true);
        duplicate.host = "8.8.8.8".into();
        assert!(
            build_strategy_pick_preview(
                &duplicate,
                0,
                "NFQWS_ARGS_CUSTOM=\"\"\n",
                "rev-ip-quic".into(),
                &HashSet::from(["multisplit".to_string()]),
                &[],
            )
            .is_err()
        );
    }

    #[test]
    fn strategy_preview_accepts_a_confirmed_result_from_cancelled_scan() {
        let preview = build_strategy_pick_preview(
            &confirmed_strategy_pick("cancelled", "https", true),
            0,
            "NFQWS_ARGS=\"--filter-tcp=443\"\n",
            "rev-cancelled".into(),
            &HashSet::from(["multisplit".to_string()]),
            &[],
        )
        .expect("a completed confirmed candidate remains usable after cancellation");

        assert!(preview.proposed_content.contains("strategy=1"));
        assert_eq!(preview.original_revision, "rev-cancelled");
    }

    #[test]
    fn strategy_preview_accepts_a_confirmed_result_from_partial_scan() {
        let preview = build_strategy_pick_preview(
            &confirmed_strategy_pick("partial", "https", true),
            0,
            "NFQWS_ARGS=\"--filter-tcp=443\"\n",
            "rev-partial".into(),
            &HashSet::from(["multisplit".to_string()]),
            &[],
        )
        .expect("a confirmed result remains usable when optional refinement was incomplete");

        assert!(preview.proposed_content.contains("strategy=1"));
    }

    #[test]
    fn strategy_preview_accepts_persisted_confirmed_result_when_scan_worker_failed_later() {
        let preview = build_strategy_pick_preview(
            &confirmed_strategy_pick("failed", "https", true),
            0,
            "NFQWS_ARGS=\"--filter-tcp=443\"\n",
            "rev-failed".into(),
            &HashSet::from(["multisplit".to_string()]),
            &[],
        )
        .expect("a worker failure does not revoke the separately confirmed candidate");

        assert!(preview.proposed_content.contains("strategy=1"));
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

    #[tokio::test]
    async fn package_leftovers_api_lists_and_deletes_only_server_discovered_candidates() {
        let dir = tempfile::tempdir().unwrap();
        let config_dir = dir.path().join("etc/nfqws2");
        let lists = config_dir.join("lists");
        std::fs::create_dir_all(&lists).unwrap();
        let config = config_dir.join("nfqws2.conf");
        std::fs::write(&config, b"active configuration").unwrap();
        std::fs::write(config_dir.join("nfqws2.conf-opkg"), b"package default").unwrap();
        std::fs::write(config_dir.join("custom.yaml-opkg"), b"extra").unwrap();
        std::fs::write(lists.join("user.list"), b"active list").unwrap();
        std::fs::write(lists.join("user.list.apk-new"), b"modified list").unwrap();
        let outside = dir.path().join("outside.list-opkg");
        std::fs::write(&outside, b"outside").unwrap();
        let app = api_router(
            test_state_with_config(dir.path().join("data"), Some(config.clone())),
            dir.path().join("web"),
        );

        let listed = app
            .clone()
            .oneshot(
                Request::get("/api/nfqws2/package-leftovers")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(listed.status(), StatusCode::OK);
        let listed = body_json(listed).await;
        assert_eq!(listed["items"].as_array().unwrap().len(), 3);
        assert_eq!(listed["totalBytes"], 33);
        assert_eq!(
            listed["items"]
                .as_array()
                .unwrap()
                .iter()
                .find(|item| item["name"] == "lists/user.list.apk-new")
                .unwrap()["sameAsBase"],
            false
        );

        let compared = app
            .clone()
            .oneshot(
                Request::post("/api/nfqws2/package-leftovers/compare")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({ "name": "lists/user.list.apk-new" }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(compared.status(), StatusCode::OK);
        let compared = body_json(compared).await;
        assert_eq!(compared["base"], "user.list");
        assert_eq!(compared["originalContent"], "active list");
        assert_eq!(compared["candidateContent"], "modified list");

        let deleted = app.oneshot(
            Request::post("/api/nfqws2/package-leftovers")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::json!({ "names": ["nfqws2.conf-opkg", "custom.yaml-opkg", "lists/user.list.apk-new", "../../outside.list-opkg", "nfqws2.conf"] }).to_string()))
                .unwrap(),
        ).await.unwrap();
        assert_eq!(deleted.status(), StatusCode::OK);
        let deleted = body_json(deleted).await;
        assert_eq!(deleted["deleted"], 3);
        assert_eq!(deleted["items"].as_array().unwrap().len(), 0);
        assert!(config.is_file());
        assert!(lists.join("user.list").is_file());
        assert!(outside.is_file());
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
    fn update_release_requests_load_only_the_ten_latest_releases() {
        assert!(GITHUB_RELEASES_API.ends_with("?per_page=10"));
        assert!(NFQWS2_RELEASES_API.ends_with("?per_page=10"));
    }

    #[test]
    fn update_release_catalogs_refresh_every_two_minutes_and_have_proxy_fallbacks() {
        let expected = std::time::Duration::from_secs(2 * 60);
        assert_eq!(UI_RELEASE_CACHE_TTL, expected);
        assert_eq!(CORE_RELEASE_CACHE_TTL, expected);

        let direct = "https://api.github.com/repos/example/project/releases?per_page=10";
        assert_eq!(
            github_release_api_candidates(direct),
            vec![
                direct.to_owned(),
                format!("https://gh-proxy.com/{direct}"),
                format!("https://ghfast.top/{direct}"),
            ]
        );
        assert_eq!(
            github_release_api_candidates("https://example.com/releases"),
            vec!["https://example.com/releases"]
        );
    }

    #[test]
    fn ui_release_cache_expires_after_two_minutes_and_backs_off_errors_for_ten_minutes() {
        let now = 2_000_000_000_u64;
        let entry = UiReleaseCacheEntry {
            checked_at: now - UI_RELEASE_CACHE_TTL.as_secs(),
            releases: Vec::new(),
            last_error_at: None,
            last_error: None,
        };
        assert!(!ui_release_cache_is_fresh(&entry, now));
        let error_entry = UiReleaseCacheEntry {
            last_error_at: Some(now - UI_RELEASE_ERROR_CACHE_TTL.as_secs() + 1),
            last_error: Some("GitHub unavailable".into()),
            ..entry
        };
        assert!(ui_release_cache_is_fresh(&error_entry, now));
        assert!(!ui_release_cache_is_fresh(
            &error_entry,
            now + UI_RELEASE_ERROR_CACHE_TTL.as_secs()
        ));
    }

    #[test]
    fn background_ui_release_check_waits_until_cache_expiry_or_retries_errors_early() {
        let now = 2_000_000_000_u64;
        assert_eq!(
            ui_release_check_retry_after(now - 60, None, now),
            std::time::Duration::from_secs(UI_RELEASE_CACHE_TTL.as_secs() - 60)
        );
        assert_eq!(
            ui_release_check_retry_after(now - 60, Some("stale"), now),
            UI_RELEASE_ERROR_CACHE_TTL
        );
    }

    #[tokio::test]
    async fn ui_release_cache_persists_the_catalog_and_error_backoff() {
        let dir = tempfile::tempdir().unwrap();
        let path = ui_release_cache_path(dir.path());
        let entry = UiReleaseCacheEntry {
            checked_at: 123,
            releases: vec![PanelRelease {
                tag: "v1.9.7".into(),
                published_at: None,
                notes: "release notes".into(),
            }],
            last_error_at: Some(124),
            last_error: Some("temporary outage".into()),
        };
        write_ui_release_cache(&path, &entry).await.unwrap();
        let cached = read_ui_release_cache(&path).await.unwrap();
        assert_eq!(cached.checked_at, 123);
        assert_eq!(cached.releases[0].tag, "v1.9.7");
        assert_eq!(cached.last_error.as_deref(), Some("temporary outage"));
    }

    #[test]
    fn release_catalog_keeps_only_stable_releases_with_package_and_checksum() {
        let data = serde_json::json!([
            {"tag_name":"v1.2.3","published_at":"2026-10-22T00:00:00Z","body":"<script>alert(1)</script>","draft":false,"prerelease":false,"assets":[{"name":"nfqws2-ui-v1.2.3-aarch64-unknown-linux-musl.tar.gz"},{"name":"nfqws2-ui-v1.2.3-aarch64-unknown-linux-musl.tar.gz.sha256"}]},
            {"tag_name":"v1.2.3-r1","published_at":"2026-10-23T00:00:00Z","body":"patch revision","draft":false,"prerelease":false,"assets":[{"name":"nfqws2-ui-v1.2.3-r1-aarch64-unknown-linux-musl.tar.gz"},{"name":"nfqws2-ui-v1.2.3-r1-aarch64-unknown-linux-musl.tar.gz.sha256"}]},
            {"tag_name":"1.3.6","name":"nfqws2-ui v1.3.6","draft":false,"prerelease":false,"assets":[{"name":"nfqws2-ui-v1.3.6-aarch64-unknown-linux-musl.tar.gz"},{"name":"nfqws2-ui-v1.3.6-aarch64-unknown-linux-musl.tar.gz.sha256"}]},
            {"tag_name":"v1.3.6","draft":false,"prerelease":false,"assets":[{"name":"nfqws2-ui-v1.3.6-aarch64-unknown-linux-musl.tar.gz"},{"name":"nfqws2-ui-v1.3.6-aarch64-unknown-linux-musl.tar.gz.sha256"}]},
            {"tag_name":"v1.2.4-rc1","draft":false,"prerelease":true,"assets":[{"name":"nfqws2-ui-v1.2.4-rc1-aarch64-unknown-linux-musl.tar.gz"},{"name":"nfqws2-ui-v1.2.4-rc1-aarch64-unknown-linux-musl.tar.gz.sha256"}]},
            {"tag_name":"v1.2.2","draft":false,"prerelease":false,"assets":[{"name":"nfqws2-ui-v1.2.2-aarch64-unknown-linux-musl.tar.gz"}]}
        ]);

        let releases = eligible_panel_releases(&data.to_string()).unwrap();

        assert_eq!(releases.len(), 3);
        assert_eq!(releases[0].tag, "v1.2.3");
        assert_eq!(releases[0].notes, "<script>alert(1)</script>");
        assert_eq!(releases[1].tag, "v1.2.3-r1");
        assert_eq!(releases[2].tag, "v1.3.6");
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

    #[test]
    fn core_update_status_normalizes_package_version_and_only_flags_newer_release() {
        assert!(core_update_available(Some("v1.3.3"), Some("1.3.2")));
        assert!(!core_update_available(Some("v1.3.2"), Some("1.3.2")));
        assert!(!core_update_available(Some("v1.3.2"), None));
        assert!(!core_update_available(None, Some("v1.3.2")));
    }

    #[test]
    fn core_release_status_cache_expires_after_two_minutes_and_is_architecture_scoped() {
        let now = 2_000_000_000_u64;
        let entry = CoreReleaseCacheEntry {
            checked_at: now - CORE_RELEASE_CACHE_TTL.as_secs(),
            architecture: "aarch64-3.10".into(),
            releases: Vec::new(),
            last_error_at: None,
        };
        assert!(core_release_cache_is_fresh(&entry, "aarch64-3.10", now - 1));
        assert!(!core_release_cache_is_fresh(&entry, "aarch64-3.10", now));
        assert!(!core_release_cache_is_fresh(
            &entry,
            "mips-3.4",
            now - CORE_RELEASE_CACHE_TTL.as_secs() + 1
        ));
    }

    #[test]
    fn core_release_list_uses_cache_by_default_and_bypasses_it_on_force_refresh() {
        let now = 2_000_000_000_u64;
        let entry = CoreReleaseCacheEntry {
            checked_at: now - 60,
            architecture: "aarch64-3.10".into(),
            releases: Vec::new(),
            last_error_at: None,
        };
        assert!(should_use_core_release_cache(
            &entry,
            "aarch64-3.10",
            now,
            false
        ));
        assert!(!should_use_core_release_cache(
            &entry,
            "aarch64-3.10",
            now,
            true
        ));
    }

    #[tokio::test]
    async fn core_release_catalog_returns_a_fresh_cached_list_without_fetching_github() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state(dir.path().to_path_buf());
        let checked_at = now_unix_seconds();
        *state.core_release_cache.lock().await = Some(CoreReleaseCacheEntry {
            checked_at,
            architecture: "aarch64-3.10".into(),
            releases: vec![CoreRelease {
                tag: "v1.9.7".into(),
                published_at: None,
                notes: "cached".into(),
                asset: "nfqws2-aarch64.tar.gz".into(),
                sha256: "a".repeat(64),
            }],
            last_error_at: None,
        });

        let (releases, returned_checked_at, warning) =
            load_cached_core_releases(&state, "aarch64-3.10", false)
                .await
                .unwrap();

        assert_eq!(releases[0].tag, "v1.9.7");
        assert_eq!(returned_checked_at, checked_at);
        assert!(warning.is_none());
    }

    #[test]
    fn core_release_query_defaults_to_cached_mode() {
        assert!(!CoreReleaseQuery::default().force);
    }

    #[test]
    fn core_release_error_cache_backs_off_for_ten_minutes() {
        let now = 2_000_000_000_u64;
        let entry = CoreReleaseCacheEntry {
            checked_at: 0,
            architecture: "aarch64-3.10".into(),
            releases: Vec::new(),
            last_error_at: Some(now),
        };
        assert!(core_release_cache_is_fresh(
            &entry,
            "aarch64-3.10",
            now + 599
        ));
        assert!(!core_release_cache_is_fresh(
            &entry,
            "aarch64-3.10",
            now + 600
        ));
    }

    #[tokio::test]
    async fn core_release_cache_persists_and_reads_entries() {
        let dir = tempfile::tempdir().unwrap();
        let path = core_release_cache_path(dir.path());
        let entry = CoreReleaseCacheEntry {
            checked_at: 1_900_000_000,
            architecture: "aarch64-3.10".into(),
            releases: vec![CoreRelease {
                tag: "v1.3.6".into(),
                published_at: Some("2026-10-27T00:00:00Z".into()),
                notes: "release notes".into(),
                asset: "nfqws2-aarch64.tar.gz".into(),
                sha256: "a".repeat(64),
            }],
            last_error_at: Some(1_900_000_001),
        };
        write_core_release_cache(&path, &entry).await.unwrap();
        let actual = read_core_release_cache(&path).await.unwrap();
        assert_eq!(actual.checked_at, entry.checked_at);
        assert_eq!(actual.architecture, entry.architecture);
        assert_eq!(actual.releases[0].tag, entry.releases[0].tag);
        assert_eq!(actual.last_error_at, entry.last_error_at);
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
        let status = body_json(response).await;
        assert!(status.get("interception").is_some());
    }

    #[test]
    fn installed_core_version_cache_reuses_recent_values_and_expires_them() {
        let mut cache = InstalledCoreVersionCache::default();
        let checked = Instant::now();
        assert_eq!(cache.get(checked), None);

        cache.store(Some("v1.2.3".into()), checked);
        assert_eq!(
            cache.get(checked + std::time::Duration::from_secs(4)),
            Some(Some("v1.2.3".into()))
        );
        assert_eq!(cache.get(checked + std::time::Duration::from_secs(5)), None);
    }

    #[test]
    fn installed_core_version_cache_can_reuse_a_missing_package_result_briefly() {
        let mut cache = InstalledCoreVersionCache::default();
        let checked = Instant::now();
        cache.store(None, checked);

        assert_eq!(
            cache.get(checked + std::time::Duration::from_secs(1)),
            Some(None)
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
    async fn config_apply_persists_a_recovery_journal_until_health_verification_finishes() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let original = b"NFQWS_ARGS=--filter-tcp=443\n";
        let replacement = "NFQWS_ARGS=--filter-tcp=80,443\n";
        tokio::fs::write(&config, original).await.unwrap();
        let state = test_state_with_config(dir.path().join("state"), Some(config.clone()));
        let journal_path = state.data_dir.join("pending-config-apply.json");
        let journal_was_present = Arc::new(AtomicBool::new(false));
        let observed = journal_was_present.clone();
        let journal_seen_by_restart = journal_path.clone();

        apply_config_transaction_with_verification(
            &state,
            ConfigWrite {
                content: replacement.into(),
                revision: revision(original),
            },
            move || {
                let path = journal_seen_by_restart.clone();
                let observed = observed.clone();
                async move {
                    observed.store(tokio::fs::metadata(path).await.is_ok(), Ordering::SeqCst);
                    Ok("restart confirmed".to_string())
                }
            },
            || async { Ok(()) },
        )
        .await
        .unwrap();

        assert!(journal_was_present.load(Ordering::SeqCst));
        assert!(
            !journal_path.exists(),
            "completed transactions clear their recovery journal"
        );
    }

    #[tokio::test]
    async fn startup_recovery_restores_a_candidate_left_by_an_interrupted_config_apply() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let original = b"NFQWS_ARGS=--filter-tcp=443\n";
        let candidate = b"NFQWS_ARGS=--filter-tcp=80,443\n";
        tokio::fs::write(&config, candidate).await.unwrap();
        let state = test_state_with_config(dir.path().join("state"), Some(config.clone()));
        write_pending_config_apply(
            &state.data_dir,
            &PendingConfigApplyJournal {
                config_path: config.to_string_lossy().into_owned(),
                original_content: original.to_vec(),
                candidate_revision: revision(candidate),
            },
        )
        .await
        .unwrap();
        let restart_count = Arc::new(AtomicUsize::new(0));
        let restart_counter = restart_count.clone();

        recover_pending_config_apply_with(&state, move || {
            restart_counter.fetch_add(1, Ordering::SeqCst);
            async { Ok("service restarted".to_string()) }
        })
        .await
        .unwrap();

        assert_eq!(tokio::fs::read(config).await.unwrap(), original);
        assert_eq!(restart_count.load(Ordering::SeqCst), 1);
        assert!(!pending_config_apply_path(&state.data_dir).exists());
    }

    #[tokio::test]
    async fn startup_recovery_preserves_config_changed_outside_the_pending_transaction() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let original = b"NFQWS_ARGS=--filter-tcp=443\n";
        let candidate = b"NFQWS_ARGS=--filter-tcp=80,443\n";
        let external = b"NFQWS_ARGS=--filter-tcp=80,443,8443\n";
        tokio::fs::write(&config, external).await.unwrap();
        let state = test_state_with_config(dir.path().join("state"), Some(config.clone()));
        write_pending_config_apply(
            &state.data_dir,
            &PendingConfigApplyJournal {
                config_path: config.to_string_lossy().into_owned(),
                original_content: original.to_vec(),
                candidate_revision: revision(candidate),
            },
        )
        .await
        .unwrap();
        let restart_count = Arc::new(AtomicUsize::new(0));
        let restart_counter = restart_count.clone();

        let result = recover_pending_config_apply_with(&state, move || {
            restart_counter.fetch_add(1, Ordering::SeqCst);
            async { Ok("service restarted".to_string()) }
        })
        .await;

        assert!(result.unwrap_err().contains("отличается и от исходного"));
        assert_eq!(tokio::fs::read(config).await.unwrap(), external);
        assert_eq!(restart_count.load(Ordering::SeqCst), 0);
        assert!(pending_config_apply_path(&state.data_dir).exists());
    }

    #[tokio::test]
    async fn config_apply_rolls_back_when_post_restart_health_check_fails() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let original = b"NFQWS_ARGS=--filter-tcp=443\n";
        let replacement = "NFQWS_ARGS=--filter-tcp=80,443\n";
        tokio::fs::write(&config, original).await.unwrap();
        let state = test_state_with_config(dir.path().join("state"), Some(config.clone()));
        let restart_count = Arc::new(AtomicUsize::new(0));
        let restart_counter = restart_count.clone();

        let result = apply_config_transaction_with_verification(
            &state,
            ConfigWrite {
                content: replacement.into(),
                revision: revision(original),
            },
            move || {
                restart_counter.fetch_add(1, Ordering::SeqCst);
                async { Ok("service restarted".to_string()) }
            },
            || async { Err("target site remained unavailable".to_string()) },
        )
        .await;

        let error = result.unwrap_err();
        assert!(error.contains("target site remained unavailable"));
        assert!(error.contains("Исходный конфиг восстановлен и служба перезапущена"));
        assert_eq!(restart_count.load(Ordering::SeqCst), 2);
        assert_eq!(tokio::fs::read(config).await.unwrap(), original);
    }

    #[tokio::test]
    async fn freeze_join_transaction_rolls_back_config_and_removes_only_its_new_list() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let original = b"NFQWS_ARGS=--filter-tcp=443\n";
        let replacement = "NFQWS_ARGS=--filter-tcp=80,443\n";
        tokio::fs::write(&config, original).await.unwrap();
        let state = test_state_with_config(dir.path().join("state"), Some(config.clone()));
        let list = dir.path().join("ipset_as15169.list");
        let sibling = dir.path().join("ipset_as64500.list");
        let list_content = "# AS15169 managed by nfqws2-ui via RIPEstat\n8.8.8.0/24\n";
        tokio::fs::write(&list, list_content).await.unwrap();
        let mut managed = ManagedAsnListStore::default();
        managed.register(ManagedAsnList::new(
            15169,
            "ipset_as15169.list",
            1,
            list_content.as_bytes(),
        ));
        managed.save(&state.data_dir).unwrap();
        tokio::fs::write(&sibling, "192.0.2.0/24\n").await.unwrap();
        let restart_count = Arc::new(AtomicUsize::new(0));
        let restart_counter = restart_count.clone();

        let result = apply_freeze_join_transaction(
            &state,
            ConfigWrite {
                content: replacement.into(),
                revision: revision(original),
            },
            &list,
            &state.data_dir,
            15169,
            move || {
                let attempt = restart_counter.fetch_add(1, Ordering::SeqCst);
                async move {
                    if attempt == 0 {
                        Err("candidate restart failed".to_string())
                    } else {
                        Ok("restored service restarted".to_string())
                    }
                }
            },
            || async { Ok(()) },
        )
        .await;

        let error = result.unwrap_err();
        assert!(error.contains("Исходный конфиг восстановлен"));
        assert_eq!(restart_count.load(Ordering::SeqCst), 2);
        assert_eq!(tokio::fs::read(&config).await.unwrap(), original);
        assert!(!list.exists(), "only the newly-created ASN list is removed");
        assert_eq!(
            tokio::fs::read_to_string(sibling).await.unwrap(),
            "192.0.2.0/24\n"
        );
        assert!(
            ManagedAsnListStore::load(&state.data_dir)
                .entries
                .is_empty()
        );
    }

    #[tokio::test]
    async fn freeze_join_transaction_keeps_registered_list_after_verified_apply() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let original = b"NFQWS_ARGS=--filter-tcp=443\n";
        let replacement = "NFQWS_ARGS=--filter-tcp=80,443\n";
        tokio::fs::write(&config, original).await.unwrap();
        let state = test_state_with_config(dir.path().join("state"), Some(config.clone()));
        let list = dir.path().join("ipset_as15169.list");
        let list_content = "# AS15169 managed by nfqws2-ui via RIPEstat\n8.8.8.0/24\n";
        tokio::fs::write(&list, list_content).await.unwrap();
        let mut managed = ManagedAsnListStore::default();
        managed.register(ManagedAsnList::new(
            15169,
            "ipset_as15169.list",
            1,
            list_content.as_bytes(),
        ));
        managed.save(&state.data_dir).unwrap();
        let restart_count = Arc::new(AtomicUsize::new(0));
        let restart_counter = restart_count.clone();
        let verify_count = Arc::new(AtomicUsize::new(0));
        let verify_counter = verify_count.clone();

        let applied = apply_freeze_join_transaction(
            &state,
            ConfigWrite {
                content: replacement.into(),
                revision: revision(original),
            },
            &list,
            &state.data_dir,
            15169,
            move || {
                restart_counter.fetch_add(1, Ordering::SeqCst);
                async { Ok("service restarted".to_string()) }
            },
            move || {
                verify_counter.fetch_add(1, Ordering::SeqCst);
                async { Ok(()) }
            },
        )
        .await
        .unwrap();

        assert!(applied.message.contains("service restarted"));
        assert_eq!(restart_count.load(Ordering::SeqCst), 1);
        assert_eq!(verify_count.load(Ordering::SeqCst), 1);
        assert_eq!(
            tokio::fs::read(&config).await.unwrap(),
            replacement.as_bytes()
        );
        assert!(list.exists());
        assert_eq!(
            ManagedAsnListStore::load(&state.data_dir)
                .entries
                .iter()
                .map(|entry| entry.asn)
                .collect::<Vec<_>>(),
            vec![15169]
        );
    }

    #[tokio::test]
    async fn freeze_join_transaction_conflict_removes_list_without_restart_or_config_write() {
        use std::sync::atomic::{AtomicBool, Ordering};

        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let original = b"NFQWS_ARGS=--filter-tcp=443\n";
        tokio::fs::write(&config, original).await.unwrap();
        let state = test_state_with_config(dir.path().join("state"), Some(config.clone()));
        let list = dir.path().join("ipset_as15169.list");
        tokio::fs::write(&list, "8.8.8.0/24\n").await.unwrap();
        let restarted = Arc::new(AtomicBool::new(false));
        let restart_flag = restarted.clone();

        let result = apply_freeze_join_transaction(
            &state,
            ConfigWrite {
                content: "NFQWS_ARGS=--filter-tcp=80,443\n".into(),
                revision: "stale-revision".into(),
            },
            &list,
            &state.data_dir,
            15169,
            move || {
                restart_flag.store(true, Ordering::SeqCst);
                async { Ok("unexpected restart".to_string()) }
            },
            || async { Ok(()) },
        )
        .await;

        assert!(
            result
                .unwrap_err()
                .contains("Файл изменился после загрузки")
        );
        assert!(!restarted.load(Ordering::SeqCst));
        assert_eq!(tokio::fs::read(&config).await.unwrap(), original);
        assert!(!list.exists());
    }

    #[test]
    fn strategy_apply_verifies_target_once_and_only_previously_healthy_peers() {
        let store = MonitoringStore {
            sites: vec![
                MonitoredSite {
                    host: "target.example".into(),
                    results: vec![MonitoredResult {
                        checked_at: 3,
                        status: "ok".into(),
                        summary: "ok".into(),
                        latency_ms: Some(100),
                        steps: vec![],
                    }],
                },
                MonitoredSite {
                    host: "healthy.example".into(),
                    results: vec![MonitoredResult {
                        checked_at: 3,
                        status: "ok".into(),
                        summary: "ok".into(),
                        latency_ms: Some(100),
                        steps: vec![],
                    }],
                },
                MonitoredSite {
                    host: "already-down.example".into(),
                    results: vec![MonitoredResult {
                        checked_at: 3,
                        status: "error".into(),
                        summary: "down".into(),
                        latency_ms: None,
                        steps: vec![],
                    }],
                },
            ],
            ..MonitoringStore::default()
        };

        assert_eq!(
            strategy_apply_verification_hosts("target.example", &store),
            vec!["target.example", "healthy.example"]
        );
        assert_eq!(
            strategy_apply_verification_hosts_for_protocol("target.example", "quic", &store),
            vec!["target.example"]
        );
        assert_eq!(
            strategy_apply_verification_hosts_for_protocol("target.example", "https", &store),
            vec!["target.example", "healthy.example"]
        );
    }

    #[test]
    fn config_apply_verification_hosts_use_only_recent_successful_monitor_baselines() {
        let now = 2_000_000_000_u64;
        let result = |checked_at, status: &str| MonitoredResult {
            checked_at,
            status: status.into(),
            summary: String::new(),
            latency_ms: None,
            steps: Vec::new(),
        };
        let monitoring = MonitoringStore {
            sites: vec![
                MonitoredSite {
                    host: "healthy.example".into(),
                    results: vec![result(now - 60, "ok")],
                },
                MonitoredSite {
                    host: "failed.example".into(),
                    results: vec![result(now - 60, "error")],
                },
                MonitoredSite {
                    host: "stale.example".into(),
                    results: vec![result(
                        now - CONFIG_APPLY_MONITOR_BASELINE_TTL.as_secs() - 1,
                        "ok",
                    )],
                },
                MonitoredSite {
                    host: "future.example".into(),
                    results: vec![result(now + 60, "ok")],
                },
            ],
            ..MonitoringStore::default()
        };
        assert_eq!(
            config_apply_verification_hosts(&monitoring, now),
            ["healthy.example"]
        );
    }

    #[tokio::test]
    async fn strategy_apply_retries_only_failed_host_checks_once() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let calls = Arc::new(AtomicUsize::new(0));
        let probe_calls = calls.clone();
        let result = verify_strategy_hosts_with(vec!["target.example".into()], move |_| {
            let calls = probe_calls.clone();
            async move {
                if calls.fetch_add(1, Ordering::SeqCst) == 0 {
                    Err("temporary failure".to_string())
                } else {
                    Ok(())
                }
            }
        })
        .await;

        assert_eq!(result, Ok(()));
        assert_eq!(calls.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn strategy_apply_reports_hosts_that_still_fail_after_retry() {
        let result = verify_strategy_hosts_with(vec!["broken.example".into()], |_| async {
            Err("HTTP response missing".to_string())
        })
        .await;

        assert_eq!(
            result,
            Err("broken.example: HTTP response missing".to_string())
        );
    }

    #[tokio::test]
    async fn strategy_apply_limits_concurrent_network_probes_to_three() {
        use std::sync::atomic::{AtomicUsize, Ordering};

        let active = Arc::new(AtomicUsize::new(0));
        let maximum = Arc::new(AtomicUsize::new(0));
        let probe = {
            let active = active.clone();
            let maximum = maximum.clone();
            move |_| {
                let active = active.clone();
                let maximum = maximum.clone();
                async move {
                    let now = active.fetch_add(1, Ordering::SeqCst) + 1;
                    maximum.fetch_max(now, Ordering::SeqCst);
                    tokio::time::sleep(std::time::Duration::from_millis(20)).await;
                    active.fetch_sub(1, Ordering::SeqCst);
                    Ok(())
                }
            }
        };

        let hosts = (0..9)
            .map(|index| format!("site-{index}.example"))
            .collect();
        assert_eq!(verify_strategy_hosts_with(hosts, probe).await, Ok(()));
        assert!(maximum.load(Ordering::SeqCst) <= 3);
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
        let expected_platform = nfqws2_ui::discover_system_paths().map_or("unknown", |paths| {
            if paths.root == Path::new("/opt") {
                "entware"
            } else {
                "openwrt"
            }
        });
        assert_eq!(report["platform"], expected_platform);
        assert!(report["networkInterfaces"].is_array());
        assert!(report["defaultNetworkInterfaces"].is_array());
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
    async fn change_history_endpoint_scans_external_edits_and_revision_checks_list_revert() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let lists = dir.path().join("lists");
        tokio::fs::create_dir_all(&lists).await.unwrap();
        tokio::fs::write(&config, "NFQWS_ARGS=--filter-tcp=443\n")
            .await
            .unwrap();
        let list = lists.join("user.list");
        tokio::fs::write(&list, "one.example\n").await.unwrap();
        let state = test_state_with_config(dir.path().join("state"), Some(config));
        let app = api_router(state, dir.path().join("web"));

        let baseline = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/change-history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            body_json(baseline).await["items"]
                .as_array()
                .unwrap()
                .is_empty()
        );

        tokio::fs::write(&list, "one.example\ntwo.example\n")
            .await
            .unwrap();
        let scanned = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/change-history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let items = body_json(scanned).await["items"]
            .as_array()
            .unwrap()
            .clone();
        let change = items
            .iter()
            .find(|item| item["file"] == "user.list")
            .unwrap();
        assert_eq!(change["source"], "вне интерфейса");
        assert_eq!(change["addedLines"], 1);
        let id = change["id"].as_str().unwrap();

        let diff = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/change-history/{id}/diff"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(body_json(diff).await["from"], "one.example\n");

        tokio::fs::write(&list, "external-newer.example\n")
            .await
            .unwrap();
        let conflict = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/change-history/{id}/revert"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(conflict.status(), StatusCode::CONFLICT);
        assert_eq!(
            tokio::fs::read_to_string(&list).await.unwrap(),
            "external-newer.example\n"
        );
        tokio::fs::write(&list, "one.example\ntwo.example\n")
            .await
            .unwrap();

        let reverted = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri(format!("/api/change-history/{id}/revert"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(reverted.status(), StatusCode::OK);
        assert_eq!(
            tokio::fs::read_to_string(list).await.unwrap(),
            "one.example\n"
        );
    }

    #[tokio::test]
    async fn change_history_initial_restore_preserves_files_created_after_baseline() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let lists = dir.path().join("lists");
        tokio::fs::create_dir_all(&lists).await.unwrap();
        tokio::fs::write(&config, "NFQWS_ARGS=--filter-tcp=443\n")
            .await
            .unwrap();
        tokio::fs::write(lists.join("user.list"), "one.example\n")
            .await
            .unwrap();
        let state = test_state_with_config(dir.path().join("state"), Some(config));
        let app = api_router(state, dir.path().join("web"));
        let baseline = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/change-history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(
            body_json(baseline).await["items"]
                .as_array()
                .unwrap()
                .is_empty()
        );

        tokio::fs::write(lists.join("user.list"), "changed.example\n")
            .await
            .unwrap();
        tokio::fs::write(lists.join("new.list"), "new.example\n")
            .await
            .unwrap();
        let scanned = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/change-history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(body_json(scanned).await["canRestoreInitial"], true);

        let restored = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/change-history/initial")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(restored.status(), StatusCode::OK);
        let response_body = body_json(restored).await;
        assert!(
            response_body["message"]
                .as_str()
                .unwrap()
                .contains("восстановлено")
        );
        assert_eq!(
            tokio::fs::read_to_string(lists.join("user.list"))
                .await
                .unwrap(),
            "one.example\n"
        );
        assert_eq!(
            tokio::fs::read_to_string(lists.join("new.list"))
                .await
                .unwrap(),
            "new.example\n"
        );
        assert!(
            response_body["changed"]
                .as_array()
                .unwrap()
                .iter()
                .any(|path| {
                    path.as_str()
                        .is_some_and(|value| value.ends_with("/lists/user.list"))
                })
        );
    }

    #[tokio::test]
    async fn change_history_undo_last_preserves_unconnected_standard_list_format() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let lists = dir.path().join("lists");
        tokio::fs::create_dir_all(&lists).await.unwrap();
        tokio::fs::write(&config, "NFQWS_ARGS=\"\"\n")
            .await
            .unwrap();
        let list = lists.join("user.list");
        tokio::fs::write(&list, "+example.org\n").await.unwrap();
        let app = api_router(
            test_state_with_config(dir.path().join("state"), Some(config)),
            dir.path().join("web"),
        );
        let baseline = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/change-history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(baseline.status(), StatusCode::OK);
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
        let revision = body_json(loaded).await["revision"]
            .as_str()
            .unwrap()
            .to_owned();
        let saved = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/nfqws2/lists/user.list")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"content":"example.org\n","revision":"{revision}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(saved.status(), StatusCode::OK);
        let undo = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/change-history/undo-last")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(undo.status(), StatusCode::OK);
        assert_eq!(
            tokio::fs::read_to_string(list).await.unwrap(),
            "+example.org\n"
        );
    }

    #[tokio::test]
    async fn change_history_undo_keeps_validation_when_target_config_connects_list() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let lists = dir.path().join("lists");
        tokio::fs::create_dir_all(&lists).await.unwrap();
        tokio::fs::write(&config, "NFQWS_ARGS=--hostlist=lists/user.list\n")
            .await
            .unwrap();
        let list = lists.join("user.list");
        tokio::fs::write(&list, "+example.org\n").await.unwrap();
        let state_dir = dir.path().join("state");
        let app = api_router(
            test_state_with_config(state_dir.clone(), Some(config.clone())),
            dir.path().join("web"),
        );
        let baseline = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/change-history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(baseline.status(), StatusCode::OK);
        tokio::fs::write(&config, "NFQWS_ARGS=\"\"\n")
            .await
            .unwrap();
        tokio::fs::write(&list, "example.org\n").await.unwrap();
        let events = change_history::scan(
            &state_dir,
            &config,
            "интерфейс",
            "список отключён вместе с настройкой",
        )
        .unwrap();
        assert_eq!(events.len(), 2);

        let undo = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/change-history/undo-last")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(undo.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            tokio::fs::read_to_string(&config).await.unwrap(),
            "NFQWS_ARGS=\"\"\n"
        );
        assert_eq!(
            tokio::fs::read_to_string(&list).await.unwrap(),
            "example.org\n"
        );
    }

    #[tokio::test]
    async fn change_history_undo_keeps_validation_when_current_config_connects_list() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let lists = dir.path().join("lists");
        tokio::fs::create_dir_all(&lists).await.unwrap();
        tokio::fs::write(&config, "NFQWS_ARGS=--hostlist=lists/user.list\n")
            .await
            .unwrap();
        let list = lists.join("user.list");
        tokio::fs::write(&list, "+example.org\n").await.unwrap();
        let state_dir = dir.path().join("state");
        let app = api_router(
            test_state_with_config(state_dir.clone(), Some(config.clone())),
            dir.path().join("web"),
        );
        let baseline = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/change-history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(baseline.status(), StatusCode::OK);
        tokio::fs::write(&list, "example.org\n").await.unwrap();
        change_history::scan(
            &state_dir,
            &config,
            "интерфейс",
            "исправлена подключённая запись",
        )
        .unwrap();

        let undo = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/change-history/undo-last")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(undo.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            tokio::fs::read_to_string(&list).await.unwrap(),
            "example.org\n"
        );
    }

    #[tokio::test]
    async fn change_history_undo_last_restores_all_lists_in_the_latest_ui_batch_once() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let lists = dir.path().join("lists");
        tokio::fs::create_dir_all(&lists).await.unwrap();
        tokio::fs::write(&config, "NFQWS_ARGS=--filter-tcp=443\n")
            .await
            .unwrap();
        let first = lists.join("first.list");
        let second = lists.join("second.list");
        tokio::fs::write(&first, "before-first.example\n")
            .await
            .unwrap();
        tokio::fs::write(&second, "before-second.example\n")
            .await
            .unwrap();
        let state_dir = dir.path().join("state");
        let state = test_state_with_config(state_dir.clone(), Some(config.clone()));
        let app = api_router(state, dir.path().join("web"));

        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/change-history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        tokio::fs::write(&first, "after-first.example\n")
            .await
            .unwrap();
        tokio::fs::write(&second, "after-second.example\n")
            .await
            .unwrap();
        let events =
            change_history::scan(&state_dir, &config, "интерфейс", "двойное изменение").unwrap();
        assert_eq!(events.len(), 2);
        assert_eq!(events[0].batch, events[1].batch);

        let before = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/change-history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(body_json(before).await["canUndoLast"], true);
        let undone = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/change-history/undo-last")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(undone.status(), StatusCode::OK);
        assert_eq!(
            tokio::fs::read_to_string(first).await.unwrap(),
            "before-first.example\n"
        );
        assert_eq!(
            tokio::fs::read_to_string(second).await.unwrap(),
            "before-second.example\n"
        );
        let after = app
            .oneshot(
                Request::builder()
                    .uri("/api/change-history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(body_json(after).await["canUndoLast"], false);
    }

    #[tokio::test]
    async fn change_history_undo_last_validates_all_list_versions_before_restoring_any() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let lists = dir.path().join("lists");
        tokio::fs::create_dir_all(&lists).await.unwrap();
        tokio::fs::write(&config, "NFQWS_ARGS=--filter-tcp=443\n")
            .await
            .unwrap();
        let first = lists.join("first.list");
        let second = lists.join("second.list");
        tokio::fs::write(&first, "before-first.example\n")
            .await
            .unwrap();
        // Existing list content may have been edited outside the UI and is
        // captured by history even when it would not pass the UI validator.
        tokio::fs::write(&second, "%%% invalid previous value\n")
            .await
            .unwrap();
        let state_dir = dir.path().join("state");
        let state = test_state_with_config(state_dir.clone(), Some(config.clone()));
        let app = api_router(state, dir.path().join("web"));

        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/change-history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        tokio::fs::write(&first, "after-first.example\n")
            .await
            .unwrap();
        tokio::fs::write(&second, "after-second.example\n")
            .await
            .unwrap();
        change_history::scan(&state_dir, &config, "интерфейс", "двойное изменение").unwrap();

        let undone = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/change-history/undo-last")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(undone.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            tokio::fs::read_to_string(first).await.unwrap(),
            "after-first.example\n",
            "a failed batch undo must not leave earlier files reverted"
        );
        assert_eq!(
            tokio::fs::read_to_string(second).await.unwrap(),
            "after-second.example\n"
        );
        let history = app
            .oneshot(
                Request::builder()
                    .uri("/api/change-history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(body_json(history).await["canUndoLast"], true);
    }

    #[tokio::test]
    async fn change_history_undo_last_restores_the_full_batch_if_a_file_write_fails() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let lists = dir.path().join("lists");
        tokio::fs::create_dir_all(&lists).await.unwrap();
        tokio::fs::write(&config, "NFQWS_ARGS=--filter-tcp=443\n")
            .await
            .unwrap();
        let first = lists.join("first.list");
        let second = lists.join("second.list");
        tokio::fs::write(&first, "before-first.example\n")
            .await
            .unwrap();
        tokio::fs::write(&second, "before-second.example\n")
            .await
            .unwrap();
        let state_dir = dir.path().join("state");
        let state = test_state_with_config(state_dir.clone(), Some(config.clone()));
        *state.change_history_fail_path.lock().await = Some(second.canonicalize().unwrap());
        let app = api_router(state, dir.path().join("web"));

        let _ = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/change-history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        tokio::fs::write(&first, "after-first.example\n")
            .await
            .unwrap();
        tokio::fs::write(&second, "after-second.example\n")
            .await
            .unwrap();
        change_history::scan(&state_dir, &config, "интерфейс", "двойное изменение").unwrap();
        let batch = change_history::undoable_batch(&state_dir).unwrap().unwrap();
        assert_eq!(batch.len(), 2);

        let undone = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/change-history/undo-last")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(undone.status(), StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(
            tokio::fs::read_to_string(first).await.unwrap(),
            "after-first.example\n"
        );
        assert_eq!(
            tokio::fs::read_to_string(second).await.unwrap(),
            "after-second.example\n"
        );

        let history = app
            .oneshot(
                Request::builder()
                    .uri("/api/change-history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(body_json(history).await["canUndoLast"], true);
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

    #[tokio::test]
    async fn freeze_scan_status_decorates_results_without_persisting_route_metadata() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        tokio::fs::create_dir_all(dir.path().join("lists"))
            .await
            .unwrap();
        tokio::fs::write(
            &config,
            "NFQWS_ARGS_CUSTOM=\"--name=Video --filter-tcp=443 --filter-l7=tls --ipset=isp.list --lua-desync=hostfakesplit:repeats=4:tcp_ts=-600000:tcp_md5:host=ya.ru\"\n",
        )
        .await
        .unwrap();
        tokio::fs::write(dir.path().join("lists/isp.list"), "203.0.113.0/24\n")
            .await
            .unwrap();
        let data = dir.path().join("data");
        let mut operation = confirmed_freeze_scan(freeze_diagnostic::FreezeTargetKind::UserHost);
        operation.results[0].found_strategy.as_mut().unwrap().step =
            "--lua-desync=hostfakesplit:strategy=9:repeats=4:tcp_ts=-600000:tcp_md5:host=ya.ru"
                .into();
        let directory = data.join("freeze-diagnostic-operations");
        write_freeze_scan_operation(&directory, &operation).unwrap();
        let app = api_router(
            test_state_with_config(data, Some(config)),
            dir.path().join("web"),
        );
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/nfqws2/diagnostics/freeze-scan/operations/{}",
                        operation.id
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response_value = body_json(response).await;
        assert_eq!(response_value["results"][0]["inConf"]["state"], "working");
        assert_eq!(
            response_value["results"][0]["inConf"]["route"]["name"],
            "Video"
        );

        let persisted = tokio::fs::read(directory.join(format!("{}.json", operation.id)))
            .await
            .unwrap();
        let persisted: serde_json::Value = serde_json::from_slice(&persisted).unwrap();
        assert!(persisted["results"][0].get("inConf").is_none());
    }

    #[tokio::test]
    async fn freeze_scan_status_stays_available_when_config_context_is_missing() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let operation = confirmed_freeze_scan(freeze_diagnostic::FreezeTargetKind::UserHost);
        let directory = data.join("freeze-diagnostic-operations");
        write_freeze_scan_operation(&directory, &operation).unwrap();
        let app = api_router(test_state(data), dir.path().join("web"));
        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!(
                        "/api/nfqws2/diagnostics/freeze-scan/operations/{}",
                        operation.id
                    ))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        let response_value = body_json(response).await;
        assert_eq!(response_value["results"][0]["inConf"]["state"], "unknown");
        assert!(
            response_value["results"][0]["inConf"]["why"]
                .as_str()
                .unwrap()
                .contains("не найден"),
            "unexpected status: {response_value}"
        );
    }

    #[tokio::test]
    async fn freeze_scan_profile_context_caps_config_reads() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        tokio::fs::write(&config, vec![b'x'; MAX_CONFIG_BYTES + 1])
            .await
            .unwrap();
        let error = read_freeze_scan_profile_context(&config).await.unwrap_err();
        assert!(error.contains("превышает лимит 512 КиБ"));
    }

    #[test]
    fn freeze_in_config_status_requires_the_strategy_in_the_actual_route_profile() {
        let step = "--lua-desync=fake:strategy=2:blob=tls_clienthello";
        let matching_config = "NFQWS_ARGS_CUSTOM=\"--name=Video --filter-tcp=443 --filter-l7=tls --ipset=lists/isp.list --lua-desync=fake:strategy=1:blob=tls_clienthello\"\n";
        let profiles = nfqws2_ui::runtime_profiles::from_config(matching_config, "route").unwrap();
        let lists = HashMap::from([("lists/isp.list".into(), "203.0.113.0/24\n".into())]);
        let status = freeze_in_config_status(
            "video.example",
            Some("203.0.113.8".parse().unwrap()),
            step,
            &profiles.profiles,
            &lists,
        );
        assert_eq!(status["state"], "working");
        assert_eq!(status["profiles"][0]["name"], "Video");
        assert_eq!(status["route"]["index"], 1);

        let later_config = "NFQWS_ARGS_CUSTOM=\"--name=CatchAll --filter-tcp=443 --filter-l7=tls --lua-desync=multisplit:pos=1 --new=Video --filter-tcp=443 --filter-l7=tls --ipset=lists/isp.list --lua-desync=fake:strategy=1:blob=tls_clienthello\"\n";
        let profiles = nfqws2_ui::runtime_profiles::from_config(later_config, "route").unwrap();
        let status = freeze_in_config_status(
            "video.example",
            Some("203.0.113.8".parse().unwrap()),
            step,
            &profiles.profiles,
            &lists,
        );
        assert_eq!(status["state"], "elsewhere");
        assert_eq!(status["route"]["name"], "CatchAll");
    }

    #[test]
    fn freeze_in_config_status_reports_unknown_when_current_route_cannot_be_proven() {
        let config = "NFQWS_ARGS_CUSTOM=\"--name=Excluded --filter-tcp=443 --filter-l7=tls --ipset=lists/isp.list --ipset-exclude=lists/excluded.list --lua-desync=fake:blob=tls_clienthello\"\n";
        let profiles = nfqws2_ui::runtime_profiles::from_config(config, "route").unwrap();
        let lists = HashMap::from([("lists/isp.list".into(), "203.0.113.0/24\n".into())]);
        let status = freeze_in_config_status(
            "video.example",
            Some("203.0.113.8".parse().unwrap()),
            "--lua-desync=fake:blob=tls_clienthello",
            &profiles.profiles,
            &lists,
        );
        assert_eq!(status["state"], "unknown");
        assert!(status["why"].as_str().unwrap().contains("Excluded"));
    }

    #[test]
    fn freeze_in_config_status_explains_a_known_profile_exclusion() {
        let config = "NFQWS_ARGS_CUSTOM=\"--name=Excluded --filter-tcp=443 --filter-l7=tls --ipset-exclude=lists/excluded.list --lua-desync=fake:blob=tls_clienthello --new=General --filter-tcp=443 --filter-l7=tls --lua-desync=multisplit:pos=1\"\n";
        let profiles = nfqws2_ui::runtime_profiles::from_config(config, "route").unwrap();
        let lists = HashMap::from([(
            "lists/excluded.list".into(),
            "203.0.113.4-203.0.113.20\n".into(),
        )]);
        let status = freeze_in_config_status(
            "video.example",
            Some("203.0.113.8".parse().unwrap()),
            "--lua-desync=fake:blob=tls_clienthello",
            &profiles.profiles,
            &lists,
        );
        assert_eq!(status["state"], "elsewhere");
        assert_eq!(status["route"]["name"], "General");
        assert!(
            status["why"]
                .as_str()
                .unwrap()
                .contains("--ipset-exclude=lists/excluded.list")
        );
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

    #[test]
    fn custom_strategy_mode_does_not_enable_catalog_candidates() {
        assert!(!picker_mode_uses_catalog_candidates(true));
        assert!(picker_mode_uses_catalog_candidates(false));
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
            hosts: Vec::new(),
            podkop_hosts: Vec::new(),
            multi_site_results: Vec::new(),
            multi_site_baseline: Vec::new(),
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
                    hosts: Vec::new(),
                    podkop_hosts: Vec::new(),
                    multi_site_results: Vec::new(),
                    multi_site_baseline: Vec::new(),
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
    async fn clearing_strategy_pick_history_removes_finished_jobs_and_keeps_running_jobs() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        let operation_dir = data.join("strategy-pick-operations");
        for (index, state, host) in [
            (1, "complete", "done.example"),
            (2, "running", "active.example"),
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
                    hosts: Vec::new(),
                    podkop_hosts: Vec::new(),
                    multi_site_results: Vec::new(),
                    multi_site_baseline: Vec::new(),
                    message: state.into(),
                    updated_at: index as u64,
                },
            )
            .unwrap();
        }
        let app = api_router(test_state(data), dir.path().join("web"));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("DELETE")
                    .uri("/api/nfqws2/strategies/pick/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body_json(response).await["removed"], 1);

        let history = app
            .oneshot(
                Request::builder()
                    .uri("/api/nfqws2/strategies/pick/history")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert!(body_json(history).await.as_array().unwrap().is_empty());
        assert!(operation_dir.join(format!("{:032x}.json", 2)).exists());
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

    #[test]
    fn quic_picker_keeps_serial_bounded_flow_and_skips_when_baseline_answers() {
        assert!(!strategy_uses_full_batch_runner(
            StrategyProtocol::Quic,
            Some(StrategyMode::Full)
        ));
        assert!(strategy_uses_full_batch_runner(
            StrategyProtocol::Https,
            Some(StrategyMode::Full)
        ));
        assert!(quic_baseline_is_already_reachable(
            StrategyProtocol::Quic,
            Some(true)
        ));
        assert!(!quic_baseline_is_already_reachable(
            StrategyProtocol::Quic,
            Some(false)
        ));
        assert!(!quic_baseline_is_already_reachable(
            StrategyProtocol::Quic,
            None
        ));
    }

    #[tokio::test]
    async fn quic_picker_requires_a_domain_and_reaches_runtime_preconditions() {
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
        let ip_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/nfqws2/strategies/pick")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"host":"8.8.8.8","protocol":"quic"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ip_response.status(), StatusCode::BAD_REQUEST);

        let domain_response = app
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
        assert_eq!(domain_response.status(), StatusCode::CONFLICT);
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
    async fn config_format_endpoint_formats_only_the_draft_and_rejects_ambiguous_shell() {
        let dir = tempfile::tempdir().unwrap();
        let config_path = dir.path().join("nfqws2.conf");
        let original = "NFQWS_ARGS=\"--filter-tcp=443 --lua-desync=fake\"\n";
        tokio::fs::write(&config_path, original).await.unwrap();
        let app = api_router(
            test_state_with_config(dir.path().to_path_buf(), Some(config_path.clone())),
            dir.path().into(),
        );
        let formatted = app
            .clone()
            .oneshot(
                Request::post("/api/nfqws2/config/format")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"content": original}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(formatted.status(), StatusCode::OK);
        assert_eq!(
            body_json(formatted).await["content"],
            "NFQWS_ARGS=\"--filter-tcp=443\n    --lua-desync=fake\"\n"
        );
        let ambiguous = app
            .oneshot(
                Request::post("/api/nfqws2/config/format")
                    .header("content-type", "application/json")
                    .body(Body::from(
                        serde_json::json!({"content": "NFQWS_ARGS=\"$MODE\"\n"}).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(ambiguous.status(), StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(
            tokio::fs::read_to_string(&config_path).await.unwrap(),
            original
        );
        assert!(!dir.path().join("backups").exists());
    }

    #[tokio::test]
    async fn connected_package_user_list_can_be_read_validated_and_reread() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let list_dir = dir.path().join("lists");
        tokio::fs::create_dir_all(&list_dir).await.unwrap();
        tokio::fs::write(&config, "NFQWS_ARGS=--hostlist=lists/user.list\n")
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
            "NFQWS_ARGS=\"--hostlist=lists/user.list --hostlist-auto=lists/auto.list --hostlist-exclude=lists/exclude.list --ipset=lists/ipset.list --ipset-exclude=lists/ipset_exclude.list\"\n",
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
        let user = report["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|file| file["id"] == "user.list")
            .unwrap();
        assert_eq!(user["usageKnown"], true);
        assert_eq!(user["validationRequired"], true);
        assert!(
            user["usedBy"]
                .as_array()
                .unwrap()
                .iter()
                .any(|usage| usage["role"] == "include")
        );
        let exclude = report["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|file| file["id"] == "exclude.list")
            .unwrap();
        assert!(
            exclude["usedBy"]
                .as_array()
                .unwrap()
                .iter()
                .any(|usage| usage["role"] == "exclude")
        );
        let auto = report["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|file| file["id"] == "auto.list")
            .unwrap();
        assert!(
            auto["usedBy"]
                .as_array()
                .unwrap()
                .iter()
                .any(|usage| usage["role"] == "auto")
        );
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
        let files = report["files"].as_array().unwrap();
        let connected = files.iter().find(|file| file["id"] == "user.list").unwrap();
        let unconnected = files
            .iter()
            .find(|file| file["id"] == "google.list")
            .unwrap();
        assert_eq!(connected["validationRequired"], true);
        assert_eq!(unconnected["validationRequired"], false);
    }

    #[tokio::test]
    async fn list_index_exposes_managed_asn_refresh_health_for_the_matching_list() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let list_dir = dir.path().join("lists");
        tokio::fs::create_dir_all(&list_dir).await.unwrap();
        tokio::fs::write(
            &config,
            "NFQWS_ARGS_CUSTOM=--ipset=lists/ipset_as15169.list\n",
        )
        .await
        .unwrap();
        tokio::fs::write(list_dir.join("ipset_as15169.list"), "8.8.8.0/24\n")
            .await
            .unwrap();
        let state = test_state_with_config(dir.path().join("state"), Some(config));
        let mut managed = ManagedAsnListStore::default();
        let mut feed = ManagedAsnList::new(15169, "ipset_as15169.list", 1, b"8.8.8.0/24\n");
        feed.last_attempt_date = Some("2026-10-27".into());
        feed.last_success_date = Some("2026-10-26".into());
        feed.last_error = Some("RIPEstat не ответил".into());
        managed.register(feed);
        managed.save(&state.data_dir).unwrap();
        let app = api_router(state, dir.path().join("web"));

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
        let file = report["files"]
            .as_array()
            .unwrap()
            .iter()
            .find(|file| file["id"] == "ipset_as15169.list")
            .unwrap();
        assert_eq!(file["managedAsn"]["asn"], 15169);
        assert_eq!(file["managedAsn"]["prefixCount"], 1);
        assert_eq!(file["managedAsn"]["lastError"], "RIPEstat не ответил");
        assert_eq!(file["managedAsn"]["lastSuccessDate"], "2026-10-26");
    }

    #[test]
    fn managed_asn_scheduler_accepts_only_iso_local_dates() {
        assert!(valid_iso_local_date("2026-10-27"));
        assert!(!valid_iso_local_date("10/27/2026"));
        assert!(!valid_iso_local_date("2026-1-27"));
        assert!(!valid_iso_local_date("2026-10-2x"));
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
    async fn unconnected_standard_list_can_save_other_tool_format() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let list_dir = dir.path().join("lists");
        tokio::fs::create_dir_all(&list_dir).await.unwrap();
        tokio::fs::write(&config, "NFQWS_ARGS=\"\"\n")
            .await
            .unwrap();
        let path = list_dir.join("user.list");
        tokio::fs::write(&path, "example.org\n").await.unwrap();
        let app = api_router(
            test_state_with_config(dir.path().join("state"), Some(config)),
            dir.path().join("web"),
        );
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
        let revision = body_json(loaded).await["revision"]
            .as_str()
            .unwrap()
            .to_owned();
        let saved = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/nfqws2/lists/user.list")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"content":"+example.org\n","revision":"{revision}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(saved.status(), StatusCode::OK);
        assert_eq!(
            tokio::fs::read_to_string(path).await.unwrap(),
            "+example.org\n"
        );
    }

    #[tokio::test]
    async fn unconnected_custom_list_can_save_other_tool_format() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let list_dir = dir.path().join("lists");
        tokio::fs::create_dir_all(&list_dir).await.unwrap();
        tokio::fs::write(&config, "NFQWS_ARGS=\"\"\n")
            .await
            .unwrap();
        let path = list_dir.join("custom.list");
        tokio::fs::write(&path, "example.org\n").await.unwrap();
        let app = api_router(
            test_state_with_config(dir.path().join("state"), Some(config)),
            dir.path().join("web"),
        );
        let loaded = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/nfqws2/lists/custom.list")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(loaded.status(), StatusCode::OK);
        let revision = body_json(loaded).await["revision"]
            .as_str()
            .unwrap()
            .to_owned();
        let saved = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/nfqws2/lists/custom.list")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"content":"+example.org\n","revision":"{revision}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(saved.status(), StatusCode::OK);
        assert_eq!(
            tokio::fs::read_to_string(path).await.unwrap(),
            "+example.org\n"
        );
    }

    #[tokio::test]
    async fn connected_standard_list_rejects_other_tool_format_without_writing() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let list_dir = dir.path().join("lists");
        tokio::fs::create_dir_all(&list_dir).await.unwrap();
        tokio::fs::write(&config, "NFQWS_ARGS=--hostlist=lists/user.list\n")
            .await
            .unwrap();
        let path = list_dir.join("user.list");
        tokio::fs::write(&path, "example.org\n").await.unwrap();
        let app = api_router(
            test_state_with_config(dir.path().join("state"), Some(config)),
            dir.path().join("web"),
        );
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
        let revision = body_json(loaded).await["revision"]
            .as_str()
            .unwrap()
            .to_owned();
        let saved = app
            .oneshot(
                Request::builder()
                    .method("PUT")
                    .uri("/api/nfqws2/lists/user.list")
                    .header("content-type", "application/json")
                    .body(Body::from(format!(
                        r#"{{"content":"+example.org\n","revision":"{revision}"}}"#
                    )))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(saved.status(), StatusCode::BAD_REQUEST);
        assert_eq!(
            tokio::fs::read_to_string(path).await.unwrap(),
            "example.org\n"
        );
    }

    #[tokio::test]
    async fn custom_list_delete_is_revision_checked_and_rejects_standard_lists() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let list_dir = dir.path().join("lists");
        tokio::fs::create_dir_all(&list_dir).await.unwrap();
        tokio::fs::write(
            &config,
            "MODE_LIST=\"--hostlist=lists/user.list\"\nNFQWS_EXTRA_ARGS=\"$MODE_LIST\"\n",
        )
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
    async fn custom_list_delete_rejects_a_list_still_used_by_a_profile() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let lists = dir.path().join("lists");
        tokio::fs::create_dir_all(&lists).await.unwrap();
        tokio::fs::write(&config, "NFQWS_ARGS_CUSTOM=--hostlist=lists/google.list\n")
            .await
            .unwrap();
        let list_path = lists.join("google.list");
        tokio::fs::write(&list_path, "google.com\n").await.unwrap();
        let app = api_router(
            test_state_with_config(dir.path().join("state"), Some(config)),
            dir.path().join("web"),
        );
        let loaded = app
            .clone()
            .oneshot(
                Request::get("/api/nfqws2/lists/google.list")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let revision = body_json(loaded).await["revision"]
            .as_str()
            .unwrap()
            .to_owned();
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
        assert_eq!(deleted.status(), StatusCode::CONFLICT);
        assert!(list_path.is_file());
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

    #[test]
    fn list_profile_usage_reports_include_exclude_and_ipset_roles() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let config = root.join("nfqws2.conf");
        let lists = root.join("lists");
        std::fs::create_dir_all(&lists).unwrap();
        std::fs::write(&config, "").unwrap();
        std::fs::write(lists.join("user.list"), "example.org\n").unwrap();
        std::fs::write(lists.join("ipset.list"), "203.0.113.7\n").unwrap();
        let content = concat!(
            "NFQWS_ARGS=\"--hostlist=lists/user.list --hostlist-exclude=lists/user.list\"\n",
            "NFQWS_ARGS_IPSET=\"--ipset=lists/ipset.list\"\n",
        );
        let profiles = nfqws2_ui::runtime_profiles::from_config(content, "revision").unwrap();
        let user_usage = list_profile_usage(&config, &lists.join("user.list"), &profiles.profiles);
        assert!(user_usage.iter().any(|usage| usage.role == "include"));
        assert!(user_usage.iter().any(|usage| usage.role == "exclude"));
        assert!(
            user_usage
                .iter()
                .all(|usage| !usage.profile_name.is_empty())
        );
        let ipset_usage =
            list_profile_usage(&config, &lists.join("ipset.list"), &profiles.profiles);
        assert!(ipset_usage.iter().any(|usage| usage.role == "include"));
    }

    #[test]
    fn configured_list_reference_parser_keeps_multiple_paths_and_marks_dynamic_values() {
        let (references, ambiguous) = configured_list_references_for_flag(
            "NFQWS_ARGS=\"--hostlist=lists/one.list --hostlist=lists/two.list\"\n",
            "--hostlist",
        );
        assert_eq!(
            references,
            [
                PathBuf::from("lists/one.list"),
                PathBuf::from("lists/two.list")
            ]
        );
        assert!(!ambiguous);

        let (references, ambiguous) = configured_list_references_for_flag(
            "NFQWS_ARGS=\"--hostlist=${USER_LIST}\"\n",
            "--hostlist",
        );
        assert!(references.is_empty());
        assert!(ambiguous);
    }

    #[test]
    fn list_validation_is_skipped_only_when_unreferenced_is_proven() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let lists = dir.path().join("lists");
        std::fs::create_dir_all(&lists).unwrap();
        std::fs::write(&config, "").unwrap();
        let user_list = lists.join("user.list");
        std::fs::write(&user_list, "+example.org\n").unwrap();

        assert!(!list_requires_nfqws_validation(
            &config,
            &user_list,
            "user.list",
            "",
            None,
        ));
        assert!(list_requires_nfqws_validation(
            &config,
            &user_list,
            "user.list",
            "NFQWS_ARGS=\"--hostlist=lists/user.list --hostlist=lists/other.list\"\n",
            None,
        ));
        assert!(list_requires_nfqws_validation(
            &config,
            &user_list,
            "user.list",
            "NFQWS_ARGS=\"--hostlist=${USER_LIST}\"\n",
            None,
        ));
        let active_args = ["nfqws2".to_owned(), "--hostlist=lists/user.list".to_owned()];
        assert!(list_requires_nfqws_validation(
            &config,
            &user_list,
            "user.list",
            "",
            Some(&active_args),
        ));
        assert!(!list_requires_nfqws_validation(
            &config,
            &user_list,
            "custom.list",
            "",
            None,
        ));
        let custom_list = lists.join("custom.list");
        std::fs::write(&custom_list, "+example.org\n").unwrap();
        for reference in ["--hostlist=lists/custom.list", "--ipset=lists/custom.list"] {
            assert!(list_requires_nfqws_validation(
                &config,
                &custom_list,
                "custom.list",
                &format!("NFQWS_ARGS=\"{reference}\"\n"),
                None,
            ));
        }
        assert!(list_requires_nfqws_validation(
            &config,
            &custom_list,
            "custom.list",
            "NFQWS_ARGS=\"--ipset=${CUSTOM_IPSET}\"\\n",
            None,
        ));
    }

    #[test]
    fn active_list_inventory_reads_custom_profile_lists_and_keeps_their_kind() {
        let args = [
            "nfqws2",
            "--hostlist=lists/custom-domains.txt",
            "--new",
            "--ipset",
            "lists/as15169.list",
            "--hostlist-auto-debug=/tmp/auto.log",
        ]
        .map(str::to_owned);
        assert_eq!(
            runtime_list_references(&args),
            [
                (PathBuf::from("lists/custom-domains.txt"), false),
                (PathBuf::from("lists/as15169.list"), true),
            ]
        );
    }

    #[test]
    fn diagnostic_list_candidates_include_active_custom_lists_without_duplicates() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let lists = dir.path().join("lists");
        std::fs::create_dir_all(&lists).unwrap();
        let standard = lists.join("user.list");
        let custom = lists.join("custom-domains.txt");
        let ipset = lists.join("as15169.list");
        std::fs::write(&config, "").unwrap();
        std::fs::write(&standard, "example.com\n").unwrap();
        std::fs::write(&custom, "custom.example.org\n").unwrap();
        std::fs::write(&ipset, "8.8.8.0/24\n").unwrap();
        let args = [
            "nfqws2".to_owned(),
            "--hostlist=lists/user.list".to_owned(),
            "--hostlist=lists/custom-domains.txt".to_owned(),
            "--ipset".to_owned(),
            "lists/as15169.list".to_owned(),
        ];

        let candidates = diagnostic_list_candidates(&config, &args);
        assert_eq!(candidates.len(), 3);
        assert!(candidates.iter().any(|(label, path, ip)| {
            label == "lists/custom-domains.txt" && path == &custom.canonicalize().unwrap() && !ip
        }));
        assert!(candidates.iter().any(|(label, path, ip)| {
            label == "lists/as15169.list" && path == &ipset.canonicalize().unwrap() && *ip
        }));
        assert_eq!(
            candidates
                .iter()
                .filter(|(_, path, _)| path == &standard.canonicalize().unwrap())
                .count(),
            1
        );
    }

    #[tokio::test]
    async fn active_runtime_list_matching_finds_domain_and_ip_entries_only_in_referenced_files() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let lists = dir.path().join("lists");
        std::fs::create_dir_all(&lists).unwrap();
        std::fs::write(&config, "").unwrap();
        std::fs::write(lists.join("active-domains.list"), "example.com\n").unwrap();
        std::fs::write(lists.join("active-ips.list"), "8.8.8.0/24\n").unwrap();
        std::fs::write(lists.join("other-tool.list"), "example.com\n").unwrap();
        let args = [
            "nfqws2".to_owned(),
            "--hostlist=lists/active-domains.list".to_owned(),
            "--ipset=lists/active-ips.list".to_owned(),
        ];
        let candidates = diagnostic_list_candidates(&config, &args);

        let (matches, unreadable) = read_matching_runtime_list_entries(
            &candidates,
            "example.com",
            &["8.8.8.8".parse().unwrap()],
        )
        .await;

        assert!(unreadable.is_empty());
        assert_eq!(matches.len(), 2);
        assert!(
            matches
                .iter()
                .any(|entry| { entry == "lists/active-domains.list: example.com" })
        );
        assert!(
            matches
                .iter()
                .any(|entry| { entry == "lists/active-ips.list: 8.8.8.0/24" })
        );
        assert!(
            !matches
                .iter()
                .any(|entry| entry.contains("other-tool.list"))
        );
    }

    #[test]
    fn diagnostic_list_candidates_skip_standard_lists_not_used_by_nfqws2() {
        let dir = tempfile::tempdir().unwrap();
        let config = dir.path().join("nfqws2.conf");
        let lists = dir.path().join("lists");
        std::fs::create_dir_all(&lists).unwrap();
        let inactive = lists.join("user.list");
        let active = lists.join("custom-domains.list");
        std::fs::write(&config, "").unwrap();
        std::fs::write(&inactive, "unrelated.example\n").unwrap();
        std::fs::write(&active, "target.example\n").unwrap();
        let args = [
            "nfqws2".to_owned(),
            "--hostlist=lists/custom-domains.list".to_owned(),
        ];

        let candidates = diagnostic_list_candidates(&config, &args);

        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].0, "lists/custom-domains.list");
        assert_eq!(candidates[0].1, active.canonicalize().unwrap());
    }

    #[test]
    fn active_list_inventory_rejects_paths_outside_the_config_root_and_symlinks() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let config = root.join("nfqws2.conf");
        let lists = root.join("lists");
        std::fs::create_dir_all(&lists).unwrap();
        std::fs::write(&config, "").unwrap();
        std::fs::write(lists.join("safe.txt"), "example.com\n").unwrap();
        let outside = dir.path().parent().unwrap().join("outside-list.txt");
        std::fs::write(&outside, "example.com\n").unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, lists.join("linked.txt")).unwrap();

        let args = vec![
            "nfqws2".to_owned(),
            "--hostlist=lists/safe.txt".to_owned(),
            format!("--hostlist={}", outside.display()),
            "--hostlist=lists/linked.txt".to_owned(),
        ];

        let resolved = resolve_runtime_list_references(&config, &args);
        assert_eq!(resolved.len(), 1);
        assert_eq!(
            resolved[0].0,
            lists.join("safe.txt").canonicalize().unwrap()
        );
        assert!(!resolved[0].1);
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

    fn empty_classic_pcap() -> Vec<u8> {
        let mut bytes = vec![0xd4, 0xc3, 0xb2, 0xa1];
        bytes.extend_from_slice(&2u16.to_le_bytes());
        bytes.extend_from_slice(&4u16.to_le_bytes());
        bytes.extend_from_slice(&[0; 8]);
        bytes.extend_from_slice(&65_535u32.to_le_bytes());
        bytes.extend_from_slice(&1u32.to_le_bytes());
        bytes
    }

    #[tokio::test]
    async fn dpi_lab_api_analyzes_saves_lists_and_reloads_captures() {
        let dir = tempfile::tempdir().unwrap();
        let capture = empty_classic_pcap();
        let app = api_router(test_state(dir.path().join("data")), dir.path().join("web"));
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/nfqws2/dpi-lab/analyze?name=test.pcap")
                    .header("content-type", "application/octet-stream")
                    .body(Body::from(capture.clone()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body_json(response).await["capture"]["packetCount"], 0);

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/nfqws2/dpi-lab/experiments?name=..%2Fsecret.pcap")
                    .header("content-type", "application/octet-stream")
                    .body(Body::from(capture))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::CREATED);
        let saved = body_json(response).await;
        assert_eq!(saved["name"], "secret.pcap");
        let id = saved["id"].as_str().unwrap();

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/api/nfqws2/dpi-lab/experiments")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            body_json(response).await["items"].as_array().unwrap().len(),
            1
        );

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri(format!("/api/nfqws2/dpi-lab/experiments/{id}"))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(body_json(response).await["capture"]["name"], "secret.pcap");

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/nfqws2/dpi-lab/analyze")
                    .header("content-type", "application/octet-stream")
                    .body(Body::from(vec![0u8; dpi_lab::MAX_PCAP_BYTES + 1]))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::PAYLOAD_TOO_LARGE);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/nfqws2/dpi-lab/experiments/../../auth.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::NOT_FOUND);
    }

    #[tokio::test]
    async fn dpi_lab_api_rejects_unauthenticated_access_when_panel_auth_is_enabled() {
        let dir = tempfile::tempdir().unwrap();
        let data = dir.path().join("data");
        std::fs::create_dir_all(&data).unwrap();
        std::fs::write(
            data.join("auth.json"),
            r#"{"enabled":true,"passwordHash":"invalid","rememberDays":30,"sessions":[]}"#,
        )
        .unwrap();
        let app = api_router(test_state(data), dir.path().join("web"));
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/nfqws2/dpi-lab/experiments")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }
}
