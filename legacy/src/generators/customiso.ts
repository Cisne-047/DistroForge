import type { DistroConfig } from "../types";
import { baseById } from "../catalog";
import {
  autologinFiles,
  extraUserShellLines,
  firewallServiceLine,
  serviceShellLines,
} from "./accounts";

/** Familia de paquetería de la ISO base, deducida del motor de la base elegida. */
export function pkgTypeOf(cfg: DistroConfig): "deb" | "rpm" | "pacman" | "none" {
  const engine = baseById(cfg.base).engine;
  if (engine === "livebuild") return "deb";
  if (engine === "kickstart" || engine === "kiwi") return "rpm";
  if (engine === "mkarchiso") return "pacman";
  return "none";
}

function adminGroupOf(cfg: DistroConfig): "sudo" | "wheel" {
  return pkgTypeOf(cfg) === "deb" ? "sudo" : "wheel";
}

/** Script que corre DENTRO del chroot de la ISO remasterizada. */
export function chrootScript(cfg: DistroConfig): string {
  const sys = cfg.system;
  const group = adminGroupOf(cfg);
  const type = pkgTypeOf(cfg);
  const heredoc = (path: string, content: string) =>
    `mkdir -p "$(dirname "${path}")"\ncat > "${path}" <<'DFEOF'\n${content}\nDFEOF`;

  const lines: string[] = [
    "#!/bin/bash",
    "set -euo pipefail",
    "# DistroForge - chroot de la ISO remasterizada (avanzado)",
  ];
  if (type !== "none") {
    lines.push(
      `echo "${sys.hostname}" > /etc/hostname`,
      `ln -sf /usr/share/zoneinfo/${sys.timezone} /etc/localtime`
    );
  }
  if (type === "deb" || type === "rpm" || type === "pacman") {
    lines.push(
      `id "${sys.username}" >/dev/null 2>&1 || useradd -m -s /bin/bash -G ${group} "${sys.username}"`,
      `echo "${sys.username}:${sys.password.replace(/"/g, '\\"')}" | chpasswd`
    );
    lines.push(...extraUserShellLines(cfg, group));
  }
  if (cfg.motd) lines.push(heredoc("/etc/motd", cfg.motd));
  lines.push(...(cfg.extraFiles ?? []).map((f) => heredoc(f.path, f.content)));
  lines.push(...Object.entries(autologinFiles(cfg)).map(([p, c]) => heredoc(p, c)));
  lines.push(firewallServiceLine(cfg), ...serviceShellLines(cfg));
  if (cfg.grub?.extraParams?.trim() || (cfg.grub?.timeout ?? 10) !== 10) {
    lines.push(
      'if [ -f /etc/default/grub ]; then',
      `  sed -i "s/^GRUB_TIMEOUT=.*/GRUB_TIMEOUT=${cfg.grub?.timeout ?? 10}/" /etc/default/grub || true`,
      ...(cfg.grub?.extraParams?.trim()
        ? [
            `  sed -i "s/^GRUB_CMDLINE_LINUX=\\".*\\"/GRUB_CMDLINE_LINUX=\\"${cfg.grub.extraParams.replace(/["`\\]/g, "")}\\"/" /etc/default/grub || true`,
          ]
        : []),
      "fi"
    );
  }
  const userScript = cfg.customIso?.chrootScript?.trim();
  if (userScript) {
    lines.push("# --- script avanzado del usuario ---", userScript);
  }
  return lines.join("\n") + "\n";
}

/** Proyecto completo del modo advanced: descarga la ISO, remasteriza y repack. */
export function customIsoFiles(cfg: DistroConfig): Record<string, string> {
  const name = cfg.name;
  const download = `#!/usr/bin/env bash
set -euo pipefail
# DistroForge - descarga de la ISO base
mkdir -p out
wget -c "${cfg.customIso.url}" -O base.iso
echo "ISO base descargada: base.iso"
`;

  const remaster = `#!/usr/bin/env bash
set -euo pipefail
# DistroForge - remaster de ISO custom (EXPERIMENTAL)
# Extrae la ISO, aplica tu config en chroot y reconstruye con los mismos
# parámetros de arranque que la original (xorriso -report_el_torito).
[ "$(id -u)" -eq 0 ] || { echo "Ejecuta como root (monta y desmonta ficheros)."; exit 1; }
command -v xorriso >/dev/null 2>&1 || { apt-get update && apt-get install -y xorriso squashfs-tools; }
rm -rf isoroot chroot && mkdir -p isoroot chroot out
xorriso -osirrox on -indev base.iso -extract / isoroot
SQ="$(find isoroot \\( -name 'filesystem.squashfs' -o -name 'airootfs.sfs' -o -name 'squashfs.img' \\) | head -n1)"
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
REL="\${SQ#isoroot/}"
mksquashfs chroot "isoroot/$REL" -comp xz -noappend
OPTS="$(xorriso -indev base.iso -report_el_torito as_mkisofs 2>/dev/null | tail -n1)"
# shellcheck disable=SC2086
xorriso -as mkisofs \${OPTS#xorriso -as mkisofs} -o "out/${name}.iso" isoroot \\
  || xorriso -as mkisofs -volid "${name.slice(0, 28)}" -o "out/${name}.iso" isoroot
echo "ISO remasterizada en out/${name}.iso"
`;

  const buildSh = `#!/usr/bin/env bash
set -euo pipefail
# DistroForge - build local del modo advanced (Linux, requiere Podman/Docker)
DIR="$(cd "$(dirname "$0")" && pwd)"
CMD="$(command -v docker || command -v podman)"
"$CMD" run --rm --privileged -v "$DIR":/project -w /project debian:bookworm bash -ec '
  apt-get update && apt-get install -y wget ca-certificates xorriso squashfs-tools
  bash advanced/download.sh
  bash advanced/remaster.sh
'
echo "ISO lista en ./out/${name}.iso"
`;

  const readme = `# ${name} — modo advanced (ISO custom)

**Experimental.** Remasteriza una ISO que tú aportas: la extrae, aplica tu
configuración en un chroot y la reconstruye con los mismos parámetros de
arranque que la original.

- Base: ${baseById(cfg.base).label} (familia de paquetería: ${pkgTypeOf(cfg)})
- ISO base: ${cfg.customIso.url}
- Salida: ISO única

## Ficheros

- \`advanced/download.sh\` — descarga la ISO base
- \`advanced/chroot.sh\` — tu configuración dentro del chroot (edítalo a mano para control total)
- \`advanced/remaster.sh\` — extrae, aplica y reconstruye

## Requisitos

- Linux con root, \`xorriso\` y \`squashfs-tools\` (o \`bash build.sh\` con Podman/Docker)
- ISOs multi-arquitectura o de otra arquitectura necesitan \`qemu-user-static\`

## Seguridad

La contraseña va en claro dentro de \`advanced/chroot.sh\`. No publiques el repo sin cambiarla.
`;

  const workflow = `name: build
on:
  workflow_dispatch:
  push:
    branches: [main]
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4
      - name: Descargar y remasterizar
        run: |
          sudo apt-get update && sudo apt-get install -y wget ca-certificates xorriso squashfs-tools
          bash advanced/download.sh
          sudo bash advanced/remaster.sh
      - name: Subir artefacto
        uses: actions/upload-artifact@v4
        with:
          name: ${name}-iso
          path: out/*.iso
          if-no-files-found: error
`;

  return {
    "advanced/download.sh": download,
    "advanced/chroot.sh": chrootScript(cfg),
    "advanced/remaster.sh": remaster,
    "build.sh": buildSh,
    "README.md": readme,
    "distroforge.json": JSON.stringify(cfg, null, 2) + "\n",
    ".github/workflows/build.yml": workflow,
  };
}
