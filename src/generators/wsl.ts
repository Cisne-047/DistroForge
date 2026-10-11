import type { DistroConfig } from "../types";
import { baseById, DEB_ARCH, desktopPackages, effectiveRepos } from "../catalog";
import { flatpakPackages, flatpakShellLines } from "./flatpak";
import { dirnameOf } from "./kickstart";
import { sanitizeAlias } from "./livebuild";

function sq(s: string): string {
  return `'${s.replace(/'/g, `'\\''`)}'`;
}

function wslConf(cfg: DistroConfig): string {
  return `[automount]
enabled = true

[user]
default = ${cfg.system.username}

[boot]
systemd = true`;
}

export function wslFiles(cfg: DistroConfig): Record<string, string> {
  const base = baseById(cfg.base);
  const sys = cfg.system;
  const name = cfg.name;
  const pkgs = [...desktopPackages(cfg.base, cfg.desktop), ...flatpakPackages(cfg), ...cfg.packages];
  const repos = effectiveRepos(cfg);
  const engine = base.engine;

  const extraFilesBlock = (cfg.extraFiles ?? [])
    .map((f) => {
      const dir = dirnameOf(f.path);
      return `mkdir -p /rootfs${dir}\ncat > /rootfs${f.path} <<'DFEOF'\n${f.content}\nDFEOF`;
    })
    .join("\n");
  const flatpakBlock = flatpakShellLines(cfg, "chroot /rootfs ").join("\n");
  const wslUserLines = (cfg.users ?? [])
    .map(
      (u) =>
        `chroot /rootfs id "${u.name}" >/dev/null 2>&1 || chroot /rootfs useradd -m -s /bin/bash${u.admin ? " -G wheel" : ""} "${u.name}"
chroot /rootfs bash -c "echo ${sq(`${u.name}:${u.password}`)} | chpasswd"`
    )
    .join("\n");
  const wslServiceLines = (cfg.services ?? [])
    .filter((s) => /^[A-Za-z0-9@._-]+$/.test(s.unit))
    .map((s) => `chroot /rootfs systemctl ${s.enabled ? "enable" : "disable"} "${s.unit}"`)
    .join("\n");
  const preConf = [extraFilesBlock, flatpakBlock, wslUserLines, wslServiceLines]
    .filter(Boolean)
    .join("\n");

  const confLines = [
    `cat > /rootfs/etc/wsl.conf <<'WSLEOF'`,
    wslConf(cfg),
    `WSLEOF`,
    `echo "${sys.hostname}" > /rootfs/etc/hostname`,
    `ln -sf /usr/share/zoneinfo/${sys.timezone} /rootfs/etc/localtime`,
  ].join("\n");

  let script: string;
  if (engine === "livebuild") {
    const distro = base.distribution ?? "bookworm";
    const mirror = base.mirror || "http://deb.debian.org/debian";
    const needKey = repos.some((r) => r.gpgKey);
    const ensureBootstrap = needKey
      ? "apt-get update\napt-get install -y debootstrap wget ca-certificates"
      : 'command -v debootstrap >/dev/null || { apt-get update; apt-get install -y debootstrap; }';
    const repoLines = repos
      .map((r, i) => {
        const alias = sanitizeAlias(r.alias, i);
        const signed = r.gpgKey
          ? `[signed-by=/usr/share/keyrings/${alias}.asc]`
          : "[trusted=yes]";
        const src = r.suite ? `deb ${signed} ${r.url} ${r.suite}` : `deb ${signed} ${r.url} ./`;
        const keyLine = r.gpgKey
          ? `mkdir -p /rootfs/usr/share/keyrings\nwget -qO /rootfs/usr/share/keyrings/${alias}.asc ${r.gpgKey}\n`
          : "";
        return `${keyLine}cat > /rootfs/etc/apt/sources.list.d/distroforge-${alias}.list <<'EOF'\n${src}\nEOF`;
      })
      .join("\n");
    script = `#!/usr/bin/env bash
set -euxo pipefail
# DistroForge - rootfs WSL para ${base.label} (debootstrap)
${ensureBootstrap}
rm -rf /rootfs && mkdir -p /rootfs
debootstrap --variant=minbase --arch=${DEB_ARCH[cfg.arch] ?? "amd64"} ${distro} /rootfs ${mirror}
${repoLines}
chroot /rootfs apt-get update
chroot /rootfs env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends sudo ${pkgs.join(" ")}
chroot /rootfs useradd -m -s /bin/bash -G sudo ${sys.username}
chroot /rootfs bash -c "echo ${sq(`${sys.username}:${sys.password}`)} | chpasswd"
${preConf ? preConf + "\n" : ""}${confLines}
mkdir -p out
tar -C /rootfs -cpf out/${name}-wsl.tar .
echo "Rootfs WSL listo en ./out/${name}-wsl.tar"
`;
  } else if (engine === "mkarchiso") {
    const repoLines = repos
      .map(
        (r) =>
          `printf '[${r.alias}]\\nSigLevel = Optional TrustAll\\nServer = ${r.url}\\n' >> /etc/pacman.conf`
      )
      .join("\n");
    script = `#!/usr/bin/env bash
set -euxo pipefail
# DistroForge - rootfs WSL para ${base.label} (pacstrap)
pacman -Sy --noconfirm arch-install-scripts
rm -rf /rootfs && mkdir -p /rootfs
${repoLines}
pacstrap -c /rootfs base sudo ${pkgs.join(" ")}
arch-chroot /rootfs useradd -m -s /bin/bash -G wheel ${sys.username}
arch-chroot /rootfs bash -c "echo ${sq(`${sys.username}:${sys.password}`)} | chpasswd"
${preConf ? preConf + "\n" : ""}${confLines}
mkdir -p out
tar -C /rootfs -cpf out/${name}-wsl.tar .
echo "Rootfs WSL listo en ./out/${name}-wsl.tar"
`;
  } else if (engine === "kiwi") {
    const repoLines = repos
      .map((r) => `zypper --installroot /rootfs ar -f ${r.url} ${r.alias}`)
      .join("\n");
    script = `#!/usr/bin/env bash
set -euxo pipefail
# DistroForge - rootfs WSL para ${base.label} (zypper installroot)
rm -rf /rootfs && mkdir -p /rootfs
zypper --installroot /rootfs ar -f ${base.mirror} distroforge-base
${repoLines}
zypper --installroot /rootfs --no-gpg-checks -n install filesystem zypper sudo ${pkgs.join(" ")}
chroot /rootfs useradd -m -s /bin/bash -G users ${sys.username}
chroot /rootfs bash -c "echo ${sq(`${sys.username}:${sys.password}`)} | chpasswd"
${preConf ? preConf + "\n" : ""}${confLines}
mkdir -p out
tar -C /rootfs -cpf out/${name}-wsl.tar .
echo "Rootfs WSL listo en ./out/${name}-wsl.tar"
`;
  } else {
    script = `#!/usr/bin/env bash
set -euxo pipefail
# DistroForge - rootfs WSL para ${base.label} (dnf installroot)
rm -rf /rootfs && mkdir -p /rootfs
dnf install -y --installroot=/rootfs --releasever=41 --setopt=install_weak_deps=False --nodocs @core sudo ${pkgs.join(" ")}
chroot /rootfs useradd -m -s /bin/bash -G wheel ${sys.username}
chroot /rootfs bash -c "echo ${sq(`${sys.username}:${sys.password}`)} | chpasswd"
${preConf ? preConf + "\n" : ""}${confLines}
mkdir -p out
tar -C /rootfs -cpf out/${name}-wsl.tar .
echo "Rootfs WSL listo en ./out/${name}-wsl.tar"
`;
  }

  const container =
    engine === "livebuild"
      ? base.container ?? "debian:bookworm"
      : engine === "mkarchiso"
        ? "archlinux:base-devel"
        : engine === "kiwi"
          ? "opensuse/tumbleweed"
          : "fedora:latest";

  const buildSh = `#!/usr/bin/env bash
set -euo pipefail
# DistroForge - build rootfs WSL (Linux o WSL2, requiere docker o podman)
DIR="$(cd "$(dirname "$0")" && pwd)"
podman run --rm --privileged -v "$DIR":/project -w /project ${container} bash wsl/rootfs.sh
echo "Importar en Windows: wsl --import ${name} C:\\WSL\\${name} <ruta>\\out\\${name}-wsl.tar"
`;

  return {
    "wsl/rootfs.sh": script,
    "build.sh": buildSh,
  };
}
