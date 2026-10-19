#!/usr/bin/env python3
"""mewki 「Isometric Nature Blocks」 팩 → proto2d 지형 타일 정규화.

팩 타일(≈130px, 크기·투상비가 타일마다 약간씩 다름)을 정준 캔버스로 맞춘다:
- 상판 다이아몬드 **256×200** (비율 0.78125 — 팩 실측 중앙값 0.783)
- 캔버스 **256×312**: 상단 여백 16 + 다이아몬드 200 + 측벽존 + 하단 여백
- 다이아몬드 상단정점 = (128, 16) — ground.rs 스폰 오프셋(아래 20 월드)과 계약
- 마지막에 **normalize_block_tiles 스냅+블리드를 통과**시킨다(계단 시임 방지 —
  변형별 ±2~12px 기하 편차가 피치 200px와 어긋나 벽 띠가 비쳤던 2026-10-19 수정)

측정은 알파 에지의 중앙값 기울기 피팅(꽃·수풀 장식 아웃라이어에 강건).
매핑: grass2/7/8/1→grass · grass9/3/10→forest(꽃·수초) · stone1-4→rock ·
dirt4/1/2/3→dirt. water·sand는 assetgen 서비스가 생성(같은 정준 규격).
grass6(큰 나무)은 블럭 기하가 없어 제외 — 향후 props 소재.

실행: python3 tools/painted/apply_mewki_blocks.py <추출된 팩 디렉터리>
"""
import os
import sys

from PIL import Image

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import normalize_block_tiles as NB  # 스냅+블리드+게이트 (정준 규격 단일 원천)

ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
OUT = os.path.join(ROOT, 'proto2d', 'assets', 'tiles')

# 재생성 시 기록하는 계약 문서(normalize_block_tiles와 동일 내용 유지)
SPEC_DOC = (
'# 지형 블록 타일 정준 규격 (mewki 팩 기준)\n\n- 출처: mewki 「Isometric Nature Blocks」 (itch.io, 상업적 사용 허가·재배포 금지)\n- 캔버스: **256×312** — 상판 다이아몬드 **256×200** (비율 0.78125), 상단정점\n  **(128, 16)**, 중심 (128, 116)\n- **에지 블리드 2px** — 불투명 마스크 팽창+가장자리 색 복제. 앞 타일(y-sort)\n  이 시임을 겹쳐 덮어 AA 시임·벽 노출을 봉쇄한다 (2026-10-19 \'계단 시임\' 수정)\n- 월드: TW=128, TH=100(=캔버스 200px 피치와 정합), 스프라이트 오프셋 (0, −20),\n  z=py·0.001 (y-sort)\n- **물 평면 낙차 12 월드** — 해안에서 육지 벽이 절벽으로 드러난다(섬 슬래브).\n  내부 육지는 하나의 이어진 평면\n- 변형: GID별 water 4(애님)·sand 4·grass 4·**forest 3**(forest_3은 큰 나무 장식\n  — 블럭 기하가 없어 평면 계약 밖, 향후 props 소재)·rock 4·dirt 4\n- water·sand: assetgen 서비스가 동일 규격으로 생성\n- 재생성: `python3 tools/painted/apply_mewki_blocks.py <팩 디렉터리>` →\n  `python3 tools/painted/normalize_block_tiles.py` (스냅+블리드+게이트)\n'
)

CAN_W, CAN_H = NB.CAN_W, NB.CAN_H
DIA_W, DIA_H = NB.DIA_W, NB.DIA_H
TOP_PAD = NB.TOP_Y

MAPPING = [  # (팩 파일, 출력 GID, 변형 인덱스)
    ('grass2.png', 'grass', 0), ('grass7.png', 'grass', 1),
    ('grass8.png', 'grass', 2), ('grass1.png', 'grass', 3),
    ('grass9.png', 'forest', 0), ('grass3.png', 'forest', 1),
    ('grass10.png', 'forest', 2),
    ('stone1.png', 'rock', 0), ('stone2.png', 'rock', 1),
    ('stone3.png', 'rock', 2), ('stone4.png', 'rock', 3),
    ('dirt4.png', 'dirt', 0), ('dirt1.png', 'dirt', 1),
    ('dirt2.png', 'dirt', 2), ('dirt3.png', 'dirt', 3),
]


def _lsq_slope(pts):
    """(y, x) 점집합의 |dx/dy| 최소제곱 — 반환 0이면 피팅 실패."""
    n = len(pts)
    sy = sum(p[0] for p in pts) / n
    sx = sum(p[1] for p in pts) / n
    denom = sum((p[0] - sy) ** 2 for p in pts)
    return abs(sum((p[0] - sy) * (p[1] - sx) for p in pts) / denom) if denom else 0.0


def fit_diamond(im):
    """알파 에지에서 상판 다이아몬드 (W, H, 상단정점 x/y) 추정.

    에지 기울기 |dx/dy| = W/H. 계단형 픽셀 에지·측벽(수직) 구간·꽃·풀 장식
    아웃라이어에 강건하게: 좌우 에지를 각각 (1)조피팅 → (2)잔차>3px 행 제거 →
    (3)재피팅. 피팅 구간은 다이아몬드 좌우정점(=H/2 지점)까지만 — H는 반복 갱신.
    """
    w, h = im.size
    a = im.getchannel('A').load()
    spans = {}
    for y in range(h):
        xs = [x for x in range(w) if a[x, y] > 40]
        spans[y] = (xs[0], xs[-1]) if xs else None
    rows = [y for y in range(h) if spans[y]]
    top, bottom = rows[0], rows[-1]
    vx = sum(spans[y][0] + spans[y][1] for y in range(top, top + 3)) / 6.0

    widest = next((spans[y] for y in rows
                   if spans[y][1] - spans[y][0] + 1 >= w * 0.97), None)
    W = (widest[1] - widest[0] + 1) if widest else float(w)

    H = W / 1.3  # 초기 가정(비율 0.77) → 좌우정점 y ≈ top + H/2
    slope = 0.0
    for _ in range(3):
        y_end = min(bottom, top + H / 2 * 0.92)
        slopes = []
        for edge in (0, 1):
            pts = [(y, spans[y][edge]) for y in range(top + 3, int(y_end))
                   if spans[y]]
            if len(pts) < 8:
                continue
            s = _lsq_slope(pts)
            if not s:
                continue
            # 잔차 기반 이상치(꽃·풀 장식) 제거 후 재피팅
            sy = sum(p[0] for p in pts) / len(pts)
            sx = sum(p[1] for p in pts) / len(pts)
            pts2 = [p for p in pts if abs(p[1] - sx - s * (p[0] - sy)) <= 3.0]
            if len(pts2) >= 8:
                s = _lsq_slope(pts2) or s
            slopes.append(s)
        if not slopes:
            break
        slope = sum(slopes) / len(slopes)
        H = W / slope
    if not slope:
        H = W * 0.7656  # 정준 폴백(프리뷰 실측 비율)
    return W, H, vx, top


def normalize(src):
    """팩 타일 1장 → 정준 캔버스 + 스냅 4회 + 블리드(계단 시임 봉쇄)."""
    im = Image.open(src).convert('RGBA')
    W, H, vx, top = fit_diamond(im)
    sx, sy = DIA_W / W, DIA_H / H
    im2 = im.resize((round(im.width * sx), round(im.height * sy)), Image.LANCZOS)
    canvas = Image.new('RGBA', (CAN_W, CAN_H), (0, 0, 0, 0))
    # 상단 정점 (vx, top) → (중앙x=128, TOP_PAD) 정렬
    canvas.alpha_composite(
        im2, (round(CAN_W / 2 - vx * sx), round(TOP_PAD - top * sy)))
    for _ in range(4):
        canvas = NB.snap(canvas)
    return NB.bleed(canvas)


def main():
    src_dir = sys.argv[1] if len(sys.argv) > 1 else '/tmp/mewki-iso/extracted2/pack'
    os.makedirs(OUT, exist_ok=True)
    for fn, gid, v in MAPPING:
        p = os.path.join(src_dir, fn)
        if not os.path.exists(p):
            sys.exit(f'원본 없음: {p}')
        out = os.path.join(OUT, f'iso_{gid}_{v}.png')
        normalize(p).save(out)
        print(f'{fn:14s} → iso_{gid}_{v}.png')
    # 정준 규격 계약 문서(ground.rs·assetgen과 공유)
    open(os.path.join(OUT, 'BLOCK_SPEC.md'), 'w', encoding='utf-8').write(
        SPEC_DOC)
    print('BLOCK_SPEC.md 기록 →', OUT)


if __name__ == '__main__':
    main()
