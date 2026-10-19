#!/usr/bin/env python3
"""지형 텍스처 툰 리그레이드 — 레퍼런스(따뜻한 카툰 팔레트)로 보정.

원본은 건드리지 않고 파라미터만으로 결정론 재현(커밋된 산출물의 정본).
대상: assets/textures/ground/*.png (512×512)
  - grass  : hue +4°·채도 ×1.10·명도 ×1.04  → #87AA46 → #8FBC5A 근사
  - forest : hue +5°·채도 ×1.06·명도 ×1.16  → #577F35 → #7FA84A 근사(캐노피)
  - rock   : RGB 웜 시프트(r×1.06,g×1.02,b×0.94) → #89847B → #B5A98A 방향
  - sand   : r×1.03·명도 ×1.02 (이미 웜탄 — 미세)
  - dirt   : 무변경
"""
import sys
from PIL import Image

PARAMS = {
    "grass": dict(hue=-6, sat=0.98, val=1.08),
    "forest": dict(hue=10, sat=1.02, val=1.42),
    "sand": dict(rgb=(1.03, 1.0, 0.99), val=1.02),
    "rock": dict(rgb=(1.15, 1.05, 0.88), val=1.08),
    "dirt": {},
}


def regrade(path, hue=0, sat=1.0, val=1.0, rgb=None):
    im = Image.open(path).convert("RGBA")
    if hue or sat != 1.0 or val != 1.0:
        hsv = im.convert("HSV")
        h, s, v = hsv.split()
        if hue:
            h = h.point(lambda x: (x + hue) % 256)
        if sat != 1.0:
            s = s.point(lambda x: min(255, int(x * sat)))
        if val != 1.0:
            v = v.point(lambda x: min(255, int(x * val)))
        im = Image.merge("HSV", (h, s, v)).convert("RGBA")
    if rgb:
        r, g, b, a = im.split()
        r = r.point(lambda x: min(255, int(x * rgb[0])))
        g = g.point(lambda x: min(255, int(x * rgb[1])))
        b = b.point(lambda x: min(255, int(x * rgb[2])))
        im = Image.merge("RGBA", (r, g, b, a))
    im.save(path)
    px = list(im.convert("RGB").resize((16, 16)).getdata())
    n = len(px)
    return "#%02X%02X%02X" % (
        sum(p[0] for p in px) // n,
        sum(p[1] for p in px) // n,
        sum(p[2] for p in px) // n,
    )


if __name__ == "__main__":
    base = sys.argv[1] if len(sys.argv) > 1 else "assets/textures/ground"
    for name, p in PARAMS.items():
        out = regrade(f"{base}/{name}.png", **p)
        print(f"{name}: {out}")
