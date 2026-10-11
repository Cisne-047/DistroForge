//! cloud-init (NoCloud). Port de `src/generators/cloudinit.ts`.

use crate::types::DistroConfig;

/// Paquete de cloud-init.
pub const CLOUD_INIT_PACKAGE: &str = "cloud-init";

/// Escapa backslash y comilla (backslash primero: si no, "c:\" acabaría como
/// "c:\\" en YAML inválido).
fn esc(s: &str) -> String {
    s.replace('\\', "\\\\").replace('"', "\\\"")
}

/// YAML `#cloud-config` con hostname, timezone y usuarios (principal con sudo
/// total + extras según su flag admin).
pub fn cloud_init_yaml(cfg: &DistroConfig) -> String {
    let sys = &cfg.system;
    let extras: String = cfg
        .users
        .iter()
        .map(|u| {
            format!(
                "  - name: {}\n    plain_text_passwd: \"{}\"\n    lock_passwd: false\n    shell: /bin/bash\n    sudo: {}\n",
                u.name,
                esc(&u.password),
                if u.admin { "ALL=(ALL) ALL" } else { "false" }
            )
        })
        .collect();
    format!(
        "#cloud-config generado por DistroForge\ndatasource_list: [NoCloud, None]\nhostname: {}\ntimezone: {}\nssh_pwauth: true\nusers:\n  - name: {}\n    plain_text_passwd: \"{}\"\n    lock_passwd: false\n    shell: /bin/bash\n    sudo: ALL=(ALL) ALL\n{}",
        sys.hostname,
        sys.timezone,
        sys.username,
        esc(&sys.password),
        extras
    )
}
