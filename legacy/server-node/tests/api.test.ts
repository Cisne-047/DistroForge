import { afterEach, beforeEach, describe, expect, it } from "vitest";
import { openDb, type DatabaseSync } from "../src/db.js";
import { buildServer } from "../src/routes.js";
import type { FastifyInstance } from "fastify";

let app: FastifyInstance;
let db: DatabaseSync;

beforeEach(() => {
  db = openDb(":memory:");
  app = buildServer({ db, qemuBin: "qemu-inexistente" });
});

afterEach(async () => {
  await app.close();
});

const register = (name = "tester", password = "password123") =>
  app.inject({ method: "POST", url: "/api/register", payload: { name, password } });

describe("auth", () => {
  it("registra, loguea y devuelve el usuario actual", async () => {
    const r = await register();
    expect(r.statusCode).toBe(200);
    const cookie = r.cookies.find((c) => c.name === "df_session")?.value;
    expect(cookie).toBeTruthy();
    const me = await app.inject({ method: "GET", url: "/api/me", cookies: { df_session: cookie! } });
    expect(me.json()).toEqual({ user: { id: expect.any(Number), name: "tester" } });
  });

  it("rechaza nombres inválidos, contraseñas cortas y duplicados", async () => {
    expect((await register("x", "password123")).statusCode).toBe(400);
    expect((await register("buen", "corta")).statusCode).toBe(400);
    await register();
    expect((await register()).statusCode).toBe(409);
  });

  it("login falla con credenciales equivocadas y logout cierra la sesión", async () => {
    await register();
    const bad = await app.inject({
      method: "POST",
      url: "/api/login",
      payload: { name: "tester", password: "incorrecta" },
    });
    expect(bad.statusCode).toBe(401);
    const ok = await app.inject({
      method: "POST",
      url: "/api/login",
      payload: { name: "tester", password: "password123" },
    });
    const cookie = ok.cookies.find((c) => c.name === "df_session")!.value;
    await app.inject({ method: "POST", url: "/api/logout", cookies: { df_session: cookie } });
    const me = await app.inject({ method: "GET", url: "/api/me", cookies: { df_session: cookie } });
    expect(me.json().user).toBeNull();
  });
});

describe("galería con búsqueda server-side", () => {
  async function loginCookie(): Promise<string> {
    const r = await register("autor", "password123");
    return r.cookies.find((c) => c.name === "df_session")!.value;
  }

  it("publica packs (auth) y los busca por texto", async () => {
    const cookie = await loginCookie();
    const mk = (name: string, desc: string, packages: string[]) =>
      app.inject({
        method: "POST",
        url: "/api/packs",
        cookies: { df_session: cookie },
        payload: { name, desc, json: { name, packages } },
      });
    expect((await mk("Gaming total", "steam y lutris", ["steam", "lutris"])).statusCode).toBe(200);
    expect((await mk("Servidor NAS", "samba y nfs", ["samba", "nfs-kernel-server"])).statusCode).toBe(200);
    expect((await mk("roto", "sin packages", {})).statusCode).toBe(400);

    const q = await app.inject({ method: "GET", url: "/api/packs?query=steam" });
    const names = q.json().packs.map((p: { name: string }) => p.name);
    expect(names).toContain("Gaming total");
    expect(names).not.toContain("Servidor NAS");

    const all = await app.inject({ method: "GET", url: "/api/packs" });
    expect(all.json().packs).toHaveLength(2);
  });

  it("sin login no se puede publicar ni valorar", async () => {
    expect((await app.inject({ method: "POST", url: "/api/packs", payload: { name: "x", json: { packages: [] } } })).statusCode).toBe(401);
    expect((await app.inject({ method: "POST", url: "/api/rating", payload: { packId: 1, stars: 5 } })).statusCode).toBe(401);
  });

  it("valora packs y el promedio aparece en la búsqueda", async () => {
    const cookie = await loginCookie();
    const r = await app.inject({
      method: "POST",
      url: "/api/packs",
      cookies: { df_session: cookie },
      payload: { name: "media", desc: "video", json: { packages: ["vlc"] } },
    });
    const id = r.json().id;
    await app.inject({
      method: "POST",
      url: "/api/rating",
      cookies: { df_session: cookie },
      payload: { packId: id, stars: 4 },
    });
    // re-valorar actualiza, no duplica
    await app.inject({
      method: "POST",
      url: "/api/rating",
      cookies: { df_session: cookie },
      payload: { packId: id, stars: 5 },
    });
    const q = await app.inject({ method: "GET", url: "/api/packs?query=video" });
    expect(q.json().packs[0].rating).toEqual({ avg: 5, n: 1 });
  });
});

describe("test drive", () => {
  it("available informa de QEMU ausente y start devuelve 503 sin QEMU", async () => {
    const r = await app.inject({ method: "GET", url: "/api/testdrive/available" });
    expect(r.json().available).toBe(false);
    const reg = await register();
    const cookie = reg.cookies.find((c) => c.name === "df_session")!.value;
    const start = await app.inject({
      method: "POST",
      url: "/api/testdrive",
      cookies: { df_session: cookie },
      payload: { imagePath: "/tmp/noexiste.iso" },
    });
    expect(start.statusCode).toBe(503);
  });

  it("sin login o con ruta relativa rechaza", async () => {
    expect((await app.inject({ method: "POST", url: "/api/testdrive", payload: { imagePath: "/x" } })).statusCode).toBe(401);
    const reg = await register();
    const cookie = reg.cookies.find((c) => c.name === "df_session")!.value;
    const rel = await app.inject({
      method: "POST",
      url: "/api/testdrive",
      cookies: { df_session: cookie },
      payload: { imagePath: "relativa.iso" },
    });
    expect(rel.statusCode).toBe(400);
  });
});
