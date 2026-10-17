# Historial de cambios

Las etiquetas del repositorio siguen `vX.Y.Z`. Cada versión se publica desde su etiqueta, con los paquetes firmados y la ISO construida y arrancada en CI.

## [Sin publicar]

### Cambiado

- **Solo queda Rust**: se elimina el código heredado (la app React/TS y el backend
  Node) y su job de CI. El repositorio contiene únicamente la implementación
  actual: núcleo, CLI, UI Leptos/WASM, backend axum y app de escritorio Tauri.

### Seguridad

- **Núcleo (`forge-core`)**: módulo `security` con validadores para locale, zona
  horaria, teclado, paquetes, URLs, suites, contraseñas, parámetros de GRUB,
  servicios, puertos, unidades systemd y rutas. `validate()` los aplica y el CLI y
  el WASM **rechazan** una config inválida; la UI ya lo hacía. Todas las
  interpolaciones de las recetas (live-build, kickstart, KIWI, archiso, Dockerfile,
  WSL, cloud-init, Flatpak, NetworkManager) pasan por `sq`/`dq`, los heredocs usan
  marcadores únicos que el contenido no puede cerrar, el YAML de cloud-init se
  comilla como JSON y el CLI confina las rutas al directorio de salida.
- **Backend**: se eliminan los `expect` bajo el candado de SQLite (y el candado
  tolera envenenamiento), el alta duplicada responde 409 en vez de 500, scrypt
  corre fuera del candado y en `spawn_blocking`, login y registro tienen límite de
  intentos, el Test Drive canonicaliza la ruta de la imagen, la confina a
  `DF_ISO_DIR`, exige extensión de imagen, limita sesiones y comprueba dueño en
  `DELETE` y en el WebSocket, la API limita cuerpos a 256 KB y packs a 64/2048
  caracteres, añade cabeceras de seguridad y CSP, marca la cookie `Secure` tras un
  proxy HTTPS y deja de filtrar errores de SQL/ficheros.
- **Escritorio y CI**: la app Tauri pasa de `"csp": null` a una CSP real; los
  workflows declaran permisos mínimos, los secretos se limitan al step que los usa,
  `flyctl` queda fijado a un commit y se validan el tag de release y los nombres de
  paquete antes de usarlos en comandos.

## [1.0.0] - 2026-10-17

Primera versión estable. Lo que la hace 1.0 no es una función nueva, sino que todo
lo que promete el README está hecho **y comprobado**:

- La app web y la de escritorio generan proyectos para 14 bases y 4 motores, con 8
  formatos de salida. El núcleo pasa 80 tests.
- **Una ISO real se hornea y arranca en CI** en cada release: estructura (El
  Torito), contenido (lo configurado dentro del squashfs) y arranque en QEMU, con
  la ISO como artefacto descargable.
- **Auto-actualización** de la app de escritorio, con paquetes firmados con claves
  propias y el manifiesto servido en un host público. Verificado descargando el
  paquete publicado y validando su firma con la clave pública.
- **Pruebas de interfaz** con navegador real sobre la aplicación construida.
- **Backend desplegado** —cuentas, galería compartida y Test Drive con QEMU— y la
  URL por defecto horneada en la app web.

Limitaciones conocidas, dichas claramente: los instaladores **no están firmados a
nivel de sistema** (requiere un certificado de pago con identidad verificada), no
hay build de **macOS** (requiere cuenta de Apple) y el backend vive en un
**sandbox**, así que su URL depende de que siga en pie.
## [0.5.1] - 2026-10-17

### Arreglado

- **Ningún proyecto Debian se construía.** El `auto/config` generado pasaba `--grub-timeout` a `lb config`, opción que no existe: `unrecognized option` y el build moría en el primer paso. Lo encontró el primer horneado real de una ISO en CI, que hasta ahora no existía.
- **El workflow de CI que se generaba para live-build corría sin `--privileged`**, pero `lb build` monta `/proc` y `/dev/pts` dentro del chroot y fallaba con `mount: permission denied`. El `build.sh` local ya lo llevaba.
- **Ningún hook se ejecutaba.** Se generaban en `config/hooks/*.hooks.chroot` y live-build solo recorre `config/hooks/{normal,live}/*.<fase>`: el usuario y la contraseña, los repos, los servicios, Flatpak, la red y el fondo no llegaban a la imagen.
- **Un repo personalizado rompía el build.** El hook escribía la línea con comillas simples, así que `$SIGNED` quedaba literal en el fichero, apt no podía leer el origen (`The list of sources could not be read`) y el build moría.
- **El timeout de arranque no hacía nada**: solo se aplicaba a GRUB, y en arranque BIOS manda isolinux, cuyo `timeout 0` significa *esperar siempre*.
- Cuatro avisos de `clippy --all-targets -D warnings` (el comando del CI) que habrían roto la integración al actualizar el toolchain.

### Añadido

- **Auto-actualización del escritorio**: la app comprueba al arrancar si hay versión nueva, la instala y se reinicia. Paquetes firmados con claves propias y manifiesto publicado en el host público. La firma *a nivel de sistema* sigue necesitando un certificado de pago, y así queda documentado.
- **Una ISO real de punta a punta en CI**: se hornea con live-build, se comprueba que es arrancable (El Torito), que dentro está lo configurado y que **arranca en QEMU**. Sube la ISO y la captura del arranque.
- **Pruebas de interfaz** con Playwright contra la aplicación construida (WASM incluido), en local y en CI.
- **Modo claro** de *Grafito & Verde*, con conmutador en la cabecera y `?tema=claro|oscuro`.
- **URL del backend por defecto**, horneada desde una variable del repositorio: la app de Pages ya no pide configuración para cuentas, galería compartida y Test Drive.
## v0.5.0 (sin publicar)

### Arreglado

- **Ningún proyecto Debian se construía.** El `auto/config` que generaba DistroForge pasaba `--grub-timeout` a `lb config`, que no acepta esa opción: `lb config` abortaba con `unrecognized option` y el build moría en el primer paso. Lo encontró el primer horneado real de una ISO en CI —los 77 tests comparaban el texto generado, y uno de ellos **consagraba el fallo** (`assert!(auto.contains("--grub-timeout 7"))`)—. Ahora el timeout se aplica con un hook de la fase *binary* sobre el menú ya generado, y el test comprueba el comportamiento.
- **Galería comunitaria en producción.** La app pedía los packs a `raw.githubusercontent` del propio repositorio, que es privado: el fetch sin token devolvía 404 y la galería no cargaba. Ahora Trunk empaqueta `gallery/` dentro del bundle y la UI la lee por ruta relativa, así que funciona en Pages, en `trunk serve` y en la app de escritorio (que no tiene red).
- **Empaquetado de escritorio.** El workflow de release nunca se había ejecutado y no podía funcionar: `tauri.conf.json` no tenía sección `bundle` ni iconos, y `beforeBuildCommand` corría desde `rust/tauri-app` (no desde `src-tauri`), así que la ruta relativa a la UI no resolvía.
- **ZIP sin conexión.** `fflate` se carga desde una copia local (0.8.2, MIT) en lugar de un CDN: el escritorio ya puede generar el zip.
- La ayuda de la galería describía el comportamiento antiguo ("se cargan desde GitHub").

### Añadido

- **App de escritorio publicable**: matriz de paquetes Linux (`.deb`, `.rpm`, `.AppImage`) y Windows (`.msi`, `.exe`), artefactos en cada ejecución y GitHub Release al empujar un tag. Iconos generados de forma reproducible con `tools/make_icons.py`.
- **Servidor todo-en-uno**: con `DF_WEB_DIR`, `server-rs` sirve la interfaz junto a la API **en el mismo origen**, que es lo que hace que la cookie de sesión viaje: cuentas, galería compartida y Test Drive funcionan sin configurar ninguna URL. `tools/serve_tunnel.ps1` lo publica con un túnel de Cloudflare sin necesidad de cuentas.
- **Backend listo para desplegar**: `GET /api/health` para el healthcheck, Dockerfile con la capa de dependencias cacheada, usuario sin privilegios, volumen para el SQLite y `HEALTHCHECK`; `docker-compose.yml`; workflow que publica la imagen en GHCR; y [docs/DESPLIEGUE.md](docs/DESPLIEGUE.md) con las opciones de hosting y el aviso sobre cookies entre dominios.
- **Imagen Docker con la interfaz dentro**, verificada **en ejecución** y no solo construida: así se detectó que el compilador (`rust:1-slim`, glibc 2.39) y el runtime (`debian:bookworm-slim`, 2.36) no coincidían y el binario pedía símbolos inexistentes. Ahora ambas bases son bookworm y el contenedor arranca sin avisos. Incluye cómo probarlo en un sandbox cuando no hay Docker local.
- **Manual técnico reproducible** (`tools/make_manual.py` → DOCX + PDF): antes el PDF era anterior al rewrite y describía la app React.
- **Prueba de imagen en CI** (`image-smoke.yml`): construye la imagen y la **ejecuta** en un sandbox, comprobando salud, interfaz, estáticos, 404 en JSON, registro con sesión y `healthy` del contenedor. Sin el secreto de instabox avisa y se queda en verde.
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
