"""Generates the PWA icons in apps/web/public/icons (committed; rerun only to change them).

A crosshair reticle in the hud accent on the hud background. The maskable icon
keeps the glyph inside the central 80% safe zone.
"""
from pathlib import Path

from PIL import Image, ImageDraw

BG = (7, 17, 13)
ACCENT = (77, 255, 166)
OUT = Path(__file__).resolve().parents[2] / "apps/web/public/icons"


def draw(size: int, glyph: float) -> Image.Image:
    s = 4  # supersample, then downscale for smooth edges
    n = size * s
    img = Image.new("RGB", (n, n), BG)
    d = ImageDraw.Draw(img)
    c = n / 2
    r = n * glyph / 2
    w = max(1, round(n * 0.045))
    d.ellipse([c - r, c - r, c + r, c + r], outline=ACCENT, width=w)
    tick = r * 0.55
    for x0, y0, x1, y1 in [(c, c - r - w, c, c - r + tick), (c, c + r - tick, c, c + r + w),
                           (c - r - w, c, c - r + tick, c), (c + r - tick, c, c + r + w, c)]:
        d.line([x0, y0, x1, y1], fill=ACCENT, width=w)
    dot = n * 0.05
    d.ellipse([c - dot, c - dot, c + dot, c + dot], fill=ACCENT)
    return img.resize((size, size), Image.LANCZOS)


OUT.mkdir(parents=True, exist_ok=True)
draw(192, 0.72).save(OUT / "icon-192.png", optimize=True)
draw(512, 0.72).save(OUT / "icon-512.png", optimize=True)
draw(512, 0.56).save(OUT / "maskable-512.png", optimize=True)
draw(180, 0.66).save(OUT / "apple-touch-icon.png", optimize=True)
(OUT / "icon.svg").write_text(
    '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64">'
    '<rect width="64" height="64" rx="12" fill="#07110d"/>'
    '<g stroke="#4dffa6" stroke-width="3" fill="none"><circle cx="32" cy="32" r="21"/>'
    '<path d="M32 9v12M32 43v12M9 32h12M43 32h12"/></g>'
    '<circle cx="32" cy="32" r="3" fill="#4dffa6"/></svg>\n'
)
print("icons written to", OUT)
