import JSZip from "jszip";
import type { DistroConfig } from "../types";
import { baseById, effectiveRepos, archsForEngine } from "../catalog";
import { IPV4_RE } from "./network";
import { LOGO_RE } from "./branding";
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
    ...(cfg.testDrive && cfg.output === "iso" && !cfg.advanced
      ? { ".github/workflows/testdrive.yml": testDriveYaml(cfg) }
      : {}),
    ...(cfg.cloudUpload !== "none" && !cfg.advanced
      ? { ".github/workflows/cloud-upload.yml": cloudUploadYaml(cfg) }
      : {}),
    ...(cfg.smokeTest && cfg.output === "iso" && !cfg.advanced
      ? { ".github/workflows/smoke.yml": smokeYaml(cfg) }
      : {}),
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
  if (["qcow2", "usb", "vmdk", "vdi", "vhd"].includes(cfg.output) && baseById(cfg.base).engine !== "kiwi")
    out.push("QCOW2/USB/VMDK/VDI/VHD solo disponibles con motor KIWI (bases openSUSE).");
  if (cfg.unattended && !["livebuild", "kiwi"].includes(baseById(cfg.base).engine))
    out.push("Desatendida: preseed (Debian) o AutoYaST/oem-unattended (openSUSE); el kickstart de Fedora ya es desatendido de serie.");
  if (cfg.logo && !LOGO_RE.test(cfg.logo))
    out.push("Logo: formato no soportado (usa png, jpeg o webp).");
  if (cfg.testDrive && cfg.output !== "iso")
    out.push("Test Drive: las capturas del arranque solo aplican a la salida ISO.");
  if (cfg.cloudUpload !== "none" && !["qcow2", "usb", "vmdk", "vdi", "vhd"].includes(cfg.output))
    out.push("Nube: la subida AWS/Azure necesita una imagen de disco (qcow2/usb/vmdk/vdi/vhd).");
  if (cfg.smokeTest && (cfg.output !== "iso" || cfg.advanced))
    out.push("Smoke test: solo aplica a la salida ISO (sin modo advanced).");
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
      (cfg.autostart?.length ?? 0) > 0 && "autostart",
      cfg.logo && "logo",
      cfg.eulaText && "EULA",
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

/** Estimación grosera del tamaño de la imagen final, en MB. Solo orientativa:
 *  base mínima + escritorio + paquetes + flatpaks; el peso real depende de cada
 *  familia y del motor. Sirve para que nadie se descargue 4 GB pensando en 1. */
export function estimateSizeMB(cfg: DistroConfig): number {
  let mb = 1100; // sistema base mínimo instalable
  const desktop = baseById(cfg.base).desktops.find((d) => d.id === cfg.desktop);
  if (desktop && desktop.id !== "none") mb += 1400 + desktop.packages.length * 60;
  mb += cfg.packages.length * 45;
  mb += (cfg.flatpakApps?.length ?? 0) * 260;
  if (cfg.cloudInit) mb += 40;
  if (cfg.wallpaper || cfg.logo) mb += 6;
  return Math.round(mb);
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

/** Smoke test: arranca la ISO en QEMU headless dentro de CI y espera el login
 *  por consola serie. Para ver el arranque en el log añade `console=ttyS0` a los
 *  parámetros del kernel en Branding. */
function smokeYaml(cfg: DistroConfig): string {
  return `name: smoke
on:
  workflow_dispatch:
jobs:
  boot:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4
      - name: QEMU boot test
        run: |
          sudo apt-get update && sudo apt-get install -y qemu-system-x86
          ISO=$(find . -name '*.iso' | head -n1)
          [ -n "$ISO" ] || { echo "No hay ISO que probar"; exit 1; }
          timeout 180 qemu-system-x86_64 -m 2048 -no-reboot -display none             -cdrom "$ISO" -serial file:serial.log || true
          if grep -qE 'login:|Welcome to|Gracefully powering off' serial.log; then
            echo "Smoke OK: la ISO arrancó"
          else
            echo "No se vio el login en el log serie (añade console=ttyS0 a los parámetros del kernel)"
            tail -n 50 serial.log || true
            exit 1
          fi
      - name: Log del arranque
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: serial-log-${cfg.name}
          path: serial.log
          if-no-files-found: ignore
`;
}

/** Test Drive: arranca la ISO en QEMU y captura pantallas del arranque cada
 *  12 s (consola VGA). El artefacto es un "paseo" visual por tu distro sin
 *  descargar nada. */
function testDriveYaml(cfg: DistroConfig): string {
  return `name: testdrive
on:
  workflow_dispatch:
jobs:
  drive:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4
      - name: Capturas del arranque
        run: |
          sudo apt-get update && sudo apt-get install -y qemu-system-x86 netpbm socat
          ISO=$(find . -name '*.iso' | head -n1)
          [ -n "$ISO" ] || { echo "No hay ISO"; exit 1; }
          mkdir -p shots
          qemu-system-x86_64 -m 2048 -no-reboot -display none -cdrom "$ISO" \\
            -monitor unix:qemu.mon,server,nowait &
          QPID=$!
          for i in 1 2 3 4 5 6 7 8; do
            sleep 12
            echo "screendump shots/shot-\${i}.ppm" | socat - UNIX-CONNECT:qemu.mon || true
          done
          kill $QPID 2>/dev/null || true
          for f in shots/*.ppm; do [ -f "$f" ] && pnmtopng "$f" > "\${f%.ppm}.png" && rm "$f"; done || true
          ls shots/*.png >/dev/null 2>&1 || { echo "Sin capturas"; exit 1; }
      - name: Subir capturas
        if: always()
        uses: actions/upload-artifact@v4
        with:
          name: ${cfg.name}-testdrive
          path: shots/
          if-no-files-found: error
`;
}

/** Subida a nube: AWS (S3 + import a AMI) o Azure (VHD en blob). Requiere
 *  secrets en el repo del proyecto; si faltan, avisa y termina bien. */
function cloudUploadYaml(cfg: DistroConfig): string {
  const aws = cfg.cloudUpload === "aws";
  const envBlock = aws
    ? `          AWS_ACCESS_KEY_ID: \${{ secrets.AWS_ACCESS_KEY_ID }}
          AWS_SECRET_ACCESS_KEY: \${{ secrets.AWS_SECRET_ACCESS_KEY }}
          AWS_REGION: \${{ secrets.AWS_REGION }}
          BUCKET: \${{ secrets.AWS_BUCKET }}`
    : `          AZ_STORAGE: \${{ secrets.AZ_STORAGE }}
          AZ_KEY: \${{ secrets.AZ_KEY }}
          AZ_CONTAINER: \${{ secrets.AZ_CONTAINER }}`;
  const runBlock = aws
    ? `          if [ -z "\${{ secrets.AWS_ACCESS_KEY_ID }}" ]; then echo "Faltan secrets AWS_*; configura el repo"; exit 0; fi
          aws s3 cp "$DISK" "s3://\${{ secrets.AWS_BUCKET }}/${cfg.name}" || { pip install awscli && aws s3 cp "$DISK" "s3://\${{ secrets.AWS_BUCKET }}/${cfg.name}"; }
          echo "Despues registra la AMI con import-snapshot apuntando a s3://\${{ secrets.AWS_BUCKET }}/${cfg.name}"`
    : `          if [ -z "\${{ secrets.AZ_STORAGE }}" ]; then echo "Faltan secrets AZ_*; configura el repo"; exit 0; fi
          pip install azure-cli >/dev/null 2>&1 || true
          az storage blob upload --account-name "\${{ secrets.AZ_STORAGE }}" --account-key "\${{ secrets.AZ_KEY }}" -c "\${{ secrets.AZ_CONTAINER }}" -f "$DISK" -n "${cfg.name}.vhd"
          echo "Crea la imagen de Azure desde el blob ${cfg.name}.vhd"`;
  return `name: cloud-upload
on:
  workflow_dispatch:
jobs:
  ${aws ? "aws" : "azure"}:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4
      - name: Descargar artefacto del build
        uses: dawidd6/action-download-artifact@v6
        with:
          workflow: build.yml
          name: ${cfg.name}-image
          path: image
      - name: ${aws ? "Registrar AMI" : "Subir VHD a Azure"}
        env:
${envBlock}
        run: |
          DISK=$(find image -type f | head -n1)
          [ -n "$DISK" ] || { echo "No hay imagen de disco"; exit 1; }
${runBlock}
`;
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
${cfg.signImage ? `      - name: Firmar imagen (cosign keyless)
        uses: sigstore/cosign-installer@v3
      - name: Cosign sign
        run: cosign sign --yes ${cfg.name}:1.0.0
` : ""}      - name: Subir artefacto
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
- Logo: ${cfg.logo ? "sí (copiado a /usr/share/distroforge + greeter LightDM)" : "no"} · EULA: ${cfg.eulaText ? "sí (primer login)" : "no"}${cfg.unattended ? " · instalación desatendida (preseed)" : ""}${cfg.smokeTest ? " · smoke test de arranque en CI (smoke.yml)" : ""}${cfg.signImage ? " · imagen firmada con cosign" : ""}

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
