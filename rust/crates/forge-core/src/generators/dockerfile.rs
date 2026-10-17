//! Salida OCI: Dockerfile por familia. Port de `src/generators/dockerfile.ts`.
//!
//! Nota de port: en el TS hay template literals con `\` + salto de línea
//! (continuación de línea de JS), que "cosan" varias líneas en una sola; otras
//! usan `\\` + salto real (backslash de Dockerfile). Aquí se replica el
//! resultado cocinado exacto.

use crate::catalog::{base_by_id, desktop_packages, effective_repos};
use crate::generators::flatpak::{flatpak_packages, has_flatpak_apps};
use crate::generators::livebuild::sanitize_alias;
use crate::generators::shell::{dirname_of, dq, sq};
use crate::types::{BaseId, DistroConfig, EngineId};

/// Imagen base OCI para cada distro (salida "oci"). `None` = no soportada
/// (Gentoo/Solus).
fn base_image(base: BaseId) -> Option<&'static str> {
    Some(match base {
        BaseId::Debian => "debian:bookworm",
        BaseId::Ubuntu => "ubuntu:24.04",
        BaseId::Mint => "ubuntu:24.04",
        BaseId::Elementary => "ubuntu:24.04",
        BaseId::Pop => "ubuntu:22.04",
        BaseId::Zorin => "ubuntu:24.04",
        BaseId::Fedora => "fedora:latest",
        BaseId::OpensuseTumbleweed => "opensuse/tumbleweed",
        BaseId::OpensuseLeap => "opensuse/leap:15.6",
        BaseId::Arch | BaseId::Manjaro | BaseId::Cachyos => "archlinux:latest",
        BaseId::Gentoo | BaseId::Solus => return None,
    })
}

/// Grupo de admin que existe de verdad en cada familia (sudo no existe fuera
/// de Debian).
fn admin_group_of(engine: EngineId) -> &'static str {
    match engine {
        EngineId::Livebuild => "sudo",
        EngineId::Kiwi => "users",
        // mkarchiso y kickstart
        _ => "wheel",
    }
}

/// ¿Se puede generar OCI para esta config?
pub fn oci_supported(cfg: &DistroConfig) -> bool {
    base_image(cfg.base).is_some()
}

/// Usuarios extra como bloque Dockerfile: cada línea es
/// `    id ... ;     echo sq | chpasswd; \`, unidas con salto real, con el
/// `; \` final recortado (`.replace(/; \\$/, "")`). Los grupos usan
/// `adminGroupOf(engine)`.
fn extra_users_block(cfg: &DistroConfig, engine: EngineId) -> String {
    let lines: Vec<String> = cfg
        .users
        .iter()
        .map(|u| {
            let group = if u.admin {
                format!(" -G {}", admin_group_of(engine))
            } else {
                String::new()
            };
            format!(
                "    id \"{}\" >/dev/null 2>&1 || useradd -m -s /bin/bash{group} \"{}\"; \\\n    echo {} | chpasswd; \\",
                dq(&u.name),
                dq(&u.name),
                sq(&format!("{}:{}", u.name, u.password))
            )
        })
        .collect();
    let mut joined = lines.join("\n");
    if joined.ends_with("; \\") {
        joined.truncate(joined.len() - 3);
    }
    joined
}

/// Cola del RUN tras el último comando de instalación: alta del usuario
/// principal + chpasswd + usuarios extra. En el TS estas líneas acaban en
/// `\\` + salto real (backslash de Dockerfile) y los usuarios extra van tras
/// "; " (el TS consume ese salto con una continuación `\` de template
/// literal).
fn run_tail(cfg: &DistroConfig, engine: EngineId) -> String {
    let extra = extra_users_block(cfg, engine);
    let tail = if extra.is_empty() {
        String::new()
    } else {
        format!("; {extra}")
    };
    let sys = &cfg.system;
    let cred = sq(&format!("{}:{}", sys.username, sys.password));
    let body = match engine {
        EngineId::Livebuild => format!(
            "    useradd -m -s /bin/bash -G sudo {}; \\\n    echo {} | chpasswd",
            sys.username, cred
        ),
        EngineId::Mkarchiso => format!(
            "    useradd -m -s /bin/bash -G wheel {}; \\\n    echo \"%wheel ALL=(ALL:ALL) ALL\" > /etc/sudoers.d/10-distroforge; \\\n    echo {} | chpasswd",
            sys.username, cred
        ),
        EngineId::Kiwi => format!(
            "    useradd -m -s /bin/bash -G users {}; \\\n    echo \"{} ALL=(ALL) ALL\" > /etc/sudoers.d/50-distroforge; \\\n    echo {} | chpasswd",
            sys.username, sys.username, cred
        ),
        _ => format!(
            "    dnf clean all; \\\n    useradd -m -s /bin/bash -G wheel {}; \\\n    echo {} | chpasswd",
            sys.username, cred
        ),
    };
    format!("{body}{tail}")
}

/// Ficheros del proyecto OCI, en el orden de construcción del TS.
pub fn dockerfile_files(cfg: &DistroConfig) -> Vec<(String, String)> {
    let base = base_by_id(cfg.base);
    let sys = &cfg.system;
    let Some(image) = base_image(cfg.base) else {
        return vec![];
    };
    let mut pkgs: Vec<String> = desktop_packages(cfg.base, &cfg.desktop)
        .iter()
        .map(|s| s.to_string())
        .collect();
    pkgs.extend(flatpak_packages(cfg));
    pkgs.extend(cfg.packages.iter().cloned());
    let repos = effective_repos(cfg);
    let engine = base.engine;
    let flatpak_run = if has_flatpak_apps(cfg) {
        format!(
            "    flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo; \\\n    flatpak install -y --noninteractive flathub {}; \\\n",
            cfg.flatpak_apps.join(" ")
        )
    } else {
        String::new()
    };

    // Dedupe ordenado de directorios ([...new Set(...)] conserva inserción).
    let mut extra_dirs: Vec<String> = Vec::new();
    for f in &cfg.extra_files {
        let d = dirname_of(&f.path);
        if !extra_dirs.contains(&d) {
            extra_dirs.push(d);
        }
    }

    let run = match engine {
        EngineId::Livebuild => {
            let key_lines: Vec<String> = repos
                .iter()
                .enumerate()
                .filter(|(_, r)| !r.gpg_key.is_empty())
                .map(|(i, r)| {
                    format!(
                        "    mkdir -p /usr/share/keyrings; \\\n    wget -qO /usr/share/keyrings/{}.asc {}; \\",
                        sanitize_alias(&r.alias, i),
                        r.gpg_key
                    )
                })
                .collect();
            let key_lines = key_lines.join("\n");
            let repo_lines: Vec<String> = repos
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
                    format!(
                        "    echo {} > /etc/apt/sources.list.d/distroforge-{alias}.list; \\",
                        sq(&src)
                    )
                })
                .collect();
            let repo_lines = repo_lines.join("\n");
            let body = format!(
                "RUN set -eu; \\\n    apt-get update; \\\n    apt-get install -y --no-install-recommends wget ca-certificates; \\\n{key}{repo}    apt-get update; \\\n    apt-get install -y --no-install-recommends {pkgs}; \\\n    apt-get clean; \\\n    rm -rf /var/lib/apt/lists/*; \\\n{flatpak}",
                key = if key_lines.is_empty() { String::new() } else { format!("{key_lines}\n") },
                repo = if repo_lines.is_empty() { String::new() } else { format!("{repo_lines}\n") },
                pkgs = [vec!["sudo".to_string()], pkgs.clone()].concat().join(" "),
                flatpak = flatpak_run,
            );
            format!("{body}{}", run_tail(cfg, engine))
        }
        EngineId::Mkarchiso => {
            let repo_lines: Vec<String> = repos
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    format!(
                        "    printf '[%s]\\nSigLevel = Optional TrustAll\\nServer = %s\\n' \"{}\" \"{}\" >> /etc/pacman.conf; \\",
                        sanitize_alias(&r.alias, i),
                        r.url
                    )
                })
                .collect();
            let repo_lines = repo_lines.join("\n");
            // Manjaro OCI parte de la imagen Arch pero con sus propios repos
            let base_repo = if cfg.base == BaseId::Manjaro && !base.mirror.is_empty() {
                format!("    printf '[manjaro]\\nSigLevel = Optional TrustAll\\nServer = %s\\n' \"{}\" >> /etc/pacman.conf; \\\n", base.mirror)
            } else {
                String::new()
            };
            let body = format!(
                "RUN set -eu; \\\n{base_repo}{repo}    pacman -Syu --noconfirm {pkgs}; \\\n    pacman -Scc --noconfirm; \\\n{flatpak}",
                base_repo = base_repo,
                repo = if repo_lines.is_empty() { String::new() } else { format!("{repo_lines}\n") },
                pkgs = [vec!["sudo".to_string()], pkgs.clone()].concat().join(" "),
                flatpak = flatpak_run,
            );
            format!("{body}{}", run_tail(cfg, engine))
        }
        EngineId::Kiwi => {
            let repo_lines: Vec<String> = repos
                .iter()
                .map(|r| {
                    format!(
                        "    zypper --non-interactive ar --gpg-auto-import-keys \"{}\" \"{}\"; \\",
                        r.url, r.alias
                    )
                })
                .collect();
            let repo_lines = repo_lines.join("\n");
            let body = format!(
                "RUN set -eu; \\\n{repo}    zypper --gpg-auto-import-keys --non-interactive install {pkgs}; \\\n{flatpak}",
                repo = if repo_lines.is_empty() { String::new() } else { format!("{repo_lines}\n") },
                pkgs = pkgs.join(" "),
                flatpak = flatpak_run,
            );
            format!("{body}{}", run_tail(cfg, engine))
        }
        _ => {
            let repo_lines: Vec<String> = repos
                .iter()
                .map(|r| format!("    dnf config-manager --add-repo={}; \\", r.url))
                .collect();
            let repo_lines = repo_lines.join("\n");
            let body = format!(
                "RUN set -eu; \\\n    dnf -y install dnf-plugins-core; \\\n{repo}    dnf install -y {pkgs}; \\\n{flatpak}",
                repo = if repo_lines.is_empty() { String::new() } else { format!("{repo_lines}\n") },
                pkgs = pkgs.join(" "),
                flatpak = flatpak_run,
            );
            format!("{body}{}", run_tail(cfg, engine))
        }
    };

    let env_line = if engine == EngineId::Livebuild {
        "ENV DEBIAN_FRONTEND=noninteractive\n"
    } else {
        ""
    };
    let extra_block = if !extra_dirs.is_empty() {
        format!("RUN mkdir -p {}\n", extra_dirs.join(" "))
    } else {
        String::new()
    };
    let copy_lines = cfg
        .extra_files
        .iter()
        .enumerate()
        .map(|(i, f)| format!("COPY files/extra-{i} {}", f.path))
        .collect::<Vec<_>>()
        .join("\n");

    let dockerfile = format!(
        "FROM {image}\nLABEL org.opencontainers.image.title=\"{}\"\nLABEL org.opencontainers.image.description=\"Distro personalizada generada con DistroForge\"\n{env}{run}\n{extra}{copy}\nUSER {}\nWORKDIR /home/{}\nCMD [\"/bin/bash\"]\n",
        cfg.name,
        sys.username,
        sys.username,
        env = env_line,
        run = run,
        extra = extra_block,
        copy = copy_lines
    );

    let build_sh = format!(
        "#!/usr/bin/env bash\nset -euo pipefail\n# DistroForge - build imagen OCI (docker o podman)\nDIR=\"$(cd \"$(dirname \"$0\")\" && pwd)\"\nCMD=\"$(command -v docker || command -v podman)\"\nmkdir -p out\n\"$CMD\" build -t {}:1.0.0 \"$DIR\"\n\"$CMD\" save -o \"out/{}.tar\" \"{}:1.0.0\"\necho \"Imagen OCI en ./out/{}.tar (cargar con: docker load -i ...)\"\n",
        cfg.name, cfg.name, cfg.name, cfg.name
    );

    let mut files = vec![
        ("Dockerfile".to_string(), dockerfile),
        ("build.sh".to_string(), build_sh),
    ];
    for (i, f) in cfg.extra_files.iter().enumerate() {
        files.push((format!("files/extra-{i}"), f.content.clone()));
    }
    files
}
