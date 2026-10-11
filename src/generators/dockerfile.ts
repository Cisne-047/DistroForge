import type { DistroConfig } from "../types";
import { baseById, desktopPackages, effectiveRepos } from "../catalog";
import { flatpakPackages, hasFlatpakApps } from "./flatpak";
import { dirnameOf } from "./kickstart";
import { sanitizeAlias } from "./livebuild";

/** Imagen base OCI para cada distro (salida "oci"). */
const BASE_IMAGE: Record<string, string> = {
  debian: "debian:bookworm",
  ubuntu: "ubuntu:24.04",
  mint: "ubuntu:24.04",
  elementary: "ubuntu:24.04",
  pop: "ubuntu:22.04",
  zorin: "ubuntu:24.04",
  fedora: "fedora:41",
  "opensuse-tumbleweed": "opensuse/tumbleweed",
  "opensuse-leap": "opensuse/leap:15.6",
  arch: "archlinux:latest",
  manjaro: "archlinux:latest",
  cachyos: "archlinux:latest",
};

function sq(s: string): string {
  return `'${s.replace(/'/g, `'\\''`)}'`;
}

export function dockerfileFiles(cfg: DistroConfig): Record<string, string> {
  const base = baseById(cfg.base);
  const sys = cfg.system;
  const image = BASE_IMAGE[cfg.base];
  if (!image) return {};
  const pkgs = [...desktopPackages(cfg.base, cfg.desktop), ...flatpakPackages(cfg), ...cfg.packages];
  const repos = effectiveRepos(cfg);
  const engine = base.engine;
  const flatpakRun = hasFlatpakApps(cfg)
    ? `    flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo; \\
    flatpak install -y --noninteractive flathub ${cfg.flatpakApps.join(" ")}; \\
`
    : "";
  const extraFiles = cfg.extraFiles ?? [];
  const extraDirs = [...new Set(extraFiles.map((f) => dirnameOf(f.path)))];
  // líneas en formato RUN (continuaciones `; \`); la última queda sin continuación
  const extraUsers = (cfg.users ?? [])
    .map(
      (u) =>
        `    id "${u.name}" >/dev/null 2>&1 || useradd -m -s /bin/bash${u.admin ? " -G sudo" : ""} "${u.name}"; \\
    echo ${sq(`${u.name}:${u.password}`)} | chpasswd; \\`
    )
    .join("\n")
    .replace(/; \\$/, "");

  let run: string;
  if (engine === "livebuild") {
    const keyLines = repos
      .map((r, i) =>
        r.gpgKey
          ? `    mkdir -p /usr/share/keyrings; \\
    wget -qO /usr/share/keyrings/${sanitizeAlias(r.alias, i)}.asc ${r.gpgKey}; \\`
          : ""
      )
      .filter(Boolean)
      .join("\n");
    const repoLines = repos
      .map((r, i) => {
        const alias = sanitizeAlias(r.alias, i);
        const signed = r.gpgKey
          ? `[signed-by=/usr/share/keyrings/${alias}.asc]`
          : "[trusted=yes]";
        const src = r.suite ? `deb ${signed} ${r.url} ${r.suite}` : `deb ${signed} ${r.url} ./`;
        return `    echo ${sq(src)} > /etc/apt/sources.list.d/distroforge-${alias}.list; \\`;
      })
      .join("\n");
    run = `RUN set -eux; \\
    apt-get update; \\
    apt-get install -y --no-install-recommends wget ca-certificates; \\
${keyLines ? keyLines + "\n" : ""}${repoLines ? repoLines + "\n" : ""}    apt-get update; \\
    apt-get install -y --no-install-recommends ${["sudo", ...pkgs].join(" ")}; \\
    apt-get clean; \\
    rm -rf /var/lib/apt/lists/*; \\
${flatpakRun}    useradd -m -s /bin/bash -G sudo ${sys.username}; \\
    echo ${sq(`${sys.username}:${sys.password}`)} | chpasswd${extraUsers ? `; \\\n${extraUsers}` : ""}`;
  } else if (engine === "mkarchiso") {
    const repoLines = repos
      .map(
        (r) =>
          `    printf '[${r.alias}]\\nSigLevel = Optional TrustAll\\nServer = ${r.url}\\n' >> /etc/pacman.conf; \\`
      )
      .join("\n");
    run = `RUN set -eux; \\
${repoLines ? repoLines + "\n" : ""}    pacman -Syu --noconfirm ${["sudo", ...pkgs].join(" ")}; \\
    pacman -Scc --noconfirm; \\
${flatpakRun}    useradd -m -s /bin/bash -G wheel ${sys.username}; \\
    echo ${sq(`${sys.username}:${sys.password}`)} | chpasswd${extraUsers ? `; \\\n${extraUsers}` : ""}`;
  } else if (engine === "kiwi") {
    run = `RUN zypper --gpg-auto-import-keys --non-interactive install ${pkgs.join(" ")}; \\
${flatpakRun}    useradd -m -s /bin/bash -G users ${sys.username}; \\
    echo ${sq(`${sys.username}:${sys.password}`)} | chpasswd${extraUsers ? `; \\\n${extraUsers}` : ""}`;
  } else {
    run = `RUN dnf install -y ${pkgs.join(" ")}; \\
${flatpakRun}    dnf clean all; \\
    useradd -m -s /bin/bash -G wheel ${sys.username}; \\
    echo ${sq(`${sys.username}:${sys.password}`)} | chpasswd${extraUsers ? `; \\\n${extraUsers}` : ""}`;
  }

  const dockerfile = `FROM ${image}
LABEL org.opencontainers.image.title="${cfg.name}"
LABEL org.opencontainers.image.description="Distro personalizada generada con DistroForge"
${engine === "livebuild" ? "ENV DEBIAN_FRONTEND=noninteractive\n" : ""}${run}
${extraDirs.length ? `RUN mkdir -p ${extraDirs.join(" ")}\n` : ""}${extraFiles
    .map((f, i) => `COPY files/extra-${i} ${f.path}`)
    .join("\n")}
USER ${sys.username}
WORKDIR /home/${sys.username}
CMD ["/bin/bash"]
`;

  const buildSh = `#!/usr/bin/env bash
set -euo pipefail
# DistroForge - build imagen OCI (docker o podman)
DIR="$(cd "$(dirname "$0")" && pwd)"
CMD="$(command -v docker || command -v podman)"
mkdir -p out
"$CMD" build -t ${cfg.name}:1.0.0 "$DIR"
"$CMD" save -o "out/${cfg.name}.tar" "${cfg.name}:1.0.0"
echo "Imagen OCI en ./out/${cfg.name}.tar (cargar con: docker load -i ...)"
`;

  return {
    Dockerfile: dockerfile,
    "build.sh": buildSh,
    ...Object.fromEntries(extraFiles.map((f, i) => [`files/extra-${i}`, f.content])),
  };
}

export function ociSupported(cfg: DistroConfig): boolean {
  return cfg.base in BASE_IMAGE;
}
