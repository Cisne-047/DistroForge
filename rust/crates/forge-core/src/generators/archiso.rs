//! Generador mkarchiso (Arch, Manjaro, CachyOS). Port de
//! `src/generators/archiso.ts`.

use crate::catalog::{base_by_id, desktop_packages, effective_repos};
use crate::generators::accounts::{
    autologin_files, autostart_files, eula_files, firewall_exec_start, service_exec_start_lines,
    target_shell_line, user_exec_start_lines, AdminGroup,
};
use crate::generators::branding::{
    arch_dconf_boot_files, dconf_db_files, dconf_schema_for, logo_files, wallpaper_applies,
    wallpaper_data_url, wallpaper_file_name, wallpaper_packages, xfconf_wallpaper_xml, PkgFamily,
};
use crate::generators::cloudinit::{cloud_init_yaml, CLOUD_INIT_PACKAGE};
use crate::generators::flatpak::{flatpak_apps, flatpak_packages, has_flatpak_apps, FLATHUB};
use crate::generators::livebuild::sanitize_alias;
use crate::generators::shell::sq;
use crate::types::{js_num, DistroConfig};

/// Ficheros del proyecto archiso, en el orden de construcción del TS.
pub fn archiso_files(cfg: &DistroConfig) -> Vec<(String, String)> {
    let base = base_by_id(cfg.base);
    let sys = &cfg.system;

    let mut pkg_items: Vec<String> = vec![
        "base".to_string(),
        base.kernel_package.unwrap_or("linux").to_string(),
        "linux-firmware".to_string(),
        "networkmanager".to_string(),
        "sudo".to_string(),
    ];
    if cfg.cloud_init {
        pkg_items.push(CLOUD_INIT_PACKAGE.to_string());
    }
    if cfg.firewall {
        pkg_items.push("firewalld".to_string());
    }
    pkg_items.extend(wallpaper_packages(cfg, PkgFamily::Pacman));
    pkg_items.extend(flatpak_packages(cfg));
    pkg_items.extend(
        desktop_packages(cfg.base, &cfg.desktop)
            .iter()
            .map(|s| s.to_string()),
    );
    pkg_items.extend(cfg.packages.iter().cloned());
    let packages = pkg_items.join("\n");

    let repo_sections = effective_repos(cfg)
        .iter()
        .enumerate()
        .map(|(i, r)| {
            format!(
                "[{}]\nSigLevel = Optional TrustAll\nServer = {}",
                sanitize_alias(&r.alias, i),
                r.url
            )
        })
        .collect::<Vec<_>>()
        .join("\n\n");

    let pacman_conf = format!(
        "[options]\nHoldPkg = pacman glibc\nArchitecture = auto\nCheckSpace\nSigLevel = Required DatabaseOptional\nLocalFileSigLevel = Optional\nParallelDownloads = 5\n\n[core]\nServer = {mirror}\n\n[extra]\nServer = {mirror}\n{sections}",
        mirror = base.mirror,
        sections = if repo_sections.is_empty() {
            String::new()
        } else {
            format!("\n{repo_sections}\n")
        }
    );

    let grub_params = cfg
        .grub
        .extra_params
        .replace(['"', '`', '$', '\\', '/'], "")
        .trim()
        .to_string();
    let grub_seds = format!(
        "  if [ -f /tmp/profile/grub/grub.cfg ]; then\n  sed -i \"s/^set timeout.*/set timeout={timeout}/\" /tmp/profile/grub/grub.cfg{extra}\n  fi",
        timeout = js_num(cfg.grub.timeout),
        extra = if grub_params.is_empty() {
            String::new()
        } else {
            format!(
                "\n  sed -i \"/^[[:space:]]*linux /s/archisolabel=[^ ]*/& {grub_params}/\" /tmp/profile/grub/grub.cfg"
            )
        }
    );

    let build_sh = format!(
        "#!/usr/bin/env bash\nset -euo pipefail\n# DistroForge - build local (Linux o WSL2, requiere Podman)\n# Parte del perfil releng de archiso y aplica tu capa: paquetes, repos y airootfs.\nDIR=\"$(cd \"$(dirname \"$0\")\" && pwd)\"\npodman run --rm --privileged -v \"$DIR\":/project -v \"$DIR/out\":/out archlinux:base-devel bash -ec '\n  pacman -Syu --noconfirm archiso\n  cp -r /usr/share/archiso/configs/releng /tmp/profile\n  cp /project/archiso/packages.x86_64 /tmp/profile/packages.x86_64\n  cp /project/archiso/pacman.conf /tmp/profile/pacman.conf\n  cp -a /project/archiso/airootfs/. /tmp/profile/airootfs/\n{grub_seds}\n  mkdir -p /tmp/work\n  mkarchiso -v -w /tmp/work -o /out /tmp/profile\n'\necho \"Imagen lista en ./out\"\n",
        grub_seds = grub_seds
    );

    let mut flatpak_files: Vec<(String, String)> = Vec::new();
    if has_flatpak_apps(cfg) {
        // archiso no tiene hooks de chroot: Flathub y apps se instalan al primer arranque
        flatpak_files.push((
            "archiso/airootfs/etc/systemd/system/distroforge-flatpak.service".to_string(),
            format!(
                "[Unit]\nDescription=DistroForge - configura Flathub e instala apps Flatpak\nWants=network-online.target\nAfter=network-online.target\n[Service]\nType=oneshot\nExecStart=/usr/bin/flatpak remote-add --if-not-exists flathub {FLATHUB}\nExecStart=/usr/bin/flatpak install -y --noninteractive flathub {}\nRemainAfterExit=yes\n[Install]\nWantedBy=multi-user.target\n",
                flatpak_apps(cfg)
                    .iter()
                    .map(|a| sq(a))
                    .collect::<Vec<_>>()
                    .join(" ")
            ),
        ));
        flatpak_files.push((
            "archiso/airootfs/etc/tmpfiles.d/distroforge-flatpak.conf".to_string(),
            "L /etc/systemd/system/multi-user.target.wants/distroforge-flatpak.service - - - - /etc/systemd/system/distroforge-flatpak.service\n".to_string(),
        ));
    }

    // usuarios extra, servicios y auto-login: sin hooks de chroot en archiso,
    // todo se ejecuta en el primer arranque vía unidades oneshot
    let mut account_files: Vec<(String, String)> = Vec::new();
    let user_execs = user_exec_start_lines(cfg, AdminGroup::Wheel);
    if !user_execs.is_empty() {
        account_files.push((
            "archiso/airootfs/etc/systemd/system/distroforge-users.service".to_string(),
            format!(
                "[Unit]\nDescription=DistroForge - crea los usuarios extra\n[Service]\nType=oneshot\n{}\nRemainAfterExit=yes\n[Install]\nWantedBy=multi-user.target\n",
                user_execs.join("\n")
            ),
        ));
    }
    let mut svc_execs = vec![format!(
        "ExecStart=/bin/sh -c '{} || true'",
        target_shell_line(cfg)
    )];
    svc_execs.extend(service_exec_start_lines(cfg));
    svc_execs.push(firewall_exec_start(cfg));
    svc_execs.extend(cfg.firewall_ports.iter().filter(|x| {
        // filtro /^([A-Za-z][A-Za-z0-9._-]*|\d+\/(tcp|udp))$/
        if let Some((num, proto)) = x.split_once('/') {
            !num.is_empty()
                && num.bytes().all(|c| c.is_ascii_digit())
                && (proto == "tcp" || proto == "udp")
        } else {
            let mut chars = x.chars();
            matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
                && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        }
    }).map(|x| {
        let kind = if x.contains('/') { "add-port" } else { "add-service" };
        format!("ExecStart=/bin/sh -c 'firewall-cmd --permanent --{kind}={x} || firewall-cmd --{kind}={x} || true'")
    }));
    if !svc_execs.is_empty() {
        account_files.push((
            "archiso/airootfs/etc/systemd/system/distroforge-services.service".to_string(),
            format!(
                "[Unit]\nDescription=DistroForge - habilita/deshabilita servicios\n[Service]\nType=oneshot\n{}\nRemainAfterExit=yes\n[Install]\nWantedBy=multi-user.target\n",
                svc_execs.join("\n")
            ),
        ));
    }
    if !sys.locale.is_empty() && sys.locale != "en_US.UTF-8" {
        account_files.push((
            "archiso/airootfs/etc/systemd/system/distroforge-locale.service".to_string(),
            format!(
                "[Unit]\nDescription=DistroForge - genera el locale {}\n[Service]\nType=oneshot\nExecStart=/bin/sh -c 'echo \"{} UTF-8\" >> /etc/locale.gen && locale-gen || true'\nRemainAfterExit=yes\n[Install]\nWantedBy=multi-user.target\n",
                sys.locale, sys.locale
            ),
        ));
    }
    for name in [
        "distroforge-users",
        "distroforge-services",
        "distroforge-locale",
    ] {
        if account_files
            .iter()
            .any(|(p, _)| *p == format!("archiso/airootfs/etc/systemd/system/{name}.service"))
        {
            account_files.push((
                format!("archiso/airootfs/etc/tmpfiles.d/{name}.conf"),
                format!("L /etc/systemd/system/multi-user.target.wants/{name}.service - - - - /etc/systemd/system/{name}.service\n"),
            ));
        }
    }
    for (p, c) in autologin_files(cfg) {
        account_files.push((format!("archiso/airootfs{p}"), c));
    }
    for (p, c) in autostart_files(cfg) {
        account_files.push((format!("archiso/airootfs{p}"), c));
    }
    for (p, c) in logo_files(cfg) {
        account_files.push((format!("archiso/airootfs{p}"), c));
    }
    for (p, c) in eula_files(cfg) {
        account_files.push((format!("archiso/airootfs{p}"), c));
    }

    let mut wallpaper_files: Vec<(String, String)> = Vec::new();
    if wallpaper_applies(cfg) {
        if let Some(wf) = wallpaper_file_name(cfg) {
            wallpaper_files.push((
                format!("archiso/airootfs/usr/share/backgrounds/{wf}"),
                wallpaper_data_url(cfg).unwrap_or_default().to_string(),
            ));
            if dconf_schema_for(&cfg.desktop).is_some() {
                for (p, c) in dconf_db_files(cfg) {
                    wallpaper_files.push((format!("archiso/airootfs{p}"), c));
                }
                for (p, c) in arch_dconf_boot_files() {
                    wallpaper_files.push((format!("archiso/airootfs{p}"), c));
                }
            }
            if cfg.desktop == "xfce" {
                wallpaper_files.push((
                    "archiso/airootfs/etc/xdg/xfce4/xfconf/xfce-perchannel-xml/xfce4-desktop.xml"
                        .to_string(),
                    xfconf_wallpaper_xml(cfg),
                ));
            }
        }
    }

    let mut files: Vec<(String, String)> = vec![
        (
            "archiso/packages.x86_64".to_string(),
            format!("{packages}\n"),
        ),
        ("archiso/pacman.conf".to_string(), pacman_conf),
        (
            "archiso/airootfs/etc/hostname".to_string(),
            format!("{}\n", sys.hostname),
        ),
        (
            "archiso/airootfs/etc/locale.conf".to_string(),
            format!("LANG={}\n", sys.locale),
        ),
        (
            "archiso/airootfs/etc/vconsole.conf".to_string(),
            format!("KEYMAP={}\n", sys.keyboard),
        ),
        (
            "archiso/airootfs/etc/motd".to_string(),
            format!("{}\n", cfg.motd),
        ),
    ];
    if cfg.cloud_init {
        files.push((
            "archiso/airootfs/etc/cloud/cloud.cfg.d/80-distroforge.cfg".to_string(),
            cloud_init_yaml(cfg),
        ));
    }
    for f in &cfg.extra_files {
        files.push((format!("archiso/airootfs{}", f.path), f.content.clone()));
    }
    files.extend(flatpak_files);
    files.extend(account_files);
    files.extend(wallpaper_files);
    files.push(("build.sh".to_string(), build_sh));
    files
}
