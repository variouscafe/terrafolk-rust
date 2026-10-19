#!/usr/bin/env python3
"""proto2d 에셋 생성 — DESIGN.md 아트 파이프라인 v0.

- 지형: gen_ground.py 페인티드 텍스처 → 아이소 다이아몬드 128×64 ×2변형 ×5 GID
  + 물 2프레임(페인티드 합성)
- 주민: assets/lpc/parts + tf-data.js 리컬러 표 → walk 시트 ×2 업스케일(LANCZOS+언샵)
- 소품: 원본 픽셀 프롭 ×3 업스케일(페인티드 톤 통합)
- 부속: 그림자 블롭·선택 링 / 맵: shared-map.json → map.txt

실행: python3 tools/painted/gen_proto2d_assets.py  (결정론 — 시드 고정)
"""
import json
import os
import shutil
import sys

from PIL import Image, ImageDraw, ImageFilter

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import gen_ground as gg

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
OUT = os.path.join(ROOT, 'proto2d', 'assets')
GROUND_TEX = os.path.join(ROOT, 'assets', 'textures', 'ground')
PARTS = os.path.join(ROOT, 'assets', 'lpc', 'parts')
SEED = 20261019
NAMES = ['강돌', '하르', '마루', '시울', '뿌리', '달래', '노아', '바람']
TOL = 8  # 팔레트 매칭 톨러런스(FE tf-look.js QA 값)


# ── LPC 합성 ──────────────────────────────────────────────
def load_tf_data():
    src = open(os.path.join(ROOT, 'assets', 'web', 'tf-data.js'), encoding='utf-8').read()
    i = src.index('window.TF_DATA = ') + len('window.TF_DATA = ')
    depth, j, instr, esc = 0, i, False, False
    while j < len(src):
        c = src[j]
        if instr:
            if esc:
                esc = False
            elif c == '\\':
                esc = True
            elif c == '"':
                instr = False
        else:
            if c == '"':
                instr = True
            elif c == '{':
                depth += 1
            elif c == '}':
                depth -= 1
                if depth == 0:
                    return json.loads(src[i:j + 1])
        j += 1
    raise RuntimeError('TF_DATA JSON 추출 실패')


def hex2rgb(h):
    return tuple(int(h[k:k + 2], 16) for k in (1, 3, 5))


def nearest(c, lut):
    if c in lut:
        return lut[c]
    best, bestd = None, TOL * 3 + 1
    for s, d in lut.items():
        dd = abs(c[0] - s[0]) + abs(c[1] - s[1]) + abs(c[2] - s[2])
        if dd <= TOL * 3 and dd < bestd:
            best, bestd = d, dd
    return best


def recolor(im, pairs):
    """src→dst 헥스 색쌍 적용. P모드는 팔레트 직접 수정(고속)."""
    lut = {hex2rgb(s): hex2rgb(d) for s, d in pairs}
    if not lut:
        return im.convert('RGBA')
    if im.mode == 'P':
        pal = im.getpalette()
        for i in range(0, len(pal), 3):
            d = nearest((pal[i], pal[i + 1], pal[i + 2]), lut)
            if d:
                pal[i:i + 3] = list(d)
        im2 = im.copy()
        im2.putpalette(pal)
        return im2.convert('RGBA')
    im = im.convert('RGBA')
    px = im.load()
    for y in range(im.size[1]):
        for x in range(im.size[0]):
            r, g, b, a = px[x, y]
            if a:
                d = nearest((r, g, b), lut)
                if d:
                    px[x, y] = (d[0], d[1], d[2], a)
    return im


def pick_layers(rng, lpc, bucket):
    pool = lpc['POOL_ITEMS']
    pal, srcp = lpc['PALETTES'], lpc['SOURCE_PALETTES']
    by_cat = {}

    def ent_of(it):
        bt = it.get('byBt', {})
        return bt.get(bucket) or bt.get('*')

    for it in pool:
        if ent_of(it):
            by_cat.setdefault(it['cat'], []).append(it)

    chosen = {mat: rng.choice(list(ramps.keys()))
              for mat, ramps in pal.items() if mat != 'eye'}

    lower = 'skirt' if (bucket == 'female' and rng.random() < 0.4) else 'bottom'
    plan = [('body', 1.0), (lower, 1.0), ('top', 1.0), ('feet', 1.0),
            ('head', 1.0), ('hair', 1.0), ('brows', 0.6), ('nose', 0.5),
            ('beard', 0.35 if bucket == 'male' else 0.0), ('glasses', 0.12)]
    layers = []
    for cat, prob in plan:
        items = by_cat.get(cat)
        if not items or rng.random() >= prob:
            continue
        it = rng.choice(items)
        ent = ent_of(it)[0]
        pairs = []
        for _slot, mat in it.get('mats', {}).items():
            mk = mat if mat in pal else 'cloth'
            s = srcp.get(mk)
            if s and mk in chosen and s != chosen[mk]:
                pairs += list(zip(pal[mk][s], pal[mk][chosen[mk]]))
        layers.append((ent['z'], it['id'], ent['png'], pairs))
    layers.sort(key=lambda t: t[0])
    return layers


def compose_sheet(layers):
    sheet = Image.new('RGBA', (576, 256), (0, 0, 0, 0))
    for _z, pid, png, pairs in layers:
        p = os.path.join(PARTS, pid, f'{png}.png')
        if os.path.exists(p):
            sheet.alpha_composite(recolor(Image.open(p), pairs))
    return sheet


# ── 아이소 타일 ───────────────────────────────────────────
def diamond(tex, variant=0, w=256, h=128):
    """정사각 타일랩 텍스처 → 아이소 다이아몬드(투영 샘플링, 하단 에지 셰이드)."""
    off = 256 * variant
    tw, th = tex.size
    tpx = tex.load()
    im = Image.new('RGBA', (w, h), (0, 0, 0, 0))
    px = im.load()
    for py in range(h):
        for x in range(w):
            cx, cy = x - (w - 1) / 2, py - (h - 1) / 2
            e = abs(cx) / (w / 2) + abs(cy) / (h / 2)
            if e > 1.0:
                continue
            u = (cx / w + cy / h) % 1.0
            v = (cx / w - cy / h) % 1.0
            sx = int(u * tw + off) % tw
            sy = int(v * th + off) % th
            r, g, b = tpx[sx, sy][:3]
            m = 0.90 if (e > 0.93 and cy > 0) else 1.0
            px[x, py] = (int(r * m), int(g * m), int(b * m), 255)
    return im


def water_tex(seed):
    colors = [gg.hex2rgb(c) for c in ('#22587E', '#2E6E9E', '#3F86B8', '#5AA5D0')]
    noise = gg.wrap_value_noise(seed, 4, (1.0, 0.5))
    im = gg.paint_base(512, seed, colors, None, noise, sx=1.0, sy=0.6)
    im = gg.dab_strokes(im, seed + 1,
                        [lambda t: gg.mix(gg.hex2rgb('#7CC4E8'), gg.hex2rgb('#BCE8F8'), t)],
                        260, (10, 26), 3, 15, 80)
    im = gg.speckles(im, seed + 2,
                     [lambda t: gg.mix(gg.hex2rgb('#8FD0E8'), gg.hex2rgb('#D8F0FA'), t)],
                     130, 1, 3, 0.4)
    return im


def gen_tiles(tiles_out):
    for name in ('grass', 'forest', 'sand', 'rock', 'dirt'):
        tex = Image.open(os.path.join(GROUND_TEX, f'{name}.png')).convert('RGB')
        for v in range(4):
            diamond(tex, v).save(os.path.join(tiles_out, f'iso_{name}_{v}.png'))
    for f, seed in ((0, 901), (1, 902), (2, 903), (3, 904)):
        diamond(water_tex(seed), 0).save(os.path.join(tiles_out, f'iso_water_{f}.png'))


# ── 업스케일(픽셀→페인티드 톤) ─────────────────────────────
def upscale(im, k, sharp=1.4, amt=50):
    im = im.resize((im.width * k, im.height * k), Image.LANCZOS)
    return im.filter(ImageFilter.UnsharpMask(radius=sharp, percent=amt, threshold=2))


def main():
    import random
    rng = random.Random(SEED)
    lpc = load_tf_data()['LPC']

    tiles = os.path.join(OUT, 'tiles')
    props = os.path.join(OUT, 'props')
    fonts = os.path.join(OUT, 'fonts')
    lpcout = os.path.join(OUT, 'lpc')
    for d in (tiles, props, fonts, lpcout):
        os.makedirs(d, exist_ok=True)

    gen_tiles(tiles)

    # 소품 ×3 업스케일
    for f in sorted(os.listdir(os.path.join(ROOT, 'assets', 'props'))):
        if f.endswith('.png'):
            upscale(Image.open(os.path.join(ROOT, 'assets', 'props', f)), 3) \
                .save(os.path.join(props, f))

    shutil.copy(os.path.join(ROOT, 'assets', 'fonts', 'NotoSansKR-Regular.otf'),
                os.path.join(fonts, 'NotoSansKR-Regular.otf'))

    # 맵: Tiled JSON → GID 나열
    m = json.load(open(os.path.join(ROOT, 'assets', 'maps', 'shared-map.json')))
    open(os.path.join(OUT, 'map.txt'), 'w').write(','.join(map(str, m['layers'][0]['data'])))

    # 주민 8인: walk 시트 합성 + ×2 업스케일(프레임 64→128)
    buckets = ['male', 'male', 'male', 'female', 'female', 'male', 'female', 'teen']
    manifest = []
    for i, (name, bucket) in enumerate(zip(NAMES, buckets)):
        sheet = compose_sheet(pick_layers(rng, lpc, bucket))
        fn = f'villager_{i}.png'
        upscale(sheet, 4, sharp=1.6, amt=55).save(os.path.join(lpcout, fn))
        manifest.append({'name': name, 'bucket': bucket, 'file': f'lpc/{fn}',
                         'cols': 9, 'rows': 4, 'frame': 256, 'fps': 9})
        print(f'{name}({bucket}) ok')

    json.dump(manifest, open(os.path.join(lpcout, 'villagers.json'), 'w'),
              ensure_ascii=False, indent=1)

    # 그림자 블롭·선택 링
    sh = Image.new('RGBA', (64, 24), (0, 0, 0, 0))
    ImageDraw.Draw(sh).ellipse([2, 2, 61, 21], fill=(10, 8, 6, 95))
    sh.filter(ImageFilter.GaussianBlur(3)).save(os.path.join(OUT, 'shadow.png'))
    ring = Image.new('RGBA', (24, 14), (0, 0, 0, 0))
    ImageDraw.Draw(ring).ellipse([0, 0, 23, 13], outline=(255, 216, 88, 255), width=2)
    ring.save(os.path.join(OUT, 'ring.png'))

    print('proto2d assets →', OUT)


if __name__ == '__main__':
    main()
