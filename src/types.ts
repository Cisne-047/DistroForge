export type EngineId = "kiwi" | "livebuild" | "mkarchiso" | "kickstart" | "manual";
export type ArchId = "x86_64" | "aarch64" | "armhf" | "i386";
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
export type OutputFormat = "iso" | "qcow2" | "usb" | "vmdk" | "vdi" | "oci" | "wsl";

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

/** Usuario adicional al principal (paridad con la pestaña General de SUSE Studio). */
export interface ExtraUser {
  name: string;
  password: string;
  admin: boolean;
}

/** Unidad systemd que se habilita o deshabilita en la imagen (pestaña Startup). */
export interface ServiceToggle {
  unit: string;
  enabled: boolean;
}

/** Configuración de red (pestaña General de SUSE Studio). */
export interface NetworkConfig {
  mode: "dhcp" | "static";
  /** IPv4 (solo en modo estático) */
  ip: string;
  /** prefijo CIDR 8-32, como texto */
  prefix: string;
  gateway: string;
  /** servidores DNS IPv4 */
  dns: string[];
}

/** Opciones del arranque GRUB (pestaña Startup de SUSE Studio). */
export interface GrubConfig {
  /** segundos que el menú espera antes de arrancar la primera entrada */
  timeout: number;
  /** parámetros extra del kernel (appended al cmdline) */
  extraParams: string;
}

/** Modo advanced: remasterizado de una ISO base que trae el usuario. */
export interface CustomIsoConfig {
  /** URL de la ISO base descargable (http/https, termina en .iso) */
  url: string;
  /** script avanzado del usuario, ejecutado dentro del chroot de la ISO */
  chrootScript: string;
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
  /** usuarios adicionales (nombre, contraseña, admin) */
  users: ExtraUser[];
  /** servicios systemd a habilitar/deshabilitar */
  services: ServiceToggle[];
  /** red: DHCP (por defecto) o IP estática */
  network: NetworkConfig;
  /** opciones del arranque GRUB */
  grub: GrubConfig;
  /** firewall (firewalld) habilitado en la imagen */
  firewall: boolean;
  /** arquitectura objetivo */
  arch: ArchId;
  /** modo advanced: remasterizar una ISO custom en vez de construir desde la base */
  advanced: boolean;
  /** datos del modo advanced (URL de la ISO y script chroot) */
  customIso: CustomIsoConfig;
  /** auto-login del usuario principal en el escritorio */
  autologin: boolean;
  /** inyectar cloud-init (NoCloud) para VMs/cloud */
  cloudInit: boolean;
  /** mensaje del día (/etc/motd) */
  motd: string;
  /** wallpaper como data URL (png/jpeg/webp); vacío = sin wallpaper */
  wallpaper: string;
}
