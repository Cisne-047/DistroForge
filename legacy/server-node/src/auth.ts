/**
 * Autenticación: hash scrypt con sal por usuario, sesiones con token opaco
 * guardado en cookie httpOnly. Sin dependencias externas (node:crypto).
 */
import { randomBytes, scryptSync, timingSafeEqual } from "node:crypto";
import type { DatabaseSync } from "./db.js";

const SESSION_DAYS = 30;

export function hashPassword(password: string): string {
  const salt = randomBytes(16).toString("hex");
  const hash = scryptSync(password, salt, 32).toString("hex");
  return `${salt}:${hash}`;
}

export function verifyPassword(password: string, stored: string): boolean {
  const [salt, hash] = stored.split(":");
  if (!salt || !hash) return false;
  const candidate = scryptSync(password, salt, 32);
  const expected = Buffer.from(hash, "hex");
  return candidate.length === expected.length && timingSafeEqual(candidate, expected);
}

export function createUser(db: DatabaseSync, name: string, password: string): { id: number } {
  const r = db
    .prepare("INSERT INTO users (name, pass_hash) VALUES (?, ?)")
    .run(name, hashPassword(password));
  return { id: Number(r.lastInsertRowid) };
}

export function findUser(db: DatabaseSync, name: string) {
  return db.prepare("SELECT id, name, pass_hash FROM users WHERE name = ?").get(name) as
    | { id: number; name: string; pass_hash: string }
    | undefined;
}

/** Crea una sesión y devuelve el token (guárdalo en cookie). */
export function createSession(db: DatabaseSync, userId: number): string {
  const token = randomBytes(24).toString("hex");
  db.prepare(
    `INSERT INTO sessions (token, user_id, expires)
     VALUES (?, ?, datetime('now', '+${SESSION_DAYS} days'))`
  ).run(token, userId);
  return token;
}

/** Resuelve el usuario de una sesión válida; undefined si el token no existe o caducó. */
export function userForToken(db: DatabaseSync, token: string | undefined) {
  if (!token) return undefined;
  return db
    .prepare(
      `SELECT u.id, u.name FROM sessions s JOIN users u ON u.id = s.user_id
       WHERE s.token = ? AND s.expires > datetime('now')`
    )
    .get(token) as { id: number; name: string } | undefined;
}

export function destroySession(db: DatabaseSync, token: string | undefined): void {
  if (token) db.prepare("DELETE FROM sessions WHERE token = ?").run(token);
}
