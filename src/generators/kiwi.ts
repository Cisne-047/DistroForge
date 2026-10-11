import type { DistroConfig } from "../types";
import { baseById, desktopPackages, effectiveRepos } from "../catalog";
import { CLOUD_INIT_PACKAGE, cloudInitYaml } from "./cloudinit";
import {
  dconfDbFiles,
  dconfSchemaFor,
  wallpaperApplies,
  wallpaperDataUrl,
  wallpaperFileName,
  wallpaperPackages,
  xfconfWallpaperXml,
} from "./branding";

function esc(s: string): string {
  return s
    .replace(/&/g, "&amp;")
    .replace(/</g, "&lt;")
    .replace(/>/g, "&gt;")
    .replace(/"/g, "&quot;");
}

export function kiwiFiles(cfg: DistroConfig): Record<string, string> {
  const base = baseById(cfg.base);
  const sys = cfg.system;
  const withDesktop = desktopPackages(cfg.base, cfg.desktop).length > 0;
  const typeTag =
    cfg.output === "qcow2"
      ? `<type image="qcow2" filesystem="btrfs" primary="true"/>`
      : `<type image="iso" primary="true" firmware="uefi"/>`;

  const repoTags = [
    `    <repository type="rpm-md" alias="${esc(base.id)}" url="${esc(base.mirror)}"/>`,
    ...effectiveRepos(cfg).map(
        (r) =>
          `    <repository type="rpm-md" alias="${esc(r.alias)}" url="${esc(r.url)}"${r.gpgKey ? "" : ` gpgcheck="false"`}/>`
      ),
  ].join("\n");

  const pkgs = [
    base.kernelPackage ?? "kernel-default",
    "grub2",
    "dracut",
    "patterns-base-base",
    ...(cfg.cloudInit ? [CLOUD_INIT_PACKAGE] : []),
    ...wallpaperPackages(cfg, "zypper"),
    ...desktopPackages(cfg.base, cfg.desktop),
    ...cfg.packages,
  ]
    .map((p) => `    <package name="${esc(p)}"/>`)
    .join("\n");

  const configXml = `<?xml version="1.0" encoding="utf-8"?>
<image schemaversion="9.2" name="${esc(cfg.name)}">
  <description type="system">
    <author>${esc(sys.username)}</author>
    <contact>${esc(sys.username)}@localhost</contact>
    <specification>${esc(cfg.name)} - distro personalizada generada con DistroForge</specification>
  </description>
  <preferences>
    <version>1.0.0</version>
    <packagemanager>zypper</packagemanager>
    <locale>${esc(sys.locale)}</locale>
    <keytable>${esc(sys.keyboard)}</keytable>
    <timezone>${esc(sys.timezone)}</timezone>
    <rpm-excludedocs>true</rpm-excludedocs>
    ${typeTag}
  </preferences>
  <users>
    <user name="${esc(sys.username)}" password="${esc(sys.password)}" home="/home/${esc(sys.username)}" groups="users"/>
  </users>
${repoTags}
  <packages type="image">
${pkgs}
  </packages>
  <packages type="bootstrap">
    <package name="filesystem"/>
  </packages>
</image>
`;

  const shLines = [
    "#!/bin/bash",
    "set -euo pipefail",
    "# DistroForge - configuracion post-instalacion (se ejecuta en chroot)",
    `systemctl set-default ${withDesktop ? "graphical" : "multi-user"}.target`,
    `echo "${sys.username} ALL=(ALL) ALL" > /etc/sudoers.d/50-${sys.username}`,
  ];
  if (cfg.motd) shLines.push("cat > /etc/motd <<'MOTDEOF'", cfg.motd, "MOTDEOF");
  if (cfg.cloudInit)
    shLines.push(
      "mkdir -p /etc/cloud/cloud.cfg.d",
      "cat > /etc/cloud/cloud.cfg.d/80-distroforge.cfg <<'CIEOF'",
      cloudInitYaml(cfg),
      "CIEOF"
    );
  if (wallpaperApplies(cfg) && dconfSchemaFor(cfg.desktop))
    shLines.push("dconf update || true");
  const configSh = shLines.join("\n") + "\n";

  const wallpaperFiles: Record<string, string> = {};
  if (wallpaperApplies(cfg)) {
    const wf = wallpaperFileName(cfg)!;
    wallpaperFiles[`kiwi/root/usr/share/backgrounds/${wf}`] = wallpaperDataUrl(cfg)!;
    for (const [p, c] of Object.entries(dconfDbFiles(cfg)))
      wallpaperFiles[`kiwi/root${p}`] = c;
    if (cfg.desktop === "xfce")
      wallpaperFiles[
        "kiwi/root/etc/xdg/xfce4/xfconf/xfce-perchannel-xml/xfce4-desktop.xml"
      ] = xfconfWallpaperXml(cfg);
  }

  const buildSh = `#!/usr/bin/env bash
set -euo pipefail
# DistroForge - build local (Linux o WSL2, requiere Podman)
DIR="$(cd "$(dirname "$0")" && pwd)"
podman run --rm --privileged -v "$DIR":/project -v "$DIR/out":/out \\
  registry.opensuse.org/opensuse/tumbleweed:latest bash -ec '
    zypper --gpg-auto-import-keys --non-interactive install \\
      python3-kiwi kiwi-systemdeps-core kiwi-systemdeps-iso-media kiwi-systemdeps-disk-images
    kiwi-ng system build --description /project/kiwi --target-dir /out
  '
echo "Imagen lista en ./out"
`;

  return {
    "kiwi/config.xml": configXml,
    "kiwi/config.sh": configSh,
    ...wallpaperFiles,
    "build.sh": buildSh,
  };
}
