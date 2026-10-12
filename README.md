# DistroForge — monta tu propia distro

Constructor web de distribuciones Linux personalizadas: eliges base, escritorio, paquetes, repos y sistema, y DistroForge genera el **proyecto de construcción completo** (receta del motor, `build.sh` local y workflow de GitHub Actions) listo para descargar como zip.

Todo corre en el navegador — no hay backend, no hay cuentas, la configuración vive en `localStorage`.

## Qué genera

- **14 bases**: openSUSE (Tumbleweed/Leap), familia Debian (Debian, Ubuntu, Mint, elementary, Pop!_OS, Zorin), familia Arch (Arch, Manjaro, CachyOS), Fedora, Gentoo y Solus.
- **4 motores de ISO** según familia: KIWI NG (openSUSE), live-build (Debian), mkarchiso (Arch) y kickstart + livemedia-creator (Fedora). Gentoo/Solus generan guía manual.
- **7 salidas**: ISO (los 4 motores), QCOW2, USB/disco raw, VMDK y VDI (KIWI), **OCI/Docker** (Dockerfile + CI con `docker build`/`docker save`, sin `--privileged`) y **WSL** (rootfs tar importable con `wsl --import`).
- **Extras**: hasta 10 escritorios, repos custom con suite/GPG (la key se importa de verdad en apt, zypper y dnf), cloud-init NoCloud opcional, MOTD editable, wallpaper aplicado vía dconf/xfconf, apps Flatpak, ficheros extra (dotfiles) por ruta absoluta, usuarios múltiples con admin, servicios systemd enable/disable, auto-login (SDDM/GDM/LightDM), logo personalizado (greeter LightDM), EULA en el primer login, instalación desatendida con preseed (Debian), target de arranque, autostart, firewall con puertos, modpacks estilo Prism + galería comunitaria (`gallery/`, ampliable por PR) — paridad con las pestañas General/Startup/Personalize/Files de SUSE Studio.
- **Formatos de salida**: ISO, QCOW2, imagen USB/disco raw (KIWI `oem`), VMDK (VMware) y VDI (VirtualBox) en openSUSE; OCI y WSL en todas las bases no manuales.
- **CI incluido en cada proyecto**: el zip trae `.github/workflows/build.yml` que construye la imagen en GitHub Actions y la sube como artefacto. Un lint propio (test `parsea el YAML de CI...`) valida todas las combinaciones base × salida.

## Uso

**Online:** la app está desplegada en **https://cisne-047.github.io/DistroForge/** (deploy automático con cada push).

**Local:**

```bash
npm install
npm run dev        # http://127.0.0.1:5173
```

Otros comandos: `npm test` (vitest), `npm run build` (tsc + vite build), `npx vite-node scripts/dump.ts <base> <nombre> <salida>` vuelca a disco la receta que generaría la app (útil para inspeccionar sin abrir el navegador).

## Flujo recomendado

1. Configura tu distro en los 5 pasos.
2. **Descargar zip** y súbelo a un repo nuevo de GitHub → *Actions → build → Run workflow* → el artefacto es tu imagen. Sin Docker en local, esta es la vía para tener "tu distro" hoy.
3. Alternativa local: `bash build.sh` dentro del zip (Linux o WSL2, requiere Podman/Docker).

## Test Drive y nube

Además del smoke test, cada proyecto ISO puede llevar `testdrive.yml`: Actions arranca la ISO en QEMU y **captura pantallas del arranque cada 12 s** — el artefacto es un paseo visual por tu distro. Y las imágenes de disco pueden subirse a la nube con `cloud-upload.yml` (AWS: S3 + import-snapshot a AMI; Azure: blob VHD), workflows que se saltan con aviso si faltan los secrets correspondientes. La instalación desatendida cubre Debian (preseed) y openSUSE (`oem-unattended` + perfil AutoYaST `autoinst.xml`); el kickstart de Fedora ya es desatendido de serie.

## QA extra

Con «Smoke test de arranque» activo, cada proyecto ISO incluye `smoke.yml`: Actions arranca la ISO en QEMU headless y comprueba que llega al login (añade `console=ttyS0` a los parámetros del kernel para ver todo el arranque en el log). La salida OCI puede firmarse con cosign keyless (OIDC de GitHub).

## Servidor opcional (backend)

`server/` es un backend Node 24 (Fastify + `node:sqlite`, Dockerfile incluido) que añade lo que la app estática no puede: **cuentas de usuario**, **galería con búsqueda server-side (FTS5)** y **Test Drive VNC interactivo** — arranca tu imagen en QEMU y la manejas desde el navegador con noVNC (sesiones con TTL de 15 min). La app lo detecta en *Paquetes → Servidor DistroForge*; sin él, todo lo demás funciona igual. Ver `server/README.md` para desplegarlo.

## Documentación

Manual técnico completo (arquitectura, guía de pasos, cobertura por motor, CI, modo advanced y núcleo Rust) en [docs/DistroForge-Documentacion.pdf](docs/DistroForge-Documentacion.pdf).

## Núcleo en Rust

`rust/` contiene el port del generador a Rust: `forge-core` (librería documentada con tests propios), `forge-cli` (versión de escritorio: `forge dump <base> <dir> [salida] [config.json]`), `forge-wasm` (bindings web) y `tauri-app` (scaffold de app de escritorio). Compilar y probar: `cargo test -p forge-core && cargo build -p forge-cli`.

## Roadmap

- **Paridad SUSE Studio restante:** logo personalizado, EULA de primer arranque, ISO de instalación desatendida (preseed/AutoYaST), estimación de tamaño de imagen.
- **Requiere backend:** Test Drive VNC real en el navegador y galería pública de distros.
- **Post-paridad:** firmado de imágenes, resolución de dependencias de paquetes, salidas de nube (AMI/VHD).

## Seguridad

La contraseña del usuario se incrusta en texto plano dentro de las recetas generadas (así funcionan los kickstart/presets de todos los motores). No publiques repos de proyectos generados con tu contraseña real — los tests de este repo solo usan `distroforge-test`.

## Estructura

```
src/
  catalog.ts        bases, escritorios y paquetes por familia
  steps.tsx         UI de los 5 pasos
  generators/       un fichero por motor + index (zip, CI, README generado)
  types.ts          DistroConfig
tests/              suite de generadores (incluye lint de YAML de CI)
scripts/dump.ts     vuelca recetas a disco
project/            proyecto de distro generado, listo para subir a su propio repo
```
