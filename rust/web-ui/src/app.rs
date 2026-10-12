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
    ("branding", "3 · Branding"),
    ("repos", "4 · Repos"),
    ("packages", "5 · Paquetes"),
    ("output", "6 · Construir"),
];

#[component]
pub fn App() -> impl IntoView {
    let cfg = RwSignal::new(load_config());
    let projects: RwSignal<BTreeMap<String, DistroConfig>> = RwSignal::new(load_projects());
    let step = RwSignal::new(0usize);

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
                <span class="brand">"DistroForge"</span>
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
                        view! {
                            <button class:on=move || step.get() == i on:click=move |_| step.set(i)>
                                {s.1}
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
