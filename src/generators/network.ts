import type { DistroConfig, NetworkConfig } from "../types";

export const IPV4_RE =
  /^(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)(\.(25[0-5]|2[0-4]\d|1\d\d|[1-9]?\d)){3}$/;

/** La red estática si está completa y válida; null si va por DHCP o falta algo. */
export function staticNetwork(cfg: DistroConfig): NetworkConfig | null {
  const n = cfg.network;
  if (!n || n.mode !== "static" || !IPV4_RE.test(n.ip) || !IPV4_RE.test(n.gateway))
    return null;
  return n;
}

/** Máscara dotted a partir de prefijo CIDR (anaconda --netmask la quiere así). */
export function dottedMask(prefix: string | number): string {
  const p = Math.max(8, Math.min(32, Number(prefix) || 24));
  const bits = (0xffffffff << (32 - p)) >>> 0;
  return [24, 16, 8, 0].map((s) => String((bits >>> s) & 255)).join(".");
}

/** Keyfile de NetworkManager con la IP estática (live-build y KIWI). */
export function nmConnection(n: NetworkConfig): string {
  return `[connection]
id=distroforge
type=ethernet

[ipv4]
method=manual
address1=${n.ip}/${n.prefix},${n.gateway}
dns=${n.dns.join(";")};
`;
}
