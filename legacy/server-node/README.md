# Servidor DistroForge

Backend opcional que añade lo que la app web estática no puede hacer sola:
**galería con cuentas y búsqueda server-side** y **Test Drive interactivo**
(arranca tu imagen en QEMU y la manejas desde el navegador con noVNC).

Sin él, la app funciona igual (galería comunitaria vía `gallery/` del repo y
Test Drive por capturas en CI).

## Requisitos

- Node ≥ 24 (usa `node:sqlite` nativo)
- Opcional: `qemu-system-x86_64` en el PATH para el Test Drive

## Desarrollo

```bash
cd server
npm install
npm test          # tests de API con fastify.inject (no abren puertos)
npm run dev       # escucha en :8080 con recarga
```

## Despliegue

```bash
docker build -t distroforge-server .
docker run -p 8080:8080 -v df-data:/app/data distroforge-server
```

Las cookies de sesión van firmadas por cookie httpOnly; la base SQLite vive
en memoria salvo que pases `DB_PATH` (extiende `openDb` con la ruta deseada).

## API

| Método | Ruta | Descripción |
|---|---|---|
| POST | `/api/register` | crea cuenta `{name, password}` y loguea |
| POST | `/api/login` | login (cookie `df_session`) |
| POST | `/api/logout` | cierra sesión |
| GET | `/api/me` | usuario actual |
| GET | `/api/packs?query=` | búsqueda server-side (FTS5, con ratings) |
| POST | `/api/packs` | publica un pack (auth) |
| POST | `/api/rating` | valora 1-5 (auth) |
| GET | `/api/testdrive/available` | ¿hay QEMU en el host? |
| POST | `/api/testdrive` | arranca VM `{imagePath}` (auth, ruta absoluta del servidor) |
| GET | `/api/testdrive` | sesiones propias |
| DELETE | `/api/testdrive/:id` | apaga la VM |
| GET | `/vnc.html?session=` | cliente noVNC de la sesión |

Las sesiones de Test Drive viven 15 minutos (TTL) y se limpian solas.
