/**
 * Capa de datos: SQLite embebido (node:sqlite, sin dependencias nativas).
 * Tablas: usuarios, sesiones de login, packs de la galería y valoraciones.
 * La búsqueda usa FTS5 (índice de texto sobre nombre+descripción+paquetes).
 */
import { createRequire } from "node:module";

/** Superficie de node:sqlite que usamos (DatabaseSync), tipada a mano para no
 *  depender de que el grafo de vite conozca el builtin. */
export interface DatabaseSync {
  exec(sql: string): void;
  prepare(sql: string): {
    run(...args: unknown[]): { lastInsertRowid: number | bigint };
    all(...args: unknown[]): unknown[];
    get(...args: unknown[]): unknown;
  };
}

const { DatabaseSync } = createRequire(import.meta.url)("node:sqlite") as {
  DatabaseSync: new (path: string) => DatabaseSync;
};

export interface UserRow {
  id: number;
  name: string;
  pass_hash: string;
  created: string;
}

export interface PackRow {
  id: number;
  owner: string;
  name: string;
  desc: string;
  json: string;
  created: string;
}

export interface SessionRow {
  token: string;
  user_id: number;
  expires: string;
}

const SCHEMA = `
CREATE TABLE IF NOT EXISTS users (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  name TEXT UNIQUE NOT NULL,
  pass_hash TEXT NOT NULL,
  created TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS sessions (
  token TEXT PRIMARY KEY,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  expires TEXT NOT NULL
);
CREATE TABLE IF NOT EXISTS packs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  owner TEXT NOT NULL,
  name TEXT NOT NULL,
  desc TEXT NOT NULL DEFAULT '',
  json TEXT NOT NULL,
  created TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE TABLE IF NOT EXISTS ratings (
  pack_id INTEGER NOT NULL REFERENCES packs(id) ON DELETE CASCADE,
  user_id INTEGER NOT NULL REFERENCES users(id) ON DELETE CASCADE,
  stars INTEGER NOT NULL CHECK (stars BETWEEN 1 AND 5),
  PRIMARY KEY (pack_id, user_id)
);
CREATE VIRTUAL TABLE IF NOT EXISTS packs_fts USING fts5(
  name, desc, body, content='packs', content_rowid='id', tokenize='unicode61'
);
CREATE TRIGGER IF NOT EXISTS packs_ai AFTER INSERT ON packs BEGIN
  INSERT INTO packs_fts(rowid, name, desc, body)
  VALUES (new.id, new.name, new.desc, new.json);
END;
CREATE TRIGGER IF NOT EXISTS packs_ad AFTER DELETE ON packs BEGIN
  INSERT INTO packs_fts(packs_fts, rowid, name, desc, body)
  VALUES ('delete', old.id, old.name, old.desc, old.json);
END;
`;

export function openDb(path = ":memory:"): DatabaseSync {
  const db = new DatabaseSync(path);
  db.exec(SCHEMA);
  // limpieza de sesiones caducadas en cada arranque
  db.prepare("DELETE FROM sessions WHERE expires < datetime('now')").run();
  return db;
}

/** Busca packs por texto libre (FTS5 con fallback a LIKE si la consulta
 *  contiene operadores FTS inválidos). Devuelve filas ordenadas por relevancia. */
export function searchPacks(db: DatabaseSync, query: string, limit = 50): PackRow[] {
  const q = query.trim();
  if (!q) {
    return db
      .prepare("SELECT id, owner, name, desc, json, created FROM packs ORDER BY id DESC LIMIT ?")
      .all(limit) as PackRow[];
  }
  const safe = q.replace(/["'^]/g, " ").trim();
  try {
    return db
      .prepare(
        `SELECT p.id, p.owner, p.name, p.desc, p.json, p.created
         FROM packs_fts f JOIN packs p ON p.id = f.rowid
         WHERE packs_fts MATCH ? ORDER BY rank LIMIT ?`
      )
      .all(`"${safe}"`, limit) as PackRow[];
  } catch {
    return db
      .prepare(
        `SELECT id, owner, name, desc, json, created FROM packs
         WHERE name LIKE ? OR desc LIKE ? ORDER BY id DESC LIMIT ?`
      )
      .all(`%${safe}%`, `%${safe}%`, limit) as PackRow[];
  }
}
