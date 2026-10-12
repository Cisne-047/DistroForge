/**
 * Rutas REST del servidor DistroForge.
 *
 *   POST /api/register {name, password}        → crea cuenta y loguea
 *   POST /api/login    {name, password}        → loguea (cookie df_session)
 *   POST /api/logout                            → cierra sesión
 *   GET  /api/me                                → usuario actual o null
 *   GET  /api/packs?query=                      → búsqueda server-side (FTS5)
 *   POST /api/packs       {name, desc, json}    → publica un pack (auth)
 *   POST /api/rating      {packId, stars}       → valora un pack (auth)
 *   GET  /api/testdrive/available               → ¿hay QEMU en el host?
 *   POST /api/testdrive   {imagePath}           → arranca VM (auth)
 *   DELETE /api/testdrive/:id                   → apaga la VM (auth)
 *
 * El cliente noVNC se sirve en /vnc.html y conecta con /ws/:sessionId.
 */
import path from "node:path";
import fastifyCookie from "@fastify/cookie";
import fastifyCors from "@fastify/cors";
import fastifyStatic from "@fastify/static";
import Fastify, { type FastifyInstance } from "fastify";
import { WebSocketServer, type WebSocket } from "ws";
import {
  createSession,
  createUser,
  destroySession,
  findUser,
  userForToken,
  verifyPassword,
} from "./auth.js";
import { openDb, searchPacks, type DatabaseSync } from "./db.js";
import { DriveManager, type DriveSession } from "./testdrive.js";
import { bridge } from "./vncbridge.js";

const COOKIE = "df_session";
const NAME_RE = /^[a-zA-Z0-9_.-]{3,32}$/;

export function buildServer(opts: {
  db?: DatabaseSync;
  qemuBin?: string;
  dataDir?: string;
}): FastifyInstance {
  const db = opts.db ?? openDb(":memory:");
  const drives = new DriveManager(opts.qemuBin ?? "qemu-system-x86_64");
  const app = Fastify({ logger: false });
  void app.register(fastifyCookie);
  // la app (Pages/localhost) llama cross-origin con cookie de sesión
  void app.register(fastifyCors, { origin: true, credentials: true });

  const user = (req: { cookies: Record<string, string | undefined>; headers: Record<string, unknown> }) => {
    const bearer = String(req.headers["authorization"] ?? "").replace(/^Bearer /, "");
    return userForToken(db, bearer || req.cookies[COOKIE]);
  };

  // ---------- auth ----------
  app.post("/api/register", async (req, reply) => {
    const body = req.body as { name?: string; password?: string };
    if (!body?.name || !NAME_RE.test(body.name))
      return reply.code(400).send({ error: "nombre inválido (3-32, letras/números/_.-)" });
    if (!body.password || body.password.length < 8)
      return reply.code(400).send({ error: "contraseña de al menos 8 caracteres" });
    if (findUser(db, body.name)) return reply.code(409).send({ error: "ese nombre ya existe" });
    const u = createUser(db, body.name, body.password);
    const token = createSession(db, u.id);
    reply.setCookie(COOKIE, token, { httpOnly: true, sameSite: "lax", path: "/" });
    return { name: body.name, token };
  });

  app.post("/api/login", async (req, reply) => {
    const body = req.body as { name?: string; password?: string };
    const row = body?.name ? findUser(db, body.name) : undefined;
    if (!row || !body.password || !verifyPassword(body.password, row.pass_hash))
      return reply.code(401).send({ error: "usuario o contraseña incorrectos" });
    const token = createSession(db, row.id);
    reply.setCookie(COOKIE, token, { httpOnly: true, sameSite: "lax", path: "/" });
    return { name: row.name, token };
  });

  app.post("/api/logout", async (req, reply) => {
    destroySession(db, req.cookies[COOKIE]);
    reply.clearCookie(COOKIE, { path: "/" });
    return { ok: true };
  });

  app.get("/api/me", async (req) => ({ user: user(req) ?? null }));

  // ---------- galería ----------
  app.get("/api/packs", async (req) => {
    const q = (req.query as { query?: string }).query ?? "";
    const rows = searchPacks(db, q);
    const ratings = db
      .prepare(
        `SELECT pack_id, ROUND(AVG(stars), 1) AS avg, COUNT(*) AS n
         FROM ratings GROUP BY pack_id`
      )
      .all() as { pack_id: number; avg: number; n: number }[];
    const byId = new Map(ratings.map(({ pack_id, avg, n }) => [pack_id, { avg, n }]));
    return {
      packs: rows.map((p) => ({
        id: p.id,
        owner: p.owner,
        name: p.name,
        desc: p.desc,
        json: JSON.parse(p.json),
        created: p.created,
        rating: byId.get(p.id) ?? { avg: 0, n: 0 },
      })),
    };
  });

  app.post("/api/packs", async (req, reply) => {
    const me = user(req);
    if (!me) return reply.code(401).send({ error: "haz login primero" });
    const body = req.body as { name?: string; desc?: string; json?: unknown };
    if (!body?.name || !Array.isArray((body.json as { packages?: unknown[] })?.packages))
      return reply.code(400).send({ error: "pack inválido: falta name o packages[]" });
    const r = db
      .prepare("INSERT INTO packs (owner, name, desc, json) VALUES (?, ?, ?, ?)")
      .run(me.name, body.name, body.desc ?? "", JSON.stringify(body.json));
    return { id: Number(r.lastInsertRowid) };
  });

  app.post("/api/rating", async (req, reply) => {
    const me = user(req);
    if (!me) return reply.code(401).send({ error: "haz login primero" });
    const body = req.body as { packId?: number; stars?: number };
    if (!body?.packId || !(body.stars && body.stars >= 1 && body.stars <= 5))
      return reply.code(400).send({ error: "packId y stars 1-5 requeridos" });
    db.prepare(
      `INSERT INTO ratings (pack_id, user_id, stars) VALUES (?, ?, ?)
       ON CONFLICT (pack_id, user_id) DO UPDATE SET stars = excluded.stars`
    ).run(body.packId, me.id, Math.floor(body.stars));
    return { ok: true };
  });

  // ---------- test drive (VNC interactivo) ----------
  app.get("/api/testdrive/available", async () => ({ available: await drives.available() }));

  app.post("/api/testdrive", async (req, reply) => {
    const me = user(req);
    if (!me) return reply.code(401).send({ error: "haz login primero" });
    const body = req.body as { imagePath?: string };
    if (!body?.imagePath || !path.isAbsolute(body.imagePath))
      return reply.code(400).send({ error: "imagePath debe ser una ruta absoluta del servidor" });
    if (!(await drives.available()))
      return reply.code(503).send({ error: "QEMU no está instalado en el servidor" });
    const s: DriveSession = drives.start(me.name, body.imagePath);
    return { id: s.id, vncUrl: `/vnc.html?session=${s.id}`, expires: s.expires };
  });

  app.get("/api/testdrive", async (req) => ({ sessions: drives.listFor(user(req)?.name ?? "") }));

  app.delete("/api/testdrive/:id", async (req) => ({
    ok: drives.stop((req.params as { id: string }).id),
  }));

  // puente ws por sesión: /ws/:id
  const wss = new WebSocketServer({ noServer: true });
  app.server.on("upgrade", (req, sock, head) => {
    const url = new URL(req.url ?? "/", "http://x");
    const m = url.pathname.match(/^\/ws\/([0-9a-f-]+)$/);
    const s = m && drives.get(m[1]);
    if (!s) {
      sock.destroy();
      return;
    }
    wss.handleUpgrade(req, sock, head, (ws: WebSocket) => bridge(ws, s.vncPort));
  });

  // ---------- estáticos: cliente noVNC ----------
  // el paquete @novnc/novnc expone sus módulos ES directamente en la raíz
  const novncDist = path.join(process.cwd(), "node_modules", "@novnc", "novnc");
  void app.register(fastifyStatic, { root: novncDist, prefix: "/novnc/" });
  app.get("/vnc.html", async (_req, reply) =>
    reply.type("text/html").send(vncPage())
  );

  return app;
}

/** Página mínima del cliente VNC (usa el noVNC servido en /novnc/). */
function vncPage(): string {
  return `<!doctype html>
<html lang="es"><head><meta charset="utf-8">
<title>DistroForge Test Drive</title>
<style>
  body { margin: 0; background: #0a0e1a; color: #e8ebf2; font: 14px system-ui, sans-serif; }
  #bar { padding: 8px 12px; background: #171c2c; border-bottom: 1px solid #262b36; display: flex; gap: 12px; align-items: center; }
  #bar b { color: #8b7bff; }
  #screen { width: 100vw; height: calc(100vh - 40px); }
</style></head>
<body>
<div id="bar"><b>DistroForge Test Drive</b><span id="status">conectando…</span></div>
<div id="screen"></div>
<script type="module">
  import RFB from '/novnc/core/rfb.js';
  const sid = new URLSearchParams(location.search).get('session');
  const proto = location.protocol === 'https:' ? 'wss' : 'ws';
  const rfb = new RFB(document.getElementById('screen'), proto + '://' + location.host + '/ws/' + sid);
  rfb.addEventListener('connect', () => document.getElementById('status').textContent = 'conectado');
  rfb.addEventListener('disconnect', (e) => {
    document.getElementById('status').textContent = e.detail.clean ? 'sesión cerrada' : 'desconectado';
  });
</script>
</body></html>`;
}
