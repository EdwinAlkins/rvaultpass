//! Vault REST service. It opens one `.vault` file and serves it while unlocked.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::{ConnectInfo, Path, Query};
use axum::http::{HeaderMap, Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{delete, get, post, put};
use axum::{Json, Router};
use base64::Engine;
use rvault_application::VaultService;
use rvault_core::{EntryPatch, EntrySummary, PasswordGenerationParams, TOTPResult, VaultFolder, VaultTag};
use rvault_storage::StorageError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::net::TcpListener;
use tokio::sync::oneshot;
use tower_http::cors::{Any, CorsLayer};

pub const DEFAULT_IDLE_SECS: i64 = 900;
const MAX_UNLOCKS_IN_FLIGHT: usize = 2;

#[derive(Clone)]
pub struct ServerConfig {
    pub bind: String,
    pub vault_path: PathBuf,
    pub cors_origins: Vec<String>,
    pub idle_lock_secs: i64,
}

struct Gate {
    vault_path: PathBuf,
    idle_secs: i64,
    service: Option<VaultService>,
    token_hash: Option<String>,
    expires_at: i64,
}

struct UnlockLimit {
    in_flight: usize,
    failures: HashMap<String, u32>,
}

static GATE: Mutex<Option<Gate>> = Mutex::new(None);
static ALLOWED_ORIGINS: Mutex<Vec<String>> = Mutex::new(Vec::new());
static UNLOCK_LIMIT: std::sync::LazyLock<Mutex<UnlockLimit>> = std::sync::LazyLock::new(|| {
    Mutex::new(UnlockLimit {
        in_flight: 0,
        failures: HashMap::new(),
    })
});

pub async fn serve(
    config: ServerConfig,
    shutdown: oneshot::Receiver<()>,
    bound: Option<oneshot::Sender<SocketAddr>>,
) -> Result<(), String> {
    if !config.vault_path.exists() {
        return Err("Vault file not found".to_string());
    }
    if let Ok(mut allowed) = ALLOWED_ORIGINS.lock() {
        *allowed = config.cors_origins.clone();
    }
    {
        let mut gate = GATE.lock().map_err(|_| "Vault session is unavailable".to_string())?;
        *gate = Some(Gate {
            vault_path: config.vault_path.clone(),
            idle_secs: config.idle_lock_secs.max(1),
            service: None,
            token_hash: None,
            expires_at: 0,
        });
    }

    let mut app = Router::new()
        .route("/v1/health", get(health))
        .route("/v1/vault/unlock", post(unlock))
        .route("/v1/vault/lock", post(lock_vault))
        .route("/v1/vault/status", get(status))
        .route("/v1/entries", get(list_entries).post(create_entry))
        .route("/v1/entries/{id}", get(get_entry).put(update_entry).delete(delete_entry))
        .route("/v1/entries/{id}/password", get(reveal_password))
        .route("/v1/entries/{id}/totp", get(reveal_totp))
        .route("/v1/folders", get(list_folders).post(create_folder))
        .route("/v1/folders/{id}", put(update_folder).delete(delete_folder))
        .route("/v1/tags", get(list_tags).post(create_tag))
        .route("/v1/tags/{id}", delete(delete_tag))
        .route("/v1/search", get(search))
        .route("/v1/passwords/generate", post(generate_password))
        .route("/v1/passphrases/generate", post(generate_passphrase))
        .layer(middleware::from_fn(require_local_client));
    let configured = cors_origin_list(&config.cors_origins)?;
    let cors = CorsLayer::new()
        .allow_origin(tower_http::cors::AllowOrigin::predicate(move |origin, _| {
            configured.iter().any(|item| item == origin)
                || origin.to_str().map(origin_allowed).unwrap_or(false)
        }))
        .allow_methods(Any)
        .allow_headers(Any);
    app = app.layer(cors);

    let listener = TcpListener::bind(&config.bind)
        .await
        .map_err(|e| format!("Failed to bind vault server: {e}"))?;
    let addr = listener.local_addr().map_err(|e| format!("Failed to read bind address: {e}"))?;
    if let Some(bound) = bound {
        let _ = bound.send(addr);
    }
    tracing::info!("Vault server listening on {addr}");
    let idle = tokio::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
            expire_if_idle();
        }
    });
    let result = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(async move {
        let _ = shutdown.await;
    })
    .await
    .map_err(|e| format!("Vault server failed: {e}"));
    idle.abort();
    clear_session();
    result
}

async fn require_local_client(request: Request<axum::body::Body>, next: Next) -> Result<Response, ApiError> {
    let host = request
        .headers()
        .get(axum::http::header::HOST)
        .and_then(|value| value.to_str().ok())
        .unwrap_or("");
    if !is_loopback_host(host) {
        return Err(ApiError::forbidden("Host is not allowed"));
    }
    if let Some(origin) = request
        .headers()
        .get(axum::http::header::ORIGIN)
        .and_then(|value| value.to_str().ok())
    {
        if !origin_allowed(origin) {
            return Err(ApiError::forbidden("Origin is not allowed"));
        }
    }
    Ok(next.run(request).await)
}

fn is_loopback_host(host: &str) -> bool {
    matches!(host_name(host), "localhost" | "127.0.0.1" | "::1")
}

fn host_name(value: &str) -> &str {
    let value = value.trim();
    if let Some(rest) = value.strip_prefix('[') {
        return rest.split(']').next().unwrap_or(rest);
    }
    value.split(':').next().unwrap_or(value)
}

fn origin_allowed(origin: &str) -> bool {
    if origin.starts_with("chrome-extension://") || origin.starts_with("moz-extension://") {
        return true;
    }
    if let Ok(allowed) = ALLOWED_ORIGINS.lock() {
        if allowed.iter().any(|item| item == origin) {
            return true;
        }
    }
    let Some(rest) = origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
    else {
        return false;
    };
    is_loopback_host(rest)
}

fn cors_origin_list(configured: &[String]) -> Result<Vec<axum::http::HeaderValue>, String> {
    let mut names = Vec::new();
    for origin in configured {
        push_unique(&mut names, origin.clone());
        if let Some(alias) = loopback_alias(origin) {
            push_unique(&mut names, alias);
        }
    }
    names
        .into_iter()
        .map(|origin| origin.parse().map_err(|_| "Invalid CORS origin".to_string()))
        .collect()
}

fn push_unique(names: &mut Vec<String>, origin: String) {
    if !names.iter().any(|existing| existing == &origin) {
        names.push(origin);
    }
}

fn loopback_alias(origin: &str) -> Option<String> {
    let (scheme, rest) = if let Some(rest) = origin.strip_prefix("http://") {
        ("http", rest)
    } else if let Some(rest) = origin.strip_prefix("https://") {
        ("https", rest)
    } else {
        return None;
    };
    let (host, port) = rest.split_once(':')?;
    if port.contains('/') {
        return None;
    }
    let other = match host {
        "127.0.0.1" => "localhost",
        "localhost" => "127.0.0.1",
        _ => return None,
    };
    Some(format!("{scheme}://{other}:{port}"))
}

#[cfg(test)]
mod cors_tests {
    use super::*;

    #[test]
    fn loopback_spellings_are_both_allowed() {
        let origins = cors_origin_list(&["http://127.0.0.1:1421".to_string()]).unwrap();
        let text: Vec<_> = origins.iter().map(|value| value.to_str().unwrap().to_string()).collect();
        assert!(text.contains(&"http://127.0.0.1:1421".to_string()));
        assert!(text.contains(&"http://localhost:1421".to_string()));
    }

    #[test]
    fn extension_origins_are_allowed_without_configuration() {
        assert!(origin_allowed("moz-extension://4b1c2d3e-0000-4000-8000-000000000000"));
        assert!(origin_allowed("chrome-extension://abcdefghijklmnopabcdefghijklmnop"));
        assert!(!origin_allowed("https://example.com"));
    }

    #[test]
    fn host_must_be_loopback() {
        assert!(is_loopback_host("127.0.0.1:8443"));
        assert!(is_loopback_host("[::1]:8443"));
        assert!(!is_loopback_host("example.com"));
    }
}

fn expire_if_idle() {
    let Ok(mut guard) = GATE.lock() else {
        return;
    };
    let Some(gate) = guard.as_mut() else {
        return;
    };
    if gate.service.is_some() && unix_now() >= gate.expires_at {
        match gate.service.as_ref().map(|service| service.close()) {
            Some(Ok(())) => {
                gate.service = None;
                gate.token_hash = None;
                gate.expires_at = 0;
                tracing::info!("Vault locked after inactivity");
            }
            Some(Err(error)) => tracing::error!("Idle lock kept the vault open: {error}"),
            None => {}
        }
    }
}

fn clear_session() {
    if let Ok(mut guard) = GATE.lock() {
        if let Some(gate) = guard.as_mut() {
            if let Some(service) = gate.service.as_ref() {
                if let Err(error) = service.close() {
                    tracing::error!("Failed to persist vault before shutdown: {error}");
                }
            }
            gate.service = None;
            gate.token_hash = None;
            gate.expires_at = 0;
        }
    }
}

#[derive(Serialize)]
struct Health {
    status: &'static str,
}

async fn health() -> Json<Health> {
    Json(Health { status: "ok" })
}

#[derive(Deserialize)]
struct UnlockBody {
    password: String,
}

#[derive(Serialize)]
struct SessionResponse {
    token: String,
    expires_at: i64,
}

async fn unlock(
    ConnectInfo(peer): ConnectInfo<SocketAddr>,
    Json(body): Json<UnlockBody>,
) -> Result<Json<SessionResponse>, ApiError> {
    let delay = begin_unlock(&peer)?;
    let password = body.password;
    let result = blocking(move || {
        if delay > 0 {
            std::thread::sleep(std::time::Duration::from_millis(delay));
        }
        unlock_blocking(password)
    })
    .await;
    finish_unlock(&peer, result.is_ok());
    result.map(Json)
}

fn begin_unlock(peer: &SocketAddr) -> Result<u64, ApiError> {
    let mut limit = UNLOCK_LIMIT.lock().map_err(|_| ApiError::internal("Unlock limiter is unavailable"))?;
    if limit.in_flight >= MAX_UNLOCKS_IN_FLIGHT {
        return Err(ApiError::too_many("Too many unlock attempts"));
    }
    let failures = limit.failures.get(&peer.ip().to_string()).copied().unwrap_or(0);
    if failures >= 8 {
        return Err(ApiError::too_many("Too many unlock attempts"));
    }
    limit.in_flight += 1;
    Ok(u64::from(failures) * 250)
}

fn finish_unlock(peer: &SocketAddr, success: bool) {
    if let Ok(mut limit) = UNLOCK_LIMIT.lock() {
        limit.in_flight = limit.in_flight.saturating_sub(1);
        let key = peer.ip().to_string();
        if success {
            limit.failures.remove(&key);
        } else {
            let failures = limit.failures.entry(key).or_insert(0);
            *failures = failures.saturating_add(1);
        }
    }
}

fn unlock_blocking(password: String) -> Result<SessionResponse, ApiError> {
    let mut guard = gate_lock()?;
    let gate = guard.as_mut().ok_or_else(|| ApiError::internal("Vault server is not ready"))?;
    if gate.service.as_ref().is_some_and(|service| service.is_open()) {
        let accepted = gate
            .service
            .as_ref()
            .and_then(|service| service.accepts_password(&password).ok())
            .unwrap_or(false);
        if !accepted {
            tracing::info!("Vault unlock rejected");
            return Err(ApiError::unauthorized("Unlock failed"));
        }
        return Ok(issue_token(gate));
    }
    let path = gate.vault_path.clone();
    let service = match VaultService::open(&path, &password) {
        Ok(service) => service,
        Err(error) => {
            tracing::info!("Vault unlock rejected: {error}");
            return Err(ApiError::unauthorized("Unlock failed"));
        }
    };
    gate.service = Some(service);
    Ok(issue_token(gate))
}

fn issue_token(gate: &mut Gate) -> SessionResponse {
    let mut raw = [0u8; 32];
    rand::RngCore::fill_bytes(&mut rand::rngs::OsRng, &mut raw);
    let token = base64::engine::general_purpose::STANDARD.encode(raw);
    let expires_at = unix_now() + gate.idle_secs;
    gate.token_hash = Some(token_hash(&token));
    gate.expires_at = expires_at;
    SessionResponse { token, expires_at }
}

async fn lock_vault(headers: HeaderMap) -> Result<StatusCode, ApiError> {
    blocking(move || {
        let mut guard = gate_lock()?;
        authenticate_guard(guard.as_mut().ok_or_else(|| ApiError::unauthorized("Vault is locked"))?, &headers)?;
        let gate = guard.as_mut().ok_or_else(|| ApiError::unauthorized("Vault is locked"))?;
        if let Some(service) = gate.service.as_ref() {
            service.close().map_err(|error| ApiError::internal(error))?;
        }
        gate.service = None;
        gate.token_hash = None;
        gate.expires_at = 0;
        Ok(StatusCode::NO_CONTENT)
    })
    .await
}

#[derive(Serialize)]
struct StatusBody {
    open: bool,
    expires_at: i64,
}

async fn status(headers: HeaderMap) -> Result<Json<StatusBody>, ApiError> {
    blocking(move || {
        let mut guard = gate_lock()?;
        authenticate_guard(guard.as_mut().ok_or_else(|| ApiError::unauthorized("Vault is locked"))?, &headers)?;
        let gate = guard.as_ref().ok_or_else(|| ApiError::unauthorized("Vault is locked"))?;
        Ok(Json(StatusBody {
            open: gate.service.as_ref().is_some_and(|service| service.is_open()),
            expires_at: gate.expires_at,
        }))
    })
    .await
}

async fn list_entries(headers: HeaderMap) -> Result<Json<Vec<EntrySummary>>, ApiError> {
    with_service(headers, |service| service.list_entries()).await.map(Json)
}

#[derive(Deserialize)]
struct EntryBody {
    folder_id: Option<String>,
    title: String,
    url: Option<String>,
    username: Option<String>,
    username2: Option<String>,
    username3: Option<String>,
    password: String,
    notes: Option<String>,
    totp_secret: Option<String>,
    is_favorite: Option<bool>,
    tag_ids: Option<Vec<String>>,
}

async fn create_entry(headers: HeaderMap, Json(body): Json<EntryBody>) -> Result<Json<EntrySummary>, ApiError> {
    with_service(headers, move |service| {
        service.create_entry(
            body.folder_id,
            body.title,
            body.url,
            body.username,
            body.username2,
            body.username3,
            body.password,
            body.notes,
            body.totp_secret,
            body.is_favorite.unwrap_or(false),
            body.tag_ids,
        )
    })
    .await
    .map(Json)
}

async fn get_entry(headers: HeaderMap, Path(id): Path<String>) -> Result<Json<rvault_core::EntryEditData>, ApiError> {
    with_service(headers, move |service| service.edit_entry(&id)).await.map(Json)
}

fn deserialize_some<'de, T, D>(deserializer: D) -> Result<Option<T>, D::Error>
where
    T: Deserialize<'de>,
    D: serde::Deserializer<'de>,
{
    T::deserialize(deserializer).map(Some)
}

#[derive(Deserialize)]
struct EntryUpdate {
    #[serde(default)]
    title: Option<String>,
    #[serde(default, deserialize_with = "deserialize_some")]
    folder_id: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    url: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    username: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    username2: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    username3: Option<Option<String>>,
    #[serde(default)]
    password: Option<String>,
    #[serde(default, deserialize_with = "deserialize_some")]
    notes: Option<Option<String>>,
    #[serde(default, deserialize_with = "deserialize_some")]
    totp_secret: Option<Option<String>>,
    #[serde(default)]
    is_favorite: Option<bool>,
    #[serde(default)]
    tag_ids: Option<Vec<String>>,
}

async fn update_entry(
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<EntryUpdate>,
) -> Result<Json<EntrySummary>, ApiError> {
    with_service(headers, move |service| {
        let patch = EntryPatch {
            folder_id: body.folder_id,
            title: body.title,
            url: body.url,
            username: body.username,
            username2: body.username2,
            username3: body.username3,
            password: body.password.filter(|value| !value.is_empty()),
            notes: body.notes,
            totp_secret: body.totp_secret,
            is_favorite: body.is_favorite,
            tag_ids: body.tag_ids,
        };
        service.update_entry(id, patch)
    })
    .await
    .map(Json)
}

async fn delete_entry(headers: HeaderMap, Path(id): Path<String>) -> Result<StatusCode, ApiError> {
    with_service(headers, move |service| service.delete_entry(&id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Serialize)]
struct PasswordBody {
    password: String,
}

async fn reveal_password(headers: HeaderMap, Path(id): Path<String>) -> Result<Json<PasswordBody>, ApiError> {
    with_service(headers, move |service| service.reveal_password(&id))
        .await
        .map(|password| Json(PasswordBody { password }))
}

async fn reveal_totp(headers: HeaderMap, Path(id): Path<String>) -> Result<Json<TOTPResult>, ApiError> {
    with_service(headers, move |service| service.generate_totp(&id)).await.map(Json)
}

async fn list_folders(headers: HeaderMap) -> Result<Json<Vec<VaultFolder>>, ApiError> {
    with_service(headers, |service| service.list_folders()).await.map(Json)
}

#[derive(Deserialize)]
struct FolderBody {
    name: String,
    parent_id: Option<String>,
}

async fn create_folder(headers: HeaderMap, Json(body): Json<FolderBody>) -> Result<Json<VaultFolder>, ApiError> {
    with_service(headers, move |service| service.create_folder(body.name, body.parent_id))
        .await
        .map(Json)
}

async fn update_folder(
    headers: HeaderMap,
    Path(id): Path<String>,
    Json(body): Json<FolderBody>,
) -> Result<Json<VaultFolder>, ApiError> {
    with_service(headers, move |service| service.update_folder(&id, body.name, body.parent_id))
        .await
        .map(Json)
}

async fn delete_folder(headers: HeaderMap, Path(id): Path<String>) -> Result<StatusCode, ApiError> {
    with_service(headers, move |service| service.delete_folder(&id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

async fn list_tags(headers: HeaderMap) -> Result<Json<Vec<VaultTag>>, ApiError> {
    with_service(headers, |service| service.list_tags()).await.map(Json)
}

#[derive(Deserialize)]
struct TagBody {
    name: String,
}

async fn create_tag(headers: HeaderMap, Json(body): Json<TagBody>) -> Result<Json<VaultTag>, ApiError> {
    with_service(headers, move |service| service.create_tag(body.name)).await.map(Json)
}

async fn delete_tag(headers: HeaderMap, Path(id): Path<String>) -> Result<StatusCode, ApiError> {
    with_service(headers, move |service| service.delete_tag(&id)).await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize)]
struct SearchQuery {
    q: String,
}

async fn search(headers: HeaderMap, Query(query): Query<SearchQuery>) -> Result<Json<Vec<EntrySummary>>, ApiError> {
    with_service(headers, move |service| service.search(&query.q)).await.map(Json)
}

#[derive(Serialize)]
struct GeneratedPassword {
    password: String,
}

async fn generate_password(
    headers: HeaderMap,
    Json(params): Json<PasswordGenerationParams>,
) -> Result<Json<GeneratedPassword>, ApiError> {
    with_service(headers, move |_| {
        rvault_application::generate_password(&params).map_err(StorageError::Invalid)
    })
    .await
    .map(|password| Json(GeneratedPassword { password }))
}

#[derive(Deserialize)]
struct PassphraseBody {
    word_count: usize,
}

#[derive(Serialize)]
struct GeneratedPassphrase {
    passphrase: String,
}

async fn generate_passphrase(
    headers: HeaderMap,
    Json(body): Json<PassphraseBody>,
) -> Result<Json<GeneratedPassphrase>, ApiError> {
    with_service(headers, move |_| {
        rvault_application::generate_passphrase(body.word_count).map_err(StorageError::Invalid)
    })
    .await
    .map(|passphrase| Json(GeneratedPassphrase { passphrase }))
}

async fn with_service<T: Send + 'static>(
    headers: HeaderMap,
    work: impl FnOnce(&VaultService) -> Result<T, StorageError> + Send + 'static,
) -> Result<T, ApiError> {
    blocking(move || {
        let mut guard = gate_lock()?;
        authenticate_guard(
            guard.as_mut().ok_or_else(|| ApiError::unauthorized("Vault is locked"))?,
            &headers,
        )?;
        let gate = guard.as_ref().ok_or_else(|| ApiError::unauthorized("Vault is locked"))?;
        let service = gate.service.as_ref().ok_or_else(|| ApiError::unauthorized("Vault is locked"))?;
        work(service).map_err(ApiError::from_storage)
    })
    .await
}

fn authenticate_guard(gate: &mut Gate, headers: &HeaderMap) -> Result<(), ApiError> {
    let header = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .ok_or_else(|| ApiError::unauthorized("Vault is locked"))?;
    let token = header
        .strip_prefix("Bearer ")
        .ok_or_else(|| ApiError::unauthorized("Vault is locked"))?;
    let now = unix_now();
    let open = gate.service.as_ref().is_some_and(|service| service.is_open());
    let valid = open && gate.expires_at > now && gate.token_hash.as_deref() == Some(token_hash(token).as_str());
    if !valid {
        if open && now >= gate.expires_at {
            match gate.service.as_ref().map(|service| service.close()) {
                Some(Ok(())) => {
                    gate.service = None;
                    gate.token_hash = None;
                    gate.expires_at = 0;
                }
                Some(Err(error)) => {
                    tracing::error!("Expired session kept the vault open: {error}");
                    return Err(ApiError::conflict("Vault has unsaved changes"));
                }
                None => {}
            }
        }
        return Err(ApiError::unauthorized("Vault is locked"));
    }
    gate.expires_at = now + gate.idle_secs;
    Ok(())
}

async fn blocking<T: Send + 'static>(work: impl FnOnce() -> Result<T, ApiError> + Send + 'static) -> Result<T, ApiError> {
    tokio::task::spawn_blocking(work)
        .await
        .map_err(|_| ApiError::internal("Vault worker stopped"))?
}

fn gate_lock() -> Result<std::sync::MutexGuard<'static, Option<Gate>>, ApiError> {
    GATE.lock().map_err(|_| ApiError::internal("Vault session is unavailable"))
}

fn unix_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or(0)
}

fn token_hash(token: &str) -> String {
    let digest = Sha256::digest(token.as_bytes());
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

struct ApiError {
    status: StatusCode,
    message: String,
}

impl ApiError {
    fn status(status: StatusCode, message: impl Into<String>) -> Self {
        Self { status, message: message.into() }
    }

    fn unauthorized(message: impl Into<String>) -> Self {
        Self::status(StatusCode::UNAUTHORIZED, message)
    }

    fn forbidden(message: impl Into<String>) -> Self {
        Self::status(StatusCode::FORBIDDEN, message)
    }

    fn conflict(message: impl Into<String>) -> Self {
        Self::status(StatusCode::CONFLICT, message)
    }

    fn too_many(message: impl Into<String>) -> Self {
        Self::status(StatusCode::TOO_MANY_REQUESTS, message)
    }

    fn internal(message: impl Into<String>) -> Self {
        Self::status(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    fn from_storage(error: StorageError) -> Self {
        match error {
            StorageError::NotFound(message) => Self::status(StatusCode::NOT_FOUND, message),
            StorageError::Conflict(message) => Self::conflict(message),
            StorageError::Invalid(message) => Self::status(StatusCode::UNPROCESSABLE_ENTITY, message),
            StorageError::Internal(message) => Self::internal(message),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body = Json(serde_json::json!({ "error": self.message }));
        (self.status, body).into_response()
    }
}
