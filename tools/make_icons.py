"""Genera el juego de iconos de DistroForge (escritorio Tauri + favicon web).

Marca: hexágono (paquete/distro) atravesado por una barra de forja.
Se dibuja a 4x y se reduce con LANCZOS para bordes limpios.

Uso:  python tools/make_icons.py
Salidas:
  rust/tauri-app/src-tauri/icons/{icon.png,32x32.png,128x128.png,128x128@2x.png,
                                 256x256.png,icon-512.png,icon.ico}
  rust/web-ui/public/{favicon.png,apple-touch-icon.png}
"""

import math
import pathlib
import sys

from PIL import Image, ImageDraw

SS = 4  # supermuestreo
BASE = 1024
ROOT = pathlib.Path(__file__).resolve().parents[1]
ICON_DIR = ROOT / "rust" / "tauri-app" / "src-tauri" / "icons"
PUBLIC = ROOT / "rust" / "web-ui" / "public"

BG_TOP = (32, 35, 40)
BG_BOTTOM = (13, 14, 17)
MARK = (240, 242, 245)
ACCENT = (31, 122, 76)  # verde "Grafito & Verde"


def lerp(a, b, t):
    return tuple(round(x + (y - x) * t) for x, y in zip(a, b))


def hexagon(cx, cy, r, rot_deg=0.0):
    return [
        (cx + r * math.cos(math.radians(rot_deg + 60 * i)),
         cy + r * math.sin(math.radians(rot_deg + 60 * i)))
        for i in range(6)
    ]


def render(size=BASE):
    s = size * SS
    img = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)

    # fondo: cuadrado redondeado con degradado vertical suave
    radius = round(s * 0.225)
    grad = Image.new("RGB", (1, s))
    gp = grad.load()
    for y in range(s):
        gp[0, y] = lerp(BG_TOP, BG_BOTTOM, y / max(1, s - 1))
    grad = grad.resize((s, s))
    mask = Image.new("L", (s, s), 0)
    ImageDraw.Draw(mask).rounded_rectangle([0, 0, s - 1, s - 1], radius=radius, fill=255)
    img.paste(grad, (0, 0), mask)

    # brillo superior continuo (sin costura)
    sheen = Image.new("L", (1, s))
    sp = sheen.load()
    for y in range(s):
        t = y / max(1, s - 1)
        sp[0, y] = 0 if t > 0.55 else round(26 * (1 - t / 0.55) ** 1.6)
    sheen = sheen.resize((s, s))
    img.paste(Image.new("RGB", (s, s), (255, 255, 255)), (0, 0),
              Image.composite(sheen, Image.new("L", (s, s), 0), mask))

    cx = cy = s / 2
    d.polygon(hexagon(cx, cy, s * 0.315, -90), outline=MARK + (255,), width=round(s * 0.042))
    inner = hexagon(cx, cy, s * 0.185, -90)
    d.polygon(inner, outline=MARK + (120,), width=round(s * 0.022))

    # barra de forja, recortada al hexágono interior
    ang = math.radians(-58)
    w, length = s * 0.085, s * 0.42
    dx, dy = math.cos(ang) * length / 2, math.sin(ang) * length / 2
    nx, ny = -math.sin(ang) * w / 2, math.cos(ang) * w / 2
    bar = Image.new("RGBA", (s, s), (0, 0, 0, 0))
    ImageDraw.Draw(bar).polygon(
        [(cx - dx + nx, cy - dy + ny), (cx + dx + nx, cy + dy + ny),
         (cx + dx - nx, cy + dy - ny), (cx - dx - nx, cy - dy - ny)],
        fill=ACCENT + (255,),
    )
    clip = Image.new("L", (s, s), 0)
    ImageDraw.Draw(clip).polygon(inner, fill=255)
    img.paste(bar, (0, 0), Image.composite(bar.split()[3], Image.new("L", (s, s), 0), clip))

    return img.resize((size, size), Image.LANCZOS)


def main():
    ICON_DIR.mkdir(parents=True, exist_ok=True)
    PUBLIC.mkdir(parents=True, exist_ok=True)
    master = render(BASE)
    master.save(ICON_DIR / "icon.png", "PNG", optimize=True)
    for name, size in (("32x32.png", 32), ("128x128.png", 128), ("128x128@2x.png", 256),
                       ("256x256.png", 256), ("icon-512.png", 512)):
        master.resize((size, size), Image.LANCZOS).save(ICON_DIR / name, "PNG", optimize=True)
    master.resize((256, 256), Image.LANCZOS).save(
        ICON_DIR / "icon.ico", "ICO",
        sizes=[(16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256)],
    )
    master.resize((64, 64), Image.LANCZOS).save(PUBLIC / "favicon.png", "PNG", optimize=True)
    master.resize((180, 180), Image.LANCZOS).save(PUBLIC / "apple-touch-icon.png", "PNG",
                                                  optimize=True)

    for f in sorted(ICON_DIR.iterdir()):
        print(f"{f.name:20} {f.stat().st_size / 1024:7.1f} KB")
    for f in ("favicon.png", "apple-touch-icon.png"):
        print(f"{f:20} {(PUBLIC / f).stat().st_size / 1024:7.1f} KB")
    return 0


if __name__ == "__main__":
    sys.exit(main())
