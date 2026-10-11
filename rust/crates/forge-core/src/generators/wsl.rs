//! Rootfs WSL (`wsl --import`). Port de `src/generators/wsl.ts`.

use crate::catalog::{base_by_id, deb_arch, desktop_packages, effective_repos};
use crate::generators::flatpak::{flatpak_packages, flatpak_shell_lines};
use crate::generators::livebuild::sanitize_alias;
use crate::generators::shell::{dirname_of, sq};
use crate::types::{DistroConfig, EngineId};

fn wsl_conf(cfg: &DistroConfig) -> String {
    format!(
        "[automount]\nenabled = true\n\n[user]\ndefault = {}\n\n[boot]\nsystemd = true",
        cfg.system.username
    )
}

/// Ficheros del proyecto WSL, en el orden de construcción del TS.
pub fn wsl_files(cfg: &DistroConfig) -> Vec<(String, String)> {
    let base = base_by_id(cfg.base);
    let sys = &cfg.system;
    let name = &cfg.name;
    let mut pkgs: Vec<String> =
        desktop_packages(cfg.base, &cfg.desktop).iter().map(|s| s.to_string()).collect();
    pkgs.extend(flatpak_packages(cfg));
    pkgs.extend(cfg.packages.iter().cloned());
    let repos = effective_repos(cfg);
    let engine = base.engine;

    let extra_files_block = cfg
        .extra_files
        .iter()
        .map(|f| {
            let dir = dirname_of(&f.path);
            format!("mkdir -p /rootfs{dir}\ncat > /rootfs{} <<'DFEOF'\n{}\nDFEOF", f.path, f.content)
        })
        .collect::<Vec<_>>()
        .join("\n");
    let flatpak_block = flatpak_shell_lines(cfg, "chroot /rootfs ").join("\n");
    let wsl_user_lines = cfg
        .users
        .iter()
        .map(|u| {
            format!(
                "chroot /rootfs id \"{}\" >/dev/null 2>&1 || chroot /rootfs useradd -m -s /bin/bash{} \"{}\"\necho {} | chroot /rootfs chpasswd",
                u.name,
                if u.admin { " -G wheel" } else { "" },
                u.name,
                sq(&format!("{}:{}", u.name, u.password))
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let wsl_service_lines = cfg
        .services
        .iter()
        .filter(|s| {
            !s.unit.is_empty()
                && s.unit
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'@' | b'.' | b'_' | b'-'))
        })
        .map(|s| {
            format!(
                "chroot /rootfs systemctl {} \"{}\"",
                if s.enabled { "enable" } else { "disable" },
                s.unit
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    let pre_conf = [extra_files_block, flatpak_block, wsl_user_lines, wsl_service_lines]
        .into_iter()
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n");

    let conf_lines = format!(
        "cat > /rootfs/etc/wsl.conf <<'WSLEOF'\n{}\nWSLEOF\necho \"{}\" > /rootfs/etc/hostname\nln -sf /usr/share/zoneinfo/{} /rootfs/etc/localtime",
        wsl_conf(cfg),
        sys.hostname,
        sys.timezone
    );

    let script = match engine {
        EngineId::Livebuild => {
            let distro = base.distribution.unwrap_or("bookworm");
            let mirror = if base.mirror.is_empty() {
                "http://deb.debian.org/debian"
            } else {
                base.mirror
            };
            let need_key = repos.iter().any(|r| !r.gpg_key.is_empty());
            let ensure_bootstrap = if need_key {
                "apt-get update\napt-get install -y debootstrap wget ca-certificates"
            } else {
                "command -v debootstrap >/dev/null || { apt-get update; apt-get install -y debootstrap; }"
            };
            let repo_lines = repos
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    let alias = sanitize_alias(&r.alias, i);
                    let signed = if !r.gpg_key.is_empty() {
                        format!("[signed-by=/usr/share/keyrings/{alias}.asc]")
                    } else {
                        "[trusted=yes]".to_string()
                    };
                    let src = if !r.suite.is_empty() {
                        format!("deb {signed} {} {}", r.url, r.suite)
                    } else {
                        format!("deb {signed} {} ./", r.url)
                    };
                    let key_line = if !r.gpg_key.is_empty() {
                        format!("mkdir -p /rootfs/usr/share/keyrings\nwget -qO /rootfs/usr/share/keyrings/{alias}.asc {}\n", r.gpg_key)
                    } else {
                        String::new()
                    };
                    format!("{key_line}cat > /rootfs/etc/apt/sources.list.d/distroforge-{alias}.list <<'EOF'\n{src}\nEOF")
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "#!/usr/bin/env bash\nset -euo pipefail\n# DistroForge - rootfs WSL para {} (debootstrap)\n{}\nrm -rf /rootfs && mkdir -p /rootfs\ndebootstrap --variant=minbase --arch={} {} /rootfs {}\n{}\nchroot /rootfs apt-get update\nchroot /rootfs env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends sudo {}\nchroot /rootfs useradd -m -s /bin/bash -G sudo {}\necho {} | chroot /rootfs chpasswd\n{pre}{conf}\nmkdir -p out\ntar -C /rootfs -cpf out/{name}-wsl.tar .\necho \"Rootfs WSL listo en ./out/{name}-wsl.tar\"\n",
                base.label,
                ensure_bootstrap,
                deb_arch(cfg.arch),
                distro,
                mirror,
                repo_lines,
                pkgs.join(" "),
                sys.username,
                sq(&format!("{}:{}", sys.username, sys.password)),
                pre = if pre_conf.is_empty() { String::new() } else { format!("{pre_conf}\n") },
                conf = conf_lines,
                name = name,
            )
        }
        EngineId::Mkarchiso => {
            // Ojo: el TS usa el alias SIN sanear aquí (a diferencia del
            // Dockerfile); se replica tal cual.
            let repo_lines = repos
                .iter()
                .map(|r| {
                    format!(
                        "printf '[{}]\\nSigLevel = Optional TrustAll\\nServer = {}\\n' >> /etc/pacman.conf",
                        r.alias, r.url
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "#!/usr/bin/env bash\nset -euo pipefail\n# DistroForge - rootfs WSL para {} (pacstrap)\npacman -Sy --noconfirm arch-install-scripts\nrm -rf /rootfs && mkdir -p /rootfs\n{}\npacstrap -c /rootfs base sudo {}\narch-chroot /rootfs useradd -m -s /bin/bash -G wheel {}\narch-echo {} | chroot /rootfs chpasswd\n{pre}{conf}\nmkdir -p out\ntar -C /rootfs -cpf out/{name}-wsl.tar .\necho \"Rootfs WSL listo en ./out/{name}-wsl.tar\"\n",
                base.label,
                repo_lines,
                pkgs.join(" "),
                sys.username,
                sq(&format!("{}:{}", sys.username, sys.password)),
                pre = if pre_conf.is_empty() { String::new() } else { format!("{pre_conf}\n") },
                conf = conf_lines,
                name = name,
            )
        }
        EngineId::Kiwi => {
            let repo_lines = repos
                .iter()
                .map(|r| format!("zypper --installroot /rootfs ar -f {} {}", r.url, r.alias))
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "#!/usr/bin/env bash\nset -euo pipefail\n# DistroForge - rootfs WSL para {} (zypper installroot)\nrm -rf /rootfs && mkdir -p /rootfs\nzypper --installroot /rootfs ar -f {} distroforge-base\n{}\nzypper --installroot /rootfs --no-gpg-checks -n install filesystem zypper sudo {}\nchroot /rootfs useradd -m -s /bin/bash -G users {}\necho {} | chroot /rootfs chpasswd\n{pre}{conf}\nmkdir -p out\ntar -C /rootfs -cpf out/{name}-wsl.tar .\necho \"Rootfs WSL listo en ./out/{name}-wsl.tar\"\n",
                base.label,
                base.mirror,
                repo_lines,
                pkgs.join(" "),
                sys.username,
                sq(&format!("{}:{}", sys.username, sys.password)),
                pre = if pre_conf.is_empty() { String::new() } else { format!("{pre_conf}\n") },
                conf = conf_lines,
                name = name,
            )
        }
        _ => {
            let repo_lines = repos
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    format!(
                        "mkdir -p /rootfs/etc/yum.repos.d\ncat > /rootfs/etc/yum.repos.d/distroforge-{i}.repo <<'EOF'\n[distroforge-{i}]\nname=distroforge-{i}\nbaseurl={}\nenabled=1\ngpgcheck=0\nEOF",
                        r.url
                    )
                })
                .collect::<Vec<_>>()
                .join("\n");
            format!(
                "#!/usr/bin/env bash\nset -euo pipefail\n# DistroForge - rootfs WSL para {} (dnf installroot)\nrm -rf /rootfs && mkdir -p /rootfs\n{repo}dnf install -y --installroot=/rootfs --releasever=44 --setopt=install_weak_deps=False --nodocs @core sudo {}\nchroot /rootfs useradd -m -s /bin/bash -G wheel {}\necho {} | chroot /rootfs chpasswd\n{pre}{conf}\nmkdir -p out\ntar -C /rootfs -cpf out/{name}-wsl.tar .\necho \"Rootfs WSL listo en ./out/{name}-wsl.tar\"\n",
                base.label,
                pkgs.join(" "),
                sys.username,
                sq(&format!("{}:{}", sys.username, sys.password)),
                repo = if repo_lines.is_empty() { String::new() } else { format!("{repo_lines}\n") },
                pre = if pre_conf.is_empty() { String::new() } else { format!("{pre_conf}\n") },
                conf = conf_lines,
                name = name,
            )
        }
    };

    let container = match engine {
        EngineId::Livebuild => base.container.unwrap_or("debian:bookworm"),
        EngineId::Mkarchiso => "archlinux:base-devel",
        EngineId::Kiwi => "opensuse/tumbleweed",
        _ => "fedora:latest",
    };

    let build_sh = format!(
        "#!/usr/bin/env bash\nset -euo pipefail\n# DistroForge - build rootfs WSL (Linux o WSL2, requiere docker o podman)\nDIR=\"$(cd \"$(dirname \"$0\")\" && pwd)\"\npodman run --rm --privileged -v \"$DIR\":/project -w /project {container} bash wsl/rootfs.sh\necho \"Importar en Windows: wsl --import {name} C:\\WSL\\{name} <ruta>\\out\\{name}-wsl.tar\"\n",
        container = container,
        name = name
    );

    vec![
        ("wsl/rootfs.sh".to_string(), script),
        ("build.sh".to_string(), build_sh),
    ]
}
