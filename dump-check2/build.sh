#!/usr/bin/env bash
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
