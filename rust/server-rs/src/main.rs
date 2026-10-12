//! Entrada del servidor: escucha en PORT (8080 por defecto).

use std::time::Duration;

use distroforge_server::testdrive::DriveManager;
use distroforge_server::{build_router_with_drives, db::open_db, routes};

#[tokio::main]
async fn main() {
    let port: u16 = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(8080);
    let qemu_bin = std::env::var("QEMU_BIN").unwrap_or_else(|_| "qemu-system-x86_64".into());
    // BD en memoria por defecto; DF_DB_PATH permite persistir en disco.
    let db_path = std::env::var("DF_DB_PATH").unwrap_or_else(|_| ":memory:".into());

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

    let app = build_router_with_drives(db, drives);
    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port))
        .await
        .unwrap_or_else(|e| panic!("no se pudo escuchar en 0.0.0.0:{port}: {e}"));
    println!("DistroForge server en http://0.0.0.0:{port}");
    axum::serve(listener, app).await.expect("servir axum");
}
