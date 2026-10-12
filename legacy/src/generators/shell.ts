/** Utilidades de quoting shell compartidas por todos los generadores. */

/** Single-quote seguro para shell: 'foo' → 'foo', "it's" → 'it'\''s'. */
export function sq(s: string): string {
  return `'${s.replace(/'/g, `'\\''`)}'`;
}

/** Double-quote seguro: escapa backslash, comillas, $ y backtick. */
export function dq(s: string): string {
  return s
    .replace(/\\/g, "\\\\")
    .replace(/"/g, '\\"')
    .replace(/\$/g, "\\$")
    .replace(/`/g, "\\`");
}

/** dirname POSIX de una ruta absoluta ("/etc/x.conf" → "/etc", "/f" → "/"). */
export function dirnameOf(p: string): string {
  const d = p.replace(/\/[^/]*$/, "");
  return d || "/";
}

/** Escaping para literales dentro de unit files de systemd ('' y %%). */
export function systemdSafe(s: string): string {
  return s.replace(/'/g, `'\\''`).replace(/%/g, "%%");
}
