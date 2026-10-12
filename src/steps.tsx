import { useEffect, useMemo, useRef, useState } from "react";
import type { CustomRepo, DistroConfig, ExtraFile, ExtraUser, OutputFormat, ServiceToggle, SystemConfig } from "./types";
import { BASES, baseById, archsForEngine, mergeConfig, mergePresets, TIMEZONES, LOCALES, KEYBOARDS } from "./catalog";
import { TEMPLATES, applyTemplate } from "./templates";
import { PACKS, applyPack, parsePack, type Pack } from "./packs";
import { api, getServerUrl, setServerUrl, type ServerPack } from "./serverClient";
import { estimateSizeMB } from "./generators";
import { shareUrl } from "./share";
import { generateProject, validate, downloadZip } from "./generators";

type Upd = (patch: Partial<DistroConfig>) => void;

type ChromeKind = "gnome" | "panel" | "tiling" | "terminal";

function chromeOf(desktopId: string): ChromeKind {
  if (["gnome", "zorincore", "popgnome", "pantheon", "cosmic", "budgie"].includes(desktopId))
    return "gnome";
  if (["kde", "xfce", "mate", "lxqt", "cinnamon"].includes(desktopId)) return "panel";
  if (["i3", "sway"].includes(desktopId)) return "tiling";
  return "terminal"; // "none" y desconocidos
}

/** Mock del escritorio recién instalado, según familia: top bar + dock (GNOME),
 *  taskbar inferior (KDE/Xfce/...), barra de workspaces (i3/Sway) o TTY (servidor). */
function DesktopPreview({
  desktopId,
  hostname,
  username,
  motd,
  wallpaper,
  label,
}: {
  desktopId: string;
  hostname: string;
  username: string;
  motd?: string;
  wallpaper?: string;
  label: string;
}) {
  const kind = chromeOf(desktopId);
  const host = hostname || "mi-distro";
  const user = username || "user";
  const bg = wallpaper ? { backgroundImage: `url(${wallpaper})`, backgroundSize: "cover" } : undefined;
  if (kind === "terminal")
    return (
      <div className="desk-mock dp-term" style={bg}>
        <div className="dp-term-body">
          <div>{host} login: {user}</div>
          <div>Password:</div>
          <div className="dp-dim">Last login: {new Date().toDateString()} on tty1</div>
          {motd && <div className="dp-motd">{motd}</div>}
          <div>{user}@{host}:~$ _</div>
        </div>
        <div className="desk-note">{label}</div>
      </div>
    );
  return (
    <div className="desk-mock" style={bg}>
      {kind === "gnome" && (
        <>
          <div className="dp-topbar">
            <span>Actividades</span>
            <span className="dp-clock">{new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</span>
            <span>⌗ ▤ ⏻</span>
          </div>
          <div className="dp-dock">
            {["🌐", "📁", "✉", "⚙", "📋"].map((i, n) => (
              <span key={n} className="dp-icon">{i}</span>
            ))}
          </div>
        </>
      )}
      {kind === "panel" && (
        <div className="dp-panel">
          <span className="dp-start">◆</span>
          <span className="dp-task">🌐 {label}</span>
          <span className="dp-task dp-dim">terminal</span>
          <span className="dp-clock">🔊 🔔 {new Date().toLocaleTimeString([], { hour: "2-digit", minute: "2-digit" })}</span>
        </div>
      )}
      {kind === "tiling" && (
        <div className="dp-tile">
          <div className="dp-win">1: {label}</div>
          <div className="dp-win">2: terminal</div>
        </div>
      )}
      {kind === "tiling" && <div className="dp-wsbar">1 2 3 4 5 · {host}</div>}
      {kind !== "tiling" && (
        <div className="desk-login">
          <div className="desk-host">{host}</div>
          <div className="desk-user">{user}@{host}:~$</div>
          {motd && <div className="desk-motd">{motd}</div>}
        </div>
      )}
      <div className="desk-note">{label}</div>
    </div>
  );
}

export function BaseStep({ cfg, upd }: { cfg: DistroConfig; upd: Upd }) {
  const b = baseById(cfg.base);
  const archs = archsForEngine(b.engine);
  const desk = b.desktops.find((d) => d.id === cfg.desktop)?.label ?? "—";
  return (
    <section>
      <h2>1. Base</h2>
      <p className="hint">
        Distribución sobre la que montas tu distro. El motor de construcción se elige solo:
        KIWI (openSUSE), live-build (Debian), mkarchiso (Arch) o kickstart (Fedora).
      </p>
      <h3>Plantillas rápidas</h3>
      <div className="chips">
        {TEMPLATES.map((t) => (
          <button key={t.id} className="chip" title={t.desc} onClick={() => upd(applyTemplate(cfg, t))}>
            ✨ {t.label}
          </button>
        ))}
      </div>
      <div className="cards">
        {BASES.map((x) => (
          <button
            key={x.id}
            className={"card clickable" + (x.id === cfg.base ? " sel" : "")}
            onClick={() =>
              upd({
                base: x.id,
                desktop: x.desktops[0].id,
                repos: mergePresets(x, cfg.repos),
                arch: archsForEngine(x.engine).includes(cfg.arch) ? cfg.arch : "x86_64",
                output:
                  x.engine === "kiwi"
                    ? cfg.output
                    : ["qcow2", "usb", "vmdk", "vdi"].includes(cfg.output)
                      ? "iso"
                      : cfg.output,
              })
            }
          >
            <strong>{x.label}</strong>
            <span className="tag">motor: {x.engine}</span>
          </button>
        ))}
      </div>
      <div className="preview" style={{ margin: "0 0 16px" }}>
        <DesktopPreview
          desktopId={cfg.desktop}
          hostname={cfg.system.hostname}
          username={cfg.system.username}
          wallpaper={cfg.wallpaper || undefined}
          label={`${b.label} · ${desk} · ${cfg.arch}`}
        />
        <span className="hint">
          Así arrancará una instalación limpia con esta base y este escritorio
          (salida {cfg.output.toUpperCase()}).
        </span>
      </div>
      <h3>Escritorio</h3>
      <div className="cards">
        {b.desktops.map((d) => (
          <button
            key={d.id}
            className={"card clickable" + (d.id === cfg.desktop ? " sel" : "")}
            onClick={() => upd({ desktop: d.id })}
          >
            <strong>{d.label}</strong>
          </button>
        ))}
      </div>
      <h3>Arquitectura</h3>
      <div className="cards">
        {archs.map((a) => (
          <button
            key={a}
            className={"card clickable" + (a === cfg.arch ? " sel" : "")}
            onClick={() => upd({ arch: a })}
          >
            <strong>{a}</strong>
          </button>
        ))}
      </div>
      <h3>Modo advanced (experimental)</h3>
      <p className="hint">
        Remasteriza una ISO que tú aportas (cualquier distro exótica): se extrae, se aplica tu
        config en un chroot y se reconstruye. Salida ISO única.
      </p>
      <label className="chk">
        <input
          type="checkbox"
          checked={cfg.advanced}
          onChange={(e) => upd({ advanced: e.target.checked })}
        />
        Usar una ISO custom como base
      </label>
      {cfg.advanced && (
        <div className="fields">
          <label className="field" style={{ gridColumn: "1 / -1" }}>
            URL de la ISO base (descargable, termina en .iso)
            <input
              value={cfg.customIso?.url ?? ""}
              placeholder="https://ejemplo.com/distro-amd64.iso"
              onChange={(e) =>
                upd({ customIso: { ...(cfg.customIso ?? { url: "", chrootScript: "" }), url: e.target.value } })
              }
            />
          </label>
          <label className="field" style={{ gridColumn: "1 / -1" }}>
            Script chroot avanzado (se añade al generado; control total)
            <textarea
              rows={5}
              value={cfg.customIso?.chrootScript ?? ""}
              placeholder="#!/bin/bash — comandos libres dentro del chroot"
              onChange={(e) =>
                upd({
                  customIso: { ...(cfg.customIso ?? { url: "", chrootScript: "" }), chrootScript: e.target.value },
                })
              }
            />
          </label>
        </div>
      )}
    </section>
  );
}

export function SystemStep({ cfg, upd }: { cfg: DistroConfig; upd: Upd }) {
  const s = cfg.system;
  const setS = (patch: Partial<SystemConfig>) => upd({ system: { ...s, ...patch } });
  const n = cfg.network ?? { mode: "dhcp" as const, ip: "", prefix: "24", gateway: "", dns: [] };
  const setNet = (patch: Partial<typeof n>) => upd({ network: { ...n, ...patch } });
  const extras = cfg.extraFiles ?? [];
  const setExtra = (i: number, patch: Partial<ExtraFile>) =>
    upd({ extraFiles: extras.map((f, j) => (j === i ? { ...f, ...patch } : f)) });
  const addExtra = () => upd({ extraFiles: [...extras, { path: "", content: "" }] });
  const delExtra = (i: number) => upd({ extraFiles: extras.filter((_, j) => j !== i) });
  return (
    <section>
      <h2>2. Sistema</h2>
      <p className="hint">Identidad, usuario e idioma de tu distro.</p>
      <div className="fields">
        <label className="field">
          Hostname
          <input value={s.hostname} onChange={(e) => setS({ hostname: e.target.value })} />
        </label>
        <label className="field">
          Usuario
          <input value={s.username} onChange={(e) => setS({ username: e.target.value })} />
        </label>
        <label className="field">
          Contraseña (va en claro a la receta de build)
          <input
            type="password"
            value={s.password}
            onChange={(e) => setS({ password: e.target.value })}
          />
        </label>
        <label className="field">
          Zona horaria
          <select value={s.timezone} onChange={(e) => setS({ timezone: e.target.value })}>
            {TIMEZONES.map((t) => (
              <option key={t}>{t}</option>
            ))}
          </select>
        </label>
        <label className="field">
          Locale
          <select value={s.locale} onChange={(e) => setS({ locale: e.target.value })}>
            {LOCALES.map((t) => (
              <option key={t}>{t}</option>
            ))}
          </select>
        </label>
        <label className="field">
          Teclado
          <select value={s.keyboard} onChange={(e) => setS({ keyboard: e.target.value })}>
            {KEYBOARDS.map((t) => (
              <option key={t}>{t}</option>
            ))}
          </select>
        </label>
        <label className="field">
          Red
          <select
            value={n.mode}
            onChange={(e) => setNet({ mode: e.target.value as "dhcp" | "static" })}
          >
            <option value="dhcp">DHCP (automática)</option>
            <option value="static">IP estática</option>
          </select>
        </label>
        {n.mode === "static" && (
          <>
            <label className="field">
              IP
              <input
                value={n.ip}
                placeholder="192.168.1.50"
                onChange={(e) => setNet({ ip: e.target.value })}
              />
            </label>
            <label className="field">
              Prefijo CIDR
              <input
                value={n.prefix}
                placeholder="24"
                onChange={(e) => setNet({ prefix: e.target.value })}
              />
            </label>
            <label className="field">
              Gateway
              <input
                value={n.gateway}
                placeholder="192.168.1.1"
                onChange={(e) => setNet({ gateway: e.target.value })}
              />
            </label>
            <label className="field" style={{ gridColumn: "1 / -1" }}>
              DNS (separados por comas; no aplica en la ISO de Arch)
              <input
                value={(n.dns ?? []).join(", ")}
                placeholder="1.1.1.1, 8.8.8.8"
                onChange={(e) =>
                  setNet({ dns: e.target.value.split(",").map((x) => x.trim()).filter(Boolean) })
                }
              />
            </label>
          </>
        )}
        <label className="chk" style={{ gridColumn: "1 / -1" }}>
          <input
            type="checkbox"
            checked={cfg.firewall}
            onChange={(e) => upd({ firewall: e.target.checked })}
          />
          Firewall habilitado (firewalld; en Fedora lo gestiona el kickstart)
        </label>
        <label className="field" style={{ gridColumn: "1 / -1" }}>
          Puertos/servicios abiertos en el firewall (separados por comas: ssh, http, 8080/tcp)
          <input
            value={(cfg.firewallPorts ?? []).join(", ")}
            placeholder="ssh, 8080/tcp"
            onChange={(e) =>
              upd({
                firewallPorts: e.target.value
                  .split(",")
                  .map((x) => x.trim())
                  .filter(Boolean),
              })
            }
          />
        </label>
        <label className="field">
          Target de arranque
          <select
            value={cfg.bootTarget ?? "auto"}
            onChange={(e) =>
              upd({ bootTarget: e.target.value as "auto" | "graphical" | "text" })
            }
          >
            <option value="auto">Auto (según escritorio)</option>
            <option value="graphical">Gráfico</option>
            <option value="text">Texto (multi-user)</option>
          </select>
        </label>
        <label className="field">
          Autostart (programas al iniciar sesión, separados por comas)
          <input
            value={(cfg.autostart ?? []).join(", ")}
            placeholder="firefox, konsole"
            onChange={(e) =>
              upd({
                autostart: e.target.value
                  .split(",")
                  .map((x) => x.trim())
                  .filter(Boolean),
              })
            }
          />
        </label>
        <label className="field">
          Logo (se copia a /usr/share/distroforge y lo usa el greeter LightDM)
          <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
            {cfg.logo ? (
              <>
                <img src={cfg.logo} alt="logo actual" className="wall-prev" />
                <button className="btn ghost" onClick={() => upd({ logo: "" })}>
                  Quitar
                </button>
              </>
            ) : (
              <input
                type="file"
                accept="image/png,image/jpeg,image/webp"
                onChange={(e) => {
                  const f = e.target.files?.[0];
                  e.target.value = "";
                  if (!f) return;
                  if (f.size > 1024 * 1024) {
                    alert("Logo demasiado grande (máximo 1 MB).");
                    return;
                  }
                  const reader = new FileReader();
                  reader.onload = () => upd({ logo: String(reader.result) });
                  reader.readAsDataURL(f);
                }}
              />
            )}
          </div>
        </label>
        <label className="field" style={{ gridColumn: "1 / -1" }}>
          EULA (texto legal mostrado una vez en el primer login interactivo)
          <textarea
            rows={3}
            value={cfg.eulaText}
            placeholder="Al usar esta distro aceptas que..."
            onChange={(e) => upd({ eulaText: e.target.value })}
          />
        </label>
        <label className="chk" style={{ gridColumn: "1 / -1" }}>
          <input
            type="checkbox"
            checked={cfg.unattended}
            onChange={(e) => upd({ unattended: e.target.checked })}
          />
          Instalación desatendida con preseed (solo familia Debian; la ISO instala sin preguntar)
        </label>
        <label className="chk" style={{ gridColumn: "1 / -1" }}>
          <input
            type="checkbox"
            checked={cfg.smokeTest}
            onChange={(e) => upd({ smokeTest: e.target.checked })}
          />
          Smoke test de arranque en CI (smoke.yml: QEMU headless sobre la ISO)
        </label>
        <label className="chk" style={{ gridColumn: "1 / -1" }}>
          <input
            type="checkbox"
            checked={cfg.signImage}
            onChange={(e) => upd({ signImage: e.target.checked })}
          />
          Firmar la imagen OCI con cosign keyless (OIDC de GitHub)
        </label>
        <label className="chk" style={{ gridColumn: "1 / -1" }}>
          <input
            type="checkbox"
            checked={cfg.testDrive}
            onChange={(e) => upd({ testDrive: e.target.checked })}
          />
          Test Drive en CI (testdrive.yml: capturas del arranque en QEMU, solo ISO)
        </label>
        <label className="field" style={{ maxWidth: 320 }}>
          Subir a la nube tras el build (workflow aparte, requiere secrets en el repo)
          <select
            value={cfg.cloudUpload ?? "none"}
            onChange={(e) =>
              upd({ cloudUpload: e.target.value as "none" | "aws" | "azure" })
            }
          >
            <option value="none">No subir</option>
            <option value="aws">AWS (S3 + AMI)</option>
            <option value="azure">Azure (VHD en blob)</option>
          </select>
        </label>
        <label className="chk" style={{ gridColumn: "1 / -1" }}>
          <input
            type="checkbox"
            checked={cfg.autologin}
            onChange={(e) => upd({ autologin: e.target.checked })}
          />
          Auto-login del usuario principal (SDDM/GDM/LightDM)
        </label>
        <div className="field" style={{ gridColumn: "1 / -1" }}>
          Usuarios extra
          {(cfg.users ?? []).map((u, i) => {
            const setU = (patch: Partial<ExtraUser>) =>
              upd({ users: (cfg.users ?? []).map((x, j) => (j === i ? { ...x, ...patch } : x)) });
            return (
              <div key={i} style={{ display: "flex", gap: 8, margin: "6px 0" }}>
                <input
                  value={u.name}
                  placeholder="usuario"
                  style={{ flex: 1 }}
                  onChange={(e) => setU({ name: e.target.value })}
                />
                <input
                  type="password"
                  value={u.password}
                  placeholder="contraseña"
                  style={{ flex: 1 }}
                  onChange={(e) => setU({ password: e.target.value })}
                />
                <label className="chk">
                  <input
                    type="checkbox"
                    checked={u.admin}
                    onChange={(e) => setU({ admin: e.target.checked })}
                  />
                  admin
                </label>
                <button
                  className="icon"
                  title="Eliminar"
                  onClick={() =>
                    upd({ users: (cfg.users ?? []).filter((_, j) => j !== i) })
                  }
                >
                  ✕
                </button>
              </div>
            );
          })}
          <button
            className="btn ghost"
            onClick={() =>
              upd({ users: [...(cfg.users ?? []), { name: "", password: "", admin: false }] })
            }
          >
            + Añadir usuario
          </button>
        </div>
        <div className="field" style={{ gridColumn: "1 / -1" }}>
          Servicios systemd (habilitar/deshabilitar en la imagen)
          {(cfg.services ?? []).map((s, i) => {
            const setSvc = (patch: Partial<ServiceToggle>) =>
              upd({
                services: (cfg.services ?? []).map((x, j) => (j === i ? { ...x, ...patch } : x)),
              });
            return (
              <div key={i} style={{ display: "flex", gap: 8, margin: "6px 0" }}>
                <input
                  value={s.unit}
                  placeholder="ej. sshd.service"
                  style={{ flex: 1 }}
                  onChange={(e) => setSvc({ unit: e.target.value })}
                />
                <label className="chk">
                  <input
                    type="checkbox"
                    checked={s.enabled}
                    onChange={(e) => setSvc({ enabled: e.target.checked })}
                  />
                  activo
                </label>
                <button
                  className="icon"
                  title="Eliminar"
                  onClick={() =>
                    upd({ services: (cfg.services ?? []).filter((_, j) => j !== i) })
                  }
                >
                  ✕
                </button>
              </div>
            );
          })}
          <button
            className="btn ghost"
            onClick={() => upd({ services: [...(cfg.services ?? []), { unit: "", enabled: true }] })}
          >
            + Añadir servicio
          </button>
        </div>
        <div className="field" style={{ gridColumn: "1 / -1" }}>
          Wallpaper (se aplica en GNOME/MATE/Cinnamon/Budgie/Xfce; en KDE/i3/Sway solo se copia)
          <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
            {cfg.wallpaper ? (
              <>
                <img src={cfg.wallpaper} alt="wallpaper actual" className="wall-prev" />
                <button className="btn ghost" onClick={() => upd({ wallpaper: "" })}>
                  Quitar
                </button>
              </>
            ) : (
              <input
                type="file"
                accept="image/png,image/jpeg,image/webp"
                onChange={(e) => {
                  const f = e.target.files?.[0];
                  e.target.value = "";
                  if (!f) return;
                  if (f.size > 2 * 1024 * 1024) {
                    alert("Imagen demasiado grande (máximo 2 MB).");
                    return;
                  }
                  const reader = new FileReader();
                  reader.onload = () => upd({ wallpaper: String(reader.result) });
                  reader.readAsDataURL(f);
                }}
              />
            )}
          </div>
        </div>
        <div className="field" style={{ gridColumn: "1 / -1" }}>
          Ficheros extra (dotfiles, configs — ruta absoluta dentro de la imagen)
          {extras.map((f, i) => (
            <div key={i} style={{ display: "grid", gap: 6, margin: "8px 0" }}>
              <div style={{ display: "flex", gap: 8 }}>
                <input
                  value={f.path}
                  style={{ flex: 1 }}
                  placeholder="/etc/skel/.bashrc"
                  onChange={(e) => setExtra(i, { path: e.target.value })}
                />
                <button className="icon" title="Eliminar" onClick={() => delExtra(i)}>
                  ✕
                </button>
              </div>
              <textarea
                rows={3}
                value={f.content}
                placeholder="contenido del fichero"
                onChange={(e) => setExtra(i, { content: e.target.value })}
              />
            </div>
          ))}
          <button className="btn ghost" onClick={addExtra}>
            + Añadir fichero
          </button>
        </div>
      </div>
    </section>
  );
}

export function BrandingStep({ cfg, upd }: { cfg: DistroConfig; upd: Upd }) {
  const timeout = cfg.grub?.timeout ?? 10;
  const params = cfg.grub?.extraParams ?? "";
  const b = baseById(cfg.base);
  return (
    <section>
      <h2>3. Branding</h2>
      <p className="hint">
        Cómo luce y arranca tu distro: mensaje de bienvenida, fondo de pantalla y menú GRUB.
        La vista previa refleja la config al instante.
      </p>
      <div className="branding-grid">
        <div>
          <div className="fields">
            <label className="field" style={{ gridColumn: "1 / -1" }}>
              Mensaje MOTD (/etc/motd)
              <textarea
                rows={3}
                value={cfg.motd}
                onChange={(e) => upd({ motd: e.target.value })}
              />
            </label>
            <div className="field" style={{ gridColumn: "1 / -1" }}>
              Fondo de pantalla (dconf/xfconf en GNOME/MATE/Cinnamon/Budgie/Xfce; solo copia en
              KDE/i3/Sway)
              <div style={{ display: "flex", gap: 10, alignItems: "center" }}>
                {cfg.wallpaper ? (
                  <>
                    <img src={cfg.wallpaper} alt="wallpaper actual" className="wall-prev" />
                    <button className="btn ghost" onClick={() => upd({ wallpaper: "" })}>
                      Quitar
                    </button>
                  </>
                ) : (
                  <input
                    type="file"
                    accept="image/png,image/jpeg,image/webp"
                    onChange={(e) => {
                      const f = e.target.files?.[0];
                      e.target.value = "";
                      if (!f) return;
                      if (f.size > 2 * 1024 * 1024) {
                        alert("Imagen demasiado grande (máximo 2 MB).");
                        return;
                      }
                      const reader = new FileReader();
                      reader.onload = () => upd({ wallpaper: String(reader.result) });
                      reader.readAsDataURL(f);
                    }}
                  />
                )}
              </div>
            </div>
            <label className="field">
              GRUB: timeout (segundos)
              <input
                type="number"
                min={0}
                max={999}
                value={timeout}
                onChange={(e) => upd({ grub: { ...cfg.grub, timeout: Number(e.target.value) } })}
              />
            </label>
            <label className="field">
              GRUB: parámetros del kernel
              <input
                value={params}
                placeholder="ej. quiet splash"
                onChange={(e) => upd({ grub: { ...cfg.grub, extraParams: e.target.value } })}
              />
            </label>
            <label className="chk" style={{ gridColumn: "1 / -1" }}>
              <input
                type="checkbox"
                checked={cfg.autologin}
                onChange={(e) => upd({ autologin: e.target.checked })}
              />
              Auto-login del usuario principal (SDDM/GDM/LightDM)
            </label>
          </div>
        </div>
        <div className="preview">
          <div className="grub-mock">
            <div className="grub-title">GNU GRUB&nbsp;&nbsp;version 2.12</div>
            <div className="grub-entry grub-sel">* {cfg.name || "mi-distro"} (linux)</div>
            <div className="grub-entry">Memory test (memtest86+)</div>
            <div className="grub-entry">System setup</div>
            <div className="grub-foot">
              Use ↑ y ↓ para seleccionar… timeout: {Math.max(0, timeout)}s
            </div>
          </div>
          <DesktopPreview
            desktopId={cfg.desktop}
            hostname={cfg.system.hostname}
            username={cfg.system.username}
            motd={cfg.motd}
            wallpaper={cfg.wallpaper || undefined}
            label={b.label}
          />
        </div>
      </div>
    </section>
  );
}

export function ReposStep({ cfg, upd }: { cfg: DistroConfig; upd: Upd }) {
  const setRepo = (i: number, patch: Partial<CustomRepo>) =>
    upd({ repos: cfg.repos.map((r, j) => (j === i ? { ...r, ...patch } : r)) });
  const add = () =>
    upd({
      repos: [
        ...cfg.repos,
        { alias: "", url: "", gpgKey: "", suite: "", enabled: true },
      ],
    });
  const del = (i: number) => upd({ repos: cfg.repos.filter((_, j) => j !== i) });
  return (
    <section>
      <h2>3. Repositorios custom</h2>
      <p className="hint">
        URLs extra: OBS, COPR, tu propio mirror... La URL va tal cual al gestor de paquetes
        del motor (zypper/apt/pacman/dnf). Si el repo necesita clave GPG, pon la URL de la key.
      </p>
      {cfg.repos.map((r, i) => (
        <div className="repo-row" key={i}>
          <input
            value={r.alias}
            onChange={(e) => setRepo(i, { alias: e.target.value })}
            placeholder="alias"
          />
          <input
            value={r.url}
            onChange={(e) => setRepo(i, { url: e.target.value })}
            placeholder="https://... url del repo"
          />
          <input
            value={r.gpgKey}
            onChange={(e) => setRepo(i, { gpgKey: e.target.value })}
            placeholder="URL key GPG (opcional)"
          />
          <input
            value={r.suite}
            onChange={(e) => setRepo(i, { suite: e.target.value })}
            placeholder="suite comps (ej. noble main)"
          />
          <label className="chk">
            <input
              type="checkbox"
              checked={r.enabled}
              onChange={(e) => setRepo(i, { enabled: e.target.checked })}
            />
            activo
          </label>
          <button className="icon" title="Eliminar" onClick={() => del(i)}>
            ✕
          </button>
        </div>
      ))}
      <button className="btn ghost" onClick={add}>
        + Añadir repo
      </button>
    </section>
  );
}

export function PackagesStep({ cfg, upd }: { cfg: DistroConfig; upd: Upd }) {
  const [q, setQ] = useState("");
  const [draft, setDraft] = useState("");
  const [fdraft, setFdraft] = useState("");
  const [packDraft, setPackDraft] = useState("");
  const [packApplied, setPackApplied] = useState<string | null>(null);
  const [gallery, setGallery] = useState<{ file: string; name: string; desc: string }[]>([]);
  const [galleryQuery, setGalleryQuery] = useState("");
  const [srvUrl, setSrvUrl] = useState(getServerUrl());
  const [me, setMe] = useState<string | null>(null);
  const [cred, setCred] = useState({ name: "", password: "" });
  const [serverPacks, setServerPacks] = useState<ServerPack[] | null>(null);
  const [serverQuery, setServerQuery] = useState("");

  const refreshMe = () => api.me().then((r) => setMe(r.user?.name ?? null)).catch(() => setMe(null));
  useEffect(() => {
    if (getServerUrl()) refreshMe();
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);
  const b = baseById(cfg.base);
  const flatpak = cfg.flatpakApps ?? [];
  const sugg = b.suggestions.filter(
    (s) => s.toLowerCase().includes(q.toLowerCase()) && !cfg.packages.includes(s)
  );
  const toggle = (p: string) =>
    upd({
      packages: cfg.packages.includes(p)
        ? cfg.packages.filter((x) => x !== p)
        : [...cfg.packages, p],
    });
  const add = () => {
    const p = draft.trim();
    if (p && !cfg.packages.includes(p)) upd({ packages: [...cfg.packages, p] });
    setDraft("");
  };
  const addFlatpak = () => {
    const a = fdraft.trim();
    if (a && !flatpak.includes(a)) upd({ flatpakApps: [...flatpak, a] });
    setFdraft("");
  };
  const applyPackDraft = async () => {
    const src = packDraft.trim();
    if (!src) return;
    try {
      let pk: Pack;
      if (/^https?:\/\//.test(src)) {
        const res = await fetch(src);
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        pk = parsePack(await res.text());
      } else {
        pk = parsePack(src);
      }
      upd(applyPack(cfg, pk));
      setPackApplied(pk.name);
      setPackDraft("");
      setTimeout(() => setPackApplied(null), 3000);
    } catch (e) {
      alert(`Pack inválido: ${e instanceof Error ? e.message : e}`);
    }
  };
  return (
    <section>
      <h2>4. Paquetes</h2>
      <p className="hint">
        Los nombres deben existir en los repos de {b.label}. Al cambiar de base, revisa que
        sigan siendo válidos (cada distro nombra los paquetes distinto).
      </p>
      <div className="fields">
        <label className="field">
          Añadir paquete (Enter)
          <input
            value={draft}
            onChange={(e) => setDraft(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && add()}
            placeholder="ej. neofetch"
          />
        </label>
        <label className="field">
          Filtrar sugeridos
          <input value={q} onChange={(e) => setQ(e.target.value)} placeholder="buscar..." />
        </label>
      </div>
      <h3>Tus paquetes ({cfg.packages.length})</h3>
      <div className="chips">
        {cfg.packages.map((p) => (
          <button key={p} className="chip sel" onClick={() => toggle(p)}>
            {p}
            <span className="x">×</span>
          </button>
        ))}
        {cfg.packages.length === 0 && <span className="hint">Ninguno todavía.</span>}
      </div>
      <h3>Sugeridos</h3>
      <div className="chips">
        {sugg.map((p) => (
          <button key={p} className="chip" onClick={() => toggle(p)}>
            + {p}
          </button>
        ))}
      </div>
      <h3>Apps Flatpak ({flatpak.length})</h3>
      <p className="hint">
        Se instalan desde Flathub dentro de la imagen. En la ISO de Arch se instalan al primer
        arranque (necesita red).
      </p>
      <div className="fields">
        <label className="field">
          Añadir app Flatpak (Enter)
          <input
            value={fdraft}
            onChange={(e) => setFdraft(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && addFlatpak()}
            placeholder="ej. org.mozilla.firefox"
          />
        </label>
      </div>
      <div className="chips">
        {flatpak.map((a) => (
          <button
            key={a}
            className="chip sel"
            onClick={() => upd({ flatpakApps: flatpak.filter((x) => x !== a) })}
          >
            {a}
            <span className="x">×</span>
          </button>
        ))}
        {flatpak.length === 0 && <span className="hint">Ninguna todavía.</span>}
      </div>
      <h3>Modpacks (estilo Prism Launcher)</h3>
      <p className="hint">
        Colecciones listas que se aplican de forma aditiva sobre tus paquetes. Haz clic para
        añadir el pack completo.
        {packApplied && (
          <span className="tag" style={{ marginLeft: 8, borderColor: "var(--acc-2)", color: "var(--acc-2)" }}>
            pack «{packApplied}» aplicado
          </span>
        )}
      </p>
      <div className="cards">
        {PACKS.map((pk) => (
          <button
            key={pk.id}
            className="card clickable"
            title={pk.desc}
            onClick={() => upd(applyPack(cfg, pk))}
          >
            <strong>{pk.name}</strong>
            <span className="tag">{pk.packages.length} paquetes{pk.flatpakApps?.length ? ` + ${pk.flatpakApps.length} flatpak` : ""}</span>
            <span className="hint" style={{ margin: 0 }}>{pk.desc}</span>
          </button>
        ))}
      </div>
      <h3>Galería comunitaria</h3>
      <p className="hint">
        Packs compartidos por la comunidad en la carpeta <code>gallery/</code> del repo — se
        cargan desde GitHub y se amplían con un PR.
      </p>
      {gallery.length > 0 && (
        <input
          value={galleryQuery}
          placeholder="filtrar packs de la galería..."
          onChange={(e) => setGalleryQuery(e.target.value)}
          style={{ marginBottom: 8 }}
        />
      )}
      <div className="chips">
        <button
          className="chip"
          onClick={async () => {
            try {
              const res = await fetch(
                "https://raw.githubusercontent.com/Cisne-047/DistroForge/main/gallery/index.json",
                { cache: "no-store" }
              );
              if (!res.ok) throw new Error(`HTTP ${res.status}`);
              const idx = (await res.json()) as { packs: { file: string; name: string; desc: string }[] };
              setGallery(idx.packs ?? []);
              if (!idx.packs?.length) alert("La galería está vacía todavía.");
            } catch (e) {
              alert(`No se pudo cargar la galería: ${e instanceof Error ? e.message : e}`);
            }
          }}
        >
          ↻ Cargar galería
        </button>
        {gallery
          .filter((g) =>
            (g.name + " " + g.desc).toLowerCase().includes(galleryQuery.toLowerCase())
          )
          .map((g) => (
          <button
            key={g.file}
            className="chip"
            title={g.desc}
            onClick={async () => {
              try {
                const res = await fetch(
                  `https://raw.githubusercontent.com/Cisne-047/DistroForge/main/gallery/${g.file}`,
                  { cache: "no-store" }
                );
                if (!res.ok) throw new Error(`HTTP ${res.status}`);
                upd(applyPack(cfg, parsePack(await res.text())));
                setPackApplied(g.name);
                setTimeout(() => setPackApplied(null), 3000);
              } catch (e) {
                alert(`Pack no válido: ${e instanceof Error ? e.message : e}`);
              }
            }}
          >
            + {g.name}
          </button>
        ))}
      </div>
      <h3>Servidor DistroForge (opcional)</h3>
      <p className="hint">
        Conecta un servidor (ver <code>server/</code> del repo) para cuentas, búsqueda server-side
        y Test Drive VNC interactivo. Sin él, todo lo demás sigue funcionando igual.
      </p>
      <div className="fields">
        <label className="field">
          URL del servidor
          <input
            value={srvUrl}
            placeholder="http://localhost:8080"
            onChange={(e) => setSrvUrl(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter") {
                setServerUrl(srvUrl);
                refreshMe();
              }
            }}
          />
        </label>
        {me ? (
          <>
            <span className="tag" style={{ alignSelf: "end", padding: "8px 12px" }}>
              conectado como {me}
            </span>
            <button
              className="btn ghost small"
              style={{ alignSelf: "end" }}
              onClick={() => api.logout().then(refreshMe).catch(() => {})}
            >
              Salir
            </button>
          </>
        ) : (
          <>
            <label className="field">
              Usuario
              <input value={cred.name} onChange={(e) => setCred({ ...cred, name: e.target.value })} />
            </label>
            <label className="field">
              Contraseña
              <input
                type="password"
                value={cred.password}
                onChange={(e) => setCred({ ...cred, password: e.target.value })}
              />
            </label>
            <div style={{ display: "flex", gap: 8, alignSelf: "end" }}>
              <button
                className="btn small"
                onClick={async () => {
                  try {
                    await api.login(cred.name, cred.password);
                    refreshMe();
                  } catch (e) {
                    alert("Login: " + (e instanceof Error ? e.message : e));
                  }
                }}
              >
                Entrar
              </button>
              <button
                className="btn ghost small"
                onClick={async () => {
                  try {
                    await api.register(cred.name, cred.password);
                    refreshMe();
                  } catch (e) {
                    alert("Registro: " + (e instanceof Error ? e.message : e));
                  }
                }}
              >
                Crear cuenta
              </button>
            </div>
          </>
        )}
      </div>
      {me && (
        <>
          <div className="fields" style={{ marginTop: 10 }}>
            <label className="field">
              Búsqueda server-side (FTS5)
              <input
                value={serverQuery}
                placeholder="steam, nas, multimedia..."
                onChange={(e) => setServerQuery(e.target.value)}
                onKeyDown={(e) => {
                  if (e.key === "Enter")
                    api
                      .search(serverQuery)
                      .then((r) => setServerPacks(r.packs))
                      .catch((err) => alert(err instanceof Error ? err.message : String(err)));
                }}
              />
            </label>
            <button
              className="btn ghost small"
              style={{ alignSelf: "end" }}
              onClick={() =>
                api
                  .search(serverQuery)
                  .then((r) => setServerPacks(r.packs))
                  .catch((err) => alert(err instanceof Error ? err.message : String(err)))
              }
            >
              Buscar en el servidor
            </button>
            <button
              className="btn ghost small"
              style={{ alignSelf: "end" }}
              title="Publica tus paquetes y flatpaks actuales como pack"
              onClick={() =>
                api
                  .publish({
                    name: cfg.name,
                    desc: baseById(cfg.base).label + " - " + cfg.packages.length + " paquetes",
                    json: { name: cfg.name, packages: cfg.packages, flatpakApps: cfg.flatpakApps },
                  })
                  .then(() => api.search("").then((r) => setServerPacks(r.packs)))
                  .catch((err) => alert(err instanceof Error ? err.message : String(err)))
              }
            >
              Publicar mi config como pack
            </button>
          </div>
          {serverPacks && (
            <div className="chips">
              {serverPacks.map((sp) => (
                <button
                  key={sp.id}
                  className="chip"
                  title={sp.desc}
                  onClick={() =>
                    upd(
                      applyPack(cfg, {
                        id: String(sp.id),
                        name: sp.name,
                        desc: sp.desc,
                        packages: sp.json.packages,
                        flatpakApps: sp.json.flatpakApps,
                      })
                    )
                  }
                >
                  + {sp.name} - estrella {sp.rating.avg} ({sp.rating.n})
                </button>
              ))}
              {serverPacks.length === 0 && (
                <span className="hint">Sin resultados en el servidor.</span>
              )}
            </div>
          )}
          <div className="fields" style={{ marginTop: 10 }}>
            <button
              className="btn ghost small"
              style={{ alignSelf: "end" }}
              onClick={async () => {
                try {
                  const avail = await api.driveAvailable();
                  if (!avail.available) {
                    alert("El servidor no tiene QEMU instalado.");
                    return;
                  }
                  const path = prompt(
                    "Ruta absoluta de la imagen en el SERVIDOR (la VM arranca alli):",
                    "/tmp/mi-distro.qcow2"
                  );
                  if (!path) return;
                  const sess = await api.driveStart(path);
                  window.open(getServerUrl() + sess.vncUrl, "_blank");
                } catch (e) {
                  alert("Test Drive: " + (e instanceof Error ? e.message : e));
                }
              }}
            >
              Play: Test Drive VNC (arranca tu imagen en el servidor)
            </button>
          </div>
        </>
      )}
      <div className="fields" style={{ marginTop: 12 }}>
        <label className="field" style={{ gridColumn: "1 / -1" }}>
          Importar pack (URL JSON o pega el JSON y pulsa Enter)
          <input
            value={packDraft}
            placeholder="https://.../pack.json o {&quot;name&quot;: ..., &quot;packages&quot;: [...]}"
            onChange={(e) => setPackDraft(e.target.value)}
            onKeyDown={(e) => {
              if (e.key !== "Enter") return;
              applyPackDraft();
            }}
          />
        </label>
      </div>
    </section>
  );
}

export function OutputStep({ cfg, upd }: { cfg: DistroConfig; upd: Upd }) {
  const files = useMemo(() => generateProject(cfg), [cfg]);
  const names = Object.keys(files);
  const [sel, setSel] = useState<string | null>(null);
  const current = sel && sel in files ? sel : names[0];
  const issues = validate(cfg);
  const [busy, setBusy] = useState(false);
  const [copied, setCopied] = useState(false);
  const fileRef = useRef<HTMLInputElement>(null);
  const b = baseById(cfg.base);

  const outOptions: { v: OutputFormat; l: string }[] = cfg.advanced
    ? []
    : b.engine === "kiwi"
      ? [
          { v: "iso", l: "ISO híbrida (live/installer)" },
          { v: "qcow2", l: "Disco QCOW2 (VM)" },
          { v: "usb", l: "Imagen USB / disco raw" },
          { v: "vmdk", l: "Disco VMware (VMDK)" },
          { v: "vdi", l: "Disco VirtualBox (VDI)" },
          { v: "vhd", l: "Disco Hyper-V (VHD)" },
          { v: "oci", l: "Imagen OCI (Docker/Podman)" },
          { v: "wsl", l: "Rootfs WSL (importar en Windows)" },
        ]
      : b.engine === "manual"
        ? []
        : [
            { v: "iso", l: "ISO híbrida (live/installer)" },
            { v: "oci", l: "Imagen OCI (Docker/Podman)" },
            { v: "wsl", l: "Rootfs WSL (importar en Windows)" },
          ];
  const effectiveOut = outOptions.some((o) => o.v === cfg.output)
    ? cfg.output
    : outOptions[0]?.v;

  const exportJson = () => {
    const blob = new Blob([JSON.stringify(cfg, null, 2)], { type: "application/json" });
    const url = URL.createObjectURL(blob);
    const a = document.createElement("a");
    a.href = url;
    a.download = `${cfg.name}.distroforge.json`;
    document.body.appendChild(a);
    a.click();
    a.remove();
    URL.revokeObjectURL(url);
  };

  const importJson = async (f?: File | null) => {
    if (!f) return;
    try {
      const parsed = JSON.parse(await f.text()) as DistroConfig;
      if (!parsed || !BASES.some((x) => x.id === parsed.base))
        throw new Error("base desconocida o fichero no válido");
      upd(mergeConfig(parsed.base, parsed));
    } catch (e) {
      alert(`Config inválida: ${e instanceof Error ? e.message : e}`);
    }
  };

  return (
    <section>
      <h2>5. Construir</h2>
      <p className="hint">Motor: {b.engine}. Revisa la receta generada y descarga el proyecto.</p>
      {outOptions.length > 0 ? (
        <label className="field" style={{ maxWidth: 260 }}>
          Formato de salida
          <select
            value={effectiveOut}
            onChange={(e) => upd({ output: e.target.value as OutputFormat })}
          >
            {outOptions.map((o) => (
              <option key={o.v} value={o.v}>
                {o.l}
              </option>
            ))}
          </select>
        </label>
      ) : (
        <p className="hint">
          {cfg.advanced
            ? "Modo advanced: salida ISO única (remaster de tu ISO base)."
            : "Motor manual: el zip incluye lista de paquetes y guía, sin build automático."}
        </p>
      )}
      {!cfg.advanced && (
        <p className="hint">
          Tamaño estimado de la imagen: <strong>~{(estimateSizeMB(cfg) / 1024).toFixed(1)} GB</strong>{" "}
          ({estimateSizeMB(cfg)} MB — muy orientativo; el peso real depende de la familia y del
          motor).
        </p>
      )}
      {issues.length > 0 && (
        <div className="warnings">
          {issues.map((w, i) => (
            <div key={i}>· {w}</div>
          ))}
        </div>
      )}
      <div className="tabs">
        {names.map((n) => (
          <button key={n} className={n === current ? "on" : ""} onClick={() => setSel(n)}>
            {n}
          </button>
        ))}
      </div>
      <pre className="file">
        {files[current].startsWith("data:image/")
          ? `<<wallpaper binario: ${Math.round(files[current].length / 1.37 / 1024)} KB aprox. — se empaqueta dentro del zip>>`
          : files[current]}
      </pre>
      <p style={{ marginTop: 14, display: "flex", gap: 10, flexWrap: "wrap" }}>
        <button
          className="btn"
          disabled={busy || issues.length > 0}
          onClick={async () => {
            setBusy(true);
            try {
              await downloadZip(cfg);
            } finally {
              setBusy(false);
            }
          }}
        >
          {busy ? "Empaquetando..." : `Descargar ${cfg.name}-distroforge.zip`}
        </button>
        <button className="btn ghost" onClick={exportJson}>
          Guardar config (.json)
        </button>
        <button className="btn ghost" onClick={() => fileRef.current?.click()}>
          Cargar config
        </button>
        <button
          className="btn ghost"
          onClick={async () => {
            try {
              await navigator.clipboard.writeText(shareUrl(cfg));
              setCopied(true);
              setTimeout(() => setCopied(false), 2000);
            } catch {
              alert("El navegador no permitió copiar al portapapeles.");
            }
          }}
        >
          {copied ? "¡Enlace copiado!" : "Copiar enlace"}
        </button>
        <input
          ref={fileRef}
          type="file"
          accept="application/json,.json"
          style={{ display: "none" }}
          onChange={(e) => {
            importJson(e.target.files?.[0]);
            e.target.value = "";
          }}
        />
      </p>
      {(cfg.output === "iso" || (b.engine === "kiwi" && cfg.output !== "oci" && cfg.output !== "wsl")) && !cfg.advanced && (
        <div className="warnings" style={{ borderColor: "var(--acc-2)" }}>
          <strong>Test Drive local (QEMU):</strong>
          <pre style={{ margin: "6px 0 0", overflow: "auto", fontFamily: "inherit" }}>
{cfg.output === "iso"
  ? `qemu-system-x86_64 -enable-kvm -m 2048 -cdrom <ruta del .iso>`
  : `qemu-system-x86_64 -enable-kvm -m 2048 -drive file=<ruta>,format=${cfg.output === "usb" ? "raw" : cfg.output === "qcow2" ? "qcow2" : cfg.output}`}
          </pre>
          <span style={{ color: "var(--dim)", fontSize: 12 }}>
            Requiere QEMU instalado (sudo apt/dnf/pacman install qemu). Para arm64 usa
            qemu-system-aarch64 con UEFI.
          </span>
        </div>
      )}
      <p className="hint">
        El zip incluye la receta, build.sh (Podman en Linux/WSL2), un workflow de GitHub
        Actions para construir la imagen y <code>distroforge.json</code> (reimportable en la
        app y diffable en git). El enlace para compartir lleva la config en la URL —
        incluida la contraseña, en claro.
      </p>
    </section>
  );
}
