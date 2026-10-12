//! Cliente del backend DistroForge (rust/server-rs) desde la UI Rust.
//!
//! Mantiene la URL del servidor y el token Bearer en localStorage. Con
//! SameSite=Lax las cookies no viajan cross-origin en desarrollo: por eso
//! guardamos el token que devuelve login/register y lo enviamos en la
//! cabecera Authorization (el backend acepta cookie o Bearer).

use crate::util::{ls_get, ls_remove, ls_set};
use serde::{Deserialize, Serialize};

const URL_KEY: &str = "distroforge-server-url";
const TOKEN_KEY: &str = "distroforge-server-token";

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct ServerPack {
    pub id: i64,
    pub owner: String,
    pub name: String,
    #[serde(default)]
    pub desc: String,
    pub json: serde_json::Value,
    #[serde(default)]
    pub rating: Rating,
}

#[derive(Clone, Debug, Deserialize, Serialize, Default)]
pub struct Rating {
    #[serde(default)]
    pub avg: f64,
    #[serde(default)]
    pub n: i64,
}

pub fn server_url() -> String {
    ls_get(URL_KEY).unwrap_or_default()
}

pub fn set_server_url(url: &str) {
    ls_set(URL_KEY, url.trim_end_matches('/'));
}

fn token() -> Option<String> {
    ls_get(TOKEN_KEY)
}

async fn call(method: &str, path: &str, body: Option<serde_json::Value>) -> Result<serde_json::Value, String> {
    let base = server_url();
    if base.is_empty() {
        return Err("sin servidor configurado".into());
    }
    let url = format!("{base}{path}");
    let mut req = if method == "GET" {
        gloo_net::http::Request::get(&url)
    } else {
        gloo_net::http::Request::post(&url)
    };
    if let Some(t) = token() {
        req = req.header("authorization", &format!("Bearer {t}"));
    }
    if let Some(b) = body {
        req = req.header("content-type", "application/json");
        let payload = serde_json::to_string(&b).expect("json serializable");
        let res = req
            .body(payload)
            .map_err(|e| e.to_string())?
            .send()
            .await
            .map_err(|e| e.to_string())?;
        return finish(res).await;
    }
    let res = req.send().await.map_err(|e| e.to_string())?;
    finish(res).await
}

/// Consume la respuesta: error con el mensaje del backend o el JSON parseado.
async fn finish(res: gloo_net::http::Response) -> Result<serde_json::Value, String> {
    let status = res.status();
    let text = res.text().await.map_err(|e| e.to_string())?;
    let json: serde_json::Value = serde_json::from_str(&text).unwrap_or(serde_json::Value::Null);
    if !(200..300).contains(&status) {
        let err = json
            .get("error")
            .and_then(|e| e.as_str())
            .map(str::to_string)
            .unwrap_or_else(|| format!("HTTP {status}"));
        return Err(err);
    }
    Ok(json)
}

fn store_token(v: &serde_json::Value) {
    if let Some(t) = v.get("token").and_then(|t| t.as_str()) {
        ls_set(TOKEN_KEY, t);
    }
}

pub async fn me() -> Result<Option<String>, String> {
    let v = call("GET", "/api/me", None).await?;
    Ok(v.get("user")
        .and_then(|u| u.get("name"))
        .and_then(|n| n.as_str())
        .map(str::to_string))
}

pub async fn register(name: &str, password: &str) -> Result<String, String> {
    let v = call(
        "POST",
        "/api/register",
        Some(serde_json::json!({ "name": name, "password": password })),
    )
    .await?;
    store_token(&v);
    Ok(v.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string())
}

pub async fn login(name: &str, password: &str) -> Result<String, String> {
    let v = call(
        "POST",
        "/api/login",
        Some(serde_json::json!({ "name": name, "password": password })),
    )
    .await?;
    store_token(&v);
    Ok(v.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string())
}

pub async fn logout() -> Result<(), String> {
    call("POST", "/api/logout", None).await.map(|_| ())
}

pub async fn search(query: &str) -> Result<Vec<ServerPack>, String> {
    let v = call(
        "GET",
        &format!("/api/packs?query={}", urlencode(query)),
        None,
    )
    .await?;
    serde_json::from_value(v.get("packs").cloned().unwrap_or_default())
        .map_err(|e| e.to_string())
}

pub async fn publish(name: &str, desc: &str, json: serde_json::Value) -> Result<(), String> {
    call(
        "POST",
        "/api/packs",
        Some(serde_json::json!({ "name": name, "desc": desc, "json": json })),
    )
    .await
    .map(|_| ())
}

pub async fn drive_available() -> Result<bool, String> {
    let v = call("GET", "/api/testdrive/available", None).await?;
    Ok(v.get("available").and_then(|a| a.as_bool()).unwrap_or(false))
}

/// Arranca una VM en el servidor y abre noVNC en una pestaña nueva.
pub async fn drive_start(image_path: &str) -> Result<String, String> {
    let v = call(
        "POST",
        "/api/testdrive",
        Some(serde_json::json!({ "imagePath": image_path })),
    )
    .await?;
    v.get("vncUrl")
        .and_then(|u| u.as_str())
        .map(|u| format!("{}{}", server_url(), u))
        .ok_or_else(|| "respuesta sin vncUrl".into())
}

/// urlencode mínimo (query de búsqueda: espacio y reservados habituales).
fn urlencode(s: &str) -> String {
    let mut out = String::new();
    for b in s.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(b as char)
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

/// Convierte el json de un pack del servidor al Pack de la UI (aplicable).
pub fn json_to_pack(v: &serde_json::Value) -> Option<crate::packs::Pack> {
    let name = v.get("name")?.as_str()?.to_string();
    let packages: Vec<String> = v
        .get("packages")?
        .as_array()?
        .iter()
        .filter_map(|p| p.as_str().map(str::to_string))
        .collect();
    let flatpak_apps: Option<Vec<String>> = v.get("flatpakApps").and_then(|f| f.as_array()).map(|a| {
        a.iter().filter_map(|p| p.as_str().map(str::to_string)).collect()
    });
    let id = v
        .get("id")
        .and_then(|i| i.as_i64())
        .map(|i| i.to_string())
        .unwrap_or_else(|| name.to_lowercase().replace([' ', '.'], "-"));
    let desc = v.get("desc").and_then(|d| d.as_str()).unwrap_or("").to_string();
    let desktop = v
        .get("desktop")
        .and_then(|d| d.as_str())
        .map(str::to_string);
    Some(crate::packs::Pack {
        id,
        name,
        desc,
        packages,
        flatpak_apps,
        desktop,
    })
}

/// Limpia el token local (tras logout).
pub fn clear_token() {
    ls_remove(TOKEN_KEY);
}
