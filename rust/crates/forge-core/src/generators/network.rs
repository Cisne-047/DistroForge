//! Red estática: validación IPv4, máscara dotted y keyfile de NetworkManager.
//! Port de `src/generators/network.ts`.

use crate::types::{DistroConfig, NetworkConfig};

/// ¿Es una IPv4 válida según `IPV4_RE` del TS
/// (`^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\....){3}$`): 4 octetos, sin ceros a la
/// izquierda, 0-255.
pub fn is_ipv4(s: &str) -> bool {
    let parts: Vec<&str> = s.split('.').collect();
    if parts.len() != 4 {
        return false;
    }
    parts.iter().all(|p| {
        let b = p.as_bytes();
        if b.is_empty() || b.len() > 3 || !b.iter().all(|c| c.is_ascii_digit()) {
            return false;
        }
        // Sin ceros a la izquierda ("0" vale, "01" no): [1-9]?\d / 1\d\d / 2[0-4]\d / 25[0-5].
        if b.len() > 1 && b[0] == b'0' {
            return false;
        }
        p.parse::<u32>().map(|v| v <= 255).unwrap_or(false)
    })
}

/// La red estática si está completa y válida; `None` si va por DHCP o falta
/// algo (IP o gateway inválidos).
pub fn static_network(cfg: &DistroConfig) -> Option<&NetworkConfig> {
    let n = &cfg.network;
    if n.mode != crate::types::NetMode::Static || !is_ipv4(&n.ip) || !is_ipv4(&n.gateway) {
        return None;
    }
    Some(n)
}

/// Máscara dotted a partir de prefijo CIDR (anaconda --netmask la quiere así).
/// Replica `Number(prefix) || 24` (0/NaN → 24) y el clamp 8..32.
pub fn dotted_mask(prefix: &str) -> String {
    let p = prefix.trim().parse::<f64>().ok().filter(|v| *v != 0.0).unwrap_or(24.0);
    let p = p.clamp(8.0, 32.0);
    // 0xffffffff << (32 - p), con el shift truncado a entero como en JS.
    let shift = (32.0 - p).trunc() as u32;
    let bits: u32 = 0xffffffffu32.wrapping_shl(shift);
    [24, 16, 8, 0]
        .iter()
        .map(|s| ((bits >> s) & 255).to_string())
        .collect::<Vec<_>>()
        .join(".")
}

/// Keyfile de NetworkManager con la IP estática (live-build y KIWI).
pub fn nm_connection(n: &NetworkConfig) -> String {
    format!(
        "[connection]\nid=distroforge\ntype=ethernet\n\n[ipv4]\nmethod=manual\naddress1={}/{},{}\ndns={};\n",
        n.ip,
        n.prefix,
        n.gateway,
        n.dns.join(";")
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ipv4_y_mascara() {
        assert!(is_ipv4("192.168.1.50"));
        assert!(is_ipv4("0.0.0.0"));
        assert!(!is_ipv4("01.2.3.4"));
        assert!(!is_ipv4("256.1.1.1"));
        assert!(!is_ipv4("999.1.1.1"));
        assert!(!is_ipv4("1.2.3"));
        assert_eq!(dotted_mask("24"), "255.255.255.0");
        assert_eq!(dotted_mask("8"), "255.0.0.0");
        assert_eq!(dotted_mask("32"), "255.255.255.255");
        assert_eq!(dotted_mask("7"), "255.0.0.0"); // clamp a 8
        assert_eq!(dotted_mask("basura"), "255.255.255.0"); // || 24
    }
}
