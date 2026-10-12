//! Usuarios, servicios, firewall, autostart, autologin y target systemd. Port
//! de `src/generators/accounts.ts`.

use crate::catalog::desktop_packages;
use crate::generators::shell::{dq, systemd_safe};
use crate::types::DistroConfig;

/// Línea shell para el firewall (firewalld); tolerante si no está instalado.
pub fn firewall_service_line(cfg: &DistroConfig) -> String {
    if cfg.firewall {
        "systemctl enable \"firewalld\" || true".to_string()
    } else {
        "systemctl disable \"firewalld\" || true".to_string()
    }
}

/// Igual que [`firewall_service_line`] pero como ExecStart de systemd.
pub fn firewall_exec_start(cfg: &DistroConfig) -> String {
    if cfg.firewall {
        "ExecStart=/bin/sh -c 'systemctl enable --now \"firewalld\" || true'".to_string()
    } else {
        "ExecStart=/bin/sh -c 'systemctl disable --now \"firewalld\" || true'".to_string()
    }
}

/// Sudo/wheel según familia de gestor de paquetes (lo pasan los generadores).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdminGroup {
    /// Grupo sudo (familia deb).
    Sudo,
    /// Grupo wheel (rpm/pacman).
    Wheel,
    /// Grupo users (KIWI).
    Users,
}

impl AdminGroup {
    /// Nombre del grupo como string.
    pub fn as_str(self) -> &'static str {
        match self {
            AdminGroup::Sudo => "sudo",
            AdminGroup::Wheel => "wheel",
            AdminGroup::Users => "users",
        }
    }
}

/// ¿Es un nombre de unidad systemd válido? (`/^[A-Za-z0-9@._-]+$/`)
fn valid_unit(unit: &str) -> bool {
    !unit.is_empty()
        && unit
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'@' | b'.' | b'_' | b'-'))
}

/// Líneas shell para crear los usuarios extra (live-build, Dockerfile, WSL).
pub fn extra_user_shell_lines(cfg: &DistroConfig, admin_group: AdminGroup) -> Vec<String> {
    cfg.users
        .iter()
        .map(|u| {
            let groups = if u.admin {
                format!(" -G {}", admin_group.as_str())
            } else {
                String::new()
            };
            format!(
                "id \"{}\" >/dev/null 2>&1 || useradd -m -s /bin/bash{groups} \"{}\"\necho \"{}:{}\" | chpasswd",
                dq(&u.name),
                dq(&u.name),
                dq(&u.name),
                dq(&u.password)
            )
        })
        .collect()
}

/// Líneas shell para systemctl enable/disable de los servicios elegidos.
pub fn service_shell_lines(cfg: &DistroConfig) -> Vec<String> {
    cfg.services
        .iter()
        .filter(|s| valid_unit(&s.unit))
        .map(|s| {
            if s.enabled {
                format!("systemctl enable \"{}\"", s.unit)
            } else {
                format!("systemctl disable \"{}\" || true", s.unit)
            }
        })
        .collect()
}

/// Líneas systemd ExecStart para crear usuarios extra al arrancar (archiso no
/// tiene hooks de chroot; tolera re-ejecuciones).
pub fn user_exec_start_lines(cfg: &DistroConfig, admin_group: AdminGroup) -> Vec<String> {
    cfg.users
        .iter()
        .map(|u| {
            let groups = if u.admin {
                format!(" -G {}", admin_group.as_str())
            } else {
                String::new()
            };
            format!(
                "ExecStart=/bin/sh -c 'id \"{}\" >/dev/null 2>&1 || useradd -m -s /bin/bash{groups} \"{}\"; echo \"{}:{}\" | chpasswd'",
                systemd_safe(&u.name),
                systemd_safe(&u.name),
                systemd_safe(&u.name),
                systemd_safe(&u.password)
            )
        })
        .collect()
}

/// Líneas systemd ExecStart para enable --now / disable --now de servicios.
pub fn service_exec_start_lines(cfg: &DistroConfig) -> Vec<String> {
    cfg.services
        .iter()
        .filter(|s| valid_unit(&s.unit))
        .map(|s| {
            if s.enabled {
                format!("ExecStart=/bin/sh -c 'systemctl enable --now \"{}\" || true'", s.unit)
            } else {
                format!("ExecStart=/bin/sh -c 'systemctl disable --now \"{}\" || true'", s.unit)
            }
        })
        .collect()
}

/// Target systemd de arranque, respetando la elección del usuario y el
/// escritorio.
pub fn default_target(cfg: &DistroConfig) -> &'static str {
    use crate::types::BootTarget;
    if cfg.boot_target == BootTarget::Text {
        return "multi-user";
    }
    let has_desktop = !desktop_packages(cfg.base, &cfg.desktop).is_empty();
    if cfg.boot_target == BootTarget::Graphical {
        return "graphical";
    }
    if has_desktop {
        "graphical"
    } else {
        "multi-user"
    }
}

/// Línea systemctl set-default correspondiente.
pub fn target_shell_line(cfg: &DistroConfig) -> String {
    format!("systemctl set-default {}.target", default_target(cfg))
}

/// ¿Es un puerto "8080/tcp" o un servicio "ssh" según el filtro del TS?
fn valid_port_or_service(x: &str) -> bool {
    if let Some((num, proto)) = x.split_once('/') {
        !num.is_empty()
            && num.bytes().all(|c| c.is_ascii_digit())
            && (proto == "tcp" || proto == "udp")
    } else {
        let mut chars = x.chars();
        matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
            && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
    }
}

/// Líneas firewalld para abrir los puertos/servicios elegidos.
pub fn firewall_open_lines(cfg: &DistroConfig) -> Vec<String> {
    cfg.firewall_ports
        .iter()
        .filter(|x| valid_port_or_service(x))
        .map(|x| {
            if x.contains('/') {
                format!("firewall-cmd --permanent --add-port={x} || true")
            } else {
                format!("firewall-cmd --permanent --add-service={x} || true")
            }
        })
        .collect()
}

/// EULA en el primer login interactivo: fichero + profile.d que lo muestra una
/// vez. Port de `eulaFiles`.
pub fn eula_files(cfg: &DistroConfig) -> Vec<(String, String)> {
    let text = cfg.eula_text.trim();
    if text.is_empty() {
        return vec![];
    }
    let heredoc = format!("cat > /etc/distroforge/EULA <<'DFEULA'\n{text}\nDFEULA");
    vec![
        (
            "/etc/distroforge/EULA".to_string(),
            format!("{text}\n"),
        ),
        (
            "/etc/profile.d/zz-distroforge-eula.sh".to_string(),
            format!(
                "#!/bin/sh\n# DistroForge - muestra la EULA en el primer login interactivo\nif [ -t 0 ] && [ ! -f /var/lib/distroforge/eula-shown ]; then\n{heredoc}\n  mkdir -p /var/lib/distroforge\n  touch /var/lib/distroforge/eula-shown\nfi\n"
            ),
        ),
    ]
}

/// Entradas .desktop de autostart para el usuario principal: ruta → contenido./// Devuelve pares en el orden del TS (índice del programa).
pub fn autostart_files(cfg: &DistroConfig) -> Vec<(String, String)> {
    let u = &cfg.system.username;
    let mut out = Vec::new();
    for (i, prog) in cfg.autostart.iter().enumerate() {
        // prog.replace(/["'\/\$`]/g, "").trim()
        let name: String = prog
            .chars()
            .filter(|c| !matches!(c, '"' | '\'' | '/' | '$' | '`'))
            .collect();
        let name = name.trim();
        if name.is_empty() {
            continue;
        }
        let fname: String = name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                    c
                } else {
                    '_'
                }
            })
            .collect();
        out.push((
            format!("/home/{u}/.config/autostart/10-distroforge-{i}-{fname}.desktop"),
            format!(
                "[Desktop Entry]\nType=Application\nName={name}\nExec={name}\nX-GNOME-Autostart-enabled=true\n"
            ),
        ));
    }
    out
}

/// Display manager deducido de los paquetes del escritorio.
fn dm_of(cfg: &DistroConfig) -> Option<&'static str> {
    let pkgs = desktop_packages(cfg.base, &cfg.desktop);
    if pkgs.contains(&"sddm") {
        return Some("sddm");
    }
    if pkgs.contains(&"gdm3") || pkgs.contains(&"gdm") {
        return Some("gdm");
    }
    if pkgs.contains(&"lightdm") {
        return Some("lightdm");
    }
    // Fedora (kickstart) usa grupos @^... que no nombran el DM: deducimos por escritorio
    if matches!(cfg.desktop.as_str(), "gnome" | "zorincore" | "popgnome") {
        return Some("gdm");
    }
    if cfg.desktop == "kde" {
        return Some("sddm");
    }
    if matches!(cfg.desktop.as_str(), "xfce" | "mate" | "lxqt" | "cinnamon") {
        return Some("lightdm");
    }
    None
}

/// Ficheros de auto-login por gestor de sesión, con ruta absoluta de destino.
/// Devuelve vec vacío si no procede (sin checkbox o sin DM conocido).
pub fn autologin_files(cfg: &DistroConfig) -> Vec<(String, String)> {
    if !cfg.autologin {
        return vec![];
    }
    let dm = dm_of(cfg);
    let u = &cfg.system.username;
    match dm {
        Some("sddm") => vec![(
            "/etc/sddm.conf.d/zz-distroforge-autologin.conf".to_string(),
            format!("[Autologin]\nUser={u}\n"),
        )],
        Some("gdm") => vec![
            (
                "/etc/gdm/custom.conf".to_string(),
                format!("[daemon]\nAutomaticLoginEnable=True\nAutomaticLogin={u}\n"),
            ),
            (
                "/etc/gdm3/custom.conf".to_string(),
                format!("[daemon]\nAutomaticLoginEnable=True\nAutomaticLogin={u}\n"),
            ),
        ],
        Some("lightdm") => vec![(
            "/etc/lightdm/lightdm.conf".to_string(),
            format!("[Seat:*]\nautologin-user={u}\nautologin-user-timeout=0\n"),
        )],
        _ => vec![],
    }
}
