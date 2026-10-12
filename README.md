# DistroForge — monta tu propia distro (100% Rust)

Constructor de distribuciones Linux personalizadas: eliges base, escritorio, paquetes, repos y sistema, y genera el **proyecto de construcción completo** (receta del motor, `build.sh` local y workflow de GitHub Actions) listo para descargar como zip. Sucesor espiritual de SUSE Studio.

**Todo el código es Rust** desde la v0.4.0: la UI web (Leptos/WASM), el núcleo generador (`forge-core`), el CLI de escritorio, el backend con Test Drive VNC (`axum`) y el scaffold Tauri. La app React/TS original y el backend Node viven en `legacy/` como referencia y suite de paridad.

App en producción: **https://cisne-047.github.io/DistroForge/**

## Qué genera

- **14 bases**: openSUSE (Tumbleweed/Leap), familia Debian (Debian, Ubuntu, Mint, elementary, Pop!_OS, Zorin), familia Arch (Arch, Manjaro, CachyOS), Fedora, Gentoo y Solus (guía manual).
- **4 motores ISO**: KIWI NG, live-build, mkarchiso y kickstart + livemedia-creator.
- **8 salidas**: ISO, QCOW2, USB/raw, VMDK, VDI, VHD, **OCI** (con SBOM syft + escaneo trivy + firma cosign opcional) y **WSL**.
- **Arquitecturas**: x86_64, aarch64 (runners ARM nativos en CI), armhf (QEMU), i386.
- **Features**: escritorios hasta 10, repos custom con GPG real, cloud-init, MOTD, wallpaper dconf/xfconf, logo, EULA, preseed/AutoYaST desatendida, usuarios múltiples, servicios, firewall con puertos, red estática, GRUB configurable, autostart, auto-login, flatpaks, ficheros overlay, modpacks estilo Prism + galería comunitaria (`gallery/`, por PR), plantillas rápidas, smoke test QEMU en CI, Test Drive con capturas, subida AWS/Azure, modo advanced (remaster de ISO custom con xorriso) y estimación de tamaño.

## Estructura (Rust)

```
rust/
  crates/forge-core   núcleo generador (79 tests, byte-parity con legacy)
  crates/forge-cli    CLI: forge dump <base> <dir> [salida] [config.json]
  crates/forge-wasm   bindings del core para la web
  rust/web-ui         la app: Leptos 0.7 CSR + Trunk → dist/ para Pages
  rust/server-rs      backend axum: cuentas, galería FTS5, Test Drive VNC (QEMU+noVNC)
  rust/tauri-app      scaffold de app de escritorio (Tauri 2)
legacy/               app React/TS + backend Node (referencia; 97+8 tests de paridad)
gallery/              packs comunitarios (los consume la UI via raw.githubusercontent)
docs/                 manual técnico (PDF)
project/              ejemplo de proyecto generado
```

## Desarrollo

```bash
# UI (dev con recarga)
cd rust/web-ui && trunk serve

# core + CLI
cd rust && cargo test -p forge-core && cargo run -p forge-cli -- dump debian /tmp/check iso

# backend (auth, galería FTS5, Test Drive VNC)
cd rust/server-rs && cargo run --release   # :8080, QEMU opcional

# legacy (referencia TS)
cd legacy && npm ci && npm test
```

## Seguridad

Las contraseñas viajan en claro dentro de las recetas generadas (así funcionan kickstart/preseed) y en los enlaces para compartir. No publiques proyectos generados con tu contraseña real.

## Roadmap

- Compilación del binario Tauri de escritorio en CI (release artifacts).
- wasm-opt/binaryen para reducir el bundle de la UI (~2.4MB → ~700KB con brotli).
- Sección "Servidor DistroForge" en la UI Rust (login/búsqueda/VNC; ya existe en legacy).
