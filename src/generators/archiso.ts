import type { DistroConfig } from "../types";
import { baseById, desktopPackages, effectiveRepos } from "../catalog";
import { sanitizeAlias } from "./livebuild";
import { CLOUD_INIT_PACKAGE, cloudInitYaml } from "./cloudinit";
import {
  archDconfBootFiles,
  dconfDbFiles,
  dconfSchemaFor,
  wallpaperApplies,
  wallpaperDataUrl,
  wallpaperFileName,
  wallpaperPackages,
  xfconfWallpaperXml,
} from "./branding";

export function archisoFiles(cfg: DistroConfig): Record<string, string> {
  const base = baseById(cfg.base);
  const sys = cfg.system;

  const packages = [
    "base",
    base.kernelPackage ?? "linux",
    "linux-firmware",
    "networkmanager",
    "sudo",
    ...(cfg.cloudInit ? [CLOUD_INIT_PACKAGE] : []),
    ...wallpaperPackages(cfg, "pacman"),
    ...desktopPackages(cfg.base, cfg.desktop),
    ...cfg.packages,
  ].join("\n");

  const repoSections = effectiveRepos(cfg)
    .map(
      (r, i) =>
        `[${sanitizeAlias(r.alias, i)}]\nSigLevel = Optional TrustAll\nServer = ${r.url}`
    )
    .join("\n\n");

  const pacmanConf = `[options]
HoldPkg = pacman glibc
Architecture = auto
CheckSpace
SigLevel = Required DatabaseOptional
LocalFileSigLevel = Optional
ParallelDownloads = 5

[core]
Server = ${base.mirror}

[extra]
Server = ${base.mirror}
${repoSections ? "\n" + repoSections + "\n" : ""}`;

  const buildSh = `#!/usr/bin/env bash
set -euo pipefail
# DistroForge - build local (Linux o WSL2, requiere Podman)
# Parte del perfil releng de archiso y aplica tu capa: paquetes, repos y airootfs.
DIR="$(cd "$(dirname "$0")" && pwd)"
podman run --rm --privileged -v "$DIR":/project -v "$DIR/out":/out archlinux:base-devel bash -ec '
  pacman -Syu --noconfirm archiso
  cp -r /usr/share/archiso/configs/releng /tmp/profile
  cp /project/archiso/packages.x86_64 /tmp/profile/packages.x86_64
  cp /project/archiso/pacman.conf /tmp/profile/pacman.conf
  cp -a /project/archiso/airootfs/. /tmp/profile/airootfs/
  mkdir -p /tmp/work
  mkarchiso -v -w /tmp/work -o /out /tmp/profile
'
echo "Imagen lista en ./out"
`;

  const wallpaperFiles: Record<string, string> = {};
  if (wallpaperApplies(cfg)) {
    const wf = wallpaperFileName(cfg)!;
    wallpaperFiles[`archiso/airootfs/usr/share/backgrounds/${wf}`] = wallpaperDataUrl(cfg)!;
    if (dconfSchemaFor(cfg.desktop)) {
      for (const [p, c] of Object.entries(dconfDbFiles(cfg)))
        wallpaperFiles[`archiso/airootfs${p}`] = c;
      for (const [p, c] of Object.entries(archDconfBootFiles()))
        wallpaperFiles[`archiso/airootfs${p}`] = c;
    }
    if (cfg.desktop === "xfce")
      wallpaperFiles[
        "archiso/airootfs/etc/xdg/xfce4/xfconf/xfce-perchannel-xml/xfce4-desktop.xml"
      ] = xfconfWallpaperXml(cfg);
  }

  return {
    "archiso/packages.x86_64": packages + "\n",
    "archiso/pacman.conf": pacmanConf,
    "archiso/airootfs/etc/hostname": sys.hostname + "\n",
    "archiso/airootfs/etc/locale.conf": `LANG=${sys.locale}\n`,
    "archiso/airootfs/etc/vconsole.conf": `KEYMAP=${sys.keyboard}\n`,
    "archiso/airootfs/etc/motd": cfg.motd + "\n",
    ...(cfg.cloudInit
      ? { "archiso/airootfs/etc/cloud/cloud.cfg.d/80-distroforge.cfg": cloudInitYaml(cfg) }
      : {}),
    ...wallpaperFiles,
    "build.sh": buildSh,
  };
}
