# DistroForge server-rs

Réplica en Rust del backend Node/Fastify de `server/`. Misma API REST, misma
semántica, mismo esquema SQLite (incluido FTS5 con triggers) y mismo formato de
hash de contraseñas (`salt:hash` hex de scrypt, N=16384 r=8 p=1, sal de 16
bytes usada como texto, clave de 32 bytes — idéntico a
`crypto.scryptSync(password, salt, 32)`).

Proyecto cargo **independiente** (`[workspace]` vacío en `Cargo.toml`): no
pertenece al workspace de `rust/Cargo.toml`.

## Ejecutar

```sh
cargo run --release            # escucha en 0.0.0.0:8080
PORT=3000 cargo run            # otro puerto
QEMU_BIN=/usr/bin/qemu-system-x86_64 cargo run   # binario QEMU concreto
DF_DB_PATH=df.db cargo run     # SQLite en disco (por defecto :memory:)
```

Variables de entorno: `PORT` (default 8080), `QEMU_BIN`
(default `qemu-system-x86_64`), `DF_DB_PATH` (default `:memory:`).

Docker (debian-slim + qemu-system-x86):

```sh
docker build -t distroforge-server .
docker run -p 8080:8080 distroforge-server
```

## API

| Método | Ruta | Auth | Descripción |
|---|---|---|---|
| POST | `/api/register` `{name,password}` | — | valida nombre `^[a-zA-Z0-9_.-]{3,32}$`, password >= 8; 409 si existe; cookie `df_session` httpOnly SameSite=Lax (30 días) + `{name,token}` |
| POST | `/api/login` `{name,password}` | — | 401 si mal; cookie + `{name,token}` |
| POST | `/api/logout` | — | destruye sesión, limpia cookie, `{ok:true}` |
| GET | `/api/me` | cookie o `Authorization: Bearer <token>` | `{user:{id,name}}` o `{user:null}` |
| GET | `/api/packs?query=` | — | FTS5 (`packs_fts` + triggers, mismo DDL que `server/src/db.ts`) con fallback LIKE; `{packs:[{id,owner,name,desc,json,created,rating:{avg,n}}]}` |
| POST | `/api/packs` `{name,desc,json}` | sí | valida `name` y `json.packages[]`; `{id}` |
| POST | `/api/rating` `{packId,stars 1-5}` | sí | upsert `(pack_id,user_id)`; `{ok:true}` |
| GET | `/api/testdrive/available` | — | `{available}` (lanza `qemu-system-x86_64 --version`) |
| POST | `/api/testdrive` `{imagePath}` | sí | 400 ruta relativa, 503 sin QEMU; lanza QEMU `-m 2048 -display none -vnc 127.0.0.1:<display> -snapshot <path>`; `{id,vncUrl:"/vnc.html?session=<id>",expires}` |
| GET | `/api/testdrive` | — | `{sessions:[{id,expires}]}` del dueño |
| DELETE | `/api/testdrive/{id}` | — | mata la VM; `{ok}` |
| GET | `/vnc.html` | — | página del cliente VNC |
| GET (WS) | `/ws/{id}` | — | puente WebSocket ↔ TCP al puerto VNC de la sesión |

TTL de sesiones de test drive: 15 min, con barrido periódico y al morir QEMU
se elimina la sesión. CORS: cualquier origen espejado con credenciales
(equivale a `origin:true, credentials:true`).

## Estructura

- `src/main.rs` — arranque: PORT, QEMU_BIN, sweeper periódico, `axum::serve`.
- `src/routes.rs` — router + handlers (réplica de `server/src/routes.ts`).
- `src/db.rs` — SQLite: esquema, FTS5, `search_packs` (réplica de `db.ts`).
- `src/auth.rs` — scrypt `salt:hash`, sesiones 24 bytes hex (réplica de `auth.ts`).
- `src/testdrive.rs` — `DriveManager`: QEMU + TTL + displays 100+ (réplica de `testdrive.ts`).
- `src/vncbridge.rs` — puente ws↔TCP (websockify mínimo, réplica de `vncbridge.ts`).
- `tests/api.rs` — los 8 tests de `server/tests/api.test.ts` replicados.

## Decisiones y diferencias con el servidor Node

1. **noVNC desde CDN**: el servidor TS sirve `@novnc/novnc` desde
   `node_modules` en `/novnc/*`; aquí no hay node_modules, así que `/vnc.html`
   carga noVNC 1.5.0 por import ESM desde
   `https://cdn.jsdelivr.net/npm/@novnc/novnc@1.5.0/lib/rfb.js`. El puente
   `/ws/{id}` es idéntico, por lo que un noVNC autoalojado también funcionaría.
2. **WebSocket**: el upgrade de axum (`axum::extract::ws`, corre sobre
   `tokio-tungstenite`) puentea directo al puerto VNC de la sesión; el TS
   además abría un `WebSocketServer` efímero por sesión que nadie usaba — no
   se replica.
3. **Rutas absolutas**: `imagePath` se valida con `is_absolute() || has_root()`
   para igualar a `path.isAbsolute` de Node también en Windows ("/tmp/x" vale).
4. **FKs ON**: `node:sqlite` activa foreign keys por defecto; se replica con
   `PRAGMA foreign_keys=ON`.
5. **Cookie**: además de `HttpOnly; SameSite=Lax; Path=/` lleva
   `Max-Age=2592000` (30 días, la vida real de la sesión en BD).

## Tests

```sh
cargo test                  # 8 tests de integración (réplica del vitest)
cargo clippy --all-targets -- -D warnings
```
