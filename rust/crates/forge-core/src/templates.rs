//! Plantillas de config, portado de `src/templates.ts`.
//!
//! Plantilla = parche de config "describe lo que quieres" aplicable a cualquier
//! base. Los ids de escritorio se aplican solo si la base los tiene (ver
//! [`apply_template`]).

use crate::catalog::base_by_id;
use crate::types::{
    ArchId, BaseId, BootTarget, CustomIsoConfig, CustomRepo, DistroConfig, ExtraFile, ExtraUser,
    GrubConfig, NetworkConfig, OutputFormat, PartialGrubConfig, PartialSystemConfig, ServiceToggle,
    SystemConfig,
};

/// Parche de plantilla: `Partial<Omit<DistroConfig, "system">> & { system? }`.
/// Cada campo `Some` sustituye al de la config; `grub` y `system` se fusionan
/// en profundidad (merge por claves, como el spread del TS).
#[derive(Debug, Clone, Default)]
pub struct TemplatePatch {
    /// Nombre.
    pub name: Option<String>,
    /// Base.
    pub base: Option<BaseId>,
    /// Escritorio (se aplica solo si la base lo tiene).
    pub desktop: Option<String>,
    /// Paquetes.
    pub packages: Option<Vec<String>>,
    /// Repos.
    pub repos: Option<Vec<CustomRepo>>,
    /// Salida.
    pub output: Option<OutputFormat>,
    /// Flatpaks.
    pub flatpak_apps: Option<Vec<String>>,
    /// Ficheros extra.
    pub extra_files: Option<Vec<ExtraFile>>,
    /// Usuarios extra.
    pub users: Option<Vec<ExtraUser>>,
    /// Servicios.
    pub services: Option<Vec<ServiceToggle>>,
    /// Red.
    pub network: Option<NetworkConfig>,
    /// GRUB (fusión por claves).
    pub grub: Option<PartialGrubConfig>,
    /// Firewall.
    pub firewall: Option<bool>,
    /// Arquitectura.
    pub arch: Option<ArchId>,
    /// Modo advanced.
    pub advanced: Option<bool>,
    /// ISO custom.
    pub custom_iso: Option<CustomIsoConfig>,
    /// Auto-login.
    pub autologin: Option<bool>,
    /// Autostart.
    pub autostart: Option<Vec<String>>,
    /// Puertos firewall.
    pub firewall_ports: Option<Vec<String>>,
    /// Target de arranque.
    pub boot_target: Option<BootTarget>,
    /// cloud-init.
    pub cloud_init: Option<bool>,
    /// MOTD.
    pub motd: Option<String>,
    /// Wallpaper.
    pub wallpaper: Option<String>,
    /// Sistema (fusión por claves).
    pub system: Option<PartialSystemConfig>,
}

/// Plantilla (`Template`).
#[derive(Debug, Clone)]
pub struct Template {
    /// Id corto ("kiosk", "server"...).
    pub id: &'static str,
    /// Etiqueta visible.
    pub label: &'static str,
    /// Descripción.
    pub desc: &'static str,
    /// Parche de config.
    pub patch: TemplatePatch,
}

/// Catálogo de plantillas (`TEMPLATES` del TS).
pub static TEMPLATES: std::sync::LazyLock<Vec<Template>> = std::sync::LazyLock::new(|| {
    vec![
        Template {
            id: "server",
            label: "Servidor mínimo",
            desc: "SSH, htop, tmux; sin escritorio; salida OCI para contenedor o VM.",
            patch: TemplatePatch {
                packages: Some(vec![
                    "openssh-server".into(),
                    "htop".into(),
                    "vim".into(),
                    "tmux".into(),
                    "git".into(),
                ]),
                flatpak_apps: Some(vec![]),
                output: Some(OutputFormat::Oci),
                cloud_init: Some(true),
                firewall: Some(true),
                autologin: Some(false),
                ..Default::default()
            },
        },
        Template {
            id: "kiosk",
            label: "Kiosko",
            desc: "Firefox a pantalla completa con auto-login; GNOME si la base lo tiene.",
            patch: TemplatePatch {
                packages: Some(vec!["firefox".into(), "xdotool".into()]),
                desktop: Some("gnome".into()),
                autologin: Some(true),
                grub: Some(PartialGrubConfig {
                    timeout: Some(1.0),
                    extra_params: Some("quiet splash".into()),
                }),
                ..Default::default()
            },
        },
        Template {
            id: "office",
            label: "Oficina",
            desc: "LibreOffice, Thunderbird y Firefox con auto-login.",
            patch: TemplatePatch {
                packages: Some(vec![
                    "libreoffice".into(),
                    "thunderbird".into(),
                    "firefox".into(),
                ]),
                desktop: Some("gnome".into()),
                autologin: Some(true),
                ..Default::default()
            },
        },
        Template {
            id: "wsldev",
            label: "Entorno WSL dev",
            desc: "Sin escritorio, herramientas de consola; salida WSL para wsl --import.",
            patch: TemplatePatch {
                packages: Some(vec![
                    "git".into(),
                    "vim".into(),
                    "curl".into(),
                    "htop".into(),
                    "build-essential".into(),
                ]),
                desktop: Some("none".into()),
                output: Some(OutputFormat::Wsl),
                flatpak_apps: Some(vec![]),
                ..Default::default()
            },
        },
        Template {
            id: "media",
            label: "Multimedia",
            desc: "VLC y GIMP nativos + Flatpaks de Flathub.",
            patch: TemplatePatch {
                packages: Some(vec!["vlc".into(), "gimp".into(), "audacity".into()]),
                flatpak_apps: Some(vec!["org.videolan.VLC".into(), "org.gimp.GIMP".into()]),
                ..Default::default()
            },
        },
    ]
});

/// Aplica una plantilla sobre la config actual respetando la base elegida: los
/// ids de escritorio que la base no tiene se conservan tal cual. Port de
/// `applyTemplate`.
pub fn apply_template(cfg: &DistroConfig, tpl: &Template) -> DistroConfig {
    let b = base_by_id(cfg.base);
    let p = &tpl.patch;
    let mut out = cfg.clone();

    // `...rest`: todos los campos del parche salvo desktop/system/grub.
    if let Some(v) = &p.name {
        out.name = v.clone();
    }
    if let Some(v) = &p.base {
        out.base = *v;
    }
    if let Some(v) = &p.packages {
        out.packages = v.clone();
    }
    if let Some(v) = &p.repos {
        out.repos = v.clone();
    }
    if let Some(v) = &p.output {
        out.output = *v;
    }
    if let Some(v) = &p.flatpak_apps {
        out.flatpak_apps = v.clone();
    }
    if let Some(v) = &p.extra_files {
        out.extra_files = v.clone();
    }
    if let Some(v) = &p.users {
        out.users = v.clone();
    }
    if let Some(v) = &p.services {
        out.services = v.clone();
    }
    if let Some(v) = &p.network {
        out.network = v.clone();
    }
    if let Some(v) = &p.firewall {
        out.firewall = *v;
    }
    if let Some(v) = &p.arch {
        out.arch = *v;
    }
    if let Some(v) = &p.advanced {
        out.advanced = *v;
    }
    if let Some(v) = &p.custom_iso {
        out.custom_iso = v.clone();
    }
    if let Some(v) = &p.autologin {
        out.autologin = *v;
    }
    if let Some(v) = &p.autostart {
        out.autostart = v.clone();
    }
    if let Some(v) = &p.firewall_ports {
        out.firewall_ports = v.clone();
    }
    if let Some(v) = &p.boot_target {
        out.boot_target = *v;
    }
    if let Some(v) = &p.cloud_init {
        out.cloud_init = *v;
    }
    if let Some(v) = &p.motd {
        out.motd = v.clone();
    }
    if let Some(v) = &p.wallpaper {
        out.wallpaper = v.clone();
    }

    // Desktop: solo si la base lo tiene ("" cuenta como ausente, como en el
    // TS, donde `desktop &&` es falsy con cadena vacía).
    let desktop = p.desktop.as_deref().filter(|d| !d.is_empty());
    if let Some(d) = desktop {
        if b.desktops.iter().any(|x| x.id == d) {
            out.desktop = d.to_string();
        }
    }

    // grub y system: fusión por claves.
    if let Some(g) = &p.grub {
        if let Some(v) = g.timeout {
            out.grub.timeout = v;
        }
        if let Some(v) = &g.extra_params {
            out.grub.extra_params = v.clone();
        }
    }
    if let Some(s) = &p.system {
        let SystemConfig {
            hostname,
            username,
            password,
            timezone,
            locale,
            keyboard,
        } = &mut out.system;
        if let Some(v) = &s.hostname {
            *hostname = v.clone();
        }
        if let Some(v) = &s.username {
            *username = v.clone();
        }
        if let Some(v) = &s.password {
            *password = v.clone();
        }
        if let Some(v) = &s.timezone {
            *timezone = v.clone();
        }
        if let Some(v) = &s.locale {
            *locale = v.clone();
        }
        if let Some(v) = &s.keyboard {
            *keyboard = v.clone();
        }
    }
    out
}

/// Constructor auxiliar para tests/usos avanzados: `GrubConfig` completo.
#[allow(dead_code)]
fn full_grub(timeout: f64, extra_params: &str) -> GrubConfig {
    GrubConfig {
        timeout,
        extra_params: extra_params.to_string(),
    }
}
