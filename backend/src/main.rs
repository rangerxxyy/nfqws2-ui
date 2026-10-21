use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{
    Json, Router,
    body::Body,
    extract::{Query, State},
    http::{HeaderMap, HeaderName, HeaderValue, Request, StatusCode, header},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use nfqws2_ui::{
    ServiceAction, ServiceState, known_config_paths, parse_meminfo_kb, service_action,
    service_status,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    io::Read,
    net::SocketAddr,
    path::{Path, PathBuf},
    sync::Arc,
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use subtle::ConstantTimeEq;
use tokio::io::AsyncWriteExt;
use tokio::{
    net::TcpListener,
    sync::{Mutex, Semaphore},
};
use tower_http::{services::ServeDir, trace::TraceLayer};

const SESSION_COOKIE: &str = "nfqws2ui_session";
const SESSION_MAX_AGE: Duration = Duration::from_secs(8 * 60 * 60);
const MAX_CONFIG_BYTES: usize = 512 * 1024;
const MAX_LOG_BYTES: usize = 256 * 1024;

struct AppState {
    password_hash: Option<String>,
    sessions: Arc<Mutex<HashMap<String, Session>>>,
    data_dir: PathBuf,
    port: u16,
    cpu_previous: Arc<Mutex<Option<(u64, u64)>>>,
    login_failures: Mutex<(u8, Option<Instant>)>,
    login_gate: Semaphore,
    config_write_lock: Mutex<()>,
}

struct Session {
    csrf: String,
    expires: Instant,
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
}

#[derive(Serialize)]
struct PanelInfo {
    version: &'static str,
    port: u16,
}

#[derive(Deserialize)]
struct Credentials {
    password: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SessionResponse {
    authenticated: bool,
    csrf_token: Option<String>,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct ConfigResponse {
    file: String,
    content: String,
    revision: String,
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

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct BackupItem {
    id: String,
    created_at: u64,
    file: String,
}

#[derive(Serialize)]
struct BackupsResponse {
    backups: Vec<BackupItem>,
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

fn random_token() -> String {
    use rand::{Rng, distr::Alphanumeric};
    rand::rng()
        .sample_iter(&Alphanumeric)
        .take(48)
        .map(char::from)
        .collect()
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookies = headers.get(header::COOKIE)?.to_str().ok()?;
    cookies
        .split(';')
        .filter_map(|part| part.trim().split_once('='))
        .find_map(|(key, value)| (key == name).then(|| value.to_string()))
}

async fn authenticated(
    headers: &HeaderMap,
    state: &AppState,
    mutation: bool,
) -> Result<String, (StatusCode, String)> {
    let Some(token) = cookie_value(headers, SESSION_COOKIE) else {
        return Err((StatusCode::UNAUTHORIZED, "Требуется вход в панель".into()));
    };
    let mut sessions = state.sessions.lock().await;
    sessions.retain(|_, session| session.expires > Instant::now());
    let Some(session) = sessions.get(&token) else {
        return Err((
            StatusCode::UNAUTHORIZED,
            "Сессия истекла; войдите снова".into(),
        ));
    };
    if mutation {
        let Some(csrf) = headers
            .get("x-csrf-token")
            .and_then(|value| value.to_str().ok())
        else {
            return Err((StatusCode::FORBIDDEN, "Отсутствует CSRF-токен".into()));
        };
        if csrf.as_bytes().ct_eq(session.csrf.as_bytes()).unwrap_u8() != 1 {
            return Err((StatusCode::FORBIDDEN, "Неверный CSRF-токен".into()));
        }
    }
    Ok(token)
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({"ok":true,"service":"nfqws2-ui"}))
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
        HeaderName::from_static("content-security-policy"),
        HeaderValue::from_static("default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'self'"),
    );
    response
}

async fn get_session(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
) -> Json<SessionResponse> {
    if let Some(token) = cookie_value(&headers, SESSION_COOKIE) {
        let mut sessions = state.sessions.lock().await;
        sessions.retain(|_, session| session.expires > Instant::now());
        if let Some(session) = sessions.get(&token) {
            return Json(SessionResponse {
                authenticated: true,
                csrf_token: Some(session.csrf.clone()),
            });
        }
    }
    Json(SessionResponse {
        authenticated: false,
        csrf_token: None,
    })
}

async fn login(
    State(state): State<Arc<AppState>>,
    Json(credentials): Json<Credentials>,
) -> Response {
    let _login_guard = match state.login_gate.try_acquire() {
        Ok(guard) => guard,
        Err(_) => {
            return error(
                StatusCode::TOO_MANY_REQUESTS,
                "Проверка другого входа ещё выполняется",
            );
        }
    };
    if credentials.password.len() < 8 || credentials.password.len() > 1024 {
        return error(StatusCode::UNAUTHORIZED, "Неверный пароль");
    }
    {
        let failures = state.login_failures.lock().await;
        if let Some(until) = failures.1.filter(|_| failures.0 >= 5)
            && until > Instant::now()
        {
            return error(
                StatusCode::TOO_MANY_REQUESTS,
                "Слишком много неудачных входов; подождите минуту",
            );
        }
    }
    let Some(stored_hash) = state.password_hash.as_deref() else {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Пароль администратора не настроен. Запустите мастер установки.",
        );
    };
    if PasswordHash::new(stored_hash).is_err() {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "Файл пароля администратора повреждён",
        );
    }
    let owned_hash = stored_hash.to_owned();
    let password = credentials.password;
    let password_valid = tokio::task::spawn_blocking(move || {
        let Ok(parsed) = PasswordHash::new(&owned_hash) else {
            return false;
        };
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
    .await
    .unwrap_or(false);
    if !password_valid {
        let mut failures = state.login_failures.lock().await;
        failures.0 = failures.0.saturating_add(1);
        if failures.0 >= 5 {
            failures.1 = Some(Instant::now() + Duration::from_secs(60));
        }
        return error(StatusCode::UNAUTHORIZED, "Неверный пароль");
    }
    *state.login_failures.lock().await = (0, None);
    let token = random_token();
    let csrf = random_token();
    state.sessions.lock().await.insert(
        token.clone(),
        Session {
            csrf: csrf.clone(),
            expires: Instant::now() + SESSION_MAX_AGE,
        },
    );
    let mut response = Json(SessionResponse {
        authenticated: true,
        csrf_token: Some(csrf),
    })
    .into_response();
    let cookie = format!(
        "{SESSION_COOKIE}={token}; Path=/; HttpOnly; SameSite=Strict; Max-Age={}",
        SESSION_MAX_AGE.as_secs()
    );
    if let Ok(value) = HeaderValue::from_str(&cookie) {
        response.headers_mut().insert(header::SET_COOKIE, value);
    }
    response
}

async fn logout(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    let token = match authenticated(&headers, &state, true).await {
        Ok(token) => token,
        Err((status, message)) => return error(status, message),
    };
    state.sessions.lock().await.remove(&token);
    let mut response = Json(serde_json::json!({"ok":true})).into_response();
    response.headers_mut().insert(
        header::SET_COOKIE,
        HeaderValue::from_static("nfqws2ui_session=; Path=/; HttpOnly; SameSite=Strict; Max-Age=0"),
    );
    response
}

async fn get_status(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Err((status, message)) = authenticated(&headers, &state, false).await {
        return error(status, message);
    }
    let (service_state, detail) = service_status().await;
    let meminfo = tokio::fs::read_to_string("/proc/meminfo").await.ok();
    let total_kb = meminfo.as_deref().and_then(parse_meminfo_kb);
    let available_kb = meminfo
        .as_deref()
        .and_then(|text| parse_meminfo_kb_key(text, "MemAvailable"))
        .or_else(|| {
            meminfo
                .as_deref()
                .and_then(|text| parse_meminfo_kb_key(text, "MemFree"))
        });
    let memory_total_bytes = total_kb.map(|kb| kb.saturating_mul(1024));
    let memory_used_bytes = total_kb
        .zip(available_kb)
        .map(|(total, free)| total.saturating_sub(free).saturating_mul(1024));
    let cpu_percent = cpu_sample(&state).await;
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
            version: None,
        },
        system: SystemInfo {
            memory_total_bytes,
            memory_used_bytes,
            cpu_percent,
        },
        panel: PanelInfo {
            version: env!("CARGO_PKG_VERSION"),
            port: state.port,
        },
    })
    .into_response()
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

async fn post_service(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Json(request): Json<ServiceRequest>,
) -> Response {
    if let Err((status, message)) = authenticated(&headers, &state, true).await {
        return error(status, message);
    }
    let action = match ServiceAction::try_from(request.action.as_str()) {
        Ok(action) => action,
        Err(()) => {
            return error(
                StatusCode::BAD_REQUEST,
                "Допустимые действия: start, stop, restart",
            );
        }
    };
    match service_action(action).await {
        Ok(message) => Json(MessageResponse { message }).into_response(),
        Err(message) => error(StatusCode::BAD_GATEWAY, message),
    }
}

fn config_path(name: &str) -> Option<PathBuf> {
    if name != "nfqws2.conf" {
        return None;
    }
    known_config_paths().into_iter().find(|path| path.is_file())
}

fn revision(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

async fn get_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ConfigQuery>,
) -> Response {
    if let Err((status, message)) = authenticated(&headers, &state, false).await {
        return error(status, message);
    }
    let Some(path) = config_path(&query.file) else {
        return error(
            StatusCode::NOT_FOUND,
            "Разрешённый конфигурационный файл не найден",
        );
    };
    match tokio::fs::read(&path).await {
        Ok(bytes) if bytes.len() <= MAX_CONFIG_BYTES => Json(ConfigResponse {
            file: query.file,
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

async fn put_config(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<ConfigQuery>,
    Json(write): Json<ConfigWrite>,
) -> Response {
    if let Err((status, message)) = authenticated(&headers, &state, true).await {
        return error(status, message);
    }
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
    let Some(path) = config_path(&query.file) else {
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
    if let Err(message) = backup_and_write(&state.data_dir, &path, write.content.as_bytes()).await {
        return error(StatusCode::INTERNAL_SERVER_ERROR, message);
    }
    Json(ConfigResponse {
        file: query.file,
        content: write.content.clone(),
        revision: revision(write.content.as_bytes()),
    })
    .into_response()
}

async fn backup_and_write(
    data_dir: &Path,
    config: &Path,
    new_content: &[u8],
) -> Result<(), String> {
    if new_content.len() > MAX_CONFIG_BYTES {
        return Err("Новая конфигурация превышает лимит 512 КБ".into());
    }
    let old = tokio::fs::read(config)
        .await
        .map_err(|error| error.to_string())?;
    if old.len() > MAX_CONFIG_BYTES {
        return Err("Текущая конфигурация превышает лимит 512 КБ; изменение отменено".into());
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
    let backup = backup_dir.join(format!("nfqws2.conf.{stamp}.bak"));
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
    let meta = tokio::fs::metadata(config)
        .await
        .map_err(|error| error.to_string())?;
    let tmp = config.with_extension(format!("conf.{stamp}.tmp"));
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
        use std::os::unix::fs::PermissionsExt;
        let mode = meta.permissions().mode();
        tokio::fs::set_permissions(&tmp, std::fs::Permissions::from_mode(mode))
            .await
            .map_err(|error| error.to_string())?;
    }
    drop(tmp_file);
    if let Err(error) = tokio::fs::rename(&tmp, config).await {
        let _ = tokio::fs::remove_file(&tmp).await;
        return Err(format!(
            "Не удалось атомарно заменить конфиг; backup сохранён: {error}"
        ));
    }
    #[cfg(unix)]
    {
        let directory = tokio::fs::File::open(config.parent().unwrap_or_else(|| Path::new("/")))
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

async fn get_backups(State(state): State<Arc<AppState>>, headers: HeaderMap) -> Response {
    if let Err((status, message)) = authenticated(&headers, &state, false).await {
        return error(status, message);
    }
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

async fn restore_backup(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    axum::extract::Path(id): axum::extract::Path<String>,
) -> Response {
    if let Err((status, message)) = authenticated(&headers, &state, true).await {
        return error(status, message);
    }
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
    let Some(config) = config_path("nfqws2.conf") else {
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

async fn get_logs(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    Query(query): Query<LogsQuery>,
) -> Response {
    if let Err((status, message)) = authenticated(&headers, &state, false).await {
        return error(status, message);
    }
    if query.source != "nfqws2" {
        return error(StatusCode::BAD_REQUEST, "Неизвестный источник журнала");
    }
    let paths = [
        "/opt/var/log/nfqws2.log",
        "/opt/var/log/nfqws.log",
        "/var/log/nfqws2.log",
    ];
    let Some(path) = paths.iter().find(|path| Path::new(path).is_file()) else {
        return Json(LogsResponse {
            content: "Журнал nfqws2 не найден".into(),
        })
        .into_response();
    };
    let bytes = match tokio::fs::read(path).await {
        Ok(bytes) => bytes,
        Err(io_error) => return error(StatusCode::INTERNAL_SERVER_ERROR, io_error.to_string()),
    };
    let tail_bytes = bytes.len().min(MAX_LOG_BYTES);
    let mut content = String::from_utf8_lossy(&bytes[bytes.len() - tail_bytes..]).into_owned();
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
        .route("/api/health", get(health))
        .route("/api/session", get(get_session))
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
        .route("/api/status", get(get_status))
        .route("/api/service", post(post_service))
        .route("/api/config", get(get_config).put(put_config))
        .route("/api/backups", get(get_backups))
        .route("/api/backups/{id}/restore", post(restore_backup))
        .route("/api/logs", get(get_logs))
        .with_state(state)
        .fallback_service(ServeDir::new(web_root).append_index_html_on_directories(true))
        .layer(TraceLayer::new_for_http())
        .layer(middleware::from_fn(security_headers))
}

#[tokio::main]
async fn main() {
    if std::env::args().nth(1).as_deref() == Some("hash-password") {
        if let Err(error) = hash_password_from_stdin() {
            eprintln!("nfqws2-ui: {error}");
            std::process::exit(2);
        }
        return;
    }
    if std::env::args().nth(1).as_deref() == Some("--version") {
        println!("nfqws2-ui {}", env!("CARGO_PKG_VERSION"));
        return;
    }
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();
    let password_path = std::env::var_os("NFQWS2_UI_PASSWORD_HASH")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("/opt/etc/nfqws2-ui/password.hash"));
    let password_hash = std::fs::read_to_string(password_path)
        .ok()
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
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
        password_hash,
        sessions: Arc::new(Mutex::new(HashMap::new())),
        data_dir,
        port,
        cpu_previous: Arc::new(Mutex::new(None)),
        login_failures: Mutex::new((0, None)),
        login_gate: Semaphore::new(1),
        config_write_lock: Mutex::new(()),
    });
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

fn hash_password_from_stdin() -> Result<(), String> {
    use password_hash::{SaltString, rand_core::OsRng};
    let mut password = String::new();
    std::io::stdin()
        .read_to_string(&mut password)
        .map_err(|error| error.to_string())?;
    let password = password.trim_end_matches(['\r', '\n']);
    if password.len() < 12 || password.len() > 1024 {
        return Err("Пароль должен содержать от 12 до 1024 байт".into());
    }
    let salt = SaltString::generate(&mut OsRng);
    let hash = Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map_err(|error| error.to_string())?;
    println!("{hash}");
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use password_hash::{PasswordHasher, SaltString};
    use tower::ServiceExt;

    fn test_state(password_hash: Option<String>, data_dir: PathBuf) -> Arc<AppState> {
        Arc::new(AppState {
            password_hash,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            data_dir,
            port: 1002,
            cpu_previous: Arc::new(Mutex::new(None)),
            login_failures: Mutex::new((0, None)),
            login_gate: Semaphore::new(1),
            config_write_lock: Mutex::new(()),
        })
    }

    async fn body_json(response: Response) -> serde_json::Value {
        let bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();
        serde_json::from_slice(&bytes).unwrap()
    }

    #[tokio::test]
    async fn unauthenticated_status_is_rejected() {
        let dir = tempfile::tempdir().unwrap();
        let app = api_router(
            test_state(None, dir.path().to_path_buf()),
            dir.path().join("web"),
        );
        let response = app
            .oneshot(
                Request::builder()
                    .uri("/api/status")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
        assert_eq!(response.headers().get("x-frame-options").unwrap(), "DENY");
        assert_eq!(
            response.headers().get("x-content-type-options").unwrap(),
            "nosniff"
        );
    }

    #[tokio::test]
    async fn login_rejects_parallel_argon_work_on_router() {
        let dir = tempfile::tempdir().unwrap();
        let state = test_state(None, dir.path().to_path_buf());
        let permit = state.login_gate.try_acquire().unwrap();
        let app = api_router(state.clone(), dir.path().join("web"));
        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"password":"a-valid-long-test-password"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::TOO_MANY_REQUESTS);
        drop(permit);
    }

    #[tokio::test]
    async fn authenticated_api_enforces_csrf_and_rejects_arbitrary_service_actions() {
        let dir = tempfile::tempdir().unwrap();
        let salt = SaltString::from_b64("YS1sb25nLXRlc3Qtc2FsdA").unwrap();
        let hash = Argon2::default()
            .hash_password(b"a-strong-unit-test-passphrase", &salt)
            .unwrap()
            .to_string();
        let app = api_router(
            test_state(Some(hash), dir.path().to_path_buf()),
            dir.path().join("web"),
        );
        let login = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/login")
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(
                        r#"{"password":"a-strong-unit-test-passphrase"}"#,
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(login.status(), StatusCode::OK);
        let cookie = login
            .headers()
            .get(header::SET_COOKIE)
            .unwrap()
            .to_str()
            .unwrap()
            .split(';')
            .next()
            .unwrap()
            .to_string();
        let payload = body_json(login).await;
        let csrf = payload["csrfToken"].as_str().unwrap();
        let without_csrf = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/service")
                    .header(header::COOKIE, &cookie)
                    .header(header::CONTENT_TYPE, "application/json")
                    .body(Body::from(r#"{"action":"start"}"#))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(without_csrf.status(), StatusCode::FORBIDDEN);
        let invalid_action = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/service")
                    .header(header::COOKIE, &cookie)
                    .header("x-csrf-token", csrf)
                    .header(header::CONTENT_TYPE, "application/json")
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
}
