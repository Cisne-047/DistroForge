# mi-distro

Distro personalizada generada con DistroForge.

- Base: Debian 12 (bookworm) (motor: livebuild)
- Escritorio: Sin escritorio (servidor)
- Paquetes extra: git, vim, htop
- Flatpak: ninguno
- Ficheros extra: 0
- Repos custom: ninguno
- Salida: OCI
- Wallpaper: no

Salida OCI: construye con `bash build.sh` (docker o podman) o el workflow de CI; carga el .tar con `docker load -i`.




## Construir la imagen

**Opción A — GitHub Actions (recomendado):** sube esta carpeta a un repo nuevo de GitHub. En *Actions → build → Run workflow*. El artefacto contendrá la imagen.

**Opción B — local (Linux o WSL2 con Podman):**

    bash build.sh

## Estructura

- `livebuild/` — receta de construcción del motor
- `build.sh` — build local con Podman
- `.github/workflows/build.yml` — build en CI

## Seguridad

La contraseña del usuario va en texto plano dentro de la receta (`livebuild/`). No publiques el repo sin cambiarla o moverla a secretos de CI.
