"""Genera los iconos de DistroForge a partir del logotipo original.

Fuente única: `docs/diseno/marca-ilustracion.png` (Tux forjando en el yunque).
De ahí salen tres cosas:

  * el icono de la app (tile crema redondeado con la ilustración completa),
    en PNG de 32 a 1024 y un .ico multirresolución para Windows;
  * el favicon y el apple-touch-icon de la web;
  * `marca-cabeza.png`, el recorte de la cabeza, que es lo que usa la cabecera
    de la aplicación: la escena entera es ilegible a 26 px, la cara no.

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
    """Caja de la cabeza, deducida del pico.

    El pico es la única zona naranja de la mitad superior (la otra mancha naranja
    es la tapa del yunque, más abajo), así que centrar el recorte en él evita
    mantener coordenadas a mano: si cambia la ilustración, el recorte sigue
    cayendo en la cara.
    """
    import numpy as np

    a = np.asarray(source.convert("RGB")).astype(int)
    width, height = source.size
    r, g, b = a[:, :, 0], a[:, :, 1], a[:, :, 2]
    orange = (r > 200) & (g > 60) & (g < 150) & (b < 70)
    rows, cols = np.where(orange[: height // 2])
    if len(rows) == 0:
        # sin pico localizable: cuadrado centrado en la parte alta
        side = min(width, height) // 2
        return (width // 2 - side // 2, 0, width // 2 + side // 2, side)

    cx, cy = float(cols.mean()), float(rows.mean())
    side = 430.0
    # la cabeza tiene más masa por encima del pico que por debajo
    left = max(0, round(cx - side * 0.47))
    top = max(0, round(cy - side * 0.60))
    return (left, top, min(width, round(left + side)), min(height, round(top + side)))


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
    art = trim_background(source)

    master = tile(art, 1024)
    master.save(ICON_DIR / "icon.png", "PNG", optimize=True)
    for name, size in (("32x32.png", 32), ("128x128.png", 128), ("128x128@2x.png", 256),
                       ("256x256.png", 256), ("icon-512.png", 512)):
        master.resize((size, size), Image.LANCZOS).save(ICON_DIR / name, "PNG", optimize=True)
    master.resize((256, 256), Image.LANCZOS).save(
        ICON_DIR / "icon.ico", "ICO",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )

    for name, size in (("favicon.png", 64), ("apple-touch-icon.png", 180)):
        master.resize((size, size), Image.LANCZOS).save(PUBLIC / name, "PNG", optimize=True)

    head = tile(source.crop(head_box(source)), 128, radius_ratio=0.26, fill=0.94)
    head.save(PUBLIC / "marca-cabeza.png", "PNG", optimize=True)

    for f in sorted(ICON_DIR.iterdir()):
        print(f"{f.name:20} {f.stat().st_size / 1024:7.1f} KB")
    for f in ("favicon.png", "apple-touch-icon.png", "marca-cabeza.png"):
        print(f"{f:20} {(PUBLIC / f).stat().st_size / 1024:7.1f} KB")
    return 0


if __name__ == "__main__":
    sys.exit(main())
