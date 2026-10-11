import type { DistroConfig } from "../types";
import { desktopPackages } from "../catalog";

/** Línea shell para el firewall (firewalld); tolerante si no está instalado. */
export function firewallServiceLine(cfg: DistroConfig): string {
  return cfg.firewall
    ? `systemctl enable "firewalld" || true`
    : `systemctl disable "firewalld" || true`;
}

/** Igual que firewallServiceLine pero como ExecStart de systemd. */
export function firewallExecStart(cfg: DistroConfig): string {
  return cfg.firewall
    ? `ExecStart=/bin/sh -c 'systemctl enable --now "firewalld" || true'`
    : `ExecStart=/bin/sh -c 'systemctl disable --now "firewalld" || true'`;
}

/** Sudo/wheel según familia de gestor de paquetes (lo pasan los generadores). */
export type AdminGroup = "sudo" | "wheel" | "users";

/** Líneas shell para crear los usuarios extra (live-build, Dockerfile, WSL). */
export function extraUserShellLines(
  cfg: DistroConfig,
  adminGroup: AdminGroup
): string[] {
  return (cfg.users ?? []).map((u) => {
    const groups = u.admin ? ` -G ${adminGroup}` : "";
    return `id "${u.name}" >/dev/null 2>&1 || useradd -m -s /bin/bash${groups} "${u.name}"
echo "${u.name}:${u.password.replace(/"/g, '\\"')}" | chpasswd`;
  });
}

/** Líneas shell para systemctl enable/disable de los servicios elegidos. */
export function serviceShellLines(cfg: DistroConfig): string[] {
  return (cfg.services ?? [])
    .filter((s) => /^[A-Za-z0-9@._-]+$/.test(s.unit))
    .map((s) =>
      s.enabled
        ? `systemctl enable "${s.unit}"`
        : `systemctl disable "${s.unit}" || true`
    );
}

/** Líneas systemd ExecStart para crear usuarios extra al arrancar (archiso no
 *  tiene hooks de chroot; tolera re-ejecuciones). */
export function userExecStartLines(
  cfg: DistroConfig,
  adminGroup: AdminGroup
): string[] {
  return (cfg.users ?? []).map(
    (u) =>
      `ExecStart=/bin/sh -c 'id "${u.name}" >/dev/null 2>&1 || useradd -m -s /bin/bash${u.admin ? ` -G ${adminGroup}` : ""} "${u.name}"; echo "${u.name}:${u.password.replace(/"/g, '\\"')}" | chpasswd'`
  );
}

/** Líneas systemd ExecStart para enable --now / disable --now de servicios. */
export function serviceExecStartLines(cfg: DistroConfig): string[] {
  return (cfg.services ?? [])
    .filter((s) => /^[A-Za-z0-9@._-]+$/.test(s.unit))
    .map((s) =>
      s.enabled
        ? `ExecStart=/bin/sh -c 'systemctl enable --now "${s.unit}" || true'`
        : `ExecStart=/bin/sh -c 'systemctl disable --now "${s.unit}" || true'`
    );
}

function dmOf(cfg: DistroConfig): string | null {
  const pkgs = [...desktopPackages(cfg.base, cfg.desktop)];
  if (pkgs.includes("sddm")) return "sddm";
  if (pkgs.includes("gdm3") || pkgs.includes("gdm")) return "gdm";
  if (pkgs.includes("lightdm")) return "lightdm";
  // Fedora (kickstart) usa grupos @^... que no nombran el DM: dedúcimos por escritorio
  if (["gnome", "zorincore", "popgnome"].includes(cfg.desktop)) return "gdm";
  if (cfg.desktop === "kde") return "sddm";
  if (["xfce", "mate", "lxqt", "cinnamon"].includes(cfg.desktop)) return "lightdm";
  return null;
}

/** Ficheros de auto-login por gestor de sesión, con ruta absoluta de destino.
 *  Devuelve {} si no procede (sin checkbox o sin DM conocido). */
export function autologinFiles(cfg: DistroConfig): Record<string, string> {
  if (!cfg.autologin) return {};
  const dm = dmOf(cfg);
  const u = cfg.system.username;
  if (dm === "sddm")
    return {
      "/etc/sddm.conf.d/zz-distroforge-autologin.conf": `[Autologin]\nUser=${u}\n`,
    };
  if (dm === "gdm")
    return {
      "/etc/gdm/custom.conf": `[daemon]\nAutomaticLoginEnable=True\nAutomaticLogin=${u}\n`,
      "/etc/gdm3/custom.conf": `[daemon]\nAutomaticLoginEnable=True\nAutomaticLogin=${u}\n`,
    };
  if (dm === "lightdm")
    return {
      "/etc/lightdm/lightdm.conf": `[Seat:*]\nautologin-user=${u}\nautologin-user-timeout=0\n`,
    };
  return {};
}
