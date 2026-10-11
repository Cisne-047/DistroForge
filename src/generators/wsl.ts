import type { DistroConfig } from "../types";
import { baseById, desktopPackages, effectiveRepos } from "../catalog";

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
  const pkgs = [...desktopPackages(cfg.base, cfg.desktop), ...cfg.packages];
  const repos = effectiveRepos(cfg);
  const engine = base.engine;

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
    const repoLines = repos
      .map((r) => {
        const src = r.suite
          ? `deb [trusted=yes] ${r.url} ${r.suite}`
          : `deb [trusted=yes] ${r.url} ./`;
        return `cat > /rootfs/etc/apt/sources.list.d/distroforge-${r.alias || "custom"}.list <<'EOF'\n${src}\nEOF`;
      })
      .join("\n");
    script = `#!/usr/bin/env bash
set -euxo pipefail
# DistroForge - rootfs WSL para ${base.label} (debootstrap)
command -v debootstrap >/dev/null || { apt-get update; apt-get install -y debootstrap; }
rm -rf /rootfs && mkdir -p /rootfs
debootstrap --variant=minbase ${distro} /rootfs ${mirror}
${repoLines}
chroot /rootfs apt-get update
chroot /rootfs env DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends sudo ${pkgs.join(" ")}
chroot /rootfs useradd -m -s /bin/bash -G sudo ${sys.username}
chroot /rootfs bash -c "echo ${sq(`${sys.username}:${sys.password}`)} | chpasswd"
${confLines}
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
${confLines}
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
${confLines}
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
${confLines}
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
