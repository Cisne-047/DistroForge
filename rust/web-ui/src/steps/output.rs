//! Paso 6: construir (selector de salida, preview de ficheros generados con
//! tabs, warnings, descarga ZIP, guardar/cargar config JSON y enlace para
//! compartir). Port de `OutputStep`.

use std::collections::BTreeMap;

use forge_core::catalog::base_by_id;
use forge_core::generators::{estimate_size_mb, generate_project, validate};
use forge_core::share;
use forge_core::types::{DistroConfig, EngineId, OutputFormat};
use leptos::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::spawn_local;

use crate::util;

#[component]
pub fn OutputStep(cfg: RwSignal<DistroConfig>) -> impl IntoView {
    let sel: RwSignal<Option<String>> = RwSignal::new(None);
    let busy = RwSignal::new(false);
    let copied = RwSignal::new(false);

    let files: Memo<BTreeMap<String, String>> =
        Memo::new(move |_| generate_project(&cfg.get()));
    let issues = Memo::new(move |_| validate(&cfg.get()));

    // Lista de formatos válidos según motor (vacía en advanced/manual).
    let out_options = move || -> Vec<(OutputFormat, &'static str)> {
        let c = cfg.get();
        if c.advanced {
            return Vec::new();
        }
        match base_by_id(c.base).engine {
            EngineId::Kiwi => vec![
                (OutputFormat::Iso, "ISO híbrida (live/installer)"),
                (OutputFormat::Qcow2, "Disco QCOW2 (VM)"),
                (OutputFormat::Usb, "Imagen USB / disco raw"),
                (OutputFormat::Vmdk, "Disco VMware (VMDK)"),
                (OutputFormat::Vdi, "Disco VirtualBox (VDI)"),
                (OutputFormat::Vhd, "Disco Hyper-V (VHD)"),
                (OutputFormat::Oci, "Imagen OCI (Docker/Podman)"),
                (OutputFormat::Wsl, "Rootfs WSL (importar en Windows)"),
            ],
            EngineId::Manual => Vec::new(),
            _ => vec![
                (OutputFormat::Iso, "ISO híbrida (live/installer)"),
                (OutputFormat::Oci, "Imagen OCI (Docker/Podman)"),
                (OutputFormat::Wsl, "Rootfs WSL (importar en Windows)"),
            ],
        }
    };

    let current_file = move || {
        let files = files.get();
        if let Some(s) = sel.get() {
            if files.contains_key(&s) {
                return s;
            }
        }
        files.keys().next().cloned().unwrap_or_default()
    };

    view! {
        <section>
            <h2>"Revisa y construye"</h2>
            <p class="hint">
                {move || format!("Motor: {}. Revisa la receta generada, descarga el proyecto o pruébalo en el navegador.", base_by_id(cfg.get().base).engine)}
            </p>
            {move || {
                let opts = out_options();
                if opts.is_empty() {
                    let advanced = cfg.get().advanced;
                    view! {
                        <p class="hint">
                            {if advanced {
                                "Modo avanzado: salida ISO única (remaster de tu ISO base).".to_string()
                            } else {
                                "Motor manual: el zip incluye lista de paquetes y guía, sin build automático.".to_string()
                            }}
                        </p>
                    }
                        .into_any()
                } else {
                    let c = cfg.get();
                    let eff = opts
                        .iter()
                        .find(|(v, _)| *v == c.output)
                        .map(|(v, _)| *v)
                        .unwrap_or(opts[0].0);
                    view! {
                        <label class="field" style="max-width:260px">
                            "Formato de salida"
                            <select
                                prop:value=eff.as_str()
                                on:change=move |ev: web_sys::Event| {
                                    let v = util::sel_val(&ev);
                                    if let Some(f) = OutputFormat::parse(&v) {
                                        cfg.update(|c| c.output = f);
                                    }
                                }
                            >
                                {opts
                                    .iter()
                                    .map(|(v, l)| view! { <option value=v.as_str()>{*l}</option> })
                                    .collect_view()}
                            </select>
                        </label>
                    }
                        .into_any()
                }
            }}
            {move || {
                if cfg.get().advanced {
                    return ().into_any();
                }
                let mb = estimate_size_mb(&cfg.get());
                view! {
                    <p class="hint">
                        "Tamaño estimado de la imagen: "
                        <strong>{format!("~{:.1} GB", mb as f64 / 1024.0)}</strong>
                        {format!(
                            " ({mb} MB — muy orientativo; el peso real depende de la familia y del motor)."
                        )}
                    </p>
                }
                    .into_any()
            }}
            {move || {
                let list = issues.get();
                if list.is_empty() {
                    return ().into_any();
                }
                view! {
                    <div class="warnings">
                        {list
                            .iter()
                            .map(|w| view! { <div>{format!("· {w}")}</div> })
                            .collect_view()}
                    </div>
                }
                    .into_any()
            }}
            <div class="tabs">
                {move || {
                    let cur = current_file();
                    files
                        .get()
                        .keys()
                        .map(|k| {
                            let k = k.clone();
                            let is_cur = k == cur;
                            view! {
                                <button class:on=move || is_cur on:click=move |_| sel.set(Some(k.clone()))>
                                    {k.clone()}
                                </button>
                            }
                        })
                        .collect_view()
                }}
            </div>
            <pre class="file">
                {move || {
                    let files = files.get();
                    let cur = current_file();
                    match files.get(&cur) {
                        Some(c) if c.starts_with("data:image/") => {
                            let kb = (c.len() as f64 / 1.37 / 1024.0).round();
                            format!("<<wallpaper binario: {kb} KB aprox. — se empaqueta dentro del zip>>")
                        }
                        Some(c) => c.clone(),
                        None => String::new(),
                    }
                }}
            </pre>
            <p style="margin-top:14px;display:flex;gap:10px;flex-wrap:wrap">
                <button
                    class="btn"
                    disabled=move || busy.get() || !issues.get().is_empty()
                    on:click=move |_| {
                        busy.set(true);
                        let c = cfg.get();
                        let generated = generate_project(&c);
                        let ok = util::download_zip(&c.name, &generated);
                        let _ = ok;
                        busy.set(false);
                    }
                >
                    {move || {
                        if busy.get() {
                            "Empaquetando...".to_string()
                        } else {
                            format!("Descargar {}-distroforge.zip", cfg.get().name)
                        }
                    }}
                </button>
                <button
                    class="btn ghost"
                    on:click=move |_| {
                        let c = cfg.get();
                        let json = serde_json::to_string_pretty(&c).unwrap_or_default();
                        util::download_text_file(
                            &format!("{}.distroforge.json", c.name),
                            &json,
                        );
                    }
                >
                    "Guardar config (.json)"
                </button>
                <button class="btn ghost" on:click=move |_| {
                    if let Some(el) = util::window()
                        .document()
                        .and_then(|d| d.get_element_by_id("df-load-config"))
                    {
                        let _ = el.unchecked_into::<web_sys::HtmlInputElement>().click();
                    }
                }>
                    "Cargar config"
                </button>
                <button
                    class="btn ghost"
                    on:click=move |_| {
                        let encoded = share::encode_config(&cfg.get());
                        let url = util::share_url(&encoded);
                        spawn_local(async move {
                            if util::copy_to_clipboard(&url).await {
                                copied.set(true);
                                gloo_timers::callback::Timeout::new(2000, move || copied.set(false))
                                    .forget();
                            } else {
                                util::alert("El navegador no permitió copiar al portapapeles.");
                            }
                        });
                    }
                >
                    {move || if copied.get() { "¡Enlace copiado!".to_string() } else { "Copiar enlace".to_string() }}
                </button>
                <input
                    id="df-load-config"
                    type="file"
                    accept="application/json,.json"
                    style="display:none"
                    on:change=move |ev: web_sys::Event| {
                        if let Some(f) = util::file_from_input_event(&ev) {
                            util::read_as_text(&f, move |text| {
                                match serde_json::from_str::<forge_core::types::PartialConfig>(&text)
                                {
                                    Ok(p) => match p.base {
                                        Some(base) => {
                                            cfg.set(forge_core::catalog::merge_config(base, p));
                                        }
                                        None => util::alert(
                                            "Config inválida: base desconocida o fichero no válido",
                                        ),
                                    },
                                    Err(e) => util::alert(&format!("Config inválida: {e}")),
                                }
                            });
                        }
                    }
                />
            </p>
            {move || {
                let c = cfg.get();
                let engine = base_by_id(c.base).engine;
                let show = !c.advanced
                    && (c.output == OutputFormat::Iso
                        || (engine == EngineId::Kiwi
                            && c.output != OutputFormat::Oci
                            && c.output != OutputFormat::Wsl));
                if !show {
                    return ().into_any();
                }
                let cmd = if c.output == OutputFormat::Iso {
                    "qemu-system-x86_64 -enable-kvm -m 2048 -cdrom <ruta del .iso>".to_string()
                } else {
                    let fmt = match c.output {
                        OutputFormat::Usb => "raw",
                        OutputFormat::Qcow2 => "qcow2",
                        other => other.as_str(),
                    };
                    format!("qemu-system-x86_64 -enable-kvm -m 2048 -drive file=<ruta>,format={fmt}")
                };
                view! {
                    <div class="warnings" style="border-color:var(--acc-2)">
                        <strong>"Test Drive local (QEMU):"</strong>
                        <pre style="margin:6px 0 0;overflow:auto;font-family:inherit">{cmd}</pre>
                        <span style="color:var(--dim);font-size:12px">
                            "Requiere QEMU instalado (sudo apt/dnf/pacman install qemu). Para arm64 usa
                            qemu-system-aarch64 con UEFI."
                        </span>
                    </div>
                }
                    .into_any()
            }}
            <p class="hint">
                "El zip incluye la receta, build.sh (Podman en Linux/WSL2), un workflow de GitHub
                Actions para construir la imagen y "
                <code>"distroforge.json"</code>
                " (reimportable en la app y diffable en git). El enlace para compartir lleva la config
                en la URL — incluida la contraseña, en claro."
            </p>
        </section>
    }
}
