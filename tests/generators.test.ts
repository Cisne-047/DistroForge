import { describe, expect, it } from "vitest";
import { load as yamlLoad } from "js-yaml";
import { BASES, defaultConfig } from "../src/catalog";
import { generateProject, validate } from "../src/generators";
import type { BaseId, DistroConfig, OutputFormat } from "../src/types";

const PNG_1PX =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

function outputsFor(engine: string): OutputFormat[] {
  if (engine === "kiwi") return ["iso", "qcow2", "oci", "wsl"];
  if (engine === "manual") return ["iso"];
  return ["iso", "oci", "wsl"];
}

function cfgFor(base: BaseId): DistroConfig {
  const c = defaultConfig(base);
  c.system.password = "test1234";
  return c;
}

describe("generateProject", () => {
  it.each(BASES.map((b) => b.id))("base %s genera proyecto completo", (id) => {
    const files = generateProject(cfgFor(id));
    expect(files["README.md"]).toBeTruthy();
    expect(files[".github/workflows/build.yml"]).toBeTruthy();
    for (const [path, content] of Object.entries(files)) {
      expect(content, `${path} contiene "undefined"`).not.toContain("undefined");
      expect(content.length, `${path} casi vacío`).toBeGreaterThanOrEqual(3);
    }
  });

  it("KIWI exporta ISO y QCOW2 según el formato", () => {
    const c = cfgFor("opensuse-tumbleweed");
    expect(generateProject(c)["kiwi/config.xml"]).toContain('<type image="iso"');
    c.output = "qcow2";
    expect(generateProject(c)["kiwi/config.xml"]).toContain('image="qcow2"');
  });

  it("KIWI marca gpgcheck=false solo en repos sin key", () => {
    const c = cfgFor("opensuse-tumbleweed");
    c.repos = [
      { alias: "sinkey", url: "https://example.com/repo", gpgKey: "", suite: "", enabled: true },
      { alias: "conkey", url: "https://example.com/repo2", gpgKey: "https://example.com/key.asc", suite: "", enabled: true },
    ];
    const xml = generateProject(c)["kiwi/config.xml"];
    const lineSin = xml.split("\n").find((l) => l.includes('alias="sinkey"')) ?? "";
    const lineCon = xml.split("\n").find((l) => l.includes('alias="conkey"')) ?? "";
    expect(lineSin).toContain('gpgcheck="false"');
    expect(lineCon).not.toContain('gpgcheck="false"');
  });

  it.each(["debian", "ubuntu", "mint", "pop"] as BaseId[])("live-build %s: receta coherente", (id) => {
    const base = BASES.find((b) => b.id === id)!;
    const files = generateProject(cfgFor(id));
    const auto = files["livebuild/auto/config"];
    expect(auto).toContain(`--distribution ${base.distribution}`);
    expect(auto).toContain(`--linux-flavours ${base.lbFlavour}`);
    expect(auto).toContain(`--mirror-bootstrap ${base.mirror}`);
    expect(files["livebuild/config/package-lists/distroforge.list.chroot"]).toContain(
      base.kernelPackage!
    );
    expect(files["livebuild/config/includes.chroot/etc/hostname"]).toContain(
      "mi-distro"
    );
  });

  it("bases con repos preset generan hook apt con suite", () => {
    for (const id of ["mint", "pop"] as BaseId[]) {
      const base = BASES.find((b) => b.id === id)!;
      const files = generateProject(cfgFor(id));
      const hookPath = Object.keys(files).find((p) => /hooks\/\d+-repo-/.test(p));
      expect(hookPath, `hook de repo para ${id}`).toBeTruthy();
      const hook = files[hookPath!];
      expect(hook).toContain(`deb $SIGNED ${base.presetRepos![0].url} ${base.presetRepos![0].suite}`);
    }
  });

  it("mergePresets: cambia presets al cambiar de base y conserva los del usuario", async () => {
    const { mergePresets } = await import("../src/catalog");
    const { baseById } = await import("../src/catalog");
    const userRepo = { alias: "mio", url: "https://example.com/mio", gpgKey: "", suite: "", enabled: true };
    const trasMint = mergePresets(baseById("mint"), [userRepo]);
    expect(trasMint.some((r) => r.alias === "linuxmint")).toBe(true);
    const trasCachy = mergePresets(baseById("cachyos"), trasMint);
    expect(trasCachy.some((r) => r.alias === "cachyos")).toBe(true);
    expect(trasCachy.some((r) => r.alias === "linuxmint")).toBe(false);
    expect(trasCachy.some((r) => r.alias === "mio")).toBe(true);
  });

  it.each(["arch", "manjaro", "cachyos"] as BaseId[])("archiso %s: kernel y repos", (id) => {
    const base = BASES.find((b) => b.id === id)!;
    const files = generateProject(cfgFor(id));
    expect(files["archiso/packages.x86_64"]).toContain(base.kernelPackage!);
    const pacman = files["archiso/pacman.conf"];
    expect(pacman).toContain("[core]");
    for (const preset of base.presetRepos ?? []) {
      expect(pacman).toContain(`[${preset.alias}]`);
    }
  });

  it("kickstart fedora: usuario y repos con nogpgcheck", () => {
    const c = cfgFor("fedora");
    c.repos = [
      { alias: "mio", url: "https://example.com/rpm", gpgKey: "", suite: "", enabled: true },
    ];
    const ks = generateProject(c)["fedora/ks.cfg"];
    expect(ks).toContain("user --name=user");
    expect(ks).toContain("--nogpgcheck");
  });

  it.each(["gentoo", "solus"] as BaseId[])("manual %s: lista y guía sin builder", (id) => {
    const files = generateProject(cfgFor(id));
    expect(files["manual/packages.txt"]).toBeTruthy();
    expect(files["manual/README.md"]).toBeTruthy();
    expect(files["build.sh"]).toContain("exit 1");
    expect(files[".github/workflows/build.yml"]).toContain("Sin builder automático");
  });

  it("escape XML en nombre de distro", () => {
    const c = cfgFor("opensuse-tumbleweed");
    c.repos = [
      { alias: "a<b", url: "https://example.com/r", gpgKey: "", suite: "", enabled: true },
    ];
    const xml = generateProject(c)["kiwi/config.xml"];
    expect(xml).toContain('alias="a&lt;b"');
  });

  it("OCI: Dockerfile con imagen base, usuario y workflow docker", () => {
    const c = cfgFor("debian");
    c.output = "oci";
    const files = generateProject(c);
    expect(files["Dockerfile"]).toContain("FROM debian:bookworm");
    expect(files["Dockerfile"]).toContain("apt-get install");
    expect(files["Dockerfile"]).toContain(`USER ${c.system.username}`);
    expect(files["build.sh"]).toContain("docker || command -v podman");
    expect(files[".github/workflows/build.yml"]).toContain("docker build");
    expect(files["README.md"]).toContain("docker load");
  });

  it("OCI en openSUSE usa zypper y en Fedora dnf", () => {
    const k = cfgFor("opensuse-tumbleweed");
    k.output = "oci";
    expect(generateProject(k)["Dockerfile"]).toContain("zypper");
    const f = cfgFor("fedora");
    f.output = "oci";
    expect(generateProject(f)["Dockerfile"]).toContain("dnf install");
  });

  it("OCI en base Arch añade repos preset a pacman.conf", () => {
    const c = cfgFor("cachyos");
    c.output = "oci";
    const df = generateProject(c)["Dockerfile"];
    expect(df).toContain("FROM archlinux:latest");
    expect(df).toContain("[cachyos]");
  });

  it("cloud-init y MOTD llegan a la receta live-build", () => {
    const c = cfgFor("debian");
    c.cloudInit = true;
    c.motd = "Hola distro";
    const files = generateProject(c);
    expect(
      files["livebuild/config/includes.chroot/etc/cloud/cloud.cfg.d/80-distroforge.cfg"]
    ).toContain("hostname: mi-distro");
    expect(files["livebuild/config/package-lists/distroforge.list.chroot"]).toContain(
      "cloud-init"
    );
    expect(files["livebuild/config/includes.chroot/etc/motd"]).toContain("Hola distro");
  });

  it("cloud-init y MOTD en KIWI van por config.sh", () => {
    const c = cfgFor("opensuse-tumbleweed");
    c.cloudInit = true;
    c.motd = "SUSE motd";
    const files = generateProject(c);
    expect(files["kiwi/config.xml"]).toContain('name="cloud-init"');
    const sh = files["kiwi/config.sh"];
    expect(sh).toContain("80-distroforge.cfg");
    expect(sh).toContain("plain_text_passwd");
    expect(sh).toContain("SUSE motd");
  });

  it("OCI no disponible en bases manuales", () => {
    const c = cfgFor("gentoo");
    c.output = "oci";
    expect(validate(c).some((i) => i.includes("OCI"))).toBe(true);
  });

  it("WSL no disponible en bases manuales", () => {
    const c = cfgFor("solus");
    c.output = "wsl";
    expect(validate(c).some((i) => i.includes("WSL"))).toBe(true);
  });

  it("todos los workflows generados son YAML válido", () => {
    for (const b of BASES) {
      for (const o of outputsFor(b.engine)) {
        const c = cfgFor(b.id);
        c.output = o;
        const yml = generateProject(c)[".github/workflows/build.yml"];
        expect(() => yamlLoad(yml), `${b.id}/${o}`).not.toThrow();
      }
    }
  });

  it("WSL: rootfs con bootstrap correcto por familia y wsl.conf", () => {
    const d = cfgFor("debian");
    d.output = "wsl";
    const fd = generateProject(d);
    expect(fd["wsl/rootfs.sh"]).toContain("debootstrap --variant=minbase bookworm");
    expect(fd["wsl/rootfs.sh"]).toContain("default = user");
    expect(fd["wsl/rootfs.sh"]).toContain("systemd = true");
    expect(fd["build.sh"]).toContain("wsl/rootfs.sh");
    const a = cfgFor("cachyos");
    a.output = "wsl";
    expect(generateProject(a)["wsl/rootfs.sh"]).toContain("pacstrap");
    const u = cfgFor("ubuntu");
    u.output = "wsl";
    expect(generateProject(u)["build.sh"]).toContain("ubuntu:24.04");
    const f = cfgFor("fedora");
    f.output = "wsl";
    expect(generateProject(f)["wsl/rootfs.sh"]).toContain("--installroot=/rootfs");
  });

  it("WSL: repos preset llegan al rootfs", () => {
    const m = cfgFor("mint");
    m.output = "wsl";
    const sh = generateProject(m)["wsl/rootfs.sh"];
    expect(sh).toContain("packages.linuxmint.com");
    expect(sh).toContain("wilma main upstream import backport");
  });

  it("wallpaper: dconf en GNOME, xfconf en Xfce, nada en KDE", () => {
    const g = cfgFor("debian");
    g.desktop = "gnome";
    g.wallpaper = PNG_1PX;
    const gf = generateProject(g);
    expect(
      gf["livebuild/config/includes.chroot/usr/share/backgrounds/distroforge-wallpaper.png"]
    ).toBe(PNG_1PX);
    expect(
      gf["livebuild/config/includes.chroot/etc/dconf/db/distroforge.d/00-wallpaper"]
    ).toContain("org/gnome/desktop/background");
    expect(Object.keys(gf).some((k) => k.includes("0300-dconf"))).toBe(true);
    expect(gf["livebuild/config/package-lists/distroforge.list.chroot"]).toContain("dconf-cli");

    const x = cfgFor("debian");
    x.desktop = "xfce";
    x.wallpaper = PNG_1PX;
    expect(
      generateProject(x)[
        "livebuild/config/includes.chroot/etc/xdg/xfce4/xfconf/xfce-perchannel-xml/xfce4-desktop.xml"
      ]
    ).toContain("xfce4-desktop");

    const k = cfgFor("debian");
    k.desktop = "kde";
    k.wallpaper = PNG_1PX;
    expect(Object.keys(generateProject(k)).filter((p) => p.includes("dconf"))).toEqual([]);
  });

  it("wallpaper en Arch usa servicio dconf al arrancar", () => {
    const c = cfgFor("arch");
    c.desktop = "gnome";
    c.wallpaper = PNG_1PX;
    const files = generateProject(c);
    expect(files["archiso/airootfs/etc/systemd/system/distroforge-dconf.service"]).toContain(
      "dconf update"
    );
    expect(files["archiso/airootfs/etc/tmpfiles.d/distroforge-dconf.conf"]).toContain(
      "multi-user.target.wants"
    );
  });

  it("wallpaper en KIWI va por root overlay y config.sh", () => {
    const c = cfgFor("opensuse-tumbleweed");
    c.desktop = "gnome";
    c.wallpaper = PNG_1PX;
    const files = generateProject(c);
    expect(files["kiwi/root/usr/share/backgrounds/distroforge-wallpaper.png"]).toBe(PNG_1PX);
    expect(files["kiwi/root/etc/dconf/db/distroforge.d/00-wallpaper"]).toBeTruthy();
    expect(files["kiwi/config.sh"]).toContain("dconf update");
  });

  it("validación de wallpaper: formato, tamaño y Fedora", () => {
    const c = cfgFor("debian");
    c.wallpaper = "data:image/gif;base64,R0lGOD";
    expect(validate(c).some((i) => i.includes("formato"))).toBe(true);
    const big = cfgFor("debian");
    big.wallpaper = "data:image/png;base64," + "A".repeat(3_100_000);
    expect(validate(big).some((i) => i.includes("grande"))).toBe(true);
    const f = cfgFor("fedora");
    f.wallpaper = PNG_1PX;
    expect(validate(f).some((i) => i.includes("Fedora"))).toBe(true);
  });
});

describe("validate", () => {
  it("detecta nombre, contraseña y qcow2 inválidos", () => {
    const c = cfgFor("arch");
    c.name = "Nombre Invalido";
    c.system.password = "";
    c.output = "qcow2";
    const issues = validate(c);
    expect(issues.some((i) => i.includes("Nombre"))).toBe(true);
    expect(issues.some((i) => i.includes("contraseña"))).toBe(true);
    expect(issues.some((i) => i.includes("QCOW2"))).toBe(true);
  });

  it("rechaza repos sin http(s)", () => {
    const c = cfgFor("arch");
    c.repos = [
      { alias: "ftp-repo", url: "ftp://example.com/r", gpgKey: "", suite: "", enabled: true },
    ];
    expect(validate(c).some((i) => i.includes("http"))).toBe(true);
  });

  it("config por defecto de cada base pasa validación", () => {
    for (const b of BASES) {
      const c = { ...cfgFor(b.id), system: { ...cfgFor(b.id).system, password: "x12345678" } };
      expect(validate(c), `base ${b.id}`).toEqual([]);
    }
  });

  it("validación: rutas de ficheros extra y ids flatpak", () => {
    const c = cfgFor("debian");
    c.extraFiles = [{ path: "relative/file", content: "x" }];
    expect(validate(c).some((i) => i.includes("absoluta"))).toBe(true);
    c.extraFiles = [{ path: "/etc/../passwd", content: "x" }];
    expect(validate(c).some((i) => i.includes(".."))).toBe(true);
    c.extraFiles = [];
    c.flatpakApps = ["mal id!"];
    expect(validate(c).some((i) => i.includes("id de app"))).toBe(true);
  });
});

describe("bloque 4: flatpak y ficheros extra", () => {
  const appCfg = (base: BaseId): DistroConfig => {
    const c = cfgFor(base);
    c.flatpakApps = ["org.mozilla.firefox", "org.videolan.VLC"];
    c.extraFiles = [
      { path: "/etc/skel/.bashrc", content: "export EDITOR=vim\n" },
      { path: "/etc/issue", content: "mi distro" },
    ];
    return c;
  };

  it("live-build: paquete flatpak, hook de flathub y ficheros en includes.chroot", () => {
    const files = generateProject(appCfg("debian"));
    expect(files["livebuild/config/package-lists/distroforge.list.chroot"]).toContain("flatpak");
    const hook = files["livebuild/config/hooks/0350-flatpak.hooks.chroot"];
    expect(hook).toContain("remote-add --if-not-exists flathub");
    expect(hook).toContain("org.mozilla.firefox org.videolan.VLC");
    expect(files["livebuild/config/includes.chroot/etc/skel/.bashrc"]).toContain("EDITOR=vim");
    expect(files["livebuild/config/includes.chroot/etc/issue"]).toBe("mi distro");
  });

  it("archiso: flatpak en packages, servicio de primer arranque y ficheros en airootfs", () => {
    const files = generateProject(appCfg("arch"));
    expect(files["archiso/packages.x86_64"]).toContain("flatpak");
    const svc = files["archiso/airootfs/etc/systemd/system/distroforge-flatpak.service"];
    expect(svc).toContain("remote-add --if-not-exists flathub");
    expect(svc).toContain("org.mozilla.firefox org.videolan.VLC");
    expect(files["archiso/airootfs/etc/tmpfiles.d/distroforge-flatpak.conf"]).toContain(
      "multi-user.target.wants"
    );
    expect(files["archiso/airootfs/etc/skel/.bashrc"]).toContain("EDITOR=vim");
  });

  it("KIWI: paquete en config.xml, líneas en config.sh y ficheros en root overlay", () => {
    const files = generateProject(appCfg("opensuse-tumbleweed"));
    expect(files["kiwi/config.xml"]).toContain('name="flatpak"');
    expect(files["kiwi/config.sh"]).toContain("flatpak install");
    expect(files["kiwi/root/etc/skel/.bashrc"]).toContain("EDITOR=vim");
  });

  it("kickstart: flatpak en %packages y ficheros por heredoc en %post", () => {
    const ks = generateProject(appCfg("fedora"))["fedora/ks.cfg"];
    expect(ks).toContain("flatpak");
    expect(ks).toContain("mkdir -p /etc/skel");
    expect(ks).toContain("cat > /etc/skel/.bashrc <<'DFEOF'");
    expect(ks).toContain("export EDITOR=vim");
  });

  it("OCI: RUN de flathub y COPY de ficheros extra", () => {
    const c = appCfg("debian");
    c.output = "oci";
    const files = generateProject(c);
    expect(files["Dockerfile"]).toContain("flatpak remote-add --if-not-exists flathub");
    expect(files["Dockerfile"]).toContain("RUN mkdir -p /etc/skel /etc");
    expect(files["Dockerfile"]).toContain("COPY files/extra-0 /etc/skel/.bashrc");
    expect(files["files/extra-0"]).toContain("EDITOR=vim");
  });

  it("WSL: ficheros por heredoc en /rootfs y flatpak por chroot", () => {
    const c = appCfg("ubuntu");
    c.output = "wsl";
    const sh = generateProject(c)["wsl/rootfs.sh"];
    expect(sh).toContain("cat > /rootfs/etc/skel/.bashrc <<'DFEOF'");
    expect(sh).toContain("chroot /rootfs flatpak remote-add --if-not-exists flathub");
    expect(sh).toContain("chroot /rootfs flatpak install -y --noninteractive flathub");
    expect(sh).toContain("sudo flatpak");
  });

  it("manual: README lista ficheros extra y howto flatpak", () => {
    const readme = generateProject(appCfg("gentoo"))["manual/README.md"];
    expect(readme).toContain("/etc/skel/.bashrc");
    expect(readme).toContain("flatpak install");
  });

  it("sin flatpak ni ficheros extra: ningún rastro en el proyecto", () => {
    const files = generateProject(cfgFor("debian"));
    expect(
      Object.keys(files).filter((p) => p.includes("flatpak") || p.includes("extra-"))
    ).toEqual([]);
    expect(files["livebuild/config/package-lists/distroforge.list.chroot"]).not.toContain(
      "flatpak"
    );
  });
});
