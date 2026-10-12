//! Catálogo de bases, repos preset y defaults, portado de `src/catalog.ts`.
//!
//! Todo el catálogo vive en `static`s const-construibles (`&'static str` en vez
//! de `String`): `base_by_id` es una función total sobre el enum [`BaseId`] con
//! índices verificados en tiempo de compilación, así que no puede fallar (el
//! TS lanza en `baseById` con ids desconocidos; en Rust ese estado no existe).

use crate::types::{
    BaseId, CustomRepo, DistroConfig, GrubConfig, NetworkConfig, NetMode, OutputFormat,
    PartialConfig, SystemConfig,
};

/// Definición de escritorio (`DesktopDef`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DesktopDef {
    /// Id del escritorio ("gnome", "kde", "none"...).
    pub id: &'static str,
    /// Etiqueta visible.
    pub label: &'static str,
    /// Paquetes que instala.
    pub packages: &'static [&'static str],
}

/// Repo preset const-construible. Todas las entradas del catálogo TS tienen
/// `gpgKey: ""` y `enabled: true`, así que bastan alias/url/suite.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PresetRepo {
    /// Alias.
    pub alias: &'static str,
    /// URL.
    pub url: &'static str,
    /// Suite y componentes ("" = repo plano).
    pub suite: &'static str,
}

/// Definición de base (`BaseDef`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BaseDef {
    /// Id.
    pub id: BaseId,
    /// Etiqueta visible.
    pub label: &'static str,
    /// Motor de construcción.
    pub engine: crate::types::EngineId,
    /// Mirror principal ("" en bases manuales).
    pub mirror: &'static str,
    /// live-build: versión de la distro objetivo (bookworm, noble...).
    pub distribution: Option<&'static str>,
    /// live-build: áreas de archivo.
    pub archive_areas: Option<&'static str>,
    /// live-build: flavour del kernel (amd64 en Debian, generic en Ubuntu).
    pub lb_flavour: Option<&'static str>,
    /// Imagen de contenedor para build local y CI (live-build).
    pub container: Option<&'static str>,
    /// Paquete del kernel según la base.
    pub kernel_package: Option<&'static str>,
    /// Repos del fabricante que se añaden solos al elegir la base.
    pub preset_repos: &'static [PresetRepo],
    /// Escritorios disponibles.
    pub desktops: &'static [DesktopDef],
    /// Paquetes sugeridos en la UI.
    pub suggestions: &'static [&'static str],
}

const SUGGESTIONS: [&str; 17] = [
    "firefox",
    "vim",
    "git",
    "htop",
    "neovim",
    "gimp",
    "vlc",
    "libreoffice",
    "podman",
    "docker",
    "tmux",
    "mc",
    "nginx",
    "cowsay",
    "neofetch",
    "keepassxc",
    "thunderbird",
];

const SUSE_SUGGESTIONS: [&str; 19] = [
    "firefox",
    "vim",
    "git",
    "htop",
    "neovim",
    "gimp",
    "vlc",
    "libreoffice",
    "podman",
    "docker",
    "tmux",
    "mc",
    "nginx",
    "cowsay",
    "neofetch",
    "keepassxc",
    "thunderbird",
    "MozillaFirefox",
    "konsole",
];

static SUSE_DESKTOPS: [DesktopDef; 10] = [
    DesktopDef { id: "none", label: "Sin escritorio (servidor)", packages: &[] },
    DesktopDef { id: "kde", label: "KDE Plasma", packages: &["pattern:kde", "sddm"] },
    DesktopDef { id: "gnome", label: "GNOME", packages: &["pattern:gnome", "gdm"] },
    DesktopDef { id: "xfce", label: "Xfce", packages: &["pattern:xfce", "lightdm"] },
    DesktopDef { id: "mate", label: "MATE", packages: &["pattern:mate", "lightdm"] },
    DesktopDef { id: "lxqt", label: "LXQt", packages: &["pattern:lxqt", "sddm"] },
    DesktopDef { id: "cinnamon", label: "Cinnamon", packages: &["pattern:cinnamon", "lightdm"] },
    DesktopDef { id: "budgie", label: "Budgie", packages: &["pattern:budgie", "lightdm"] },
    DesktopDef { id: "i3", label: "i3 (tiling WM)", packages: &["i3", "lightdm"] },
    DesktopDef { id: "sway", label: "Sway (Wayland WM)", packages: &["sway"] },
];

static DEBIAN_DESKTOPS: [DesktopDef; 10] = [
    DesktopDef { id: "none", label: "Sin escritorio (servidor)", packages: &[] },
    DesktopDef { id: "gnome", label: "GNOME", packages: &["task-gnome-desktop", "gdm3"] },
    DesktopDef { id: "kde", label: "KDE Plasma", packages: &["task-kde-desktop", "sddm"] },
    DesktopDef { id: "xfce", label: "Xfce", packages: &["task-xfce-desktop", "lightdm"] },
    DesktopDef { id: "mate", label: "MATE", packages: &["task-mate-desktop", "lightdm"] },
    DesktopDef { id: "lxqt", label: "LXQt", packages: &["task-lxqt-desktop", "sddm"] },
    DesktopDef { id: "cinnamon", label: "Cinnamon", packages: &["task-cinnamon-desktop", "lightdm"] },
    DesktopDef { id: "budgie", label: "Budgie", packages: &["task-budgie-desktop", "lightdm"] },
    DesktopDef { id: "i3", label: "i3 (tiling WM)", packages: &["i3", "i3status", "lightdm"] },
    DesktopDef { id: "sway", label: "Sway (Wayland WM)", packages: &["sway"] },
];

static UBUNTU_DESKTOPS: [DesktopDef; 10] = [
    DesktopDef { id: "none", label: "Sin escritorio (servidor)", packages: &[] },
    DesktopDef { id: "gnome", label: "GNOME", packages: &["ubuntu-desktop"] },
    DesktopDef { id: "kde", label: "KDE Plasma", packages: &["kubuntu-desktop"] },
    DesktopDef { id: "xfce", label: "Xfce", packages: &["xubuntu-desktop"] },
    DesktopDef { id: "mate", label: "MATE", packages: &["ubuntu-mate-desktop"] },
    DesktopDef { id: "lxqt", label: "LXQt", packages: &["lubuntu-desktop"] },
    DesktopDef { id: "cinnamon", label: "Cinnamon", packages: &["cinnamon-desktop-environment"] },
    DesktopDef { id: "budgie", label: "Budgie", packages: &["ubuntu-budgie-desktop"] },
    DesktopDef { id: "i3", label: "i3 (tiling WM)", packages: &["i3", "i3status"] },
    DesktopDef { id: "sway", label: "Sway (Wayland WM)", packages: &["sway"] },
];

static MINT_DESKTOPS: [DesktopDef; 4] = [
    DesktopDef { id: "none", label: "Sin escritorio (servidor)", packages: &[] },
    DesktopDef { id: "cinnamon", label: "Cinnamon", packages: &["cinnamon", "mint-meta-cinnamon"] },
    DesktopDef { id: "mate", label: "MATE", packages: &["mint-meta-mate"] },
    DesktopDef { id: "xfce", label: "Xfce", packages: &["mint-meta-xfce"] },
];

static ARCH_DESKTOPS: [DesktopDef; 10] = [
    DesktopDef { id: "none", label: "Sin escritorio (servidor)", packages: &[] },
    DesktopDef { id: "kde", label: "KDE Plasma", packages: &["plasma-meta", "sddm"] },
    DesktopDef { id: "gnome", label: "GNOME", packages: &["gnome", "gdm"] },
    DesktopDef { id: "xfce", label: "Xfce", packages: &["xfce4", "xfce4-goodies", "lightdm"] },
    DesktopDef { id: "mate", label: "MATE", packages: &["mate", "lightdm"] },
    DesktopDef { id: "lxqt", label: "LXQt", packages: &["lxqt", "sddm"] },
    DesktopDef { id: "cinnamon", label: "Cinnamon", packages: &["cinnamon", "lightdm"] },
    DesktopDef { id: "budgie", label: "Budgie", packages: &["budgie-desktop", "lightdm"] },
    DesktopDef { id: "i3", label: "i3 (tiling WM)", packages: &["i3", "i3status"] },
    DesktopDef { id: "sway", label: "Sway (Wayland WM)", packages: &["sway"] },
];

// CachyOS = ARCH_DESKTOPS con "cachyos-settings" en "none" y
// "cachyos-kde-settings" añadido a KDE (map del TS).
static CACHYOS_DESKTOPS: [DesktopDef; 10] = [
    DesktopDef { id: "none", label: "Sin escritorio (servidor)", packages: &["cachyos-settings"] },
    DesktopDef {
        id: "kde",
        label: "KDE Plasma",
        packages: &["plasma-meta", "sddm", "cachyos-kde-settings"],
    },
    DesktopDef { id: "gnome", label: "GNOME", packages: &["gnome", "gdm"] },
    DesktopDef { id: "xfce", label: "Xfce", packages: &["xfce4", "xfce4-goodies", "lightdm"] },
    DesktopDef { id: "mate", label: "MATE", packages: &["mate", "lightdm"] },
    DesktopDef { id: "lxqt", label: "LXQt", packages: &["lxqt", "sddm"] },
    DesktopDef { id: "cinnamon", label: "Cinnamon", packages: &["cinnamon", "lightdm"] },
    DesktopDef { id: "budgie", label: "Budgie", packages: &["budgie-desktop", "lightdm"] },
    DesktopDef { id: "i3", label: "i3 (tiling WM)", packages: &["i3", "i3status"] },
    DesktopDef { id: "sway", label: "Sway (Wayland WM)", packages: &["sway"] },
];

static FEDORA_DESKTOPS: [DesktopDef; 10] = [
    DesktopDef { id: "none", label: "Sin escritorio (servidor)", packages: &[] },
    DesktopDef { id: "kde", label: "KDE Plasma", packages: &["@^kde-desktop-environment"] },
    DesktopDef { id: "gnome", label: "GNOME", packages: &["@^gnome-desktop-environment"] },
    DesktopDef { id: "xfce", label: "Xfce", packages: &["@^xfce-desktop-environment"] },
    DesktopDef { id: "mate", label: "MATE", packages: &["@mate-desktop-environment"] },
    DesktopDef { id: "lxqt", label: "LXQt", packages: &["@lxqt-desktop-environment"] },
    DesktopDef { id: "cinnamon", label: "Cinnamon", packages: &["@cinnamon-desktop-environment"] },
    DesktopDef { id: "budgie", label: "Budgie", packages: &["@budgie-desktop-environment"] },
    DesktopDef { id: "i3", label: "i3 (tiling WM)", packages: &["@i3-desktop-environment"] },
    DesktopDef { id: "sway", label: "Sway (Wayland WM)", packages: &["@sway-desktop-environment"] },
];

static GENTOO_DESKTOPS: [DesktopDef; 8] = [
    DesktopDef { id: "none", label: "Sin escritorio", packages: &[] },
    DesktopDef { id: "kde", label: "KDE Plasma", packages: &["kde-plasma/plasma-meta"] },
    DesktopDef { id: "gnome", label: "GNOME", packages: &["gnome-base/gnome"] },
    DesktopDef { id: "xfce", label: "Xfce", packages: &["xfce-base/xfce4"] },
    DesktopDef { id: "mate", label: "MATE", packages: &["mate-base/mate"] },
    DesktopDef { id: "lxqt", label: "LXQt", packages: &["lxqt-base/lxqt-meta"] },
    DesktopDef { id: "i3", label: "i3 (tiling WM)", packages: &["x11-wm/i3"] },
    DesktopDef { id: "sway", label: "Sway (Wayland WM)", packages: &["gui-wm/sway"] },
];

static ELEMENTARY_DESKTOPS: [DesktopDef; 3] = [
    DesktopDef { id: "none", label: "Sin escritorio (servidor)", packages: &[] },
    DesktopDef { id: "pantheon", label: "Pantheon", packages: &["elementary-desktop"] },
    DesktopDef { id: "gnome", label: "GNOME", packages: &["ubuntu-desktop"] },
];

static POP_DESKTOPS: [DesktopDef; 3] = [
    DesktopDef { id: "none", label: "Sin escritorio (servidor)", packages: &[] },
    DesktopDef { id: "popgnome", label: "Pop! GNOME", packages: &["pop-desktop"] },
    DesktopDef { id: "cosmic", label: "COSMIC", packages: &["cosmic-desktop"] },
];

static ZORIN_DESKTOPS: [DesktopDef; 3] = [
    DesktopDef { id: "none", label: "Sin escritorio (servidor)", packages: &[] },
    DesktopDef { id: "zorincore", label: "Zorin Core (GNOME)", packages: &["zorin-os-desktop"] },
    DesktopDef { id: "gnome", label: "GNOME estándar", packages: &["ubuntu-desktop"] },
];

static SOLUS_DESKTOPS: [DesktopDef; 4] = [
    DesktopDef { id: "none", label: "Sin escritorio", packages: &[] },
    DesktopDef { id: "budgie", label: "Budgie", packages: &["budgie-desktop"] },
    DesktopDef { id: "gnome", label: "GNOME", packages: &["gnome"] },
    DesktopDef { id: "xfce", label: "Xfce", packages: &["xfce4-session"] },
];

static MINT_PRESET_REPOS: [PresetRepo; 1] = [PresetRepo {
    alias: "linuxmint",
    url: "http://packages.linuxmint.com",
    suite: "wilma main upstream import backport",
}];

static ELEMENTARY_PRESET_REPOS: [PresetRepo; 1] = [PresetRepo {
    alias: "elementary",
    url: "https://packages.elementary.io/elementary/os/8.0",
    suite: "",
}];

static POP_PRESET_REPOS: [PresetRepo; 1] = [PresetRepo {
    alias: "pop-os",
    url: "http://apt.pop-os.org/release",
    suite: "jammy main",
}];

static ZORIN_PRESET_REPOS: [PresetRepo; 1] = [PresetRepo {
    alias: "zorin",
    url: "https://packages.zorin.com/os/18",
    suite: "",
}];

static CACHYOS_PRESET_REPOS: [PresetRepo; 1] = [PresetRepo {
    alias: "cachyos",
    url: "https://cdn.cachyos.org/repo/x86_64",
    suite: "",
}];

const DEBIAN_SUGGESTIONS: [&str; 18] = [
    "firefox",
    "vim",
    "git",
    "htop",
    "neovim",
    "gimp",
    "vlc",
    "libreoffice",
    "podman",
    "docker",
    "tmux",
    "mc",
    "nginx",
    "cowsay",
    "neofetch",
    "keepassxc",
    "thunderbird",
    "firefox-esr",
];

const UBUNTU_SUGGESTIONS: [&str; 18] = [
    "firefox",
    "vim",
    "git",
    "htop",
    "neovim",
    "gimp",
    "vlc",
    "libreoffice",
    "podman",
    "docker",
    "tmux",
    "mc",
    "nginx",
    "cowsay",
    "neofetch",
    "keepassxc",
    "thunderbird",
    "ubuntu-restricted-extras",
];

const MINT_SUGGESTIONS: [&str; 19] = [
    "firefox",
    "vim",
    "git",
    "htop",
    "neovim",
    "gimp",
    "vlc",
    "libreoffice",
    "podman",
    "docker",
    "tmux",
    "mc",
    "nginx",
    "cowsay",
    "neofetch",
    "keepassxc",
    "thunderbird",
    "mintinstall",
    "mintupdate",
];

const ELEMENTARY_SUGGESTIONS: [&str; 19] = [
    "firefox",
    "vim",
    "git",
    "htop",
    "neovim",
    "gimp",
    "vlc",
    "libreoffice",
    "podman",
    "docker",
    "tmux",
    "mc",
    "nginx",
    "cowsay",
    "neofetch",
    "keepassxc",
    "thunderbird",
    "pantheon-files",
    "epiphany-browser",
];

const POP_SUGGESTIONS: [&str; 19] = [
    "firefox",
    "vim",
    "git",
    "htop",
    "neovim",
    "gimp",
    "vlc",
    "libreoffice",
    "podman",
    "docker",
    "tmux",
    "mc",
    "nginx",
    "cowsay",
    "neofetch",
    "keepassxc",
    "thunderbird",
    "pop-shop",
    "system76-driver",
];

const CACHYOS_SUGGESTIONS: [&str; 19] = [
    "firefox",
    "vim",
    "git",
    "htop",
    "neovim",
    "gimp",
    "vlc",
    "libreoffice",
    "podman",
    "docker",
    "tmux",
    "mc",
    "nginx",
    "cowsay",
    "neofetch",
    "keepassxc",
    "thunderbird",
    "cachyos-settings",
    "paru",
];

const GENTOO_SUGGESTIONS: [&str; 20] = [
    "firefox",
    "vim",
    "git",
    "htop",
    "neovim",
    "gimp",
    "vlc",
    "libreoffice",
    "podman",
    "docker",
    "tmux",
    "mc",
    "nginx",
    "cowsay",
    "neofetch",
    "keepassxc",
    "thunderbird",
    "sys-kernel/gentoo-kernel",
    "app-portage/eix",
    "app-shells/zsh",
];

/// Catálogo de las 14 bases. El orden replica `BASES` del TS; `base_by_id`
/// indexa este array con literales verificados en compilación.
pub static BASES: [BaseDef; 14] = [
    BaseDef {
        id: BaseId::OpensuseTumbleweed,
        label: "openSUSE Tumbleweed",
        engine: crate::types::EngineId::Kiwi,
        mirror: "https://download.opensuse.org/tumbleweed/repo/oss/",
        distribution: None,
        archive_areas: None,
        lb_flavour: None,
        container: None,
        kernel_package: Some("kernel-default"),
        preset_repos: &[],
        desktops: &SUSE_DESKTOPS,
        suggestions: &SUSE_SUGGESTIONS,
    },
    BaseDef {
        id: BaseId::OpensuseLeap,
        label: "openSUSE Leap 15.6",
        engine: crate::types::EngineId::Kiwi,
        mirror: "https://download.opensuse.org/distribution/leap/15.6/repo/oss/",
        distribution: None,
        archive_areas: None,
        lb_flavour: None,
        container: None,
        kernel_package: Some("kernel-default"),
        preset_repos: &[],
        desktops: &SUSE_DESKTOPS,
        suggestions: &SUSE_SUGGESTIONS,
    },
    BaseDef {
        id: BaseId::Debian,
        label: "Debian 12 (bookworm)",
        engine: crate::types::EngineId::Livebuild,
        mirror: "http://deb.debian.org/debian",
        distribution: Some("bookworm"),
        archive_areas: Some("main contrib non-free non-free-firmware"),
        lb_flavour: Some("amd64"),
        container: Some("debian:bookworm"),
        kernel_package: Some("linux-image-amd64"),
        preset_repos: &[],
        desktops: &DEBIAN_DESKTOPS,
        suggestions: &DEBIAN_SUGGESTIONS,
    },
    BaseDef {
        id: BaseId::Ubuntu,
        label: "Ubuntu 24.04 (noble)",
        engine: crate::types::EngineId::Livebuild,
        mirror: "http://archive.ubuntu.com/ubuntu",
        distribution: Some("noble"),
        archive_areas: Some("main restricted universe multiverse"),
        lb_flavour: Some("generic"),
        container: Some("ubuntu:24.04"),
        kernel_package: Some("linux-image-generic"),
        preset_repos: &[],
        desktops: &UBUNTU_DESKTOPS,
        suggestions: &UBUNTU_SUGGESTIONS,
    },
    BaseDef {
        id: BaseId::Mint,
        label: "Linux Mint 22",
        engine: crate::types::EngineId::Livebuild,
        mirror: "http://archive.ubuntu.com/ubuntu",
        distribution: Some("noble"),
        archive_areas: Some("main restricted universe multiverse"),
        lb_flavour: Some("generic"),
        container: Some("ubuntu:24.04"),
        kernel_package: Some("linux-image-generic"),
        preset_repos: &MINT_PRESET_REPOS,
        desktops: &MINT_DESKTOPS,
        suggestions: &MINT_SUGGESTIONS,
    },
    BaseDef {
        id: BaseId::Elementary,
        label: "elementary OS 8",
        engine: crate::types::EngineId::Livebuild,
        mirror: "http://archive.ubuntu.com/ubuntu",
        distribution: Some("noble"),
        archive_areas: Some("main restricted universe multiverse"),
        lb_flavour: Some("generic"),
        container: Some("ubuntu:24.04"),
        kernel_package: Some("linux-image-generic"),
        preset_repos: &ELEMENTARY_PRESET_REPOS,
        desktops: &ELEMENTARY_DESKTOPS,
        suggestions: &ELEMENTARY_SUGGESTIONS,
    },
    BaseDef {
        id: BaseId::Pop,
        label: "Pop!_OS 22.04",
        engine: crate::types::EngineId::Livebuild,
        mirror: "http://archive.ubuntu.com/ubuntu",
        distribution: Some("jammy"),
        archive_areas: Some("main restricted universe multiverse"),
        lb_flavour: Some("generic"),
        container: Some("ubuntu:22.04"),
        kernel_package: Some("linux-image-generic"),
        preset_repos: &POP_PRESET_REPOS,
        desktops: &POP_DESKTOPS,
        suggestions: &POP_SUGGESTIONS,
    },
    BaseDef {
        id: BaseId::Zorin,
        label: "Zorin OS 18",
        engine: crate::types::EngineId::Livebuild,
        mirror: "http://archive.ubuntu.com/ubuntu",
        distribution: Some("noble"),
        archive_areas: Some("main restricted universe multiverse"),
        lb_flavour: Some("generic"),
        container: Some("ubuntu:24.04"),
        kernel_package: Some("linux-image-generic"),
        preset_repos: &ZORIN_PRESET_REPOS,
        desktops: &ZORIN_DESKTOPS,
        suggestions: &SUGGESTIONS,
    },
    BaseDef {
        id: BaseId::Fedora,
        label: "Fedora 44",
        engine: crate::types::EngineId::Kickstart,
        mirror: "https://mirrors.fedoraproject.org/mirrorlist?repo=fedora-41&arch=x86_64",
        distribution: None,
        archive_areas: None,
        lb_flavour: None,
        container: None,
        kernel_package: None,
        preset_repos: &[],
        desktops: &FEDORA_DESKTOPS,
        suggestions: &SUGGESTIONS,
    },
    BaseDef {
        id: BaseId::Arch,
        label: "Arch Linux",
        engine: crate::types::EngineId::Mkarchiso,
        mirror: "https://geo.mirror.pkgbuild.com/$repo/os/$arch",
        distribution: None,
        archive_areas: None,
        lb_flavour: None,
        container: None,
        kernel_package: Some("linux"),
        preset_repos: &[],
        desktops: &ARCH_DESKTOPS,
        suggestions: &SUGGESTIONS,
    },
    BaseDef {
        id: BaseId::Manjaro,
        label: "Manjaro",
        engine: crate::types::EngineId::Mkarchiso,
        mirror: "https://ftp.halifax.rwth-aachen.de/manjaro/stable/$repo/os/$arch",
        distribution: None,
        archive_areas: None,
        lb_flavour: None,
        container: None,
        kernel_package: Some("linux612"),
        preset_repos: &[],
        desktops: &ARCH_DESKTOPS,
        suggestions: &SUGGESTIONS,
    },
    BaseDef {
        id: BaseId::Cachyos,
        label: "CachyOS",
        engine: crate::types::EngineId::Mkarchiso,
        mirror: "https://geo.mirror.pkgbuild.com/$repo/os/$arch",
        distribution: None,
        archive_areas: None,
        lb_flavour: None,
        container: None,
        kernel_package: Some("linux-cachyos"),
        preset_repos: &CACHYOS_PRESET_REPOS,
        desktops: &CACHYOS_DESKTOPS,
        suggestions: &CACHYOS_SUGGESTIONS,
    },
    BaseDef {
        id: BaseId::Gentoo,
        label: "Gentoo",
        engine: crate::types::EngineId::Manual,
        mirror: "",
        distribution: None,
        archive_areas: None,
        lb_flavour: None,
        container: None,
        kernel_package: None,
        preset_repos: &[],
        desktops: &GENTOO_DESKTOPS,
        suggestions: &GENTOO_SUGGESTIONS,
    },
    BaseDef {
        id: BaseId::Solus,
        label: "Solus",
        engine: crate::types::EngineId::Manual,
        mirror: "",
        distribution: None,
        archive_areas: None,
        lb_flavour: None,
        container: None,
        kernel_package: None,
        preset_repos: &[],
        desktops: &SOLUS_DESKTOPS,
        suggestions: &SUGGESTIONS,
    },
];

/// Devuelve la definición de una base. Es total: el parámetro es el enum
/// `BaseId` y el array cubre todas sus variantes (índices literales que el
/// compilador comprueba contra la longitud de `BASES`).
pub fn base_by_id(id: BaseId) -> &'static BaseDef {
    match id {
        BaseId::OpensuseTumbleweed => &BASES[0],
        BaseId::OpensuseLeap => &BASES[1],
        BaseId::Debian => &BASES[2],
        BaseId::Ubuntu => &BASES[3],
        BaseId::Mint => &BASES[4],
        BaseId::Elementary => &BASES[5],
        BaseId::Pop => &BASES[6],
        BaseId::Zorin => &BASES[7],
        BaseId::Fedora => &BASES[8],
        BaseId::Arch => &BASES[9],
        BaseId::Manjaro => &BASES[10],
        BaseId::Cachyos => &BASES[11],
        BaseId::Gentoo => &BASES[12],
        BaseId::Solus => &BASES[13],
    }
}

/// Paquetes del escritorio elegido; vacío si el id no existe en la base.
pub fn desktop_packages(base: BaseId, desktop_id: &str) -> &'static [&'static str] {
    base_by_id(base)
        .desktops
        .iter()
        .find(|d| d.id == desktop_id)
        .map(|d| d.packages)
        .unwrap_or(&[])
}

/// Arquitecturas que cada motor sabe construir de forma fiable.
pub fn archs_for_engine(engine: crate::types::EngineId) -> &'static [crate::types::ArchId] {
    use crate::types::{ArchId, EngineId};
    match engine {
        EngineId::Livebuild => &[ArchId::X86_64, ArchId::Aarch64, ArchId::Armhf, ArchId::I386],
        EngineId::Kickstart | EngineId::Kiwi => &[ArchId::X86_64, ArchId::Aarch64],
        // mkarchiso y manual: solo x86_64
        _ => &[ArchId::X86_64],
    }
}

/// Nombres de arquitectura estilo Debian (live-build, debootstrap). Con
/// `ArchId` como enum el mapa es total (el TS usa `?? "amd64"`).
pub const fn deb_arch(arch: crate::types::ArchId) -> &'static str {
    use crate::types::ArchId;
    match arch {
        ArchId::X86_64 => "amd64",
        ArchId::Aarch64 => "arm64",
        ArchId::Armhf => "armhf",
        ArchId::I386 => "i386",
    }
}

/// Convierte un preset del catálogo en un repo de usuario editable.
fn preset_to_custom(p: &PresetRepo) -> CustomRepo {
    CustomRepo {
        alias: p.alias.to_string(),
        url: p.url.to_string(),
        gpg_key: String::new(),
        suite: p.suite.to_string(),
        enabled: true,
    }
}

/// ¿Es `alias` un alias preset de alguna base?
fn is_preset_alias(alias: &str) -> bool {
    BASES.iter()
        .flat_map(|b| b.preset_repos.iter())
        .any(|p| p.alias == alias)
}

/// Al cambiar de base: quita presets de la base anterior y añade los de la
/// nueva. Los repos añadidos por el usuario (que no son preset de ninguna
/// base) se conservan. Port de `mergePresets`.
pub fn merge_presets(next_base: &BaseDef, repos: &[CustomRepo]) -> Vec<CustomRepo> {
    let user_repos: Vec<CustomRepo> = repos
        .iter()
        .filter(|r| !is_preset_alias(&r.alias))
        .cloned()
        .collect();
    let presets: Vec<CustomRepo> = next_base
        .preset_repos
        .iter()
        .filter(|p| !user_repos.iter().any(|r| r.alias == p.alias))
        .map(preset_to_custom)
        .collect();
    let mut out = presets;
    out.extend(user_repos);
    out
}

/// Repos efectivos de una config: presets de la base (respetando si el usuario
/// los desactivó) + repos del usuario. Los generadores usan esto, no
/// `cfg.repos` a pelo. Port de `effectiveRepos`.
pub fn effective_repos(cfg: &DistroConfig) -> Vec<CustomRepo> {
    let base = base_by_id(cfg.base);
    let presets = base.preset_repos;
    let user_repos: Vec<&CustomRepo> = cfg
        .repos
        .iter()
        .filter(|r| {
            !presets.iter().any(|p| p.alias == r.alias) && r.enabled && !r.url.is_empty()
        })
        .collect();
    let disabled_presets: Vec<&str> = cfg
        .repos
        .iter()
        .filter(|r| presets.iter().any(|p| p.alias == r.alias) && !r.enabled)
        .map(|r| r.alias.as_str())
        .collect();
    let mut out: Vec<CustomRepo> = presets
        .iter()
        .filter(|p| {
            !disabled_presets.contains(&p.alias)
                && !user_repos.iter().any(|r| r.alias == p.alias)
        })
        .map(preset_to_custom)
        .collect();
    out.extend(user_repos.into_iter().cloned());
    out
}

/// Zonas horarias ofrecidas por la UI.
pub static TIMEZONES: [&str; 15] = [
    "Europe/Madrid",
    "Europe/Lisbon",
    "Europe/Canary",
    "UTC",
    "Europe/London",
    "Europe/Paris",
    "Europe/Berlin",
    "America/Mexico_City",
    "America/Bogota",
    "America/Lima",
    "America/Santiago",
    "America/Argentina/Buenos_Aires",
    "America/Caracas",
    "America/New_York",
    "America/Los_Angeles",
];

/// Locales ofrecidos por la UI.
pub static LOCALES: [&str; 10] = [
    "es_ES.UTF-8",
    "es_MX.UTF-8",
    "es_AR.UTF-8",
    "es_CO.UTF-8",
    "es_CL.UTF-8",
    "en_US.UTF-8",
    "pt_BR.UTF-8",
    "fr_FR.UTF-8",
    "de_DE.UTF-8",
    "it_IT.UTF-8",
];

/// Teclados ofrecidos por la UI.
pub static KEYBOARDS: [&str; 8] = ["es", "latam", "us", "gb", "de", "fr", "it", "pt"];

/// Config por defecto para una base (port de `defaultConfig`).
pub fn default_config(base: BaseId) -> DistroConfig {
    let b = base_by_id(base);
    DistroConfig {
        name: "mi-distro".to_string(),
        base,
        desktop: b.desktops[0].id.to_string(),
        packages: vec!["git".to_string(), "vim".to_string(), "htop".to_string()],
        repos: vec![],
        system: SystemConfig {
            hostname: "mi-distro".to_string(),
            username: "user".to_string(),
            password: String::new(),
            timezone: "Europe/Madrid".to_string(),
            locale: "es_ES.UTF-8".to_string(),
            keyboard: "es".to_string(),
        },
        output: OutputFormat::Iso,
        flatpak_apps: vec![],
        extra_files: vec![],
        users: vec![],
        services: vec![],
        network: NetworkConfig {
            mode: NetMode::Dhcp,
            ip: String::new(),
            prefix: "24".to_string(),
            gateway: String::new(),
            dns: vec![],
        },
        grub: GrubConfig {
            timeout: 10.0,
            extra_params: String::new(),
        },
        firewall: true,
        arch: crate::types::ArchId::X86_64,
        advanced: false,
        custom_iso: Default::default(),
        autologin: false,
        autostart: vec![],
        firewall_ports: vec![],
        boot_target: crate::types::BootTarget::Auto,
        logo: String::new(),
        start_icon: "◆".to_string(),
        eula_text: String::new(),
        unattended: false,
        smoke_test: false,
        sign_image: false,
        test_drive: false,
        cloud_upload: crate::types::CloudUpload::None,
        cloud_init: false,
        motd: "Bienvenido a tu distro personalizada (DistroForge)".to_string(),
        wallpaper: String::new(),
    }
}

/// Fusiona una config parcial (localStorage, JSON importado, URL) sobre los
/// defaults de la base, en profundidad en system/network/grub/customIso (los
/// demás campos se sustituyen en bloque, como el spread del TS).
pub fn merge_config(base: BaseId, parsed: PartialConfig) -> DistroConfig {
    let mut d = default_config(base);
    let p = parsed;
    if let Some(v) = p.name { d.name = v; }
    if let Some(v) = p.base { d.base = v; }
    if let Some(v) = p.desktop { d.desktop = v; }
    if let Some(v) = p.packages { d.packages = v; }
    if let Some(v) = p.repos { d.repos = v; }
    if let Some(s) = p.system {
        if let Some(v) = s.hostname { d.system.hostname = v; }
        if let Some(v) = s.username { d.system.username = v; }
        if let Some(v) = s.password { d.system.password = v; }
        if let Some(v) = s.timezone { d.system.timezone = v; }
        if let Some(v) = s.locale { d.system.locale = v; }
        if let Some(v) = s.keyboard { d.system.keyboard = v; }
    }
    if let Some(v) = p.output { d.output = v; }
    if let Some(v) = p.flatpak_apps { d.flatpak_apps = v; }
    if let Some(v) = p.extra_files { d.extra_files = v; }
    if let Some(v) = p.users { d.users = v; }
    if let Some(v) = p.services { d.services = v; }
    if let Some(n) = p.network {
        if let Some(v) = n.mode { d.network.mode = v; }
        if let Some(v) = n.ip { d.network.ip = v; }
        if let Some(v) = n.prefix { d.network.prefix = v; }
        if let Some(v) = n.gateway { d.network.gateway = v; }
        if let Some(v) = n.dns { d.network.dns = v; }
    }
    if let Some(g) = p.grub {
        if let Some(v) = g.timeout { d.grub.timeout = v; }
        if let Some(v) = g.extra_params { d.grub.extra_params = v; }
    }
    if let Some(v) = p.firewall { d.firewall = v; }
    if let Some(v) = p.arch { d.arch = v; }
    if let Some(v) = p.advanced { d.advanced = v; }
    if let Some(c) = p.custom_iso {
        if let Some(v) = c.url { d.custom_iso.url = v; }
        if let Some(v) = c.chroot_script { d.custom_iso.chroot_script = v; }
    }
    if let Some(v) = p.autologin { d.autologin = v; }
    if let Some(v) = p.autostart { d.autostart = v; }
    if let Some(v) = p.firewall_ports { d.firewall_ports = v; }
    if let Some(v) = p.boot_target { d.boot_target = v; }
    if let Some(v) = p.logo { d.logo = v; }
    if let Some(v) = p.start_icon { d.start_icon = v; }
    if let Some(v) = p.eula_text { d.eula_text = v; }
    if let Some(v) = p.unattended { d.unattended = v; }
    if let Some(v) = p.smoke_test { d.smoke_test = v; }
    if let Some(v) = p.sign_image { d.sign_image = v; }
    if let Some(v) = p.test_drive { d.test_drive = v; }
    if let Some(v) = p.cloud_upload { d.cloud_upload = v; }
    if let Some(v) = p.cloud_init { d.cloud_init = v; }
    if let Some(v) = p.motd { d.motd = v; }
    if let Some(v) = p.wallpaper { d.wallpaper = v; }
    d
}

/// Asignación superficial tipo `Object.assign` (la que hace `scripts/dump.ts`
/// con el config.json opcional): los objetos anidados presentes sustituyen
/// campo a campo lo actual, conservando el valor actual donde el parcial no
/// trae nada (el TS inyectaría `undefined`; aquí se mantiene el default, que
/// es lo único razonable con tipos concretos).
pub fn assign_partial(cfg: &mut DistroConfig, p: PartialConfig) {
    if let Some(v) = p.name { cfg.name = v; }
    if let Some(v) = p.base { cfg.base = v; }
    if let Some(v) = p.desktop { cfg.desktop = v; }
    if let Some(v) = p.packages { cfg.packages = v; }
    if let Some(v) = p.repos { cfg.repos = v; }
    if let Some(s) = p.system {
        if let Some(v) = s.hostname { cfg.system.hostname = v; }
        if let Some(v) = s.username { cfg.system.username = v; }
        if let Some(v) = s.password { cfg.system.password = v; }
        if let Some(v) = s.timezone { cfg.system.timezone = v; }
        if let Some(v) = s.locale { cfg.system.locale = v; }
        if let Some(v) = s.keyboard { cfg.system.keyboard = v; }
    }
    if let Some(v) = p.output { cfg.output = v; }
    if let Some(v) = p.flatpak_apps { cfg.flatpak_apps = v; }
    if let Some(v) = p.extra_files { cfg.extra_files = v; }
    if let Some(v) = p.users { cfg.users = v; }
    if let Some(v) = p.services { cfg.services = v; }
    if let Some(n) = p.network {
        if let Some(v) = n.mode { cfg.network.mode = v; }
        if let Some(v) = n.ip { cfg.network.ip = v; }
        if let Some(v) = n.prefix { cfg.network.prefix = v; }
        if let Some(v) = n.gateway { cfg.network.gateway = v; }
        if let Some(v) = n.dns { cfg.network.dns = v; }
    }
    if let Some(g) = p.grub {
        if let Some(v) = g.timeout { cfg.grub.timeout = v; }
        if let Some(v) = g.extra_params { cfg.grub.extra_params = v; }
    }
    if let Some(v) = p.firewall { cfg.firewall = v; }
    if let Some(v) = p.arch { cfg.arch = v; }
    if let Some(v) = p.advanced { cfg.advanced = v; }
    if let Some(c) = p.custom_iso {
        if let Some(v) = c.url { cfg.custom_iso.url = v; }
        if let Some(v) = c.chroot_script { cfg.custom_iso.chroot_script = v; }
    }
    if let Some(v) = p.autologin { cfg.autologin = v; }
    if let Some(v) = p.autostart { cfg.autostart = v; }
    if let Some(v) = p.firewall_ports { cfg.firewall_ports = v; }
    if let Some(v) = p.boot_target { cfg.boot_target = v; }
    if let Some(v) = p.logo { cfg.logo = v; }
    if let Some(v) = p.start_icon { cfg.start_icon = v; }
    if let Some(v) = p.eula_text { cfg.eula_text = v; }
    if let Some(v) = p.unattended { cfg.unattended = v; }
    if let Some(v) = p.smoke_test { cfg.smoke_test = v; }
    if let Some(v) = p.sign_image { cfg.sign_image = v; }
    if let Some(v) = p.test_drive { cfg.test_drive = v; }
    if let Some(v) = p.cloud_upload { cfg.cloud_upload = v; }
    if let Some(v) = p.cloud_init { cfg.cloud_init = v; }
    if let Some(v) = p.motd { cfg.motd = v; }
    if let Some(v) = p.wallpaper { cfg.wallpaper = v; }
}
