/**
 * Cliente del servidor DistroForge (opcional). La URL del servidor y el
 * estado de login viven en localStorage; sin servidor configurado, la app
 * funciona igual con la galería estática del repo.
 *
 * La sesión la emite el servidor como cookie httpOnly: fetch con credentials
 * "include" (mismo origen vía proxy en dev, o CORS con credentials en prod).
 */

const URL_KEY = "distroforge-server-url";
const TOKEN_KEY = "distroforge-server-token";

export interface ServerPack {
  id: number;
  owner: string;
  name: string;
  desc: string;
  json: { name?: string; packages: string[]; flatpakApps?: string[] };
  created: string;
  rating: { avg: number; n: number };
}

export function getServerUrl(): string {
  return localStorage.getItem(URL_KEY) ?? "";
}

export function setServerUrl(url: string): void {
  localStorage.setItem(URL_KEY, url.replace(/\/+$/, ""));
}

async function call<T>(path: string, init?: RequestInit): Promise<T> {
  const base = getServerUrl();
  if (!base) throw new Error("sin servidor configurado");
  const token = localStorage.getItem(TOKEN_KEY);
  const res = await fetch(base + path, {
    ...init,
    credentials: "include",
    headers: {
      "content-type": "application/json",
      // token Bearer: las cookies SameSite=Lax no viajan cross-origin en dev
      ...(token ? { authorization: `Bearer ${token}` } : {}),
      ...(init?.headers ?? {}),
    },
  });
  const body = (await res.json().catch(() => ({}))) as T & { error?: string };
  if (!res.ok) throw new Error(body.error ?? `HTTP ${res.status}`);
  return body;
}

export const api = {
  me: () => call<{ user: { name: string } | null }>("/api/me"),
  register: (name: string, password: string) =>
    call<{ name: string; token: string }>("/api/register", {
      method: "POST",
      body: JSON.stringify({ name, password }),
    }).then((r) => (localStorage.setItem(TOKEN_KEY, r.token), r)),
  login: (name: string, password: string) =>
    call<{ name: string; token: string }>("/api/login", {
      method: "POST",
      body: JSON.stringify({ name, password }),
    }).then((r) => (localStorage.setItem(TOKEN_KEY, r.token), r)),
  logout: () =>
    call<{ ok: true }>("/api/logout", { method: "POST" }).then((r) => {
      localStorage.removeItem(TOKEN_KEY);
      return r;
    }),
  search: (query: string) =>
    call<{ packs: ServerPack[] }>(`/api/packs?query=${encodeURIComponent(query)}`),
  publish: (pack: { name: string; desc: string; json: unknown }) =>
    call<{ id: number }>("/api/packs", {
      method: "POST",
      body: JSON.stringify(pack),
    }),
  driveAvailable: () => call<{ available: boolean }>("/api/testdrive/available"),
  driveStart: (imagePath: string) =>
    call<{ id: string; vncUrl: string; expires: number }>("/api/testdrive", {
      method: "POST",
      body: JSON.stringify({ imagePath }),
    }),
  driveStop: (id: string) => call<{ ok: boolean }>(`/api/testdrive/${id}`, { method: "DELETE" }),
};
