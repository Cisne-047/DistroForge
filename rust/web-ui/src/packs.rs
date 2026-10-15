//! Modpacks estilo Prism Launcher: port de `src/packs.ts` (data pura) más la
//! galería comunitaria (gallery/index.json en GitHub).

use forge_core::catalog::base_by_id;
use forge_core::types::DistroConfig;

/// Pack (colección aditiva de paquetes/flatpaks). Compartible como JSON/URL.
#[derive(Debug, Clone)]
pub struct Pack {
    /// Solo lo usa `parse_pack` (fidelidad con el TS); la UI no lo lee.
    #[allow(dead_code)]
    pub id: String,
    pub name: String,
    /// Informativo (title/tooltips); los packs importados no lo muestran.
    #[allow(dead_code)]
    pub desc: String,
    pub packages: Vec<String>,
    pub flatpak_apps: Option<Vec<String>>,
    pub desktop: Option<String>,
}

/// Fila estática del catálogo (strings const-construibles).
pub struct PackDef {
    pub id: &'static str,
    pub name: &'static str,
    pub desc: &'static str,
    pub packages: &'static [&'static str],
    pub flatpak_apps: &'static [&'static str],
}

/// Catálogo de packs (port literal de `PACKS` en packs.ts).
pub static PACKS: [PackDef; 8] = [
    PackDef {
        id: "gaming",
        name: "Gaming",
        desc: "Steam, Lutris, Wine y GameMode con MangoHud para overlays.",
        packages: &[
            "steam",
            "lutris",
            "wine",
            "mangohud",
            "gamemode",
            "steam-native-runtime",
        ],
        flatpak_apps: &["com.valvesoftware.Steam", "net.lutris.Lutris"],
    },
    PackDef {
        id: "dev",
        name: "Desarrollo",
        desc: "Editor, contenedores y toolchain web/Python listos para trabajar.",
        packages: &[
            "git",
            "vim",
            "neovim",
            "docker",
            "docker-compose",
            "nodejs",
            "npm",
            "python3-pip",
            "tmux",
        ],
        flatpak_apps: &["com.visualstudio.code"],
    },
    PackDef {
        id: "media",
        name: "Edición multimedia",
        desc: "Kdenlive, GIMP, Audacity e Inkscape para foto, vídeo y audio.",
        packages: &[
            "kdenlive",
            "gimp",
            "audacity",
            "inkscape",
            "ffmpeg",
            "obs-studio",
        ],
        flatpak_apps: &["org.kde.kdenlive", "org.gimp.GIMP"],
    },
    PackDef {
        id: "servidor-web",
        name: "Servidor web",
        desc: "nginx + certbot + herramientas de red para servir cosas.",
        packages: &["nginx", "certbot", "curl", "htop", "fail2ban", "rsync"],
        flatpak_apps: &[],
    },
    PackDef {
        id: "virtualizacion",
        name: "Virtualización",
        desc: "libvirt/QEMU con virt-manager para VMs al estilo hypervisor.",
        packages: &["qemu-kvm", "libvirt", "virt-manager", "bridge-utils"],
        flatpak_apps: &["org.virt_manager.virt-manager"],
    },
    PackDef {
        id: "retro",
        name: "Retro gaming",
        desc: "Emuladores y frontends retro con soporte de mandos.",
        packages: &["retroarch", "dolphin-emu", "mame", "pcsx2", "steam"],
        flatpak_apps: &["org.libretro.RetroArch"],
    },
    PackDef {
        id: "streaming",
        name: "Streaming",
        desc: "OBS Studio, V4L2 y utilidades de captura para emitir.",
        packages: &["obs-studio", "v4l2loopback", "ffmpeg"],
        flatpak_apps: &["com.obsproject.Studio"],
    },
    PackDef {
        id: "privacidad",
        name: "Privacidad",
        desc: "Tor, VPN wireguard y cifrado de discos/portapapeles.",
        packages: &["tor", "wireguard-tools", "keepassxc", "veracrypt"],
        flatpak_apps: &[
            "org.torproject.torbrowser-launcher",
            "org.keepassxc.KeePassXC",
        ],
    },
];

/// Convierte una fila estática en `Pack` propiedad.
pub fn def_to_pack(d: &PackDef) -> Pack {
    Pack {
        id: d.id.to_string(),
        name: d.name.to_string(),
        desc: d.desc.to_string(),
        packages: d.packages.iter().map(|s| s.to_string()).collect(),
        flatpak_apps: if d.flatpak_apps.is_empty() {
            None
        } else {
            Some(d.flatpak_apps.iter().map(|s| s.to_string()).collect())
        },
        desktop: None,
    }
}

/// Aplica un pack de forma ADITIVA (dedupe); el escritorio sugerido solo se
/// aplica si la base lo tiene y la config está en "none". Port de `applyPack`.
pub fn apply_pack(cfg: &DistroConfig, pack: &Pack) -> DistroConfig {
    let b = base_by_id(cfg.base);
    let mut packages = cfg.packages.clone();
    for p in &pack.packages {
        if !packages.contains(p) {
            packages.push(p.clone());
        }
    }
    let mut flatpaks = cfg.flatpak_apps.clone();
    if let Some(apps) = &pack.flatpak_apps {
        for a in apps {
            if !flatpaks.contains(a) {
                flatpaks.push(a.clone());
            }
        }
    }
    let desktop = match &pack.desktop {
        Some(d) if b.desktops.iter().any(|x| x.id == *d) && cfg.desktop == "none" => {
            d.clone()
        }
        _ => cfg.desktop.clone(),
    };
    DistroConfig {
        packages,
        flatpak_apps: flatpaks,
        desktop,
        ..cfg.clone()
    }
}

/// `name.toLowerCase().replace(/[^a-z0-9-]+/g, "-")`.
fn slugify(name: &str) -> String {
    let mut out = String::new();
    let mut last_dash = false;
    for ch in name.to_lowercase().chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '-' {
            out.push(ch);
            last_dash = false;
        } else if !last_dash {
            out.push('-');
            last_dash = true;
        }
    }
    out
}

/// Parsea un pack externo (objeto JSON o `{ pack: {...} }`); `Err` si es
/// inválido. Port de `parsePack`.
pub fn parse_pack(raw: &str) -> Result<Pack, String> {
    let obj: serde_json::Value = serde_json::from_str(raw).map_err(|e| e.to_string())?;
    let p = if obj.get("pack").is_some() {
        obj.get("pack").unwrap()
    } else {
        &obj
    };
    let bad = || "el pack debe ser JSON con name y packages[]".to_string();
    let name = p
        .get("name")
        .and_then(|v| v.as_str())
        .ok_or_else(bad)?
        .to_string();
    let arr = p.get("packages").and_then(|v| v.as_array()).ok_or_else(bad)?;
    let packages = arr
        .iter()
        .map(|x| match x.as_str() {
            Some(s) => s.to_string(),
            None => x.to_string(), // String(x) de JS
        })
        .collect();
    let id = match p.get("id").and_then(|v| v.as_str()) {
        Some(s) => s.to_string(),
        None => slugify(&name),
    };
    Ok(Pack {
        id,
        name,
        desc: match p.get("desc") {
            Some(serde_json::Value::String(s)) => s.clone(),
            Some(v) if !v.is_null() => v.to_string(),
            _ => String::new(),
        },
        packages,
        flatpak_apps: p.get("flatpakApps").and_then(|v| v.as_array()).map(|a| {
            a.iter()
                .map(|x| x.as_str().map(String::from).unwrap_or_else(|| x.to_string()))
                .collect()
        }),
        desktop: p.get("desktop").and_then(|v| v.as_str()).map(String::from),
    })
}

/// Entrada del índice de la galería comunitaria.
#[derive(Debug, Clone, serde::Deserialize)]
pub struct GalleryEntry {
    pub file: String,
    pub name: String,
    pub desc: String,
}

/// `gallery/index.json`.
#[derive(serde::Deserialize, Default)]
pub struct GalleryIndex {
    #[serde(default)]
    pub packs: Vec<GalleryEntry>,
}

/// Índice de la galería comunitaria.
///
/// Ruta **relativa**: Trunk copia el directorio `gallery/` del repo dentro de
/// `dist/` (ver `index.html`, `data-trunk rel="copy-dir"`), así que la galería
/// viaja con la app y no depende de que el repo sea público. Funciona igual en
/// GitHub Pages, en `trunk serve` y en el binario de escritorio (Tauri sirve
/// `rust/web-ui/dist` como frontend).
pub const GALLERY_INDEX_URL: &str = "gallery/index.json";

/// Prefijo para resolver los `file` del índice (`packs/nas.json` → `gallery/packs/nas.json`).
pub const GALLERY_BASE: &str = "gallery/";

/// Carga y parsea un pack desde una URL.
pub async fn fetch_pack(url: &str) -> Result<Pack, String> {
    let text = crate::util::fetch_text(url).await?;
    parse_pack(&text)
}
