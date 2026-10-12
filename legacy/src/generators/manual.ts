import type { DistroConfig } from "../types";
import { baseById, desktopPackages } from "../catalog";

export function manualFiles(cfg: DistroConfig): Record<string, string> {
  const base = baseById(cfg.base);
  const pkgs = [...desktopPackages(cfg.base, cfg.desktop), ...cfg.packages].join("\n");
  const isGentoo = cfg.base === "gentoo";
  const howto = isGentoo
    ? `1. Arranca un stage3 de Gentoo y configura el sistema base.
2. Añade los paquetes de arriba a tu \`world\` con \`emerge --ask\`.
3. Para una ISO live usa **catalyst** con un spec stage4 que incluya esos paquetes.`
    : `1. Instala Solus y añade los paquetes con \`eopkg install\`.
2. Para una ISO remasterizada, parte de la imagen oficial + chroot, o usa respin propio.`;

  const extras = cfg.extraFiles ?? [];
  const extrasSection = extras.length
    ? `\n## Ficheros extra\n\nCopia estos ficheros a mano (rutas absolutas dentro del sistema):\n\n${extras
        .map((f) => `- \`${f.path}\``)
        .join("\n")}\n`
    : "";
  const flatpakSection = cfg.flatpakApps?.length
    ? `\n## Flatpak\n\n    flatpak remote-add --if-not-exists flathub https://dl.flathub.org/repo/flathub.flatpakrepo\n    flatpak install -y flathub ${cfg.flatpakApps.join(" ")}\n`
    : "";

  const readme = `# ${cfg.name} — base ${base.label} (motor manual)

${base.label} no tiene un builder público de imágenes que DistroForge pueda lanzar en contenedor todavía. Este proyecto genera la **lista de paquetes** y notas para montarla a mano.

## Paquetes seleccionados

\`\`\`
${pkgs}
\`\`\`

## Cómo montarlo

${howto}
${extrasSection}${flatpakSection}
## Notas

- Los nombres de paquete son orientativos: ajústalos al repositorio de la distro.
- DistroForge añadirá builder propio para estas bases en el futuro.
`;

  const buildSh = `#!/usr/bin/env bash
echo "Motor manual: ${base.label} no tiene builder automático en esta versión."
echo "Revisa manual/README.md."
exit 1
`;

  return {
    "manual/packages.txt": pkgs + "\n",
    "manual/README.md": readme,
    "build.sh": buildSh,
  };
}
