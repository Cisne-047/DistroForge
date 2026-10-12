import type { DistroConfig } from "../types";

const WALLPAPER_RE = /^data:image\/(png|jpeg|webp);base64,/;

export function wallpaperFileName(cfg: DistroConfig): string | null {
  const m = WALLPAPER_RE.exec(cfg.wallpaper ?? "");
  if (!m) return null;
  return `distroforge-wallpaper.${m[1] === "jpeg" ? "jpg" : m[1]}`;
}

export function wallpaperDataUrl(cfg: DistroConfig): string | null {
  return WALLPAPER_RE.test(cfg.wallpaper ?? "") ? cfg.wallpaper : null;
}

/** Esquema dconf del wallpaper según escritorio; null si no usa dconf. */
export function dconfSchemaFor(desktopId: string): string | null {
  switch (desktopId) {
    case "gnome":
    case "zorincore":
    case "popgnome":
    case "budgie":
      return "org/gnome/desktop/background";
    case "mate":
      return "org/mate/desktop/background";
    case "cinnamon":
      return "org/cinnamon/desktop/background";
    default:
      return null;
  }
}

/** El wallpaper se aplica de verdad en GNOME-family (dconf) y Xfce (xfconf). */
export function wallpaperApplies(cfg: DistroConfig): boolean {
  return !!(cfg.wallpaper && (dconfSchemaFor(cfg.desktop) || cfg.desktop === "xfce"));
}

/** Ficheros dconf (perfil + base de datos) con rutas absolutas de destino. */
export function dconfDbFiles(cfg: DistroConfig): Record<string, string> {
  const schema = dconfSchemaFor(cfg.desktop);
  const file = wallpaperFileName(cfg);
  if (!schema || !file) return {};
  return {
    "/etc/dconf/profile/user": "user-db:user\nsystem-db:distroforge\n",
    "/etc/dconf/db/distroforge.d/00-wallpaper":
      `[${schema}]\n` +
      `picture-uri='file:///usr/share/backgrounds/${file}'\n` +
      `picture-uri-dark='file:///usr/share/backgrounds/${file}'\n`,
  };
}

export function xfconfWallpaperXml(cfg: DistroConfig): string {
  const file = wallpaperFileName(cfg)!;
  return `<?xml version="1.0" encoding="UTF-8"?>

<channel name="xfce4-desktop" version="1.0">
  <property name="backdrop" type="empty">
    <property name="screen0" type="empty">
      <property name="monitor0" type="empty">
        <property name="workspace0" type="empty">
          <property name="last-image" type="string" value="/usr/share/backgrounds/${file}"/>
        </property>
      </property>
    </property>
  </property>
</channel>
`;
}

export const LOGO_RE = /^data:image\/(png|jpeg|webp);base64,/;

/** Ficheros del logo Personalize: binario en /usr/share/distroforge y config
 *  del greeter LightDM cuando aplica. Devuelve {} si no hay logo. */
export function logoFiles(cfg: DistroConfig): Record<string, string> {
  if (!LOGO_RE.test(cfg.logo ?? "")) return {};
  const ext = cfg.logo.startsWith("data:image/jpeg;") ? "jpg" : cfg.logo.startsWith("data:image/webp;") ? "webp" : "png";
  const dest = `/usr/share/distroforge/logo.${ext}`;
  const files: Record<string, string> = { [dest]: cfg.logo };
  files["/etc/lightdm/lightdm-gtk-greeter.conf.d/50-distroforge-logo.conf"] =
    `[greeter]
logo=${dest}
`;
  return files;
}

/** Paquetes extra para aplicar el wallpaper según familia de gestor. */
export function wallpaperPackages(
  cfg: DistroConfig,
  family: "apt" | "pacman" | "zypper"
): string[] {
  if (!wallpaperApplies(cfg)) return [];
  if (dconfSchemaFor(cfg.desktop)) {
    if (family === "apt") return ["dconf-cli"];
    return ["dconf"]; // pacman y zypper
  }
  return []; // xfconf ya viene con xfce4
}

/** Servicio + tmpfiles para compilar la base dconf al arrancar (necesario en archiso,
 *  donde no hay chroot hooks en build). */
export function archDconfBootFiles(): Record<string, string> {
  return {
    "/etc/systemd/system/distroforge-dconf.service": `[Unit]
Description=Compila la base de datos dconf de DistroForge
[Service]
Type=oneshot
ExecStart=/usr/bin/dconf update
[Install]
WantedBy=multi-user.target
`,
    "/etc/tmpfiles.d/distroforge-dconf.conf":
      "L /etc/systemd/system/multi-user.target.wants/distroforge-dconf.service - - - - /etc/systemd/system/distroforge-dconf.service\n",
  };
}
