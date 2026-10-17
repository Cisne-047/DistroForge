//! Wallpaper: data URLs, dconf (GNOME-family) y xfconf (Xfce). Port de
//! `src/generators/branding.ts`.

use crate::types::DistroConfig;

/// Familia de paquetería para los paquetes extra del wallpaper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PkgFamily {
    /// apt (live-build).
    Apt,
    /// pacman (archiso).
    Pacman,
    /// zypper (KIWI).
    Zypper,
}

/// ¿La config trae un wallpaper con data URL soportada?
fn wallpaper_matches(cfg: &DistroConfig) -> Option<&str> {
    for ext in ["png", "jpeg", "webp"] {
        let prefix = format!("data:image/{ext};base64,");
        if let Some(rest) = cfg.wallpaper.strip_prefix(prefix.as_str()) {
            let _ = rest;
            return Some(ext);
        }
    }
    None
}

/// Nombre de fichero del wallpaper ("distroforge-wallpaper.png" etc.), o `None`
/// si no hay wallpaper válido.
pub fn wallpaper_file_name(cfg: &DistroConfig) -> Option<String> {
    let m = wallpaper_matches(cfg)?;
    Some(format!(
        "distroforge-wallpaper.{}",
        if m == "jpeg" { "jpg" } else { m }
    ))
}

/// La data URL completa del wallpaper, o `None`.
pub fn wallpaper_data_url(cfg: &DistroConfig) -> Option<&str> {
    if wallpaper_matches(cfg).is_some() {
        Some(&cfg.wallpaper)
    } else {
        None
    }
}

/// Esquema dconf del wallpaper según escritorio; `None` si no usa dconf.
pub fn dconf_schema_for(desktop_id: &str) -> Option<&'static str> {
    match desktop_id {
        "gnome" | "zorincore" | "popgnome" | "budgie" => Some("org/gnome/desktop/background"),
        "mate" => Some("org/mate/desktop/background"),
        "cinnamon" => Some("org/cinnamon/desktop/background"),
        _ => None,
    }
}

/// El wallpaper se aplica de verdad en GNOME-family (dconf) y Xfce (xfconf).
pub fn wallpaper_applies(cfg: &DistroConfig) -> bool {
    !cfg.wallpaper.is_empty() && (dconf_schema_for(&cfg.desktop).is_some() || cfg.desktop == "xfce")
}

/// Ficheros dconf (perfil + base de datos) con rutas absolutas de destino.
pub fn dconf_db_files(cfg: &DistroConfig) -> Vec<(String, String)> {
    let Some(schema) = dconf_schema_for(&cfg.desktop) else {
        return vec![];
    };
    let Some(file) = wallpaper_file_name(cfg) else {
        return vec![];
    };
    vec![
        (
            "/etc/dconf/profile/user".to_string(),
            "user-db:user\nsystem-db:distroforge\n".to_string(),
        ),
        (
            "/etc/dconf/db/distroforge.d/00-wallpaper".to_string(),
            format!(
                "[{schema}]\npicture-uri='file:///usr/share/backgrounds/{file}'\npicture-uri-dark='file:///usr/share/backgrounds/{file}'\n"
            ),
        ),
    ]
}

/// XML de xfconf para fijar el wallpaper en Xfce.
pub fn xfconf_wallpaper_xml(cfg: &DistroConfig) -> String {
    let file = wallpaper_file_name(cfg).unwrap_or_else(|| "distroforge-wallpaper.png".to_string());
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\n<channel name=\"xfce4-desktop\" version=\"1.0\">\n  <property name=\"backdrop\" type=\"empty\">\n    <property name=\"screen0\" type=\"empty\">\n      <property name=\"monitor0\" type=\"empty\">\n        <property name=\"workspace0\" type=\"empty\">\n          <property name=\"last-image\" type=\"string\" value=\"/usr/share/backgrounds/{file}\"/>\n        </property>\n      </property>\n    </property>\n  </property>\n</channel>\n"
    )
}

/// Paquetes extra para aplicar el wallpaper según familia de gestor.
pub fn wallpaper_packages(cfg: &DistroConfig, family: PkgFamily) -> Vec<String> {
    if !wallpaper_applies(cfg) {
        return vec![];
    }
    if dconf_schema_for(&cfg.desktop).is_some() {
        // dconf-cli en apt; dconf en pacman y zypper
        return vec![match family {
            PkgFamily::Apt => "dconf-cli",
            _ => "dconf",
        }
        .to_string()];
    }
    vec![] // xfconf ya viene con xfce4
}

/// Servicio + tmpfiles para compilar la base dconf al arrancar (necesario en
/// archiso, donde no hay chroot hooks en build).
pub fn arch_dconf_boot_files() -> Vec<(String, String)> {
    vec![
        (
            "/etc/systemd/system/distroforge-dconf.service".to_string(),
            "[Unit]\nDescription=Compila la base de datos dconf de DistroForge\n[Service]\nType=oneshot\nExecStart=/usr/bin/dconf update\n[Install]\nWantedBy=multi-user.target\n".to_string(),
        ),
        (
            "/etc/tmpfiles.d/distroforge-dconf.conf".to_string(),
            "L /etc/systemd/system/multi-user.target.wants/distroforge-dconf.service - - - - /etc/systemd/system/distroforge-dconf.service\n".to_string(),
        ),
    ]
}

/// `LOGO_RE` del TS (`/^data:image\/(png|jpeg|webp);base64,/`): ¿el logo es una
/// data URL png/jpeg/webp?
pub fn logo_matches(s: &str) -> bool {
    ["png", "jpeg", "webp"]
        .iter()
        .any(|e| s.starts_with(&format!("data:image/{e};base64,")))
}

/// Ficheros del logo Personalize: binario en /usr/share/distroforge y config
/// del greeter LightDM cuando aplica. Vacío si no hay logo. Port de `logoFiles`.
pub fn logo_files(cfg: &DistroConfig) -> Vec<(String, String)> {
    if !logo_matches(&cfg.logo) {
        return vec![];
    }
    let ext = if cfg.logo.starts_with("data:image/jpeg;") {
        "jpg"
    } else if cfg.logo.starts_with("data:image/webp;") {
        "webp"
    } else {
        "png"
    };
    let dest = format!("/usr/share/distroforge/logo.{ext}");
    vec![
        (dest.clone(), cfg.logo.clone()),
        (
            "/etc/lightdm/lightdm-gtk-greeter.conf.d/50-distroforge-logo.conf".to_string(),
            format!("[greeter]\nlogo={dest}\n"),
        ),
    ]
}
