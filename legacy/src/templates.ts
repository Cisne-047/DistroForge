import type { DistroConfig } from "./types";
import { baseById } from "./catalog";

/** Plantilla = parche de config "describe lo que quieres" aplicable a cualquier base.
 *  Los ids de escritorio se aplican solo si la base los tiene (ver applyTemplate). */
export interface Template {
  id: string;
  label: string;
  desc: string;
  patch: Partial<Omit<DistroConfig, "system">> & { system?: Partial<DistroConfig["system"]> };
}

export const TEMPLATES: Template[] = [
  {
    id: "server",
    label: "Servidor mínimo",
    desc: "SSH, htop, tmux; sin escritorio; salida OCI para contenedor o VM.",
    patch: {
      packages: ["openssh-server", "htop", "vim", "tmux", "git"],
      flatpakApps: [],
      output: "oci",
      cloudInit: true,
      firewall: true,
      autologin: false,
    },
  },
  {
    id: "kiosk",
    label: "Kiosko",
    desc: "Firefox a pantalla completa con auto-login; GNOME si la base lo tiene.",
    patch: {
      packages: ["firefox", "xdotool"],
      desktop: "gnome",
      autologin: true,
      grub: { timeout: 1, extraParams: "quiet splash" },
    },
  },
  {
    id: "office",
    label: "Oficina",
    desc: "LibreOffice, Thunderbird y Firefox con auto-login.",
    patch: {
      packages: ["libreoffice", "thunderbird", "firefox"],
      desktop: "gnome",
      autologin: true,
    },
  },
  {
    id: "wsldev",
    label: "Entorno WSL dev",
    desc: "Sin escritorio, herramientas de consola; salida WSL para wsl --import.",
    patch: {
      packages: ["git", "vim", "curl", "htop", "build-essential"],
      desktop: "none",
      output: "wsl",
      flatpakApps: [],
    },
  },
  {
    id: "media",
    label: "Multimedia",
    desc: "VLC y GIMP nativos + Flatpaks de Flathub.",
    patch: {
      packages: ["vlc", "gimp", "audacity"],
      flatpakApps: ["org.videolan.VLC", "org.gimp.GIMP"],
    },
  },
];

/** Aplica una plantilla sobre la config actual respetando la base elegida:
 *  los ids de escritorio que la base no tiene se conservan tal cual. */
export function applyTemplate(cfg: DistroConfig, tpl: Template): DistroConfig {
  const b = baseById(cfg.base);
  const { desktop, system: patchSystem, grub: patchGrub, ...rest } = tpl.patch;
  if (desktop && !b.desktops.some((d) => d.id === desktop)) return { ...cfg, ...rest, grub: { ...cfg.grub, ...(patchGrub ?? {}) }, system: { ...cfg.system, ...(patchSystem ?? {}) } } as DistroConfig;
  return {
    ...cfg,
    ...rest,
    ...(desktop ? { desktop } : {}),
    grub: { ...cfg.grub, ...(patchGrub ?? {}) },
    system: { ...cfg.system, ...(patchSystem ?? {}) },
  };
}
