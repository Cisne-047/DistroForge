//! Paso 5: paquetes (input+Enter, chips, sugerencias, flatpaks, modpacks,
//! galería comunitaria e importación por URL/JSON). Port de `PackagesStep`
//! (sin la sección opcional de servidor). Los modpacks son el port de
//! `src/packs.ts` (ver `crate::packs`).

use forge_core::catalog::base_by_id;
use forge_core::types::DistroConfig;
use leptos::prelude::*;
use wasm_bindgen_futures::spawn_local;

use crate::packs::{
    apply_pack, def_to_pack, fetch_pack, parse_pack, GalleryEntry, GalleryIndex, GALLERY_BASE,
    GALLERY_INDEX_URL, PACKS,
};
use crate::util;

/// Muestra un tag transitorio ("pack «X» aplicado") que caduca solo.
fn flash(sig: RwSignal<Option<String>>, val: String, ms: u32) {
    sig.set(Some(val));
    gloo_timers::callback::Timeout::new(ms, move || sig.set(None)).forget();
}

#[component]
pub fn PackagesStep(cfg: RwSignal<DistroConfig>) -> impl IntoView {
    let q = RwSignal::new(String::new());
    let draft = RwSignal::new(String::new());
    let fdraft = RwSignal::new(String::new());
    let pack_draft = RwSignal::new(String::new());
    let pack_applied: RwSignal<Option<String>> = RwSignal::new(None);
    let gallery: RwSignal<Vec<GalleryEntry>> = RwSignal::new(Vec::new());
    let gallery_query = RwSignal::new(String::new());

    let toggle = move |p: String| {
        cfg.update(|c| {
            if let Some(i) = c.packages.iter().position(|x| *x == p) {
                c.packages.remove(i);
            } else {
                c.packages.push(p);
            }
        })
    };
    let add_pkg = move |_| {
        let p = draft.get().trim().to_string();
        if !p.is_empty() {
            cfg.update(|c| {
                if !c.packages.contains(&p) {
                    c.packages.push(p);
                }
            });
        }
        draft.set(String::new());
    };
    let add_flatpak = move |_| {
        let a = fdraft.get().trim().to_string();
        if !a.is_empty() {
            cfg.update(|c| {
                if !c.flatpak_apps.contains(&a) {
                    c.flatpak_apps.push(a);
                }
            });
        }
        fdraft.set(String::new());
    };

    view! {
        <section>
            <h2>"4. Paquetes"</h2>
            <p class="hint">
                {move || {
                    let c = cfg.get();
                    format!(
                        "Los nombres deben existir en los repos de {}. Al cambiar de base, revisa que
                        sigan siendo válidos (cada distro nombra los paquetes distinto).",
                        base_by_id(c.base).label,
                    )
                }}
            </p>
            <div class="fields">
                <label class="field">
                    "Añadir paquete (Enter)"
                    <input
                        prop:value=move || draft.get()
                        placeholder="ej. neofetch"
                        on:input=move |ev: web_sys::Event| {
                            let v = util::input_val(&ev);
                            draft.set(v);
                        }
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            if ev.key() == "Enter" {
                                add_pkg(());
                            }
                        }
                    />
                </label>
                <label class="field">
                    "Filtrar sugeridos"
                    <input
                        prop:value=move || q.get()
                        placeholder="buscar..."
                        on:input=move |ev: web_sys::Event| {
                            let v = util::input_val(&ev);
                            q.set(v);
                        }
                    />
                </label>
            </div>
            <h3>{move || format!("Tus paquetes ({})", cfg.get().packages.len())}</h3>
            <div class="chips">
                {move || {
                    let c = cfg.get();
                    if c.packages.is_empty() {
                        return view! { <span class="hint">"Ninguno todavía."</span> }.into_any();
                    }
                    c.packages
                        .iter()
                        .map(|p| {
                            let p = p.clone();
                            view! {
                                <button
                                    class="chip sel"
                                    on:click=move |_| {
                                        let p = p.clone();
                                        toggle(p);
                                    }
                                >
                                    {p.clone()}
                                    <span class="x">"×"</span>
                                </button>
                            }
                        })
                        .collect_view()
                        .into_any()
                }}
            </div>
            <h3>"Sugeridos"</h3>
            <div class="chips">
                {move || {
                    let c = cfg.get();
                    let ql = q.get().to_lowercase();
                    base_by_id(c.base)
                        .suggestions
                        .iter()
                        .filter(|s| {
                            s.to_lowercase().contains(&ql) && !c.packages.iter().any(|p| p == *s)
                        })
                        .map(|s| {
                            let s = s.to_string();
                            view! {
                                <button
                                    class="chip"
                                    on:click=move |_| {
                                        let s = s.clone();
                                        toggle(s);
                                    }
                                >
                                    {format!("+ {s}")}
                                </button>
                            }
                        })
                        .collect_view()
                }}
            </div>
            <h3>{move || format!("Apps Flatpak ({})", cfg.get().flatpak_apps.len())}</h3>
            <p class="hint">
                "Se instalan desde Flathub dentro de la imagen. En la ISO de Arch se instalan al primer
                arranque (necesita red)."
            </p>
            <div class="fields">
                <label class="field">
                    "Añadir app Flatpak (Enter)"
                    <input
                        prop:value=move || fdraft.get()
                        placeholder="ej. org.mozilla.firefox"
                        on:input=move |ev: web_sys::Event| {
                            let v = util::input_val(&ev);
                            fdraft.set(v);
                        }
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            if ev.key() == "Enter" {
                                add_flatpak(());
                            }
                        }
                    />
                </label>
            </div>
            <div class="chips">
                {move || {
                    let c = cfg.get();
                    if c.flatpak_apps.is_empty() {
                        return view! { <span class="hint">"Ninguna todavía."</span> }.into_any();
                    }
                    c.flatpak_apps
                        .iter()
                        .map(|a| {
                            let a = a.clone();
                            view! {
                                <button
                                    class="chip sel"
                                    on:click=move |_| cfg.update(|c| {
                                        c.flatpak_apps.retain(|x| *x != a);
                                    })
                                >
                                    {a.clone()}
                                    <span class="x">"×"</span>
                                </button>
                            }
                        })
                        .collect_view()
                        .into_any()
                }}
            </div>
            <h3>"Modpacks (estilo Prism Launcher)"</h3>
            <p class="hint">
                "Colecciones listas que se aplican de forma aditiva sobre tus paquetes. Haz clic para
                añadir el pack completo."
                {move || {
                    match pack_applied.get() {
                        Some(name) => view! {
                            <span
                                class="tag"
                                style="margin-left:8px;border-color:var(--acc-2);color:var(--acc-2)"
                            >
                                {format!("pack «{name}» aplicado")}
                            </span>
                        }
                            .into_any(),
                        None => ().into_any(),
                    }
                }}
            </p>
            <div class="cards">
                {PACKS
                    .iter()
                    .map(|pk| {
                        let owned = def_to_pack(pk);
                        view! {
                            <button
                                class="card clickable"
                                title=pk.desc
                                on:click=move |_| {
                                    let cur = cfg.get();
                                    cfg.set(apply_pack(&cur, &owned));
                                }
                            >
                                <strong>{pk.name}</strong>
                                <span class="tag">
                                    {format!(
                                        "{} paquetes{}",
                                        pk.packages.len(),
                                        if pk.flatpak_apps.is_empty() {
                                            String::new()
                                        } else {
                                            format!(" + {} flatpak", pk.flatpak_apps.len())
                                        },
                                    )}
                                </span>
                                <span class="hint" style="margin:0">{pk.desc}</span>
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
            <h3>"Galería comunitaria"</h3>
            <p class="hint">
                "Packs compartidos por la comunidad en la carpeta "
                <code>"gallery/"</code>
                " del repo — se cargan desde GitHub y se amplían con un PR."
            </p>
            {move || if !gallery.get().is_empty() {
                view! {
                    <input
                        prop:value=move || gallery_query.get()
                        placeholder="filtrar packs de la galería..."
                        style="margin-bottom:8px"
                        on:input=move |ev: web_sys::Event| {
                            let v = util::input_val(&ev);
                            gallery_query.set(v);
                        }
                    />
                }
                    .into_any()
            } else {
                ().into_any()
            }}
            <div class="chips">
                <button
                    class="chip"
                    on:click=move |_| {
                        spawn_local(async move {
                            match util::fetch_text(GALLERY_INDEX_URL).await {
                                Ok(text) => match serde_json::from_str::<GalleryIndex>(&text) {
                                    Ok(idx) => {
                                        if idx.packs.is_empty() {
                                            util::alert("La galería está vacía todavía.");
                                        }
                                        gallery.set(idx.packs);
                                    }
                                    Err(e) => {
                                        util::alert(&format!("No se pudo cargar la galería: {e}"));
                                    }
                                },
                                Err(e) => {
                                    util::alert(&format!("No se pudo cargar la galería: {e}"));
                                }
                            }
                        });
                    }
                >
                    "↻ Cargar galería"
                </button>
                {move || {
                    let gq = gallery_query.get().to_lowercase();
                    gallery
                        .get()
                        .iter()
                        .filter(|g| format!("{} {}", g.name, g.desc).to_lowercase().contains(&gq))
                        .map(|g| {
                            let file = g.file.clone();
                            let name = g.name.clone();
                            let desc = g.desc.clone();
                            view! {
                                <button
                                    class="chip"
                                    title=desc
                                    on:click=move |_| {
                                        let url = format!("{GALLERY_BASE}{file}");
                                        spawn_local(async move {
                                            match fetch_pack(&url).await {
                                                Ok(pk) => {
                                                    let name = pk.name.clone();
                                                    let cur = cfg.get();
                                                    cfg.set(apply_pack(&cur, &pk));
                                                    flash(pack_applied, name, 3000);
                                                }
                                                Err(e) => {
                                                    util::alert(&format!("Pack no válido: {e}"));
                                                }
                                            }
                                        });
                                    }
                                >
                                    {format!("+ {name}")}
                                </button>
                            }
                        })
                        .collect_view()
                }}
            </div>
            <div class="fields" style="margin-top:12px">
                <label class="field" style="grid-column: 1 / -1">
                    "Importar pack (URL JSON o pega el JSON y pulsa Enter)"
                    <input
                        prop:value=move || pack_draft.get()
                        placeholder=r#"https://.../pack.json o {"name": ..., "packages": [...]}"#
                        on:input=move |ev: web_sys::Event| {
                            let v = util::input_val(&ev);
                            pack_draft.set(v);
                        }
                        on:keydown=move |ev: web_sys::KeyboardEvent| {
                            if ev.key() != "Enter" {
                                return;
                            }
                            let src = pack_draft.get_untracked().trim().to_string();
                            if src.is_empty() {
                                return;
                            }
                            if src.starts_with("http://") || src.starts_with("https://") {
                                spawn_local(async move {
                                    match fetch_pack(&src).await {
                                        Ok(pk) => {
                                            let name = pk.name.clone();
                                            let cur = cfg.get_untracked();
                                            cfg.set(apply_pack(&cur, &pk));
                                            flash(pack_applied, name, 3000);
                                            pack_draft.set(String::new());
                                        }
                                        Err(e) => {
                                            util::alert(&format!("Pack inválido: {e}"));
                                        }
                                    }
                                });
                            } else {
                                match parse_pack(&src) {
                                    Ok(pk) => {
                                        let name = pk.name.clone();
                                        let cur = cfg.get_untracked();
                                        cfg.set(apply_pack(&cur, &pk));
                                        flash(pack_applied, name, 3000);
                                        pack_draft.set(String::new());
                                    }
                                    Err(e) => util::alert(&format!("Pack inválido: {e}")),
                                }
                            }
                        }
                    />
                </label>
            </div>
        </section>
    }
}
