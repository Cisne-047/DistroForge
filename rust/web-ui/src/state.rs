//! Estado persistente: config autoguardada en localStorage, config compartida
//! por URL (#hash) y proyectos guardados ("Mis distros").

use std::collections::BTreeMap;

use forge_core::catalog;
use forge_core::share;
use forge_core::types::{BaseId, DistroConfig, PartialConfig};

use crate::util;

pub const STORAGE_KEY: &str = "distroforge-config";
pub const PROJECTS_KEY: &str = "distroforge-projects";

/// `Partial<DistroConfig>` a partir de una config completa (round-trip JSON),
/// para pasarlo a `merge_config` igual que hace el TS con el spread.
pub fn to_partial(cfg: &DistroConfig) -> PartialConfig {
    serde_json::to_value(cfg)
        .ok()
        .and_then(|v| serde_json::from_value(v).ok())
        .unwrap_or_default()
}

/// Config inicial: 1) hash de la URL, 2) localStorage, 3) defaults.
pub fn load_config() -> DistroConfig {
    // 1) config compartida por URL (#hash) tiene prioridad
    if let Ok(hash) = util::window().location().hash() {
        if hash.len() > 20 {
            if let Some(from_hash) = share::decode_config(&hash[1..]) {
                return catalog::merge_config(from_hash.base, to_partial(&from_hash));
            }
        }
    }
    // 2) localStorage (config corrupta → seguimos con defaults)
    if let Some(raw) = util::ls_get(STORAGE_KEY) {
        if let Ok(p) = serde_json::from_str::<PartialConfig>(&raw) {
            let completa = p.base.is_some()
                && p.name.is_some()
                && p.packages.is_some()
                && p.repos.is_some()
                && p.system.is_some();
            if completa {
                if let Some(base) = p.base {
                    return catalog::merge_config(base, p);
                }
            }
        }
    }
    catalog::default_config(BaseId::OpensuseTumbleweed)
}

/// Proyectos guardados por nombre ("Mis distros").
pub fn load_projects() -> BTreeMap<String, DistroConfig> {
    util::ls_get(PROJECTS_KEY)
        .and_then(|raw| serde_json::from_str(&raw).ok())
        .unwrap_or_default()
}
