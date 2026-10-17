//! Apps Flatpak / Flathub. Port de `src/generators/flatpak.ts`.

use crate::generators::shell;
use crate::types::DistroConfig;

/// Paquete que habilita el runtime de Flatpak.
pub const FLATPAK_PACKAGE: &str = "flatpak";

/// Remote de Flathub.
pub const FLATHUB: &str = "https://dl.flathub.org/repo/flathub.flatpakrepo";

/// Apps Flatpak de la config (en el TS tolera `flatpakApps` no-array; aquí el
/// tipo ya lo garantiza).
pub fn flatpak_apps(cfg: &DistroConfig) -> &[String] {
    &cfg.flatpak_apps
}

/// ¿Hay apps Flatpak que instalar?
pub fn has_flatpak_apps(cfg: &DistroConfig) -> bool {
    !cfg.flatpak_apps.is_empty()
}

/// El paquete "flatpak" solo entra en la lista cuando hay apps que instalar.
pub fn flatpak_packages(cfg: &DistroConfig) -> Vec<String> {
    if has_flatpak_apps(cfg) {
        vec![FLATPAK_PACKAGE.to_string()]
    } else {
        vec![]
    }
}

/// Líneas shell: alta de Flathub + instalación de las apps. Para chroot/root
/// directo; `prefix` se antepone a cada línea (p. ej. "chroot /rootfs ").
pub fn flatpak_shell_lines(cfg: &DistroConfig, prefix: &str) -> Vec<String> {
    let apps = flatpak_apps(cfg);
    if apps.is_empty() {
        return vec![];
    }
    let ids = apps
        .iter()
        .filter(|a| crate::security::valid_flatpak_id(a))
        .map(|a| shell::sq(a))
        .collect::<Vec<_>>()
        .join(" ");
    if ids.is_empty() {
        return vec![];
    }
    vec![
        format!("{prefix}flatpak remote-add --if-not-exists flathub {FLATHUB}"),
        format!("{prefix}flatpak install -y --noninteractive flathub {ids}"),
    ]
}
