//! Generador KIWI NG (openSUSE). Port de `src/generators/kiwi.ts`.

use crate::catalog::{base_by_id, desktop_packages, effective_repos};
use crate::generators::accounts::{
    autologin_files, autostart_files, eula_files, firewall_open_lines, firewall_service_line,
    service_shell_lines, target_shell_line,
};
use crate::generators::branding::{
    dconf_db_files, dconf_schema_for, logo_files, wallpaper_applies, wallpaper_data_url,
    wallpaper_file_name, wallpaper_packages, xfconf_wallpaper_xml, PkgFamily,
};
use crate::generators::cloudinit::{cloud_init_yaml, CLOUD_INIT_PACKAGE};
use crate::generators::flatpak::{flatpak_packages, flatpak_shell_lines, has_flatpak_apps};
use crate::generators::network::{nm_connection, static_network};
use crate::generators::shell::dq;
use crate::types::{js_num, DistroConfig, OutputFormat};

/// Escapa entidades XML básicas, igual que el TS (no escapa comilla simple).
fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

/// Ficheros del proyecto KIWI, en el orden de construcción del TS.
pub fn kiwi_files(cfg: &DistroConfig) -> Vec<(String, String)> {
    let base = base_by_id(cfg.base);
    let sys = &cfg.system;
    let grub_params = cfg
        .grub
        .extra_params
        .replace(['"', '`', '$'], "")
        .trim()
        .to_string();
    let kern_attr = if grub_params.is_empty() {
        String::new()
    } else {
        format!(" kernelcmdline=\"{}\"", esc(&grub_params))
    };
    let type_tag = match cfg.output {
        OutputFormat::Qcow2 => {
            format!("<type image=\"qcow2\" filesystem=\"btrfs\" primary=\"true\"{kern_attr}/>")
        }
        OutputFormat::Usb => {
            let unattended = if cfg.unattended {
                " oem-unattended=\"true\""
            } else {
                ""
            };
            format!("<type image=\"oem\" filesystem=\"btrfs\" primary=\"true\"{kern_attr}{unattended}/>")
        }
        OutputFormat::Vmdk | OutputFormat::Vdi | OutputFormat::Vhd => format!(
            "<type image=\"vmx\" filesystem=\"btrfs\" format=\"{}\" primary=\"true\"{kern_attr}/>",
            cfg.output.as_str()
        ),
        _ => format!("<type image=\"iso\" primary=\"true\" firmware=\"uefi\"{kern_attr}/>"),
    };

    let mut repo_parts = vec![format!(
        "    <repository type=\"rpm-md\" alias=\"{}\" url=\"{}\"/>",
        esc(base.id.as_str()),
        esc(base.mirror)
    )];
    for r in effective_repos(cfg) {
        let gpg = if r.gpg_key.is_empty() {
            " gpgcheck=\"false\""
        } else {
            ""
        };
        repo_parts.push(format!(
            "    <repository type=\"rpm-md\" alias=\"{}\" url=\"{}\"{gpg}/>",
            esc(&r.alias),
            esc(&r.url)
        ));
    }
    let repo_tags = repo_parts.join("\n");

    let mut pkg_items: Vec<String> = vec![
        base.kernel_package.unwrap_or("kernel-default").to_string(),
        "grub2".to_string(),
        "dracut".to_string(),
        "patterns-base-base".to_string(),
    ];
    if cfg.cloud_init {
        pkg_items.push(CLOUD_INIT_PACKAGE.to_string());
    }
    if cfg.firewall {
        pkg_items.push("firewalld".to_string());
    }
    pkg_items.extend(wallpaper_packages(cfg, PkgFamily::Zypper));
    pkg_items.extend(flatpak_packages(cfg));
    pkg_items.extend(
        desktop_packages(cfg.base, &cfg.desktop)
            .iter()
            .map(|s| s.to_string()),
    );
    pkg_items.extend(cfg.packages.iter().cloned());
    let pkgs = pkg_items
        .iter()
        .map(|p| format!("    <package name=\"{}\"/>", esc(p)))
        .collect::<Vec<_>>()
        .join("\n");

    let mut user_lines = vec![format!(
        "    <user name=\"{}\" password=\"{}\" home=\"/home/{}\" groups=\"users\" pwdformat=\"plain\"/>",
        esc(&sys.username),
        esc(&sys.password),
        esc(&sys.username)
    )];
    for u in &cfg.users {
        user_lines.push(format!(
            "    <user name=\"{}\" password=\"{}\" home=\"/home/{}\" groups=\"users{}\" pwdformat=\"plain\"/>",
            esc(&u.name),
            esc(&u.password),
            esc(&u.name),
            if u.admin { ",wheel" } else { "" }
        ));
    }
    let users_xml = user_lines.join("\n");

    let config_xml = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<image schemaversion=\"9.2\" name=\"{}\">\n  <description type=\"system\">\n    <author>{}</author>\n    <contact>{}@localhost</contact>\n    <specification>{} - distro personalizada generada con DistroForge</specification>\n  </description>\n  <preferences>\n    <version>1.0.0</version>\n    <packagemanager>zypper</packagemanager>\n    <locale>{}</locale>\n    <keytable>{}</keytable>\n    <timezone>{}</timezone>\n    <rpm-excludedocs>true</rpm-excludedocs>\n    {}\n    <bootloader name=\"grub2\" timeout=\"{}\"/>\n  </preferences>\n  <users>\n{}\n  </users>\n{}\n  <packages type=\"image\">\n{}\n  </packages>\n  <packages type=\"bootstrap\">\n    <package name=\"filesystem\"/>\n  </packages>\n</image>\n",
        esc(&cfg.name),
        esc(&sys.username),
        esc(&sys.username),
        esc(&cfg.name),
        esc(&sys.locale),
        esc(&sys.keyboard),
        esc(&sys.timezone),
        type_tag,
        js_num(cfg.grub.timeout),
        users_xml,
        repo_tags,
        pkgs
    );

    let net = static_network(cfg);
    let mut net_file: Vec<(String, String)> = Vec::new();
    if let Some(n) = net {
        net_file.push((
            "kiwi/root/etc/NetworkManager/system-connections/distroforge.nmconnection".to_string(),
            nm_connection(n),
        ));
    }

    let mut sh_lines = vec![
        "#!/bin/bash".to_string(),
        "set -euo pipefail".to_string(),
        "# DistroForge - configuracion post-instalacion (se ejecuta en chroot)".to_string(),
        target_shell_line(cfg),
        format!(
            "echo \"{} ALL=(ALL) ALL\" > /etc/sudoers.d/50-{}",
            dq(&sys.username),
            dq(&sys.username)
        ),
    ];
    // las keys de repos custom entran al keyring antes de que zypper las verifique
    for r in effective_repos(cfg) {
        if !r.gpg_key.is_empty() {
            sh_lines.push(format!("rpm --import \"{}\"", dq(&r.gpg_key)));
        }
    }
    sh_lines.push(firewall_service_line(cfg));
    sh_lines.extend(firewall_open_lines(cfg));
    sh_lines.extend(service_shell_lines(cfg));
    if !cfg.motd.is_empty() {
        let marker = crate::security::heredoc_marker("MOTDEOF", &cfg.motd);
        sh_lines.push(format!("cat > /etc/motd <<'{marker}'"));
        sh_lines.push(cfg.motd.clone());
        sh_lines.push(marker);
    }
    if has_flatpak_apps(cfg) {
        sh_lines.extend(flatpak_shell_lines(cfg, ""));
    }
    if cfg.cloud_init {
        let yaml = cloud_init_yaml(cfg);
        let marker = crate::security::heredoc_marker("CIEOF", &yaml);
        sh_lines.push("mkdir -p /etc/cloud/cloud.cfg.d".to_string());
        sh_lines.push(format!(
            "cat > /etc/cloud/cloud.cfg.d/80-distroforge.cfg <<'{marker}'"
        ));
        sh_lines.push(yaml);
        sh_lines.push(marker);
    }
    if wallpaper_applies(cfg) && dconf_schema_for(&cfg.desktop).is_some() {
        sh_lines.push("dconf update || true".to_string());
    }
    if net.is_some() {
        sh_lines.push(
            "chmod 600 /etc/NetworkManager/system-connections/distroforge.nmconnection".to_string(),
        );
    }
    let config_sh = format!("{}\n", sh_lines.join("\n"));

    let mut wallpaper_files: Vec<(String, String)> = Vec::new();
    if wallpaper_applies(cfg) {
        if let Some(wf) = wallpaper_file_name(cfg) {
            wallpaper_files.push((
                format!("kiwi/root/usr/share/backgrounds/{wf}"),
                wallpaper_data_url(cfg).unwrap_or_default().to_string(),
            ));
            for (p, c) in dconf_db_files(cfg) {
                wallpaper_files.push((format!("kiwi/root{p}"), c));
            }
            if cfg.desktop == "xfce" {
                wallpaper_files.push((
                    "kiwi/root/etc/xdg/xfce4/xfconf/xfce-perchannel-xml/xfce4-desktop.xml"
                        .to_string(),
                    xfconf_wallpaper_xml(cfg),
                ));
            }
        }
    }

    let build_sh = "#!/usr/bin/env bash\nset -euo pipefail\n# DistroForge - build local (Linux o WSL2, requiere Podman)\nDIR=\"$(cd \"$(dirname \"$0\")\" && pwd)\"\npodman run --rm --privileged -v \"$DIR\":/project -v \"$DIR/out\":/out \\\n  registry.opensuse.org/opensuse/tumbleweed:latest bash -ec '\n    zypper --gpg-auto-import-keys --non-interactive install \\\n      python3-kiwi kiwi-systemdeps-core kiwi-systemdeps-iso-media kiwi-systemdeps-disk-images\n    kiwi-ng system build --description /project/kiwi --target-dir /out\n  '\necho \"Imagen lista en ./out\"\n".to_string();

    let mut files: Vec<(String, String)> = vec![
        ("kiwi/config.xml".to_string(), config_xml),
        ("kiwi/config.sh".to_string(), config_sh),
    ];
    for f in &cfg.extra_files {
        files.push((format!("kiwi/root{}", f.path), f.content.clone()));
    }
    for (p, c) in autologin_files(cfg) {
        files.push((format!("kiwi/root{p}"), c));
    }
    for (p, c) in autostart_files(cfg) {
        files.push((format!("kiwi/root{p}"), c));
    }
    for (p, c) in logo_files(cfg) {
        files.push((format!("kiwi/root{p}"), c));
    }
    for (p, c) in eula_files(cfg) {
        files.push((format!("kiwi/root{p}"), c));
    }
    files.extend(net_file);
    if cfg.unattended {
        files.push((
            "kiwi/autoinst.xml".to_string(),
            format!(
                "<?xml version=\"1.0\"?>\n<profile xmlns=\"http://www.suse.com/1.0/yast2ns\" xmlns:config=\"http://www.suse.com/1.0/configns\">\n  <general><mode><confirm config:type=\"boolean\">false</confirm></mode></general>\n  <language><language>{l}</language></language>\n  <keyboard><keymap>{k}</keymap></keyboard>\n  <timezone><hwclock>UTC</hwclock><timezone>{tz}</timezone></timezone>\n  <networking><keep_network config:type=\"boolean\">true</keep_network></networking>\n  <users config:type=\"list\">\n    <user><username>{u}</username><password>{p}</password><encrypt_password config:type=\"boolean\">false</encrypt_password><home>/home/{u}</home></user>\n  </users>\n</profile>\n",
                l = esc(&sys.locale),
                k = esc(&sys.keyboard),
                tz = esc(&sys.timezone),
                u = esc(&sys.username),
                p = esc(&sys.password)
            ),
        ));
    }
    files.extend(wallpaper_files);
    files.push(("build.sh".to_string(), build_sh));
    files
}
