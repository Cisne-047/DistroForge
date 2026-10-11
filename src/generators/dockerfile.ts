import type { DistroConfig } from "../types";
import { baseById, desktopPackages, effectiveRepos } from "../catalog";

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
  const pkgs = [...desktopPackages(cfg.base, cfg.desktop), ...cfg.packages];
  const repos = effectiveRepos(cfg);
  const engine = base.engine;

  let run: string;
  if (engine === "livebuild") {
    const repoLines = repos
      .map((r) => {
        const src = r.suite
          ? `deb [trusted=yes] ${r.url} ${r.suite}`
          : `deb [trusted=yes] ${r.url} ./`;
        return `    echo ${sq(src)} > /etc/apt/sources.list.d/distroforge-${r.alias || "custom"}.list; \\`;
      })
      .join("\n");
    run = `RUN set -eux; \\
${repoLines ? repoLines + "\n" : ""}    apt-get update; \\
    apt-get install -y --no-install-recommends ${["sudo", ...pkgs].join(" ")}; \\
    apt-get clean; \\
    rm -rf /var/lib/apt/lists/*; \\
    useradd -m -s /bin/bash -G sudo ${sys.username}; \\
    echo ${sq(`${sys.username}:${sys.password}`)} | chpasswd`;
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
    useradd -m -s /bin/bash -G wheel ${sys.username}; \\
    echo ${sq(`${sys.username}:${sys.password}`)} | chpasswd`;
  } else if (engine === "kiwi") {
    run = `RUN zypper --gpg-auto-import-keys --non-interactive install ${pkgs.join(" ")}; \\
    useradd -m -s /bin/bash -G users ${sys.username}; \\
    echo ${sq(`${sys.username}:${sys.password}`)} | chpasswd`;
  } else {
    run = `RUN dnf install -y ${pkgs.join(" ")}; \\
    dnf clean all; \\
    useradd -m -s /bin/bash -G wheel ${sys.username}; \\
    echo ${sq(`${sys.username}:${sys.password}`)} | chpasswd`;
  }

  const dockerfile = `FROM ${image}
LABEL org.opencontainers.image.title="${cfg.name}"
LABEL org.opencontainers.image.description="Distro personalizada generada con DistroForge"
${engine === "livebuild" ? "ENV DEBIAN_FRONTEND=noninteractive\n" : ""}${run}

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

  return { Dockerfile: dockerfile, "build.sh": buildSh };
}

export function ociSupported(cfg: DistroConfig): boolean {
  return cfg.base in BASE_IMAGE;
}
