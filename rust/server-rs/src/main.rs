//! Entrada del servidor: escucha en PORT (8080 por defecto).
//!
//! Variables de entorno:
//!   PORT         puerto (8080)
//!   DF_DB_PATH   SQLite (por defecto :memory:)
//!   QEMU_BIN     binario de QEMU para el Test Drive
//!   DF_WEB_DIR   directorio con la interfaz web ya compilada. Si existe, el
//!                servidor sirve la UI y la API en el mismo origen (la cookie de
//!                sesión viaja y no hace falta configurar la URL en la app); si
//!                no existe, es una API pura.

#![forbid(unsafe_code)]

use std::time::Duration;

use distroforge_server::testdrive::DriveManager;
use distroforge_server::{build_router_with_ui, db::open_db, routes};

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let qemu_bin = std::env::var("QEMU_BIN").unwrap_or_else(|_| "qemu-system-x86_64".into());
    // BD en memoria por defecto; DF_DB_PATH permite persistir en disco.
    let db_path = std::env::var("DF_DB_PATH").unwrap_or_else(|_| ":memory:".into());
    // UI opcional, en el mismo origen que la API.
    let web_dir: Option<std::path::PathBuf> = std::env::var("DF_WEB_DIR")
        .ok()
        .filter(|d| !d.trim().is_empty())
        .map(std::path::PathBuf::from);

    let db = open_db(&db_path);
    let drives = DriveManager::new(&qemu_bin, routes::TTL);
    // limpieza periódica de sesiones de test drive caducadas (TTL 15 min)
    let sweeper = drives.clone();
    tokio::spawn(async move {
        let mut tick = tokio::time::interval(Duration::from_secs(60));
        loop {
            tick.tick().await;
            sweeper.sweep();
        }
    });

    let ui = web_dir.filter(|d| d.is_dir());
    match &ui {
        Some(dir) => println!("sirviendo la interfaz desde {}", dir.display()),
        None => println!("sin interfaz web (DF_WEB_DIR no apunta a un directorio)"),
    }
    let app = build_router_with_ui(db, drives, ui);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .unwrap_or_else(|e| panic!("no se pudo escuchar en 0.0.0.0:{port}: {e}"));
    println!("DistroForge server en http://0.0.0.0:{port}");
    axum::serve(listener, app).await.expect("servir axum");
}
