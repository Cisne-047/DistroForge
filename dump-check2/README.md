# mi-distro

Distro personalizada generada con DistroForge.

- Base: Arch Linux (motor: mkarchiso)
- Escritorio: Sin escritorio (servidor)
- Paquetes extra: git, vim, htop
- Flatpak: ninguno
- Ficheros extra: 0
- Repos custom: ninguno
- Salida: ISO
- Wallpaper: no



Nota Arch: usuario, contraseña y zona horaria del live los gestiona el perfil releng de archiso; ajústalos tras arrancar o editando la receta.


## Construir la imagen

**Opción A — GitHub Actions (recomendado):** sube esta carpeta a un repo nuevo de GitHub. En *Actions → build → Run workflow*. El artefacto contendrá la imagen.

**Opción B — local (Linux o WSL2 con Podman):**

    bash build.sh

## Estructura

- `archiso/` — receta de construcción del motor
- `build.sh` — build local con Podman
- `.github/workflows/build.yml` — build en CI

## Seguridad

La contraseña del usuario va en texto plano dentro de la receta (`archiso/`). No publiques el repo sin cambiarla o moverla a secretos de CI.
