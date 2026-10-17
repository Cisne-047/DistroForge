//! # forge-core
//!
//! Núcleo portable de DistroForge: port fiel a Rust del TypeScript de `src/`.
//!
//! Mapa módulo TS → módulo Rust:
//!
//! | TS | Rust |
//! |---|---|
//! | `src/types.ts` | [`types`] |
//! | `src/catalog.ts` | [`catalog`] |
//! | `src/templates.ts` | [`templates`] |
//! | `src/share.ts` | [`share`] (sin `shareUrl`, que depende del navegador) |
//! | `src/generators/*` | [`generators`] |
//!
//! Los strings generados son idénticos a los del frontend TypeScript: las
//! plantillas se portaron línea a línea. `generate_project` devuelve un
//! `BTreeMap` (iteración en orden lexicográfico) donde el TS usa un objeto con
//! orden de inserción: el contenido de cada fichero es idéntico, solo cambia el
//! orden en que se recorre el mapa (irrelevante al volcar a disco o a un zip).
//!
//! El crate es infalible en su API principal (`generate_project`, `validate`,
//! `workflow_yaml`, `readme_md` no devuelven `Result`): `BaseId` y el resto de
//! ids son enums cerrados, de modo que estados "imposibles" del TS (base
//! desconocida que haría saltar `baseById`) no pueden existir en Rust.

pub mod catalog;
pub mod generators;
pub mod security;
pub mod share;
pub mod templates;
pub mod types;
