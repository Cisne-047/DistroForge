//! Mock WYSIWYG del escritorio recién instalado, según familia del escritorio:
//! top bar + dock (GNOME), taskbar inferior (KDE/Xfce/...), barra de
//! workspaces (i3/Sway) o TTY (servidor). Port de `DesktopPreview` en steps.tsx.

use forge_core::types::DistroConfig;
use leptos::prelude::*;

use crate::util::{date_string, hhmm};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChromeKind {
    Gnome,
    Panel,
    Tiling,
    Terminal,
}

/// Familia de "chrome" del escritorio (port de `chromeOf`).
pub fn chrome_of(desktop_id: &str) -> ChromeKind {
    match desktop_id {
        "gnome" | "zorincore" | "popgnome" | "pantheon" | "cosmic" | "budgie" => {
            ChromeKind::Gnome
        }
        "kde" | "xfce" | "mate" | "lxqt" | "cinnamon" => ChromeKind::Panel,
        "i3" | "sway" => ChromeKind::Tiling,
        _ => ChromeKind::Terminal, // "none" y desconocidos
    }
}

/// Mock del escritorio: todos los datos salen de la config (reactiva).
///
/// `label` se muestra en la esquina (etiqueta base·escritorio·arq o label de
/// la base); `show_motd` controla si se pinta la línea de MOTD (el paso Base
/// no la pasa en el original, Branding sí).
#[component]
pub fn DesktopPreview(
    cfg: RwSignal<DistroConfig>,
    label: Signal<String>,
    show_motd: bool,
) -> impl IntoView {
    move || {
        let c = cfg.get();
        let kind = chrome_of(&c.desktop);
        let host = if c.system.hostname.is_empty() {
            "mi-distro".to_string()
        } else {
            c.system.hostname.clone()
        };
        let user = if c.system.username.is_empty() {
            "user".to_string()
        } else {
            c.system.username.clone()
        };
        let bg = if c.wallpaper.is_empty() {
            String::new()
        } else {
            format!("background-image:url({});background-size:cover;", c.wallpaper)
        };
        let label = label.get().clone();
        let start = if c.start_icon.is_empty() {
            "◆".to_string()
        } else {
            c.start_icon.clone()
        };
        let motd = c.motd.clone();
        let logo = c.logo.clone();
        match kind {
            ChromeKind::Terminal => term_view(host, user, motd, show_motd, bg, label).into_any(),
            _ => desk_view(kind, host, user, motd, show_motd, bg, label, start, logo).into_any(),
        }
    }
}

/// TTY (escritorio "none").
fn term_view(
    host: String,
    user: String,
    motd: String,
    show_motd: bool,
    bg: String,
    label: String,
) -> impl IntoView {
    view! {
        <div class="desk-mock dp-term" style=bg>
            <div class="dp-term-body">
                <div>{format!("{host} login: {user}")}</div>
                <div>"Password:"</div>
                <div class="dp-dim">{format!("Last login: {} on tty1", date_string())}</div>
                {(show_motd && !motd.is_empty())
                    .then(move || view! { <div class="dp-motd">{motd}</div> })}
                <div>{format!("{user}@{host}:~$ _")}</div>
            </div>
            <div class="desk-note">{label}</div>
        </div>
    }
}

/// Escritorio gráfico (gnome / panel / tiling).
#[allow(clippy::too_many_arguments)]
fn desk_view(
    kind: ChromeKind,
    host: String,
    user: String,
    motd: String,
    show_motd: bool,
    bg: String,
    label: String,
    start_icon: String,
    logo: String,
) -> impl IntoView {
    let clock = hhmm();
    let is_gnome = kind == ChromeKind::Gnome;
    let is_panel = kind == ChromeKind::Panel;
    let is_tiling = kind == ChromeKind::Tiling;
    let start_icon_3: String = start_icon.chars().take(3).collect();
    view! {
        <div class="desk-mock" style=bg>
            {is_gnome.then({
                let clock = clock.clone();
                move || {
                    view! {
                        <div class="dp-topbar">
                            <span>"Actividades"</span>
                            <span class="dp-clock">{clock.clone()}</span>
                            <span>"⌗ ▤ ⏻"</span>
                        </div>
                        <div class="dp-dock">
                            {["🌐", "📁", "✉", "⚙", "📋"]
                                .iter()
                                .map(|i| view! { <span class="dp-icon">{*i}</span> })
                                .collect_view()}
                        </div>
                    }
                }
            })}
            {is_panel.then({
                let clock = clock.clone();
                let label = label.clone();
                let logo = logo.clone();
                move || {
                    view! {
                        <div class="dp-panel">
                            <span class="dp-start">
                                {if logo.is_empty() {
                                    view! { {start_icon_3.clone()} }.into_any()
                                } else {
                                    view! {
                                        <img src=logo.clone() alt="icono inicio"
                                            style="width:14px;height:14px"/>
                                    }.into_any()
                                }}
                            </span>
                            <span class="dp-task">{format!("🌐 {label}")}</span>
                            <span class="dp-task dp-dim">"terminal"</span>
                            <span class="dp-clock">{format!("🔊 🔔 {clock}")}</span>
                        </div>
                    }
                }
            })}
            {is_tiling.then({
                let label = label.clone();
                let host = host.clone();
                move || {
                    view! {
                        <div class="dp-tile">
                            <div class="dp-win">{format!("1: {label}")}</div>
                            <div class="dp-win">"2: terminal"</div>
                        </div>
                        <div class="dp-wsbar">{format!("1 2 3 4 5 · {host}")}</div>
                    }
                }
            })}
            {(!is_tiling).then(move || {
                view! {
                    <div class="desk-login">
                        <div class="desk-host">{host.clone()}</div>
                        <div class="desk-user">{format!("{user}@{host}:~$")}</div>
                        {(show_motd && !motd.is_empty())
                            .then(move || view! { <div class="desk-motd">{motd}</div> })}
                    </div>
                }
            })}
            <div class="desk-note">{label}</div>
        </div>
    }
}
