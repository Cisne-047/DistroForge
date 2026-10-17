//! Utilidades de quoting shell compartidas por todos los generadores. Port de
//! `src/generators/shell.ts`.

/// Single-quote seguro para shell: `foo` → `'foo'`, `it's` → `'it'\''s'`.
/// Una sola pasada (sin allocaciones intermedias).
pub fn sq(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('\'');
    for c in s.chars() {
        if c == '\'' {
            out.push_str("'\\''");
        } else {
            out.push(c);
        }
    }
    out.push('\'');
    out
}

/// Double-quote seguro: escapa backslash, comillas, `$` y backtick.
/// Una sola pasada en vez de cuatro `replace` encadenados.
pub fn dq(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '$' => out.push_str("\\$"),
            '`' => out.push_str("\\`"),
            _ => out.push(c),
        }
    }
    out
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

/// Escaping para literales dentro de unit files de systemd y de los `sh -c` que
/// estos ejecutan: `''` para las comillas, `%%` para los especificadores y
/// `\$`/`` \` `` para que el shell interno no expanda nada.
pub fn systemd_safe(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('\'', "'\\''")
        .replace('$', "\\$")
        .replace('`', "\\`")
        .replace('%', "%%")
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
