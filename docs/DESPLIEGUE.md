# Desplegar el backend de DistroForge

El backend (`rust/server-rs`) da cuentas, galería con FTS5 y Test Drive VNC. La
app web funciona sin él (todo lo demás se genera en el navegador con WASM): si
no hay backend, la sección *Servidor* simplemente queda sin usar.

## Sin cuentas: servidor propio + túnel

Si no quieres dar de alta nada, el backend puede correr en tu máquina y
publicarse con un túnel rápido de Cloudflare (no necesita cuenta):

```bash
cd rust/web-ui && trunk build --release --public-url / --dist dist-tunnel
DF_WEB_DIR=$PWD/rust/web-ui/dist-tunnel PORT=8090 cargo run --release   # desde rust/server-rs
cloudflared tunnel --url http://localhost:8090
```

O todo de una vez, en Windows:

```powershell
pwsh tools/serve_tunnel.ps1          # compila la UI, arranca el servidor y abre el túnel
```

Imprime una URL tipo `https://palabras-aleatorias.trycloudflare.com` que sirve
**la aplicación entera** (interfaz + API). Es la forma de probarlo de verdad: al
ir todo por el mismo origen, la cookie de sesión viaja y las cuentas, la galería
compartida y el Test Drive funcionan sin configurar la URL en la app.

Dos avisos honestos: los túneles rápidos **no tienen garantía de disponibilidad**
y la URL cambia en cada arranque, y el Test Drive necesita QEMU (y KVM para ser
usable), así que en un portátil lo normal es que la API responda
`{"available": false}`.

## La vía permanente: un secreto y un tag

`.github/workflows/deploy-server.yml` despliega en Fly.io sin que nadie tenga que
pasar credenciales por ningún sitio:

1. Crea la app y el volumen una sola vez (necesita tu cuenta):
   `fly launch --no-deploy --copy-config --name distroforge-server` dentro de
   `rust/server-rs`, y `fly volumes create df_data --size 1`.
2. Guarda el token en **Settings → Secrets and variables → Actions** como
   `FLY_API_TOKEN` (`fly auth token`).
3. Empuja un tag `v*` o lanza el workflow a mano.

El workflow **no falla si el secreto no existe**: lo dice por el log y se queda
en verde, para no llenar de avisos el repositorio. Al terminar comprueba
`https://<app>.fly.dev/api/health`.

## La interfaz, servida por el backend

Con `DF_WEB_DIR` apuntando a un directorio con la interfaz compilada, el servidor
la sirve junto a la API. Es lo que usan el túnel y la imagen de Docker (que
incluye la UI en `/app/web`), y la razón es la cookie: entre dominios distintos
el navegador no la envía.

Para compilar la interfaz de forma que funcione servida en la raíz hay que pasar
`--public-url /` (por defecto el proyecto usa `/DistroForge/` para GitHub Pages).

## Probar la imagen en un sandbox (sin Docker local)

Si no tienes Docker en tu máquina, un sandbox de [instabox](https://instabox.sh)
sirve para construir **y ejecutar** la imagen de verdad (4 vCPU, 8 GB y Docker ya
instalado):

```bash
instabox create --name distroforge-docker --template universal --timeout 3h
instabox exec distroforge-docker -- 'docker --version'      # ya viene instalado
# sube las fuentes (rust/server-rs + la UI compilada) y construye:
instabox exec distroforge-docker -- 'cd ~/df && docker build -f rust/server-rs/Dockerfile -t distroforge-server .'
instabox exec distroforge-docker -- 'docker run -d --name df -p 8080:8080 -e DF_DB_PATH=/data/df.db distroforge-server'
instabox exec distroforge-docker -- 'curl -fsS localhost:8080/api/health'
instabox port publish distroforge-docker 8080 --name distroforge   # URL pública HTTPS
```

La clave se gestiona con `instabox keys` y se pasa en `INSTABOX_API_KEY`; no debe
quedar en el repositorio.

El repositorio lo automatiza en `.github/workflows/image-smoke.yml`: construye la
imagen y la ejecuta en un sandbox (salud, interfaz, estáticos, 404 de API,
registro con sesión y estado `healthy` del contenedor). Necesita el secreto
`INSTABOX_API_KEY` y borra el sandbox al terminar; sin el secreto avisa y se queda
en verde en lugar de fallar.

Es la forma de detectar problemas que solo aparecen al **ejecutar** el
contenedor: así salió que compilar en `rust:1-slim` (trixie, glibc 2.39) y
ejecutar en `debian:bookworm-slim` (2.36) dejaba al binario pidiendo símbolos
inexistentes. El sandbox también trae QEMU, así que `testdrive/available` responde
`true` y el Test Drive se puede probar de verdad.

## URL por defecto en la app

Que nadie tenga que escribir la URL a mano: el deploy de Pages hornea la del
backend con la **variable del repositorio** `DISTROFORGE_SERVER_URL`
(Settings → Variables → Actions), que se pasa al compilar la interfaz.

El orden de prioridad al resolver el servidor es:

1. lo que haya configurado el usuario en la sección *Servidor* (localStorage),
2. la URL horneada en el build,
3. el propio origen, para cuando el backend sirve la interfaz (imagen de Docker,
   túnel o sandbox): ahí la API está en la misma URL y no hay nada que configurar.

## Variables de entorno

| Variable | Por defecto | Para qué |
|---|---|---|
| `PORT` | `8080` | Puerto HTTP |
| `DF_DB_PATH` | `:memory:` | Ruta del SQLite. **En un despliegue real, en un volumen**: con `:memory:` las cuentas y los packs desaparecen al reiniciar |
| `QEMU_BIN` | `qemu-system-x86_64` | Binario de QEMU para el Test Drive |
| `DF_WEB_DIR` | — | Directorio con la interfaz web compilada; si existe, se sirve en el mismo origen que la API |
| `DF_ISO_DIR` | — | Directorio del que el Test Drive acepta imágenes. Sin ella solo se admiten rutas absolutas existentes con extensión `.iso/.img/.qcow2/.raw` fuera de `/proc`, `/sys` y `/dev`. En un servidor público, defínela a un directorio con las ISOs |

El Test Drive canonicaliza la ruta (los symlinks se resuelven antes de comprobar el
directorio), exige un fichero regular y limita las sesiones simultáneas. La sesión
de la API se marca `Secure` cuando la petición trae `X-Forwarded-Proto: https`, y la
API responde con CSP estricta salvo en `/vnc.html` (noVNC se carga desde CDN).

Comprobación rápida: `GET /api/health` → `{"status":"ok","service":"distroforge-server","version":"1.0.0"}`.

## Imagen

`.github/workflows/server-image.yml` publica en GHCR:

```
ghcr.io/cisne-047/distroforge-server:latest
```

También se puede construir en local:

```bash
docker build -t distroforge-server rust/server-rs
docker run --rm -p 8080:8080 -v df-data:/data distroforge-server
# o, con el compose del repo:
cd rust/server-rs && docker compose up --build
```

## Opciones de despliegue

### Fly.io

```bash
fly launch --no-deploy --dockerfile rust/server-rs/Dockerfile --name distroforge-server
fly volumes create df_data --size 1
# fly.toml: [env] PORT=8080 DF_DB_PATH=/data/df.db  + [[mounts]] source="df_data" destination="/data"
fly deploy
```

El Test Drive necesita KVM: en Fly requiere una máquina con virtualización
(`fly platform vm` / planes con KVM). Sin KVM, QEMU arranca por emulación y es
lento, así que es razonable desplegar sin Test Drive y dejar el resto.

### Render / Railway

- Tipo: *Web Service* desde el `Dockerfile` de `rust/server-rs`.
- Health check: `/api/health`.
- Disco persistente montado en `/data` y `DF_DB_PATH=/data/df.db`.

### VPS propio

```bash
git clone https://github.com/Cisne-047/DistroForge && cd DistroForge/rust/server-rs
docker compose up -d --build      # descomenta el devices:/dev/kvm en Linux
```

Detrás de un proxy (Caddy/Nginx) que dé TLS al dominio.

## Cablear la URL en la app

En la app web, sección **Servidor**, escribe la URL del backend
(`https://api.tudominio.tld`); se guarda en `localStorage` del navegador.

### Aviso sobre cookies entre dominios

La sesión viaja en una cookie `df_session` con `SameSite=Lax`. Si la UI (Pages)
y la API están en **dominios distintos**, el navegador no envía esa cookie en las
peticiones `fetch`, así que el login no se mantiene. Dos salidas:

1. **Mismo origen** (recomendado): sirve la UI y la API bajo el mismo dominio
   (por ejemplo, el proxy sirve `/` desde Pages y `/api/*` al backend, o el
   propio backend sirve `rust/web-ui/dist`).
2. **Token**: el backend también acepta `Authorization: Bearer <token>`; la UI
   tendría que guardar el token del login y enviarlo en cada petición.

Es el primer punto a resolver al desplegar; el resto (CORS con credenciales) ya
funciona, porque el backend espeja el origen de la petición.
