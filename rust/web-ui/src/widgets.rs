//! Componentes compartidos.

use forge_core::types::DistroConfig;
use leptos::prelude::*;

use crate::util;

/// Input de imagen (logo/wallpaper): sube el fichero como data URL con límite
/// de tamaño, o muestra la preview + botón "Quitar" si ya hay valor.
///
/// `logo = true` → campo `cfg.logo` (máx 1 MB); `false` → `cfg.wallpaper`
/// (máx 2 MB). `max_mb` solo se usa para el mensaje de error.
#[component]
pub fn ImagePicker(cfg: RwSignal<DistroConfig>, logo: bool, max_mb: f64) -> impl IntoView {
    let value = move || {
        let c = cfg.get();
        if logo { c.logo.clone() } else { c.wallpaper.clone() }
    };
    let clear = move |_| {
        cfg.update(|c| {
            if logo {
                c.logo = String::new();
            } else {
                c.wallpaper = String::new();
            }
        })
};
    let alt = if logo { "logo actual" } else { "wallpaper actual" };
    view! {
        <div style="display:flex;gap:10px;align-items:center">
            {move || {
                let v = value();
                if v.is_empty() {
                    view! {
                        <input
                            type="file"
                            accept="image/png,image/jpeg,image/webp"
                            on:change=move |ev: web_sys::Event| {
                                if let Some(f) = util::file_from_input_event(&ev) {
                                    if f.size() as f64 > max_mb * 1024.0 * 1024.0 {
                                        util::alert(&format!(
                                            "{} demasiado grande (máximo {} MB).",
                                            if logo { "Logo" } else { "Imagen" },
                                            max_mb as u32,
                                        ));
                                        return;
                                    }
                                    util::read_as_data_url(&f, move |data| {
                                        cfg.update(|c| {
                                            if logo {
                                                c.logo = data;
                                            } else {
                                                c.wallpaper = data;
                                            }
                                        })
                                    });
                                }
                            }
                        />
                    }
                        .into_any()
                } else {
                    view! {
                        <>
                            <img class="wall-prev" src=v alt=alt/>
                            <button class="btn ghost" on:click=clear>"Quitar"</button>
                        </>
                    }
                        .into_any()
                }
            }}
        </div>
    }
}
