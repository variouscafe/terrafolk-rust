#!/usr/bin/env python3
"""페인티드 부품 조합기 — assets/painted-parts 부품으로 주민 시트 재구성.

paint_parts.py가 자산화한 부품 시트(2304×1024)를 pick_layers 결정론 조합으로
겹쳐 주민 시트(proto2d/assets/lpc/villager_N.png)를 만든다. 부품별 재컬러는
'유니크 색 → 원본 램프 최근접 → 대상 램프' 매핑(페인티드 색 드리프트를 흡수).

폴백: 부품이 아직 없거나 게이트 FAIL(마커 ok=false)이면 픽셀 부품 ×4
업스케일 + 기존 정확매칭 재컬러로 대체 — 부품 배치가 완성되어도 즉시 조합 가능.

사용:
  python3 tools/painted/compose_painted.py [--villager 0,3] [--dry]
"""
import argparse
import json
import os
import sys

import numpy as np
from PIL import Image

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
PARTS = os.path.join(ROOT, 'assets', 'lpc', 'parts')
PAINTED = os.path.join(ROOT, 'assets', 'painted-parts')
LPC_OUT = os.path.join(ROOT, 'proto2d', 'assets', 'lpc')

sys.path.insert(0, HERE)
import gen_proto2d_assets as g  # noqa: E402

BUCKETS = ['male', 'male', 'male', 'female', 'female', 'male', 'female', 'teen']
PAINT_TOL = 72   # 재컬러 매칭 톨러런스(채널합) — 페인티드 드리프트(±10~20) 흡수
SHEET = (2304, 1024)


def painted_part(pid, png):
    """게이트 통과한 페인티드 부품 시트(없으면 None)."""
    marker = os.path.join(PAINTED, pid, f'{png}.painted.json')
    out = os.path.join(PAINTED, pid, f'{png}.png')
    if not (os.path.exists(marker) and os.path.exists(out)):
        return None
    try:
        return Image.open(out).convert('RGBA') if json.load(
            open(marker, encoding='utf-8')).get('ok') else None
    except (json.JSONDecodeError, OSError):
        return None


def recolor_painted(im, pairs):
    """유니크 색 → 원본 램프 최근접(±PAINT_TOL) → 대상 램프. 음영 구조 보존."""
    lut = {g.hex2rgb(s): g.hex2rgb(d) for s, d in pairs}
    if not lut:
        return im
    srcs = np.array(list(lut.keys()), dtype=np.int32)
    dsts = np.array([lut[tuple(s)] for s in srcs], dtype=np.int32)
    a = np.asarray(im, dtype=np.int32).copy()
    mask = a[..., 3] > 8
    px = a[..., :3][mask]
    uniq, inv = np.unique(px >> 3, axis=0, return_inverse=True)
    ucol = uniq * 8 + 4                                   # 양자화 복원 대표색
    d = np.abs(ucol[:, None, :] - srcs[None, :, :]).sum(2)
    best, bestd = d.argmin(1), d.min(1)
    mapping = np.where((bestd <= PAINT_TOL)[:, None], dsts[best], ucol)
    a[..., :3][mask] = mapping[inv]
    return Image.fromarray(a.astype('uint8'), 'RGBA')


def compose(layers):
    """z순 조합 — 페인티드 우선, 폴백=픽셀 부품 ×4(LANCZOS)+정확매칭 재컬러."""
    sheet = Image.new('RGBA', SHEET, (0, 0, 0, 0))
    fallbacks = []
    for _z, pid, png, pairs in layers:
        p = painted_part(pid, png)
        if p is not None:
            if pairs:
                p = recolor_painted(p, pairs)
            if p.size != SHEET:
                p = p.resize(SHEET, Image.LANCZOS)
        else:
            src = os.path.join(PARTS, pid, f'{png}.png')
            if not os.path.exists(src):
                continue
            p = g.recolor(Image.open(src).convert('RGBA'), pairs)
            p = g.upscale(p, 4)
            fallbacks.append(f'{pid}/{png}')
        sheet.alpha_composite(p)
    return sheet, fallbacks


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--villager', default='', help='대상 인덱스 쉼표 나열(기본 전원)')
    ap.add_argument('--dry', action='store_true', help='파일 쓰지 않고 조합만 검증')
    ap.add_argument('--force', action='store_true',
                    help='폴백 과다 가드 무시(부품 라이브러리 미완성 단계 보호)')
    a = ap.parse_args()
    want = ({int(x) for x in a.villager.split(',') if x.strip()}
            if a.villager else None)

    import random
    rng = random.Random(g.SEED)
    lpc = g.load_tf_data()['LPC']
    for i, (name, bucket) in enumerate(zip(g.NAMES, BUCKETS)):
        # pick_layers는 주민마다 가변 개수의 rng를 소비한다 — 건너뛸 때도
        # 반드시 전원 소비해 시퀀스를 보존한다(안 그러면 --villager 3이
        # 0번의 레이어 시퀀스로 3번 시트를 잘못 덮어쓴다).
        layers = g.pick_layers(rng, lpc, bucket)
        if want is not None and i not in want:
            continue
        sheet, fb = compose(layers)
        # 가드: 부품 라이브러리가 덜 차면 폴백(픽셀×4) 비중이 커져 기존
        # 페인티드 시트보다 퇴행한 산출로 게임을 덮어쓰게 된다 — 거부.
        if len(fb) > max(1, len(layers) // 3) and not (a.force or a.dry):
            print(f'[{i}] {name}: 폴백 {len(fb)}/{len(layers)} 과다 — '
                  f'시트 유지(부품 라이브러리 채우는 중)', flush=True)
            continue
        if a.dry:
            sheet.save(f'/tmp/compose_dry_{i}.png')
            print(f'[{i}] {name}: 레이어 {len(layers)}, 폴백 {len(fb)} {fb}')
            continue
        out = os.path.join(LPC_OUT, f'villager_{i}.png')
        sheet.save(out)
        open(os.path.join(LPC_OUT, f'villager_{i}.painted.json'), 'w').write(
            json.dumps({'villager': i, 'name': name,
                        'pipeline': 'painted-parts-compose',
                        'layers': [l[1] for l in layers],
                        'fallbacks': fb}, ensure_ascii=False))
        print(f'[{i}] {name}: 조합 완료(레이어 {len(layers)}, 폴백 {len(fb)}) → {out}',
              flush=True)


if __name__ == '__main__':
    main()
