//! Modo advanced: remasterizado de una ISO base que trae el usuario. Port de
//! `src/generators/customiso.ts`.

use crate::catalog::base_by_id;
use crate::generators::accounts::{
    autologin_files, extra_user_shell_lines, firewall_service_line, service_shell_lines,
    AdminGroup,
};
use crate::types::{DistroConfig, EngineId};

/// Familia de paquetería de la ISO base, deducida del motor de la base elegida.
pub fn pkg_type_of(cfg: &DistroConfig) -> &'static str {
    match base_by_id(cfg.base).engine {
        EngineId::Livebuild => "deb",
        EngineId::Kickstart | EngineId::Kiwi => "rpm",
        EngineId::Mkarchiso => "pacman",
        EngineId::Manual => "none",
    }
}

fn admin_group_of(cfg: &DistroConfig) -> AdminGroup {
    if pkg_type_of(cfg) == "deb" {
        AdminGroup::Sudo
    } else {
        AdminGroup::Wheel
    }
}

/// Primeros 28 caracteres (como `name.slice(0, 28)` de JS).
fn slice28(s: &str) -> String {
    s.chars().take(28).collect()
}

fn heredoc(path: &str, content: &str) -> String {
    format!(
        "mkdir -p \"$(dirname \"{path}\")\"\ncat > \"{path}\" <<'DFEOF'\n{content}\nDFEOF"
    )
}

/// Script que corre DENTRO del chroot de la ISO remasterizada.
pub fn chroot_script(cfg: &DistroConfig) -> String {
    let sys = &cfg.system;
    let group = admin_group_of(cfg);
    let ptype = pkg_type_of(cfg);

    let mut lines: Vec<String> = vec![
        "#!/bin/bash".to_string(),
        "set -euo pipefail".to_string(),
        "# DistroForge - chroot de la ISO remasterizada (avanzado)".to_string(),
    ];
    if ptype != "none" {
        lines.push(format!("echo \"{}\" > /etc/hostname", sys.hostname));
        lines.push(format!(
            "ln -sf /usr/share/zoneinfo/{} /etc/localtime",
            sys.timezone
        ));
    }
    if ptype == "deb" || ptype == "rpm" || ptype == "pacman" {
        lines.push(format!(
            "id \"{}\" >/dev/null 2>&1 || useradd -m -s /bin/bash -G {} \"{}\"",
            sys.username,
            group.as_str(),
            sys.username
        ));
        lines.push(format!(
            "echo \"{}:{}\" | chpasswd",
            sys.username,
            sys.password.replace('"', "\\\"")
        ));
        lines.extend(extra_user_shell_lines(cfg, group));
    }
    if !cfg.motd.is_empty() {
        lines.push(heredoc("/etc/motd", &cfg.motd));
    }
    for f in &cfg.extra_files {
        lines.push(heredoc(&f.path, &f.content));
    }
    for (p, c) in autologin_files(cfg) {
        lines.push(heredoc(&p, &c));
    }
    lines.push(firewall_service_line(cfg));
    lines.extend(service_shell_lines(cfg));
    if !cfg.grub.extra_params.trim().is_empty() || cfg.grub.timeout != 10.0 {
        lines.push("if [ -f /etc/default/grub ]; then".to_string());
        lines.push(format!(
            "  sed -i \"s/^GRUB_TIMEOUT=.*/GRUB_TIMEOUT={}/\" /etc/default/grub || true",
            crate::types::js_num(cfg.grub.timeout)
        ));
        if !cfg.grub.extra_params.trim().is_empty() {
            let cleaned = cfg.grub.extra_params.replace(['"', '`', '\\'], "");
            lines.push(format!(
                "  sed -i \"s/^GRUB_CMDLINE_LINUX=\\\".*\\\"/GRUB_CMDLINE_LINUX=\\\"{cleaned}\\\"/\" /etc/default/grub || true"
            ));
        }
        lines.push("fi".to_string());
    }
    let user_script = cfg.custom_iso.chroot_script.trim();
    if !user_script.is_empty() {
        lines.push("# --- script avanzado del usuario ---".to_string());
        lines.push(user_script.to_string());
    }
    format!("{}\n", lines.join("\n"))
}

/// Proyecto completo del modo advanced: descarga la ISO, remasteriza y repack.
pub fn custom_iso_files(cfg: &DistroConfig) -> Vec<(String, String)> {
    let name = &cfg.name;
    let name28 = slice28(name);

    let download = format!(
        "#!/usr/bin/env bash\nset -euo pipefail\n# DistroForge - descarga de la ISO base\nmkdir -p out\nwget -c \"{}\" -O base.iso\necho \"ISO base descargada: base.iso\"\n",
        cfg.custom_iso.url
    );

    // El script de remaster contiene `${...}` literal de shell; se usan tokens
    // en vez de format! para no escapar llaves.
    let remaster = r#"#!/usr/bin/env bash
set -euo pipefail
# DistroForge - remaster de ISO custom (EXPERIMENTAL)
# Extrae la ISO, aplica tu config en chroot y reconstruye con los mismos
# parámetros de arranque que la original (xorriso -report_el_torito).
[ "$(id -u)" -eq 0 ] || { echo "Ejecuta como root (monta y desmonta ficheros)."; exit 1; }
command -v xorriso >/dev/null 2>&1 || { apt-get update && apt-get install -y xorriso squashfs-tools; }
rm -rf isoroot chroot && mkdir -p isoroot chroot out
xorriso -osirrox on -indev base.iso -extract / isoroot
SQ="$(find isoroot \( -name 'filesystem.squashfs' -o -name 'airootfs.sfs' -o -name 'squashfs.img' \) | head -n1)"
[ -n "$SQ" ] || { echo "No encuentro un squashfs raíz en la ISO"; exit 1; }
unsquashfs -d chroot "$SQ" >/dev/null
mount --bind /dev chroot/dev
mount -t proc proc chroot/proc
mount --bind /sys chroot/sys 2>/dev/null || true
cp -L /etc/resolv.conf chroot/etc/resolv.conf
bash advanced/chroot.sh
rm -f chroot/etc/resolv.conf
umount chroot/dev chroot/proc 2>/dev/null || true
umount chroot/sys 2>/dev/null || true
REL="${SQ#isoroot/}"
mksquashfs chroot "isoroot/$REL" -comp xz -noappend
OPTS="$(xorriso -indev base.iso -report_el_torito as_mkisofs 2>/dev/null | tail -n1)"
# shellcheck disable=SC2086
xorriso -as mkisofs ${OPTS#xorriso -as mkisofs} -o "out/@NAME@.iso" isoroot \
  || xorriso -as mkisofs -volid "@NAME28@" -o "out/@NAME@.iso" isoroot
echo "ISO remasterizada en out/@NAME@.iso"
"#
    .replace("@NAME28@", &name28)
    .replace("@NAME@", name);

    let build_sh = format!(
        "#!/usr/bin/env bash\nset -euo pipefail\n# DistroForge - build local del modo advanced (Linux, requiere Podman/Docker)\nDIR=\"$(cd \"$(dirname \"$0\")\" && pwd)\"\nCMD=\"$(command -v docker || command -v podman)\"\n\"$CMD\" run --rm --privileged -v \"$DIR\":/project -w /project debian:bookworm bash -ec '\n  apt-get update && apt-get install -y wget ca-certificates xorriso squashfs-tools\n  bash advanced/download.sh\n  bash advanced/remaster.sh\n'\necho \"ISO lista en ./out/{name}.iso\"\n"
    );

    let readme = format!(
        "# {name} — modo advanced (ISO custom)\n\n**Experimental.** Remasteriza una ISO que tú aportas: la extrae, aplica tu\nconfiguración en un chroot y la reconstruye con los mismos parámetros de\narranque que la original.\n\n- Base: {} (familia de paquetería: {})\n- ISO base: {}\n- Salida: ISO única\n\n## Ficheros\n\n- `advanced/download.sh` — descarga la ISO base\n- `advanced/chroot.sh` — tu configuración dentro del chroot (edítalo a mano para control total)\n- `advanced/remaster.sh` — extrae, aplica y reconstruye\n\n## Requisitos\n\n- Linux con root, `xorriso` y `squashfs-tools` (o `bash build.sh` con Podman/Docker)\n- ISOs multi-arquitectura o de otra arquitectura necesitan `qemu-user-static`\n\n## Seguridad\n\nLa contraseña va en claro dentro de `advanced/chroot.sh`. No publiques el repo sin cambiarla.\n",
        base_by_id(cfg.base).label,
        pkg_type_of(cfg),
        cfg.custom_iso.url
    );

    let workflow = format!(
        "name: build\non:\n  workflow_dispatch:\n  push:\n    branches: [main]\njobs:\n  build:\n    runs-on: ubuntu-latest\n    steps:\n      - name: Checkout\n        uses: actions/checkout@v4\n      - name: Descargar y remasterizar\n        run: |\n          sudo apt-get update && sudo apt-get install -y wget ca-certificates xorriso squashfs-tools\n          bash advanced/download.sh\n          sudo bash advanced/remaster.sh\n      - name: Subir artefacto\n        uses: actions/upload-artifact@v4\n        with:\n          name: {name}-iso\n          path: out/*.iso\n          if-no-files-found: error\n"
    );

    let json = serde_json::to_string_pretty(cfg).unwrap_or_else(|_| "null".to_string());
    vec![
        ("advanced/download.sh".to_string(), download),
        ("advanced/chroot.sh".to_string(), chroot_script(cfg)),
        ("advanced/remaster.sh".to_string(), remaster),
        ("build.sh".to_string(), build_sh),
        ("README.md".to_string(), readme),
        ("distroforge.json".to_string(), format!("{json}\n")),
        (".github/workflows/build.yml".to_string(), workflow),
    ]
}
