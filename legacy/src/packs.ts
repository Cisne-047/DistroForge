import type { DistroConfig } from "./types";
import { baseById } from "./catalog";

/** Pack estilo Prism Launcher: colección lista de paquetes (y Flatpaks) que se
 *  aplica de forma ADITIVA sobre tu config. Compartible como JSON o por URL. */
export interface Pack {
  id: string;
  name: string;
  desc: string;
  packages: string[];
  flatpakApps?: string[];
  /** escritorio sugerido; solo se aplica si la base lo tiene */
  desktop?: string;
}

export const PACKS: Pack[] = [
  {
    id: "gaming",
    name: "Gaming",
    desc: "Steam, Lutris, Wine y GameMode con MangoHud para overlays.",
    packages: ["steam", "lutris", "wine", "mangohud", "gamemode", "steam-native-runtime"],
    flatpakApps: ["com.valvesoftware.Steam", "net.lutris.Lutris"],
  },
  {
    id: "dev",
    name: "Desarrollo",
    desc: "Editor, contenedores y toolchain web/Python listos para trabajar.",
    packages: ["git", "vim", "neovim", "docker", "docker-compose", "nodejs", "npm", "python3-pip", "tmux"],
    flatpakApps: ["com.visualstudio.code"],
  },
  {
    id: "media",
    name: "Edición multimedia",
    desc: "Kdenlive, GIMP, Audacity e Inkscape para foto, vídeo y audio.",
    packages: ["kdenlive", "gimp", "audacity", "inkscape", "ffmpeg", "obs-studio"],
    flatpakApps: ["org.kde.kdenlive", "org.gimp.GIMP"],
  },
  {
    id: "servidor-web",
    name: "Servidor web",
    desc: "nginx + certbot + herramientas de red para servir cosas.",
    packages: ["nginx", "certbot", "curl", "htop", "fail2ban", "rsync"],
  },
  {
    id: "virtualizacion",
    name: "Virtualización",
    desc: "libvirt/QEMU con virt-manager para VMs al estilo hypervisor.",
    packages: ["qemu-kvm", "libvirt", "virt-manager", "bridge-utils"],
    flatpakApps: ["org.virt_manager.virt-manager"],
  },
  {
    id: "retro",
    name: "Retro gaming",
    desc: "Emuladores y frontends retro con soporte de mandos.",
    packages: ["retroarch", "dolphin-emu", "mame", "pcsx2", "steam"],
    flatpakApps: ["org.libretro.RetroArch"],
  },
  {
    id: "streaming",
    name: "Streaming",
    desc: "OBS Studio, V4L2 y utilidades de captura para emitir.",
    packages: ["obs-studio", "v4l2loopback", "ffmpeg"],
    flatpakApps: ["com.obsproject.Studio"],
  },
  {
    id: "privacidad",
    name: "Privacidad",
    desc: "Tor, VPN wireguard y cifrado de discos/portapapeles.",
    packages: ["tor", "wireguard-tools", "keepassxc", "veracrypt"],
    flatpakApps: ["org.torproject.torbrowser-launcher", "org.keepassxc.KeePassXC"],
  },
];

/** Aplica un pack de forma ADITIVA: dedupe contra lo que ya hay. */
export function applyPack(cfg: DistroConfig, pack: Pack): DistroConfig {
  const b = baseById(cfg.base);
  const packages = [...cfg.packages];
  for (const p of pack.packages) if (!packages.includes(p)) packages.push(p);
  const flatpakApps = [...(cfg.flatpakApps ?? [])];
  for (const a of pack.flatpakApps ?? [])
    if (!flatpakApps.includes(a)) flatpakApps.push(a);
  const desktop =
    pack.desktop && b.desktops.some((d) => d.id === pack.desktop) && cfg.desktop === "none"
      ? pack.desktop
      : cfg.desktop;
  return { ...cfg, packages, flatpakApps, desktop };
}

/** Parsea un pack externo (objeto JSON o { pack: {...} }); lanza si es inválido. */
export function parsePack(raw: string): Pack {
  const obj = JSON.parse(raw);
  const p = obj && typeof obj === "object" && "pack" in obj ? obj.pack : obj;
  if (!p || typeof p !== "object" || typeof p.name !== "string" || !Array.isArray(p.packages))
    throw new Error("el pack debe ser JSON con name y packages[]");
  return {
    id: String(p.id ?? p.name.toLowerCase().replace(/[^a-z0-9-]+/g, "-")),
    name: p.name,
    desc: String(p.desc ?? ""),
    packages: p.packages.map(String),
    flatpakApps: Array.isArray(p.flatpakApps) ? p.flatpakApps.map(String) : undefined,
    desktop: typeof p.desktop === "string" ? p.desktop : undefined,
  };
}
