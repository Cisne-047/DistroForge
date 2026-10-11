import type { DistroConfig } from "../types";

export const CLOUD_INIT_PACKAGE = "cloud-init";

export function cloudInitYaml(cfg: DistroConfig): string {
  const sys = cfg.system;
  return `#cloud-config generado por DistroForge
datasource_list: [NoCloud, None]
hostname: ${sys.hostname}
timezone: ${sys.timezone}
ssh_pwauth: true
users:
  - name: ${sys.username}
    plain_text_passwd: "${sys.password.replace(/"/g, '\\"')}"
    lock_passwd: false
    shell: /bin/bash
    sudo: ALL=(ALL) ALL
`;
}
