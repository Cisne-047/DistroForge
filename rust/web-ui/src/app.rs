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

/// Clave de localStorage donde se recuerda el tema elegido.
const TEMA_KEY: &str = "distroforge-tema";

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

    // Tema claro/oscuro. El atributo lo pone antes de pintar un script en
    // index.html (para no ver un destello); aquí se recuerda la elección y se
    // aplica al vuelo.
    let tema = RwSignal::new(util::ls_get(TEMA_KEY).unwrap_or_else(|| "oscuro".to_string()));
    let aplicar_tema = move |t: &str| {
        util::ls_set(TEMA_KEY, t);
        if let Some(html) = util::window().document().and_then(|d| d.document_element()) {
            let _ = html.set_attribute("data-tema", t);
        }
    };

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
                    <img class="mark" src="marca-cabeza.png" alt="" width="26" height="26" />
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
                    class="btn ghost small"
                    aria-label="Cambiar entre tema claro y oscuro"
                    title=move || {
                        if tema.get() == "claro" { "Pasar a oscuro" } else { "Pasar a claro" }
                    }
                    on:click=move |_| {
                        let nuevo = if tema.get() == "claro" { "oscuro" } else { "claro" };
                        tema.set(nuevo.to_string());
                        aplicar_tema(nuevo);
                    }
                >
                    {move || {
                        if tema.get() == "claro" {
                            view! {
                                <svg class="ic sm" viewBox="0 0 24 24">
                                    <path d="M20.2 14.8A8.6 8.6 0 0 1 9.2 3.8a8.6 8.6 0 1 0 11 11Z" />
                                </svg>
                            }
                                .into_any()
                        } else {
                            view! {
                                <svg class="ic sm" viewBox="0 0 24 24">
                                    <circle cx="12" cy="12" r="4.2" />
                                    <path d="M12 2.6v2.6M12 18.8v2.6M2.6 12h2.6M18.8 12h2.6M5.4 5.4l1.9 1.9M16.7 16.7l1.9 1.9M18.6 5.4l-1.9 1.9M7.3 16.7l-1.9 1.9" />
                                </svg>
                            }
                                .into_any()
                        }
                    }}
                </button>
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
