//! CLI `forge` — equivalente a `scripts/dump.ts`: vuelca a disco el proyecto
//! generado para una base, para probar builds reales.
//!
//! Uso: `forge dump [base] [dir-salida] [iso|qcow2|usb|vmdk|vdi|oci|wsl] [config.json]`
//!
//! - `base` por defecto "debian".
//! - `dir-salida` por defecto `dump-<base>`.
//! - El formato ajusta `cfg.output`; en el TS solo iso/qcow2/oci/wsl se
//!   aplicaban (usb/vmdk/vdi se ignoraban en silencio); aquí se aceptan los
//!   siete tal como pide el spec del CLI.
//! - `config.json` (exportado por la app) se fusiona sobre los valores por
//!   defecto con semántica `Object.assign` (superficial).
//!
//! Sin clap: los argumentos se leen a mano con `std::env` para minimizar deps.

#![forbid(unsafe_code)]

use std::fs;
use std::path::Path;
use std::process::exit;

use forge_core::catalog::{assign_partial, default_config, BASES};
use forge_core::generators::{generate_project, validate};
use forge_core::types::{BaseId, OutputFormat, PartialConfig};

fn usage() -> ! {
    let bases: Vec<&str> = BASES.iter().map(|b| b.id.as_str()).collect();
    eprintln!(
        "Uso: forge dump [base] [dir-salida] [iso|qcow2|usb|vmdk|vdi|oci|wsl] [config.json]\nBases: {}",
        bases.join(", ")
    );
    exit(1);
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "-h" || a == "--help") {
        usage();
    }
    // Subcomando `dump <base> <dir> [fmt] [config.json]`; se tolera invocarlo
    // sin el "dump" (los args se desplazan).
    let rest: &[String] = match args.get(1).map(|s| s.as_str()) {
        Some("dump") => &args[2..],
        Some(_) => &args[1..],
        None => &[],
    };
    let base_arg = rest.first().map(|s| s.as_str()).unwrap_or("debian");
    let base = match BaseId::parse(base_arg) {
        Some(b) => b,
        None => {
            let bases: Vec<&str> = BASES.iter().map(|b| b.id.as_str()).collect();
            eprintln!("Base desconocida: {base_arg}. Bases: {}", bases.join(", "));
            exit(1);
        }
    };

    let mut cfg = default_config(base);
    cfg.system.password = "distroforge-test".to_string();

    if let Some(fmt_arg) = rest.get(2) {
        match OutputFormat::parse(fmt_arg) {
            Some(f) => cfg.output = f,
            None => eprintln!("Formato ignorado (no reconocido): {fmt_arg}"),
        }
    }

    if let Some(cfg_path) = rest.get(3) {
        let data = match fs::read_to_string(cfg_path) {
            Ok(d) => d,
            Err(e) => {
                eprintln!("No se pudo leer {cfg_path}: {e}");
                exit(1);
            }
        };
        let parsed: PartialConfig = match serde_json::from_str(&data) {
            Ok(p) => p,
            Err(e) => {
                eprintln!("JSON inválido en {cfg_path}: {e}");
                exit(1);
            }
        };
        assign_partial(&mut cfg, parsed);
    }

    let out = rest
        .get(1)
        .cloned()
        .unwrap_or_else(|| format!("dump-{}", base.as_str()));

    // Barrera de seguridad: el CLI también valida, aunque la config venga de un
    // fichero a mano o de un pack compartido.
    let problems = validate(&cfg);
    if !problems.is_empty() {
        eprintln!("Configuración no válida:");
        for p in &problems {
            eprintln!("  - {p}");
        }
        exit(2);
    }

    let files = generate_project(&cfg);
    for (p, content) in &files {
        // Ni rutas absolutas ni saltos hacia atrás fuera del directorio de salida.
        let safe = !p.starts_with('/')
            && !p.contains('\\')
            && p.split('/')
                .all(|seg| seg != ".." && seg != "." && !seg.is_empty());
        if !safe {
            eprintln!("Ruta no segura en el proyecto generado: {p}");
            exit(1);
        }
        let full = Path::new(&out).join(p);
        if let Some(parent) = full.parent() {
            if let Err(e) = fs::create_dir_all(parent) {
                eprintln!("No se pudo crear {}: {e}", parent.display());
                exit(1);
            }
        }
        if let Err(e) = fs::write(&full, content.as_bytes()) {
            eprintln!("No se pudo escribir {}: {e}", full.display());
            exit(1);
        }
    }
    println!(
        "Proyecto {} volcado en {} ({} ficheros)",
        base.as_str(),
        out,
        files.len()
    );
}
