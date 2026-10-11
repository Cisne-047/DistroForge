//! Binding WASM del núcleo de DistroForge (sustituye al bundle TS del
//! frontend para generar/validar proyectos desde el navegador).
//!
//! No hace falta compilar a `wasm32` para verificarlo: `cargo check -p
//! forge-wasm` en host es suficiente (wasm-bindgen compila en host con stubs).

use wasm_bindgen::prelude::*;

use forge_core::generators::{generate_project as core_generate, validate as core_validate};
use forge_core::types::DistroConfig;

/// Genera el proyecto completo a partir de un JSON de `DistroConfig`.
/// Devuelve `{"ruta": "contenido", ...}` o `Err` si el JSON no parsea.
#[wasm_bindgen]
pub fn generate_project(cfg_json: String) -> Result<String, JsValue> {
    let cfg: DistroConfig = serde_json::from_str(&cfg_json)
        .map_err(|e| JsValue::from_str(&format!("config inválida: {e}")))?;
    let files = core_generate(&cfg);
    serde_json::to_string(&files).map_err(|e| JsValue::from_str(&e.to_string()))
}

/// Valida una config en JSON y devuelve la lista de mensajes (vacía = ok).
#[wasm_bindgen]
pub fn validate(cfg_json: String) -> Result<Vec<String>, JsValue> {
    let cfg: DistroConfig = serde_json::from_str(&cfg_json)
        .map_err(|e| JsValue::from_str(&format!("config inválida: {e}")))?;
    Ok(core_validate(&cfg))
}

/// Codifica una config en JSON a base64url para compartir por URL (port de
/// `share.encodeConfig`).
#[wasm_bindgen]
pub fn encode_config(cfg_json: String) -> Result<String, JsValue> {
    let cfg: DistroConfig = serde_json::from_str(&cfg_json)
        .map_err(|e| JsValue::from_str(&format!("config inválida: {e}")))?;
    Ok(forge_core::share::encode_config(&cfg))
}

/// Decodifica un fragmento base64url a JSON de config; `null` JS si es basura
/// (port de `share.decodeConfig`).
#[wasm_bindgen]
pub fn decode_config(hash: String) -> Result<Option<String>, JsValue> {
    Ok(forge_core::share::decode_config(&hash)
        .and_then(|cfg| serde_json::to_string(&cfg).ok()))
}
