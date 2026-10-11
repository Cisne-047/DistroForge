import type { DistroConfig } from "./types";

/** Codifica la config para compartirla por URL (fragmento #, base64url unicode-safe). */
export function encodeConfig(cfg: DistroConfig): string {
  const json = JSON.stringify(cfg);
  const b64 = btoa(unescape(encodeURIComponent(json)));
  return b64.replace(/\+/g, "-").replace(/\//g, "_").replace(/=+$/, "");
}

/** Decodifica un fragmento de URL a config; null si es basura. */
export function decodeConfig(hash: string): DistroConfig | null {
  try {
    let b64 = hash.replace(/-/g, "+").replace(/_/g, "/");
    while (b64.length % 4) b64 += "=";
    const parsed = JSON.parse(decodeURIComponent(escape(atob(b64))));
    if (!parsed || typeof parsed.name !== "string" || typeof parsed.base !== "string")
      return null;
    return parsed as DistroConfig;
  } catch {
    return null;
  }
}

/** URL completa para compartir la config actual. */
export function shareUrl(cfg: DistroConfig): string {
  return `${location.origin}${location.pathname}#${encodeConfig(cfg)}`;
}
