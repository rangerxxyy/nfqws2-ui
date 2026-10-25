use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier, password_hash::SaltString};
use axum::{
    Json,
    extract::{ConnectInfo, State},
    http::{HeaderMap, HeaderValue, StatusCode, header},
    middleware::Next,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
    },
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};
use tokio::sync::Mutex as AsyncMutex;
use uuid::Uuid;

const SESSION_COOKIE: &str = "nfqws2_ui_session";
const SESSION_TTL: u64 = 24 * 60 * 60;
const MAX_PASSWORD_BYTES: usize = 1024;
const MAX_ATTEMPTS: u32 = 5;
const LOCKOUT: Duration = Duration::from_secs(60);
static TEMP_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Clone)]
pub struct AuthManager {
    path: PathBuf,
    io_lock: Arc<AsyncMutex<()>>,
    update_lock: Arc<AsyncMutex<()>>,
    attempts: Arc<Mutex<HashMap<String, (u32, Instant)>>>,
}

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthStore {
    enabled: bool,
    password_hash: Option<String>,
    #[serde(default)]
    sessions: Vec<StoredSession>,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct StoredSession {
    token_hash: String,
    expires_at: Option<u64>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoginRequest {
    password: String,
    #[serde(default)]
    remember: bool,
    #[serde(default)]
    remember_days: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SettingsRequest {
    enabled: bool,
    password: Option<String>,
    current_password: Option<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AuthStatus {
    enabled: bool,
    has_password: bool,
    authenticated: bool,
}

impl AuthManager {
    pub fn new(data_dir: PathBuf) -> Self {
        Self {
            path: data_dir.join("auth.json"),
            io_lock: Arc::new(AsyncMutex::new(())),
            update_lock: Arc::new(AsyncMutex::new(())),
            attempts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn load(&self) -> Result<AuthStore, String> {
        let _guard = self.io_lock.lock().await;
        match tokio::fs::read(&self.path).await {
            Ok(bytes) => serde_json::from_slice(&bytes)
                .map_err(|error| format!("Не удалось прочитать настройки авторизации: {error}")),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(AuthStore::default()),
            Err(error) => Err(format!(
                "Не удалось прочитать настройки авторизации: {error}"
            )),
        }
    }

    async fn save(&self, store: &AuthStore) -> Result<(), String> {
        let _guard = self.io_lock.lock().await;
        tokio::fs::create_dir_all(
            self.path
                .parent()
                .unwrap_or_else(|| std::path::Path::new(".")),
        )
        .await
        .map_err(|e| e.to_string())?;
        let sequence = TEMP_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let temp = self
            .path
            .with_extension(format!("{}.{}.tmp", std::process::id(), sequence));
        let bytes = serde_json::to_vec(store).map_err(|e| e.to_string())?;
        tokio::fs::write(&temp, bytes)
            .await
            .map_err(|e| e.to_string())?;
        #[cfg(unix)]
        tokio::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o600))
            .await
            .map_err(|e| e.to_string())?;
        if let Err(error) = tokio::fs::rename(&temp, &self.path).await {
            let _ = tokio::fs::remove_file(&temp).await;
            return Err(error.to_string());
        }
        Ok(())
    }

    fn request_ip(headers: &HeaderMap) -> String {
        // Forwarded headers are deliberately ignored: the listener does not
        // currently have a trusted-proxy configuration.
        headers
            .get("x-nfqws2-client-ip")
            .and_then(|v| v.to_str().ok())
            .filter(|v| {
                v.len() <= 64
                    && v.bytes()
                        .all(|b| b.is_ascii_alphanumeric() || b".:[]".contains(&b))
            })
            .unwrap_or("unknown")
            .to_owned()
    }

    fn set_cookie(headers: &HeaderMap, token: &str, max_age: Option<u64>) -> HeaderMap {
        let secure = (headers
            .get("x-forwarded-proto")
            .and_then(|v| v.to_str().ok())
            == Some("https")) as u8;
        let secure_attr = if secure == 1 { "Secure; " } else { "" };
        let age = max_age
            .map(|v| format!("; Max-Age={v}"))
            .unwrap_or_default();
        let mut result = HeaderMap::new();
        let value = format!(
            "{SESSION_COOKIE}={token}; HttpOnly; {secure_attr}SameSite=Strict; Path=/{age}"
        );
        if let Ok(value) = HeaderValue::from_str(&value) {
            result.insert(header::SET_COOKIE, value);
        }
        result
    }

    fn clear_cookie() -> HeaderMap {
        let mut result = HeaderMap::new();
        result.insert(
            header::SET_COOKIE,
            HeaderValue::from_static(
                "nfqws2_ui_session=; HttpOnly; SameSite=Strict; Path=/; Max-Age=0",
            ),
        );
        result
    }

    fn cookie(headers: &HeaderMap) -> Option<&str> {
        headers
            .get(header::COOKIE)?
            .to_str()
            .ok()?
            .split(';')
            .find_map(|part| part.trim().strip_prefix("nfqws2_ui_session="))
            .filter(|v| !v.is_empty())
    }

    async fn valid_session(&self, headers: &HeaderMap, store: &AuthStore) -> bool {
        let Some(token) = Self::cookie(headers) else {
            return false;
        };
        let now = now_ts();
        store
            .sessions
            .iter()
            .any(|session| session_matches(session, token, now))
    }

    pub async fn status(State(manager): State<Arc<Self>>, headers: HeaderMap) -> Response {
        let store = match manager.load().await {
            Ok(store) => store,
            Err(error) => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({"error":error})),
                )
                    .into_response();
            }
        };
        Json(AuthStatus {
            enabled: store.enabled,
            has_password: store.password_hash.is_some(),
            authenticated: manager.valid_session(&headers, &store).await,
        })
        .into_response()
    }

    pub async fn login(
        State(manager): State<Arc<Self>>,
        headers: HeaderMap,
        Json(request): Json<LoginRequest>,
    ) -> Response {
        let ip = Self::request_ip(&headers);
        {
            let mut attempts = manager.attempts.lock().unwrap_or_else(|e| e.into_inner());
            attempts.retain(|_, (_, last)| last.elapsed() < LOCKOUT * 2);
            if attempts
                .get(&ip)
                .is_some_and(|(_, last)| last.elapsed() >= LOCKOUT)
            {
                attempts.remove(&ip);
            }
            if attempts
                .get(&ip)
                .is_some_and(|(count, _)| *count >= MAX_ATTEMPTS)
            {
                return (StatusCode::TOO_MANY_REQUESTS, Json(serde_json::json!({"error":"Слишком много попыток. Повторите через минуту."}))).into_response();
            }
        }
        let _update = manager.update_lock.lock().await;
        let mut store = match manager.load().await {
            Ok(store) => store,
            Err(error) => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({"error":error})),
                )
                    .into_response();
            }
        };
        let Some(hash) = store.password_hash.clone() else {
            return (
                StatusCode::FORBIDDEN,
                Json(serde_json::json!({"error":"Пароль панели ещё не настроен."})),
            )
                .into_response();
        };
        let password = request.password;
        let valid = tokio::task::spawn_blocking(move || verify_password(&password, &hash))
            .await
            .unwrap_or(false);
        if !valid {
            let mut attempts = manager.attempts.lock().unwrap_or_else(|e| e.into_inner());
            let entry = attempts.entry(ip).or_insert((0, Instant::now()));
            entry.0 += 1;
            entry.1 = Instant::now();
            return (
                StatusCode::UNAUTHORIZED,
                Json(serde_json::json!({"error":"Неверный пароль."})),
            )
                .into_response();
        }
        manager
            .attempts
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .remove(&ip);
        let token = Uuid::new_v4().simple().to_string();
        let now = now_ts();
        store
            .sessions
            .retain(|session| session.expires_at.is_none_or(|expires_at| expires_at > now));
        let remember_days = if request.remember {
            match request.remember_days.unwrap_or(30) {
                days @ (1 | 7 | 30 | 90 | 365) => Some(days),
                0 => Some(0),
                _ => return (StatusCode::BAD_REQUEST, Json(serde_json::json!({"error":"Выберите срок запоминания: 1, 7, 30, 90, 365 дней или никогда."}))).into_response(),
            }
        } else {
            None
        };
        let ttl = remember_days
            .map(|days| u64::from(days) * 24 * 60 * 60)
            .unwrap_or(SESSION_TTL);
        let cookie_age = request
            .remember
            .then_some(if ttl == 0 { 2_147_483_647 } else { ttl });
        store.sessions.push(StoredSession {
            token_hash: hash_token(&token),
            expires_at: (ttl > 0).then_some(now + ttl),
        });
        let expired_count = store.sessions.len().saturating_sub(16);
        if expired_count > 0 {
            store.sessions.drain(..expired_count);
        }
        if let Err(message) = manager.save(&store).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error":message})),
            )
                .into_response();
        }
        (
            Self::set_cookie(&headers, &token, cookie_age),
            Json(serde_json::json!({"success":true})),
        )
            .into_response()
    }

    pub async fn update_settings(
        State(manager): State<Arc<Self>>,
        headers: HeaderMap,
        Json(request): Json<SettingsRequest>,
    ) -> Response {
        let _update = manager.update_lock.lock().await;
        let mut store = match manager.load().await {
            Ok(store) => store,
            Err(error) => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({"error":error})),
                )
                    .into_response();
            }
        };
        if store.enabled {
            let current = request
                .current_password
                .as_deref()
                .unwrap_or_default()
                .to_owned();
            let Some(hash) = store.password_hash.clone() else {
                return (
                    StatusCode::FORBIDDEN,
                    Json(serde_json::json!({"error":"Настроенный пароль отсутствует."})),
                )
                    .into_response();
            };
            let valid = tokio::task::spawn_blocking(move || verify_password(&current, &hash))
                .await
                .unwrap_or(false);
            if !valid {
                return (
                    StatusCode::UNAUTHORIZED,
                    Json(serde_json::json!({"error":"Текущий пароль не подходит."})),
                )
                    .into_response();
            }
        }
        if let Some(password) = request.password.filter(|value| !value.is_empty()) {
            if let Err(message) = validate_password(&password) {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(serde_json::json!({"error":message})),
                )
                    .into_response();
            }
            match tokio::task::spawn_blocking(move || hash_password(&password)).await {
                Ok(Ok(hash)) => store.password_hash = Some(hash),
                _ => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(serde_json::json!({"error":"Не удалось сохранить пароль."})),
                    )
                        .into_response();
                }
            }
        }
        if request.enabled && store.password_hash.is_none() {
            return (
                StatusCode::BAD_REQUEST,
                Json(serde_json::json!({"error":"Сначала задайте пароль панели."})),
            )
                .into_response();
        }
        let was_enabled = store.enabled;
        store.enabled = request.enabled;
        if !store.enabled || !was_enabled {
            store.sessions.clear();
        }
        if let Err(message) = manager.save(&store).await {
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(serde_json::json!({"error":message})),
            )
                .into_response();
        }
        if store.enabled {
            let token = Uuid::new_v4().simple().to_string();
            let now = now_ts();
            store.sessions.push(StoredSession {
                token_hash: hash_token(&token),
                expires_at: Some(now + SESSION_TTL),
            });
            if let Err(message) = manager.save(&store).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error":message})),
                )
                    .into_response();
            }
            return (
                Self::set_cookie(&headers, &token, None),
                Json(serde_json::json!({"saved":true})),
            )
                .into_response();
        }
        (
            Self::clear_cookie(),
            Json(serde_json::json!({"saved":true})),
        )
            .into_response()
    }

    pub async fn logout(State(manager): State<Arc<Self>>, headers: HeaderMap) -> Response {
        let _update = manager.update_lock.lock().await;
        if let Some(token) = Self::cookie(&headers) {
            let token_hash = hash_token(token);
            let mut store = match manager.load().await {
                Ok(store) => store,
                Err(error) => {
                    return (
                        StatusCode::SERVICE_UNAVAILABLE,
                        Json(serde_json::json!({"error":error})),
                    )
                        .into_response();
                }
            };
            store
                .sessions
                .retain(|session| session.token_hash != token_hash);
            if let Err(message) = manager.save(&store).await {
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(serde_json::json!({"error":message})),
                )
                    .into_response();
            }
        }
        (
            Self::clear_cookie(),
            Json(serde_json::json!({"success":true})),
        )
            .into_response()
    }

    pub async fn middleware(
        State(manager): State<Arc<Self>>,
        request: axum::extract::Request,
        next: Next,
    ) -> Response {
        let mut request = request;
        if let Some(ConnectInfo(address)) = request
            .extensions()
            .get::<ConnectInfo<std::net::SocketAddr>>()
            && let Ok(value) = HeaderValue::from_str(&address.ip().to_string())
        {
            request.headers_mut().insert("x-nfqws2-client-ip", value);
        }
        let path = request.uri().path();
        if !path.starts_with("/api/") || path == "/api/auth/status" || path == "/api/auth/login" {
            return next.run(request).await;
        }
        let store = match manager.load().await {
            Ok(store) => store,
            Err(error) => {
                return (
                    StatusCode::SERVICE_UNAVAILABLE,
                    Json(serde_json::json!({"error":error})),
                )
                    .into_response();
            }
        };
        if !store.enabled || manager.valid_session(request.headers(), &store).await {
            return next.run(request).await;
        }
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({"error":"Требуется войти в панель."})),
        )
            .into_response()
    }
}

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn validate_password(password: &str) -> Result<(), String> {
    if password.is_empty() {
        return Err("Пароль не должен быть пустым.".into());
    }
    if password.len() > MAX_PASSWORD_BYTES {
        return Err("Пароль слишком длинный.".into());
    }
    Ok(())
}

fn now_ts() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn hash_token(token: &str) -> String {
    hex::encode(Sha256::digest(token.as_bytes()))
}
fn session_matches(session: &StoredSession, token: &str, now: u64) -> bool {
    use subtle::ConstantTimeEq;
    let provided = hash_token(token);
    session.expires_at.is_none_or(|expires_at| expires_at > now)
        && session
            .token_hash
            .as_bytes()
            .ct_eq(provided.as_bytes())
            .into()
}
fn hash_password(password: &str) -> Result<String, String> {
    let random_salt = Uuid::new_v4().simple().to_string();
    let salt = SaltString::from_b64(&random_salt).map_err(|e| e.to_string())?;
    Argon2::default()
        .hash_password(password.as_bytes(), &salt)
        .map(|hash| hash.to_string())
        .map_err(|e| e.to_string())
}
fn verify_password(password: &str, hash: &str) -> bool {
    PasswordHash::new(hash).ok().is_some_and(|parsed| {
        Argon2::default()
            .verify_password(password.as_bytes(), &parsed)
            .is_ok()
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::{Router, body::Body, routing::get};
    use tower::ServiceExt;
    #[test]
    fn password_policy_accepts_short_passwords_and_caps_input_size() {
        assert!(validate_password("correct horse battery").is_ok());
        assert!(validate_password("1").is_ok());
        assert!(validate_password("short").is_ok());
        assert!(validate_password("").is_err());
        assert!(validate_password("парольпароль").is_ok());
        assert!(validate_password(&"x".repeat(MAX_PASSWORD_BYTES + 1)).is_err());
    }
    #[test]
    fn cookie_session_only_accepts_unexpired_hash_matches() {
        let token = "opaque-session-token";
        let session = StoredSession {
            token_hash: hash_token(token),
            expires_at: Some(200),
        };
        assert!(session_matches(&session, token, 100));
        assert!(!session_matches(&session, token, 200));
        assert!(!session_matches(&session, "another-token", 100));
        let never_expires = StoredSession {
            token_hash: hash_token(token),
            expires_at: None,
        };
        assert!(session_matches(&never_expires, token, u64::MAX));
    }

    #[test]
    fn argon2_password_hash_verifies_only_the_original_password() {
        let hash = hash_password("correct horse battery").unwrap();
        assert!(verify_password("correct horse battery", &hash));
        assert!(!verify_password("incorrect password", &hash));
    }

    #[test]
    fn never_remember_cookie_has_long_lived_persistence_and_revocable_server_session() {
        let headers =
            AuthManager::set_cookie(&HeaderMap::new(), "opaque-token", Some(2_147_483_647));
        let cookie = headers.get(header::SET_COOKIE).unwrap().to_str().unwrap();
        assert!(cookie.contains("Max-Age=2147483647"));
        let session = StoredSession {
            token_hash: hash_token("opaque-token"),
            expires_at: None,
        };
        assert!(session_matches(&session, "opaque-token", u64::MAX));
    }

    #[tokio::test]
    async fn enabled_auth_rejects_api_requests_without_a_valid_cookie() {
        let dir = tempfile::tempdir().unwrap();
        let manager = Arc::new(AuthManager::new(dir.path().to_path_buf()));
        manager
            .save(&AuthStore {
                enabled: true,
                ..AuthStore::default()
            })
            .await
            .unwrap();
        let app = Router::new()
            .route("/api/protected", get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn_with_state(
                manager,
                AuthManager::middleware,
            ));
        let response = app
            .oneshot(
                axum::http::Request::builder()
                    .uri("/api/protected")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(response.status(), StatusCode::UNAUTHORIZED);
    }

    #[tokio::test]
    async fn valid_cookie_session_allows_api_requests() {
        let dir = tempfile::tempdir().unwrap();
        let manager = Arc::new(AuthManager::new(dir.path().to_path_buf()));
        let token = "test-session-token";
        manager
            .save(&AuthStore {
                enabled: true,
                sessions: vec![StoredSession {
                    token_hash: hash_token(token),
                    expires_at: Some(now_ts() + 120),
                }],
                ..AuthStore::default()
            })
            .await
            .unwrap();
        let app = Router::new()
            .route("/api/protected", get(|| async { StatusCode::OK }))
            .layer(axum::middleware::from_fn_with_state(
                manager,
                AuthManager::middleware,
            ));
        let request = axum::http::Request::builder()
            .uri("/api/protected")
            .header(header::COOKIE, format!("{SESSION_COOKIE}={token}"))
            .body(Body::empty())
            .unwrap();
        let response = app.oneshot(request).await.unwrap();
        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn corrupt_auth_store_fails_closed_instead_of_disabling_the_gate() {
        let dir = tempfile::tempdir().unwrap();
        let manager = AuthManager::new(dir.path().to_path_buf());
        tokio::fs::write(dir.path().join("auth.json"), b"not-json")
            .await
            .unwrap();
        assert!(manager.load().await.is_err());
    }
}
