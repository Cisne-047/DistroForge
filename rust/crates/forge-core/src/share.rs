//! Compartir config por URL, portado de `src/share.ts` (sin `shareUrl`, que
//! depende de `location`).
//!
//! El TS codifica con `btoa(unescape(encodeURIComponent(json)))` (base64
//! estándar de los bytes UTF-8) y sustituye `+/` por `-_` quitando el padding
//! (base64url). En Rust el JSON ya es UTF-8, así que basta base64 estándar
//! sobre los bytes + la misma sustitución. Se implementa a mano para no añadir
//! la crate `base64` al árbol de dependencias.

use crate::types::DistroConfig;

const ALPHABET: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

/// Codifica bytes a base64 estándar con padding (`=`).
fn b64_encode(data: &[u8]) -> String {
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b = [
            chunk[0],
            chunk.get(1).copied().unwrap_or(0),
            chunk.get(2).copied().unwrap_or(0),
        ];
        let n = ((b[0] as u32) << 16) | ((b[1] as u32) << 8) | b[2] as u32;
        out.push(ALPHABET[(n >> 18) as usize & 63] as char);
        out.push(ALPHABET[(n >> 12) as usize & 63] as char);
        out.push(if chunk.len() > 1 { ALPHABET[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if chunk.len() > 2 { ALPHABET[n as usize & 63] as char } else { '=' });
    }
    out
}

/// Decodifica base64 estándar (con o sin padding). `Err` si hay caracteres
/// fuera del alfabeto o una longitud imposible (como `atob`, que lanza).
fn b64_decode(s: &str) -> Option<Vec<u8>> {
    fn val(c: u8) -> Option<u32> {
        match c {
            b'A'..=b'Z' => Some((c - b'A') as u32),
            b'a'..=b'z' => Some((c - b'a' + 26) as u32),
            b'0'..=b'9' => Some((c - b'0' + 52) as u32),
            b'+' => Some(62),
            b'/' => Some(63),
            _ => None,
        }
    }
    let bytes = s.as_bytes();
    // Longitud tras padding no puede dejar resto 1 ("A===" es inválido, igual
    // que en atob).
    let data_len = bytes.iter().take_while(|&&c| c != b'=').count();
    if !bytes.len().is_multiple_of(4) || bytes.len() - data_len > 3 {
        return None;
    }
    if bytes[data_len..].iter().any(|&c| c != b'=') {
        return None;
    }
    if !bytes[..data_len].iter().all(|&c| val(c).is_some()) {
        return None;
    }
    let mut out = Vec::with_capacity(data_len * 3 / 4);
    for chunk in bytes.chunks(4) {
        let mut n: u32 = 0;
        let mut got = 0;
        for &c in chunk {
            if c == b'=' {
                break;
            }
            n = (n << 6) | val(c)?;
            got += 1;
        }
        match got {
            4 => {
                out.push((n >> 16) as u8);
                out.push((n >> 8) as u8);
                out.push(n as u8);
            }
            3 => {
                n <<= 6;
                out.push((n >> 16) as u8);
                out.push((n >> 8) as u8);
            }
            2 => {
                n <<= 12;
                out.push((n >> 16) as u8);
            }
            _ => return None,
        }
    }
    Some(out)
}

/// Codifica la config para compartirla por URL (fragmento `#`, base64url
/// unicode-safe). Port de `encodeConfig`.
pub fn encode_config(cfg: &DistroConfig) -> String {
    // La serialización de DistroConfig no puede fallar: no hay mapas con claves
    // no-string y NaN/∞ se serializan como `null` (como JSON.stringify). El
    // fallback es defensivo y no debería alcanzarse nunca.
    let json = serde_json::to_string(cfg).unwrap_or_else(|_| "null".to_string());
    b64_encode(json.as_bytes())
        .replace('+', "-")
        .replace('/', "_")
        // `.replace(/=+$/, "")`
        .trim_end_matches('=')
        .to_string()
}

/// Decodifica un fragmento de URL a config; `None` si es basura. Port de
/// `decodeConfig`.
///
/// Desviaciones mínimas respecto al TS, documentadas:
/// - El TS solo exige `typeof name === "string" && typeof base === "string"`;
///   en Rust además la `base` debe ser una de las 14 variantes del enum
///   (una base desconocida devolvería `null` igualmente al serializar mal) y
///   los campos que falten toman defaults coherentes en vez de `undefined`.
pub fn decode_config(hash: &str) -> Option<DistroConfig> {
    let b64 = hash.replace('-', "+").replace('_', "/");
    // while (b64.length % 4) b64 += "="
    let mut b64 = b64;
    while !b64.len().is_multiple_of(4) {
        b64.push('=');
    }
    let bytes = b64_decode(&b64)?;
    let json = String::from_utf8(bytes).ok()?;
    // Comprobación del TS: name y base deben ser strings.
    let parsed: serde_json::Value = serde_json::from_str(&json).ok()?;
    if !parsed.get("name").is_some_and(|v| v.is_string())
        || !parsed.get("base").is_some_and(|v| v.is_string())
    {
        return None;
    }
    serde_json::from_value(parsed).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_conoce_los_clasicos() {
        assert_eq!(b64_encode(b""), "");
        assert_eq!(b64_encode(b"f"), "Zg==");
        assert_eq!(b64_encode(b"fo"), "Zm8=");
        assert_eq!(b64_encode(b"foo"), "Zm9v");
        assert_eq!(b64_encode(b"foob"), "Zm9vYg==");
        assert_eq!(b64_encode(b"fooba"), "Zm9vYmE=");
        assert_eq!(b64_encode(b"foobar"), "Zm9vYmFy");
        assert_eq!(b64_decode("Zm9vYmFy").as_deref(), Some(b"foobar".as_slice()));
        assert!(b64_decode("!!!").is_none());
        assert!(b64_decode("A===").is_none());
    }
}
