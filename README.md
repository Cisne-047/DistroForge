# DistroForge — monta tu propia distro (100% Rust)

Constructor de distribuciones Linux personalizadas: eliges base, escritorio, paquetes, repos y sistema, y genera el **proyecto de construcción completo** (receta del motor, `build.sh` local y workflow de GitHub Actions) listo para descargar como zip. Sucesor espiritual de SUSE Studio.

- **App web**: <https://cisne-047.github.io/DistroForge/>
- **App de escritorio**: `.deb`, `.rpm`, `.AppImage`, `.msi` y `.exe` en [Releases](../../releases)
- **Historial de cambios**: [CHANGELOG.md](CHANGELOG.md)
- **Licencia**: MIT para el código; el logotipo queda fuera ([LICENSE](LICENSE))

Todo el código es Rust desde la v0.4.0: la UI web (Leptos/WASM), el núcleo generador (`forge-core`), el CLI, el backend con Test Drive VNC (`axum`) y la app de escritorio (Tauri 2). La app React/TS original y el backend Node viven en `legacy/` como referencia y suite de paridad.

## Qué genera

- **14 bases**: openSUSE (Tumbleweed/Leap), familia Debian (Debian, Ubuntu, Mint, elementary, Pop!_OS, Zorin), familia Arch (Arch, Manjaro, CachyOS), Fedora, Gentoo y Solus (guía manual).
- **4 motores ISO**: KIWI NG, live-build, mkarchiso y kickstart + livemedia-creator.
- **8 salidas**: ISO, QCOW2, USB/raw, VMDK, VDI, VHD, **OCI** (con SBOM syft + escaneo trivy + firma cosign opcional) y **WSL**.
- **Arquitecturas**: x86_64, aarch64 (runners ARM nativos en CI), armhf (QEMU), i386.
- **Features**: escritorios hasta 10, repos custom con GPG real, cloud-init, MOTD, wallpaper dconf/xfconf, logo, EULA, preseed/AutoYaST desatendida, usuarios múltiples, servicios, cortafuegos con puertos, red estática, GRUB configurable, autostart, auto-login, flatpaks, ficheros overlay, modpacks estilo Prism + galería comunitaria (`gallery/`, por PR), plantillas rápidas, smoke test QEMU en CI, Test Drive con capturas, subida AWS/Azure, modo avanzado (remaster de ISO custom con xorriso) y estimación de tamaño.

## Estructura

```
rust/
  crates/forge-core   núcleo generador (~80 tests, paridad byte a byte con legacy)
  crates/forge-cli    CLI: forge dump <base> <dir> [salida] [config.json]
  crates/forge-wasm   bindings del core para la web
  rust/web-ui         la app web: Leptos 0.7 CSR + Trunk → dist/ para Pages
  rust/server-rs      backend axum: cuentas, galería FTS5, Test Drive VNC (QEMU+noVNC)
  rust/tauri-app      app de escritorio (Tauri 2) que sirve el bundle de web-ui
legacy/               app React/TS + backend Node (referencia; suite de paridad)
gallery/              packs comunitarios; la UI los empaqueta en el build
tools/                scripts de mantenimiento (generación de iconos)
docs/                 manual técnico (DOCX + PDF), guía de despliegue y diseño
project/              ejemplo de proyecto generado
```

## Galería comunitaria

Los packs viven en `gallery/` (`index.json` + `packs/*.json`) y se añaden por PR. La app **no** los descarga de `raw.githubusercontent`: Trunk copia `gallery/` dentro de `dist/` y la UI la lee por ruta relativa (`gallery/index.json`). Así funciona con el repositorio privado, en GitHub Pages, en `trunk serve` y dentro del binario de escritorio, que no tiene red.

Para publicar un pack: añade `gallery/packs/mi-pack.json` y su entrada en `gallery/index.json`. También puedes importar un pack por URL desde la UI (pestaña *Paquetes*).

## App de escritorio (Tauri 2)

`rust/tauri-app` sirve el bundle ya compilado de la UI Leptos en una ventana nativa y expone el núcleo por IPC como punto de extensión.

```bash
cd rust/web-ui && trunk build --release          # bundle que consume Tauri
cd rust/tauri-app/src-tauri && cargo tauri build # paquete de la plataforma
cd rust/web-ui && trunk serve                    # desarrollo: Tauri apunta a :8080
```

La UI se compila **antes** de Tauri y en su propio paso: los hooks `beforeBuildCommand` corren desde `rust/tauri-app`, no desde `src-tauri`, así que una ruta relativa no resuelve.

El binario no necesita conexión: la galería viaja en el bundle y el ZIP se genera con una copia local de [fflate](https://github.com/101arrowz/fflate) (MIT).

[`.github/workflows/release.yml`](.github/workflows/release.yml) empaqueta en matriz Linux (`deb`, `rpm`, `appimage`) + Windows (`msi`, `nsis`), sube artefactos en cada ejecución y publica la GitHub Release solo al empujar un tag `v*`. Con `workflow_dispatch` construye sin publicar, para poder verificarlo. Los iconos se generan con `python tools/make_icons.py` (Pillow); macOS necesitaría añadir un `.icns`.

## Backend (cuentas, galería y Test Drive)

`rust/server-rs` replica el backend Node de `legacy/` en axum: cuentas con scrypt, galería con FTS5 y Test Drive VNC (QEMU + noVNC).

```bash
cd rust/server-rs && cargo run --release     # 0.0.0.0:8080
PORT=3000 cargo run                          # otro puerto
DF_DB_PATH=/data/df.db cargo run             # SQLite en disco (por defecto :memory:)
```

Tiene `Dockerfile` (Debian slim + QEMU, usuario sin privilegios y `HEALTHCHECK` contra `GET /api/health`), `docker-compose.yml` y un workflow que publica la imagen en GHCR. La UI apunta al backend con la URL que configures en la sección *Servidor* (se guarda en el navegador). Guía completa en [docs/DESPLIEGUE.md](docs/DESPLIEGUE.md).

## Desarrollo

```bash
# UI web (dev con recarga)
cd rust/web-ui && trunk serve

# core + CLI
cd rust && cargo test -p forge-core && cargo run -p forge-cli -- dump debian /tmp/check iso

# backend
cd rust/server-rs && cargo test && cargo run --release

# escritorio
cd rust/tauri-app/src-tauri && cargo tauri dev

# legacy (referencia)
cd legacy && npm ci && npm test
```

El CI ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)) compila y prueba en cada push y PR: núcleo Rust con clippy, backend, UI WASM con `trunk build` + `wasm-opt` y la suite legacy. El deploy a Pages solo corre en `main`.

Hay además dos workflows que no se conforman con compilar:

- [`server-image.yml`](.github/workflows/server-image.yml) publica la imagen del backend en GHCR, con la interfaz dentro.
- [`image-smoke.yml`](.github/workflows/image-smoke.yml) **ejecuta** esa imagen en un sandbox y comprueba salud, interfaz, estáticos, el 404 en JSON, el registro con sesión y que el contenedor llega a `healthy`. Construir no bastaba: un desajuste de glibc entre compilador y runtime solo se ve al arrancar.

## Seguridad

Las contraseñas viajan en claro dentro de las recetas generadas (así funcionan kickstart/preseed) y en los enlaces para compartir. Usa una contraseña provisional en cualquier proyecto que vayas a publicar.

## Roadmap

Lo que queda, por orden de importancia:

- **Hosting permanente del backend con URL por defecto.** El despliegue está automatizado (Fly con `FLY_API_TOKEN`) y verificado en ejecución, pero la URL pública sigue siendo efímera: o se le da un host fijo, o la UI tendrá que seguir pidiéndola.
- **Firma de los paquetes de escritorio y auto-actualización** (`tauri-plugin-updater`): hoy se publican sin firmar y sin actualizaciones.
- **macOS** (`.icns` y firma de código) si alguien lo necesita: la matriz de release es Linux + Windows.
- **Una ISO real de punta a punta en CI.** El núcleo genera las recetas y las valida (77 tests) y los workflows generados se comprueban como YAML, pero ninguna ISO se hornea en CI: es el hueco de verificación más grande que queda, y el más caro de cerrar.
- **Pruebas de interfaz**: hoy la UI solo se compila y se construye.
- **Modo claro** de *Grafito & Verde*: los tokens están agrupados para que sea cambiar un bloque de color, pero aún no existe.
