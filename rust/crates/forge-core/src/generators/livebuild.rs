//! Generador live-build (Debian y derivadas de Ubuntu). Port de
//! `src/generators/livebuild.ts`.

use crate::catalog::{base_by_id, deb_arch, desktop_packages, effective_repos};
use crate::generators::accounts::{
    autologin_files, autostart_files, eula_files, extra_user_shell_lines, firewall_open_lines,
    firewall_service_line, service_shell_lines, target_shell_line, AdminGroup,
};
use crate::generators::branding::{
    dconf_db_files, dconf_schema_for, logo_files, wallpaper_applies, wallpaper_data_url,
    wallpaper_file_name, wallpaper_packages, xfconf_wallpaper_xml, PkgFamily,
};
use crate::generators::cloudinit::{cloud_init_yaml, CLOUD_INIT_PACKAGE};
use crate::generators::flatpak::{flatpak_packages, flatpak_shell_lines, has_flatpak_apps};
use crate::generators::network::{nm_connection, static_network};
use crate::generators::shell::{dq, sq};
use crate::types::{js_num, DistroConfig};

/// Sanea un alias de repo para nombres de fichero
/// (`replace(/[^a-zA-Z0-9._-]/g, "_").replace(/^[._-]+/, "")`).
pub fn sanitize_alias(a: &str, i: usize) -> String {
    let s: String = a
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-') {
                c
            } else {
                '_'
            }
        })
        .collect();
    let s = s.trim_start_matches(['.', '_', '-']);
    if s.is_empty() {
        format!("repo-{}", i + 1)
    } else {
        s.to_string()
    }
}

/// Primeros 28 caracteres, como `String.prototype.slice(0, 28)`.
pub(crate) fn slice28(s: &str) -> String {
    s.chars().take(28).collect()
}

/// Ficheros del proyecto live-build, en el orden de construcción del TS.
pub fn livebuild_files(cfg: &DistroConfig) -> Vec<(String, String)> {
    let base = base_by_id(cfg.base);
    let sys = &cfg.system;
    let distro = base.distribution.unwrap_or("bookworm");
    let areas = base
        .archive_areas
        .unwrap_or("main contrib non-free non-free-firmware");
    let mirror = if base.mirror.is_empty() {
        "http://deb.debian.org/debian"
    } else {
        base.mirror
    };
    let kernel = base.kernel_package.unwrap_or("linux-image-amd64");
    let flavour = base.lb_flavour.unwrap_or("amd64");
    let container = base.container.unwrap_or("debian:bookworm");
    let repos = effective_repos(cfg);

    let lb_arch = deb_arch(cfg.arch);
    let extra_params = cfg
        .grub
        .extra_params
        .replace(['"', '`', '$'], "")
        .trim()
        .to_string();
    let boot_append = format!(
        "boot=live components locales={l} keyboard-layouts={k} timezone={tz} hostname={h} username={u}{extra}",
        l = sys.locale,
        k = sys.keyboard,
        tz = sys.timezone,
        h = sys.hostname,
        u = sys.username,
        extra = if extra_params.is_empty() {
            String::new()
        } else {
            format!(" {extra_params}")
        }
    );

    let auto_config = format!(
        "#!/bin/sh\nset -e\n# DistroForge - configuracion live-build ({label})\nlb config noauto \\\n    --architectures {lb_arch} \\\n    --distribution {distro} \\\n    --archive-areas \"{areas}\" \\\n    --mirror-bootstrap {mirror} \\\n    --mirror-binary {mirror} \\\n    --binary-images iso-hybrid \\\n    --linux-packages \"linux-image\" \\\n    --linux-flavours {flavour} \\\n    --iso-volume \"{vol}\" \\\n    --bootappend-live \"{boot_append}\" \\\n    --bootappend-install \"{boot_install}\" \\\n{di_line}    \"$@\"\n",
        label = base.label,
        lb_arch = lb_arch,
        distro = distro,
        areas = areas,
        mirror = mirror,
        flavour = flavour,
        vol = slice28(&cfg.name),
        boot_append = boot_append,
        // `--bootappend-install` SIEMPRE (vacío si no es desatendida); el
        // `--debian-installer live` solo con unattended. El TS mete DOS
        // backslashes en esa línea (`"\\\\\n"` en su código): se replica tal cual.
        boot_install = if cfg.unattended {
            "auto=true priority=critical"
        } else {
            ""
        },
        di_line = if cfg.unattended {
            "    --debian-installer live \\\\\n"
        } else {
            ""
        }
    );

    let auto_build = "#!/bin/sh\nset -e\nlb build noauto \"$@\" 2>&1 | tee build.log\n".to_string();

    let user_hook = format!(
        "#!/bin/sh\nset -e\n# DistroForge - usuario y contrasena del live\nid \"{user}\" >/dev/null 2>&1 || useradd -m -s /bin/bash -G sudo \"{user}\"\necho \"{uq}:{pq}\" | chpasswd\nusermod -aG sudo \"{user}\" || true\n{extras}\n",
        user = sys.username,
        uq = dq(&sys.username),
        pq = dq(&sys.password),
        extras = extra_user_shell_lines(cfg, AdminGroup::Sudo).join("\n")
    );

    let repo_hooks: Vec<String> = repos
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let alias = sanitize_alias(&r.alias, i);
            let key_lines = if !r.gpg_key.is_empty() {
                format!(
                    "wget -qO /usr/share/keyrings/{alias}.asc \"{}\"\nSIGNED=\"[signed-by=/usr/share/keyrings/{alias}.asc]\"",
                    r.gpg_key
                )
            } else {
                "SIGNED=\"[trusted=yes]\"".to_string()
            };
            let deb_line = if !r.suite.is_empty() {
                sq(&format!("deb $SIGNED {} {}", r.url, r.suite))
            } else {
                sq(&format!("deb $SIGNED {} ./", r.url))
            };
            // Ojo: el TS genera un salto de línea REAL dentro del formato de
            // printf ('%s\n' partido en dos líneas del fichero). Se replica.
            format!(
                "#!/bin/sh\nset -e\n# DistroForge - repo custom: {alias}\nmkdir -p /usr/share/keyrings\n{key_lines}\nprintf '%s\n' {deb_line} > /etc/apt/sources.list.d/distroforge-{alias}.list\n"
            )
        })
        .collect();

    let mut pkg_items: Vec<String> = vec![
        kernel.to_string(),
        "live-boot".to_string(),
        "live-config".to_string(),
        "systemd".to_string(),
        "sudo".to_string(),
        "wget".to_string(),
        "ca-certificates".to_string(),
    ];
    if cfg.cloud_init {
        pkg_items.push(CLOUD_INIT_PACKAGE.to_string());
    }
    if cfg.firewall {
        pkg_items.push("firewalld".to_string());
    }
    pkg_items.extend(wallpaper_packages(cfg, PkgFamily::Apt));
    pkg_items.extend(flatpak_packages(cfg));
    pkg_items.extend(
        desktop_packages(cfg.base, &cfg.desktop)
            .iter()
            .map(|s| s.to_string()),
    );
    pkg_items.extend(cfg.packages.iter().cloned());
    let pkg_list = pkg_items.join("\n");

    let build_sh = format!(
        "#!/usr/bin/env bash\nset -euo pipefail\n# DistroForge - build local (Linux o WSL2, requiere Podman)\nDIR=\"$(cd \"$(dirname \"$0\")\" && pwd)\"\npodman run --rm --privileged -v \"$DIR\":/project -w /project/livebuild {container} bash -ec '\n  chmod +x auto/* config/hooks/* 2>/dev/null || true\n  apt-get update\n  apt-get install -y live-build wget ca-certificates\n  lb clean >/dev/null 2>&1 || true\n  lb config\n  lb build\n'\necho \"ISO en ./livebuild/live-image-amd64.hybrid.iso\"\n",
        container = container
    );

    let flatpak_hook = if has_flatpak_apps(cfg) {
        format!(
            "#!/bin/sh\nset -e\n# DistroForge - flatpak: flathub + apps\n{}\n",
            flatpak_shell_lines(cfg, "").join("\n")
        )
    } else {
        String::new()
    };

    let services_hook = {
        let mut lines = vec![target_shell_line(cfg), firewall_service_line(cfg)];
        lines.extend(firewall_open_lines(cfg));
        lines.extend(service_shell_lines(cfg));
        format!(
            "#!/bin/sh\nset -e\n# DistroForge - firewall y servicios systemd\n{}\n",
            lines.join("\n")
        )
    };

    let net = static_network(cfg);
    let mut net_files: Vec<(String, String)> = Vec::new();
    if let Some(n) = net {
        net_files.push((
            "livebuild/config/includes.chroot/etc/NetworkManager/system-connections/distroforge.nmconnection".to_string(),
            nm_connection(n),
        ));
        // NM ignora keyfiles legibles por el mundo: 600 en el build
        net_files.push((
            "livebuild/config/hooks/0330-distroforge-net.hooks.chroot".to_string(),
            "#!/bin/sh\nset -e\n# DistroForge - permisos del keyfile de red\nchmod 600 /etc/NetworkManager/system-connections/distroforge.nmconnection || true\n".to_string(),
        ));
    }

    let mut wallpaper_files: Vec<(String, String)> = Vec::new();
    if wallpaper_applies(cfg) {
        if let Some(wf) = wallpaper_file_name(cfg) {
            wallpaper_files.push((
                format!("livebuild/config/includes.chroot/usr/share/backgrounds/{wf}"),
                wallpaper_data_url(cfg).unwrap_or_default().to_string(),
            ));
            if dconf_schema_for(&cfg.desktop).is_some() {
                for (p, c) in dconf_db_files(cfg) {
                    wallpaper_files.push((format!("livebuild/config/includes.chroot{p}"), c));
                }
                wallpaper_files.push((
                    "livebuild/config/hooks/0300-dconf.hooks.chroot".to_string(),
                    "#!/bin/sh\nset -e\n# DistroForge - compila la base dconf\ndconf update || true\n".to_string(),
                ));
            }
            if cfg.desktop == "xfce" {
                wallpaper_files.push((
                    "livebuild/config/includes.chroot/etc/xdg/xfce4/xfconf/xfce-perchannel-xml/xfce4-desktop.xml".to_string(),
                    xfconf_wallpaper_xml(cfg),
                ));
            }
        }
    }

    let mut files: Vec<(String, String)> = vec![
        ("livebuild/auto/config".to_string(), auto_config),
        ("livebuild/auto/build".to_string(), auto_build),
        (
            "livebuild/config/package-lists/distroforge.list.chroot".to_string(),
            format!("{pkg_list}\n"),
        ),
        (
            "livebuild/config/includes.chroot/etc/hostname".to_string(),
            format!("{}\n", sys.hostname),
        ),
        (
            "livebuild/config/includes.chroot/etc/motd".to_string(),
            format!("{}\n", cfg.motd),
        ),
        (
            "livebuild/config/hooks/0100-distroforge-user.hooks.chroot".to_string(),
            user_hook,
        ),
    ];
    if cfg.cloud_init {
        files.push((
            "livebuild/config/includes.chroot/etc/cloud/cloud.cfg.d/80-distroforge.cfg"
                .to_string(),
            cloud_init_yaml(cfg),
        ));
    }
    for f in &cfg.extra_files {
        files.push((
            format!("livebuild/config/includes.chroot{}", f.path),
            f.content.clone(),
        ));
    }
    for (p, c) in autologin_files(cfg) {
        files.push((format!("livebuild/config/includes.chroot{p}"), c));
    }
    for (p, c) in autostart_files(cfg) {
        files.push((format!("livebuild/config/includes.chroot{p}"), c));
    }
    for (p, c) in logo_files(cfg) {
        files.push((format!("livebuild/config/includes.chroot{p}"), c));
    }
    for (p, c) in eula_files(cfg) {
        files.push((format!("livebuild/config/includes.chroot{p}"), c));
    }
    files.push((
        "livebuild/config/hooks/0340-services.hooks.chroot".to_string(),
        services_hook,
    ));
    files.extend(net_files);
    // live-build no acepta ninguna opción de timeout (`--grub-timeout` no existe:
    // `lb config` aborta con "unrecognized option" y el proyecto no se construye).
    // Se ajusta en la fase binary, que es donde live-build sí lo permite. En
    // arranque BIOS manda isolinux y su `timeout` va en décimas de segundo (0 =
    // esperar siempre); en GRUB/UEFI el menú usa `set timeout=`.
    let arranque_timeout = js_num(cfg.grub.timeout);
    let isolinux_timeout = format!("{}", (cfg.grub.timeout * 10.0).round() as i64);
    files.push((
        "livebuild/config/hooks/0900-grub-timeout.hook.binary".to_string(),
        format!(
            "#!/bin/sh\nset -e\n# DistroForge - timeout del menu de arranque ({t}s)\nfor cfg in binary/isolinux/isolinux.cfg binary/boot/grub/grub.cfg binary/boot/grub/config.cfg; do\n  if [ -f \"$cfg\" ]; then\n    sed -i \"s/^timeout .*/timeout {tt}/\" \"$cfg\" || true\n    sed -i \"s/^set timeout=.*/set timeout={t}/\" \"$cfg\" || true\n  fi\ndone\n",
            t = arranque_timeout,
            tt = isolinux_timeout
        ),
    ));
    if !flatpak_hook.is_empty() {
        files.push((
            "livebuild/config/hooks/0350-flatpak.hooks.chroot".to_string(),
            flatpak_hook,
        ));
    }
    if cfg.unattended {
        files.push((
            "livebuild/config/includes.installer/preseed.cfg".to_string(),
            format!(
                "d-i debian-installer/locale string {l}\nd-i keyboard-configuration/xkb-keymap select {k}\nd-i clock-setup/utc boolean true\nd-i time/zone string {tz}\nd-i passwd/user-fullname string {u}\nd-i passwd/username string {u}\nd-i passwd/user-password password {p}\nd-i passwd/user-password-again password {p}\nd-i netcfg/get_hostname string {h}\nd-i netcfg/get_domain string local\nd-i grub-installer/only_debian boolean true\nd-i finish-install/reboot_in_progress note\n",
                l = sys.locale,
                k = sys.keyboard,
                tz = sys.timezone,
                u = sys.username,
                p = sys.password,
                h = sys.hostname
            ),
        ));
    }
    files.extend(wallpaper_files);
    files.push(("build.sh".to_string(), build_sh));
    for (i, hook) in repo_hooks.iter().enumerate() {
        let alias = sanitize_alias(&repos[i].alias, i);
        files.push((
            format!("livebuild/config/hooks/02{i:02}-repo-{alias}.hooks.chroot"),
            hook.clone(),
        ));
    }
    files
}
