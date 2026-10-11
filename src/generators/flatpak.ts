import type { DistroConfig } from "../types";

export const FLATPAK_PACKAGE = "flatpak";
export const FLATHUB = "https://dl.flathub.org/repo/flathub.flatpakrepo";

export function flatpakApps(cfg: DistroConfig): string[] {
  return Array.isArray(cfg.flatpakApps) ? cfg.flatpakApps : [];
}

export function hasFlatpakApps(cfg: DistroConfig): boolean {
  return flatpakApps(cfg).length > 0;
}

/** El paquete "flatpak" solo entra en la lista cuando hay apps que instalar. */
export function flatpakPackages(cfg: DistroConfig): string[] {
  return hasFlatpakApps(cfg) ? [FLATPAK_PACKAGE] : [];
}

/** Líneas shell: alta de Flathub + instalación de las apps. Para chroot/root directo. */
export function flatpakShellLines(cfg: DistroConfig, prefix = ""): string[] {
  const apps = flatpakApps(cfg);
  if (!apps.length) return [];
  return [
    `${prefix}flatpak remote-add --if-not-exists flathub ${FLATHUB}`,
    `${prefix}flatpak install -y --noninteractive flathub ${apps.join(" ")}`,
  ];
}
