//! UI web de DistroForge en Leptos 0.7 (CSR puro, Trunk + wasm32).
//!
//! Replica la app React de `src/` usando el núcleo `forge-core` (hermano del
//! workspace) para catálogo, plantillas, generadores y share por URL.
//!
//! Estructura:
//! - `app`: shell (cabecera, navegación, resumen lateral)
//! - `state`: carga/persistencia de la config (hash, localStorage, proyectos)
//! - `steps/*`: los 6 pasos del wizard
//! - `preview`: mock del escritorio/GRUB
//! - `widgets`: componentes compartidos (picker de imágenes)
//! - `packs`: modpacks (port de `src/packs.ts`) + galería
//! - `util`: interop con el navegador (zip, ficheros, clipboard, fetch)

#![forbid(unsafe_code)]

mod app;
mod packs;
mod server;
mod preview;
mod state;
mod steps;
mod util;
mod widgets;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
