import { useEffect, useState } from "react";
import type { DistroConfig } from "./types";
import { BASES, baseById, defaultConfig } from "./catalog";
import { BaseStep, SystemStep, ReposStep, PackagesStep, OutputStep } from "./steps";

const STEPS = [
  { id: "base", label: "1 · Base" },
  { id: "system", label: "2 · Sistema" },
  { id: "repos", label: "3 · Repos" },
  { id: "packages", label: "4 · Paquetes" },
  { id: "output", label: "5 · Construir" },
] as const;

type StepId = (typeof STEPS)[number]["id"];

const STORAGE_KEY = "distroforge-config";
const PROJECTS_KEY = "distroforge-projects";

function loadConfig(): DistroConfig {
  try {
    const raw = localStorage.getItem(STORAGE_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as DistroConfig;
      if (
        parsed &&
        BASES.some((b) => b.id === parsed.base) &&
        typeof parsed.name === "string" &&
        Array.isArray(parsed.packages) &&
        Array.isArray(parsed.repos) &&
        parsed.system
      ) {
        return { ...defaultConfig(parsed.base), ...parsed };
      }
    }
  } catch {
    // config corrupta en localStorage: seguimos con valores por defecto
  }
  return defaultConfig("opensuse-tumbleweed");
}

function loadProjects(): Record<string, DistroConfig> {
  try {
    const raw = localStorage.getItem(PROJECTS_KEY);
    if (raw) {
      const parsed = JSON.parse(raw) as Record<string, DistroConfig>;
      if (parsed && typeof parsed === "object") return parsed;
    }
  } catch {
    // proyectos corruptos: empezamos vacíos
  }
  return {};
}

export default function App() {
  const [cfg, setCfg] = useState<DistroConfig>(loadConfig);
  const [step, setStep] = useState<StepId>("base");
  const [projects, setProjects] = useState<Record<string, DistroConfig>>(loadProjects);
  const upd = (patch: Partial<DistroConfig>) => setCfg((c) => ({ ...c, ...patch }));
  useEffect(() => {
    try {
      localStorage.setItem(STORAGE_KEY, JSON.stringify(cfg));
    } catch {
      // sin espacio o modo privado: la app sigue funcionando sin persistencia
    }
  }, [cfg]);
  useEffect(() => {
    try {
      localStorage.setItem(PROJECTS_KEY, JSON.stringify(projects));
    } catch {
      // ídem
    }
  }, [projects]);
  const saveProject = () => setProjects((p) => ({ ...p, [cfg.name]: cfg }));
  const deleteProject = (n: string) =>
    setProjects((p) => {
      const q = { ...p };
      delete q[n];
      return q;
    });
  const loadProject = (n: string) => {
    const p = projects[n];
    if (p) setCfg({ ...defaultConfig(p.base), ...p });
  };
  const b = baseById(cfg.base);
  const desk = b.desktops.find((d) => d.id === cfg.desktop)?.label ?? "—";
  const activeRepos = cfg.repos.filter((r) => r.enabled && r.url).length;

  return (
    <div className="app">
      <header>
        <span className="brand">DistroForge</span>
        <input
          className="name-input"
          value={cfg.name}
          spellCheck={false}
          onChange={(e) => upd({ name: e.target.value })}
          aria-label="Nombre de la distro"
        />
        <span className="tag">{b.label}</span>
        <span className="spacer" />
        <select
          className="proj-select"
          value=""
          onChange={(e) => e.target.value && loadProject(e.target.value)}
          aria-label="Mis distros"
        >
          <option value="">Mis distros ({Object.keys(projects).length})</option>
          {Object.keys(projects).map((n) => (
            <option key={n} value={n}>
              {n}
            </option>
          ))}
        </select>
        <button className="btn small" onClick={saveProject} title="Guarda esta config por su nombre">
          Guardar
        </button>
        {projects[cfg.name] && (
          <button
            className="btn ghost small"
            onClick={() => deleteProject(cfg.name)}
            title="Borra la config guardada con este nombre"
          >
            ✕
          </button>
        )}
      </header>
      <nav>
        {STEPS.map((s) => (
          <button key={s.id} className={s.id === step ? "on" : ""} onClick={() => setStep(s.id)}>
            {s.label}
          </button>
        ))}
      </nav>
      <main className="grid">
        <div>
          {step === "base" && <BaseStep cfg={cfg} upd={upd} />}
          {step === "system" && <SystemStep cfg={cfg} upd={upd} />}
          {step === "repos" && <ReposStep cfg={cfg} upd={upd} />}
          {step === "packages" && <PackagesStep cfg={cfg} upd={upd} />}
          {step === "output" && <OutputStep cfg={cfg} upd={upd} />}
        </div>
        <aside className="summary">
          <h3>Resumen</h3>
          <div className="row">
            <span>Base</span>
            <span>{b.label}</span>
          </div>
          <div className="row">
            <span>Motor</span>
            <span>{b.engine}</span>
          </div>
          <div className="row">
            <span>Escritorio</span>
            <span>{desk}</span>
          </div>
          <div className="row">
            <span>Paquetes</span>
            <span>{cfg.packages.length}</span>
          </div>
          <div className="row">
            <span>Flatpak</span>
            <span>{(cfg.flatpakApps ?? []).length}</span>
          </div>
          <div className="row">
            <span>Ficheros extra</span>
            <span>{(cfg.extraFiles ?? []).length}</span>
          </div>
          <div className="row">
            <span>Usuarios</span>
            <span>{1 + (cfg.users ?? []).length}</span>
          </div>
          <div className="row">
            <span>Servicios</span>
            <span>{(cfg.services ?? []).length}</span>
          </div>
          <div className="row">
            <span>Repos custom</span>
            <span>{activeRepos}</span>
          </div>
          <div className="row">
            <span>Salida</span>
            <span>{cfg.output.toUpperCase()}</span>
          </div>
          <div className="row">
            <span>Host</span>
            <span>{cfg.system.hostname}</span>
          </div>
          <div className="row">
            <span>Red</span>
            <span>{cfg.network?.mode === "static" ? "Estática" : "DHCP"}</span>
          </div>
        </aside>
      </main>
    </div>
  );
}
