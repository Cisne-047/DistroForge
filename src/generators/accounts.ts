import type { DistroConfig } from "../types";
import { desktopPackages } from "../catalog";
import { dq, systemdSafe } from "./shell";

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
    return `id "${dq(u.name)}" >/dev/null 2>&1 || useradd -m -s /bin/bash${groups} "${dq(u.name)}"
echo "${dq(u.name)}:${dq(u.password)}" | chpasswd`;
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
      `ExecStart=/bin/sh -c 'id "${systemdSafe(u.name)}" >/dev/null 2>&1 || useradd -m -s /bin/bash${u.admin ? ` -G ${adminGroup}` : ""} "${systemdSafe(u.name)}"; echo "${systemdSafe(u.name)}:${systemdSafe(u.password)}" | chpasswd'`
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

/** Target systemd de arranque: respetando la elección del usuario y el escritorio. */
export function defaultTarget(cfg: DistroConfig): "graphical" | "multi-user" {
  if (cfg.bootTarget === "text") return "multi-user";
  const hasDesktop = desktopPackages(cfg.base, cfg.desktop).length > 0;
  if (cfg.bootTarget === "graphical") return "graphical";
  return hasDesktop ? "graphical" : "multi-user";
}

/** Línea systemctl set-default correspondiente. */
export function targetShellLine(cfg: DistroConfig): string {
  return `systemctl set-default ${defaultTarget(cfg)}.target`;
}

/** Líneas firewalld para abrir los puertos/servicios elegidos. */
export function firewallOpenLines(cfg: DistroConfig): string[] {
  return (cfg.firewallPorts ?? [])
    .filter((x) => /^[A-Za-z0-9._-]+(\/\d+(\/(tcp|udp))?)?$/.test(x))
    .map((x) =>
      x.includes("/")
        ? `firewall-cmd --permanent --add-port=${x} || true`
        : `firewall-cmd --permanent --add-service=${x} || true`
    );
}

/** Entradas .desktop de autostart para el usuario principal: ruta → contenido. */
export function autostartFiles(cfg: DistroConfig): Record<string, string> {
  const u = cfg.system.username;
  const out: Record<string, string> = {};
  (cfg.autostart ?? []).forEach((prog, i) => {
    const name = prog.replace(/["'\/\$`]/g, "").trim();
    if (!name) return;
    out[`/home/${u}/.config/autostart/10-distroforge-${i}-${name.replace(/[^a-zA-Z0-9._-]/g, "_")}.desktop`] =
      `[Desktop Entry]
Type=Application
Name=${name}
Exec=${name}
X-GNOME-Autostart-enabled=true
`;
  });
  return out;
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
