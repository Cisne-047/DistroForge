import { describe, expect, it } from "vitest";
import { load as yamlLoad } from "js-yaml";
import { BASES, defaultConfig } from "../src/catalog";
import { generateProject, validate } from "../src/generators";
import type { BaseId, DistroConfig, OutputFormat } from "../src/types";

const PNG_1PX =
  "data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVR42mNk+M9QDwADhgGAWjR9awAAAABJRU5ErkJggg==";

function outputsFor(engine: string): OutputFormat[] {
  if (engine === "kiwi") return ["iso", "qcow2", "usb", "vmdk", "vdi", "oci", "wsl"];
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
    expect(ks).toContain("user --name=ana --password=ana12345 --plaintext --groups=wheel");
    expect(ks).toContain("user --name=pepe --password=pepe12345 --plaintext");
    expect(ks).toContain('systemctl enable "sshd.service"');
    expect(ks).toContain("cat > /etc/gdm/custom.conf <<'DFEOF'");
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
