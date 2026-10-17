//! Autenticación: hash scrypt con sal por usuario (mismo formato `salt:hash`
//! hex que el servidor TS: scrypt(password, salt, 32), N=16384 r=8 p=1),
//! sesiones con token opaco en cookie httpOnly.

use rand::{rng, RngCore};
use rusqlite::Connection;
use scrypt::{scrypt, Params};

const SESSION_DAYS: i32 = 30;

/// Parámetros por defecto de `crypto.scryptSync` de Node (N=16384, r=8, p=1).
fn scrypt_params() -> Params {
    Params::new(14, 8, 1, 32).expect("parámetros scrypt válidos")
}

pub fn to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(HEX[(b >> 4) as usize] as char);
        s.push(HEX[(b & 0x0f) as usize] as char);
    }
    s
}

pub fn hash_password(password: &str) -> String {
    let mut salt = [0u8; 16];
    rng().fill_bytes(&mut salt);
    let salt_hex = to_hex(&salt);
    // Node trata la sal hex COMO TEXTO: scrypt(password, "<hex>", 32)
    format!("{salt_hex}:{}", hash_with_salt_str(password, &salt_hex))
}

fn hash_with_salt_str(password: &str, salt_str: &str) -> String {
    let mut out = [0u8; 32];
    scrypt(
        password.as_bytes(),
        salt_str.as_bytes(),
        &scrypt_params(),
        &mut out,
    )
    .expect("scrypt no puede fallar con parámetros fijos");
    to_hex(&out)
}

/// Comparación en tiempo constante (equivalente a `timingSafeEqual`).
fn ct_eq(a: &[u8], b: &[u8]) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.iter()
        .zip(b.iter())
        .fold(0u8, |acc, (x, y)| acc | (x ^ y))
        == 0
}

pub fn verify_password(password: &str, stored: &str) -> bool {
    let Some((salt, hash)) = stored.split_once(':') else {
        return false;
    };
    if salt.is_empty() || hash.is_empty() {
        return false;
    }
    let candidate = hash_with_salt_str(password, salt);
    // ambos lados son hex: comparación en tiempo constante sobre el texto
    ct_eq(candidate.as_bytes(), hash.as_bytes())
}

pub fn random_token() -> String {
    let mut bytes = [0u8; 24];
    rng().fill_bytes(&mut bytes);
    to_hex(&bytes)
}

pub fn create_user(db: &Connection, name: &str, password: &str) -> rusqlite::Result<i64> {
    create_user_hashed(db, name, &hash_password(password))
}

/// Alta con el hash ya calculado (para no retener el candado de la BD mientras
/// corre scrypt, que es lento a propósito).
pub fn create_user_hashed(db: &Connection, name: &str, pass_hash: &str) -> rusqlite::Result<i64> {
    db.execute(
        "INSERT INTO users (name, pass_hash) VALUES (?1, ?2)",
        rusqlite::params![name, pass_hash],
    )?;
    Ok(db.last_insert_rowid())
}

pub struct FoundUser {
    pub id: i64,
    pub name: String,
    pub pass_hash: String,
}

pub fn find_user(db: &Connection, name: &str) -> Option<FoundUser> {
    db.query_row(
        "SELECT id, name, pass_hash FROM users WHERE name = ?1",
        rusqlite::params![name],
        |row| {
            Ok(FoundUser {
                id: row.get(0)?,
                name: row.get(1)?,
                pass_hash: row.get(2)?,
            })
        },
    )
    .ok()
}

/// Crea una sesión y devuelve el token (guárdalo en cookie).
pub fn create_session(db: &Connection, user_id: i64) -> rusqlite::Result<String> {
    // Purga perezosa: en un servidor de larga vida las sesiones caducadas no
    // deben acumularse solo porque no se reinicia el proceso.
    let _ = db.execute("DELETE FROM sessions WHERE expires < datetime('now')", []);
    let token = random_token();
    db.execute(
        &format!(
            "INSERT INTO sessions (token, user_id, expires)
             VALUES (?1, ?2, datetime('now', '+{SESSION_DAYS} days'))"
        ),
        rusqlite::params![token, user_id],
    )?;
    Ok(token)
}

/// Un token de sesión son 24 bytes en hex (48 caracteres).
pub fn valid_token(token: &str) -> bool {
    token.len() == 48 && token.bytes().all(|b| b.is_ascii_hexdigit())
}

pub struct CurrentUser {
    pub id: i64,
    pub name: String,
}

/// Resuelve el usuario de una sesión válida; `None` si no existe o caducó.
pub fn user_for_token(db: &Connection, token: &str) -> Option<CurrentUser> {
    if token.is_empty() {
        return None;
    }
    db.query_row(
        "SELECT u.id, u.name FROM sessions s JOIN users u ON u.id = s.user_id
         WHERE s.token = ?1 AND s.expires > datetime('now')",
        rusqlite::params![token],
        |row| {
            Ok(CurrentUser {
                id: row.get(0)?,
                name: row.get(1)?,
            })
        },
    )
    .ok()
}

pub fn destroy_session(db: &Connection, token: Option<&str>) {
    if let Some(token) = token {
        let _ = db.execute(
            "DELETE FROM sessions WHERE token = ?1",
            rusqlite::params![token],
        );
    }
}
