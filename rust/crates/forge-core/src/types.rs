//! Tipos de configuración, portados de `src/types.ts`.
//!
//! Todos los structs derivan `serde::Serialize`/`Deserialize` con
//! `rename_all = "camelCase"` para que el JSON sea idéntico al de la app TS
//! (`distroforge.json`, hash de compartir por URL). El orden de declaración de
//! los campos replica el de las interfaces TS, porque `JSON.stringify` y
//! `serde_json` serializan los campos en ese orden.
//!
//! Al deserializar, los campos que faltan toman valores coherentes con los
//! "fallbacks" repartidos por el código TS (p. ej. `grub.timeout` por defecto
//! 10, como el `?? 10`; `network` por defecto DHCP con prefijo "24").

use serde::ser::{SerializeStruct, Serializer};
use serde::{Deserialize, Serialize};
use std::fmt;

/// Motor de construcción de la imagen (`EngineId`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum EngineId {
    /// KIWI NG (openSUSE).
    Kiwi,
    /// live-build (Debian/Ubuntu).
    Livebuild,
    /// mkarchiso (Arch y derivadas).
    Mkarchiso,
    /// Kickstart/anaconda (Fedora).
    Kickstart,
    /// Guía manual (Gentoo, Solus).
    Manual,
}

impl EngineId {
    /// Valor exacto del enum en JSON/TS.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Kiwi => "kiwi",
            Self::Livebuild => "livebuild",
            Self::Mkarchiso => "mkarchiso",
            Self::Kickstart => "kickstart",
            Self::Manual => "manual",
        }
    }
}

impl fmt::Display for EngineId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Arquitectura objetivo (`ArchId`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
pub enum ArchId {
    /// AMD64/Intel64.
    #[serde(rename = "x86_64")]
    #[default]
    X86_64,
    /// ARM 64 bits.
    #[serde(rename = "aarch64")]
    Aarch64,
    /// ARM 32 bits hard-float.
    #[serde(rename = "armhf")]
    Armhf,
    /// x86 32 bits.
    #[serde(rename = "i386")]
    I386,
}

impl ArchId {
    /// Valor exacto del enum en JSON/TS.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::X86_64 => "x86_64",
            Self::Aarch64 => "aarch64",
            Self::Armhf => "armhf",
            Self::I386 => "i386",
        }
    }
}

impl fmt::Display for ArchId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Base del sistema (`BaseId`), 21 distribuciones soportadas.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BaseId {
    /// openSUSE Tumbleweed.
    #[serde(rename = "opensuse-tumbleweed")]
    #[default]
    OpensuseTumbleweed,
    /// openSUSE Leap 15.6.
    #[serde(rename = "opensuse-leap")]
    OpensuseLeap,
    /// Debian 12 (bookworm).
    Debian,
    /// Ubuntu 24.04 (noble).
    Ubuntu,
    /// Linux Mint 22.
    Mint,
    /// elementary OS 8.
    Elementary,
    /// Pop!_OS 22.04.
    Pop,
    /// Zorin OS 18.
    Zorin,
    /// Kali Linux (rolling).
    Kali,
    /// MX Linux 23.
    Mx,
    /// Fedora 44.
    Fedora,
    /// AlmaLinux 9.
    Almalinux,
    /// Rocky Linux 9.
    Rocky,
    /// Oracle Linux 9.
    Oracle,
    /// Arch Linux.
    Arch,
    /// Manjaro.
    Manjaro,
    /// CachyOS.
    Cachyos,
    /// Garuda Linux.
    Garuda,
    /// SteamOS (Arch, solo x86_64).
    Steamos,
    /// Gentoo.
    Gentoo,
    /// Solus.
    Solus,
}

impl BaseId {
    /// Valor exacto del enum en JSON/TS.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpensuseTumbleweed => "opensuse-tumbleweed",
            Self::OpensuseLeap => "opensuse-leap",
            Self::Debian => "debian",
            Self::Ubuntu => "ubuntu",
            Self::Mint => "mint",
            Self::Elementary => "elementary",
            Self::Pop => "pop",
            Self::Zorin => "zorin",
            Self::Kali => "kali",
            Self::Mx => "mx",
            Self::Fedora => "fedora",
            Self::Almalinux => "almalinux",
            Self::Rocky => "rocky",
            Self::Oracle => "oracle",
            Self::Arch => "arch",
            Self::Manjaro => "manjaro",
            Self::Cachyos => "cachyos",
            Self::Garuda => "garuda",
            Self::Steamos => "steamos",
            Self::Gentoo => "gentoo",
            Self::Solus => "solus",
        }
    }

    /// Parsea un id de base (`FromStr`); `Err` con la lista de ids válidos
    /// implícita en [`catalog::BASES`].
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "opensuse-tumbleweed" => Self::OpensuseTumbleweed,
            "opensuse-leap" => Self::OpensuseLeap,
            "debian" => Self::Debian,
            "ubuntu" => Self::Ubuntu,
            "mint" => Self::Mint,
            "elementary" => Self::Elementary,
            "pop" => Self::Pop,
            "zorin" => Self::Zorin,
            "kali" => Self::Kali,
            "mx" => Self::Mx,
            "arch" => Self::Arch,
            "manjaro" => Self::Manjaro,
            "cachyos" => Self::Cachyos,
            "garuda" => Self::Garuda,
            "steamos" => Self::Steamos,
            "fedora" => Self::Fedora,
            "almalinux" => Self::Almalinux,
            "rocky" => Self::Rocky,
            "oracle" => Self::Oracle,
            "gentoo" => Self::Gentoo,
            "solus" => Self::Solus,
            _ => return None,
        })
    }
}

impl fmt::Display for BaseId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Formato de salida (`OutputFormat`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OutputFormat {
    /// Imagen ISO híbrida.
    #[default]
    Iso,
    /// Disco virtual qcow2 (solo KIWI).
    Qcow2,
    /// Imagen OEM para USB (solo KIWI).
    Usb,
    /// VMware VMDK (solo KIWI).
    Vmdk,
    /// VirtualBox VDI (solo KIWI).
    Vdi,
    /// Hyper-V VHD (solo KIWI).
    Vhd,
    /// Imagen de contenedor OCI (Dockerfile).
    Oci,
    /// rootfs para WSL (`wsl --import`).
    Wsl,
}

impl OutputFormat {
    /// Valor exacto del enum en JSON/TS.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Iso => "iso",
            Self::Qcow2 => "qcow2",
            Self::Usb => "usb",
            Self::Vmdk => "vmdk",
            Self::Vdi => "vdi",
            Self::Vhd => "vhd",
            Self::Oci => "oci",
            Self::Wsl => "wsl",
        }
    }

    /// Parsea un formato de salida.
    pub fn parse(s: &str) -> Option<Self> {
        Some(match s {
            "iso" => Self::Iso,
            "qcow2" => Self::Qcow2,
            "usb" => Self::Usb,
            "vmdk" => Self::Vmdk,
            "vdi" => Self::Vdi,
            "vhd" => Self::Vhd,
            "oci" => Self::Oci,
            "wsl" => Self::Wsl,
            _ => return None,
        })
    }
}

impl fmt::Display for OutputFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Target de arranque systemd (`bootTarget`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum BootTarget {
    /// Auto: gráfico si hay escritorio, multi-user si no.
    #[default]
    Auto,
    /// Forzar graphical.target.
    Graphical,
    /// Forzar multi-user.target.
    Text,
}

impl BootTarget {
    /// Valor exacto del enum en JSON/TS.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Auto => "auto",
            Self::Graphical => "graphical",
            Self::Text => "text",
        }
    }
}

impl fmt::Display for BootTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Modo de red (`NetworkConfig.mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum NetMode {
    /// DHCP por defecto.
    #[default]
    Dhcp,
    /// IP estática.
    Static,
}

/// Destino de subida a la nube tras el build (`cloudUpload`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum CloudUpload {
    /// Sin subida.
    #[default]
    None,
    /// AWS (S3 + import a AMI).
    Aws,
    /// Azure (VHD en blob).
    Azure,
}

impl CloudUpload {
    /// Valor exacto del enum en JSON/TS.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Aws => "aws",
            Self::Azure => "azure",
        }
    }
}

impl fmt::Display for CloudUpload {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Repo custom (`CustomRepo`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct CustomRepo {
    /// Alias corto del repo (se sanea en las recetas).
    pub alias: String,
    /// URL del repo (http/https).
    pub url: String,
    /// URL de la key GPG, o cadena vacía si el repo va sin firmar.
    pub gpg_key: String,
    /// "suite componentes" para repos no planos (ej. "noble main"). Vacío = repo plano (./).
    pub suite: String,
    /// Repo activo.
    pub enabled: bool,
}

/// Config del sistema (`SystemConfig`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct SystemConfig {
    /// Hostname de la imagen.
    pub hostname: String,
    /// Usuario principal.
    pub username: String,
    /// Contraseña en claro (va dentro de la receta).
    pub password: String,
    /// Zona horaria (ej. Europe/Madrid).
    pub timezone: String,
    /// Locale (ej. es_ES.UTF-8).
    pub locale: String,
    /// Distribución de teclado (ej. es).
    pub keyboard: String,
}

/// Fichero arbitrario que se copia tal cual a la imagen (`ExtraFile`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExtraFile {
    /// Ruta absoluta dentro de la imagen (ej. /etc/skel/.bashrc).
    pub path: String,
    /// Contenido literal del fichero.
    pub content: String,
}

/// Usuario adicional al principal (`ExtraUser`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ExtraUser {
    /// Nombre del usuario.
    pub name: String,
    /// Contraseña.
    pub password: String,
    /// Pertenece al grupo de admin (sudo/wheel).
    pub admin: bool,
}

/// Unidad systemd que se habilita o deshabilita (`ServiceToggle`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ServiceToggle {
    /// Nombre de la unidad (ej. sshd.service).
    pub unit: String,
    /// true = enable, false = disable.
    pub enabled: bool,
}

/// Configuración de red (`NetworkConfig`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct NetworkConfig {
    /// DHCP o estática.
    pub mode: NetMode,
    /// IPv4 (solo en modo estático).
    pub ip: String,
    /// Prefijo CIDR 8-32, como texto.
    pub prefix: String,
    /// Gateway IPv4.
    pub gateway: String,
    /// Servidores DNS IPv4.
    pub dns: Vec<String>,
}

/// Config GRUB (`GrubConfig`).
///
/// Se serializa a mano para que `timeout` salga como número entero cuando su
/// parte fraccional es cero, igual que hace `JSON.stringify` con `10` (en vez
/// del `10.0` que produciría serde con un `f64`).
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(default, rename_all = "camelCase")]
pub struct GrubConfig {
    /// Segundos que el menú espera antes de arrancar la primera entrada.
    pub timeout: f64,
    /// Parámetros extra del kernel (appended al cmdline).
    pub extra_params: String,
}

impl Default for GrubConfig {
    fn default() -> Self {
        // El TS reparte `?? 10` por todo el código: el default coherente es 10.
        Self {
            timeout: 10.0,
            extra_params: String::new(),
        }
    }
}

impl Serialize for GrubConfig {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut st = serializer.serialize_struct("GrubConfig", 2)?;
        st.serialize_field("timeout", &JsNumber(self.timeout))?;
        st.serialize_field("extraParams", &self.extra_params)?;
        st.end()
    }
}

/// Número JSON estilo JS: entero cuando `fract() == 0`, `null` para NaN/∞
/// (misma salida que `JSON.stringify`).
struct JsNumber(f64);
impl Serialize for JsNumber {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let v = self.0;
        if v.is_finite() && v.fract() == 0.0 && v.abs() < 9.0e15 {
            serializer.serialize_i64(v as i64)
        } else {
            match serde_json::Number::from_f64(v) {
                Some(n) => n.serialize(serializer),
                None => serializer.serialize_none(),
            }
        }
    }
}

/// Formatea un número como lo haría `String(n)` de JS para los valores que
/// aparecen en plantillas (timeouts): `10` en vez de `10.0`.
pub fn js_num(v: f64) -> String {
    if v.is_finite() && v.fract() == 0.0 && v.abs() < 9.0e15 {
        format!("{}", v as i64)
    } else {
        format!("{}", v)
    }
}

/// Default de `startIcon` al deserializar configs que no lo traen (el
/// `defaultConfig` del TS lo fija en "◆" y `mergeConfig` lo conservaría).
fn default_start_icon() -> String {
    "◆".to_string()
}

/// Modo advanced: remasterizado de una ISO base (`CustomIsoConfig`).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct CustomIsoConfig {
    /// URL de la ISO base descargable (http/https, termina en .iso).
    pub url: String,
    /// Script avanzado del usuario, ejecutado dentro del chroot de la ISO.
    pub chroot_script: String,
}

/// Configuración completa de la distro (`DistroConfig`).
///
/// El orden de los campos replica el objeto literal de `defaultConfig` en
/// `src/catalog.ts`, de modo que `distroforge.json` (JSON con 2 espacios de
/// indentación) es byte a byte idéntico al generado por el TS.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[serde(default, rename_all = "camelCase")]
pub struct DistroConfig {
    /// Nombre de la distro (minúsculas y guiones, 2-32).
    pub name: String,
    /// Base del sistema.
    pub base: BaseId,
    /// Id del escritorio elegido (uno de `BaseDef::desktops`).
    pub desktop: String,
    /// Paquetes extra.
    pub packages: Vec<String>,
    /// Repos custom.
    pub repos: Vec<CustomRepo>,
    /// Sistema: hostname, usuario, contraseña, zona, locale, teclado.
    pub system: SystemConfig,
    /// Formato de salida.
    pub output: OutputFormat,
    /// Apps Flatpak de Flathub (ids tipo org.mozilla.firefox).
    pub flatpak_apps: Vec<String>,
    /// Ficheros a copiar a la imagen por ruta absoluta.
    pub extra_files: Vec<ExtraFile>,
    /// Usuarios adicionales.
    pub users: Vec<ExtraUser>,
    /// Servicios systemd a habilitar/deshabilitar.
    pub services: Vec<ServiceToggle>,
    /// Red: DHCP o estática.
    pub network: NetworkConfig,
    /// Opciones del arranque GRUB.
    pub grub: GrubConfig,
    /// Firewall (firewalld) habilitado en la imagen.
    pub firewall: bool,
    /// Arquitectura objetivo.
    pub arch: ArchId,
    /// Modo advanced: remasterizar una ISO custom.
    pub advanced: bool,
    /// Datos del modo advanced (URL de la ISO y script chroot).
    pub custom_iso: CustomIsoConfig,
    /// Auto-login del usuario principal en el escritorio.
    pub autologin: bool,
    /// Programas que arrancan al iniciar sesión (entradas .desktop).
    pub autostart: Vec<String>,
    /// Puertos/servicios abiertos en el firewall (ej. "ssh", "8080/tcp").
    pub firewall_ports: Vec<String>,
    /// Target de arranque: auto, gráfico o texto.
    pub boot_target: BootTarget,
    /// Logo personalizado como data URL (png/jpeg/webp); se copia a /usr/share/distroforge.
    pub logo: String,
    /// Icono del botón inicio de la taskbar (emoji/texto corto); el logo tiene prioridad.
    #[serde(default = "default_start_icon")]
    pub start_icon: String,
    /// Texto de EULA mostrado en el primer login interactivo.
    pub eula_text: String,
    /// Debian family: ISO de instalación desatendida con preseed.
    pub unattended: bool,
    /// Incluir workflow smoke.yml que arranca la ISO en QEMU dentro de CI.
    pub smoke_test: bool,
    /// Firmar la imagen OCI con cosign keyless (OIDC de GitHub).
    pub sign_image: bool,
    /// Test Drive en CI: capturas del arranque en QEMU (solo ISO).
    pub test_drive: bool,
    /// Subir la imagen de disco a la nube tras el build (requiere secrets en el repo).
    pub cloud_upload: CloudUpload,
    /// Inyectar cloud-init (NoCloud) para VMs/cloud.
    pub cloud_init: bool,
    /// Mensaje del día (/etc/motd).
    pub motd: String,
    /// Wallpaper como data URL (png/jpeg/webp); vacío = sin wallpaper.
    pub wallpaper: String,
}

/// Config parcial (`Partial<DistroConfig>`): los campos `Option` presentes
/// sustituyen a los actuales. La usan `catalog::merge_config` (fusión profunda
/// en system/network/grub/customIso, como el TS) y el CLI para el config.json
/// opcional (asignación superficial tipo `Object.assign`).
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartialConfig {
    /// Nombre.
    pub name: Option<String>,
    /// Base.
    pub base: Option<BaseId>,
    /// Escritorio.
    pub desktop: Option<String>,
    /// Paquetes.
    pub packages: Option<Vec<String>>,
    /// Repos.
    pub repos: Option<Vec<CustomRepo>>,
    /// Sistema (parcial).
    pub system: Option<PartialSystemConfig>,
    /// Salida.
    pub output: Option<OutputFormat>,
    /// Flatpaks.
    pub flatpak_apps: Option<Vec<String>>,
    /// Ficheros extra.
    pub extra_files: Option<Vec<ExtraFile>>,
    /// Usuarios extra.
    pub users: Option<Vec<ExtraUser>>,
    /// Servicios.
    pub services: Option<Vec<ServiceToggle>>,
    /// Red (parcial).
    pub network: Option<PartialNetworkConfig>,
    /// GRUB (parcial).
    pub grub: Option<PartialGrubConfig>,
    /// Firewall.
    pub firewall: Option<bool>,
    /// Arquitectura.
    pub arch: Option<ArchId>,
    /// Modo advanced.
    pub advanced: Option<bool>,
    /// ISO custom (parcial).
    pub custom_iso: Option<PartialCustomIsoConfig>,
    /// Auto-login.
    pub autologin: Option<bool>,
    /// Autostart.
    pub autostart: Option<Vec<String>>,
    /// Puertos firewall.
    pub firewall_ports: Option<Vec<String>>,
    /// Target de arranque.
    pub boot_target: Option<BootTarget>,
    /// Logo (data URL).
    pub logo: Option<String>,
    /// Icono del botón inicio.
    pub start_icon: Option<String>,
    /// Texto de EULA.
    pub eula_text: Option<String>,
    /// Instalación desatendida.
    pub unattended: Option<bool>,
    /// Smoke test en CI.
    pub smoke_test: Option<bool>,
    /// Firmar imagen OCI con cosign.
    pub sign_image: Option<bool>,
    /// Test Drive en CI.
    pub test_drive: Option<bool>,
    /// Subida a la nube.
    pub cloud_upload: Option<CloudUpload>,
    /// cloud-init.
    pub cloud_init: Option<bool>,
    /// MOTD.
    pub motd: Option<String>,
    /// Wallpaper.
    pub wallpaper: Option<String>,
}

/// `Partial<SystemConfig>`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartialSystemConfig {
    /// Hostname.
    pub hostname: Option<String>,
    /// Usuario.
    pub username: Option<String>,
    /// Contraseña.
    pub password: Option<String>,
    /// Zona horaria.
    pub timezone: Option<String>,
    /// Locale.
    pub locale: Option<String>,
    /// Teclado.
    pub keyboard: Option<String>,
}

/// `Partial<NetworkConfig>`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartialNetworkConfig {
    /// Modo.
    pub mode: Option<NetMode>,
    /// IP.
    pub ip: Option<String>,
    /// Prefijo.
    pub prefix: Option<String>,
    /// Gateway.
    pub gateway: Option<String>,
    /// DNS.
    pub dns: Option<Vec<String>>,
}

/// `Partial<GrubConfig>`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartialGrubConfig {
    /// Timeout.
    pub timeout: Option<f64>,
    /// Parámetros extra.
    pub extra_params: Option<String>,
}

/// `Partial<CustomIsoConfig>`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PartialCustomIsoConfig {
    /// URL de la ISO base.
    pub url: Option<String>,
    /// Script chroot.
    pub chroot_script: Option<String>,
}
