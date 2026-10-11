import JSZip from "jszip";
import type { DistroConfig } from "../types";
import { baseById, effectiveRepos, archsForEngine } from "../catalog";
import { IPV4_RE } from "./network";
import { customIsoFiles } from "./customiso";
import { kiwiFiles } from "./kiwi";
import { livebuildFiles } from "./livebuild";
import { archisoFiles } from "./archiso";
import { kickstartFiles } from "./kickstart";
import { manualFiles } from "./manual";
import { dockerfileFiles, ociSupported } from "./dockerfile";
import { wslFiles } from "./wsl";

export type FileMap = Record<string, string>;

export function generateProject(cfg: DistroConfig): FileMap {
  if (cfg.advanced) return customIsoFiles(cfg);
  const engine = baseById(cfg.base).engine;
  const shared = () => ({
    ".github/workflows/build.yml": workflowYaml(cfg),
    "README.md": readmeMd(cfg),
    // round-trip: el proyecto generado se reimporta en la app tal cual
    "distroforge.json": JSON.stringify(cfg, null, 2) + "\n",
  });
  if (cfg.output === "oci" && engine !== "manual" && ociSupported(cfg))
    return { ...dockerfileFiles(cfg), ...shared() };
  if (cfg.output === "wsl" && engine !== "manual")
    return { ...wslFiles(cfg), ...shared() };
  const engineFiles =
    engine === "kiwi"
      ? kiwiFiles(cfg)
      : engine === "livebuild"
        ? livebuildFiles(cfg)
        : engine === "mkarchiso"
          ? archisoFiles(cfg)
          : engine === "kickstart"
            ? kickstartFiles(cfg)
            : manualFiles(cfg);
  return { ...engineFiles, ...shared() };
}

export function validate(cfg: DistroConfig): string[] {
  const out: string[] = [];
  if (!/^[a-z0-9][a-z0-9-]{1,31}$/.test(cfg.name))
    out.push("Nombre: minúsculas, números y guiones (2-32 caracteres).");
  if (!/^[a-zA-Z0-9][a-zA-Z0-9.-]{1,31}$/.test(cfg.system.hostname))
    out.push("Hostname inválido (letras, números, puntos y guiones).");
  if (!/^[a-z_][a-z0-9_-]{0,31}$/.test(cfg.system.username))
    out.push("Usuario: minúsculas, debe empezar por letra o _.");
  if (!cfg.system.password) out.push("Falta la contraseña del usuario.");
  if (["qcow2", "usb", "vmdk", "vdi"].includes(cfg.output) && baseById(cfg.base).engine !== "kiwi")
    out.push("QCOW2/USB/VMDK/VDI solo disponibles con motor KIWI (bases openSUSE).");
  if (cfg.output === "oci" && baseById(cfg.base).engine === "manual")
    out.push("OCI no disponible en bases manuales (Gentoo/Solus).");
  if (cfg.output === "wsl" && baseById(cfg.base).engine === "manual")
    out.push("WSL no disponible en bases manuales (Gentoo/Solus).");
  if (cfg.wallpaper) {
    if (!/^data:image\/(png|jpeg|webp);base64,/.test(cfg.wallpaper))
      out.push("Wallpaper: formato no soportado (usa png, jpeg o webp).");
    else if (cfg.wallpaper.length > 3_000_000)
      out.push("Wallpaper demasiado grande (máximo ~2 MB).");
    else if (baseById(cfg.base).engine === "kickstart")
      out.push("Wallpaper aún no se aplica en Fedora; se ignorará.");
  }
  if (!Array.isArray(cfg.flatpakApps)) {
    out.push("Flatpak: lista de apps inválida.");
  } else {
    for (const a of cfg.flatpakApps)
      if (!/^[A-Za-z0-9][A-Za-z0-9._-]*$/.test(a))
        out.push(`Flatpak: "${a}" no parece un id de app válido (ej. org.mozilla.firefox).`);
  }
  if (!Array.isArray(cfg.extraFiles)) {
    out.push("Ficheros extra: lista inválida.");
  } else {
    cfg.extraFiles.forEach((f, i) => {
      if (!f.path || !f.path.startsWith("/"))
        out.push(`Fichero extra #${i + 1}: la ruta debe ser absoluta (empieza por /).`);
      else if (f.path.includes(".."))
        out.push(`Fichero extra #${i + 1}: la ruta no puede contener "..".`);
    });
  }
  if (baseById(cfg.base).engine === "kickstart" && cfg.repos.some((r) => r.gpgKey))
    out.push(
      "Fedora: los repos custom se instalan sin verificación GPG (anaconda no puede preguntar); la key se importa para el sistema final."
    );
  if (baseById(cfg.base).engine === "mkarchiso" && cfg.repos.some((r) => r.gpgKey))
    out.push("Arch: los repos custom van con TrustAll; la key GPG no se usa.");
  const seenUsers = new Set<string>();
  (cfg.users ?? []).forEach((u, i) => {
    if (!/^[a-z_][a-z0-9_-]{0,31}$/.test(u.name))
      out.push(`Usuario extra #${i + 1}: nombre inválido (minúsculas, empieza por letra o _).`);
    else if (u.name === cfg.system.username || seenUsers.has(u.name))
      out.push(`Usuario extra "${u.name}": nombre repetido.`);
    else seenUsers.add(u.name);
    if (!u.password) out.push(`Usuario extra "${u.name || i + 1}": falta la contraseña.`);
  });
  (cfg.services ?? []).forEach((s, i) => {
    if (!/^[A-Za-z0-9@._-]+$/.test(s.unit))
      out.push(`Servicio #${i + 1}: nombre de unidad inválido (ej. sshd.service).`);
  });
  if (cfg.network && cfg.network.mode === "static") {
    const n = cfg.network;
    if (!IPV4_RE.test(n.ip)) out.push("Red: IP estática inválida.");
    if (!(Number(n.prefix) >= 8 && Number(n.prefix) <= 32))
      out.push("Red: prefijo CIDR inválido (8-32).");
    if (!IPV4_RE.test(n.gateway)) out.push("Red: gateway inválido.");
    const dns = (n.dns ?? []).filter(Boolean);
    if (!dns.length || dns.some((d) => !IPV4_RE.test(d)))
      out.push("Red: falta al menos un servidor DNS válido (IPv4).");
    if (baseById(cfg.base).engine === "mkarchiso")
      out.push(
        "Arch: la red estática no se aplica en la ISO live (NetworkManager hace DHCP al arrancar)."
      );
  }
  if (!archsForEngine(baseById(cfg.base).engine).includes(cfg.arch))
    out.push(`Arquitectura: ${cfg.arch} no soportada por el motor ${baseById(cfg.base).engine}.`);
  if (
    (cfg.output === "oci" || cfg.output === "wsl") &&
    (cfg.arch === "armhf" || cfg.arch === "i386")
  )
    out.push("Arquitectura: armhf/i386 en OCI/WSL es experimental.");
  if (cfg.grub && (!Number.isFinite(cfg.grub.timeout) || cfg.grub.timeout < 0 || cfg.grub.timeout > 999))
    out.push("GRUB: timeout inválido (0-999 segundos).");
  if (cfg.output === "oci" || cfg.output === "wsl") {
    const dropped = [
      cfg.motd && "MOTD",
      cfg.wallpaper && "wallpaper",
      cfg.cloudInit && "cloud-init",
      cfg.autologin && "auto-login",
      cfg.network?.mode === "static" && "red estática",
      (cfg.services?.length ?? 0) > 0 && "servicios systemd",
    ].filter(Boolean);
    if (dropped.length)
      out.push(
        `Salida ${cfg.output.toUpperCase()}: se ignoran ${dropped.join(", ")} (no aplican a contenedor/WSL).`
      );
  }
  if (cfg.advanced) {
    const url = cfg.customIso?.url ?? "";
    if (!/^https?:\/\/.+/i.test(url)) out.push("Advanced: falta la URL de la ISO base (http/https).");
    else if (!/\.iso([?#]|$)/i.test(url)) out.push("Advanced: la URL debe apuntar a un fichero .iso.");
    if (cfg.output !== "iso") out.push("Advanced: la salida es siempre ISO.");
  }
  cfg.repos.forEach((r, i) => {
    if (!r.url) {
      if (r.alias || r.gpgKey) out.push(`Repo #${i + 1}: falta la URL.`);
    } else if (!/^https?:\/\//.test(r.url)) {
      out.push(`Repo "${r.alias}": la URL debe empezar por http(s)://.`);
    }
    if (r.gpgKey && !/^https?:\/\//.test(r.gpgKey))
      out.push(`Repo "${r.alias}": la key GPG debe ser una URL http(s).`);
  });
  return out;
}

const B64_RE = /^data:image\/(png|jpeg|webp);base64,([A-Za-z0-9+/=]+)$/;

export async function downloadZip(cfg: DistroConfig): Promise<void> {
  const zip = new JSZip();
  for (const [path, content] of Object.entries(generateProject(cfg))) {
    const bin = B64_RE.exec(content);
    if (bin) zip.file(path, bin[2], { base64: true });
    else zip.file(path, content);
  }
  const blob = await zip.generateAsync({ type: "blob" });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = `${cfg.name}-distroforge.zip`;
  document.body.appendChild(a);
  a.click();
  a.remove();
  URL.revokeObjectURL(url);
}

function workflowYaml(cfg: DistroConfig): string {
  const engine = baseById(cfg.base).engine;
  const artifact = `${cfg.name}-image`;
  const runner = cfg.arch === "aarch64" ? "ubuntu-24.04-arm" : "ubuntu-latest";
  const qemuStep =
    cfg.arch === "armhf" ? "      - name: QEMU\n        uses: docker/setup-qemu-action@v3\n" : "";
  const grubExtra = (cfg.grub?.extraParams ?? "").replace(/["`$\\/]/g, "").trim();
  const head = `name: build
on:
  workflow_dispatch:
  push:
    branches: [main]
jobs:
  build:
    runs-on: ${runner}
`;
  const upload = (path: string) => `      - name: Subir artefacto
        uses: actions/upload-artifact@v4
        with:
          name: ${artifact}
          path: ${path}
          if-no-files-found: error
`;

  if (engine === "manual")
    return `name: build
on:
  workflow_dispatch:
jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - name: Sin builder automático
        run: |
          echo "Base manual: ${cfg.name} no tiene builder automático en DistroForge."
          echo "Revisa manual/README.md en el repo."
          exit 1
`;

  if (cfg.output === "oci")
    return `${head}    steps:
      - name: Checkout
        uses: actions/checkout@v4
${qemuStep}      - name: Construir imagen OCI
        run: |
          docker build -t ${cfg.name}:1.0.0 .
          docker save ${cfg.name}:1.0.0 -o ${cfg.name}.tar
      - name: SBOM (syft)
        uses: anchore/sbom-action@v0
        with:
          format: spdx-json
          artifact: image
          image: ${cfg.name}:1.0.0
      - name: Escaneo CVE (trivy)
        uses: aquasecurity/trivy-action@0.28.0
        with:
          image-ref: ${cfg.name}:1.0.0
          severity: HIGH,CRITICAL
          exit-code: "0"
      - name: Subir artefacto
        uses: actions/upload-artifact@v4
        with:
          name: ${cfg.name}-oci
          path: ${cfg.name}.tar
          if-no-files-found: error
`;

  if (cfg.output === "wsl") {
    const b = baseById(cfg.base);
    const container =
      engine === "livebuild"
        ? b.container ?? "debian:bookworm"
        : engine === "mkarchiso"
          ? "archlinux:base-devel"
          : engine === "kiwi"
            ? "opensuse/tumbleweed:latest"
            : "fedora:latest";
    const privileged = engine === "livebuild" ? "" : "\n      options: --privileged";
    return `${head}    container:
      image: ${container}${privileged}
    steps:
      - name: Checkout
        uses: actions/checkout@v4
${qemuStep}      - name: Construir rootfs WSL
        run: bash wsl/rootfs.sh
${upload("out/*.tar")}`;
  }

  if (engine === "kiwi")
    return `${head}    container:
      image: registry.opensuse.org/opensuse/tumbleweed:latest
      options: --privileged
    steps:
      - name: Checkout
        uses: actions/checkout@v4
      - name: Instalar KIWI NG
        run: zypper --non-interactive install python3-kiwi kiwi-systemdeps-core kiwi-systemdeps-iso-media kiwi-systemdeps-disk-images
      - name: Construir imagen
        run: kiwi-ng system build --description kiwi --target-dir out
${upload("out/*")}`;

  if (engine === "livebuild")
    return `${head}    container:
      image: ${baseById(cfg.base).container ?? "debian:bookworm"}
    steps:
      - name: Checkout
        uses: actions/checkout@v4
${qemuStep}      - name: Instalar live-build
        run: apt-get update && apt-get install -y live-build
      - name: Construir imagen
        run: |
          cd livebuild
          chmod +x auto/* config/hooks/* || true
          lb config
          lb build
${upload("livebuild/*.iso")}`;

  if (engine === "mkarchiso")
    return `${head}    container:
      image: archlinux:base-devel
      options: --privileged
    steps:
      - name: Checkout
        uses: actions/checkout@v4
      - name: Instalar archiso
        run: pacman -Syu --noconfirm archiso
      - name: Construir imagen
        run: |
          cp -r /usr/share/archiso/configs/releng /tmp/profile
          cp archiso/packages.x86_64 /tmp/profile/packages.x86_64
          cp archiso/pacman.conf /tmp/profile/pacman.conf
          cp -a archiso/airootfs/. /tmp/profile/airootfs/
          if [ -f /tmp/profile/grub/grub.cfg ]; then
            sed -i "s/^set timeout.*/set timeout=${cfg.grub?.timeout ?? 10}/" /tmp/profile/grub/grub.cfg${grubExtra ? `\n            sed -i "/^[[:space:]]*linux /s/archisolabel=[^ ]*/& ${grubExtra}/" /tmp/profile/grub/grub.cfg` : ""}
          fi
          mkdir -p /tmp/work out
          mkarchiso -v -w /tmp/work -o out /tmp/profile
${upload("out/*.iso")}`;

  return `${head}    container:
      image: fedora:latest
      options: --privileged
    steps:
      - name: Checkout
        uses: actions/checkout@v4
      - name: Instalar livemedia-creator
        run: dnf install -y livemedia-creator
      - name: Construir imagen
        run: livemedia-creator --no-virt --make-iso --ks fedora/ks.cfg --releasever 41 --iso-only --iso-name ${cfg.name}.iso --resultdir out
${upload(`out/${cfg.name}.iso`)}`;
}

function readmeMd(cfg: DistroConfig): string {
  const b = baseById(cfg.base);
  const dir = {
    kiwi: "kiwi",
    livebuild: "livebuild",
    mkarchiso: "archiso",
    kickstart: "fedora",
    manual: "manual",
  }[b.engine];
  const desk = b.desktops.find((d) => d.id === cfg.desktop)?.label ?? "ninguno";
  const repos = effectiveRepos(cfg);
  const archNote =
    b.engine === "mkarchiso"
      ? "\nNota Arch: usuario, contraseña y zona horaria del live los gestiona el perfil releng de archiso; ajústalos tras arrancar o editando la receta.\n"
      : "";
  const flatpakNote =
    b.engine === "mkarchiso" && cfg.flatpakApps?.length
      ? "\nNota Flatpak: en la ISO Arch las apps Flathub se instalan en el primer arranque (servicio distroforge-flatpak); necesita red.\n"
      : "";
  return `# ${cfg.name}

Distro personalizada generada con DistroForge.

- Base: ${b.label} (motor: ${b.engine})
- Escritorio: ${desk}
- Paquetes extra: ${cfg.packages.length ? cfg.packages.join(", ") : "ninguno"}
- Flatpak: ${cfg.flatpakApps?.length ? cfg.flatpakApps.join(", ") : "ninguno"}
- Ficheros extra: ${cfg.extraFiles?.length ?? 0}
- Usuarios extra: ${cfg.users?.length ?? 0} · Servicios systemd: ${cfg.services?.length ?? 0}${cfg.autologin ? " · auto-login" : ""}
- Arquitectura: ${cfg.arch} · GRUB: timeout ${cfg.grub?.timeout ?? 10}s${cfg.grub?.extraParams ? ` · kernel: ${cfg.grub.extraParams}` : ""} · Firewall: ${cfg.firewall ? "habilitado (firewalld)" : "deshabilitado"}
- Red: ${cfg.network?.mode === "static" ? `estática (${cfg.network.ip}/${cfg.network.prefix})` : "DHCP"}

> Este proyecto incluye \`distroforge.json\`: puedes reimportarlo en DistroForge (Cargar config) o editarlo en git y volver a construir.
- Repos custom: ${repos.length ? repos.map((r) => r.url).join(", ") : "ninguno"}
- Salida: ${cfg.output.toUpperCase()}${cfg.cloudInit ? " + cloud-init" : ""}
- Wallpaper: ${cfg.wallpaper ? "sí" : "no"}
${cfg.output === "oci" ? "\nSalida OCI: construye con `bash build.sh` (docker o podman) o el workflow de CI; carga el .tar con `docker load -i`.\n" : ""}${cfg.output === "wsl" ? `\nSalida WSL: importa el tar en Windows con:\n\n    wsl --import ${cfg.name} C:\\WSL\\${cfg.name} <ruta>\\out\\${cfg.name}-wsl.tar\n\nLuego \`wsl -d ${cfg.name}\`. Usuario por defecto configurado vía /etc/wsl.conf.\n` : ""}${cfg.wallpaper ? "\nWallpaper: se copia a /usr/share/backgrounds y se aplica vía dconf (GNOME/MATE/Cinnamon/Budgie) o xfconf (Xfce). En KDE, i3, Sway y Pantheon solo se copia el fichero.\n" : ""}
${b.engine === "manual" ? "\n> Motor **manual**: sin builder automático; el zip trae lista de paquetes y guía.\n" : ""}
${archNote}
${flatpakNote}
## Construir la imagen

**Opción A — GitHub Actions (recomendado):** sube esta carpeta a un repo nuevo de GitHub. En *Actions → build → Run workflow*. El artefacto contendrá la imagen.

**Opción B — local (Linux o WSL2 con Podman):**

    bash build.sh

## Estructura

- \`${dir}/\` — receta de construcción del motor
- \`build.sh\` — build local con Podman
- \`.github/workflows/build.yml\` — build en CI

## Seguridad

La contraseña del usuario va en texto plano dentro de la receta (\`${dir}/\`). No publiques el repo sin cambiarla o moverla a secretos de CI.
`;
}
