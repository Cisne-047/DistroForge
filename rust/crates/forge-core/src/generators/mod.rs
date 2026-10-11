//! Punto de entrada de los generadores. Port de `src/generators/index.ts`
//! (`downloadZip` no se porta: es código de navegador con JSZip).

pub mod accounts;
pub mod archiso;
pub mod branding;
pub mod cloudinit;
pub mod customiso;
pub mod dockerfile;
pub mod flatpak;
pub mod kickstart;
pub mod kiwi;
pub mod livebuild;
pub mod manual;
pub mod network;
pub mod shell;
pub mod wsl;

use std::collections::BTreeMap;

use crate::catalog::{archs_for_engine, base_by_id, effective_repos};
use crate::generators::network::is_ipv4;
use crate::generators::customiso::custom_iso_files;
use crate::generators::dockerfile::{dockerfile_files, oci_supported};
use crate::types::{ArchId, DistroConfig, EngineId, NetMode, OutputFormat};

/// Mapa ruta → contenido del proyecto generado.
pub type FileMap = BTreeMap<String, String>;

/// Proyecto completo para una config. Port de `generateProject`.
///
/// El TS mezcla objetos con spread (el último spread gana en colisiones); aquí
/// se inserta en el mismo orden en un `BTreeMap`, donde una inserción posterior
/// sustituye a la anterior — misma semántica.
pub fn generate_project(cfg: &DistroConfig) -> FileMap {
    if cfg.advanced {
        return custom_iso_files(cfg).into_iter().collect();
    }
    let engine = base_by_id(cfg.base).engine;
    let shared = || -> Vec<(String, String)> {
        let json =
            serde_json::to_string_pretty(cfg).unwrap_or_else(|_| "null".to_string());
        vec![
            (".github/workflows/build.yml".to_string(), workflow_yaml(cfg)),
            ("README.md".to_string(), readme_md(cfg)),
            // round-trip: el proyecto generado se reimporta en la app tal cual
            ("distroforge.json".to_string(), format!("{json}\n")),
        ]
    };
    let mut engine_files = if cfg.output == OutputFormat::Oci
        && engine != EngineId::Manual
        && oci_supported(cfg)
    {
        dockerfile_files(cfg)
    } else if cfg.output == OutputFormat::Wsl && engine != EngineId::Manual {
        wsl::wsl_files(cfg)
    } else {
        match engine {
            EngineId::Kiwi => kiwi::kiwi_files(cfg),
            EngineId::Livebuild => livebuild::livebuild_files(cfg),
            EngineId::Mkarchiso => archiso::archiso_files(cfg),
            EngineId::Kickstart => kickstart::kickstart_files(cfg),
            EngineId::Manual => manual::manual_files(cfg),
        }
    };
    engine_files.extend(shared());
    engine_files.into_iter().collect()
}

/// Nombre de unidad válido (`/^[A-Za-z0-9@._-]+$/`).
fn valid_unit(unit: &str) -> bool {
    !unit.is_empty()
        && unit
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, b'@' | b'.' | b'_' | b'-'))
}

/// ¿Nombre de distro válido? (`/^[a-z0-9][a-z0-9-]{1,31}$/`)
fn valid_name(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c.is_ascii_digit() => {}
        _ => return false,
    }
    let rest = chars.count();
    (1..=31).contains(&rest)
        && s.chars()
            .skip(1)
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '-')
}

/// ¿Hostname válido? (`/^[a-zA-Z0-9][a-zA-Z0-9.-]{1,31}$/`)
fn valid_hostname(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphanumeric() => {}
        _ => return false,
    }
    let rest = chars.count();
    (1..=31).contains(&rest)
        && s.chars()
            .skip(1)
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-')
}

/// ¿Nombre de usuario válido? (`/^[a-z_][a-z0-9_-]{0,31}$/`)
fn valid_username(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_lowercase() || c == '_' => {}
        _ => return false,
    }
    let rest = chars.count();
    rest <= 31
        && s.chars()
            .skip(1)
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
}

/// ¿Id de app Flatpak válido? (`/^[A-Za-z0-9][A-Za-z0-9._-]*$/`)
fn valid_flatpak_id(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphanumeric() => {}
        _ => return false,
    }
    chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Número de prefijo CIDR estilo `Number(prefix)` (NaN/0 → treated como inválido
/// o default según el punto de uso); aquí solo se usa para validar el rango
/// 8-32, replicando `Number(n.prefix) >= 8 && Number(n.prefix) <= 32`.
fn prefix_in_range(prefix: &str) -> bool {
    match prefix.trim().parse::<f64>() {
        Ok(v) => v >= 8.0 && v <= 32.0,
        Err(_) => false,
    }
}

/// ¿URL http(s)? (`/^https?:\/\//`)
fn is_http_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// ¿URL http(s) con algo tras el esquema? (`/^https?:\/\/.+/i`)
fn is_http_url_nonempty(s: &str) -> bool {
    is_http_url(s) && s.len() > "http://".len()
}

/// ¿Termina en .iso con ?/# o fin? (`/\.iso([?#]|$)/i`)
fn ends_with_iso(s: &str) -> bool {
    let lower = s.to_lowercase();
    for (i, _) in lower.match_indices(".iso") {
        let rest = &lower[i + 4..];
        if rest.is_empty() || rest.starts_with('?') || rest.starts_with('#') {
            return true;
        }
    }
    false
}

/// ¿Data URL de imagen soportada? (`/^data:image\/(png|jpeg|webp);base64,/`)
fn is_wallpaper_data_url(s: &str) -> bool {
    ["png", "jpeg", "webp"]
        .iter()
        .any(|e| s.starts_with(&format!("data:image/{e};base64,")))
}

/// Validaciones de la config. Devuelve la lista de mensajes (vacía = ok). Port
/// de `validate`. Las ramas `!Array.isArray(...)` del TS son imposibles con
/// tipos concretos y no se portan.
pub fn validate(cfg: &DistroConfig) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let engine = base_by_id(cfg.base).engine;

    if !valid_name(&cfg.name) {
        out.push("Nombre: minúsculas, números y guiones (2-32 caracteres).".to_string());
    }
    if !valid_hostname(&cfg.system.hostname) {
        out.push("Hostname inválido (letras, números, puntos y guiones).".to_string());
    }
    if !valid_username(&cfg.system.username) {
        out.push("Usuario: minúsculas, debe empezar por letra o _.".to_string());
    }
    if cfg.system.password.is_empty() {
        out.push("Falta la contraseña del usuario.".to_string());
    }
    if matches!(
        cfg.output,
        OutputFormat::Qcow2 | OutputFormat::Usb | OutputFormat::Vmdk | OutputFormat::Vdi
    ) && engine != EngineId::Kiwi
    {
        out.push("QCOW2/USB/VMDK/VDI solo disponibles con motor KIWI (bases openSUSE).".to_string());
    }
    if cfg.output == OutputFormat::Oci && engine == EngineId::Manual {
        out.push("OCI no disponible en bases manuales (Gentoo/Solus).".to_string());
    }
    if cfg.output == OutputFormat::Wsl && engine == EngineId::Manual {
        out.push("WSL no disponible en bases manuales (Gentoo/Solus).".to_string());
    }
    if !cfg.wallpaper.is_empty() {
        if !is_wallpaper_data_url(&cfg.wallpaper) {
            out.push("Wallpaper: formato no soportado (usa png, jpeg o webp).".to_string());
        } else if cfg.wallpaper.len() > 3_000_000 {
            out.push("Wallpaper demasiado grande (máximo ~2 MB).".to_string());
        } else if engine == EngineId::Kickstart {
            out.push("Wallpaper aún no se aplica en Fedora; se ignorará.".to_string());
        }
    }
    for a in &cfg.flatpak_apps {
        if !valid_flatpak_id(a) {
            out.push(format!(
                "Flatpak: \"{a}\" no parece un id de app válido (ej. org.mozilla.firefox)."
            ));
        }
    }
    for (i, f) in cfg.extra_files.iter().enumerate() {
        if f.path.is_empty() || !f.path.starts_with('/') {
            out.push(format!(
                "Fichero extra #{}: la ruta debe ser absoluta (empieza por /).",
                i + 1
            ));
        } else if f.path.contains("..") {
            out.push(format!(
                "Fichero extra #{}: la ruta no puede contener \"..\".",
                i + 1
            ));
        }
    }
    if engine == EngineId::Kickstart && cfg.repos.iter().any(|r| !r.gpg_key.is_empty()) {
        out.push(
            "Fedora: los repos custom se instalan sin verificación GPG (anaconda no puede preguntar); la key se importa para el sistema final.".to_string(),
        );
    }
    if engine == EngineId::Mkarchiso && cfg.repos.iter().any(|r| !r.gpg_key.is_empty()) {
        out.push("Arch: los repos custom van con TrustAll; la key GPG no se usa.".to_string());
    }
    let mut seen_users: Vec<&str> = Vec::new();
    for (i, u) in cfg.users.iter().enumerate() {
        if !valid_username(&u.name) {
            out.push(format!(
                "Usuario extra #{}: nombre inválido (minúsculas, empieza por letra o _).",
                i + 1
            ));
        } else if u.name == cfg.system.username || seen_users.contains(&u.name.as_str()) {
            out.push(format!("Usuario extra \"{}\": nombre repetido.", u.name));
        } else {
            seen_users.push(&u.name);
        }
        if u.password.is_empty() {
            let who = if u.name.is_empty() {
                (i + 1).to_string()
            } else {
                u.name.clone()
            };
            out.push(format!("Usuario extra \"{who}\": falta la contraseña."));
        }
    }
    for (i, s) in cfg.services.iter().enumerate() {
        if !valid_unit(&s.unit) {
            out.push(format!(
                "Servicio #{}: nombre de unidad inválido (ej. sshd.service).",
                i + 1
            ));
        }
    }
    if cfg.network.mode == NetMode::Static {
        let n = &cfg.network;
        if !is_ipv4(&n.ip) {
            out.push("Red: IP estática inválida.".to_string());
        }
        if !prefix_in_range(&n.prefix) {
            out.push("Red: prefijo CIDR inválido (8-32).".to_string());
        }
        if !is_ipv4(&n.gateway) {
            out.push("Red: gateway inválido.".to_string());
        }
        let dns: Vec<&String> = n.dns.iter().filter(|d| !d.is_empty()).collect();
        if dns.is_empty() || dns.iter().any(|d| !is_ipv4(d)) {
            out.push("Red: falta al menos un servidor DNS válido (IPv4).".to_string());
        }
        if engine == EngineId::Mkarchiso {
            out.push(
                "Arch: la red estática no se aplica en la ISO live (NetworkManager hace DHCP al arrancar).".to_string(),
            );
        }
    }
    if !archs_for_engine(engine).contains(&cfg.arch) {
        out.push(format!(
            "Arquitectura: {} no soportada por el motor {}.",
            cfg.arch, engine
        ));
    }
    if (cfg.output == OutputFormat::Oci || cfg.output == OutputFormat::Wsl)
        && (cfg.arch == ArchId::Armhf || cfg.arch == ArchId::I386)
    {
        out.push("Arquitectura: armhf/i386 en OCI/WSL es experimental.".to_string());
    }
    if !cfg.grub.timeout.is_finite() || cfg.grub.timeout < 0.0 || cfg.grub.timeout > 999.0 {
        out.push("GRUB: timeout inválido (0-999 segundos).".to_string());
    }
    if cfg.output == OutputFormat::Oci || cfg.output == OutputFormat::Wsl {
        let mut dropped: Vec<&str> = Vec::new();
        if !cfg.motd.is_empty() {
            dropped.push("MOTD");
        }
        if !cfg.wallpaper.is_empty() {
            dropped.push("wallpaper");
        }
        if cfg.cloud_init {
            dropped.push("cloud-init");
        }
        if cfg.autologin {
            dropped.push("auto-login");
        }
        if cfg.network.mode == NetMode::Static {
            dropped.push("red estática");
        }
        if !cfg.services.is_empty() {
            dropped.push("servicios systemd");
        }
        if !cfg.autostart.is_empty() {
            dropped.push("autostart");
        }
        if !dropped.is_empty() {
            out.push(format!(
                "Salida {}: se ignoran {} (no aplican a contenedor/WSL).",
                cfg.output.as_str().to_uppercase(),
                dropped.join(", ")
            ));
        }
    }
    if cfg.advanced {
        let url = &cfg.custom_iso.url;
        if !is_http_url_nonempty(url) {
            out.push("Advanced: falta la URL de la ISO base (http/https).".to_string());
        } else if !ends_with_iso(url) {
            out.push("Advanced: la URL debe apuntar a un fichero .iso.".to_string());
        }
        if cfg.output != OutputFormat::Iso {
            out.push("Advanced: la salida es siempre ISO.".to_string());
        }
    }
    for (i, r) in cfg.repos.iter().enumerate() {
        if r.url.is_empty() {
            if !r.alias.is_empty() || !r.gpg_key.is_empty() {
                out.push(format!("Repo #{}: falta la URL.", i + 1));
            }
        } else if !is_http_url(&r.url) {
            out.push(format!("Repo \"{}\": la URL debe empezar por http(s)://.", r.alias));
        }
        if !r.gpg_key.is_empty() && !is_http_url(&r.gpg_key) {
            out.push(format!("Repo \"{}\": la key GPG debe ser una URL http(s).", r.alias));
        }
    }
    out
}

/// Workflow de GitHub Actions del proyecto (público porque el TS lo expone
/// indirectamente a través de `generateProject`; aquí también se usa en tests).
pub fn workflow_yaml(cfg: &DistroConfig) -> String {
    let engine = base_by_id(cfg.base).engine;
    let artifact = format!("{}-image", cfg.name);
    let runner = if cfg.arch == ArchId::Aarch64 {
        "ubuntu-24.04-arm"
    } else {
        "ubuntu-latest"
    };
    let qemu_step = if cfg.arch == ArchId::Armhf {
        "      - name: QEMU\n        uses: docker/setup-qemu-action@v3\n"
    } else {
        ""
    };
    let grub_extra = cfg
        .grub
        .extra_params
        .replace(['"', '`', '$', '\\', '/'], "")
        .trim()
        .to_string();
    let head = format!(
        "name: build\non:\n  workflow_dispatch:\n  push:\n    branches: [main]\njobs:\n  build:\n    runs-on: {runner}\n"
    );
    let upload = |path: &str| {
        format!(
            "      - name: Subir artefacto\n        uses: actions/upload-artifact@v4\n        with:\n          name: {artifact}\n          path: {path}\n          if-no-files-found: error\n"
        )
    };

    if engine == EngineId::Manual {
        return format!(
            "name: build\non:\n  workflow_dispatch:\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - name: Sin builder automático\n        run: |\n          echo \"Base manual: {} no tiene builder automático en DistroForge.\"\n          echo \"Revisa manual/README.md en el repo.\"\n          exit 1\n",
            cfg.name
        );
    }

    if cfg.output == OutputFormat::Oci {
        return format!(
            "{head}    steps:\n      - name: Checkout\n        uses: actions/checkout@v4\n{qemu}      - name: Construir imagen OCI\n        run: |\n          docker build -t {name}:1.0.0 .\n          docker save {name}:1.0.0 -o {name}.tar\n      - name: SBOM (syft)\n        uses: anchore/sbom-action@v0\n        with:\n          format: spdx-json\n          artifact: image\n          image: {name}:1.0.0\n      - name: Escaneo CVE (trivy)\n        uses: aquasecurity/trivy-action@0.28.0\n        with:\n          image-ref: {name}:1.0.0\n          severity: HIGH,CRITICAL\n          exit-code: \"0\"\n      - name: Subir artefacto\n        uses: actions/upload-artifact@v4\n        with:\n          name: {name}-oci\n          path: {name}.tar\n          if-no-files-found: error\n",
            head = head,
            qemu = qemu_step,
            name = cfg.name
        );
    }

    if cfg.output == OutputFormat::Wsl {
        let b = base_by_id(cfg.base);
        let container = match engine {
            EngineId::Livebuild => b.container.unwrap_or("debian:bookworm"),
            EngineId::Mkarchiso => "archlinux:base-devel",
            EngineId::Kiwi => "opensuse/tumbleweed:latest",
            _ => "fedora:latest",
        };
        let privileged = if engine == EngineId::Livebuild {
            ""
        } else {
            "\n      options: --privileged"
        };
        return format!(
            "{head}    container:\n      image: {container}{privileged}\n    steps:\n      - name: Checkout\n        uses: actions/checkout@v4\n{qemu}      - name: Construir rootfs WSL\n        run: bash wsl/rootfs.sh\n{upload}",
            head = head,
            container = container,
            privileged = privileged,
            qemu = qemu_step,
            upload = upload("out/*.tar")
        );
    }

    if engine == EngineId::Kiwi {
        return format!(
            "{head}    container:\n      image: registry.opensuse.org/opensuse/tumbleweed:latest\n      options: --privileged\n    steps:\n      - name: Checkout\n        uses: actions/checkout@v4\n      - name: Instalar KIWI NG\n        run: zypper --non-interactive install python3-kiwi kiwi-systemdeps-core kiwi-systemdeps-iso-media kiwi-systemdeps-disk-images\n      - name: Construir imagen\n        run: kiwi-ng system build --description kiwi --target-dir out\n{upload}",
            head = head,
            upload = upload("out/*")
        );
    }

    if engine == EngineId::Livebuild {
        let container = base_by_id(cfg.base).container.unwrap_or("debian:bookworm");
        return format!(
            "{head}    container:\n      image: {container}\n    steps:\n      - name: Checkout\n        uses: actions/checkout@v4\n{qemu}      - name: Instalar live-build\n        run: apt-get update && apt-get install -y live-build\n      - name: Construir imagen\n        run: |\n          cd livebuild\n          chmod +x auto/* config/hooks/* || true\n          lb config\n          lb build\n{upload}",
            head = head,
            container = container,
            qemu = qemu_step,
            upload = upload("livebuild/*.iso")
        );
    }

    if engine == EngineId::Mkarchiso {
        let grub_sed_extra = if grub_extra.is_empty() {
            String::new()
        } else {
            format!(
                "\n            sed -i \"/^[[:space:]]*linux /s/archisolabel=[^ ]*/& {grub_extra}/\" /tmp/profile/grub/grub.cfg"
            )
        };
        return format!(
            "{head}    container:\n      image: archlinux:base-devel\n      options: --privileged\n    steps:\n      - name: Checkout\n        uses: actions/checkout@v4\n      - name: Instalar archiso\n        run: pacman -Syu --noconfirm archiso\n      - name: Construir imagen\n        run: |\n          cp -r /usr/share/archiso/configs/releng /tmp/profile\n          cp archiso/packages.x86_64 /tmp/profile/packages.x86_64\n          cp archiso/pacman.conf /tmp/profile/pacman.conf\n          cp -a archiso/airootfs/. /tmp/profile/airootfs/\n          if [ -f /tmp/profile/grub/grub.cfg ]; then\n            sed -i \"s/^set timeout.*/set timeout={timeout}/\" /tmp/profile/grub/grub.cfg{extra}\n          fi\n          mkdir -p /tmp/work out\n          mkarchiso -v -w /tmp/work -o out /tmp/profile\n{upload}",
            head = head,
            timeout = crate::types::js_num(cfg.grub.timeout),
            extra = grub_sed_extra,
            upload = upload("out/*.iso")
        );
    }

    format!(
        "{head}    container:\n      image: fedora:latest\n      options: --privileged\n    steps:\n      - name: Checkout\n        uses: actions/checkout@v4\n      - name: Instalar livemedia-creator\n        run: dnf install -y livemedia-creator\n      - name: Construir imagen\n        run: livemedia-creator --no-virt --make-iso --ks fedora/ks.cfg --releasever 41 --iso-only --iso-name {name}.iso --resultdir out\n{upload}",
        head = head,
        name = cfg.name,
        upload = upload(&format!("out/{}.iso", cfg.name))
    )
}

/// README.md del proyecto. Port de `readmeMd`.
pub fn readme_md(cfg: &DistroConfig) -> String {
    let b = base_by_id(cfg.base);
    let dir = match b.engine {
        EngineId::Kiwi => "kiwi",
        EngineId::Livebuild => "livebuild",
        EngineId::Mkarchiso => "archiso",
        EngineId::Kickstart => "fedora",
        EngineId::Manual => "manual",
    };
    let desk = b
        .desktops
        .iter()
        .find(|d| d.id == cfg.desktop)
        .map(|d| d.label)
        .unwrap_or("ninguno");
    let repos = effective_repos(cfg);
    let arch_note = if b.engine == EngineId::Mkarchiso {
        "\nNota Arch: usuario, contraseña y zona horaria del live los gestiona el perfil releng de archiso; ajústalos tras arrancar o editando la receta.\n"
    } else {
        ""
    };
    let flatpak_note = if b.engine == EngineId::Mkarchiso && !cfg.flatpak_apps.is_empty() {
        "\nNota Flatpak: en la ISO Arch las apps Flathub se instalan en el primer arranque (servicio distroforge-flatpak); necesita red.\n"
    } else {
        ""
    };
    let pkgs = if !cfg.packages.is_empty() {
        cfg.packages.join(", ")
    } else {
        "ninguno".to_string()
    };
    let apps = if !cfg.flatpak_apps.is_empty() {
        cfg.flatpak_apps.join(", ")
    } else {
        "ninguno".to_string()
    };
    let autologin_note = if cfg.autologin { " · auto-login" } else { "" };
    let kernel_note = if !cfg.grub.extra_params.is_empty() {
        format!(" · kernel: {}", cfg.grub.extra_params)
    } else {
        String::new()
    };
    let net_desc = if cfg.network.mode == NetMode::Static {
        format!("estática ({}/{})", cfg.network.ip, cfg.network.prefix)
    } else {
        "DHCP".to_string()
    };
    let repo_urls = if !repos.is_empty() {
        repos.iter().map(|r| r.url.as_str()).collect::<Vec<_>>().join(", ")
    } else {
        "ninguno".to_string()
    };
    let ci_note = if cfg.cloud_init { " + cloud-init" } else { "" };
    let oci_note = if cfg.output == OutputFormat::Oci {
        "\nSalida OCI: construye con `bash build.sh` (docker o podman) o el workflow de CI; carga el .tar con `docker load -i`.\n"
    } else {
        ""
    };
    let wsl_note = if cfg.output == OutputFormat::Wsl {
        format!(
            "\nSalida WSL: importa el tar en Windows con:\n\n    wsl --import {} C:\\WSL\\{} <ruta>\\out\\{}-wsl.tar\n\nLuego `wsl -d {}`. Usuario por defecto configurado vía /etc/wsl.conf.\n",
            cfg.name, cfg.name, cfg.name, cfg.name
        )
    } else {
        String::new()
    };
    let wp_note = if !cfg.wallpaper.is_empty() {
        "\nWallpaper: se copia a /usr/share/backgrounds y se aplica vía dconf (GNOME/MATE/Cinnamon/Budgie) o xfconf (Xfce). En KDE, i3, Sway y Pantheon solo se copia el fichero.\n"
    } else {
        ""
    };
    let manual_note = if b.engine == EngineId::Manual {
        "\n> Motor **manual**: sin builder automático; el zip trae lista de paquetes y guía.\n"
    } else {
        ""
    };

    format!(
        "# {name}\n\nDistro personalizada generada con DistroForge.\n\n- Base: {label} (motor: {engine})\n- Escritorio: {desk}\n- Paquetes extra: {pkgs}\n- Flatpak: {apps}\n- Ficheros extra: {nfiles}\n- Usuarios extra: {nusers} · Servicios systemd: {nsvc}{autologin}\n- Arquitectura: {arch} · GRUB: timeout {timeout}s{kernel} · Firewall: {fw}\n- Red: {net}\n\n> Este proyecto incluye `distroforge.json`: puedes reimportarlo en DistroForge (Cargar config) o editarlo en git y volver a construir.\n- Repos custom: {repos}\n- Salida: {out}{ci}\n- Wallpaper: {wp}\n{oci}{wsl}{wpn}\n{manual}\n{arch_note}\n{flatpak_note}\n## Construir la imagen\n\n**Opción A — GitHub Actions (recomendado):** sube esta carpeta a un repo nuevo de GitHub. En *Actions → build → Run workflow*. El artefacto contendrá la imagen.\n\n**Opción B — local (Linux o WSL2 con Podman):**\n\n    bash build.sh\n\n## Estructura\n\n- `{dir}/` — receta de construcción del motor\n- `build.sh` — build local con Podman\n- `.github/workflows/build.yml` — build en CI\n\n## Seguridad\n\nLa contraseña del usuario va en texto plano dentro de la receta (`{dir}/`). No publiques el repo sin cambiarla o moverla a secretos de CI.\n",
        name = cfg.name,
        label = b.label,
        engine = b.engine,
        desk = desk,
        pkgs = pkgs,
        apps = apps,
        nfiles = cfg.extra_files.len(),
        nusers = cfg.users.len(),
        nsvc = cfg.services.len(),
        autologin = autologin_note,
        arch = cfg.arch,
        timeout = crate::types::js_num(cfg.grub.timeout),
        kernel = kernel_note,
        fw = if cfg.firewall { "habilitado (firewalld)" } else { "deshabilitado" },
        net = net_desc,
        repos = repo_urls,
        out = cfg.output.as_str().to_uppercase(),
        ci = ci_note,
        wp = if cfg.wallpaper.is_empty() { "no" } else { "sí" },
        oci = oci_note,
        wsl = wsl_note,
        wpn = wp_note,
        manual = manual_note,
        arch_note = arch_note,
        flatpak_note = flatpak_note,
        dir = dir,
    )
}
