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

export interface DistroConfig {
  name: string;
  base: BaseId;
  desktop: string;
  packages: string[];
  repos: CustomRepo[];
  system: SystemConfig;
  output: OutputFormat;
  /** inyectar cloud-init (NoCloud) para VMs/cloud */
  cloudInit: boolean;
  /** mensaje del día (/etc/motd) */
  motd: string;
  /** wallpaper como data URL (png/jpeg/webp); vacío = sin wallpaper */
  wallpaper: string;
}
