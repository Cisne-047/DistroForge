# DistroForge Desktop (scaffold Tauri 2)

Scaffold de versión de escritorio que reutiliza el núcleo Rust
(`crates/forge-core`) con el frontend actual de la app (`frontendDist`
apunta a `../../dist`, el build de Vite del repo).

## Contenido

- `src-tauri/Cargo.toml` — crate con `tauri 2`, `tauri-build 2` y
  `forge-core` como dependencia de path (`../../crates/forge-core`).
- `src-tauri/src/main.rs` — comandos `generate_project(cfg_json) -> String`
  (JSON `{"ruta": "contenido"}`) y `validate(cfg_json) -> Vec<String>`.
- `src-tauri/build.rs` — `tauri_build::build()`.
- `src-tauri/tauri.conf.json` — ventana "DistroForge", `frontendDist: "../../dist"`.

## Cómo compilar (NO se hace en CI normal)

El crate está excluido del workspace de `rust/` a propósito (las deps de
Tauri tardan mucho y `cargo test` del core no debe tocarlo):

```bash
# 1. Build del frontend de la app actual
npm install && npm run build        # en la raíz del repo -> dist/

# 2. Build de la app de escritorio (prerequisitos de Tauri 2 por plataforma)
cd rust/tauri-app/src-tauri
cargo build
```

`tauri::generate_context!` valida `tauri.conf.json` en tiempo de
compilación; la sintaxis de este scaffold es la del template oficial de
Tauri 2 (builder + invoke_handler + command macros).
