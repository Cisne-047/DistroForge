import type { BaseId, CustomRepo, DistroConfig, EngineId } from "./types";

export interface DesktopDef {
  id: string;
  label: string;
  packages: string[];
}

export interface BaseDef {
  id: BaseId;
  label: string;
  engine: EngineId;
  mirror: string;
  /** live-build: versión de la distro objetivo (bookworm, noble...) */
  distribution?: string;
  /** live-build: áreas de archivo */
  archiveAreas?: string;
  /** live-build: flavour del kernel (amd64 en Debian, generic en Ubuntu) */
  lbFlavour?: string;
  /** imagen de contenedor para build local y CI (live-build) */
  container?: string;
  /** paquete del kernel según la base */
  kernelPackage?: string;
  /** repos del fabricante que se añaden solos al elegir la base */
  presetRepos?: CustomRepo[];
  desktops: DesktopDef[];
  suggestions: string[];
}

const SUGGESTIONS = [
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

const SUSE_SUGGESTIONS = [...SUGGESTIONS, "MozillaFirefox", "konsole"];

const SUSE_DESKTOPS: DesktopDef[] = [
  { id: "none", label: "Sin escritorio (servidor)", packages: [] },
  { id: "kde", label: "KDE Plasma", packages: ["pattern:kde", "sddm"] },
  { id: "gnome", label: "GNOME", packages: ["pattern:gnome", "gdm"] },
  { id: "xfce", label: "Xfce", packages: ["pattern:xfce", "lightdm"] },
  { id: "mate", label: "MATE", packages: ["pattern:mate", "lightdm"] },
  { id: "lxqt", label: "LXQt", packages: ["pattern:lxqt", "sddm"] },
  { id: "cinnamon", label: "Cinnamon", packages: ["pattern:cinnamon", "lightdm"] },
  { id: "budgie", label: "Budgie", packages: ["pattern:budgie", "lightdm"] },
  { id: "i3", label: "i3 (tiling WM)", packages: ["i3", "lightdm"] },
  { id: "sway", label: "Sway (Wayland WM)", packages: ["sway"] },
];

const DEBIAN_DESKTOPS: DesktopDef[] = [
  { id: "none", label: "Sin escritorio (servidor)", packages: [] },
  { id: "gnome", label: "GNOME", packages: ["task-gnome-desktop", "gdm3"] },
  { id: "kde", label: "KDE Plasma", packages: ["task-kde-desktop", "sddm"] },
  { id: "xfce", label: "Xfce", packages: ["task-xfce-desktop", "lightdm"] },
  { id: "mate", label: "MATE", packages: ["task-mate-desktop", "lightdm"] },
  { id: "lxqt", label: "LXQt", packages: ["task-lxqt-desktop", "sddm"] },
  { id: "cinnamon", label: "Cinnamon", packages: ["task-cinnamon-desktop", "lightdm"] },
  { id: "budgie", label: "Budgie", packages: ["task-budgie-desktop", "lightdm"] },
  { id: "i3", label: "i3 (tiling WM)", packages: ["i3", "i3status", "lightdm"] },
  { id: "sway", label: "Sway (Wayland WM)", packages: ["sway"] },
];

const UBUNTU_DESKTOPS: DesktopDef[] = [
  { id: "none", label: "Sin escritorio (servidor)", packages: [] },
  { id: "gnome", label: "GNOME", packages: ["ubuntu-desktop"] },
  { id: "kde", label: "KDE Plasma", packages: ["kubuntu-desktop"] },
  { id: "xfce", label: "Xfce", packages: ["xubuntu-desktop"] },
  { id: "mate", label: "MATE", packages: ["ubuntu-mate-desktop"] },
  { id: "lxqt", label: "LXQt", packages: ["lubuntu-desktop"] },
  { id: "cinnamon", label: "Cinnamon", packages: ["cinnamon-desktop-environment"] },
  { id: "budgie", label: "Budgie", packages: ["ubuntu-budgie-desktop"] },
  { id: "i3", label: "i3 (tiling WM)", packages: ["i3", "i3status"] },
  { id: "sway", label: "Sway (Wayland WM)", packages: ["sway"] },
];

const MINT_DESKTOPS: DesktopDef[] = [
  { id: "none", label: "Sin escritorio (servidor)", packages: [] },
  { id: "cinnamon", label: "Cinnamon", packages: ["cinnamon", "mint-meta-cinnamon"] },
  { id: "mate", label: "MATE", packages: ["mint-meta-mate"] },
  { id: "xfce", label: "Xfce", packages: ["mint-meta-xfce"] },
];

const ARCH_DESKTOPS: DesktopDef[] = [
  { id: "none", label: "Sin escritorio (servidor)", packages: [] },
  { id: "kde", label: "KDE Plasma", packages: ["plasma-meta", "sddm"] },
  { id: "gnome", label: "GNOME", packages: ["gnome", "gdm"] },
  { id: "xfce", label: "Xfce", packages: ["xfce4", "xfce4-goodies", "lightdm"] },
  { id: "mate", label: "MATE", packages: ["mate", "lightdm"] },
  { id: "lxqt", label: "LXQt", packages: ["lxqt", "sddm"] },
  { id: "cinnamon", label: "Cinnamon", packages: ["cinnamon", "lightdm"] },
  { id: "budgie", label: "Budgie", packages: ["budgie-desktop", "lightdm"] },
  { id: "i3", label: "i3 (tiling WM)", packages: ["i3", "i3status"] },
  { id: "sway", label: "Sway (Wayland WM)", packages: ["sway"] },
];

const FEDORA_DESKTOPS: DesktopDef[] = [
  { id: "none", label: "Sin escritorio (servidor)", packages: [] },
  { id: "kde", label: "KDE Plasma", packages: ["@^kde-desktop-environment"] },
  { id: "gnome", label: "GNOME", packages: ["@^gnome-desktop-environment"] },
  { id: "xfce", label: "Xfce", packages: ["@^xfce-desktop-environment"] },
  { id: "mate", label: "MATE", packages: ["@mate-desktop-environment"] },
  { id: "lxqt", label: "LXQt", packages: ["@lxqt-desktop-environment"] },
  { id: "cinnamon", label: "Cinnamon", packages: ["@cinnamon-desktop-environment"] },
  { id: "budgie", label: "Budgie", packages: ["@budgie-desktop-environment"] },
  { id: "i3", label: "i3 (tiling WM)", packages: ["@i3-desktop-environment"] },
  { id: "sway", label: "Sway (Wayland WM)", packages: ["@sway-desktop-environment"] },
];

const GENTOO_DESKTOPS: DesktopDef[] = [
  { id: "none", label: "Sin escritorio", packages: [] },
  { id: "kde", label: "KDE Plasma", packages: ["kde-plasma/plasma-meta"] },
  { id: "gnome", label: "GNOME", packages: ["gnome-base/gnome"] },
  { id: "xfce", label: "Xfce", packages: ["xfce-base/xfce4"] },
  { id: "mate", label: "MATE", packages: ["mate-base/mate"] },
  { id: "lxqt", label: "LXQt", packages: ["lxqt-base/lxqt-meta"] },
  { id: "i3", label: "i3 (tiling WM)", packages: ["x11-wm/i3"] },
  { id: "sway", label: "Sway (Wayland WM)", packages: ["gui-wm/sway"] },
];

export const BASES: BaseDef[] = [
  {
    id: "opensuse-tumbleweed",
    label: "openSUSE Tumbleweed",
    engine: "kiwi",
    mirror: "https://download.opensuse.org/tumbleweed/repo/oss/",
    kernelPackage: "kernel-default",
    desktops: SUSE_DESKTOPS,
    suggestions: SUSE_SUGGESTIONS,
  },
  {
    id: "opensuse-leap",
    label: "openSUSE Leap 15.6",
    engine: "kiwi",
    mirror: "https://download.opensuse.org/distribution/leap/15.6/repo/oss/",
    kernelPackage: "kernel-default",
    desktops: SUSE_DESKTOPS,
    suggestions: SUSE_SUGGESTIONS,
  },
  {
    id: "debian",
    label: "Debian 12 (bookworm)",
    engine: "livebuild",
    mirror: "http://deb.debian.org/debian",
    distribution: "bookworm",
    archiveAreas: "main contrib non-free non-free-firmware",
    lbFlavour: "amd64",
    container: "debian:bookworm",
    kernelPackage: "linux-image-amd64",
    desktops: DEBIAN_DESKTOPS,
    suggestions: [...SUGGESTIONS, "firefox-esr"],
  },
  {
    id: "ubuntu",
    label: "Ubuntu 24.04 (noble)",
    engine: "livebuild",
    mirror: "http://archive.ubuntu.com/ubuntu",
    distribution: "noble",
    archiveAreas: "main restricted universe multiverse",
    lbFlavour: "generic",
    container: "ubuntu:24.04",
    kernelPackage: "linux-image-generic",
    desktops: UBUNTU_DESKTOPS,
    suggestions: [...SUGGESTIONS, "ubuntu-restricted-extras"],
  },
  {
    id: "mint",
    label: "Linux Mint 22",
    engine: "livebuild",
    mirror: "http://archive.ubuntu.com/ubuntu",
    distribution: "noble",
    archiveAreas: "main restricted universe multiverse",
    lbFlavour: "generic",
    container: "ubuntu:24.04",
    kernelPackage: "linux-image-generic",
    presetRepos: [
      {
        alias: "linuxmint",
        url: "http://packages.linuxmint.com",
        suite: "wilma main upstream import backport",
        gpgKey: "",
        enabled: true,
      },
    ],
    desktops: MINT_DESKTOPS,
    suggestions: [...SUGGESTIONS, "mintinstall", "mintupdate"],
  },
  {
    id: "elementary",
    label: "elementary OS 8",
    engine: "livebuild",
    mirror: "http://archive.ubuntu.com/ubuntu",
    distribution: "noble",
    archiveAreas: "main restricted universe multiverse",
    lbFlavour: "generic",
    container: "ubuntu:24.04",
    kernelPackage: "linux-image-generic",
    presetRepos: [
      {
        alias: "elementary",
        url: "https://packages.elementary.io/elementary/os/8.0",
        suite: "",
        gpgKey: "",
        enabled: true,
      },
    ],
    desktops: [
      { id: "none", label: "Sin escritorio (servidor)", packages: [] },
      { id: "pantheon", label: "Pantheon", packages: ["elementary-desktop"] },
      { id: "gnome", label: "GNOME", packages: ["ubuntu-desktop"] },
    ],
    suggestions: [...SUGGESTIONS, "pantheon-files", "epiphany-browser"],
  },
  {
    id: "pop",
    label: "Pop!_OS 22.04",
    engine: "livebuild",
    mirror: "http://archive.ubuntu.com/ubuntu",
    distribution: "jammy",
    archiveAreas: "main restricted universe multiverse",
    lbFlavour: "generic",
    container: "ubuntu:22.04",
    kernelPackage: "linux-image-generic",
    presetRepos: [
      {
        alias: "pop-os",
        url: "http://apt.pop-os.org/release",
        suite: "jammy main",
        gpgKey: "",
        enabled: true,
      },
    ],
    desktops: [
      { id: "none", label: "Sin escritorio (servidor)", packages: [] },
      { id: "popgnome", label: "Pop! GNOME", packages: ["pop-desktop"] },
      { id: "cosmic", label: "COSMIC", packages: ["cosmic-desktop"] },
    ],
    suggestions: [...SUGGESTIONS, "pop-shop", "system76-driver"],
  },
  {
    id: "zorin",
    label: "Zorin OS 18",
    engine: "livebuild",
    mirror: "http://archive.ubuntu.com/ubuntu",
    distribution: "noble",
    archiveAreas: "main restricted universe multiverse",
    lbFlavour: "generic",
    container: "ubuntu:24.04",
    kernelPackage: "linux-image-generic",
    presetRepos: [
      {
        alias: "zorin",
        url: "https://packages.zorin.com/os/18",
        suite: "",
        gpgKey: "",
        enabled: true,
      },
    ],
    desktops: [
      { id: "none", label: "Sin escritorio (servidor)", packages: [] },
      { id: "zorincore", label: "Zorin Core (GNOME)", packages: ["zorin-os-desktop"] },
      { id: "gnome", label: "GNOME estándar", packages: ["ubuntu-desktop"] },
    ],
    suggestions: SUGGESTIONS,
  },
  {
    id: "fedora",
    label: "Fedora 41",
    engine: "kickstart",
    mirror: "https://mirrors.fedoraproject.org/mirrorlist?repo=fedora-41&arch=x86_64",
    desktops: FEDORA_DESKTOPS,
    suggestions: SUGGESTIONS,
  },
  {
    id: "arch",
    label: "Arch Linux",
    engine: "mkarchiso",
    mirror: "https://geo.mirror.pkgbuild.com/$repo/os/$arch",
    kernelPackage: "linux",
    desktops: ARCH_DESKTOPS,
    suggestions: SUGGESTIONS,
  },
  {
    id: "manjaro",
    label: "Manjaro",
    engine: "mkarchiso",
    mirror: "https://ftp.halifax.rwth-aachen.de/manjaro/stable/$repo/os/$arch",
    kernelPackage: "linux612",
    desktops: ARCH_DESKTOPS,
    suggestions: SUGGESTIONS,
  },
  {
    id: "cachyos",
    label: "CachyOS",
    engine: "mkarchiso",
    mirror: "https://geo.mirror.pkgbuild.com/$repo/os/$arch",
    kernelPackage: "linux-cachyos",
    presetRepos: [
      {
        alias: "cachyos",
        url: "https://cdn.cachyos.org/repo/x86_64",
        suite: "",
        gpgKey: "",
        enabled: true,
      },
    ],
    desktops: ARCH_DESKTOPS.map((d) =>
      d.id === "none"
        ? { ...d, packages: ["cachyos-settings"] }
        : d.id === "kde"
          ? { ...d, packages: [...d.packages, "cachyos-kde-settings"] }
          : d
    ),
    suggestions: [...SUGGESTIONS, "cachyos-settings", "paru"],
  },
  {
    id: "gentoo",
    label: "Gentoo",
    engine: "manual",
    mirror: "",
    desktops: GENTOO_DESKTOPS,
    suggestions: [...SUGGESTIONS, "sys-kernel/gentoo-kernel", "app-portage/eix", "app-shells/zsh"],
  },
  {
    id: "solus",
    label: "Solus",
    engine: "manual",
    mirror: "",
    desktops: [
      { id: "none", label: "Sin escritorio", packages: [] },
      { id: "budgie", label: "Budgie", packages: ["budgie-desktop"] },
      { id: "gnome", label: "GNOME", packages: ["gnome"] },
      { id: "xfce", label: "Xfce", packages: ["xfce4-session"] },
    ],
    suggestions: SUGGESTIONS,
  },
];

export function baseById(id: BaseId): BaseDef {
  const b = BASES.find((x) => x.id === id);
  if (!b) throw new Error(`Base desconocida: ${id}`);
  return b;
}

export function desktopPackages(baseId: BaseId, desktopId: string): string[] {
  return baseById(baseId).desktops.find((d) => d.id === desktopId)?.packages ?? [];
}

/** Al cambiar de base: quita presets de la base anterior y añade los de la nueva.
 *  Los repos añadidos por el usuario (que no son preset de ninguna base) se conservan. */
export function mergePresets(nextBase: BaseDef, repos: CustomRepo[]): CustomRepo[] {
  const presetAliases = new Set(BASES.flatMap((b) => (b.presetRepos ?? []).map((p) => p.alias)));
  const userRepos = repos.filter((r) => !presetAliases.has(r.alias));
  const presets = (nextBase.presetRepos ?? []).filter(
    (p) => !userRepos.some((r) => r.alias === p.alias)
  );
  return [...presets, ...userRepos];
}

/** Repos efectivos de una config: presets de la base (respetando si el usuario los
 *  desactivó) + repos del usuario. Los generadores usan esto, no cfg.repos a pelo,
 *  para que la receta sea correcta aunque la UI no haya mergeado los presets. */
export function effectiveRepos(cfg: DistroConfig): CustomRepo[] {
  const base = baseById(cfg.base);
  const presets = base.presetRepos ?? [];
  const presetAliases = new Set(presets.map((p) => p.alias));
  const userRepos = cfg.repos.filter(
    (r) => !presetAliases.has(r.alias) && r.enabled && r.url
  );
  const disabledPresets = new Set(
    cfg.repos.filter((r) => presetAliases.has(r.alias) && !r.enabled).map((r) => r.alias)
  );
  return [
    ...presets.filter(
      (p) => !disabledPresets.has(p.alias) && !userRepos.some((r) => r.alias === p.alias)
    ),
    ...userRepos,
  ];
}

export const TIMEZONES = [
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

export const LOCALES = [
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

export const KEYBOARDS = ["es", "latam", "us", "gb", "de", "fr", "it", "pt"];

export function defaultConfig(base: BaseId): DistroConfig {
  const b = baseById(base);
  return {
    name: "mi-distro",
    base,
    desktop: b.desktops[0].id,
    packages: ["git", "vim", "htop"],
    repos: [],
    system: {
      hostname: "mi-distro",
      username: "user",
      password: "",
      timezone: "Europe/Madrid",
      locale: "es_ES.UTF-8",
      keyboard: "es",
    },
    output: "iso",
    cloudInit: false,
    motd: "Bienvenido a tu distro personalizada (DistroForge)",
    wallpaper: "",
  };
}
