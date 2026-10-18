# Avisos de terceros

DistroForge se distribuye bajo doble licencia **MIT OR Apache-2.0** (ver
[LICENSE-MIT](LICENSE-MIT) y [LICENSE-APACHE](LICENSE-APACHE)). Este fichero
recoge el software de terceros que va empaquetado o que se usa en tiempo de
ejecución, con su licencia y su aviso de copyright.

## Empaquetado en el repositorio

### fflate 0.8.2

- Fichero: `rust/web-ui/public/fflate.js` (copia local, sin CDN).
- Uso: generar el ZIP del proyecto sin conexión.
- Licencia: MIT, Copyright (c) 2023 Arjun Barrett.
- Origen: <https://github.com/101arrowz/fflate>

```text
MIT License

Copyright (c) 2023 Arjun Barrett

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.
```

## Cargado en tiempo de ejecución (no se redistribuye)

### noVNC 1.5.0

- Uso: cliente VNC del Test Drive, que `GET /vnc.html` carga como módulo ESM
  desde jsDelivr (`https://cdn.jsdelivr.net/npm/@novnc/novnc@1.5.0/lib/rfb.js`).
- Licencia: MPL-2.0. Al no empaquetarse ni modificarse, solo se enlaza en el
  navegador del usuario.
- Origen: <https://github.com/novnc/noVNC>

## Dentro de la imagen Docker

La imagen `ghcr.io/cisne-047/distroforge-server` instala `qemu-system-x86` desde
los repositorios de Debian, que es **GPL-2.0**. Debian incluye los avisos y la
información de copyright en `/usr/share/doc/qemu-system-x86/copyright` dentro de
la propia imagen, y el código fuente está disponible en Debian y en
<https://www.qemu.org/>. DistroForge solo invoca el binario como proceso aparte,
no lo enlaza ni lo modifica.

## Dependencias de Rust

Las dependencias de los seis paquetes (axum, rusqlite, Leptos, Tauri, etc.) son
MIT, Apache-2.0 o BSD; la lista exacta con sus versiones está en los
`Cargo.lock`. Para regenerar un inventario completo:

```sh
cargo install cargo-about
cargo about generate about.hbs > THIRD-PARTY-RUST.html
```

## Tux

La mascota del proyecto deriva de Tux, creada por Larry Ewing. Ver
[TRADEMARK.md](TRADEMARK.md).
