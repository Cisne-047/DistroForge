//! Bases manuales (Gentoo, Solus): lista de paquetes y guía. Port de
//! `src/generators/manual.ts`.

use crate::catalog::{base_by_id, desktop_packages};
use crate::types::DistroConfig;

/// Ficheros del proyecto manual, en el orden de construcción del TS.
pub fn manual_files(cfg: &DistroConfig) -> Vec<(String, String)> {
    let base = base_by_id(cfg.base);
    let mut pkgs: Vec<String> = desktop_packages(cfg.base, &cfg.desktop)
        .iter()
        .map(|s| s.to_string())
        .collect();
    pkgs.extend(cfg.packages.iter().cloned());
    let pkgs = pkgs.join("\n");
    let is_gentoo = cfg.base == crate::types::BaseId::Gentoo;
    let howto = if is_gentoo {
        "1. Arranca un stage3 de Gentoo y configura el sistema base.\n2. Añade los paquetes de arriba a tu `world` con `emerge --ask`.\n3. Para una ISO live usa **catalyst** con un spec stage4 que incluya esos paquetes."
    } else {
        "1. Instala Solus y añade los paquetes con `eopkg install`.\n2. Para una ISO remasterizada, parte de la imagen oficial + chroot, o usa respin propio."
    };

    let extras = &cfg.extra_files;
    let extras_section = if !extras.is_empty() {
        let list = extras
            .iter()
            .map(|f| format!("- `{}`", f.path))
            .collect::<Vec<_>>()
            .join("\n");
        format!("\n## Ficheros extra\n\nCopia estos ficheros a mano (rutas absolutas dentro del sistema):\n\n{list}\n")
    } else {
        String::new()
    };
    let flatpak_section = if !cfg.flatpak_apps.is_empty() {
        format!(
            "\n## Flatpak\n\n    flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo\n    flatpak install -y flathub {}\n",
            cfg.flatpak_apps.join(" ")
        )
    } else {
        String::new()
    };

    let readme = format!(
        "# {} — base {} (motor manual)\n\n{} no tiene un builder público de imágenes que DistroForge pueda lanzar en contenedor todavía. Este proyecto genera la **lista de paquetes** y notas para montarla a mano.\n\n## Paquetes seleccionados\n\n```\n{pkgs}\n```\n\n## Cómo montarlo\n\n{howto}\n{extras}{flatpak}\n## Notas\n\n- Los nombres de paquete son orientativos: ajústalos al repositorio de la distro.\n- DistroForge añadirá builder propio para estas bases en el futuro.\n",
        cfg.name,
        base.label,
        base.label,
        extras = extras_section,
        flatpak = flatpak_section,
    );

    let build_sh = format!(
        "#!/usr/bin/env bash\necho \"Motor manual: {} no tiene builder automático en esta versión.\"\necho \"Revisa manual/README.md.\"\nexit 1\n",
        base.label
    );

    vec![
        ("manual/packages.txt".to_string(), format!("{pkgs}\n")),
        ("manual/README.md".to_string(), readme),
        ("build.sh".to_string(), build_sh),
    ]
}
