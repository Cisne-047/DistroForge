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

use std::sync::{Arc, Mutex};
use std::time::Duration;

use axum::extract::ws::WebSocketUpgrade;
use axum::extract::{Path, Query, State};
use axum::http::{HeaderMap, Method, StatusCode, header};
use axum::response::{IntoResponse, Response};
use axum::routing::{any, delete, get, post};
use axum::{Json, Router};
use rusqlite::Connection;
use serde_json::{Value, json};
use tower_http::cors::{AllowHeaders, AllowOrigin, CorsLayer};
use tower_http::services::{ServeDir, ServeFile};

use crate::auth::{self, CurrentUser};
use crate::db::{self, PackRow};
use crate::testdrive::{DriveManager, is_absolute_path};
use crate::vncbridge;

const COOKIE: &str = "df_session";
const SESSION_MAX_AGE: u32 = 30 * 24 * 60 * 60; // 30 días, igual que la sesión en BD
const DEFAULT_MEM_MB: u32 = 2048;
/// TTL máximo de una sesión de test drive.
pub const TTL: Duration = Duration::from_secs(15 * 60);

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
}

/// Construye el router completo (equivalente a `buildServer` en TS).
pub fn build_router(db: Connection, qemu_bin: &str) -> Router {
    build_router_with_drives(db, DriveManager::new(qemu_bin, TTL))
}

/// Igual que `build_router` pero reutiliza un `DriveManager` existente
/// (permite al `main` barrear sesiones caducadas con el mismo gestor).
pub fn build_router_with_drives(db: Connection, drives: DriveManager) -> Router {
    let state = AppState {
        db: Arc::new(Mutex::new(db)),
        drives,
    };
    // la app (Pages/localhost) llama cross-origin con cookie de sesión
    // (equivale a fastifyCors { origin: true, credentials: true }; los headers
    // se espejan porque la spec CORS prohibe "*" junto a credentials)
    let cors = CorsLayer::new()
        .allow_origin(AllowOrigin::mirror_request())
        .allow_headers(AllowHeaders::mirror_request())
        .allow_credentials(true)
        .allow_methods([Method::GET, Method::HEAD, Method::PUT, Method::PATCH, Method::POST, Method::DELETE]);
    Router::new()
        .route("/api/register", post(register))
        .route("/api/login", post(login))
        .route("/api/logout", post(logout))
        .route("/api/me", get(me))
        .route("/api/packs", get(list_packs).post(publish_pack))
        .route("/api/rating", post(rating))
        .route(
            "/api/testdrive",
            get(list_sessions).post(start_session),
        )
        .route("/api/testdrive/available", get(available))
        .route("/api/testdrive/{id}", delete(stop_session))
        .route("/api/health", get(health))
        .route("/vnc.html", get(vnc_page))
        .route("/ws/{id}", get(ws_bridge))
        .layer(cors)
        .with_state(state)
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

fn set_cookie(token: &str) -> String {
    format!("{COOKIE}={token}; Path=/; HttpOnly; SameSite=Lax; Max-Age={SESSION_MAX_AGE}")
}

fn clear_cookie() -> String {
    format!(
        "{COOKIE}=; Path=/; HttpOnly; SameSite=Lax; Max-Age=0; Expires=Thu, 01 Jan 1970 00:00:00 GMT"
    )
}

/// Usuario actual: cookie df_session O header Authorization: Bearer <token>.
fn current_user(state: &AppState, headers: &HeaderMap) -> Option<CurrentUser> {
    let bearer = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    let bearer = bearer.strip_prefix("Bearer ").unwrap_or(bearer);
    let token = if !bearer.is_empty() {
        bearer.to_owned()
    } else {
        cookie_value(headers, COOKIE)?
    };
    let db = state.db.lock().expect("lock db");
    auth::user_for_token(&db, &token)
}

/// body.name como string no vacío (truthiness de JS aproximada).
fn name_field(body: &Value) -> Option<&str> {
    body.get("name").and_then(Value::as_str).filter(|s| !s.is_empty())
}

// ---------- auth ----------

async fn register(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    let name = body.get("name").and_then(Value::as_str).unwrap_or("");
    let password = body.get("password").and_then(Value::as_str).unwrap_or("");
    if name.is_empty() || !valid_name(name) {
        return error(StatusCode::BAD_REQUEST, "nombre inválido (3-32, letras/números/_.-)");
    }
    if password.chars().count() < 8 {
        return error(StatusCode::BAD_REQUEST, "contraseña de al menos 8 caracteres");
    }
    {
        let db = state.db.lock().expect("lock db");
        if auth::find_user(&db, name).is_some() {
            return error(StatusCode::CONFLICT, "ese nombre ya existe");
        }
    }
    let token = {
        let db = state.db.lock().expect("lock db");
        let user_id = auth::create_user(&db, name, password).expect("crear usuario");
        auth::create_session(&db, user_id)
    };
    let mut resp = Json(json!({ "name": name, "token": token })).into_response();
    resp.headers_mut()
        .append(header::SET_COOKIE, set_cookie(&token).parse().unwrap());
    resp
}

async fn login(State(state): State<AppState>, Json(body): Json<Value>) -> Response {
    let name = body.get("name").and_then(Value::as_str).unwrap_or("");
    let password = body.get("password").and_then(Value::as_str).unwrap_or("");
    let (user_name, token) = {
        let db = state.db.lock().expect("lock db");
        let row = if name.is_empty() { None } else { auth::find_user(&db, name) };
        let Some(row) = row else {
            return error(StatusCode::UNAUTHORIZED, "usuario o contraseña incorrectos");
        };
        if password.is_empty() || !auth::verify_password(password, &row.pass_hash) {
            return error(StatusCode::UNAUTHORIZED, "usuario o contraseña incorrectos");
        }
        let token = auth::create_session(&db, row.id);
        (row.name, token)
    };
    let mut resp = Json(json!({ "name": user_name, "token": token })).into_response();
    resp.headers_mut()
        .append(header::SET_COOKIE, set_cookie(&token).parse().unwrap());
    resp
}

async fn logout(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let token = cookie_value(&headers, COOKIE);
    let db = state.db.lock().expect("lock db");
    auth::destroy_session(&db, token.as_deref());
    drop(db);
    let mut resp = Json(json!({ "ok": true })).into_response();
    resp.headers_mut()
        .append(header::SET_COOKIE, clear_cookie().parse().unwrap());
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

async fn list_packs(
    State(state): State<AppState>,
    Query(q): Query<PacksQuery>,
) -> Response {
    let rows = {
        let db = state.db.lock().expect("lock db");
        let rows = db::search_packs(&db, q.query.as_deref().unwrap_or(""), 50);
        let mut ratings: Vec<(i64, f64, i64)> = Vec::new();
        if let Ok(mut stmt) = db.prepare(
            "SELECT pack_id, ROUND(AVG(stars), 1) AS avg, COUNT(*) AS n
             FROM ratings GROUP BY pack_id",
        ) {
            let mapped = stmt
                .query_map([], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)))
                .expect("query ratings");
            for r in mapped.flatten() {
                ratings.push(r);
            }
        }
        (rows, ratings)
    };
    let (rows, ratings) = rows;
    let by_id: std::collections::HashMap<i64, (f64, i64)> =
        ratings.into_iter().map(|(id, avg, n)| (id, (avg, n))).collect();
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
        return error(StatusCode::BAD_REQUEST, "pack inválido: falta name o packages[]");
    };
    let packages_ok = body
        .get("json")
        .and_then(|j| j.get("packages"))
        .map(Value::is_array)
        .unwrap_or(false);
    if !packages_ok {
        return error(StatusCode::BAD_REQUEST, "pack inválido: falta name o packages[]");
    }
    let desc = body.get("desc").and_then(Value::as_str).unwrap_or("");
    let json_str = serde_json::to_string(body.get("json").unwrap_or(&Value::Null))
        .expect("serializar json");
    let result = {
        let db = state.db.lock().expect("lock db");
        db.execute(
            "INSERT INTO packs (owner, name, desc, json) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![me.name, name, desc, json_str],
        )
        .map(|_| db.last_insert_rowid())
    };
    match result {
        Ok(id) => Json(json!({ "id": id })).into_response(),
        Err(e) => error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
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
        let db = state.db.lock().expect("lock db");
        db.execute(
            "INSERT INTO ratings (pack_id, user_id, stars) VALUES (?1, ?2, ?3)
             ON CONFLICT (pack_id, user_id) DO UPDATE SET stars = excluded.stars",
            rusqlite::params![pack_id, me.id, stars],
        )
    };
    match result {
        Ok(_) => Json(json!({ "ok": true })).into_response(),
        Err(e) => error(StatusCode::INTERNAL_SERVER_ERROR, &e.to_string()),
    }
}

// ---------- test drive (VNC interactivo) ----------

async fn available(State(state): State<AppState>) -> Response {
    let available = state.drives.available().await;
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
    if image_path.is_empty() || !is_absolute_path(image_path) {
        return error(
            StatusCode::BAD_REQUEST,
            "imagePath debe ser una ruta absoluta del servidor",
        );
    }
    if !state.drives.available().await {
        return error(StatusCode::SERVICE_UNAVAILABLE, "QEMU no está instalado en el servidor");
    }
    match state.drives.start(&me.name, image_path, DEFAULT_MEM_MB).await {
        Ok(s) => Json(json!({
            "id": s.id,
            "vncUrl": format!("/vnc.html?session={}", s.id),
            "expires": s.expires,
        }))
        .into_response(),
        Err(e) => error(StatusCode::INTERNAL_SERVER_ERROR, &e),
    }
}

async fn list_sessions(State(state): State<AppState>, headers: HeaderMap) -> Response {
    let owner = current_user(&state, &headers).map(|u| u.name).unwrap_or_default();
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
) -> Response {
    let ok = state.drives.stop(&id);
    Json(json!({ "ok": ok })).into_response()
}

async fn ws_bridge(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ws: WebSocketUpgrade,
) -> Response {
    let valid = !id.is_empty() && id.chars().all(|c| c.is_ascii_hexdigit() || c == '-');
    match valid.then(|| state.drives.get(&id)).flatten() {
        Some(session) => ws.on_upgrade(move |socket| vncbridge::bridge(socket, session.vnc_port)),
        None => StatusCode::NOT_FOUND.into_response(),
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
