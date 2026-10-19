#!/usr/bin/env python3
"""proto2d 지형 블록 타일 정준화 후처리 — '계단 시임' 제거(2026-10-19).

원인: 변형마다 상판 다이아몬드 실측이 256×188~206·중심 ±4px 흔들림.
피치(200px)보다 짧은 변형 옆으로 뒤 블럭 측벽이 1~10px 띠로 비쳐
지형 전체가 계단처럼 보였다. 본 스크립트는 현재 PNG를 원본 팩 없이:

1. 각 타일 다이아몬드를 강건 피팅(꽃·수풀 아웃라이어 제거) 후
   **정준 규격으로 어파인 스냅** — 상판 256×200, 상단정점 (128, 16).
2. **2px 바깥 블리드** — 불투명 마스크 팽창 + 가장자리 색 복제.
   앞 타일(y-sort)이 접합선을 2px 겹쳐 덮어 AA 반투명 에지 시임도 봉쇄.

적용 후 재실측으로 전 타일 동일 기하를 검증한다(파이프라인 게이트).
apply_mewki_blocks.py 재생성 시에도 마지막에 본 모듈을 통과시킬 것.

실행: python3 tools/painted/normalize_block_tiles.py
"""
import glob
import os
import sys

import numpy as np
from PIL import Image

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
TILES = os.path.join(ROOT, 'proto2d', 'assets', 'tiles')

CAN_W, CAN_H = 256, 312     # 정준 캔버스(기존 유지)
DIA_W, DIA_H = 256, 200     # 상판 다이아몬드(피치 TH=100 월드 = 200px와 정합)
TOP_Y = 16                  # 상단정점 y — ground.rs SPR_Y_OFF(20) 계약의 기준
CX = 128                    # 상단정점 x(= 좌우 중심)
BLEED = 2                   # 바깥 블리드 두께(px)


def _lsq_edge(pts):
    """(y, x) 점집합 최소제곱 직선 → x(y) = a·y + b."""
    n = len(pts)
    sy = sum(p[0] for p in pts) / n
    sx = sum(p[1] for p in pts) / n
    denom = sum((p[0] - sy) ** 2 for p in pts)
    if not denom:
        return None
    a = sum((p[0] - sy) * (p[1] - sx) for p in pts) / denom
    return a, sx - a * sy


def fit_diamond(im):
    """상판 다이아몬드 (W, H, 상단정점 x/y) 강건 추정 — 폭 프로파일 외삽.

    다이아몬드 내부 상부(좌우정점 행 cy에서 10~70행 위)는 폭이 y에 대해
    선형 → 최소제곱 후 w=0 절편이 상단정점. 프린지(상단 장식)·측벽(하단)
    구간을 원천 배제해 mewki 손그림 에지에도 강건하다.
    """
    w, h = im.size
    a = np.array(im)[:, :, 3]
    spans = {}
    for y in range(h):
        xs = np.where(a[y] > 40)[0]
        if len(xs):
            spans[y] = (int(xs.min()), int(xs.max()))
    maxw = max(s[1] - s[0] + 1 for s in spans.values())
    W = maxw
    cy = min(y for y, s in spans.items() if s[1] - s[0] + 1 >= maxw * 0.98)

    inner = [y for y in range(cy - 70, cy - 10) if y in spans]
    if len(inner) < 8:
        return W, W * 0.78125, 128.0, float(min(spans))  # 폴백: 정준 비율
    pw = [(float(y), float(spans[y][1] - spans[y][0] + 1)) for y in inner]
    fit = _lsq_edge(pw)  # (y, w) 직선 — 기울기 = W/(H/2)
    if not fit or fit[0] <= 0:
        return W, W * 0.78125, 128.0, float(min(spans))
    sl, ic = fit
    y_vertex = -ic / sl  # w=0 절편
    H = 2.0 * W / sl
    cx = float(np.median([(spans[y][0] + spans[y][1]) / 2.0 for y in inner]))
    return W, H, cx, y_vertex


def snap(im):
    """다이아몬드 실측 → 정준 (256×200 @ 상단정점 (128,16)) 어파인 정합."""
    W, H, vx, vy = fit_diamond(im)
    sx, sy = DIA_W / W, DIA_H / H
    im2 = im.resize((round(im.width * sx), round(im.height * sy)), Image.LANCZOS)
    canvas = Image.new('RGBA', (CAN_W, CAN_H), (0, 0, 0, 0))
    canvas.alpha_composite(
        im2, (round(CX - vx * sx), round(TOP_Y - vy * sy)))
    return canvas


def bleed(im):
    """불투명 영역 BLEED px 바깥 팽창 — 색은 인접 불투명 픽셀 복제, 알파 255.

    상판 에지 접합선을 앞 타일이 겹쳐 덮게 해 AA 시임·배경 노출을 봉쇄한다.
    4-이웃 전파로 BLEED 회수만큼 반복(대각선 계단 보정).
    """
    arr = np.array(im)
    opaque = arr[:, :, 3] >= 250
    for _ in range(BLEED):
        grow = np.zeros_like(opaque)
        grow[1:, :] |= opaque[:-1, :]
        grow[:-1, :] |= opaque[1:, :]
        grow[:, 1:] |= opaque[:, :-1]
        grow[:, :-1] |= opaque[:, 1:]
        # 대각선도 채워 계단 톱니 방지
        grow[1:, 1:] |= opaque[:-1, :-1]
        grow[1:, :-1] |= opaque[:-1, 1:]
        grow[:-1, 1:] |= opaque[1:, :-1]
        grow[:-1, :-1] |= opaque[1:, 1:]
        new = grow & ~opaque
        if not new.any():
            break
        ys, xs = np.where(new)
        for y, x in zip(ys, xs):
            for dy, dx in ((0, 1), (0, -1), (1, 0), (-1, 0)):
                ny, nx = y + dy, x + dx
                if 0 <= ny < opaque.shape[0] and 0 <= nx < opaque.shape[1] \
                        and opaque[ny, nx]:
                    arr[y, x] = arr[ny, nx]
                    arr[y, x, 3] = 255
                    break
        opaque = arr[:, :, 3] >= 250
    return Image.fromarray(arr)


def main():
    # forest_3 제외 — 큰 나무(수관+뾰족 밑동) 장식 타일이라 블럭 기하가 없음.
    # 지형 평면 계약 밖(향후 props 소재). 나머지 23장이 평면을 구성한다.
    skip = {'iso_forest_3.png'}
    files = sorted(f for f in glob.glob(os.path.join(TILES, 'iso_*.png'))
                   if os.path.basename(f) not in skip)
    if not files:
        sys.exit(f'타일 없음: {TILES}')
    ok = True
    for f in files:
        im = Image.open(f).convert('RGBA')
        # 반복 스냅 — 1회차 피팅은 원본 어긋남 때문에 창이 밀려 편향되므로
        # 4회 수렴(실측: r3에서 전 타일 yv≈16·H≈200·cx≈128에 수렴).
        for _ in range(4):
            im = snap(im)
        # 게이트 — 블리드 전 강건 피팅 재실측. 알파 극값은 손그림 볼록점·
        # 프린지에 오염되므로 폭 프로파일 모델 기반으로 판정한다.
        W, H, cx, yv = fit_diamond(im)
        good = (abs(yv - TOP_Y) <= 2 and abs(H - DIA_H) <= 4
                and abs(cx - CX) <= 2 and W >= CAN_W - 4)
        ok &= good
        bleed(im).save(f, optimize=True)
        print(f'{os.path.basename(f):18s} top={yv:.1f} height={H:.1f} '
              f'width={W:.0f} cx={cx:.1f} {"OK" if good else "** NG **"}')
    sys.exit(0 if ok else 1)


if __name__ == '__main__':
    main()
