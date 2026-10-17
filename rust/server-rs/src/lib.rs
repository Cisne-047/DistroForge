//! Servidor DistroForge en Rust: réplica del backend Node/Fastify de `server/`.

pub mod auth;
pub mod db;
pub mod routes;
pub mod testdrive;
pub mod vncbridge;

pub use routes::{build_router, build_router_with_drives, build_router_with_ui, TTL};
pub use testdrive::DriveManager;
