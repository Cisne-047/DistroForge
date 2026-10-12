import type { DistroConfig } from "../types";

export const CLOUD_INIT_PACKAGE = "cloud-init";

function esc(s: string): string {
  // backslash primero: si no, "c:\" acabaría como "c:\\" (YAML inválido)
  return s.replace(/\\/g, "\\\\").replace(/"/g, '\\"');
}

export function cloudInitYaml(cfg: DistroConfig): string {
  const sys = cfg.system;
  const extras = (cfg.users ?? [])
    .map(
      (u) => `  - name: ${u.name}
    plain_text_passwd: "${esc(u.password)}"
    lock_passwd: false
    shell: /bin/bash
    sudo: ${u.admin ? "ALL=(ALL) ALL" : "false"}
`
    )
    .join("");
  return `#cloud-config generado por DistroForge
datasource_list: [NoCloud, None]
hostname: ${sys.hostname}
timezone: ${sys.timezone}
ssh_pwauth: true
users:
  - name: ${sys.username}
    plain_text_passwd: "${esc(sys.password)}"
    lock_passwd: false
    shell: /bin/bash
    sudo: ALL=(ALL) ALL
${extras}`;
}
