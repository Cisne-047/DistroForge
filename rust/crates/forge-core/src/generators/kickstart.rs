//! Generador kickstart/anaconda (Fedora). Port de
//! `src/generators/kickstart.ts`.

use crate::catalog::{desktop_packages, effective_repos};
use crate::generators::accounts::{
    autologin_files, autostart_files, eula_files, service_shell_lines, target_shell_line,
};
use crate::generators::cloudinit::{cloud_init_yaml, CLOUD_INIT_PACKAGE};
use crate::generators::flatpak::{flatpak_packages, flatpak_shell_lines};
use crate::generators::livebuild::sanitize_alias;
use crate::generators::network::{dotted_mask, static_network};
use crate::generators::shell::dirname_of;
use crate::types::{js_num, ArchId, DistroConfig, ExtraFile};

/// Bloque %post que vuelca cada fichero extra con heredoc (rutas absolutas).
pub fn extra_file_post_lines(extra_files: &[ExtraFile]) -> Vec<String> {
    extra_files
        .iter()
        .enumerate()
        .flat_map(|(i, f)| {
            vec![
                format!("mkdir -p {}", dirname_of(&f.path)),
                format!("cat > {} <<'DFEOF{i}'", f.path),
                f.content.clone(),
                format!("DFEOF{i}"),
            ]
        })
        .collect()
}

/// Ficheros del proyecto kickstart, en el orden de construcción del TS.
pub fn kickstart_files(cfg: &DistroConfig) -> Vec<(String, String)> {
    let sys = &cfg.system;
    let repos = effective_repos(cfg);

    let repo_lines = repos
        .iter()
        .map(|r| {
            format!(
                "repo --name=\"{}\" --baseurl=\"{}\" --nogpgcheck",
                sanitize_alias(&r.alias, 0),
                r.url
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let mut post_lines: Vec<String> = vec![
        "echo \"DistroForge\" > /etc/distroforge-release".to_string(),
    ];
    // instalación sin verificación (anaconda no puede preguntar), pero el
    // sistema final queda confiando en las keys para futuras actualizaciones
    for r in &repos {
        if !r.gpg_key.is_empty() {
            post_lines.push(format!("rpm --import \"{}\"", r.gpg_key));
        }
    }
    if !cfg.motd.is_empty() {
        post_lines.push("cat > /etc/motd <<'MOTDEOF'".to_string());
        post_lines.push(cfg.motd.clone());
        post_lines.push("MOTDEOF".to_string());
    }
    if cfg.cloud_init {
        post_lines.push("mkdir -p /etc/cloud/cloud.cfg.d".to_string());
        post_lines.push("cat > /etc/cloud/cloud.cfg.d/80-distroforge.cfg <<'CIEOF'".to_string());
        post_lines.push(cloud_init_yaml(cfg));
        post_lines.push("CIEOF".to_string());
    }
    post_lines.extend(extra_file_post_lines(&cfg.extra_files));
    let autostart_as_files: Vec<ExtraFile> = autostart_files(cfg)
        .into_iter()
        .map(|(path, content)| ExtraFile { path, content })
        .collect();
    post_lines.extend(extra_file_post_lines(&autostart_as_files));
    let eula_as_files: Vec<ExtraFile> = eula_files(cfg)
        .into_iter()
        .map(|(path, content)| ExtraFile { path, content })
        .collect();
    post_lines.extend(extra_file_post_lines(&eula_as_files));
    post_lines.push(target_shell_line(cfg));
    for (path, content) in autologin_files(cfg) {
        post_lines.extend(extra_file_post_lines(&[ExtraFile { path, content }]));
    }
    post_lines.extend(service_shell_lines(cfg));
    post_lines.extend(flatpak_shell_lines(cfg, ""));
    let post_block = format!("%post\n{}\n%end\n", post_lines.join("\n"));

    let net = static_network(cfg);
    let net_line = match net {
        Some(n) => format!(
            "network --bootproto=static --ip={} --netmask={} --gateway={} --nameserver={} --hostname={}",
            n.ip,
            dotted_mask(&n.prefix),
            n.gateway,
            n.dns.join(","),
            sys.hostname
        ),
        None => format!("network --bootproto=dhcp --hostname={}", sys.hostname),
    };
    let ks_arch = if cfg.arch == ArchId::Aarch64 {
        "aarch64"
    } else {
        "x86_64"
    };
    let grub_append = cfg
        .grub
        .extra_params
        .replace(['"', '`', '$'], "")
        .trim()
        .to_string();

    let user_lines = cfg
        .users
        .iter()
        .map(|u| {
            format!(
                "user --name={} --password=\"{}\" --plaintext{}",
                u.name,
                u.password,
                if u.admin { " --groups=wheel" } else { "" }
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    let firewall_ports = cfg
        .firewall_ports
        .iter()
        .map(|x| {
            if x.contains('/') {
                format!(" --port={x}")
            } else {
                format!(" --service={x}")
            }
        })
        .collect::<Vec<_>>()
        .join("");

    let mut pkg_items: Vec<String> =
        desktop_packages(cfg.base, &cfg.desktop).iter().map(|s| s.to_string()).collect();
    pkg_items.extend(flatpak_packages(cfg));
    pkg_items.extend(cfg.packages.iter().cloned());
    if cfg.cloud_init {
        pkg_items.push(CLOUD_INIT_PACKAGE.to_string());
    }

    let ks = format!(
        "# DistroForge - kickstart Fedora\nlang {}\nkeyboard --vckeymap={} --xlayouts='{}'\ntimezone {} --utc\n{}\nrootpw --lock\nuser --name={} --password=\"{}\" --plaintext --groups=wheel\n{}\nfirstboot --disable\nfirewall {}\nbootloader --location=mbr --timeout={}{}\nzerombr\nclearpart --all --initlabel\nautopart\nreboot\nurl --mirrorlist=\"https://mirrors.fedoraproject.org/mirrorlist?repo=fedora-$releasever&arch={ks_arch}\"\n{repo_lines}\n%packages\n@core\n{}\n%end\n\n{post_block}",
        sys.locale,
        sys.keyboard,
        sys.keyboard,
        sys.timezone,
        net_line,
        sys.username,
        sys.password,
        user_lines,
        if cfg.firewall {
            format!("--enabled{firewall_ports}")
        } else {
            "--disabled".to_string()
        },
        js_num(cfg.grub.timeout),
        if grub_append.is_empty() {
            String::new()
        } else {
            format!(" --append=\"{grub_append}\"")
        },
        pkg_items.join("\n"),
    );

    let build_sh = format!(
        "#!/usr/bin/env bash\nset -euo pipefail\n# DistroForge - build local (Linux o WSL2, requiere Podman)\nDIR=\"$(cd \"$(dirname \"$0\")\" && pwd)\"\npodman run --rm --privileged -v \"$DIR\":/project -v \"$DIR/out\":/out fedora:latest bash -ec '\n  dnf install -y livemedia-creator\n  livemedia-creator --no-virt --make-iso --ks /project/fedora/ks.cfg --releasever 44 --iso-only --iso-name {}.iso --resultdir /out\n'\necho \"Imagen lista en ./out/{}.iso\"\n",
        cfg.name, cfg.name
    );

    vec![("fedora/ks.cfg".to_string(), ks), ("build.sh".to_string(), build_sh)]
}
