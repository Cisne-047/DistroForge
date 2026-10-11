import { useMemo, useRef, useState } from "react";
import type { CustomRepo, DistroConfig, ExtraFile, ExtraUser, OutputFormat, ServiceToggle, SystemConfig } from "./types";
import { BASES, baseById, archsForEngine, mergeConfig, mergePresets, TIMEZONES, LOCALES, KEYBOARDS } from "./catalog";
import { TEMPLATES, applyTemplate } from "./templates";
import { shareUrl } from "./share";
import { generateProject, validate, downloadZip } from "./generators";

type Upd = (patch: Partial<DistroConfig>) => void;

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
        <div
          className="desk-mock"
          style={{ minHeight: 130 }}
        >
          <div className="desk-login">
            <div className="desk-host">{cfg.name || "mi-distro"}</div>
            <div className="desk-user">{b.label} · {b.engine} · {desk}</div>
            <div className="desk-motd">x86_64 · salida {cfg.output.toUpperCase()}</div>
          </div>
          <div className="desk-note">vista previa</div>
        </div>
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
          <div
            className="desk-mock"
            style={
              cfg.wallpaper
                ? {
                    backgroundImage: `url(${cfg.wallpaper})`,
                    backgroundSize: "cover",
                  }
                : undefined
            }
          >
            <div className="desk-login">
              <div className="desk-host">{cfg.system.hostname}</div>
              <div className="desk-user">
                {cfg.system.username || "user"}@{cfg.system.hostname}:~$
              </div>
              {cfg.motd && <div className="desk-motd">{cfg.motd}</div>}
              {cfg.autologin && <div className="desk-badge">auto-login activo</div>}
            </div>
            <div className="desk-note">{b.label}</div>
          </div>
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
