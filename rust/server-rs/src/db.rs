//! Capa de datos: SQLite embebido (rusqlite bundled, FTS5 incluido).
//! Tablas: usuarios, sesiones de login, packs de la galería y valoraciones.
//! La búsqueda usa FTS5 (índice de texto sobre nombre+descripción+paquetes).

use rusqlite::Connection;

const SCHEMA: &str = "
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
";

/// Abre la BD, crea el esquema y limpia sesiones caducadas (como `openDb` en TS).
pub fn open_db(path: &str) -> Connection {
    let db = Connection::open(path).expect("abrir sqlite");
    // node:sqlite activa los FOREIGN KEYs por defecto; replicamos.
    db.pragma_update(None, "foreign_keys", "ON")
        .expect("pragma foreign_keys");
    // WAL y espera ante bloqueos: menos latencia de escritura y sin "database is
    // locked" en ráfagas. En :memory: el journal_mode se queda en "memory".
    let _ = db.busy_timeout(std::time::Duration::from_secs(5));
    let _ = db.pragma_update(None, "journal_mode", "WAL");
    let _ = db.pragma_update(None, "synchronous", "NORMAL");
    db.execute_batch(SCHEMA).expect("crear esquema");
    db.execute("DELETE FROM sessions WHERE expires < datetime('now')", [])
        .expect("limpiar sesiones");
    db
}

/// Fila de `packs` tal como la devuelve la búsqueda.
#[derive(Debug, Clone)]
pub struct PackRow {
    pub id: i64,
    pub owner: String,
    pub name: String,
    pub desc: String,
    pub json: String,
    pub created: String,
}

const PACK_COLS: &str = "id, owner, name, desc, json, created";

fn row_to_pack(row: &rusqlite::Row<'_>) -> rusqlite::Result<PackRow> {
    Ok(PackRow {
        id: row.get(0)?,
        owner: row.get(1)?,
        name: row.get(2)?,
        desc: row.get(3)?,
        json: row.get(4)?,
        created: row.get(5)?,
    })
}

/// Busca packs por texto libre (FTS5 con fallback a LIKE si la consulta
/// contiene operadores FTS inválidos). Devuelve filas ordenadas por relevancia.
pub fn search_packs(db: &Connection, query: &str, limit: i64) -> Vec<PackRow> {
    let q = query.trim();
    if q.is_empty() {
        return db
            .prepare(&format!(
                "SELECT {PACK_COLS} FROM packs ORDER BY id DESC LIMIT ?1"
            ))
            .expect("prepare all")
            .query_map([limit], row_to_pack)
            .expect("query all")
            .filter_map(Result::ok)
            .collect();
    }
    let safe: String = q
        .chars()
        .map(|c| {
            if c == '"' || c == '\'' || c == '^' {
                ' '
            } else {
                c
            }
        })
        .collect::<String>()
        .trim()
        .to_string();
    let fts = db
        .prepare(&format!(
            "SELECT p.{PACK_COLS}
             FROM packs_fts f JOIN packs p ON p.id = f.rowid
             WHERE packs_fts MATCH ?1 ORDER BY rank LIMIT ?2"
        ))
        .and_then(|mut stmt| {
            // "SELECT p.id, p.owner, ..." — prefijar p. a cada columna
            stmt.query_map(rusqlite::params![format!("\"{safe}\""), limit], row_to_pack)
                .map(|rows| rows.filter_map(Result::ok).collect::<Vec<_>>())
        });
    match fts {
        Ok(rows) => rows,
        Err(_) => {
            // `%` y `_` son comodines de LIKE: se escapan para que la búsqueda
            // sea literal (SQL ya va parametrizado).
            let like = safe
                .replace('\\', "\\\\")
                .replace('%', "\\%")
                .replace('_', "\\_");
            db.prepare(&format!(
                "SELECT {PACK_COLS} FROM packs
                 WHERE name LIKE ?1 ESCAPE '\\' OR desc LIKE ?1 ESCAPE '\\'
                 ORDER BY id DESC LIMIT ?2"
            ))
            .expect("prepare like")
            .query_map(rusqlite::params![format!("%{like}%"), limit], row_to_pack)
            .expect("query like")
            .filter_map(Result::ok)
            .collect()
        }
    }
}
