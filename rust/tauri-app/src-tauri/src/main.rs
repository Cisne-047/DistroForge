//! Scaffold de DistroForge Desktop (Tauri 2).
//!
//! Expone el núcleo (`forge-core`) a un frontend web mediante dos comandos:
//! `generate_project(cfg_json) -> {"ruta": "contenido"}` y
//! `validate(cfg_json) -> [mensajes]`. El frontend es la UI Leptos/WASM, que se
//! sirve desde `frontendDist` (`../../web-ui/dist`): compílala antes con
//! `cd rust/web-ui && trunk build --release`. Los hooks de Tauri corren desde
//! `rust/tauri-app` (no desde `src-tauri`), así que una ruta relativa ahí no
//! resolvería; por eso el workflow de release lo hace en su propio paso.
//! Los comandos quedan como punto de extensión nativo: la UI actual genera el
//! proyecto en WASM y no los invoca.
//!
//! NO se compila en el flujo normal del workspace (ver `rust/Cargo.toml`,
//! `exclude`): las dependencias de Tauri tardan mucho. Para probarlo:
//! `cd rust/tauri-app/src-tauri && cargo tauri build` (requiere los
//! prerequisitos habituales de Tauri 2 por plataforma).
//!
//! Nota: `expect` en `main` es código de aplicación (arranque), no ruta de
//! librería; la regla "sin unwrap" aplica al core.

#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use forge_core::types::DistroConfig;
use tauri_plugin_updater::UpdaterExt;

fn parse_config(cfg_json: &str) -> Result<DistroConfig, String> {
    serde_json::from_str(cfg_json).map_err(|e| format!("config inválida: {e}"))
}

/// Genera el proyecto completo; devuelve el mapa `{"ruta": "contenido"}` como
/// JSON de objeto.
#[tauri::command]
fn generate_project(cfg_json: String) -> Result<String, String> {
    let cfg = parse_config(&cfg_json)?;
    let files = forge_core::generators::generate_project(&cfg);
    serde_json::to_string(&files).map_err(|e| e.to_string())
}

/// Valida la config; devuelve la lista de mensajes (vacía = ok).
#[tauri::command]
fn validate(cfg_json: String) -> Result<Vec<String>, String> {
    let cfg = parse_config(&cfg_json)?;
    Ok(forge_core::generators::validate(&cfg))
}

fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_updater::Builder::new().build())
        .invoke_handler(tauri::generate_handler![generate_project, validate])
        .setup(|app| {
            // Auto-actualización: al arrancar se pregunta si hay versión nueva y,
            // si la hay, se instala y la app se reinicia sola. Nunca debe impedir
            // abrir: si no hay red, el endpoint no responde o la firma no cuadra,
            // se avisa por consola y se sigue.
            let handle = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let updater = match handle.updater() {
                    Ok(u) => u,
                    Err(e) => {
                        eprintln!("updater no disponible: {e}");
                        return;
                    }
                };
                match updater.check().await {
                    Ok(Some(update)) => {
                        let version = update.version.clone();
                        match update.download_and_install(|_, _| {}, || {}).await {
                            Ok(()) => {
                                println!("actualizado a {version}; reiniciando");
                                handle.restart();
                            }
                            Err(e) => eprintln!("no se pudo instalar {version}: {e}"),
                        }
                    }
                    Ok(None) => println!("sin actualizaciones"),
                    Err(e) => eprintln!("no se pudo comprobar actualizaciones: {e}"),
                }
            });
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
