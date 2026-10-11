# DistroForge — núcleo en Rust

Port fiel a Rust del núcleo TypeScript de DistroForge (`src/`, `scripts/dump.ts`).
Genera los mismos proyectos (recetas live-build/KIWI/mkarchiso/kickstart,
Dockerfile OCI, rootfs WSL, workflow de GitHub Actions) con **strings
byte a byte idénticos** a los del frontend TS.

```
rust/
├── Cargo.toml              # workspace: forge-core, forge-cli, forge-wasm (tauri-app excluido)
├── crates/
│   ├── forge-core/         # librería: types, catalog, templates, share, generators + tests
│   ├── forge-cli/          # binario `forge dump ...` (equivalente a scripts/dump.ts)
│   └── forge-wasm/         # binding wasm-bindgen para la web app
└── tauri-app/              # scaffold de escritorio (Tauri 2), fuera del workspace
```

## Compilar y probar

Requisitos: Rust 1.80+ (probado con 1.99).

```bash
cd rust

# Tests del núcleo (batería portada de tests/generators.test.ts)
cargo test -p forge-core

# CLI
cargo build -p forge-cli
cargo run -p forge-cli -- dump debian /tmp/forge-cli-check iso

# Binding WASM (verificación en host; no hace falta target wasm32)
cargo check -p forge-wasm
```

### CLI `forge`

```
forge dump [base] [dir-salida] [iso|qcow2|usb|vmdk|vdi|oci|wsl] [config.json]
```

- `base` por defecto `debian`; `dir-salida` por defecto `dump-<base>`.
- El formato ajusta `cfg.output`; `config.json` (exportado por la app) se
  fusiona sobre los defaults con semántica `Object.assign` superficial.
- Sin clap: argumentos a mano con `std::env`; deps = `forge-core` + `serde_json`.

### forge-wasm

Expone `generate_project(cfg_json) -> Result<String, JsValue>` (JSON
`{"ruta": "contenido"}`) y `validate(cfg_json) -> Result<Vec<String>, JsValue>`
(más `encode_config`/`decode_config`). Se compila a `wasm32-unknown-unknown`
para el frontend; `cargo check -p forge-wasm` en host basta para verificar.

### tauri-app

Scaffold de escritorio (Tauri 2 + `forge-core` por path). Excluido del
workspace a propósito: no se compila en el flujo normal (ver
`tauri-app/README.md`).

## Mapeo módulo TS → crate

| TS | Rust | Notas |
|---|---|---|
| `src/types.ts` | `forge-core/src/types.rs` | enums cerrados para ids; serde camelCase; `grub.timeout` serializa como entero cuando es integral (`10`, no `10.0`) |
| `src/catalog.ts` | `forge-core/src/catalog.rs` | `BASES` como `[BaseDef; 14]` estático; `base_by_id` total (índices verificados en compilación); `mergeConfig`/`mergePresets`/`effectiveRepos`/`defaultConfig` portados; `TIMEZONES`/`LOCALES`/`KEYBOARDS` |
| `src/templates.ts` | `forge-core/src/templates.rs` | `TEMPLATES` + `apply_template` (desktop solo si la base lo tiene) |
| `src/share.ts` | `forge-core/src/share.rs` | base64url a mano (sin crate `base64`); `shareUrl` no se porta (depende de `location`) |
| `src/generators/shell.ts` | `.../generators/shell.rs` | `sq`, `dq`, `dirname_of`, `systemd_safe` |
| `src/generators/accounts.ts` | `.../generators/accounts.rs` | usuarios, servicios, firewall, autostart, autologin, target |
| `src/generators/flatpak.ts` | `.../generators/flatpak.rs` | |
| `src/generators/network.ts` | `.../generators/network.rs` | IPv4 sin crate `regex`; máscara dotted replica `Number() \|\| 24` |
| `src/generators/branding.ts` | `.../generators/branding.rs` | dconf/xfconf |
| `src/generators/cloudinit.ts` | `.../generators/cloudinit.rs` | |
| `src/generators/customiso.ts` | `.../generators/customiso.rs` | ver desviación 2 abajo |
| `src/generators/livebuild.ts` | `.../generators/livebuild.rs` | `sanitize_alias` vive aquí (como en el TS) |
| `src/generators/archiso.ts` | `.../generators/archiso.rs` | |
| `src/generators/kiwi.ts` | `.../generators/kiwi.rs` | |
| `src/generators/kickstart.ts` | `.../generators/kickstart.rs` | |
| `src/generators/dockerfile.ts` | `.../generators/dockerfile.rs` | |
| `src/generators/wsl.ts` | `.../generators/wsl.rs` | |
| `src/generators/manual.ts` | `.../generators/manual.rs` | |
| `src/generators/index.ts` | `.../generators/mod.rs` | `generate_project -> BTreeMap<String,String>`, `validate -> Vec<String>`, `workflow_yaml`, `readme_md`; `downloadZip` no se porta (JSZip/browser) |
| `tests/generators.test.ts` | `forge-core/tests/generators.rs` | 67 tests; `js-yaml` → `serde_yaml` (dev-dep); el QA `bash -n` no se porta |
| `scripts/dump.ts` | `forge-cli/src/main.rs` | ver desviación 3 abajo |

## Fidelidad con el TS

Verificado generando proyectos con `scripts/dump.ts` (vite-node) y el CLI
Rust con la misma config y haciendo `diff -r`: **byte a byte idénticos** en
las 14 bases × salidas (iso/qcow2/usb→iso/oci/wsl) con una config completa
(repos con/sin key GPG, usuarios extra, servicios, autologin, autostart,
puertos firewall, red estática, cloud-init, motd, wallpaper, GRUB, flatpak,
ficheros extra). Incluye `distroforge.json` (orden de campos y formato JSON
idénticos) y los workflows de CI (parsean con `serde_yaml` igual que con
`js-yaml`).

Desviaciones conscientes (documentadas también en el código):

1. **Tipos cerrados**: `BaseId`/`EngineId`/`ArchId`/`OutputFormat` son enums.
   El TS lanza en `baseById` con una base desconocida; en Rust ese estado no
   existe. `decodeConfig` del TS acepta cualquier `base` string; el Rust
   devuelve `None` si la base del hash no es una de las 14 (y los campos
   ausentes toman defaults coherentes en vez de `undefined`).
2. **Bug evidente del TS no replicado** (`src/generators/customiso.ts:49`):
   `lines.push(...heredoc("/etc/motd", cfg.motd))` esparce el *string* del
   heredoc y vuelca el motd un carácter por línea en `advanced/chroot.sh`.
   El Rust implementa la intención (`lines.push(heredoc(...))`, heredoc
   íntegro). Todo lo demás del modo advanced es idéntico.
3. **`forge dump` acepta los 7 formatos** (`iso|qcow2|usb|vmdk|vdi|oci|wsl`)
   como pide el spec del CLI; `scripts/dump.ts` solo aplicaba
   `iso|qcow2|oci|wsl` e ignoraba el resto en silencio. Con `usb`/`vmdk`/`vdi`
   el CLI Rust genera el proyecto KIWI correspondiente; el TS habría seguido
   con `iso`.
4. `mergeConfig`/`Object.assign` con objetos anidados parciales: el TS
   inyectaría `undefined` (y luego `"undefined"` en las recetas); el Rust
   conserva el valor por defecto del campo ausente. Con configs completas
   (las que exporta la app) el resultado es idéntico.
5. `downloadZip` (JSZip, DOM) no se porta: es código de navegador. El
   equivalente CLI es `forge dump` + zip externo.

## Dependencias

- `forge-core`: `serde`, `serde_json` (dev: `serde_yaml`). Sin `regex` ni
  `base64`: validaciones y codificación implementadas a mano.
- `forge-cli`: `forge-core`, `serde_json`.
- `forge-wasm`: `forge-core`, `serde_json`, `wasm-bindgen`.
