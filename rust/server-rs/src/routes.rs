//! Rutas REST del servidor DistroForge (réplica exacta de server/src/routes.ts).
//!
//!   POST   /api/register {name, password}     → crea cuenta y loguea
//!   POST   /api/login    {name, password}     → loguea (cookie df_session)
//!   POST   /api/logout                         → cierra sesión
//!   GET    /api/me                             → usuario actual o null
//!   GET    /api/packs?query=                   → búsqueda server-side (FTS5)
//!   POST   /api/packs   {name, desc, json}     → publica un pack (auth)
//!   POST   /api/rating  {packId, stars}        → valora un pack (auth)
//!   GET    /api/testdrive/available            → ¿hay QEMU en el host?
//!   POST   /api/testdrive {imagePath}          → arranca VM (auth)
//!   GET    /api/testdrive                      → sesiones del usuario
//!   DELETE /api/testdrive/{id}                 → apaga la VM
//!   GET    /vnc.html                           → cliente noVNC
//!   GET    /ws/{id}                            → puente WebSocket → VNC
//!
//! El cliente noVNC se sirve en /vnc.html y conecta con /ws/{sessionId}.

use std::collections::HashMap;
use std::path::{Path as FsPath, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{DefaultBodyLimit, Path, Query, State};
use axum::http::{header, HeaderMap, Method, Request, StatusCode};
use axum::middleware::{self, Next};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, delete, get, post};
use axum::{Json, Router};
use rusqlite::Connection;
use serde_json::{json, Value};
use tower_http::cors::{AllowHeaders, AllowOrigin, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

use crate::auth::{self, CurrentUser};
use crate::db::{self, PackRow};
use crate::testdrive::DriveManager;
use crate::vncbridge;

const COOKIE: &str = "df_session";
const SESSION_MAX_AGE: u32 = 30 * 24 * 60 * 60; // 30 días, igual que la sesión en BD
const DEFAULT_MEM_MB: u32 = 2048;
/// TTL máximo de una sesión de test drive.
pub const TTL: Duration = Duration::from_secs(15 * 60);

/// Límite del cuerpo JSON de la API (los packs son recetas, no binarios).
const MAX_BODY_BYTES: usize = 256 * 1024;
/// Límite de packs devueltos por búsqueda.
const PACKS_LIMIT: i64 = 50;
/// Sesiones de test drive simultáneas por usuario y en total.
const MAX_SESSIONS_PER_USER: usize = 2;
const MAX_SESSIONS_TOTAL: usize = 6;
/// Ventana de los intentos de login/registro.
const RATE_WINDOW: Duration = Duration::from_secs(60);
const RATE_MAX_LOGIN: usize = 10;
const RATE_MAX_REGISTER: usize = 5;
/// Caché de `qemu-system-*` disponible (evita lanzar un proceso por petición).
const AVAILABLE_TTL: Duration = Duration::from_secs(30);

/// Límite de intentos por clave en una ventana deslizante (en memoria; una
/// instancia, como el resto de la sesión).
#[derive(Default)]
struct RateLimiter {
    hits: Mutex<HashMap<String, Vec<Instant>>>,
}

impl RateLimiter {
    fn check(&self, key: String, max: usize) -> bool {
        let now = Instant::now();
        let mut hits = self.hits.lock().unwrap_or_else(|e| e.into_inner());
        // Purga global barata y acotada: cientos de claves como mucho.
        if hits.len() > 4096 {
            hits.retain(|_, v| v.iter().any(|t| now.duration_since(*t) < RATE_WINDOW));
        }
        let entry = hits.entry(key).or_default();
        entry.retain(|t| now.duration_since(*t) < RATE_WINDOW);
        if entry.len() >= max {
            return false;
        }
        entry.push(now);
        true
    }
}

/// Nombre válido: 3-32 chars de [a-zA-Z0-9_.-] (igual que NAME_RE en TS).
fn valid_name(name: &str) -> bool {
    (3..=32).contains(&name.chars().count())
        && name
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '_' || c == '.' || c == '-')
}

#[derive(Clone)]
pub struct AppState {
    db: Arc<Mutex<Connection>>,
    drives: DriveManager,
    limiter: Arc<RateLimiter>,
    /// Directorio permitido para las ISOs del test drive (env `DF_ISO_DIR`);
    /// `None` = solo rutas absolutas existentes con extensión de imagen.
    iso_dir: Option<Arc<PathBuf>>,
    available_cache: Arc<Mutex<Option<(bool, Instant)>>>,
}

impl AppState {
    /// Candado de la BD tolerante a envenenamiento: SQLite sigue siendo válido
    /// y un pánico pasado no debe tumbar el servicio para siempre.
    fn db(&self) -> std::sync::MutexGuard<'_, Connection> {
        self.db.lock().unwrap_or_else(|e| e.into_inner())
    }
}

/// Construye el router completo (equivalente a `buildServer` en TS).
pub fn build_router(db: Connection, qemu_bin: &str) -> Router {
    build_router_with_drives(db, DriveManager::new(qemu_bin, TTL))
}

/// Igual que `build_router` pero reutiliza un `DriveManager` existente
/// (permite al `main` barrear sesiones caducadas con el mismo gestor).
pub fn build_router_with_drives(db: Connection, drives: DriveManager) -> Router {
    let iso_dir = std::env::var("DF_ISO_DIR")
        .ok()
        .filter(|s| !s.is_empty())
        .and_then(|s| std::fs::canonicalize(s).ok())
        .map(Arc::new);
    let state = AppState {
        db: Arc::new(Mutex::new(db)),
        drives,
        limiter: Arc::new(RateLimiter::default()),
        iso_dir,
        available_cache: Arc::new(Mutex::new(None)),
    };
    // la app (Pages/localhost) llama cross-origin con cookie de sesión
    // (equivale a fastifyCors { origin: true, credentials: true }; los headers
    // se espejan porque la spec CORS prohibe "*" junto a credentials)
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::mirror_request())
        .allow_headers(AllowHeaders::mirror_request())
        .allow_credentials(true)
        .allow_methods([
            Method::GET,
            Method::HEAD,
            Method::PUT,
            Method::PATCH,
            Method::POST,
            Method::DELETE,
        ]);
    Router::new()
        .route("/api/register", post(register))
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
        .route("/api/me", get(me))
        .route("/api/packs", get(list_packs).post(publish_pack))
        .route("/api/rating", post(rating))
        .route("/api/testdrive", get(list_sessions).post(start_session))
        .route("/api/testdrive/available", get(available))
        .route("/api/testdrive/{id}", delete(stop_session))
        .route("/api/health", get(health))
        .route("/vnc.html", get(vnc_page))
        .route("/ws/{id}", get(ws_bridge))
        // nada de cuerpos gigantes en la API
        .layer(DefaultBodyLimit::max(MAX_BODY_BYTES))
        .layer(middleware::from_fn(security_headers))
        .layer(cors)
        .with_state(state)
}

/// Cabeceras de seguridad en todas las respuestas. La página de noVNC carga su
/// cliente desde CDN y usa un script inline, así que se excluye de la CSP.
async fn security_headers(req: Request<axum::body::Body>, next: Next) -> Response {
    let path = req.uri().path().to_string();
    let mut res = next.run(req).await;
    let h = res.headers_mut();
    h.insert(header::X_CONTENT_TYPE_OPTIONS, "nosniff".parse().unwrap());
    h.insert(header::REFERRER_POLICY, "no-referrer".parse().unwrap());
    h.insert(header::X_FRAME_OPTIONS, "DENY".parse().unwrap());
    if path != "/vnc.html" && !path.starts_with("/ws/") {
        h.insert(
            "content-security-policy",
            "default-src 'self'; script-src 'self' 'wasm-unsafe-eval'; \
             style-src 'self' 'unsafe-inline'; img-src 'self' data: blob:; \
             connect-src 'self'; object-src 'none'; base-uri 'none'; \
             form-action 'self'; frame-ancestors 'none'"
                .parse()
                .unwrap(),
        );
    }
    res
}

/// Estado del servicio. Lo usan el `HEALTHCHECK` del contenedor y las
/// plataformas de hosting para saber si el proceso está vivo.
async fn health() -> Response {
    Json(json!({
        "status": "ok",
        "service": "distroforge-server",
        "version": env!("CARGO_PKG_VERSION"),
    }))
    .into_response()
}

/// Igual que `build_router_with_drives`, pero sirviendo además la interfaz web
/// desde `web_dir`.
///
/// Servir la UI y la API en el **mismo origen** no es un capricho: la sesión
/// viaja en una cookie `SameSite=Lax`, así que con la UI en otro dominio (por
/// ejemplo GitHub Pages) el navegador no la envía y el login no se mantiene. Con
/// la UI servida desde aquí, cuentas, galería con sesión y Test Drive funcionan
/// sin configurar nada más.
///
/// Si `web_dir` es `None` o no existe, el router es exactamente el de antes (API
/// pura): los tests y el uso como solo-backend no cambian.
pub fn build_router_with_ui(
    db: Connection,
    drives: DriveManager,
    web_dir: Option<std::path::PathBuf>,
) -> Router {
    let api = build_router_with_drives(db, drives)
        // una ruta /api desconocida debe seguir siendo un 404 en JSON, no el
        // index.html de la interfaz
        .route("/api/{*resto}", any(api_desconocido));

    match web_dir.filter(|dir| dir.is_dir()) {
        Some(dir) => {
            let index = dir.join("index.html");
            api.fallback_service(ServeDir::new(&dir).not_found_service(ServeFile::new(index)))
        }
        None => api,
    }
}

/// 404 en JSON para rutas de API que no existen.
async fn api_desconocido() -> Response {
    error(StatusCode::NOT_FOUND, "ruta no encontrada")
}

// ---------- helpers ----------

fn error(status: StatusCode, msg: &str) -> Response {
    (status, Json(json!({ "error": msg }))).into_response()
}

fn cookie_value(headers: &HeaderMap, name: &str) -> Option<String> {
    for raw in headers.get_all(header::COOKIE) {
        let raw = raw.to_str().ok()?;
        for part in raw.split(';') {
            let part = part.trim();
            if let Some(value) = part.strip_prefix(&format!("{name}=")) {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn set_cookie(token: &str, secure: bool) -> String {
    format!(
        "{COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={SESSION_MAX_AGE}{}",
        if secure { "; Secure" } else { "" }
    )
}

fn clear_cookie(secure: bool) -> String {
    format!(
        "{COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT{}",
        if secure { "; Secure" } else { "" }
    )
}

/// ¿La petición llegó por HTTPS? (túnel/proxy delante del servidor).
fn wants_secure(headers: &HeaderMap) -> bool {
    headers
        .get("x-forwarded-proto")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("https"))
        .unwrap_or(false)
}

/// Usuario actual: cookie df_session O header Authorization: Bearer <token>.
fn current_user(state: &AppState, headers: &HeaderMap) -> Option<CurrentUser> {
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));
    // Un token que no tiene la forma esperada ni siquiera consulta la BD.
    let token = match bearer {
        Some(t) if auth::valid_token(t) => t.to_owned(),
        _ => cookie_value(headers, COOKIE).filter(|t| auth::valid_token(t))?,
    };
    let db = state.db();
    auth::user_for_token(&db, &token)
}

/// body.name como string no vacío (truthiness de JS aproximada).
fn name_field(body: &Value) -> Option<&str> {
    body.get("name")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
}

fn is_unique_violation(e: &rusqlite::Error) -> bool {
    matches!(
        e,
        rusqlite::Error::SqliteFailure(f, _)
            if f.code == rusqlite::ErrorCode::ConstraintViolation
    )
}

/// Error interno sin filtrar detalles de SQL/ficheros al cliente.
fn internal(err: impl std::fmt::Display) -> Response {
    eprintln!("[distroforge] error interno: {err}");
    error(
        StatusCode::INTERNAL_SERVER_ERROR,
        "error interno del servidor",
    )
}

// ---------- auth ----------

async fn register(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    let name = body.get("name").and_then(Value::as_str).unwrap_or("");
    let password = body.get("password").and_then(Value::as_str).unwrap_or("");
    if name.is_empty() || !valid_name(name) {
        return error(
            StatusCode::BAD_REQUEST,
            "nombre inválido (3-32, letras/números/_.-)",
        );
    }
    let pw_len = password.chars().count();
    if pw_len < 8 {
        return error(
            StatusCode::BAD_REQUEST,
            "contraseña de al menos 8 caracteres",
        );
    }
    if pw_len > 128 || password.chars().any(char::is_control) {
        return error(
            StatusCode::BAD_REQUEST,
            "contraseña inválida (máx. 128, sin controles)",
        );
    }
    if !state
        .limiter
        .check(format!("reg:{name}"), RATE_MAX_REGISTER)
    {
        return error(
            StatusCode::TOO_MANY_REQUESTS,
            "demasiados intentos, espera un minuto",
        );
    }
    // scrypt fuera del candado de la BD y del hilo de la runtime.
    let pw = password.to_string();
    let hash = match tokio::task::spawn_blocking(move || auth::hash_password(&pw)).await {
        Ok(h) => h,
        Err(_) => return internal("spawn_blocking hash"),
    };
    let user_id = {
        let db = state.db();
        auth::create_user_hashed(&db, name, &hash)
    };
    let user_id = match user_id {
        Ok(id) => id,
        Err(e) if is_unique_violation(&e) => {
            return error(StatusCode::CONFLICT, "ese nombre ya existe");
        }
        Err(e) => return internal(e),
    };
    let token = {
        let db = state.db();
        auth::create_session(&db, user_id)
    };
    let token = match token {
        Ok(t) => t,
        Err(e) => return internal(e),
    };
    let mut resp = Json(json!({ "name": name, "token": token })).into_response();
    resp.headers_mut().append(
        header::SET_COOKIE,
        set_cookie(&token, false).parse().unwrap(),
    );
    resp
}

async fn login(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let name = body.get("name").and_then(Value::as_str).unwrap_or("");
    let password = body.get("password").and_then(Value::as_str).unwrap_or("");
    if name.is_empty() || !valid_name(name) || password.is_empty() {
        return error(StatusCode::UNAUTHORIZED, "usuario o contraseña incorrectos");
    }
    if !state.limiter.check(format!("login:{name}"), RATE_MAX_LOGIN) {
        return error(
            StatusCode::TOO_MANY_REQUESTS,
            "demasiados intentos, espera un minuto",
        );
    }
    // Lee el hash con el candado y suéltalo: la verificación es cara.
    let row = {
        let db = state.db();
        auth::find_user(&db, name)
    };
    let Some(row) = row else {
        return error(StatusCode::UNAUTHORIZED, "usuario o contraseña incorrectos");
    };
    let pw = password.to_string();
    let stored = row.pass_hash.clone();
    let ok = tokio::task::spawn_blocking(move || auth::verify_password(&pw, &stored))
        .await
        .unwrap_or(false);
    if !ok {
        return error(StatusCode::UNAUTHORIZED, "usuario o contraseña incorrectos");
    }
    let token = {
        let db = state.db();
        auth::create_session(&db, row.id)
    };
    let token = match token {
        Ok(t) => t,
        Err(e) => return internal(e),
    };
    let mut resp = Json(json!({ "name": row.name, "token": token })).into_response();
    resp.headers_mut().append(
        header::SET_COOKIE,
        set_cookie(&token, wants_secure(&headers)).parse().unwrap(),
    );
    resp
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let token = cookie_value(&headers, COOKIE);
    {
        let db = state.db();
        auth::destroy_session(&db, token.as_deref());
    }
    let mut resp = Json(json!({ "ok": true })).into_response();
    resp.headers_mut().append(
        header::SET_COOKIE,
        clear_cookie(wants_secure(&headers)).parse().unwrap(),
    );
    resp
}

async fn me(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let user = current_user(&state, &headers)
        .map(|u| json!({ "id": u.id, "name": u.name }))
        .unwrap_or(Value::Null);
    Json(json!({ "user": user })).into_response()
}

// ---------- galería ----------

#[derive(serde::Deserialize, Default)]
struct PacksQuery {
    query: Option<String>,
}

async fn list_packs(State(state): State<AppState>, Query(q): Query<PacksQuery>) -> Response {
    // Una consulta gigante no aporta nada y engorda el trabajo de FTS.
    let query = q.query.as_deref().unwrap_or("");
    let query: String = query.chars().take(128).collect();
    let (rows, ratings) = {
        let db = state.db();
        let rows = db::search_packs(&db, &query, PACKS_LIMIT);
        let mut ratings: Vec<(i64, f64, i64)> = Vec::new();
        if let Ok(mut stmt) = db.prepare(
            "SELECT pack_id, ROUND(AVG(stars), 1) AS avg, COUNT(*) AS n
             FROM ratings GROUP BY pack_id",
        ) {
            if let Ok(mapped) =
                stmt.query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
            {
                ratings.extend(mapped.flatten());
            }
        }
        (rows, ratings)
    };
    let by_id: std::collections::HashMap<i64, (f64, i64)> = ratings
        .into_iter()
        .map(|(id, avg, n)| (id, (avg, n)))
        .collect();
    let packs: Vec<Value> = rows
        .into_iter()
        .map(|p: PackRow| {
            let json: Value = serde_json::from_str(&p.json).unwrap_or(Value::Null);
            let rating = match by_id.get(&p.id) {
                Some((avg, n)) => json!({ "avg": avg, "n": n }),
                None => json!({ "avg": 0, "n": 0 }),
            };
            json!({
                "id": p.id,
                "owner": p.owner,
                "name": p.name,
                "desc": p.desc,
                "json": json,
                "created": p.created,
                "rating": rating,
            })
        })
        .collect();
    Json(json!({ "packs": packs })).into_response()
}

async fn publish_pack(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let Some(me) = current_user(&state, &headers) else {
        return error(StatusCode::UNAUTHORIZED, "haz login primero");
    };
    let Some(name) = name_field(&body) else {
        return error(
            StatusCode::BAD_REQUEST,
            "pack inválido: falta name o packages[]",
        );
    };
    if name.chars().count() > 64 {
        return error(
            StatusCode::BAD_REQUEST,
            "nombre de pack demasiado largo (máx. 64)",
        );
    }
    let packages_ok = body
        .get("json")
        .and_then(|j| j.get("packages"))
        .map(Value::is_array)
        .unwrap_or(false);
    if !packages_ok {
        return error(
            StatusCode::BAD_REQUEST,
            "pack inválido: falta name o packages[]",
        );
    }
    let desc = body.get("desc").and_then(Value::as_str).unwrap_or("");
    if desc.chars().count() > 2048 {
        return error(
            StatusCode::BAD_REQUEST,
            "descripción demasiado larga (máx. 2048)",
        );
    }
    let Some(json_value) = body.get("json") else {
        return error(StatusCode::BAD_REQUEST, "pack inválido: falta json");
    };
    let json_str = match serde_json::to_string(json_value) {
        Ok(s) => s,
        Err(_) => return error(StatusCode::BAD_REQUEST, "json no serializable"),
    };
    if json_str.len() > MAX_BODY_BYTES {
        return error(StatusCode::PAYLOAD_TOO_LARGE, "pack demasiado grande");
    }
    let result = {
        let db = state.db();
        db.execute(
            "INSERT INTO packs (owner, name, desc, json) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![me.name, name, desc, json_str],
        )
        .map(|_| db.last_insert_rowid())
    };
    match result {
        Ok(id) => Json(json!({ "id": id })).into_response(),
        Err(e) => internal(e),
    }
}

async fn rating(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let Some(me) = current_user(&state, &headers) else {
        return error(StatusCode::UNAUTHORIZED, "haz login primero");
    };
    // truthiness de JS: packId presente y distinto de 0
    let pack_id = body.get("packId").and_then(Value::as_i64).unwrap_or(0);
    let stars = body.get("stars").and_then(Value::as_f64);
    let stars_ok = stars.is_some_and(|s| (1.0..=5.0).contains(&s));
    if pack_id == 0 || !stars_ok {
        return error(StatusCode::BAD_REQUEST, "packId y stars 1-5 requeridos");
    }
    let stars = stars.unwrap_or_default().floor() as i64;
    let result = {
        let db = state.db();
        db.execute(
            "INSERT INTO ratings (pack_id, user_id, stars) VALUES (?1, ?2, ?3)
             ON CONFLICT (pack_id, user_id) DO UPDATE SET stars = excluded.stars",
            rusqlite::params![pack_id, me.id, stars],
        )
    };
    match result {
        Ok(_) => Json(json!({ "ok": true })).into_response(),
        Err(e) if is_unique_violation(&e) => error(StatusCode::BAD_REQUEST, "pack inexistente"),
        Err(e) => internal(e),
    }
}

// ---------- test drive (VNC interactivo) ----------

/// Rutas aceptables "en forma" antes de tocar el disco: absolutas, con
/// extensión de imagen y fuera de /proc, /sys y /dev.
fn image_path_form_ok(raw: &str) -> bool {
    if raw.contains('\0') {
        return false;
    }
    let p = FsPath::new(raw);
    if !(p.is_absolute() || p.has_root()) {
        return false;
    }
    let name = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let ext_ok = [".iso", ".img", ".qcow2", ".raw"]
        .iter()
        .any(|e| name.ends_with(e));
    if !ext_ok {
        return false;
    }
    !(raw.starts_with("/proc") || raw.starts_with("/sys") || raw.starts_with("/dev"))
}

/// Canonicaliza (resuelve symlinks) y confina la imagen al directorio
/// permitido, si `DF_ISO_DIR` está definido.
fn safe_image_path(state: &AppState, raw: &str) -> Result<PathBuf, &'static str> {
    let p = std::fs::canonicalize(raw).map_err(|_| "la imagen no existe o no es accesible")?;
    let meta = std::fs::metadata(&p).map_err(|_| "no se pudo leer la imagen")?;
    if !meta.is_file() {
        return Err("imagePath debe ser un fichero regular");
    }
    if let Some(base) = state.iso_dir.as_deref() {
        if !p.starts_with(base.as_path()) {
            return Err("imagePath fuera del directorio de imágenes permitido");
        }
    }
    Ok(p)
}

async fn available(State(state): State<AppState>) -> Response {
    let cached = *state
        .available_cache
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let available = match cached {
        Some((v, at)) if at.elapsed() < AVAILABLE_TTL => v,
        _ => {
            let v = state.drives.available().await;
            *state
                .available_cache
                .lock()
                .unwrap_or_else(|e| e.into_inner()) = Some((v, Instant::now()));
            v
        }
    };
    Json(json!({ "available": available })).into_response()
}

async fn start_session(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(body): Json<Value>,
) -> Response {
    let Some(me) = current_user(&state, &headers) else {
        return error(StatusCode::UNAUTHORIZED, "haz login primero");
    };
    let image_path = body.get("imagePath").and_then(Value::as_str).unwrap_or("");
    if image_path.is_empty() || !image_path_form_ok(image_path) {
        return error(
            StatusCode::BAD_REQUEST,
            "imagePath debe ser una ruta absoluta .iso/.img/.qcow2/.raw del servidor",
        );
    }
    if !state.drives.available().await {
        return error(
            StatusCode::SERVICE_UNAVAILABLE,
            "QEMU no está instalado en el servidor",
        );
    }
    if state.drives.active_count() >= MAX_SESSIONS_TOTAL
        || state.drives.list_for(&me.name).len() >= MAX_SESSIONS_PER_USER
    {
        return error(
            StatusCode::TOO_MANY_REQUESTS,
            "demasiadas sesiones de test drive activas, espera a que caduquen",
        );
    }
    let image = match safe_image_path(&state, image_path) {
        Ok(p) => p,
        Err(msg) => return error(StatusCode::BAD_REQUEST, msg),
    };
    match state
        .drives
        .start(&me.name, &image.to_string_lossy(), DEFAULT_MEM_MB)
        .await
    {
        Ok(s) => Json(json!({
            "id": s.id,
            "vncUrl": format!("/vnc.html?session={}", s.id),
            "expires": s.expires,
        }))
        .into_response(),
        Err(e) => {
            eprintln!("[distroforge] test drive: {e}");
            error(
                StatusCode::INTERNAL_SERVER_ERROR,
                "no se pudo arrancar la VM",
            )
        }
    }
}

async fn list_sessions(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let owner = current_user(&state, &headers)
        .map(|u| u.name)
        .unwrap_or_default();
    let sessions: Vec<Value> = state
        .drives
        .list_for(&owner)
        .into_iter()
        .map(|(id, expires)| json!({ "id": id, "expires": expires }))
        .collect();
    Json(json!({ "sessions": sessions })).into_response()
}

async fn stop_session(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
) -> Response {
    let Some(me) = current_user(&state, &headers) else {
        return error(StatusCode::UNAUTHORIZED, "haz login primero");
    };
    // Solo el dueño puede parar su VM: un id ajeno responde 404 (ni siquiera
    // confirma que existe).
    match state.drives.get(&id) {
        Some(s) if s.owner == me.name => {
            Json(json!({ "ok": state.drives.stop(&id) })).into_response()
        }
        _ => error(StatusCode::NOT_FOUND, "sesión no encontrada"),
    }
}

async fn ws_bridge(
    State(state): State<AppState>,
    Path(id): Path<String>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Response {
    let Some(me) = current_user(&state, &headers) else {
        return error(StatusCode::UNAUTHORIZED, "haz login primero");
    };
    let valid = !id.is_empty() && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    let session = valid.then(|| state.drives.get(&id)).flatten();
    match session {
        Some(s) if s.owner == me.name => {
            ws.on_upgrade(move |socket| vncbridge::bridge(socket, s.vnc_port))
        }
        _ => StatusCode::NOT_FOUND.into_response(),
    }
}

// ---------- estáticos: cliente noVNC ----------

async fn vnc_page() -> Response {
    (
        [(header::CONTENT_TYPE, "text/html; charset=utf-8")],
        VNC_PAGE_HTML,
    )
        .into_response()
}

/// Página mínima del cliente VNC. DIFERENCIA con el servidor TS: aquí no hay
/// node_modules; noVNC se carga desde CDN (import ESM) en vez de /novnc/*.
const VNC_PAGE_HTML: &str = r#"<!doctype html>
<html lang="es"><head><meta charset="utf-8">
<title>DistroForge Test Drive</title>
<style>
  body { margin: 0; background: #0a0e1a; color: #e8ebf2; font: 14px system-ui, sans-serif; }
  #bar { padding: 8px 12px; background: #171c2c; border-bottom: 1px solid #262b36; display: flex; gap: 12px; align-items: center; }
  #bar b { color: #8b7bff; }
  #screen { width: 100vw; height: calc(100vh - 40px); }
</style></head>
<body>
<div id="bar"><b>DistroForge Test Drive</b><span id="status">conectando…</span></div>
<div id="screen"></div>
<script type="module">
  import RFB from 'https://cdn.jsdelivr.net/npm/@novnc/novnc@1.5.0/lib/rfb.js';
  const sid = new URLSearchParams(location.search).get('session');
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  const rfb = new RFB(document.getElementById('screen'), proto + '://' + location.host + '/ws/' + sid);
  rfb.addEventListener('connect', () => document.getElementById('status').textContent = 'conectado');
  rfb.addEventListener('disconnect', (e) => {
    document.getElementById('status').textContent = e.detail.clean ? 'sesión cerrada' : 'desconectado';
  });
</script>
</body></html>"#;
