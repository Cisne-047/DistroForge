# Historial de cambios

Las etiquetas del repositorio siguen `vX.Y.Z`. La v0.5.0 todavía **no está etiquetada**: este es el trabajo acumulado en `main` desde la v0.4.0.

## v0.5.0 (sin publicar)

### Arreglado

- **Galería comunitaria en producción.** La app pedía los packs a `raw.githubusercontent` del propio repositorio, que es privado: el fetch sin token devolvía 404 y la galería no cargaba. Ahora Trunk empaqueta `gallery/` dentro del bundle y la UI la lee por ruta relativa, así que funciona en Pages, en `trunk serve` y en la app de escritorio (que no tiene red).
- **Empaquetado de escritorio.** El workflow de release nunca se había ejecutado y no podía funcionar: `tauri.conf.json` no tenía sección `bundle` ni iconos, y `beforeBuildCommand` corría desde `rust/tauri-app` (no desde `src-tauri`), así que la ruta relativa a la UI no resolvía.
- **ZIP sin conexión.** `fflate` se carga desde una copia local (0.8.2, MIT) en lugar de un CDN: el escritorio ya puede generar el zip.
- La ayuda de la galería describía el comportamiento antiguo ("se cargan desde GitHub").

### Añadido

- **App de escritorio publicable**: matriz de paquetes Linux (`.deb`, `.rpm`, `.AppImage`) y Windows (`.msi`, `.exe`), artefactos en cada ejecución y GitHub Release al empujar un tag. Iconos generados de forma reproducible con `tools/make_icons.py`.
- **Servidor todo-en-uno**: con `DF_WEB_DIR`, `server-rs` sirve la interfaz junto a la API **en el mismo origen**, que es lo que hace que la cookie de sesión viaje: cuentas, galería compartida y Test Drive funcionan sin configurar ninguna URL. `tools/serve_tunnel.ps1` lo publica con un túnel de Cloudflare sin necesidad de cuentas.
- **Backend listo para desplegar**: `GET /api/health` para el healthcheck, Dockerfile con la capa de dependencias cacheada, usuario sin privilegios, volumen para el SQLite y `HEALTHCHECK`; `docker-compose.yml`; workflow que publica la imagen en GHCR; y [docs/DESPLIEGUE.md](docs/DESPLIEGUE.md) con las opciones de hosting y el aviso sobre cookies entre dominios.
- **Manual técnico reproducible** (`tools/make_manual.py` → DOCX + PDF): antes el PDF era anterior al rewrite y describía la app React.
- **Enlace directo a un paso** (`?paso=N`) para compartir un punto concreto del asistente.

### Cambiado

- **Identidad**: el logotipo es la ilustración original de Tux forjando en el yunque. La **marca** es la cabeza de Tux centrada en la cara —recortada automáticamente por `tools/make_icons.py`, sin coordenadas a mano— y de ella salen el icono de la app, el favicon, el apple-touch-icon y la marca de la cabecera; la escena completa queda como mascota. Se añade la licencia MIT con la marca excluida.
- **Rediseño completo de la interfaz**: sistema *Grafito & Verde*, variante oscura. Una sola familia tipográfica, un único acento, filetes de 1 px, cifras tabulares, controles propios y cero degradados. Sustituye el tema azul marino con degradados violeta→teal y cristal.
- **Copy de los seis pasos**: títulos coherentes con las pestañas, entradillas que explican qué se decide y qué se deduce solo, y avisos que dicen el porqué y qué hacer. `Branding` pasa a `Identidad` y el `Modo advanced` a `Modo avanzado`.
- **Versiones alineadas** en 0.5.0: `forge-core`, `forge-cli`, `forge-wasm`, `web-ui` y `server-rs` estaban en 0.1.0 mientras el proyecto iba por la 0.4.0.
- **README** reescrito con la galería empaquetada, la app de escritorio, el backend y un roadmap real.

### Pendiente

- **Hosting permanente** del backend: hay despliegue verificado (túnel) y automatizado (Fly con `FLY_API_TOKEN`), pero dejar una URL fija y por defecto en la UI sigue necesitando una cuenta de hosting.
- Firma de los paquetes de escritorio y auto-actualización.

## Versiones anteriores

- **v0.4.0** — rewrite completo a Rust: UI Leptos/WASM, backend axum, núcleo re-sincronizado; React/TS y Node pasan a `legacy/` como referencia y suite de paridad.
- **v0.3.0** — backend opcional: cuentas, galería con FTS5 y Test Drive VNC interactivo.
- **v0.2.2** — AutoYaST desatendida para openSUSE, Test Drive con capturas en CI, subida a AWS/Azure y buscador de galería.
- **v0.2.1** — paridad con SUSE Studio: logo, EULA, preseed desatendida, VHD, smoke test en CI, firma cosign y galería comunitaria.
- **v0.2.0** — núcleo Rust (`forge-core`, `forge-cli`, `forge-wasm`) con tests de paridad frente al núcleo TypeScript.
