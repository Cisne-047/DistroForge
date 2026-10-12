//! Paso 1: base (14 tarjetas, plantillas rápidas, escritorios, arquitecturas,
//! modo advanced y preview del escritorio). Port de `BaseStep`.

use forge_core::catalog::{self, archs_for_engine, base_by_id, BASES};
use forge_core::templates::{apply_template, TEMPLATES};
use forge_core::types::{ArchId, DistroConfig, EngineId, OutputFormat};
use leptos::prelude::*;

use crate::preview::DesktopPreview;
use crate::util;

#[component]
pub fn BaseStep(cfg: RwSignal<DistroConfig>) -> impl IntoView {
    let label = Signal::derive(move || {
        let c = cfg.get();
        let b = base_by_id(c.base);
        let desk = b
            .desktops
            .iter()
            .find(|d| d.id == c.desktop)
            .map(|d| d.label)
            .unwrap_or("—");
        format!("{} · {} · {}", b.label, desk, c.arch)
    });
    view! {
        <section>
            <h2>"1. Base"</h2>
            <p class="hint">
                "Distribución sobre la que montas tu distro. El motor de construcción se elige solo:
                KIWI (openSUSE), live-build (Debian), mkarchiso (Arch) o kickstart (Fedora)."
            </p>
            <h3>"Plantillas rápidas"</h3>
            <div class="chips">
                {TEMPLATES
                    .iter()
                    .map(|t| {
                        let t = t.clone();
                        view! {
                            <button
                                class="chip"
                                title=t.desc
                                on:click=move |_| {
                                    let cur = cfg.get();
                                    cfg.set(apply_template(&cur, &t));
                                }
                            >
                                {format!("✨ {}", t.label)}
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
            <div class="cards">
                {BASES
                    .iter()
                    .map(|x| {
                        let x = *x;
                        view! {
                            <button
                                class="card clickable"
                                class:sel=move || cfg.get().base == x.id
                                on:click=move |_| {
                                    cfg.update(|c| {
                                        c.base = x.id;
                                        c.desktop = x.desktops[0].id.to_string();
                                        c.repos = catalog::merge_presets(&x, &c.repos);
                                        if !archs_for_engine(x.engine).contains(&c.arch) {
                                            c.arch = ArchId::X86_64;
                                        }
                                        // Los formatos de disco solo los construye KIWI.
                                        if x.engine != EngineId::Kiwi
                                            && matches!(
                                                c.output,
                                                OutputFormat::Qcow2
                                                    | OutputFormat::Usb
                                                    | OutputFormat::Vmdk
                                                    | OutputFormat::Vdi
                                            )
                                        {
                                            c.output = OutputFormat::Iso;
                                        }
                                    });
                                }
                            >
                                <strong>{x.label}</strong>
                                <span class="tag">{format!("motor: {}", x.engine)}</span>
                            </button>
                        }
                    })
                    .collect_view()}
            </div>
            <div class="preview" style="margin:0 0 16px">
                <DesktopPreview cfg label show_motd=false/>
                <span class="hint">
                    {move || {
                        let c = cfg.get();
                        format!(
                            "Así arrancará una instalación limpia con esta base y este escritorio (salida {}).",
                            c.output.as_str().to_uppercase(),
                        )
                    }}
                </span>
            </div>
            <h3>"Escritorio"</h3>
            <div class="cards">
                {move || {
                    base_by_id(cfg.get().base)
                        .desktops
                        .iter()
                        .map(|d| {
                            let id = d.id;
                            let label = d.label;
                            view! {
                                <button
                                    class="card clickable"
                                    class:sel=move || cfg.get().desktop == id
                                    on:click=move |_| cfg.update(|c| c.desktop = id.to_string())
                                >
                                    <strong>{label}</strong>
                                </button>
                            }
                        })
                        .collect_view()
                }}
            </div>
            <h3>"Arquitectura"</h3>
            <div class="cards">
                {move || {
                    let engine = base_by_id(cfg.get().base).engine;
                    archs_for_engine(engine)
                        .iter()
                        .map(|a| {
                            let a = *a;
                            view! {
                                <button
                                    class="card clickable"
                                    class:sel=move || cfg.get().arch == a
                                    on:click=move |_| cfg.update(|c| c.arch = a)
                                >
                                    <strong>{a.as_str()}</strong>
                                </button>
                            }
                        })
                        .collect_view()
                }}
            </div>
            <h3>"Modo advanced (experimental)"</h3>
            <p class="hint">
                "Remasteriza una ISO que tú aportas (cualquier distro exótica): se extrae, se aplica tu
                config en un chroot y se reconstruye. Salida ISO única."
            </p>
            <label class="chk">
                <input
                    type="checkbox"
                    prop:checked=move || cfg.get().advanced
                    on:change=move |ev: web_sys::Event| {
                        cfg.update(|c| c.advanced = util::checked_val(&ev));
                    }
                />
                "Usar una ISO custom como base"
            </label>
            {move || if cfg.get().advanced {
                view! {
                    <div class="fields">
                        <label class="field" style="grid-column: 1 / -1">
                            "URL de la ISO base (descargable, termina en .iso)"
                            <input
                                prop:value=move || cfg.get().custom_iso.url.clone()
                                placeholder="https://ejemplo.com/distro-amd64.iso"
                                on:input=move |ev: web_sys::Event| {
                                    let v = util::input_val(&ev);
                                    cfg.update(|c| c.custom_iso.url = v);
                                }
                            />
                        </label>
                        <label class="field" style="grid-column: 1 / -1">
                            "Script chroot avanzado (se añade al generado; control total)"
                            <textarea
                                rows="5"
                                prop:value=move || cfg.get().custom_iso.chroot_script.clone()
                                placeholder="#!/bin/bash — comandos libres dentro del chroot"
                                on:input=move |ev: web_sys::Event| {
                                    let v = util::input_val(&ev);
                                    cfg.update(|c| c.custom_iso.chroot_script = v);
                                }
                            >
                            </textarea>
                        </label>
                    </div>
                }
                    .into_any()
            } else {
                ().into_any()
            }}
        </section>
    }
}
