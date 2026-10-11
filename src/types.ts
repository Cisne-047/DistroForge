export type EngineId = "kiwi" | "livebuild" | "mkarchiso" | "kickstart" | "manual";
export type BaseId =
  | "opensuse-tumbleweed"
  | "opensuse-leap"
  | "debian"
  | "ubuntu"
  | "mint"
  | "elementary"
  | "pop"
  | "zorin"
  | "arch"
  | "manjaro"
  | "cachyos"
  | "fedora"
  | "gentoo"
  | "solus";
export type OutputFormat = "iso" | "qcow2" | "oci" | "wsl";

export interface CustomRepo {
  alias: string;
  url: string;
  gpgKey: string;
  /** "suite componentes" para repos no planos (ej. "noble main"). Vacío = repo plano (./) */
  suite: string;
  enabled: boolean;
}

export interface SystemConfig {
  hostname: string;
  username: string;
  password: string;
  timezone: string;
  locale: string;
  keyboard: string;
}

/** Fichero arbitrario que se copia tal cual a la imagen (dotfiles, configs...). */
export interface ExtraFile {
  /** ruta absoluta dentro de la imagen (ej. /etc/skel/.bashrc) */
  path: string;
  content: string;
}

export interface DistroConfig {
  name: string;
  base: BaseId;
  desktop: string;
  packages: string[];
  repos: CustomRepo[];
  system: SystemConfig;
  output: OutputFormat;
  /** apps Flatpak de Flathub (ids tipo org.mozilla.firefox) */
  flatpakApps: string[];
  /** ficheros a copiar a la imagen por ruta absoluta */
  extraFiles: ExtraFile[];
  /** inyectar cloud-init (NoCloud) para VMs/cloud */
  cloudInit: boolean;
  /** mensaje del día (/etc/motd) */
  motd: string;
  /** wallpaper como data URL (png/jpeg/webp); vacío = sin wallpaper */
  wallpaper: string;
}
