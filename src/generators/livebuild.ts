import type { DistroConfig } from "../types";
import { baseById, desktopPackages, DEB_ARCH, effectiveRepos } from "../catalog";
import { CLOUD_INIT_PACKAGE, cloudInitYaml } from "./cloudinit";
import { flatpakPackages, flatpakShellLines, hasFlatpakApps } from "./flatpak";
import { autologinFiles, extraUserShellLines, firewallServiceLine, serviceShellLines } from "./accounts";
import { nmConnection, staticNetwork } from "./network";
import {
  dconfDbFiles,
  dconfSchemaFor,
  wallpaperApplies,
  wallpaperDataUrl,
  wallpaperFileName,
  wallpaperPackages,
  xfconfWallpaperXml,
} from "./branding";

export function sanitizeAlias(a: string, i: number): string {
  const s = a.replace(/[^a-zA-Z0-9._-]/g, "_").replace(/^[._-]+/, "");
  return s || `repo-${i + 1}`;
}

function dq(s: string): string {
  return s
    .replace(/\\/g, "\\\\")
    .replace(/"/g, '\\"')
    .replace(/\$/g, "\\$")
    .replace(/`/g, "\\`");
}

export function livebuildFiles(cfg: DistroConfig): Record<string, string> {
  const base = baseById(cfg.base);
  const sys = cfg.system;
  const distro = base.distribution ?? "bookworm";
  const areas = base.archiveAreas ?? "main contrib non-free non-free-firmware";
  const mirror = base.mirror || "http://deb.debian.org/debian";
  const kernel = base.kernelPackage ?? "linux-image-amd64";
  const flavour = base.lbFlavour ?? "amd64";
  const container = base.container ?? "debian:bookworm";
  const repos = effectiveRepos(cfg);

  const lbArch = DEB_ARCH[cfg.arch] ?? "amd64";
  const extraParams = (cfg.grub?.extraParams ?? "").replace(/["`$]/g, "").trim();
  const bootAppend = `boot=live components locales=${sys.locale} keyboard-layouts=${sys.keyboard} timezone=${sys.timezone} hostname=${sys.hostname} username=${sys.username}${extraParams ? " " + extraParams : ""}`;

  const autoConfig = `#!/bin/sh
set -e
# DistroForge - configuracion live-build (${base.label})
lb config noauto \\
    --architectures ${lbArch} \\
    --distribution ${distro} \\
    --archive-areas "${areas}" \\
    --mirror-bootstrap ${mirror} \\
    --mirror-binary ${mirror} \\
    --binary-images iso-hybrid \\
    --linux-packages "linux-image" \\
    --linux-flavours ${flavour} \\
    --iso-volume "${cfg.name.slice(0, 28)}" \\
    --grub-timeout ${cfg.grub?.timeout ?? 10} \\
    --bootappend-live "${bootAppend}" \\
    "$@"
`;

  const autoBuild = `#!/bin/sh
set -e
lb build noauto "$@" 2>&1 | tee build.log
`;

  const userHook = `#!/bin/sh
set -e
# DistroForge - usuario y contrasena del live
id "${sys.username}" >/dev/null 2>&1 || useradd -m -s /bin/bash -G sudo "${sys.username}"
echo "${dq(sys.username)}:${dq(sys.password)}" | chpasswd
usermod -aG sudo "${sys.username}" || true
${extraUserShellLines(cfg, "sudo").join("\n")}
`;

  const repoHooks = repos.map((r, i) => {
    const alias = sanitizeAlias(r.alias, i);
    const keyLines = r.gpgKey
      ? `wget -qO /usr/share/keyrings/${alias}.asc "${r.gpgKey}"
SIGNED="[signed-by=/usr/share/keyrings/${alias}.asc]"`
      : `SIGNED="[trusted=yes]"`;
    const debLine = r.suite
      ? `echo "deb $SIGNED ${r.url} ${r.suite}" > /etc/apt/sources.list.d/distroforge-${alias}.list`
      : `echo "deb $SIGNED ${r.url} ./" > /etc/apt/sources.list.d/distroforge-${alias}.list`;
    return `#!/bin/sh
set -e
# DistroForge - repo custom: ${alias}
mkdir -p /usr/share/keyrings
${keyLines}
${debLine}
`;
  });

  const pkgList = [
    kernel,
    "live-boot",
    "live-config",
    "systemd",
    "sudo",
    "wget",
    "ca-certificates",
    ...(cfg.cloudInit ? [CLOUD_INIT_PACKAGE] : []),
    ...(cfg.firewall ? ["firewalld"] : []),
    ...wallpaperPackages(cfg, "apt"),
    ...flatpakPackages(cfg),
    ...desktopPackages(cfg.base, cfg.desktop),
    ...cfg.packages,
  ].join("\n");

  const buildSh = `#!/usr/bin/env bash
set -euo pipefail
# DistroForge - build local (Linux o WSL2, requiere Podman)
DIR="$(cd "$(dirname "$0")" && pwd)"
podman run --rm --privileged -v "$DIR":/project -w /project/livebuild ${container} bash -ec '
  chmod +x auto/* config/hooks/* 2>/dev/null || true
  apt-get update
  apt-get install -y live-build wget ca-certificates
  lb clean >/dev/null 2>&1 || true
  lb config
  lb build
'
echo "ISO en ./livebuild/live-image-amd64.hybrid.iso"
`;

  const flatpakHook = hasFlatpakApps(cfg)
    ? `#!/bin/sh
set -e
# DistroForge - flatpak: flathub + apps
${flatpakShellLines(cfg).join("\n")}
`
    : "";

  const servicesHook = `#!/bin/sh
set -e
# DistroForge - firewall y servicios systemd
${[firewallServiceLine(cfg), ...serviceShellLines(cfg)].join("\n")}
`;

  const net = staticNetwork(cfg);
  const netFiles: Record<string, string> = net
    ? {
        "livebuild/config/includes.chroot/etc/NetworkManager/system-connections/distroforge.nmconnection":
          nmConnection(net),
        // NM ignora keyfiles legibles por el mundo: 600 en el build
        "livebuild/config/hooks/0330-distroforge-net.hooks.chroot": `#!/bin/sh
set -e
# DistroForge - permisos del keyfile de red
chmod 600 /etc/NetworkManager/system-connections/distroforge.nmconnection || true
`,
      }
    : {};

  const wallpaperFiles: Record<string, string> = {};
  if (wallpaperApplies(cfg)) {
    const wf = wallpaperFileName(cfg)!;
    wallpaperFiles[`livebuild/config/includes.chroot/usr/share/backgrounds/${wf}`] =
      wallpaperDataUrl(cfg)!;
    if (dconfSchemaFor(cfg.desktop)) {
      for (const [p, c] of Object.entries(dconfDbFiles(cfg)))
        wallpaperFiles[`livebuild/config/includes.chroot${p}`] = c;
      wallpaperFiles["livebuild/config/hooks/0300-dconf.hooks.chroot"] =
        "#!/bin/sh\nset -e\n# DistroForge - compila la base dconf\ndconf update || true\n";
    }
    if (cfg.desktop === "xfce")
      wallpaperFiles[
        "livebuild/config/includes.chroot/etc/xdg/xfce4/xfconf/xfce-perchannel-xml/xfce4-desktop.xml"
      ] = xfconfWallpaperXml(cfg);
  }

  const files: Record<string, string> = {
    "livebuild/auto/config": autoConfig,
    "livebuild/auto/build": autoBuild,
    "livebuild/config/package-lists/distroforge.list.chroot": pkgList + "\n",
    "livebuild/config/includes.chroot/etc/hostname": sys.hostname + "\n",
    "livebuild/config/includes.chroot/etc/motd": cfg.motd + "\n",
    "livebuild/config/hooks/0100-distroforge-user.hooks.chroot": userHook,
    ...(cfg.cloudInit
      ? {
          "livebuild/config/includes.chroot/etc/cloud/cloud.cfg.d/80-distroforge.cfg":
            cloudInitYaml(cfg),
        }
      : {}),
    ...Object.fromEntries(
      (cfg.extraFiles ?? []).map((f) => [`livebuild/config/includes.chroot${f.path}`, f.content])
    ),
    ...Object.fromEntries(
      Object.entries(autologinFiles(cfg)).map(([p, c]) => [
        `livebuild/config/includes.chroot${p}`,
        c,
      ])
    ),
    ...(servicesHook
      ? { "livebuild/config/hooks/0340-services.hooks.chroot": servicesHook }
      : {}),
    ...netFiles,
    ...(flatpakHook ? { "livebuild/config/hooks/0350-flatpak.hooks.chroot": flatpakHook } : {}),
    ...wallpaperFiles,
    "build.sh": buildSh,
  };
  repoHooks.forEach((hook, i) => {
    const alias = sanitizeAlias(repos[i].alias, i);
    files[`livebuild/config/hooks/02${String(i).padStart(2, "0")}-repo-${alias}.hooks.chroot`] = hook;
  });
  return files;
}
