"""Genera los iconos de DistroForge a partir del logotipo original.

Fuente única: `docs/diseno/marca-ilustracion.png` (Tux forjando en el yunque).

La marca es **la cabeza de Tux centrada en la cara**, y de ella salen el icono de
la aplicación (PNG de 32 a 1024 y un .ico multirresolución para Windows), el
favicon, el apple-touch-icon y `marca-cabeza.png`, que es lo que usa la cabecera
de la interfaz. La escena completa se queda como mascota del proyecto.

Uso:  python tools/make_icons.py
"""

import pathlib
import sys

from PIL import Image, ImageChops, ImageDraw

ROOT = pathlib.Path(__file__).resolve().parents[1]
ILLUS = ROOT / "docs" / "diseno" / "marca-ilustracion.png"
ICON_DIR = ROOT / "rust" / "tauri-app" / "src-tauri" / "icons"
PUBLIC = ROOT / "rust" / "web-ui" / "public"

# Fondo propio de la ilustración: el icono respeta el original en vez de
# recolorearlo, así que el tile va en crema y no en el grafito de la interfaz.
CREAM = (253, 250, 240)
THRESHOLD = 26


def head_box(source):
    """Caja cuadrada centrada en la CARA de Tux, no en el pico ni en la escena.

    Dos errores que ya se cometieron y no hay que repetir:

    * centrar en el pico: el pico está a la derecha de la cara, así que el
      recorte deja la cara desplazada;
    * centrar la escena completa (pingüino + yunque): la ilustración es casi
      cuadrada pero el yunque ocupa la mitad derecha, de modo que el pingüino
      aparece pegado al borde izquierdo.

    La cara es la única zona blanca pura, y restringirla a la mitad superior
    evita que la barriga (también blanca) arrastre el centro hacia abajo. A
    26 px, además, la cara es lo único que se lee.
    """
    import numpy as np

    a = np.asarray(source.convert("RGB")).astype(int)
    height, width = source.size
    r, g, b = a[:, :, 0], a[:, :, 1], a[:, :, 2]
    # blanco puro de la cara (el fondo crema tiene b=240, así que no entra)
    face = (r > 248) & (g > 248) & (b > 246)
    face[int(height * 0.45):, :] = False
    rows, cols = np.where(face)

    side = 460.0
    if len(rows) == 0:
        return (width // 2 - int(side) // 2, 0, width // 2 + int(side) // 2, int(side))

    cx, cy = float(cols.mean()), float(rows.mean())
    # un poco por encima del centro de la cara: la cabeza pesa más arriba
    cy -= side * 0.04
    left = min(max(0, round(cx - side / 2)), width - int(side))
    top = min(max(0, round(cy - side / 2)), height - int(side))
    return (left, top, left + int(side), top + int(side))


def trim_background(im):
    """Quita el margen de fondo crema que rodea al dibujo."""
    bg = Image.new("RGB", im.size, CREAM)
    diff = ImageChops.difference(im.convert("RGB"), bg).convert("L")
    bbox = diff.point(lambda p: 255 if p > THRESHOLD else 0).getbbox()
    return im.crop(bbox) if bbox else im


def tile(art, size, radius_ratio=0.225, fill=0.84):
    """Compone el dibujo sobre un cuadrado crema de esquinas redondeadas."""
    s = size
    out = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    mask = Image.new("L", (s, s), 0)
    ImageDraw.Draw(mask).rounded_rectangle(
        [0, 0, s - 1, s - 1], radius=max(1, round(s * radius_ratio)), fill=255
    )
    out.paste(Image.new("RGB", (s, s), CREAM), (0, 0), mask)

    target = s * fill
    w, h = art.size
    ratio = target / max(w, h)
    scaled = art.convert("RGBA").resize(
        (max(1, round(w * ratio)), max(1, round(h * ratio))), Image.LANCZOS
    )
    out.alpha_composite(scaled, ((s - scaled.width) // 2, (s - scaled.height) // 2))
    return out


def main():
    if not ILLUS.exists():
        print(f"falta la ilustración: {ILLUS}", file=sys.stderr)
        return 1

    ICON_DIR.mkdir(parents=True, exist_ok=True)
    PUBLIC.mkdir(parents=True, exist_ok=True)

    source = Image.open(ILLUS).convert("RGB")

    # La marca es la cabeza centrada en la cara; la ilustración completa (el
    # pingüino forjando) se queda como mascota del proyecto en docs/diseno.
    # Usar la escena entera como icono dejaba al pingüino pegado al borde
    # izquierdo, porque el yunque ocupa la mitad derecha del dibujo.
    master = tile(source.crop(head_box(source)), 1024, fill=0.88)
    master.save(ICON_DIR / "icon.png", "PNG", optimize=True)
    for name, size in (("32x32.png", 32), ("128x128.png", 128), ("128x128@2x.png", 256),
                       ("256x256.png", 256), ("icon-512.png", 512)):
        master.resize((size, size), Image.LANCZOS).save(ICON_DIR / name, "PNG", optimize=True)
    master.resize((256, 256), Image.LANCZOS).save(
        ICON_DIR / "icon.ico", "ICO",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )

    for name, size in (("favicon.png", 64), ("apple-touch-icon.png", 180), ("marca-cabeza.png", 128)):
        master.resize((size, size), Image.LANCZOS).save(PUBLIC / name, "PNG", optimize=True)

    for f in sorted(ICON_DIR.iterdir()):
        print(f"{f.name:20} {f.stat().st_size / 1024:7.1f} KB")
    for f in ("favicon.png", "apple-touch-icon.png", "marca-cabeza.png"):
        print(f"{f:20} {(PUBLIC / f).stat().st_size / 1024:7.1f} KB")
    return 0


if __name__ == "__main__":
    sys.exit(main())
