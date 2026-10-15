//! Shell de la app: cabecera (nombre editable, "Mis distros" en localStorage),
//! navegación de pasos, contenido del paso activo y resumen lateral.
//! Port de `App.tsx`.

use std::collections::BTreeMap;

use forge_core::catalog::{base_by_id, merge_config};
use forge_core::types::DistroConfig;
use leptos::prelude::*;

use crate::state::{load_config, load_projects, to_partial, PROJECTS_KEY, STORAGE_KEY};
use crate::steps::{base::BaseStep, branding::BrandingStep, output::OutputStep,
    packages::PackagesStep, repos::ReposStep, system::SystemStep};
use crate::util;

const STEPS: [(&str, &str); 6] = [
    ("base", "1 · Base"),
    ("system", "2 · Sistema"),
    ("branding", "3 · Identidad"),
    ("repos", "4 · Repos"),
    ("packages", "5 · Paquetes"),
    ("output", "6 · Construir"),
];

/// Paso inicial: `?paso=N` o `#paso=N` (1..6). Permite enlazar directamente a un
/// paso concreto; sin parámetro se empieza por el primero.
fn initial_step() -> usize {
    let loc = util::window().location();
    let raw = loc
        .search()
        .ok()
        .and_then(|s| s.strip_prefix("?paso=").map(|v| v.to_string()))
        .or_else(|| {
            loc.hash()
                .ok()
                .and_then(|h| h.strip_prefix("#paso=").map(|v| v.to_string()))
        });
    match raw {
        Some(v) => v
            .split(['&', '#', '/'])
            .next()
            .unwrap_or("")
            .parse::<usize>()
            .ok()
            .map(|n| n.saturating_sub(1).min(STEPS.len() - 1))
            .unwrap_or(0),
        None => 0,
    }
}

#[component]
pub fn App() -> impl IntoView {
    let cfg = RwSignal::new(load_config());
    let projects: RwSignal<BTreeMap<String, DistroConfig>> = RwSignal::new(load_projects());
    let step = RwSignal::new(initial_step());

    // Autoguardado en localStorage (sin espacio/modo privado: la app sigue).
    Effect::new(move |_| {
        let json = serde_json::to_string(&cfg.get()).unwrap_or_default();
        util::ls_set(STORAGE_KEY, &json);
    });
    Effect::new(move |_| {
        let json = serde_json::to_string(&projects.get()).unwrap_or_default();
        util::ls_set(PROJECTS_KEY, &json);
    });

    let load_project = move |name: String| {
        let p = projects.get().get(&name).cloned();
        if let Some(p) = p {
            cfg.set(merge_config(p.base, to_partial(&p)));
        }
    };

    view! {
        <div class="app">
            <header>
                <span class="brand">
                    <svg class="mark" viewBox="0 0 120 120" aria-hidden="true">
                        <g transform="rotate(45 60 60)">
                            <rect
                                x="16"
                                y="16"
                                width="88"
                                height="88"
                                fill="none"
                                stroke="var(--accent)"
                                stroke-width="9"
                            />
                            <rect x="16" y="16" width="44" height="44" fill="var(--accent)" />
                            <rect x="60" y="60" width="44" height="44" fill="var(--accent)" />
                        </g>
                        <ellipse cx="60" cy="61.5" rx="10.8" ry="13.8" fill="var(--ink)" />
                        <ellipse cx="54.6" cy="55" rx="2.6" ry="4.3" fill="var(--bg)" />
                        <ellipse cx="65.4" cy="55" rx="2.6" ry="4.3" fill="var(--bg)" />
                        <polygon points="56,63.6 64,63.6 60,71.7" fill="#e0a33c" />
                    </svg>
                    "DistroForge"
                </span>
                <input
                    class="name-input"
                    spellcheck="false"
                    aria-label="Nombre de la distro"
                    prop:value=move || cfg.get().name.clone()
                    on:input=move |ev: web_sys::Event| {
                        let v = util::input_val(&ev);
                        cfg.update(|c| c.name = v);
                    }
                />
                <span class="tag">{move || base_by_id(cfg.get().base).label}</span>
                <span class="spacer"></span>
                <select
                    class="proj-select"
                    aria-label="Mis distros"
                    on:change=move |ev: web_sys::Event| {
                        let v = util::sel_val(&ev);
                        if !v.is_empty() {
                            load_project(v);
                        }
                    }
                >
                    <option value="">
                        {move || format!("Mis distros ({})", projects.get().len())}
                    </option>
                    {move || {
                        projects
                            .get()
                            .keys()
                            .map(|n| {
                                let n = n.clone();
                                view! { <option value=n.clone()>{n.clone()}</option> }
                            })
                            .collect_view()
                    }}
                </select>
                <button
                    class="btn small"
                    title="Guarda esta config por su nombre"
                    on:click=move |_| {
                        let c = cfg.get();
                        projects.update(|p| {
                            p.insert(c.name.clone(), c);
                        });
                    }
                >
                    "Guardar"
                </button>
                {move || {
                    let name = cfg.get().name;
                    if projects.get().contains_key(&name) {
                        view! {
                            <button
                                class="btn ghost small"
                                title="Borra la config guardada con este nombre"
                                on:click=move |_| {
                                    projects.update(|p| {
                                        p.remove(&name);
                                    });
                                }
                            >
                                "✕"
                            </button>
                        }
                            .into_any()
                    } else {
                        ().into_any()
                    }
                }}
            </header>
            <nav>
                {STEPS
                    .iter()
                    .enumerate()
                    .map(|(i, s)| {
                        // Contadores en la pestaña, como en Studio: sólo si hay algo.
                        let badge = move || -> Option<String> {
                            let c = cfg.get();
                            let n = match s.0 {
                                "packages" => c.packages.len() + c.flatpak_apps.len(),
                                "repos" => c.repos.len(),
                                "system" => c.users.len() + c.services.len(),
                                "branding" => c.extra_files.len(),
                                _ => 0,
                            };
                            (n > 0).then(|| n.to_string())
                        };
                        view! {
                            <button class:on=move || step.get() == i on:click=move |_| step.set(i)>
                                {s.1}
                                {move || badge().map(|n| view! { <span class="tag">{n}</span> })}
                            </button>
                        }
                    })
                    .collect_view()}
            </nav>
            <main class="grid">
                <div>
                    {move || match step.get() {
                        0 => view! { <BaseStep cfg/> }.into_any(),
                        1 => view! { <SystemStep cfg/> }.into_any(),
                        2 => view! { <BrandingStep cfg/> }.into_any(),
                        3 => view! { <ReposStep cfg/> }.into_any(),
                        4 => view! { <PackagesStep cfg/> }.into_any(),
                        _ => view! { <OutputStep cfg/> }.into_any(),
                    }}
                </div>
                <aside class="summary">
                    <h3>"Resumen"</h3>
                    {move || {
                        let c = cfg.get();
                        let b = base_by_id(c.base);
                        let desk = b
                            .desktops
                            .iter()
                            .find(|d| d.id == c.desktop)
                            .map(|d| d.label)
                            .unwrap_or("—");
                        let active_repos = c
                            .repos
                            .iter()
                            .filter(|r| r.enabled && !r.url.is_empty())
                            .count();
                        let rows: [(&str, String); 12] = [
                            ("Base", b.label.to_string()),
                            ("Motor", b.engine.as_str().to_string()),
                            ("Escritorio", desk.to_string()),
                            ("Paquetes", c.packages.len().to_string()),
                            ("Flatpak", c.flatpak_apps.len().to_string()),
                            ("Ficheros extra", c.extra_files.len().to_string()),
                            ("Usuarios", (1 + c.users.len()).to_string()),
                            ("Servicios", c.services.len().to_string()),
                            ("Repos custom", active_repos.to_string()),
                            ("Salida", c.output.as_str().to_uppercase()),
                            ("Host", c.system.hostname.clone()),
                            (
                                "Red",
                                if c.network.mode == forge_core::types::NetMode::Static {
                                    "Estática".to_string()
                                } else {
                                    "DHCP".to_string()
                                },
                            ),
                        ];
                        rows.iter()
                            .map(|(k, v)| {
                                view! {
                                    <div class="row">
                                        <span>{*k}</span>
                                        <span>{v.clone()}</span>
                                    </div>
                                }
                            })
                            .collect_view()
                    }}
                </aside>
            </main>
        </div>
    }
}
