//! Paso 3: branding (MOTD, wallpaper, GRUB, icono de inicio, preview WYSIWYG
//! con mock de GRUB + mock de escritorio). Port de `BrandingStep`.

use forge_core::catalog::base_by_id;
use forge_core::types::{js_num, DistroConfig};
use leptos::prelude::*;

use crate::preview::DesktopPreview;
use crate::util;
use crate::widgets::ImagePicker;

const START_ICONS: [&str; 8] = ["◆", "▲", "❖", "🐧", "🦅", "🔥", "⚡", "🍎"];

#[component]
pub fn BrandingStep(cfg: RwSignal<DistroConfig>) -> impl IntoView {
    let label = Signal::derive(move || base_by_id(cfg.get().base).label.to_string());
    view! {
        <section>
            <h2>"3. Branding"</h2>
            <p class="hint">
                "Cómo luce y arranca tu distro: mensaje de bienvenida, fondo de pantalla y menú GRUB.
                La vista previa refleja la config al instante."
            </p>
            <div class="branding-grid">
                <div>
                    <div class="fields">
                        <label class="field" style="grid-column: 1 / -1">
                            "Mensaje MOTD (/etc/motd)"
                            <textarea
                                rows="3"
                                prop:value=move || cfg.get().motd.clone()
                                on:input=move |ev: web_sys::Event| {
                                    let v = util::input_val(&ev);
                                    cfg.update(|c| c.motd = v);
                                }
                            >
                            </textarea>
                        </label>
                        <div class="field" style="grid-column: 1 / -1">
                            "Fondo de pantalla (dconf/xfconf en GNOME/MATE/Cinnamon/Budgie/Xfce; solo copia en
                            KDE/i3/Sway)"
                            <ImagePicker cfg logo=false max_mb=2.0/>
                        </div>
                        <label class="field">
                            "GRUB: timeout (segundos)"
                            <input
                                type="number"
                                min="0"
                                max="999"
                                prop:value=move || js_num(cfg.get().grub.timeout)
                                on:input=move |ev: web_sys::Event| {
                                    let v = util::input_val(&ev);
                                    // Number("") === 0
                                    let n = v.trim().parse::<f64>().unwrap_or(0.0);
                                    cfg.update(|c| c.grub.timeout = n);
                                }
                            />
                        </label>
                        <label class="field">
                            "GRUB: parámetros del kernel"
                            <input
                                prop:value=move || cfg.get().grub.extra_params.clone()
                                placeholder="ej. quiet splash"
                                on:input=move |ev: web_sys::Event| {
                                    let v = util::input_val(&ev);
                                    cfg.update(|c| c.grub.extra_params = v);
                                }
                            />
                        </label>
                        <label class="chk" style="grid-column: 1 / -1">
                            <input
                                type="checkbox"
                                prop:checked=move || cfg.get().autologin
                                on:change=move |ev: web_sys::Event| {
                                    cfg.update(|c| c.autologin = util::checked_val(&ev));
                                }
                            />
                            "Auto-login del usuario principal (SDDM/GDM/LightDM)"
                        </label>
                        <div class="field" style="grid-column: 1 / -1">
                            "Icono del botón inicio (taskbar de la vista previa; si subes un logo, tiene prioridad)"
                            <div style="display:flex;gap:6px;flex-wrap:wrap;align-items:center">
                                {START_ICONS
                                    .iter()
                                    .map(|ic| {
                                        let ic = *ic;
                                        view! {
                                            <button
                                                class="chip"
                                                class:sel=move || cfg.get().start_icon == ic
                                                style="font-size:15px"
                                                on:click=move |_| {
                                                    cfg.update(|c| c.start_icon = ic.to_string());
                                                }
                                            >
                                                {ic}
                                            </button>
                                        }
                                    })
                                    .collect_view()}
                                <input
                                    prop:value=move || cfg.get().start_icon.clone()
                                    maxlength="3"
                                    style="width:64px;text-align:center"
                                    aria-label="Icono personalizado"
                                    on:input=move |ev: web_sys::Event| {
                                        let v = util::input_val(&ev);
                                        cfg.update(|c| c.start_icon = v);
                                    }
                                />
                            </div>
                        </div>
                    </div>
                </div>
                <div class="preview">
                    <div class="grub-mock">
                        <div class="grub-title">"GNU GRUB  version 2.12"</div>
                        <div class="grub-entry grub-sel">
                            {move || {
                                let name = cfg.get().name;
                                format!("* {} (linux)", if name.is_empty() { "mi-distro" } else { &name })
                            }}
                        </div>
                        <div class="grub-entry">"Memory test (memtest86+)"</div>
                        <div class="grub-entry">"System setup"</div>
                        <div class="grub-foot">
                            {move || {
                                let t = cfg.get().grub.timeout;
                                format!("Use ↑ y ↓ para seleccionar… timeout: {}s", js_num(t.max(0.0)))
                            }}
                        </div>
                    </div>
                    <DesktopPreview cfg label show_motd=true/>
                </div>
            </div>
        </section>
    }
}
