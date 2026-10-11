import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import { BASES, defaultConfig } from "../src/catalog";
import { generateProject } from "../src/generators";
import type { DistroConfig } from "../src/types";

// Vuelca a disco el proyecto generado para una base, para probar builds reales.
// Uso: npx vite-node scripts/dump.ts [base] [dirSalida] [iso|qcow2|oci|wsl] [config.json]
// El config.json (exportado por la app) se fusiona sobre los valores por defecto.

const [, , baseArg, outArg, fmtArg, cfgArg] = process.argv;
const base = (baseArg ?? "debian") as BaseId;
if (!BASES.some((b) => b.id === base)) {
  console.error(`Base desconocida: ${base}. Bases: ${BASES.map((b) => b.id).join(", ")}`);
  process.exit(1);
}
const cfg = defaultConfig(base);
cfg.system.password = "distroforge-test";
if (fmtArg === "iso" || fmtArg === "qcow2" || fmtArg === "usb" || fmtArg === "vmdk" || fmtArg === "vdi" || fmtArg === "oci" || fmtArg === "wsl")
  cfg.output = fmtArg;
if (cfgArg) Object.assign(cfg, JSON.parse(readFileSync(cfgArg, "utf8")) as Partial<DistroConfig>);
const out = outArg ?? `dump-${base}`;
const files = generateProject(cfg);
for (const [p, content] of Object.entries(files)) {
  const full = join(out, p);
  mkdirSync(join(full, ".."), { recursive: true });
  writeFileSync(full, content);
}
console.log(`Proyecto ${base} volcado en ${out} (${Object.keys(files).length} ficheros)`);
