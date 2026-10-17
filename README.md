# DistroForge — monta tu propia distro (100% Rust)

Constructor de distribuciones Linux personalizadas: eliges base, escritorio, paquetes, repos y sistema, y genera el **proyecto de construcción completo** (receta del motor, `build.sh` local y workflow de GitHub Actions) listo para descargar como zip. Sucesor espiritual de SUSE Studio.

- **App web**: <https://cisne-047.github.io/DistroForge/>
- **App de escritorio**: `.deb`, `.rpm`, `.AppImage`, `.msi` y `.exe` en [Releases](../../releases)
- **Historial de cambios**: [CHANGELOG.md](CHANGELOG.md)
- **Licencia**: MIT para el código; el logotipo queda fuera ([LICENSE](LICENSE))

**Estado: 1.0.0.** Todo lo que promete este README está hecho y comprobado en CI, incluida una ISO real que se hornea y arranca en cada release. Las limitaciones conocidas (firma de sistema, macOS y el host del backend) están al final, en el roadmap.

Todo el código es Rust desde la v0.4.0: la UI web (Leptos/WASM), el núcleo generador (`forge-core`), el CLI, el backend con Test Drive VNC (`axum`) y la app de escritorio (Tauri 2).

## Qué genera

- **21 bases**: openSUSE (Tumbleweed/Leap), familia Debian (Debian, Ubuntu, Mint, elementary, Pop!_OS, Zorin, Kali, MX), familia Arch (Arch, Manjaro, CachyOS, Garuda, SteamOS), Fedora y clones RHEL (AlmaLinux, Rocky Linux, Oracle Linux), Gentoo y Solus (guía manual).
- **4 motores ISO**: KIWI NG, live-build, mkarchiso y kickstart + livemedia-creator.
- **8 salidas**: ISO, QCOW2, USB/raw, VMDK, VDI, VHD, **OCI** (con SBOM syft + escaneo trivy + firma cosign opcional) y **WSL**.
- **Arquitecturas**: x86_64, aarch64 (runners ARM nativos en CI), armhf (QEMU), i386.
- **Features**: escritorios hasta 10, repos custom con GPG real, cloud-init, MOTD, wallpaper dconf/xfconf, logo, EULA, preseed/AutoYaST desatendida, usuarios múltiples, servicios, cortafuegos con puertos, red estática, GRUB configurable, autostart, auto-login, flatpaks, ficheros overlay, modpacks estilo Prism + galería comunitaria (`gallery/`, por PR), plantillas rápidas, smoke test QEMU en CI, Test Drive con capturas, subida AWS/Azure, modo avanzado (remaster de ISO custom con xorriso) y estimación de tamaño.

## Estructura

```
rust/
  crates/forge-core   núcleo generador (~80 tests)
  crates/forge-cli    CLI: forge dump <base> <dir> [salida] [config.json]
  crates/forge-wasm   bindings del core para la web
  web-ui              la app web: Leptos 0.7 CSR + Trunk → dist/ para Pages
  server-rs           backend axum: cuentas, galería FTS5, Test Drive VNC (QEMU+noVNC)
  tauri-app           app de escritorio (Tauri 2) que sirve el bundle de web-ui
gallery/              packs comunitarios; la UI los empaqueta en el build
tools/                scripts de mantenimiento (generación de iconos y del manual)
docs/                 manual técnico (DOCX + PDF), guía de despliegue y diseño
project/              ejemplo de proyecto generado
assets/               imágenes y distintivos del README
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

`rust/server-rs` es el backend en axum: cuentas con scrypt, galería con FTS5 y Test Drive VNC (QEMU + noVNC).

```bash
cd rust/server-rs && cargo run --release     # 0.0.0.0:8080
PORT=3000 cargo run                          # otro puerto
DF_DB_PATH=/data/df.db cargo run             # SQLite en disco (por defecto :memory:)
```

Tiene `Dockerfile` (Debian slim + QEMU, usuario sin privilegios y `HEALTHCHECK` contra `GET /api/health`), `docker-compose.yml` y un workflow que publica la imagen en GHCR. La UI apunta al backend con la URL que configures en la sección *Servidor* (se guarda en el navegador). Guía completa en [docs/DESPLIEGUE.md](docs/DESPLIEGUE.md).

Variables de entorno del backend:

| Variable | Para qué |
|---|---|
| `PORT` | puerto de escucha (8080 por defecto) |
| `DF_DB_PATH` | SQLite en disco (`:memory:` por defecto) |
| `DF_WEB_DIR` | sirve la UI compilada en el mismo origen (si existe) |
| `QEMU_BIN` | binario de QEMU del Test Drive |
| `DF_ISO_DIR` | confina las imágenes que puede arrancar el Test Drive; sin ella solo se aceptan rutas absolutas existentes con extensión `.iso/.img/.qcow2/.raw` fuera de `/proc`, `/sys` y `/dev` |

El servidor envía `X-Content-Type-Options`, `Referrer-Policy`, `X-Frame-Options` y una CSP estricta en todo menos en `/vnc.html` (noVNC carga su cliente desde CDN), limita el cuerpo de la API a 256 KB, y marca la cookie de sesión `Secure` cuando la petición llega con `X-Forwarded-Proto: https`.

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
```

El CI ([`.github/workflows/ci.yml`](.github/workflows/ci.yml)) compila y prueba en cada push y PR: núcleo Rust con clippy, backend, UI WASM con `trunk build` + `wasm-opt` y la interfaz en un navegador real. El deploy a Pages solo corre en `main`.

Hay además dos workflows que no se conforman con compilar:

- [`server-image.yml`](.github/workflows/server-image.yml) publica la imagen del backend en GHCR, con la interfaz dentro.
- [`image-smoke.yml`](.github/workflows/image-smoke.yml) **ejecuta** esa imagen en un sandbox y comprueba salud, interfaz, estáticos, el 404 en JSON, el registro con sesión y que el contenedor llega a `healthy`. Construir no bastaba: un desajuste de glibc entre compilador y runtime solo se ve al arrancar.

## Seguridad

Las contraseñas viajan en claro dentro de las recetas generadas (así funcionan kickstart/preseed) y en los enlaces para compartir. Usa una contraseña provisional en cualquier proyecto que vayas a publicar.

El modelo de defensa es explícito, porque las recetas son **código que se ejecuta** en un chroot:

- **Validación en un solo sitio** (`forge_core::security`): locale, zona horaria, teclado, paquetes, URLs, suites, usuarios, puertos, unidades y rutas se comprueban antes de usarse. La UI, el CLI (`forge dump`) y el WASM (`generate_project`) rechazan una config que no pase `validate()`, vengan de donde vengan (pack de la galería, enlace compartido o JSON a mano).
- **Escape en cada sink**: lo que acaba en un script pasa por `sq`/`dq`, los heredocs usan un marcador único que el contenido no puede cerrar, el YAML de cloud-init se comilla como JSON y las rutas se confinan al directorio de salida.
- **Backend sin pánicos**: el candado de la BD tolera envenenamiento, el alta duplicada responde 409, scrypt corre fuera del candado y del hilo de la runtime, hay límite de intentos por cuenta y de tamaño para packs y cuerpos, y los errores internos no filtran SQL ni rutas.
- **Test Drive confinado**: ruta canonicalizada, fichero regular con extensión de imagen, directorio permitido por `DF_ISO_DIR`, límite de sesiones y comprobación de dueño en `DELETE` y en el WebSocket (un id ajeno responde 404).
- **Escritorio**: la app Tauri ya no lleva `"csp": null`; sirve su UI local con una CSP que solo permite scripts propios y WASM.
- **CI**: permisos mínimos por workflow, secretos solo en los steps que los usan, `flyctl` fijado a un commit y validación de tags y nombres de paquete antes de pasar a un comando.

## Roadmap

Lo que queda, por orden de importancia:

- **Un host propio para el backend.** Hoy vive en un sandbox y su URL por defecto está horneada en la app web. El despliegue a un host fijo está automatizado (Fly con `FLY_API_TOKEN`): es cambiar la variable del repositorio y la `endpoints` del updater.
- **Firma de paquetes a nivel de sistema**: la auto-actualización ya está (claves propias + manifiesto en el host público), pero quitar los avisos del sistema operativo exige un certificado de pago con identidad verificada, así que los instaladores siguen saliendo sin firmar.
- **macOS** (`.icns` y firma de código) si alguien lo necesita: la matriz de release es Linux + Windows.
- **Más pruebas de interfaz**: hoy cubren el arranque, el resumen, la navegación y el paso de construir; falta el resto de pasos y la integración con el backend.

---

<img src="assets/built-with-nan-dark.svg" alt="Built with NaN" width="266" height="104">
