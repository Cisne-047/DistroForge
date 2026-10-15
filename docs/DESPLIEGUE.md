# Desplegar el backend de DistroForge

El backend (`rust/server-rs`) da cuentas, galería con FTS5 y Test Drive VNC. La
app web funciona sin él (todo lo demás se genera en el navegador con WASM): si
no hay backend, la sección *Servidor* simplemente queda sin usar.

## Variables de entorno

| Variable | Por defecto | Para qué |
|---|---|---|
| `PORT` | `8080` | Puerto HTTP |
| `DF_DB_PATH` | `:memory:` | Ruta del SQLite. **En un despliegue real, en un volumen**: con `:memory:` las cuentas y los packs desaparecen al reiniciar |
| `QEMU_BIN` | `qemu-system-x86_64` | Binario de QEMU para el Test Drive |

Comprobación rápida: `GET /api/health` → `{"status":"ok","service":"distroforge-server","version":"0.5.0"}`.

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
