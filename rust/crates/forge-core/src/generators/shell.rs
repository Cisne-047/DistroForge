//! Utilidades de quoting shell compartidas por todos los generadores. Port de
//! `src/generators/shell.ts`.

/// Single-quote seguro para shell: `foo` → `'foo'`, `it's` → `'it'\''s'`.
pub fn sq(s: &str) -> String {
    format!("'{}'", s.replace('\'', "'\\''"))
}

/// Double-quote seguro: escapa backslash, comillas, `$` y backtick.
pub fn dq(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('$', "\\$")
        .replace('`', "\\`")
}

/// dirname POSIX de una ruta ("/etc/x.conf" → "/etc", "/f" → "/", "f" → "f",
/// como `p.replace(/\/[^/]*$/, "")` del TS).
pub fn dirname_of(p: &str) -> String {
    let d = match p.rfind('/') {
        Some(i) => p[..i].to_string(),
        None => p.to_string(),
    };
    if d.is_empty() {
        "/".to_string()
    } else {
        d
    }
}

/// Escaping para literales dentro de unit files de systemd (`''` y `%%`).
pub fn systemd_safe(s: &str) -> String {
    s.replace('\'', "'\\''").replace('%', "%%")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn quoting() {
        assert_eq!(sq("foo"), "'foo'");
        assert_eq!(sq("it's"), "'it'\\''s'");
        assert_eq!(dq("a\"$b`c\\d"), "a\\\"\\$b\\`c\\\\d");
        assert_eq!(dirname_of("/etc/x.conf"), "/etc");
        assert_eq!(dirname_of("/f"), "/");
        assert_eq!(dirname_of("/etc/"), "/etc");
        assert_eq!(systemd_safe("a'b%c"), "a'\\''b%%c");
    }
}
