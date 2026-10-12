//! Paso 4: repos custom. Port de `ReposStep`.

use forge_core::types::{CustomRepo, DistroConfig};
use leptos::prelude::*;

use crate::util;

#[component]
pub fn ReposStep(cfg: RwSignal<DistroConfig>) -> impl IntoView {
    view! {
        <section>
            <h2>"3. Repositorios custom"</h2>
            <p class="hint">
                "URLs extra: OBS, COPR, tu propio mirror... La URL va tal cual al gestor de paquetes
                del motor (zypper/apt/pacman/dnf). Si el repo necesita clave GPG, pon la URL de la key."
            </p>
            {move || {
                cfg.get()
                    .repos
                    .iter()
                    .enumerate()
                    .map(|(i, r)| {
                        let _ = r;
                        view! {
                            <div class="repo-row">
                                <input
                                    placeholder="alias"
                                    prop:value=move || {
                                        cfg.get().repos.get(i).map(|r| r.alias.clone()).unwrap_or_default()
                                    }
                                    on:input=move |ev: web_sys::Event| {
                                        let v = util::input_val(&ev);
                                        cfg.update(|c| {
                                            if let Some(r) = c.repos.get_mut(i) {
                                                r.alias = v;
                                            }
                                        });
                                    }
                                />
                                <input
                                    placeholder="https://... url del repo"
                                    prop:value=move || {
                                        cfg.get().repos.get(i).map(|r| r.url.clone()).unwrap_or_default()
                                    }
                                    on:input=move |ev: web_sys::Event| {
                                        let v = util::input_val(&ev);
                                        cfg.update(|c| {
                                            if let Some(r) = c.repos.get_mut(i) {
                                                r.url = v;
                                            }
                                        });
                                    }
                                />
                                <input
                                    placeholder="URL key GPG (opcional)"
                                    prop:value=move || {
                                        cfg.get().repos.get(i).map(|r| r.gpg_key.clone()).unwrap_or_default()
                                    }
                                    on:input=move |ev: web_sys::Event| {
                                        let v = util::input_val(&ev);
                                        cfg.update(|c| {
                                            if let Some(r) = c.repos.get_mut(i) {
                                                r.gpg_key = v;
                                            }
                                        });
                                    }
                                />
                                <input
                                    placeholder="suite comps (ej. noble main)"
                                    prop:value=move || {
                                        cfg.get().repos.get(i).map(|r| r.suite.clone()).unwrap_or_default()
                                    }
                                    on:input=move |ev: web_sys::Event| {
                                        let v = util::input_val(&ev);
                                        cfg.update(|c| {
                                            if let Some(r) = c.repos.get_mut(i) {
                                                r.suite = v;
                                            }
                                        });
                                    }
                                />
                                <label class="chk">
                                    <input
                                        type="checkbox"
                                        prop:checked=move || {
                                            cfg.get().repos.get(i).is_some_and(|r| r.enabled)
                                        }
                                        on:change=move |ev: web_sys::Event| {
                                            let b = util::checked_val(&ev);
                                            cfg.update(|c| {
                                                if let Some(r) = c.repos.get_mut(i) {
                                                    r.enabled = b;
                                                }
                                            });
                                        }
                                    />
                                    "activo"
                                </label>
                                <button
                                    class="icon"
                                    title="Eliminar"
                                    on:click=move |_| cfg.update(|c| {
                                        if i < c.repos.len() {
                                            c.repos.remove(i);
                                        }
                                    })
                                >
                                    "✕"
                                </button>
                            </div>
                        }
                    })
                    .collect_view()
            }}
            <button
                class="btn ghost"
                on:click=move |_| cfg.update(|c| {
                    c.repos.push(CustomRepo {
                        alias: String::new(),
                        url: String::new(),
                        gpg_key: String::new(),
                        suite: String::new(),
                        enabled: true,
                    });
                })
            >
                "+ Añadir repo"
            </button>
        </section>
    }
}
