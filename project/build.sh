#!/usr/bin/env bash
set -euo pipefail
# DistroForge - build imagen OCI (docker o podman)
DIR="$(cd "$(dirname "$0")" && pwd)"
CMD="$(command -v docker || command -v podman)"
mkdir -p out
"$CMD" build -t mi-distro:1.0.0 "$DIR"
"$CMD" save -o "out/mi-distro.tar" "mi-distro:1.0.0"
echo "Imagen OCI en ./out/mi-distro.tar (cargar con: docker load -i ...)"
