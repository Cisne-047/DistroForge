import type { DistroConfig } from "../types";
import { baseById, desktopPackages, effectiveRepos } from "../catalog";
import { sanitizeAlias } from "./livebuild";
import { CLOUD_INIT_PACKAGE, cloudInitYaml } from "./cloudinit";
import { flatpakApps, flatpakPackages, FLATHUB, hasFlatpakApps } from "./flatpak";
import {
  autologinFiles,
  serviceExecStartLines,
  userExecStartLines,
} from "./accounts";
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
    ...flatpakPackages(cfg),
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

  const flatpakFiles: Record<string, string> = {};
  if (hasFlatpakApps(cfg)) {
    // archiso no tiene hooks de chroot: Flathub y apps se instalan al primer arranque
    flatpakFiles["archiso/airootfs/etc/systemd/system/distroforge-flatpak.service"] = `[Unit]
Description=DistroForge - configura Flathub e instala apps Flatpak
Wants=network-online.target
After=network-online.target
[Service]
Type=oneshot
ExecStart=/usr/bin/flatpak remote-add --if-not-exists flathub ${FLATHUB}
ExecStart=/usr/bin/flatpak install -y --noninteractive flathub ${flatpakApps(cfg).join(" ")}
RemainAfterExit=yes
[Install]
WantedBy=multi-user.target
`;
    flatpakFiles["archiso/airootfs/etc/tmpfiles.d/distroforge-flatpak.conf"] =
      "L /etc/systemd/system/multi-user.target.wants/distroforge-flatpak.service - - - - /etc/systemd/system/distroforge-flatpak.service\n";
  }

  // usuarios extra, servicios y auto-login: sin hooks de chroot en archiso, todo
  // se ejecuta en el primer arranque vía unidades oneshot
  const accountFiles: Record<string, string> = {};
  const userExecs = userExecStartLines(cfg, "wheel");
  if (userExecs.length)
    accountFiles["archiso/airootfs/etc/systemd/system/distroforge-users.service"] = `[Unit]
Description=DistroForge - crea los usuarios extra
[Service]
Type=oneshot
${userExecs.join("\n")}
RemainAfterExit=yes
[Install]
WantedBy=multi-user.target
`;
  const svcExecs = serviceExecStartLines(cfg);
  if (svcExecs.length)
    accountFiles["archiso/airootfs/etc/systemd/system/distroforge-services.service"] = `[Unit]
Description=DistroForge - habilita/deshabilita servicios
[Service]
Type=oneshot
${svcExecs.join("\n")}
RemainAfterExit=yes
[Install]
WantedBy=multi-user.target
`;
  for (const name of ["distroforge-users", "distroforge-services"]) {
    if (accountFiles[`archiso/airootfs/etc/systemd/system/${name}.service`])
      accountFiles[`archiso/airootfs/etc/tmpfiles.d/${name}.conf`] =
        `L /etc/systemd/system/multi-user.target.wants/${name}.service - - - - /etc/systemd/system/${name}.service\n`;
  }
  for (const [p, c] of Object.entries(autologinFiles(cfg)))
    accountFiles[`archiso/airootfs${p}`] = c;

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
    ...Object.fromEntries(
      (cfg.extraFiles ?? []).map((f) => [`archiso/airootfs${f.path}`, f.content])
    ),
    ...flatpakFiles,
    ...accountFiles,
    ...wallpaperFiles,
    "build.sh": buildSh,
  };
}
