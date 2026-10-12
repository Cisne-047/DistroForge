import type { DistroConfig } from "../types";
import { baseById, desktopPackages, effectiveRepos } from "../catalog";
import { CLOUD_INIT_PACKAGE, cloudInitYaml } from "./cloudinit";
import { flatpakPackages, flatpakShellLines, hasFlatpakApps } from "./flatpak";
import {
  autologinFiles,
  eulaFiles,
  autostartFiles,
  firewallOpenLines,
  firewallServiceLine,
  serviceShellLines,
  targetShellLine,
} from "./accounts";
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
import { logoFiles } from "./branding";

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
  const grubParams = (cfg.grub?.extraParams ?? "").replace(/["`$]/g, "").trim();
  const kernAttr = grubParams ? ` kernelcmdline="${esc(grubParams)}"` : "";
  const typeTag =
    cfg.output === "qcow2"
      ? `<type image="qcow2" filesystem="btrfs" primary="true"${kernAttr}/>`
      : cfg.output === "usb"
        ? `<type image="oem" filesystem="btrfs" primary="true"${kernAttr}/>`
        : cfg.output === "vmdk" || cfg.output === "vdi" || cfg.output === "vhd"
          ? `<type image="vmx" filesystem="btrfs" format="${cfg.output}" primary="true"${kernAttr}/>`
          : `<type image="iso" primary="true" firmware="uefi"${kernAttr}/>`;

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
    ...(cfg.firewall ? ["firewalld"] : []),
    ...wallpaperPackages(cfg, "zypper"),
    ...flatpakPackages(cfg),
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
    <bootloader name="grub2" timeout="${cfg.grub?.timeout ?? 10}"/>
  </preferences>
  <users>
${[
    `    <user name="${esc(sys.username)}" password="${esc(sys.password)}" home="/home/${esc(sys.username)}" groups="users" pwdformat="plain"/>`,
    ...(cfg.users ?? []).map(
      (u) =>
        `    <user name="${esc(u.name)}" password="${esc(u.password)}" home="/home/${esc(u.name)}" groups="users${u.admin ? ",wheel" : ""}" pwdformat="plain"/>`
    ),
  ].join("\n")}
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

  const net = staticNetwork(cfg);
  const netFile: Record<string, string> = net
    ? { "kiwi/root/etc/NetworkManager/system-connections/distroforge.nmconnection": nmConnection(net) }
    : {};

  const shLines = [
    "#!/bin/bash",
    "set -euo pipefail",
    "# DistroForge - configuracion post-instalacion (se ejecuta en chroot)",
    targetShellLine(cfg),
    `echo "${sys.username} ALL=(ALL) ALL" > /etc/sudoers.d/50-${sys.username}`,
    // las keys de repos custom entran al keyring antes de que zypper las verifique
    ...effectiveRepos(cfg)
      .filter((r) => r.gpgKey)
      .map((r) => `rpm --import "${r.gpgKey}"`),
    ...[firewallServiceLine(cfg), ...firewallOpenLines(cfg), ...serviceShellLines(cfg)],
  ];
  if (cfg.motd) shLines.push("cat > /etc/motd <<'MOTDEOF'", cfg.motd, "MOTDEOF");
  if (hasFlatpakApps(cfg)) shLines.push(...flatpakShellLines(cfg));
  if (cfg.cloudInit)
    shLines.push(
      "mkdir -p /etc/cloud/cloud.cfg.d",
      "cat > /etc/cloud/cloud.cfg.d/80-distroforge.cfg <<'CIEOF'",
      cloudInitYaml(cfg),
      "CIEOF"
    );
  if (wallpaperApplies(cfg) && dconfSchemaFor(cfg.desktop))
    shLines.push("dconf update || true");
  if (net)
    shLines.push(
      "chmod 600 /etc/NetworkManager/system-connections/distroforge.nmconnection"
    );
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
    ...Object.fromEntries(
      (cfg.extraFiles ?? []).map((f) => [`kiwi/root${f.path}`, f.content])
    ),
    ...Object.fromEntries(
      Object.entries(autologinFiles(cfg)).map(([p, c]) => [`kiwi/root${p}`, c])
    ),
    ...Object.fromEntries(
      Object.entries(autostartFiles(cfg)).map(([p, c]) => [`kiwi/root${p}`, c])
    ),
    ...Object.fromEntries(
      Object.entries(logoFiles(cfg)).map(([p, c]) => [`kiwi/root${p}`, c])
    ),
    ...Object.fromEntries(
      Object.entries(eulaFiles(cfg)).map(([p, c]) => [`kiwi/root${p}`, c])
    ),
    ...netFile,
    ...wallpaperFiles,
    "build.sh": buildSh,
  };
}
