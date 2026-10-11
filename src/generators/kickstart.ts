import type { DistroConfig } from "../types";
import { desktopPackages, effectiveRepos } from "../catalog";
import { CLOUD_INIT_PACKAGE, cloudInitYaml } from "./cloudinit";
import { flatpakPackages, flatpakShellLines } from "./flatpak";
import { autologinFiles, serviceShellLines } from "./accounts";
import { dottedMask, staticNetwork } from "./network";

export function dirnameOf(p: string): string {
  const d = p.replace(/\/[^/]*$/, "");
  return d || "/";
}

/** Bloque %post que vuelca cada fichero extra con heredoc (rutas absolutas). */
export function extraFilePostLines(extraFiles: { path: string; content: string }[]): string[] {
  return (extraFiles ?? []).flatMap((f) => [
    `mkdir -p ${dirnameOf(f.path)}`,
    `cat > ${f.path} <<'DFEOF'`,
    f.content,
    "DFEOF",
  ]);
}

export function kickstartFiles(cfg: DistroConfig): Record<string, string> {
  const sys = cfg.system;
  const repos = effectiveRepos(cfg);

  const repoLines = repos
    .map((r) => `repo --name="${r.alias}" --baseurl="${r.url}" --nogpgcheck`)
    .join("\n");

  const postLines = [
    'echo "DistroForge" > /etc/distroforge-release',
    // instalación sin verificación (anaconda no puede preguntar), pero el sistema
    // final queda confiando en las keys para futuras actualizaciones
    ...repos.filter((r) => r.gpgKey).map((r) => `rpm --import "${r.gpgKey}"`),
    ...(cfg.motd ? ["cat > /etc/motd <<'MOTDEOF'", cfg.motd, "MOTDEOF"] : []),
    ...(cfg.cloudInit
      ? [
          "mkdir -p /etc/cloud/cloud.cfg.d",
          "cat > /etc/cloud/cloud.cfg.d/80-distroforge.cfg <<'CIEOF'",
          cloudInitYaml(cfg),
          "CIEOF",
        ]
      : []),
    ...extraFilePostLines(cfg.extraFiles),
    ...Object.entries(autologinFiles(cfg)).flatMap(([path, content]) =>
      extraFilePostLines([{ path, content }])
    ),
    ...serviceShellLines(cfg),
    ...flatpakShellLines(cfg),
  ];
  const postBlock = `%post
${postLines.join("\n")}
%end
`;

  const net = staticNetwork(cfg);
  const netLine = net
    ? `network --bootproto=static --ip=${net.ip} --netmask=${dottedMask(net.prefix)} --gateway=${net.gateway} --nameserver=${net.dns.join(",")} --hostname=${sys.hostname}`
    : `network --bootproto=dhcp --hostname=${sys.hostname}`;

  const ks = `# DistroForge - kickstart Fedora
lang ${sys.locale}
keyboard --vckeymap=${sys.keyboard} --xlayouts='${sys.keyboard}'
timezone ${sys.timezone} --utc
${netLine}
rootpw --lock
user --name=${sys.username} --password=${sys.password} --plaintext --groups=wheel
${(cfg.users ?? [])
  .map(
    (u) =>
      `user --name=${u.name} --password=${u.password} --plaintext${u.admin ? " --groups=wheel" : ""}`
  )
  .join("\n")}
firstboot --disable
bootloader --location=mbr
zerombr
clearpart --all --initlabel
autopart
reboot
url --mirrorlist="https://mirrors.fedoraproject.org/mirrorlist?repo=fedora-$releasever&arch=x86_64"
${repoLines}
%packages
@core
${[...desktopPackages(cfg.base, cfg.desktop), ...flatpakPackages(cfg), ...cfg.packages, ...(cfg.cloudInit ? [CLOUD_INIT_PACKAGE] : [])].join("\n")}
%end

${postBlock}`;

  const buildSh = `#!/usr/bin/env bash
set -euo pipefail
# DistroForge - build local (Linux o WSL2, requiere Podman)
DIR="$(cd "$(dirname "$0")" && pwd)"
podman run --rm --privileged -v "$DIR":/project -v "$DIR/out":/out fedora:latest bash -ec '
  dnf install -y livemedia-creator
  livemedia-creator --no-virt --make-iso --ks /project/fedora/ks.cfg --releasever 41 --iso-only --iso-name ${cfg.name}.iso --resultdir /out
'
echo "Imagen lista en ./out/${cfg.name}.iso"
`;

  return {
    "fedora/ks.cfg": ks,
    "build.sh": buildSh,
  };
}
