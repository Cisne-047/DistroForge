//! Paso 2: sistema (identidad, red, servicios, usuarios, extras). Port de
//! `SystemStep`.

use forge_core::catalog::{KEYBOARDS, LOCALES, TIMEZONES};
use forge_core::types::{BootTarget, CloudUpload, DistroConfig, NetMode};
use leptos::prelude::*;

use crate::util;
use crate::widgets::ImagePicker;

/// Split por comas con trim y filtro de vacíos (como el TS).
fn split_commas(v: &str) -> Vec<String> {
    v.split(',')
        .map(|x| x.trim().to_string())
        .filter(|x| !x.is_empty())
        .collect()
}

#[component]
pub fn SystemStep(cfg: RwSignal<DistroConfig>) -> impl IntoView {
    let net_mode_str = move || match cfg.get().network.mode {
        NetMode::Dhcp => "dhcp".to_string(),
        NetMode::Static => "static".to_string(),
    };
    view! {
        <section>
            <h2>"Sistema y usuarios"</h2>
            <p class="hint">
                "Hostname, cuenta principal, idioma y zona, red, cortafuegos y lo que se ejecuta al
                arrancar. Los valores desatendidos se traducen a preseed, AutoYaST o kickstart según
                la familia."
            </p>
            <div class="fields">
                <label class="field">
                    "Hostname"
                    <input
                        prop:value=move || cfg.get().system.hostname.clone()
                        on:input=move |ev: web_sys::Event| {
                            let v = util::input_val(&ev);
                            cfg.update(|c| c.system.hostname = v);
                        }
                    />
                </label>
                <label class="field">
                    "Usuario"
                    <input
                        prop:value=move || cfg.get().system.username.clone()
                        on:input=move |ev: web_sys::Event| {
                            let v = util::input_val(&ev);
                            cfg.update(|c| c.system.username = v);
                        }
                    />
                </label>
                <label class="field">
                    "Contraseña (viaja en claro en la receta: usa una provisional)"
                    <input
                        type="password"
                        prop:value=move || cfg.get().system.password.clone()
                        on:input=move |ev: web_sys::Event| {
                            let v = util::input_val(&ev);
                            cfg.update(|c| c.system.password = v);
                        }
                    />
                </label>
                <label class="field">
                    "Zona horaria"
                    <select
                        prop:value=move || cfg.get().system.timezone.clone()
                        on:change=move |ev: web_sys::Event| {
                            let v = util::sel_val(&ev);
                            cfg.update(|c| c.system.timezone = v);
                        }
                    >
                        {TIMEZONES
                            .iter()
                            .map(|t| view! { <option value=*t>{*t}</option> })
                            .collect_view()}
                    </select>
                </label>
                <label class="field">
                    "Locale"
                    <select
                        prop:value=move || cfg.get().system.locale.clone()
                        on:change=move |ev: web_sys::Event| {
                            let v = util::sel_val(&ev);
                            cfg.update(|c| c.system.locale = v);
                        }
                    >
                        {LOCALES
                            .iter()
                            .map(|t| view! { <option value=*t>{*t}</option> })
                            .collect_view()}
                    </select>
                </label>
                <label class="field">
                    "Teclado"
                    <select
                        prop:value=move || cfg.get().system.keyboard.clone()
                        on:change=move |ev: web_sys::Event| {
                            let v = util::sel_val(&ev);
                            cfg.update(|c| c.system.keyboard = v);
                        }
                    >
                        {KEYBOARDS
                            .iter()
                            .map(|t| view! { <option value=*t>{*t}</option> })
                            .collect_view()}
                    </select>
                </label>
                <label class="field">
                    "Red"
                    <select
                        prop:value=net_mode_str
                        on:change=move |ev: web_sys::Event| {
                            let v = util::sel_val(&ev);
                            let mode = if v == "static" { NetMode::Static } else { NetMode::Dhcp };
                            cfg.update(|c| c.network.mode = mode);
                        }
                    >
                        <option value="dhcp">"DHCP (automática)"</option>
                        <option value="static">"IP estática"</option>
                    </select>
                </label>
                {move || if cfg.get().network.mode == NetMode::Static {
                    view! {
                        <>
                            <label class="field">
                                "IP"
                                <input
                                    prop:value=move || cfg.get().network.ip.clone()
                                    placeholder="192.168.1.50"
                                    on:input=move |ev: web_sys::Event| {
                                        let v = util::input_val(&ev);
                                        cfg.update(|c| c.network.ip = v);
                                    }
                                />
                            </label>
                            <label class="field">
                                "Prefijo CIDR"
                                <input
                                    prop:value=move || cfg.get().network.prefix.clone()
                                    placeholder="24"
                                    on:input=move |ev: web_sys::Event| {
                                        let v = util::input_val(&ev);
                                        cfg.update(|c| c.network.prefix = v);
                                    }
                                />
                            </label>
                            <label class="field">
                                "Gateway"
                                <input
                                    prop:value=move || cfg.get().network.gateway.clone()
                                    placeholder="192.168.1.1"
                                    on:input=move |ev: web_sys::Event| {
                                        let v = util::input_val(&ev);
                                        cfg.update(|c| c.network.gateway = v);
                                    }
                                />
                            </label>
                            <label class="field" style="grid-column: 1 / -1">
                                "DNS (separados por comas; no aplica en la ISO de Arch)"
                                <input
                                    prop:value=move || cfg.get().network.dns.join(", ")
                                    placeholder="1.1.1.1, 8.8.8.8"
                                    on:input=move |ev: web_sys::Event| {
                                        let v = util::input_val(&ev);
                                        cfg.update(|c| c.network.dns = split_commas(&v));
                                    }
                                />
                            </label>
                        </>
                    }
                        .into_any()
                } else {
                    ().into_any()
                }}
                <label class="chk" style="grid-column: 1 / -1">
                    <input
                        type="checkbox"
                        prop:checked=move || cfg.get().firewall
                        on:change=move |ev: web_sys::Event| {
                            cfg.update(|c| c.firewall = util::checked_val(&ev));
                        }
                    />
                    "Firewall habilitado (firewalld; en Fedora lo gestiona el kickstart)"
                </label>
                <label class="field" style="grid-column: 1 / -1">
                    "Puertos/servicios abiertos en el firewall (separados por comas: ssh, http, 8080/tcp)"
                    <input
                        prop:value=move || cfg.get().firewall_ports.join(", ")
                        placeholder="ssh, 8080/tcp"
                        on:input=move |ev: web_sys::Event| {
                            let v = util::input_val(&ev);
                            cfg.update(|c| c.firewall_ports = split_commas(&v));
                        }
                    />
                </label>
                <label class="field">
                    "Target de arranque"
                    <select
                        prop:value=move || cfg.get().boot_target.as_str().to_string()
                        on:change=move |ev: web_sys::Event| {
                            let v = util::sel_val(&ev);
                            let t = match v.as_str() {
                                "graphical" => BootTarget::Graphical,
                                "text" => BootTarget::Text,
                                _ => BootTarget::Auto,
                            };
                            cfg.update(|c| c.boot_target = t);
                        }
                    >
                        <option value="auto">"Auto (según escritorio)"</option>
                        <option value="graphical">"Gráfico"</option>
                        <option value="text">"Texto (multi-user)"</option>
                    </select>
                </label>
                <label class="field">
                    "Autostart (programas al iniciar sesión, separados por comas)"
                    <input
                        prop:value=move || cfg.get().autostart.join(", ")
                        placeholder="firefox, konsole"
                        on:input=move |ev: web_sys::Event| {
                            let v = util::input_val(&ev);
                            cfg.update(|c| c.autostart = split_commas(&v));
                        }
                    />
                </label>
                <label class="field">
                    "Logo (se copia a /usr/share/distroforge y lo usa el greeter LightDM)"
                    <ImagePicker cfg logo=true max_mb=1.0/>
                </label>
                <label class="field" style="grid-column: 1 / -1">
                    "EULA (texto legal mostrado una vez en el primer login interactivo)"
                    <textarea
                        rows="3"
                        prop:value=move || cfg.get().eula_text.clone()
                        placeholder="Al usar esta distro aceptas que..."
                        on:input=move |ev: web_sys::Event| {
                            let v = util::input_val(&ev);
                            cfg.update(|c| c.eula_text = v);
                        }
                    >
                    </textarea>
                </label>
                <label class="chk" style="grid-column: 1 / -1">
                    <input
                        type="checkbox"
                        prop:checked=move || cfg.get().unattended
                        on:change=move |ev: web_sys::Event| {
                            cfg.update(|c| c.unattended = util::checked_val(&ev));
                        }
                    />
                    "Instalación desatendida con preseed (solo familia Debian; la ISO instala sin preguntar)"
                </label>
                <label class="chk" style="grid-column: 1 / -1">
                    <input
                        type="checkbox"
                        prop:checked=move || cfg.get().smoke_test
                        on:change=move |ev: web_sys::Event| {
                            cfg.update(|c| c.smoke_test = util::checked_val(&ev));
                        }
                    />
                    "Smoke test de arranque en CI (smoke.yml: QEMU headless sobre la ISO)"
                </label>
                <label class="chk" style="grid-column: 1 / -1">
                    <input
                        type="checkbox"
                        prop:checked=move || cfg.get().sign_image
                        on:change=move |ev: web_sys::Event| {
                            cfg.update(|c| c.sign_image = util::checked_val(&ev));
                        }
                    />
                    "Firmar la imagen OCI con cosign keyless (OIDC de GitHub)"
                </label>
                <label class="chk" style="grid-column: 1 / -1">
                    <input
                        type="checkbox"
                        prop:checked=move || cfg.get().test_drive
                        on:change=move |ev: web_sys::Event| {
                            cfg.update(|c| c.test_drive = util::checked_val(&ev));
                        }
                    />
                    "Test Drive en CI (testdrive.yml: capturas del arranque en QEMU, solo ISO)"
                </label>
                <label class="field" style="max-width: 320px">
                    "Subir a la nube tras el build (workflow aparte, requiere secrets en el repo)"
                    <select
                        prop:value=move || cfg.get().cloud_upload.as_str().to_string()
                        on:change=move |ev: web_sys::Event| {
                            let v = util::sel_val(&ev);
                            let u = match v.as_str() {
                                "aws" => CloudUpload::Aws,
                                "azure" => CloudUpload::Azure,
                                _ => CloudUpload::None,
                            };
                            cfg.update(|c| c.cloud_upload = u);
                        }
                    >
                        <option value="none">"No subir"</option>
                        <option value="aws">"AWS (S3 + AMI)"</option>
                        <option value="azure">"Azure (VHD en blob)"</option>
                    </select>
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
                    "Usuarios extra"
                    {move || {
                        cfg.get()
                            .users
                            .iter()
                            .enumerate()
                            .map(|(i, _)| {
                                view! {
                                    <div style="display:flex;gap:8px;margin:6px 0">
                                        <input
                                            style="flex:1"
                                            placeholder="usuario"
                                            prop:value=move || {
                                                cfg.get().users.get(i).map(|u| u.name.clone()).unwrap_or_default()
                                            }
                                            on:input=move |ev: web_sys::Event| {
                                                let v = util::input_val(&ev);
                                                cfg.update(|c| {
                                                    if let Some(u) = c.users.get_mut(i) {
                                                        u.name = v;
                                                    }
                                                });
                                            }
                                        />
                                        <input
                                            style="flex:1"
                                            type="password"
                                            placeholder="contraseña"
                                            prop:value=move || {
                                                cfg.get().users.get(i).map(|u| u.password.clone()).unwrap_or_default()
                                            }
                                            on:input=move |ev: web_sys::Event| {
                                                let v = util::input_val(&ev);
                                                cfg.update(|c| {
                                                    if let Some(u) = c.users.get_mut(i) {
                                                        u.password = v;
                                                    }
                                                });
                                            }
                                        />
                                        <label class="chk">
                                            <input
                                                type="checkbox"
                                                prop:checked=move || {
                                                    cfg.get().users.get(i).is_some_and(|u| u.admin)
                                                }
                                                on:change=move |ev: web_sys::Event| {
                                                    let b = util::checked_val(&ev);
                                                    cfg.update(|c| {
                                                        if let Some(u) = c.users.get_mut(i) {
                                                            u.admin = b;
                                                        }
                                                    });
                                                }
                                            />
                                            "admin"
                                        </label>
                                        <button
                                            class="icon"
                                            title="Eliminar"
                                            on:click=move |_| cfg.update(|c| {
                                                if i < c.users.len() {
                                                    c.users.remove(i);
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
                            c.users.push(Default::default());
                        })
                    >
                        "+ Añadir usuario"
                    </button>
                </div>
                <div class="field" style="grid-column: 1 / -1">
                    "Servicios systemd (habilitar/deshabilitar en la imagen)"
                    {move || {
                        cfg.get()
                            .services
                            .iter()
                            .enumerate()
                            .map(|(i, _)| {
                                view! {
                                    <div style="display:flex;gap:8px;margin:6px 0">
                                        <input
                                            style="flex:1"
                                            placeholder="ej. sshd.service"
                                            prop:value=move || {
                                                cfg.get().services.get(i).map(|s| s.unit.clone()).unwrap_or_default()
                                            }
                                            on:input=move |ev: web_sys::Event| {
                                                let v = util::input_val(&ev);
                                                cfg.update(|c| {
                                                    if let Some(s) = c.services.get_mut(i) {
                                                        s.unit = v;
                                                    }
                                                });
                                            }
                                        />
                                        <label class="chk">
                                            <input
                                                type="checkbox"
                                                prop:checked=move || {
                                                    cfg.get().services.get(i).is_some_and(|s| s.enabled)
                                                }
                                                on:change=move |ev: web_sys::Event| {
                                                    let b = util::checked_val(&ev);
                                                    cfg.update(|c| {
                                                        if let Some(s) = c.services.get_mut(i) {
                                                            s.enabled = b;
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
                                                if i < c.services.len() {
                                                    c.services.remove(i);
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
                            c.services.push(forge_core::types::ServiceToggle {
                                                unit: String::new(),
                                                enabled: true,
                                            });
                        })
                    >
                        "+ Añadir servicio"
                    </button>
                </div>
                <div class="field" style="grid-column: 1 / -1">
                    "Wallpaper (se aplica en GNOME/MATE/Cinnamon/Budgie/Xfce; en KDE/i3/Sway solo se copia)"
                    <ImagePicker cfg logo=false max_mb=2.0/>
                </div>
                <div class="field" style="grid-column: 1 / -1">
                    "Ficheros extra (dotfiles, configs — ruta absoluta dentro de la imagen)"
                    {move || {
                        cfg.get()
                            .extra_files
                            .iter()
                            .enumerate()
                            .map(|(i, _)| {
                                view! {
                                    <div style="display:grid;gap:6px;margin:8px 0">
                                        <div style="display:flex;gap:8px">
                                            <input
                                                style="flex:1"
                                                placeholder="/etc/skel/.bashrc"
                                                prop:value=move || {
                                                    cfg.get().extra_files.get(i).map(|f| f.path.clone()).unwrap_or_default()
                                                }
                                                on:input=move |ev: web_sys::Event| {
                                                    let v = util::input_val(&ev);
                                                    cfg.update(|c| {
                                                        if let Some(f) = c.extra_files.get_mut(i) {
                                                            f.path = v;
                                                        }
                                                    });
                                                }
                                            />
                                            <button
                                                class="icon"
                                                title="Eliminar"
                                                on:click=move |_| cfg.update(|c| {
                                                    if i < c.extra_files.len() {
                                                        c.extra_files.remove(i);
                                                    }
                                                })
                                            >
                                                "✕"
                                            </button>
                                        </div>
                                        <textarea
                                            rows="3"
                                            placeholder="contenido del fichero"
                                            prop:value=move || {
                                                cfg.get().extra_files.get(i).map(|f| f.content.clone()).unwrap_or_default()
                                            }
                                            on:input=move |ev: web_sys::Event| {
                                                let v = util::input_val(&ev);
                                                cfg.update(|c| {
                                                    if let Some(f) = c.extra_files.get_mut(i) {
                                                        f.content = v;
                                                    }
                                                });
                                            }
                                        >
                                        </textarea>
                                    </div>
                                }
                            })
                            .collect_view()
                    }}
                    <button
                        class="btn ghost"
                        on:click=move |_| cfg.update(|c| {
                            c.extra_files.push(Default::default());
                        })
                    >
                        "+ Añadir fichero"
                    </button>
                </div>
            </div>
        </section>
    }
}
