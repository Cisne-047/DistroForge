//! Batería de tests portada de `tests/generators.test.ts` (vitest).
//!
//! Equivalencias: `it.each(...)` → bucles dentro de un `#[test]`, `yamlLoad`
//! (js-yaml) → `serde_yaml`, `bash -n` QA no se porta (requiere bash en el
//! entorno de test; ver README).

use std::collections::BTreeMap;

use forge_core::catalog::{base_by_id, default_config, merge_presets, BaseDef, BASES};
use forge_core::generators::{generate_project, validate, workflow_yaml};
use forge_core::share::{decode_config, encode_config};
use forge_core::templates::{apply_template, TEMPLATES};
use forge_core::types::{BaseId, DistroConfig, EngineId, OutputFormat};

const PNG_1PX: &str =
    "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

fn outputs_for(engine: EngineId) -> Vec<OutputFormat> {
    match engine {
        EngineId::Kiwi => vec![
            OutputFormat::Iso,
            OutputFormat::Qcow2,
            OutputFormat::Usb,
            OutputFormat::Vmdk,
            OutputFormat::Vdi,
            OutputFormat::Vhd,
            OutputFormat::Oci,
            OutputFormat::Wsl,
        ],
        EngineId::Manual => vec![OutputFormat::Iso],
        _ => vec![OutputFormat::Iso, OutputFormat::Oci, OutputFormat::Wsl],
    }
}

fn cfg_for(base: BaseId) -> DistroConfig {
    let mut c = default_config(base);
    c.system.password = "test1234".to_string();
    c
}

fn keyed_repo() -> forge_core::types::CustomRepo {
    forge_core::types::CustomRepo {
        alias: "mirepo".into(),
        url: "https://example.com/r".into(),
        gpg_key: "https://example.com/key.asc".into(),
        suite: "main".into(),
        enabled: true,
    }
}

fn repo(alias: &str, url: &str, key: &str, suite: &str) -> forge_core::types::CustomRepo {
    forge_core::types::CustomRepo {
        alias: alias.into(),
        url: url.into(),
        gpg_key: key.into(),
        suite: suite.into(),
        enabled: true,
    }
}

// ---------------------------------------------------------------- generateProject

#[test]
fn cada_base_genera_proyecto_completo() {
    for b in BASES.iter() {
        let files = generate_project(&cfg_for(b.id));
        assert!(files.get("README.md").is_some_and(|s| !s.is_empty()), "{}", b.id.as_str());
        assert!(
            files.get(".github/workflows/build.yml").is_some_and(|s| !s.is_empty()),
            "{}",
            b.id.as_str()
        );
        for (path, content) in &files {
            assert!(
                !content.contains("undefined"),
                "{}/{path} contiene \"undefined\"",
                b.id.as_str()
            );
            assert!(content.chars().count() >= 3, "{}/{path} casi vacío", b.id.as_str());
        }
    }
}

#[test]
fn kiwi_exporta_iso_y_qcow2_segun_formato() {
    let mut c = cfg_for(BaseId::OpensuseTumbleweed);
    assert!(generate_project(&c)
        .get("kiwi/config.xml")
        .unwrap()
        .contains("<type image=\"iso\""));
    c.output = OutputFormat::Qcow2;
    assert!(generate_project(&c)
        .get("kiwi/config.xml")
        .unwrap()
        .contains("image=\"qcow2\""));
}

#[test]
fn kiwi_marca_gpgcheck_false_solo_en_repos_sin_key() {
    let mut c = cfg_for(BaseId::OpensuseTumbleweed);
    c.repos = vec![
        repo("sinkey", "https://example.com/repo", "", ""),
        repo("conkey", "https://example.com/repo2", "https://example.com/key.asc", ""),
    ];
    let xml = generate_project(&c).get("kiwi/config.xml").unwrap().clone();
    let line_sin = xml
        .lines()
        .find(|l| l.contains("alias=\"sinkey\""))
        .unwrap_or("")
        .to_string();
    let line_con = xml
        .lines()
        .find(|l| l.contains("alias=\"conkey\""))
        .unwrap_or("")
        .to_string();
    assert!(line_sin.contains("gpgcheck=\"false\""));
    assert!(!line_con.contains("gpgcheck=\"false\""));
}

#[test]
fn livebuild_receta_coherente() {
    for id in [BaseId::Debian, BaseId::Ubuntu, BaseId::Mint, BaseId::Pop] {
        let base: &BaseDef = base_by_id(id);
        let files = generate_project(&cfg_for(id));
        let auto = files.get("livebuild/auto/config").unwrap();
        assert!(auto.contains(&format!("--distribution {}", base.distribution.unwrap())), "{id}");
        assert!(auto.contains(&format!("--linux-flavours {}", base.lb_flavour.unwrap())), "{id}");
        assert!(auto.contains(&format!("--mirror-bootstrap {}", base.mirror)), "{id}");
        assert!(
            files
                .get("livebuild/config/package-lists/distroforge.list.chroot")
                .unwrap()
                .contains(base.kernel_package.unwrap()),
            "{id}"
        );
        assert!(
            files
                .get("livebuild/config/includes.chroot/etc/hostname")
                .unwrap()
                .contains("mi-distro"),
            "{id}"
        );
    }
}

#[test]
fn bases_con_repos_preset_generan_hook_apt_con_suite() {
    for id in [BaseId::Mint, BaseId::Pop] {
        let base = base_by_id(id);
        let files = generate_project(&cfg_for(id));
        let hook_path = files
            .keys()
            .find(|p| {
                // /hooks/<etapa>/<digitos>-repo-...   (etapa: live o normal)
                if let Some(pos) = p.find("/hooks/") {
                    let mut rest = &p[pos + 7..];
                    if let Some(r) = rest
                        .strip_prefix("live/")
                        .or_else(|| rest.strip_prefix("normal/"))
                    {
                        rest = r;
                    }
                    let digits = rest.bytes().take_while(|c| c.is_ascii_digit()).count();
                    digits > 0 && rest[digits..].starts_with("-repo-")
                } else {
                    false
                }
            })
            .expect("hook de repo")
            .clone();
        let hook = files.get(&hook_path).unwrap();
        let preset = &base.preset_repos[0];
        assert!(
            hook.contains(&format!("deb $SIGNED {} {}", preset.url, preset.suite)),
            "{id}"
        );
    }
}

#[test]
fn merge_presets_cambia_presets_y_conserva_usuario() {
    let user_repo = repo("mio", "https://example.com/mio", "", "");
    let tras_mint = merge_presets(base_by_id(BaseId::Mint), std::slice::from_ref(&user_repo));
    assert!(tras_mint.iter().any(|r| r.alias == "linuxmint"));
    let tras_cachy = merge_presets(base_by_id(BaseId::Cachyos), &tras_mint);
    assert!(tras_cachy.iter().any(|r| r.alias == "cachyos"));
    assert!(!tras_cachy.iter().any(|r| r.alias == "linuxmint"));
    assert!(tras_cachy.iter().any(|r| r.alias == "mio"));
}

#[test]
fn archiso_kernel_y_repos() {
    for id in [BaseId::Arch, BaseId::Manjaro, BaseId::Cachyos] {
        let base = base_by_id(id);
        let files = generate_project(&cfg_for(id));
        assert!(
            files
                .get("archiso/packages.x86_64")
                .unwrap()
                .contains(base.kernel_package.unwrap()),
            "{id}"
        );
        let pacman = files.get("archiso/pacman.conf").unwrap();
        assert!(pacman.contains("[core]"), "{id}");
        for preset in base.preset_repos {
            assert!(pacman.contains(&format!("[{}]", preset.alias)), "{id}");
        }
    }
}

#[test]
fn kickstart_fedora_usuario_y_repos_nogpgcheck() {
    let mut c = cfg_for(BaseId::Fedora);
    c.repos = vec![repo("mio", "https://example.com/rpm", "", "")];
    let ks = generate_project(&c).get("fedora/ks.cfg").unwrap().clone();
    assert!(ks.contains("user --name=user"));
    assert!(ks.contains("--nogpgcheck"));
}

#[test]
fn manual_lista_y_guia_sin_builder() {
    for id in [BaseId::Gentoo, BaseId::Solus] {
        let files = generate_project(&cfg_for(id));
        assert!(files.get("manual/packages.txt").is_some_and(|s| !s.is_empty()), "{id}");
        assert!(files.get("manual/README.md").is_some_and(|s| !s.is_empty()), "{id}");
        assert!(files.get("build.sh").unwrap().contains("exit 1"), "{id}");
        assert!(
            files
                .get(".github/workflows/build.yml")
                .unwrap()
                .contains("Sin builder automático"),
            "{id}"
        );
    }
}

#[test]
fn escape_xml_en_nombre_de_distro() {
    let mut c = cfg_for(BaseId::OpensuseTumbleweed);
    c.repos = vec![repo("a<b", "https://example.com/r", "", "")];
    let xml = generate_project(&c).get("kiwi/config.xml").unwrap().clone();
    assert!(xml.contains("alias=\"a&lt;b\""));
}

#[test]
fn oci_dockerfile_con_imagen_base_usuario_y_workflow() {
    let mut c = cfg_for(BaseId::Debian);
    c.output = OutputFormat::Oci;
    let files = generate_project(&c);
    let df = files.get("Dockerfile").unwrap();
    assert!(df.contains("FROM debian:bookworm"));
    assert!(df.contains("apt-get install"));
    assert!(df.contains(&format!("USER {}", c.system.username)));
    assert!(files.get("build.sh").unwrap().contains("docker || command -v podman"));
    assert!(files
        .get(".github/workflows/build.yml")
        .unwrap()
        .contains("docker build"));
    assert!(files.get("README.md").unwrap().contains("docker load"));
}

#[test]
fn oci_opensuse_usa_zypper_y_fedora_dnf() {
    let mut k = cfg_for(BaseId::OpensuseTumbleweed);
    k.output = OutputFormat::Oci;
    assert!(generate_project(&k).get("Dockerfile").unwrap().contains("zypper"));
    let mut f = cfg_for(BaseId::Fedora);
    f.output = OutputFormat::Oci;
    assert!(generate_project(&f).get("Dockerfile").unwrap().contains("dnf install"));
}

#[test]
fn oci_base_arch_anade_repos_preset_a_pacman_conf() {
    let mut c = cfg_for(BaseId::Cachyos);
    c.output = OutputFormat::Oci;
    let df = generate_project(&c).get("Dockerfile").unwrap().clone();
    assert!(df.contains("FROM archlinux:latest"));
    assert!(df.contains("\"cachyos\"")); // printf %s con el alias saneado
}

#[test]
fn cloud_init_y_motd_llegan_a_la_receta_livebuild() {
    let mut c = cfg_for(BaseId::Debian);
    c.cloud_init = true;
    c.motd = "Hola distro".to_string();
    let files = generate_project(&c);
    assert!(files
        .get("livebuild/config/includes.chroot/etc/cloud/cloud.cfg.d/80-distroforge.cfg")
        .unwrap()
        .contains("hostname: mi-distro"));
    assert!(files
        .get("livebuild/config/package-lists/distroforge.list.chroot")
        .unwrap()
        .contains("cloud-init"));
    assert!(files
        .get("livebuild/config/includes.chroot/etc/motd")
        .unwrap()
        .contains("Hola distro"));
}

#[test]
fn cloud_init_y_motd_en_kiwi_van_por_config_sh() {
    let mut c = cfg_for(BaseId::OpensuseTumbleweed);
    c.cloud_init = true;
    c.motd = "SUSE motd".to_string();
    let files = generate_project(&c);
    assert!(files.get("kiwi/config.xml").unwrap().contains("name=\"cloud-init\""));
    let sh = files.get("kiwi/config.sh").unwrap();
    assert!(sh.contains("80-distroforge.cfg"));
    assert!(sh.contains("plain_text_passwd"));
    assert!(sh.contains("SUSE motd"));
}

#[test]
fn oci_no_disponible_en_bases_manuales() {
    let mut c = cfg_for(BaseId::Gentoo);
    c.output = OutputFormat::Oci;
    assert!(validate(&c).iter().any(|i| i.contains("OCI")));
}

#[test]
fn wsl_no_disponible_en_bases_manuales() {
    let mut c = cfg_for(BaseId::Solus);
    c.output = OutputFormat::Wsl;
    assert!(validate(&c).iter().any(|i| i.contains("WSL")));
}

#[test]
fn todos_los_workflows_son_yaml_valido() {
    for b in BASES.iter() {
        for o in outputs_for(b.engine) {
            let mut c = cfg_for(b.id);
            c.output = o;
            let yml = generate_project(&c)
                .get(".github/workflows/build.yml")
                .unwrap()
                .clone();
            // equivalente a expect(() => yamlLoad(yml)).not.toThrow()
            serde_yaml::from_str::<serde_yaml::Value>(&yml)
                .unwrap_or_else(|e| panic!("{}/{o}: {e}", b.id.as_str()));
        }
    }
}

#[test]
fn wsl_rootfs_bootstrap_por_familia() {
    let mut d = cfg_for(BaseId::Debian);
    d.output = OutputFormat::Wsl;
    let fd = generate_project(&d);
    let sh = fd.get("wsl/rootfs.sh").unwrap();
    assert!(sh.contains("debootstrap --variant=minbase --arch=amd64 bookworm"));
    assert!(sh.contains("default = user"));
    assert!(sh.contains("systemd = true"));
    assert!(fd.get("build.sh").unwrap().contains("wsl/rootfs.sh"));

    let mut a = cfg_for(BaseId::Cachyos);
    a.output = OutputFormat::Wsl;
    assert!(generate_project(&a).get("wsl/rootfs.sh").unwrap().contains("pacstrap"));

    let mut u = cfg_for(BaseId::Ubuntu);
    u.output = OutputFormat::Wsl;
    assert!(generate_project(&u).get("build.sh").unwrap().contains("ubuntu:24.04"));

    let mut f = cfg_for(BaseId::Fedora);
    f.output = OutputFormat::Wsl;
    assert!(generate_project(&f)
        .get("wsl/rootfs.sh")
        .unwrap()
        .contains("--installroot=/rootfs"));
}

#[test]
fn wsl_repos_preset_llegan_al_rootfs() {
    let mut m = cfg_for(BaseId::Mint);
    m.output = OutputFormat::Wsl;
    let sh = generate_project(&m).get("wsl/rootfs.sh").unwrap().clone();
    assert!(sh.contains("packages.linuxmint.com"));
    assert!(sh.contains("wilma main upstream import backport"));
}

#[test]
fn wallpaper_dconf_gnome_xfconf_xfce_nada_kde() {
    let mut g = cfg_for(BaseId::Debian);
    g.desktop = "gnome".to_string();
    g.wallpaper = PNG_1PX.to_string();
    let gf = generate_project(&g);
    assert_eq!(
        gf.get("livebuild/config/includes.chroot/usr/share/backgrounds/distroforge-wallpaper.png")
            .unwrap(),
        PNG_1PX
    );
    assert!(gf
        .get("livebuild/config/includes.chroot/etc/dconf/db/distroforge.d/00-wallpaper")
        .unwrap()
        .contains("org/gnome/desktop/background"));
    assert!(gf.keys().any(|k| k.contains("0300-dconf")));
    assert!(gf
        .get("livebuild/config/package-lists/distroforge.list.chroot")
        .unwrap()
        .contains("dconf-cli"));

    let mut x = cfg_for(BaseId::Debian);
    x.desktop = "xfce".to_string();
    x.wallpaper = PNG_1PX.to_string();
    assert!(generate_project(&x)
        .get("livebuild/config/includes.chroot/etc/xdg/xfce4/xfconf/xfce-perchannel-xml/xfce4-desktop.xml")
        .unwrap()
        .contains("xfce4-desktop"));

    let mut k = cfg_for(BaseId::Debian);
    k.desktop = "kde".to_string();
    k.wallpaper = PNG_1PX.to_string();
    let kf = generate_project(&k);
    assert!(!kf.keys().any(|p| p.contains("dconf")));
}

#[test]
fn wallpaper_en_arch_usa_servicio_dconf_al_arrancar() {
    let mut c = cfg_for(BaseId::Arch);
    c.desktop = "gnome".to_string();
    c.wallpaper = PNG_1PX.to_string();
    let files = generate_project(&c);
    assert!(files
        .get("archiso/airootfs/etc/systemd/system/distroforge-dconf.service")
        .unwrap()
        .contains("dconf update"));
    assert!(files
        .get("archiso/airootfs/etc/tmpfiles.d/distroforge-dconf.conf")
        .unwrap()
        .contains("multi-user.target.wants"));
}

#[test]
fn wallpaper_en_kiwi_va_por_root_overlay_y_config_sh() {
    let mut c = cfg_for(BaseId::OpensuseTumbleweed);
    c.desktop = "gnome".to_string();
    c.wallpaper = PNG_1PX.to_string();
    let files = generate_project(&c);
    assert_eq!(
        files
            .get("kiwi/root/usr/share/backgrounds/distroforge-wallpaper.png")
            .unwrap(),
        PNG_1PX
    );
    assert!(files
        .get("kiwi/root/etc/dconf/db/distroforge.d/00-wallpaper")
        .is_some_and(|s| !s.is_empty()));
    assert!(files.get("kiwi/config.sh").unwrap().contains("dconf update"));
}

#[test]
fn validacion_de_wallpaper_formato_tamano_y_fedora() {
    let mut c = cfg_for(BaseId::Debian);
    c.wallpaper = "data:image/gif;base64,R0lGOD".to_string();
    assert!(validate(&c).iter().any(|i| i.contains("formato")));

    let mut big = cfg_for(BaseId::Debian);
    big.wallpaper = format!("data:image/png;base64,{}", "A".repeat(3_100_000));
    assert!(validate(&big).iter().any(|i| i.contains("grande")));

    let mut f = cfg_for(BaseId::Fedora);
    f.wallpaper = PNG_1PX.to_string();
    assert!(validate(&f).iter().any(|i| i.contains("Fedora")));
}

// ---------------------------------------------------------------- validate

#[test]
fn detecta_nombre_contrasena_y_qcow2_invalidos() {
    let mut c = cfg_for(BaseId::Arch);
    c.name = "Nombre Invalido".to_string();
    c.system.password = String::new();
    c.output = OutputFormat::Qcow2;
    let issues = validate(&c);
    assert!(issues.iter().any(|i| i.contains("Nombre")));
    assert!(issues.iter().any(|i| i.contains("contraseña")));
    assert!(issues.iter().any(|i| i.contains("QCOW2")));
}

#[test]
fn rechaza_repos_sin_http() {
    let mut c = cfg_for(BaseId::Arch);
    c.repos = vec![repo("ftp-repo", "ftp://example.com/r", "", "")];
    assert!(validate(&c).iter().any(|i| i.contains("http")));
}

#[test]
fn config_por_defecto_de_cada_base_pasa_validacion() {
    for b in BASES.iter() {
        let mut c = cfg_for(b.id);
        c.system.password = "x12345678".to_string();
        assert!(
            validate(&c).is_empty(),
            "base {}: {:?}",
            b.id.as_str(),
            validate(&c)
        );
    }
}

#[test]
fn validacion_rutas_ficheros_extra_e_ids_flatpak() {
    let mut c = cfg_for(BaseId::Debian);
    c.extra_files = vec![forge_core::types::ExtraFile {
        path: "relative/file".into(),
        content: "x".into(),
    }];
    assert!(validate(&c).iter().any(|i| i.contains("absoluta")));
    c.extra_files = vec![forge_core::types::ExtraFile {
        path: "/etc/../passwd".into(),
        content: "x".into(),
    }];
    assert!(validate(&c).iter().any(|i| i.contains("..")));
    c.extra_files = vec![];
    c.flatpak_apps = vec!["mal id!".into()];
    assert!(validate(&c).iter().any(|i| i.contains("id de app")));
}

// ---------------------------------------------------------------- bloque 4

fn app_cfg(base: BaseId) -> DistroConfig {
    let mut c = cfg_for(base);
    c.flatpak_apps = vec!["org.mozilla.firefox".into(), "org.videolan.VLC".into()];
    c.extra_files = vec![
        forge_core::types::ExtraFile {
            path: "/etc/skel/.bashrc".into(),
            content: "export EDITOR=vim\n".into(),
        },
        forge_core::types::ExtraFile {
            path: "/etc/issue".into(),
            content: "mi distro".into(),
        },
    ];
    c
}

#[test]
fn livebuild_paquete_flatpak_hook_flathub_y_ficheros() {
    let files = generate_project(&app_cfg(BaseId::Debian));
    assert!(files
        .get("livebuild/config/package-lists/distroforge.list.chroot")
        .unwrap()
        .contains("flatpak"));
    let hook = files.get("livebuild/config/hooks/live/0350-flatpak.chroot").unwrap();
    assert!(hook.contains("remote-add --if-not-exists flathub"));
    assert!(hook.contains("org.mozilla.firefox org.videolan.VLC"));
    assert!(files
        .get("livebuild/config/includes.chroot/etc/skel/.bashrc")
        .unwrap()
        .contains("EDITOR=vim"));
    assert_eq!(files.get("livebuild/config/includes.chroot/etc/issue").unwrap(), "mi distro");
}

#[test]
fn archiso_flatpak_en_packages_servicio_y_airootfs() {
    let files = generate_project(&app_cfg(BaseId::Arch));
    assert!(files.get("archiso/packages.x86_64").unwrap().contains("flatpak"));
    let svc = files
        .get("archiso/airootfs/etc/systemd/system/distroforge-flatpak.service")
        .unwrap();
    assert!(svc.contains("remote-add --if-not-exists flathub"));
    assert!(svc.contains("org.mozilla.firefox org.videolan.VLC"));
    assert!(files
        .get("archiso/airootfs/etc/tmpfiles.d/distroforge-flatpak.conf")
        .unwrap()
        .contains("multi-user.target.wants"));
    assert!(files
        .get("archiso/airootfs/etc/skel/.bashrc")
        .unwrap()
        .contains("EDITOR=vim"));
}

#[test]
fn kiwi_flatpak_en_xml_config_sh_y_root_overlay() {
    let files = generate_project(&app_cfg(BaseId::OpensuseTumbleweed));
    assert!(files.get("kiwi/config.xml").unwrap().contains("name=\"flatpak\""));
    assert!(files.get("kiwi/config.sh").unwrap().contains("flatpak install"));
    assert!(files.get("kiwi/root/etc/skel/.bashrc").unwrap().contains("EDITOR=vim"));
}

#[test]
fn kickstart_flatpak_en_packages_y_heredoc_en_post() {
    let ks = generate_project(&app_cfg(BaseId::Fedora)).get("fedora/ks.cfg").unwrap().clone();
    assert!(ks.contains("flatpak"));
    assert!(ks.contains("mkdir -p /etc/skel"));
    assert!(ks.contains("cat > /etc/skel/.bashrc <<'DFEOF0'"));
    assert!(ks.contains("export EDITOR=vim"));
}

#[test]
fn oci_run_flathub_y_copy_ficheros() {
    let mut c = app_cfg(BaseId::Debian);
    c.output = OutputFormat::Oci;
    let files = generate_project(&c);
    let df = files.get("Dockerfile").unwrap();
    assert!(df.contains("flatpak remote-add --if-not-exists flathub"));
    assert!(df.contains("RUN mkdir -p /etc/skel /etc"));
    assert!(df.contains("COPY files/extra-0 /etc/skel/.bashrc"));
    assert!(files.get("files/extra-0").unwrap().contains("EDITOR=vim"));
}

#[test]
fn wsl_ficheros_heredoc_y_flatpak_chroot() {
    let mut c = app_cfg(BaseId::Ubuntu);
    c.output = OutputFormat::Wsl;
    let sh = generate_project(&c).get("wsl/rootfs.sh").unwrap().clone();
    assert!(sh.contains("cat > /rootfs/etc/skel/.bashrc <<'DFEOF'"));
    assert!(sh.contains("chroot /rootfs flatpak remote-add --if-not-exists flathub"));
    assert!(sh.contains("chroot /rootfs flatpak install -y --noninteractive flathub"));
    assert!(sh.contains("sudo flatpak"));
}

#[test]
fn manual_readme_lista_ficheros_y_howto_flatpak() {
    let readme = generate_project(&app_cfg(BaseId::Gentoo))
        .get("manual/README.md")
        .unwrap()
        .clone();
    assert!(readme.contains("/etc/skel/.bashrc"));
    assert!(readme.contains("flatpak install"));
}

#[test]
fn sin_flatpak_ni_ficheros_ningun_rastro() {
    let files: BTreeMap<String, String> = generate_project(&cfg_for(BaseId::Debian));
    assert!(!files.keys().any(|p| p.contains("flatpak") || p.contains("extra-")));
    assert!(!files
        .get("livebuild/config/package-lists/distroforge.list.chroot")
        .unwrap()
        .contains("flatpak"));
}

// ---------------------------------------------------------------- bloque 5

#[test]
fn kiwi_importa_key_cuando_hay_y_nada_cuando_no() {
    let mut c = cfg_for(BaseId::OpensuseTumbleweed);
    c.repos = vec![keyed_repo()];
    assert!(generate_project(&c)
        .get("kiwi/config.sh")
        .unwrap()
        .contains("rpm --import \"https://example.com/key.asc\""));
    let mut sin = cfg_for(BaseId::OpensuseTumbleweed);
    sin.repos = vec![repo("mirepo", "https://example.com/r", "", "main")];
    let sh = generate_project(&sin).get("kiwi/config.sh").unwrap().clone();
    assert!(!sh.contains("rpm --import"));
}

#[test]
fn kickstart_nogpgcheck_importa_key_y_avisa() {
    let mut c = cfg_for(BaseId::Fedora);
    c.repos = vec![keyed_repo()];
    let ks = generate_project(&c).get("fedora/ks.cfg").unwrap().clone();
    assert!(ks.contains("--nogpgcheck"));
    assert!(ks.contains("rpm --import \"https://example.com/key.asc\""));
    assert!(validate(&c).iter().any(|i| i.contains("GPG")));
}

#[test]
fn oci_debian_descarga_key_y_firma_repo() {
    let mut c = cfg_for(BaseId::Debian);
    c.output = OutputFormat::Oci;
    c.repos = vec![keyed_repo()];
    let df = generate_project(&c).get("Dockerfile").unwrap().clone();
    assert!(df.contains("wget -qO /usr/share/keyrings/mirepo.asc https://example.com/key.asc"));
    assert!(df.contains(
        "deb [signed-by=/usr/share/keyrings/mirepo.asc] https://example.com/r main"
    ));
}

#[test]
fn oci_debian_sin_key_sigue_trusted() {
    let mut c = cfg_for(BaseId::Debian);
    c.output = OutputFormat::Oci;
    c.repos = vec![repo("mirepo", "https://example.com/r", "", "main")];
    let df = generate_project(&c).get("Dockerfile").unwrap().clone();
    assert!(df.contains("deb [trusted=yes] https://example.com/r main"));
}

#[test]
fn wsl_debian_key_descargada_y_repo_firmado() {
    let mut c = cfg_for(BaseId::Debian);
    c.output = OutputFormat::Wsl;
    c.repos = vec![keyed_repo()];
    let sh = generate_project(&c).get("wsl/rootfs.sh").unwrap().clone();
    assert!(sh.contains("wget -qO /rootfs/usr/share/keyrings/mirepo.asc https://example.com/key.asc"));
    assert!(sh.contains("deb [signed-by=/usr/share/keyrings/mirepo.asc] https://example.com/r main"));
    assert!(sh.contains("apt-get install -y debootstrap wget ca-certificates"));
}

#[test]
fn validate_avisa_trustall_en_arch_con_key() {
    let mut c = cfg_for(BaseId::Arch);
    c.repos = vec![keyed_repo()];
    assert!(validate(&c).iter().any(|i| i.contains("TrustAll")));
}

// ---------------------------------------------------------------- bloque 6

fn studio_cfg(base: BaseId) -> DistroConfig {
    let mut c = cfg_for(base);
    c.users = vec![
        forge_core::types::ExtraUser {
            name: "ana".into(),
            password: "ana12345".into(),
            admin: true,
        },
        forge_core::types::ExtraUser {
            name: "pepe".into(),
            password: "pepe12345".into(),
            admin: false,
        },
    ];
    c.services = vec![
        forge_core::types::ServiceToggle { unit: "sshd.service".into(), enabled: true },
        forge_core::types::ServiceToggle { unit: "bluetooth.service".into(), enabled: false },
    ];
    c.autologin = true;
    c
}

#[test]
fn livebuild_usuarios_extra_hook_servicios_y_autologin_dm() {
    let files = generate_project(&studio_cfg(BaseId::Debian));
    let hook = files
        .get("livebuild/config/hooks/live/0100-distroforge-user.chroot")
        .unwrap();
    assert!(hook.contains("useradd -m -s /bin/bash -G sudo \"ana\""));
    assert!(hook.contains("useradd -m -s /bin/bash \"pepe\""));
    let svc = files.get("livebuild/config/hooks/live/0340-services.chroot").unwrap();
    assert!(svc.contains("systemctl enable \"sshd.service\""));
    assert!(svc.contains("systemctl disable \"bluetooth.service\""));
    // escritorio "none" no tiene DM: sin fichero de autologin
    assert!(!files.keys().any(|p| p.contains("autologin")));
    let mut xf = studio_cfg(BaseId::Debian);
    xf.desktop = "xfce".to_string();
    assert!(generate_project(&xf)
        .get("livebuild/config/includes.chroot/etc/lightdm/lightdm.conf")
        .unwrap()
        .contains("autologin-user=user"));
}

#[test]
fn archiso_unidades_oneshot_usuarios_servicios_tmpfiles() {
    let files = generate_project(&studio_cfg(BaseId::Arch));
    assert!(files
        .get("archiso/airootfs/etc/systemd/system/distroforge-users.service")
        .unwrap()
        .contains("useradd -m -s /bin/bash -G wheel \"ana\""));
    assert!(files
        .get("archiso/airootfs/etc/systemd/system/distroforge-services.service")
        .unwrap()
        .contains("systemctl enable --now \"sshd.service\""));
    assert!(files
        .get("archiso/airootfs/etc/tmpfiles.d/distroforge-users.conf")
        .unwrap()
        .contains("multi-user.target.wants"));
}

#[test]
fn kiwi_usuarios_en_xml_servicios_y_formatos() {
    let mut c = studio_cfg(BaseId::OpensuseTumbleweed);
    let xml = generate_project(&c).get("kiwi/config.xml").unwrap().clone();
    assert!(xml.contains("name=\"ana\""));
    assert!(xml.contains("groups=\"users,wheel\""));
    assert!(generate_project(&c)
        .get("kiwi/config.sh")
        .unwrap()
        .contains("systemctl enable \"sshd.service\""));
    c.output = OutputFormat::Usb;
    assert!(generate_project(&c).get("kiwi/config.xml").unwrap().contains("image=\"oem\""));
    c.output = OutputFormat::Vmdk;
    let xml = generate_project(&c).get("kiwi/config.xml").unwrap().clone();
    assert!(xml.contains("image=\"vmx\""));
    assert!(xml.contains("format=\"vmdk\""));
    c.output = OutputFormat::Vdi;
    assert!(generate_project(&c).get("kiwi/config.xml").unwrap().contains("format=\"vdi\""));
}

#[test]
fn kickstart_usuarios_nativos_servicios_y_autologin_gdm() {
    let mut c = studio_cfg(BaseId::Fedora);
    c.desktop = "gnome".to_string();
    let ks = generate_project(&c).get("fedora/ks.cfg").unwrap().clone();
    assert!(ks.contains("user --name=ana --password=\"ana12345\" --plaintext --groups=wheel"));
    assert!(ks.contains("user --name=pepe --password=\"pepe12345\" --plaintext"));
    assert!(ks.contains("systemctl enable \"sshd.service\""));
    assert!(ks.contains("cat > /etc/gdm/custom.conf <<'DFEOF0'"));
}

#[test]
fn oci_y_wsl_usuarios_extra_y_servicios_con_chroot() {
    let mut c = studio_cfg(BaseId::Debian);
    c.output = OutputFormat::Oci;
    let df = generate_project(&c).get("Dockerfile").unwrap().clone();
    assert!(df.contains("useradd -m -s /bin/bash -G sudo \"ana\""));
    assert!(df.contains("ana:ana12345"));
    c.output = OutputFormat::Wsl;
    let sh = generate_project(&c).get("wsl/rootfs.sh").unwrap().clone();
    assert!(sh.contains("chroot /rootfs useradd -m -s /bin/bash -G wheel \"ana\""));
    assert!(sh.contains("chroot /rootfs systemctl enable \"sshd.service\""));
}

#[test]
fn cloud_init_usuarios_extra_con_sudo_segun_admin() {
    let mut c = studio_cfg(BaseId::Debian);
    c.cloud_init = true;
    let yml = generate_project(&c)
        .get("livebuild/config/includes.chroot/etc/cloud/cloud.cfg.d/80-distroforge.cfg")
        .unwrap()
        .clone();
    assert!(yml.contains("- name: ana"));
    assert!(yml.contains("sudo: ALL=(ALL) ALL"));
    assert!(yml.contains("- name: pepe"));
    assert!(yml.contains("sudo: false"));
}

#[test]
fn validate_usuarios_repetidos_invalidos_sin_contrasena_y_unidad() {
    let mut c = cfg_for(BaseId::Debian);
    c.users = vec![
        forge_core::types::ExtraUser { name: "user".into(), password: "x1234567".into(), admin: false },
        forge_core::types::ExtraUser { name: "Ana".into(), password: "x1234567".into(), admin: false },
        forge_core::types::ExtraUser { name: "ok2".into(), password: String::new(), admin: false },
    ];
    let issues = validate(&c);
    assert!(issues.iter().any(|i| i.contains("repetido")));
    assert!(issues.iter().any(|i| i.contains("inválido")));
    assert!(issues.iter().any(|i| i.contains("contraseña")));
    let mut s = cfg_for(BaseId::Debian);
    s.services = vec![forge_core::types::ServiceToggle {
        unit: "mal servicio!".into(),
        enabled: true,
    }];
    assert!(validate(&s).iter().any(|i| i.contains("unidad")));
}

#[test]
fn formatos_usb_vmdk_vdi_solo_kiwi_y_yaml_valido() {
    for o in [OutputFormat::Usb, OutputFormat::Vmdk, OutputFormat::Vdi] {
        let mut c = cfg_for(BaseId::Debian);
        c.output = o;
        assert!(validate(&c).iter().any(|i| i.contains("KIWI")));
    }
    for b in BASES.iter() {
        for o in outputs_for(b.engine) {
            let mut c = cfg_for(b.id);
            c.output = o;
            serde_yaml::from_str::<serde_yaml::Value>(
                generate_project(&c).get(".github/workflows/build.yml").unwrap(),
            )
            .unwrap_or_else(|e| panic!("{}/{o}: {e}", b.id.as_str()));
        }
    }
}

// ---------------------------------------------------------------- bloque 7

fn net_cfg(base: BaseId) -> DistroConfig {
    let mut c = cfg_for(base);
    c.network = forge_core::types::NetworkConfig {
        mode: forge_core::types::NetMode::Static,
        ip: "192.168.1.50".into(),
        prefix: "24".into(),
        gateway: "192.168.1.1".into(),
        dns: vec!["1.1.1.1".into(), "8.8.8.8".into()],
    };
    c
}

#[test]
fn kickstart_network_estatica_y_dhcp() {
    let ks = generate_project(&net_cfg(BaseId::Fedora))
        .get("fedora/ks.cfg")
        .unwrap()
        .clone();
    assert!(ks.contains(
        "network --bootproto=static --ip=192.168.1.50 --netmask=255.255.255.0 --gateway=192.168.1.1 --nameserver=1.1.1.1,8.8.8.8"
    ));
    assert!(generate_project(&cfg_for(BaseId::Fedora))
        .get("fedora/ks.cfg")
        .unwrap()
        .contains("--bootproto=dhcp"));
}

#[test]
fn livebuild_keyfile_nm_con_chmod() {
    let files = generate_project(&net_cfg(BaseId::Debian));
    let keyfile = files
        .get("livebuild/config/includes.chroot/etc/NetworkManager/system-connections/distroforge.nmconnection")
        .unwrap();
    assert!(keyfile.contains("address1=192.168.1.50/24,192.168.1.1"));
    assert!(keyfile.contains("dns=1.1.1.1;8.8.8.8;"));
    assert!(files.keys().any(|p| p.contains("0330-distroforge-net")));
    let dhcp: BTreeMap<String, String> = generate_project(&cfg_for(BaseId::Debian));
    assert!(!dhcp.keys().any(|p| p.contains("nmconnection")));
}

#[test]
fn kiwi_keyfile_y_chmod_en_config_sh() {
    let files = generate_project(&net_cfg(BaseId::OpensuseTumbleweed));
    assert!(files
        .get("kiwi/root/etc/NetworkManager/system-connections/distroforge.nmconnection")
        .unwrap()
        .contains("address1=192.168.1.50/24,192.168.1.1"));
    assert!(files.get("kiwi/config.sh").unwrap().contains(
        "chmod 600 /etc/NetworkManager/system-connections/distroforge.nmconnection"
    ));
}

#[test]
fn validate_ip_prefijo_gateway_dns_y_aviso_arch() {
    let mut c = net_cfg(BaseId::Debian);
    c.network.ip = "999.1.1.1".into();
    assert!(validate(&c).iter().any(|i| i.contains("IP estática")));
    c.network.ip = "192.168.1.50".into();
    c.network.prefix = "7".into();
    assert!(validate(&c).iter().any(|i| i.contains("prefijo")));
    c.network.prefix = "24".into();
    c.network.dns = vec!["no-es-ip".into()];
    assert!(validate(&c).iter().any(|i| i.contains("DNS")));
    c.network.dns = vec!["1.1.1.1".into()];
    assert!(validate(&c).is_empty());
    let a = net_cfg(BaseId::Arch);
    assert!(validate(&a).iter().any(|i| i.contains("ISO live")));
}

// ---------------------------------------------------------------- bloques 8-11

#[test]
fn firewall_en_paquetes_servicio_y_kickstart() {
    let on = cfg_for(BaseId::Debian);
    let f = generate_project(&on);
    assert!(f
        .get("livebuild/config/package-lists/distroforge.list.chroot")
        .unwrap()
        .contains("firewalld"));
    assert!(f
        .get("livebuild/config/hooks/live/0340-services.chroot")
        .unwrap()
        .contains("systemctl enable \"firewalld\""));
    assert!(generate_project(&cfg_for(BaseId::Fedora))
        .get("fedora/ks.cfg")
        .unwrap()
        .contains("firewall --enabled"));
    let mut off = cfg_for(BaseId::Fedora);
    off.firewall = false;
    assert!(generate_project(&off)
        .get("fedora/ks.cfg")
        .unwrap()
        .contains("firewall --disabled"));
    assert!(generate_project(&cfg_for(BaseId::OpensuseTumbleweed))
        .get("kiwi/config.xml")
        .unwrap()
        .contains("name=\"firewalld\""));
}

#[test]
fn round_trip_distroforge_json_reimportable() {
    let mut c = cfg_for(BaseId::Debian);
    c.users = vec![forge_core::types::ExtraUser {
        name: "ana".into(),
        password: "ana12345".into(),
        admin: true,
    }];
    let json = generate_project(&c).get("distroforge.json").unwrap().clone();
    let parsed: DistroConfig = serde_json::from_str(&json).unwrap();
    assert_eq!(parsed.name, "mi-distro");
    assert_eq!(parsed.users[0].name, "ana");
    assert!(validate(&parsed).is_empty());
}

#[test]
fn share_encode_decode_round_trip_unicode() {
    let mut c = cfg_for(BaseId::Debian);
    c.system.password = "contraseña-ñ-123".to_string();
    let decoded = decode_config(&encode_config(&c)).unwrap();
    assert_eq!(decoded.system.password, c.system.password);
    assert!(decoded.users.is_empty());
    assert!(decode_config("!!!basura!!!").is_none());
}

#[test]
fn plantillas_respetan_escritorios_de_la_base() {
    let kiosk = TEMPLATES.iter().find(|t| t.id == "kiosk").unwrap();
    let mint = apply_template(&cfg_for(BaseId::Mint), kiosk);
    assert!(mint.autologin);
    assert_eq!(mint.desktop, "none"); // mint no tiene "gnome": se conserva el escritorio actual
    assert_eq!(apply_template(&cfg_for(BaseId::Arch), kiosk).desktop, "gnome");
    let wsl = apply_template(
        &cfg_for(BaseId::Debian),
        TEMPLATES.iter().find(|t| t.id == "wsldev").unwrap(),
    );
    assert_eq!(wsl.output, OutputFormat::Wsl);
}

#[test]
fn oci_workflow_con_sbom_y_trivy() {
    let mut c = cfg_for(BaseId::Debian);
    c.output = OutputFormat::Oci;
    let yml = generate_project(&c)
        .get(".github/workflows/build.yml")
        .unwrap()
        .clone();
    assert!(yml.contains("anchore/sbom-action"));
    assert!(yml.contains("aquasecurity/trivy-action"));
}

#[test]
fn arquitecturas_mapean_ci_y_validate_limita() {
    let mut a = cfg_for(BaseId::Debian);
    a.arch = forge_core::types::ArchId::Aarch64;
    assert!(generate_project(&a)
        .get("livebuild/auto/config")
        .unwrap()
        .contains("--architectures arm64"));
    let mut f = cfg_for(BaseId::Fedora);
    f.arch = forge_core::types::ArchId::Aarch64;
    assert!(generate_project(&f)
        .get("fedora/ks.cfg")
        .unwrap()
        .contains("arch=aarch64"));
    let mut armhf = cfg_for(BaseId::Debian);
    armhf.arch = forge_core::types::ArchId::Armhf;
    assert!(generate_project(&armhf)
        .get(".github/workflows/build.yml")
        .unwrap()
        .contains("setup-qemu-action"));
    let mut k = cfg_for(BaseId::OpensuseTumbleweed);
    k.arch = forge_core::types::ArchId::Aarch64;
    assert!(generate_project(&k)
        .get(".github/workflows/build.yml")
        .unwrap()
        .contains("ubuntu-24.04-arm"));
    let mut w = cfg_for(BaseId::Ubuntu);
    w.output = OutputFormat::Wsl;
    w.arch = forge_core::types::ArchId::Aarch64;
    assert!(generate_project(&w).get("wsl/rootfs.sh").unwrap().contains("--arch=arm64"));
    let mut ar = cfg_for(BaseId::Arch);
    ar.arch = forge_core::types::ArchId::Aarch64;
    assert!(validate(&ar).iter().any(|i| i.contains("no soportada")));
}

#[test]
fn grub_timeout_y_parametros_en_los_cuatro_motores() {
    let g = |mut c: DistroConfig| {
        c.grub = forge_core::types::GrubConfig {
            timeout: 7.0,
            extra_params: "quiet splash vt.global_cursor_default=0".into(),
        };
        c
    };
    let lb = generate_project(&g(cfg_for(BaseId::Debian)));
    let auto = lb.get("livebuild/auto/config").unwrap();
    // `lb config` no tiene opción de timeout para GRUB: pasarla rompía el build
    // entero ("unrecognized option '--grub-timeout'"). El timeout se aplica con
    // un hook de la fase binary sobre el menú ya generado.
    assert!(!auto.contains("--grub-timeout"));
    assert!(auto.contains("vt.global_cursor_default=0"));
    let hook = lb
        .get("livebuild/config/hooks/live/0900-timeout.binary")
        .expect("falta el hook que aplica el timeout de arranque");
    // GRUB usa segundos; isolinux, décimas (0 = esperar siempre, que es lo que
    // hacía que el menú se quedara colgado).
    assert!(hook.contains("set timeout=7"));
    assert!(hook.contains("timeout 70"));
    // la ruta importa: live-build solo ejecuta config/hooks/{normal,live}/*.<fase>
    assert!(!lb.contains_key("livebuild/config/hooks/live/0900-grub-timeout.hook.binary"));
    let ks = generate_project(&g(cfg_for(BaseId::Fedora)))
        .get("fedora/ks.cfg")
        .unwrap()
        .clone();
    assert!(ks.contains("--timeout=7"));
    assert!(ks.contains("--append=\"quiet splash vt.global_cursor_default=0\""));
    let ki = generate_project(&g(cfg_for(BaseId::OpensuseTumbleweed)))
        .get("kiwi/config.xml")
        .unwrap()
        .clone();
    assert!(ki.contains("kernelcmdline=\"quiet splash vt.global_cursor_default=0\""));
    assert!(ki.contains("<bootloader name=\"grub2\" timeout=\"7\"/>"));
    let ar = generate_project(&g(cfg_for(BaseId::Arch)));
    assert!(ar.get("build.sh").unwrap().contains("set timeout=7"));
    assert!(ar.get("build.sh").unwrap().contains("vt.global_cursor_default=0"));
}

#[test]
fn advanced_remaster_con_chroot_workflow_y_validacion_url() {
    let mut c = cfg_for(BaseId::Debian);
    c.advanced = true;
    c.custom_iso = forge_core::types::CustomIsoConfig {
        url: "https://example.com/exotic.iso".into(),
        chroot_script: "apt install -y htop".into(),
    };
    let files = generate_project(&c);
    assert!(files.get("advanced/download.sh").unwrap().contains("wget -c"));
    let chroot = files.get("advanced/chroot.sh").unwrap();
    assert!(chroot.contains("useradd -m -s /bin/bash -G sudo \"user\""));
    assert!(chroot.contains("apt install -y htop"));
    assert!(files
        .get(".github/workflows/build.yml")
        .unwrap()
        .contains("remaster.sh"));
    assert!(validate(&c).is_empty());
    c.custom_iso.url = "ftp://mala-url".into();
    assert!(validate(&c).iter().any(|i| i.contains("Advanced")));
}

#[test]
fn config_por_defecto_sigue_pasando_validacion() {
    for b in BASES.iter() {
        let mut c = default_config(b.id);
        c.system.password = "x12345678".to_string();
        assert!(validate(&c).is_empty(), "base {}", b.id.as_str());
    }
}

// ---------------------------------------------------------------- bloque 12b

#[test]
fn livebuild_autostart_firewall_cmd_y_set_default() {
    let mut c = cfg_for(BaseId::Debian);
    c.autostart = vec!["firefox".into(), "konsole".into()];
    c.firewall_ports = vec!["ssh".into(), "8080/tcp".into()];
    c.boot_target = forge_core::types::BootTarget::Text;
    let f = generate_project(&c);
    assert!(f
        .keys()
        .any(|p| p.contains("/.config/autostart/") && p.contains("firefox")));
    let hook = f.get("livebuild/config/hooks/live/0340-services.chroot").unwrap();
    assert!(hook.contains("firewall-cmd --permanent --add-service=ssh"));
    assert!(hook.contains("firewall-cmd --permanent --add-port=8080/tcp"));
    assert!(hook.contains("systemctl set-default multi-user.target"));
}

#[test]
fn kickstart_puertos_y_archiso_target_grafico() {
    let mut k = cfg_for(BaseId::Fedora);
    k.firewall_ports = vec!["ssh".into(), "8080/tcp".into()];
    assert!(generate_project(&k)
        .get("fedora/ks.cfg")
        .unwrap()
        .contains("firewall --enabled --service=ssh --port=8080/tcp"));
    let mut a = cfg_for(BaseId::Arch);
    a.boot_target = forge_core::types::BootTarget::Graphical;
    assert!(generate_project(&a)
        .get("archiso/airootfs/etc/systemd/system/distroforge-services.service")
        .unwrap()
        .contains("set-default graphical.target"));
}

// ---------------------------------------------------------------- extras Rust

#[test]
fn base_by_id_cubre_todas_las_variantes() {
    // Garantía que sustituye al panic de baseById del TS: el match de
    // base_by_id mapea cada variante a su propia definición.
    let all = [
        BaseId::OpensuseTumbleweed,
        BaseId::OpensuseLeap,
        BaseId::Debian,
        BaseId::Ubuntu,
        BaseId::Mint,
        BaseId::Elementary,
        BaseId::Pop,
        BaseId::Zorin,
        BaseId::Fedora,
        BaseId::Arch,
        BaseId::Manjaro,
        BaseId::Cachyos,
        BaseId::Gentoo,
        BaseId::Solus,
    ];
    for id in all {
        assert_eq!(base_by_id(id).id, id);
    }
    assert_eq!(BASES.len(), 14);
}

#[test]
fn workflow_yaml_directo_es_yaml_valido() {
    let yml = workflow_yaml(&cfg_for(BaseId::Debian));
    serde_yaml::from_str::<serde_yaml::Value>(&yml).unwrap();
}

// ---------------------------------------------------------------- bloque 16

#[test]
fn logo_binario_greeter_lightdm_y_aviso_de_formato() {
    let mut c = cfg_for(BaseId::Debian);
    c.logo = PNG_1PX.to_string();
    let f = generate_project(&c);
    assert_eq!(
        f.get("livebuild/config/includes.chroot/usr/share/distroforge/logo.png")
            .unwrap(),
        PNG_1PX
    );
    assert!(f
        .get("livebuild/config/includes.chroot/etc/lightdm/lightdm-gtk-greeter.conf.d/50-distroforge-logo.conf")
        .unwrap()
        .contains("logo=/usr/share/distroforge/logo.png"));
    let mut bad = cfg_for(BaseId::Debian);
    bad.logo = "data:image/gif;base64,R0lGOD".to_string();
    assert!(validate(&bad).iter().any(|i| i.contains("Logo")));
}

#[test]
fn eula_fichero_y_profile_d_una_vez() {
    let mut c = cfg_for(BaseId::Debian);
    c.eula_text = "Al usar esta distro aceptas que...".to_string();
    let f = generate_project(&c);
    assert!(f
        .get("livebuild/config/includes.chroot/etc/distroforge/EULA")
        .unwrap()
        .contains("aceptas"));
    let sh = f
        .get("livebuild/config/includes.chroot/etc/profile.d/zz-distroforge-eula.sh")
        .unwrap();
    assert!(sh.contains("eula-shown"));
    assert!(sh.contains("[ -t 0 ]"));
}

#[test]
fn desatendida_preseed_y_bootappend_solo_debian() {
    let mut c = cfg_for(BaseId::Debian);
    c.unattended = true;
    let f = generate_project(&c);
    assert!(f.get("livebuild/auto/config").unwrap().contains("--debian-installer live"));
    let pre = f
        .get("livebuild/config/includes.installer/preseed.cfg")
        .unwrap();
    assert!(pre.contains("passwd/username string user"));
    assert!(pre.contains("time/zone string Europe/Madrid"));
    // sin unattended no hay preseed ni --debian-installer
    let off = generate_project(&cfg_for(BaseId::Debian));
    assert!(!off.contains_key("livebuild/config/includes.installer/preseed.cfg"));
    assert!(!off
        .get("livebuild/auto/config")
        .unwrap()
        .contains("--debian-installer"));
    // la línea --bootappend-install SIEMPRE está (vacía sin unattended)
    assert!(off
        .get("livebuild/auto/config")
        .unwrap()
        .contains("--bootappend-install \"\""));
    let mut k = cfg_for(BaseId::Fedora);
    k.unattended = true;
    assert!(validate(&k).iter().any(|i| i.contains("preseed")));
}

#[test]
fn vhd_solo_con_kiwi() {
    let mut k = cfg_for(BaseId::OpensuseTumbleweed);
    k.output = OutputFormat::Vhd;
    assert!(generate_project(&k)
        .get("kiwi/config.xml")
        .unwrap()
        .contains("format=\"vhd\""));
    let mut d = cfg_for(BaseId::Debian);
    d.output = OutputFormat::Vhd;
    assert!(validate(&d).iter().any(|i| i.contains("VHD")));
}

#[test]
fn smoke_yml_solo_en_iso_yaml_valido_y_cosign_en_oci() {
    let mut c = cfg_for(BaseId::Debian);
    c.smoke_test = true;
    let files = generate_project(&c);
    let smoke = files.get(".github/workflows/smoke.yml").unwrap();
    assert!(smoke.contains("qemu-system-x86_64"));
    serde_yaml::from_str::<serde_yaml::Value>(smoke).unwrap();
    assert!(!generate_project(&cfg_for(BaseId::Debian))
        .contains_key(".github/workflows/smoke.yml"));
    let mut o = cfg_for(BaseId::Debian);
    o.output = OutputFormat::Oci;
    o.sign_image = true;
    let yml = generate_project(&o)
        .get(".github/workflows/build.yml")
        .unwrap()
        .clone();
    assert!(yml.contains("cosign sign --yes"));
    serde_yaml::from_str::<serde_yaml::Value>(&yml).unwrap();
}

#[test]
fn estimate_size_mb_crece_con_escritorio_paquetes_y_flatpaks() {
    let base = forge_core::generators::estimate_size_mb(&cfg_for(BaseId::Debian));
    let mut con_desk = cfg_for(BaseId::Debian);
    con_desk.desktop = "gnome".to_string();
    let mut con_paks = cfg_for(BaseId::Debian);
    con_paks.packages = vec!["paquete".to_string(); 20];
    let mut con_flat = cfg_for(BaseId::Debian);
    con_flat.flatpak_apps = vec!["a.b.C".to_string(), "d.e.F".to_string()];
    assert!(
        forge_core::generators::estimate_size_mb(&con_desk) > base,
        "escritorio debe pesar más"
    );
    assert!(
        forge_core::generators::estimate_size_mb(&con_paks) > base,
        "20 paquetes deben pesar más"
    );
    assert!(
        forge_core::generators::estimate_size_mb(&con_flat) > base,
        "flatpaks deben pesar más"
    );
}

// ---------------------------------------------------------------- bloque 17

#[test]
fn autoyast_oem_unattended_autoinst_y_kickstart_avisa() {
    let mut k = cfg_for(BaseId::OpensuseTumbleweed);
    k.unattended = true;
    k.output = OutputFormat::Usb;
    let f = generate_project(&k);
    assert!(f.get("kiwi/config.xml").unwrap().contains("oem-unattended=\"true\""));
    let xml = f.get("kiwi/autoinst.xml").unwrap();
    assert!(xml.contains("<username>user</username>"));
    assert!(xml.contains("<timezone>Europe/Madrid</timezone>"));
    assert!(validate(&k).is_empty(), "{:?}", validate(&k));
    let mut fed = cfg_for(BaseId::Fedora);
    fed.unattended = true;
    assert!(validate(&fed).iter().any(|i| i.contains("desatendido")));
}

#[test]
fn test_drive_capturas_solo_en_iso_yaml_valido() {
    let mut c = cfg_for(BaseId::Debian);
    c.test_drive = true;
    let f = generate_project(&c);
    let td = f.get(".github/workflows/testdrive.yml").unwrap();
    assert!(td.contains("screendump"));
    assert!(td.contains("pnmtopng"));
    serde_yaml::from_str::<serde_yaml::Value>(td).unwrap();
    c.output = OutputFormat::Oci;
    assert!(!generate_project(&c)
        .contains_key(".github/workflows/testdrive.yml"));
    assert!(validate(&c).iter().any(|i| i.contains("Test Drive")));
}

#[test]
fn nube_workflows_aws_y_azure_con_disco_requerido() {
    let mut aws = cfg_for(BaseId::OpensuseTumbleweed);
    aws.output = OutputFormat::Qcow2;
    aws.cloud_upload = forge_core::types::CloudUpload::Aws;
    let yml = generate_project(&aws)
        .get(".github/workflows/cloud-upload.yml")
        .unwrap()
        .clone();
    assert!(yml.contains("secrets.AWS_ACCESS_KEY_ID"));
    assert!(yml.contains("import-snapshot"));
    serde_yaml::from_str::<serde_yaml::Value>(&yml).unwrap();

    let mut az = cfg_for(BaseId::OpensuseTumbleweed);
    az.output = OutputFormat::Vhd;
    az.cloud_upload = forge_core::types::CloudUpload::Azure;
    let yml2 = generate_project(&az)
        .get(".github/workflows/cloud-upload.yml")
        .unwrap()
        .clone();
    assert!(yml2.contains("az storage blob upload"));
    serde_yaml::from_str::<serde_yaml::Value>(&yml2).unwrap();

    let mut bad = cfg_for(BaseId::Debian);
    bad.cloud_upload = forge_core::types::CloudUpload::Aws;
    assert!(validate(&bad).iter().any(|i| i.contains("Nube")));
}

#[test]
fn el_workflow_de_livebuild_corre_con_privilegios() {
    // `lb build` monta /proc y /dev/pts dentro del chroot del live: sin
    // --privileged falla con "mount: permission denied" y el proyecto generado no
    // se construye. El build.sh local ya lo llevaba; el workflow de CI no.
    let lb = generate_project(&cfg_for(BaseId::Debian));
    let yml = lb.get(".github/workflows/build.yml").unwrap();
    assert!(
        yml.contains("options: --privileged"),
        "el workflow de live-build necesita privilegios para montar el chroot"
    );
    assert!(yml.contains("lb build"));
    serde_yaml::from_str::<serde_yaml::Value>(yml).unwrap();

    // y el script local sigue llevándolos
    assert!(lb.get("build.sh").unwrap().contains("--privileged"));
}
