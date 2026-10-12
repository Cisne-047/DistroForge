import { describe, expect, it } from "vitest";
import { load as yamlLoad } from "js-yaml";
import { spawnSync } from "node:child_process";
import { mkdtempSync, writeFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { BASES, defaultConfig } from "../src/catalog";
import { generateProject, validate } from "../src/generators";
import type { BaseId, DistroConfig, OutputFormat } from "../src/types";

const PNG_1PX =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

function outputsFor(engine: string): OutputFormat[] {
  if (engine === "kiwi") return ["iso", "qcow2", "usb", "vmdk", "vdi", "vhd", "oci", "wsl"];
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
    expect(df).toContain('"cachyos"'); // printf %s con el alias saneado
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
    expect(fd["wsl/rootfs.sh"]).toContain("debootstrap --variant=minbase --arch=amd64 bookworm");
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
    expect(ks).toContain("cat > /etc/skel/.bashrc <<'DFEOF0'");
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

describe("bloque 5: claves GPG por motor", () => {
  const keyedRepo = {
    alias: "mirepo",
    url: "https://example.com/r",
    gpgKey: "https://example.com/key.asc",
    suite: "main",
    enabled: true,
  };

  it("KIWI: importa la key en config.sh cuando hay key; nada cuando no", () => {
    const c = cfgFor("opensuse-tumbleweed");
    c.repos = [keyedRepo];
    expect(generateProject(c)["kiwi/config.sh"]).toContain(`rpm --import "${keyedRepo.gpgKey}"`);
    const sin = cfgFor("opensuse-tumbleweed");
    sin.repos = [{ ...keyedRepo, gpgKey: "" }];
    const sh = generateProject(sin)["kiwi/config.sh"];
    expect(sh).not.toContain("rpm --import");
  });

  it("kickstart: siempre --nogpgcheck e importa la key en %post; aviso en validate", () => {
    const c = cfgFor("fedora");
    c.repos = [keyedRepo];
    const ks = generateProject(c)["fedora/ks.cfg"];
    expect(ks).toContain("--nogpgcheck");
    expect(ks).toContain(`rpm --import "${keyedRepo.gpgKey}"`);
    expect(validate(c).some((i) => i.includes("GPG"))).toBe(true);
  });

  it("OCI Debian: descarga la key y firma el repo con signed-by", () => {
    const c = cfgFor("debian");
    c.output = "oci";
    c.repos = [keyedRepo];
    const df = generateProject(c)["Dockerfile"];
    expect(df).toContain(`wget -qO /usr/share/keyrings/mirepo.asc ${keyedRepo.gpgKey}`);
    expect(df).toContain("deb [signed-by=/usr/share/keyrings/mirepo.asc] https://example.com/r main");
  });

  it("OCI Debian sin key: sigue trusted=yes", () => {
    const c = cfgFor("debian");
    c.output = "oci";
    c.repos = [{ ...keyedRepo, gpgKey: "" }];
    expect(generateProject(c)["Dockerfile"]).toContain("deb [trusted=yes] https://example.com/r main");
  });

  it("WSL Debian: key descargada al rootfs y repo firmado", () => {
    const c = cfgFor("debian");
    c.output = "wsl";
    c.repos = [keyedRepo];
    const sh = generateProject(c)["wsl/rootfs.sh"];
    expect(sh).toContain(`wget -qO /rootfs/usr/share/keyrings/mirepo.asc ${keyedRepo.gpgKey}`);
    expect(sh).toContain("deb [signed-by=/usr/share/keyrings/mirepo.asc] https://example.com/r main");
    expect(sh).toContain("apt-get install -y debootstrap wget ca-certificates");
  });

  it("validate: avisa de TrustAll en Arch con key GPG", () => {
    const c = cfgFor("arch");
    c.repos = [keyedRepo];
    expect(validate(c).some((i) => i.includes("TrustAll"))).toBe(true);
  });
});

describe("bloque 6: paridad SUSE Studio (usuarios, servicios, autologin, formatos)", () => {
  const studioCfg = (base: BaseId): DistroConfig => {
    const c = cfgFor(base);
    c.users = [
      { name: "ana", password: "ana12345", admin: true },
      { name: "pepe", password: "pepe12345", admin: false },
    ];
    c.services = [
      { unit: "sshd.service", enabled: true },
      { unit: "bluetooth.service", enabled: false },
    ];
    c.autologin = true;
    return c;
  };

  it("live-build: usuarios extra en hook, hook de servicios y autologin solo con DM", () => {
    const files = generateProject(studioCfg("debian"));
    const hook = files["livebuild/config/hooks/0100-distroforge-user.hooks.chroot"];
    expect(hook).toContain('useradd -m -s /bin/bash -G sudo "ana"');
    expect(hook).toContain('useradd -m -s /bin/bash "pepe"');
    const svc = files["livebuild/config/hooks/0340-services.hooks.chroot"];
    expect(svc).toContain('systemctl enable "sshd.service"');
    expect(svc).toContain('systemctl disable "bluetooth.service"');
    // escritorio "none" no tiene DM: sin fichero de autologin
    expect(Object.keys(files).some((p) => p.includes("autologin"))).toBe(false);
    const xf = studioCfg("debian");
    xf.desktop = "xfce";
    expect(generateProject(xf)["livebuild/config/includes.chroot/etc/lightdm/lightdm.conf"])
      .toContain("autologin-user=user");
  });

  it("archiso: unidades oneshot de usuarios y servicios con tmpfiles", () => {
    const files = generateProject(studioCfg("arch"));
    expect(files["archiso/airootfs/etc/systemd/system/distroforge-users.service"]).toContain(
      'useradd -m -s /bin/bash -G wheel "ana"'
    );
    expect(files["archiso/airootfs/etc/systemd/system/distroforge-services.service"]).toContain(
      'systemctl enable --now "sshd.service"'
    );
    expect(files["archiso/airootfs/etc/tmpfiles.d/distroforge-users.conf"]).toContain(
      "multi-user.target.wants"
    );
  });

  it("KIWI: usuarios en XML, servicios en config.sh y formatos oem/vmx", () => {
    const c = studioCfg("opensuse-tumbleweed");
    const xml = generateProject(c)["kiwi/config.xml"];
    expect(xml).toContain('name="ana"');
    expect(xml).toContain('groups="users,wheel"');
    expect(generateProject(c)["kiwi/config.sh"]).toContain('systemctl enable "sshd.service"');
    c.output = "usb";
    expect(generateProject(c)["kiwi/config.xml"]).toContain('image="oem"');
    c.output = "vmdk";
    expect(generateProject(c)["kiwi/config.xml"]).toContain('image="vmx"');
    expect(generateProject(c)["kiwi/config.xml"]).toContain('format="vmdk"');
    c.output = "vdi";
    expect(generateProject(c)["kiwi/config.xml"]).toContain('format="vdi"');
  });

  it("kickstart: usuarios nativos, servicios y autologin GDM en %post", () => {
    const c = studioCfg("fedora");
    c.desktop = "gnome";
    const ks = generateProject(c)["fedora/ks.cfg"];
    expect(ks).toContain('user --name=ana --password="ana12345" --plaintext --groups=wheel');
    expect(ks).toContain('user --name=pepe --password="pepe12345" --plaintext');
    expect(ks).toContain('systemctl enable "sshd.service"');
    expect(ks).toContain("cat > /etc/gdm/custom.conf <<'DFEOF0'");
  });

  it("OCI y WSL: usuarios extra y servicios con chroot", () => {
    const c = studioCfg("debian");
    c.output = "oci";
    const df = generateProject(c)["Dockerfile"];
    expect(df).toContain('useradd -m -s /bin/bash -G sudo "ana"');
    expect(df).toContain("ana:ana12345");
    c.output = "wsl";
    const sh = generateProject(c)["wsl/rootfs.sh"];
    expect(sh).toContain('chroot /rootfs useradd -m -s /bin/bash -G wheel "ana"');
    expect(sh).toContain('chroot /rootfs systemctl enable "sshd.service"');
  });

  it("cloud-init: usuarios extra en el YAML con sudo según admin", () => {
    const c = studioCfg("debian");
    c.cloudInit = true;
    const yml =
      generateProject(c)["livebuild/config/includes.chroot/etc/cloud/cloud.cfg.d/80-distroforge.cfg"];
    expect(yml).toContain("- name: ana");
    expect(yml).toContain("sudo: ALL=(ALL) ALL");
    expect(yml).toContain("- name: pepe");
    expect(yml).toContain("sudo: false");
  });

  it("validate: usuarios repetidos, inválidos, sin contraseña y unidad inválida", () => {
    const c = cfgFor("debian");
    c.users = [
      { name: "user", password: "x1234567", admin: false },
      { name: "Ana", password: "x1234567", admin: false },
      { name: "ok2", password: "", admin: false },
    ];
    const issues = validate(c);
    expect(issues.some((i) => i.includes("repetido"))).toBe(true);
    expect(issues.some((i) => i.includes("inválido"))).toBe(true);
    expect(issues.some((i) => i.includes("contraseña"))).toBe(true);
    const s = cfgFor("debian");
    s.services = [{ unit: "mal servicio!", enabled: true }];
    expect(validate(s).some((i) => i.includes("unidad"))).toBe(true);
  });

  it("formatos USB/VMDK/VDI solo con motor KIWI y YAML de CI válido", () => {
    for (const o of ["usb", "vmdk", "vdi"] as OutputFormat[]) {
      const c = cfgFor("debian");
      c.output = o;
      expect(validate(c).some((i) => i.includes("KIWI"))).toBe(true);
    }
    for (const b of BASES) {
      for (const o of outputsFor(b.engine)) {
        const c = cfgFor(b.id);
        c.output = o;
        expect(() => yamlLoad(generateProject(c)[".github/workflows/build.yml"]), `${b.id}/${o}`)
          .not.toThrow();
      }
    }
  });
});

describe("bloque 7: red estática", () => {
  const netCfg = (base: BaseId): DistroConfig => {
    const c = cfgFor(base);
    c.network = {
      mode: "static",
      ip: "192.168.1.50",
      prefix: "24",
      gateway: "192.168.1.1",
      dns: ["1.1.1.1", "8.8.8.8"],
    };
    return c;
  };

  it("kickstart: línea network estática con máscara dotted; dhcp por defecto", () => {
    const ks = generateProject(netCfg("fedora"))["fedora/ks.cfg"];
    expect(ks).toContain(
      "network --bootproto=static --ip=192.168.1.50 --netmask=255.255.255.0 --gateway=192.168.1.1 --nameserver=1.1.1.1,8.8.8.8"
    );
    expect(generateProject(cfgFor("fedora"))["fedora/ks.cfg"]).toContain("--bootproto=dhcp");
  });

  it("live-build: keyfile NM con chmod 600 en build; sin rastro en DHCP", () => {
    const files = generateProject(netCfg("debian"));
    const keyfile =
      files["livebuild/config/includes.chroot/etc/NetworkManager/system-connections/distroforge.nmconnection"];
    expect(keyfile).toContain("address1=192.168.1.50/24,192.168.1.1");
    expect(keyfile).toContain("dns=1.1.1.1;8.8.8.8;");
    expect(Object.keys(files).some((p) => p.includes("0330-distroforge-net"))).toBe(true);
    expect(
      Object.keys(generateProject(cfgFor("debian"))).filter((p) => p.includes("nmconnection"))
    ).toEqual([]);
  });

  it("KIWI: keyfile en root overlay y chmod en config.sh", () => {
    const files = generateProject(netCfg("opensuse-tumbleweed"));
    expect(
      files["kiwi/root/etc/NetworkManager/system-connections/distroforge.nmconnection"]
    ).toContain("address1=192.168.1.50/24,192.168.1.1");
    expect(files["kiwi/config.sh"]).toContain(
      "chmod 600 /etc/NetworkManager/system-connections/distroforge.nmconnection"
    );
  });

  it("validate: IP, prefijo, gateway y DNS inválidos; aviso Arch", () => {
    const c = netCfg("debian");
    c.network.ip = "999.1.1.1";
    expect(validate(c).some((i) => i.includes("IP estática"))).toBe(true);
    c.network.ip = "192.168.1.50";
    c.network.prefix = "7";
    expect(validate(c).some((i) => i.includes("prefijo"))).toBe(true);
    c.network.prefix = "24";
    c.network.dns = ["no-es-ip"];
    expect(validate(c).some((i) => i.includes("DNS"))).toBe(true);
    c.network.dns = ["1.1.1.1"];
    expect(validate(c)).toEqual([]);
    const a = netCfg("arch");
    expect(validate(a).some((i) => i.includes("ISO live"))).toBe(true);
  });
});

describe("bloques 8-11: firewall, share, plantillas, arch, grub, advanced", () => {
  it("firewall: firewalld en paquetes y servicio; kickstart --enabled/--disabled", () => {
    const on = cfgFor("debian");
    const f = generateProject(on);
    expect(f["livebuild/config/package-lists/distroforge.list.chroot"]).toContain("firewalld");
    expect(f["livebuild/config/hooks/0340-services.hooks.chroot"]).toContain(
      'systemctl enable "firewalld"'
    );
    expect(generateProject(cfgFor("fedora"))["fedora/ks.cfg"]).toContain("firewall --enabled");
    const off = cfgFor("fedora");
    off.firewall = false;
    expect(generateProject(off)["fedora/ks.cfg"]).toContain("firewall --disabled");
    expect(generateProject(cfgFor("opensuse-tumbleweed"))["kiwi/config.xml"]).toContain(
      'name="firewalld"'
    );
  });

  it("round-trip: distroforge.json en el proyecto y reimportable", () => {
    const c = cfgFor("debian");
    c.users = [{ name: "ana", password: "ana12345", admin: true }];
    const parsed = JSON.parse(generateProject(c)["distroforge.json"]);
    expect(parsed.name).toBe("mi-distro");
    expect(parsed.users[0].name).toBe("ana");
    expect(validate(parsed)).toEqual([]);
  });

  it("share: encode/decode round-trip unicode; basura a null", async () => {
    const { encodeConfig, decodeConfig } = await import("../src/share");
    const c = cfgFor("debian");
    c.system.password = "contraseña-ñ-123";
    const decoded = decodeConfig(encodeConfig(c));
    expect(decoded?.system.password).toBe(c.system.password);
    expect(decoded?.users).toEqual([]);
    expect(decodeConfig("!!!basura!!!")).toBeNull();
  });

  it("plantillas: applyTemplate respeta los escritorios de la base", async () => {
    const { applyTemplate, TEMPLATES } = await import("../src/templates");
    const kiosk = TEMPLATES.find((t) => t.id === "kiosk")!;
    const mint = applyTemplate(cfgFor("mint"), kiosk);
    expect(mint.autologin).toBe(true);
    expect(mint.desktop).toBe("none"); // mint no tiene "gnome": se conserva el escritorio actual
    expect(applyTemplate(cfgFor("arch"), kiosk).desktop).toBe("gnome");
    const wsl = applyTemplate(cfgFor("debian"), TEMPLATES.find((t) => t.id === "wsldev")!);
    expect(wsl.output).toBe("wsl");
  });

  it("OCI: workflow con SBOM (syft) y escaneo CVE (trivy)", () => {
    const c = cfgFor("debian");
    c.output = "oci";
    const yml = generateProject(c)[".github/workflows/build.yml"];
    expect(yml).toContain("anchore/sbom-action");
    expect(yml).toContain("aquasecurity/trivy-action");
  });

  it("QA de sintaxis: todos los .sh generados pasan bash -n", () => {
    const dir = mkdtempSync(join(tmpdir(), "df-shcheck-"));
    try {
      for (const b of BASES) {
        for (const o of outputsFor(b.engine)) {
          const c = cfgFor(b.id);
          c.output = o;
          for (const [p, content] of Object.entries(generateProject(c))) {
            if (!p.endsWith(".sh") && !p.endsWith(".chroot") && !p.endsWith(".hooks.chroot"))
              continue;
            const file = join(dir, "script");
            writeFileSync(file, content);
            const r = spawnSync("bash", ["-n", file], { encoding: "utf8" });
            expect(r.status, `${b.id}/${o}: ${p} — ${r.stderr}`).toBe(0);
          }
        }
      }
    } finally {
      rmSync(dir, { recursive: true, force: true });
    }
  });

  it("arquitecturas: live-build/debootstrap mapean, CI usa arm-runner y QEMU, validate limita", () => {
    const a = cfgFor("debian");
    a.arch = "aarch64";
    expect(generateProject(a)["livebuild/auto/config"]).toContain("--architectures arm64");
    const f = cfgFor("fedora");
    f.arch = "aarch64";
    expect(generateProject(f)["fedora/ks.cfg"]).toContain("arch=aarch64");
    const armhf = cfgFor("debian");
    armhf.arch = "armhf";
    expect(generateProject(armhf)[".github/workflows/build.yml"]).toContain(
      "setup-qemu-action"
    );
    const k = cfgFor("opensuse-tumbleweed");
    k.arch = "aarch64";
    expect(generateProject(k)[".github/workflows/build.yml"]).toContain("ubuntu-24.04-arm");
    const w = cfgFor("ubuntu");
    w.output = "wsl";
    w.arch = "aarch64";
    expect(generateProject(w)["wsl/rootfs.sh"]).toContain("--arch=arm64");
    const ar = cfgFor("arch");
    ar.arch = "aarch64";
    expect(validate(ar).some((i) => i.includes("no soportada"))).toBe(true);
  });

  it("GRUB: timeout y parámetros del kernel en los cuatro motores", () => {
    const g = (c: DistroConfig) => {
      c.grub = { timeout: 7, extraParams: "quiet splash vt.global_cursor_default=0" };
      return c;
    };
    const lb = generateProject(g(cfgFor("debian")));
    expect(lb["livebuild/auto/config"]).toContain("--grub-timeout 7");
    expect(lb["livebuild/auto/config"]).toContain("vt.global_cursor_default=0");
    const ks = generateProject(g(cfgFor("fedora")))["fedora/ks.cfg"];
    expect(ks).toContain("--timeout=7");
    expect(ks).toContain('--append="quiet splash vt.global_cursor_default=0"');
    const ki = generateProject(g(cfgFor("opensuse-tumbleweed")))["kiwi/config.xml"];
    expect(ki).toContain('kernelcmdline="quiet splash vt.global_cursor_default=0"');
    expect(ki).toContain('<bootloader name="grub2" timeout="7"/>');
    const ar = generateProject(g(cfgFor("arch")));
    expect(ar["build.sh"]).toContain("set timeout=7");
    expect(ar["build.sh"]).toContain("vt.global_cursor_default=0");
  });

  it("advanced: remaster con chroot del usuario, workflow y validación de URL", () => {
    const c = cfgFor("debian");
    c.advanced = true;
    c.customIso = { url: "https://example.com/exotic.iso", chrootScript: "apt install -y htop" };
    const files = generateProject(c);
    expect(files["advanced/download.sh"]).toContain("wget -c");
    const chroot = files["advanced/chroot.sh"];
    expect(chroot).toContain('useradd -m -s /bin/bash -G sudo "user"');
    expect(chroot).toContain("apt install -y htop");
    // el motd va integro en un heredoc (regresion: se volcaba caracter a caracter)
    expect(chroot).toContain("<<'DFEOF'");
    expect(chroot).toContain("Bienvenido a tu distro personalizada");
    expect(files[".github/workflows/build.yml"]).toContain("remaster.sh");
    expect(validate(c)).toEqual([]);
    c.customIso.url = "ftp://mala-url";
    expect(validate(c).some((i) => i.includes("Advanced"))).toBe(true);
  });

  it("config por defecto de cada base sigue pasando validación", () => {
    for (const b of BASES) {
      const c = defaultConfig(b.id);
      c.system.password = "x12345678";
      expect(validate(c), `base ${b.id}`).toEqual([]);
    }
  });
});

describe("bloque 12b: autostart, puertos firewall y target de arranque", () => {
  it("live-build: autostart .desktop, firewall-cmd y set-default", () => {
    const c = cfgFor("debian");
    c.autostart = ["firefox", "konsole"];
    c.firewallPorts = ["ssh", "8080/tcp"];
    c.bootTarget = "text";
    const f = generateProject(c);
    expect(Object.keys(f).some((p) => p.includes("/.config/autostart/") && p.includes("firefox"))).toBe(true);
    const hook = f["livebuild/config/hooks/0340-services.hooks.chroot"];
    expect(hook).toContain("firewall-cmd --permanent --add-service=ssh");
    expect(hook).toContain("firewall-cmd --permanent --add-port=8080/tcp");
    expect(hook).toContain("systemctl set-default multi-user.target");
  });

  it("kickstart: --port/--service del firewall; archiso: set-default gráfico", () => {
    const k = cfgFor("fedora");
    k.firewallPorts = ["ssh", "8080/tcp"];
    expect(generateProject(k)["fedora/ks.cfg"]).toContain(
      "firewall --enabled --service=ssh --port=8080/tcp"
    );
    const a = cfgFor("arch");
    a.bootTarget = "graphical";
    expect(
      generateProject(a)["archiso/airootfs/etc/systemd/system/distroforge-services.service"]
    ).toContain("set-default graphical.target");
  });
});

describe("bloque 15: modpacks estilo Prism", () => {
  it("applyPack añade paquetes y flatpaks sin duplicar; respeta escritorio", async () => {
    const { applyPack, PACKS } = await import("../src/packs");
    const c = cfgFor("mint"); // mint no tiene kde
    c.packages = ["steam"];
    c.flatpakApps = ["com.valvesoftware.Steam"];
    const gaming = PACKS.find((p) => p.id === "gaming")!;
    const out = applyPack(c, gaming);
    expect(out.packages.filter((p) => p === "steam").length).toBe(1);
    expect(out.packages).toContain("lutris");
    expect(out.flatpakApps.filter((a) => a === "com.valvesoftware.Steam").length).toBe(1);
    expect(out.flatpakApps).toContain("net.lutris.Lutris");
    expect(validate(out)).toEqual([]);
  });

  it("parsePack acepta JSON plano, envuelto en {pack} y rechaza basura", async () => {
    const { parsePack } = await import("../src/packs");
    const flat = parsePack('{"name":"mio","packages":["vim"]}');
    expect(flat.packages).toEqual(["vim"]);
    const wrapped = parsePack('{"pack":{"name":"mio2","packages":["git"],"flatpakApps":["org.x.Y"]}}');
    expect(wrapped.flatpakApps).toEqual(["org.x.Y"]);
    expect(() => parsePack('{"nombre":"x"}')).toThrow(/packages/);
  });
});

describe("bloque 16: logo, EULA, desatendida, VHD, smoke, firma, galería, tamaño", () => {
  const PNG =
    "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

  it("logo: binario en /usr/share/distroforge + greeter LightDM; aviso de formato", () => {
    const c = cfgFor("debian");
    c.logo = PNG;
    const f = generateProject(c);
    expect(f["livebuild/config/includes.chroot/usr/share/distroforge/logo.png"]).toBe(PNG);
    expect(
      f["livebuild/config/includes.chroot/etc/lightdm/lightdm-gtk-greeter.conf.d/50-distroforge-logo.conf"]
    ).toContain("logo=/usr/share/distroforge/logo.png");
    const bad = cfgFor("debian");
    bad.logo = "data:image/gif;base64,R0lGOD";
    expect(validate(bad).some((i) => i.includes("Logo"))).toBe(true);
  });

  it("EULA: fichero + profile.d que la muestra una sola vez", () => {
    const c = cfgFor("debian");
    c.eulaText = "Al usar esta distro aceptas que...";
    const f = generateProject(c);
    expect(f["livebuild/config/includes.chroot/etc/distroforge/EULA"]).toContain("aceptas");
    const sh = f["livebuild/config/includes.chroot/etc/profile.d/zz-distroforge-eula.sh"];
    expect(sh).toContain("eula-shown");
    expect(sh).toContain("[ -t 0 ]");
  });

  it("desatendida: preseed + bootappend-install solo en familia Debian", () => {
    const c = cfgFor("debian");
    c.unattended = true;
    const f = generateProject(c);
    expect(f["livebuild/auto/config"]).toContain("--debian-installer live");
    const pre = f["livebuild/config/includes.installer/preseed.cfg"];
    expect(pre).toContain("passwd/username string user");
    expect(pre).toContain("time/zone string Europe/Madrid");
    const k = cfgFor("fedora");
    k.unattended = true;
    expect(validate(k).some((i) => i.includes("preseed"))).toBe(true);
  });

  it("VHD disponible solo con KIWI y YAML de CI válido", () => {
    const k = cfgFor("opensuse-tumbleweed");
    k.output = "vhd";
    expect(generateProject(k)["kiwi/config.xml"]).toContain('format="vhd"');
    const d = cfgFor("debian");
    d.output = "vhd";
    expect(validate(d).some((i) => i.includes("VHD"))).toBe(true);
  });

  it("smoke.yml de arranque QEMU solo en ISO, YAML válido; cosign en OCI al firmar", () => {
    const c = cfgFor("debian");
    c.smokeTest = true;
    const files = generateProject(c);
    const smoke = files[".github/workflows/smoke.yml"];
    expect(smoke).toContain("qemu-system-x86_64");
    expect(() => yamlLoad(smoke)).not.toThrow();
    const off = cfgFor("debian");
    expect(files[".github/workflows/smoke.yml"] ?? off ? undefined : undefined);
    expect(generateProject(off)[".github/workflows/smoke.yml"]).toBeUndefined();
    const o = cfgFor("debian");
    o.output = "oci";
    o.signImage = true;
    const yml = generateProject(o)[".github/workflows/build.yml"];
    expect(yml).toContain("cosign sign --yes");
    expect(() => yamlLoad(yml)).not.toThrow();
  });

  it("estimateSizeMB crece con escritorio, paquetes y flatpaks", async () => {
    const { estimateSizeMB } = await import("../src/generators");
    const base = estimateSizeMB(cfgFor("debian"));
    const conDesk = cfgFor("debian");
    conDesk.desktop = "gnome";
    const conPaks = cfgFor("debian");
    conPaks.packages = Array(20).fill("paquete");
    const conFlat = cfgFor("debian");
    conFlat.flatpakApps = ["a.b.C", "d.e.F"];
    expect(estimateSizeMB(conDesk)).toBeGreaterThan(base);
    expect(estimateSizeMB(conPaks)).toBeGreaterThan(base);
    expect(estimateSizeMB(conFlat)).toBeGreaterThan(base);
  });

  it("galería: los packs del repo parsean con parsePack", async () => {
    const { parsePack } = await import("../src/packs");
    const { readFileSync, readdirSync } = await import("node:fs");
    for (const f of readdirSync("gallery/packs")) {
      const pk = parsePack(readFileSync(`gallery/packs/${f}`, "utf8"));
      expect(pk.name.length).toBeGreaterThan(0);
      expect(pk.packages.length).toBeGreaterThan(0);
    }
  });
});
