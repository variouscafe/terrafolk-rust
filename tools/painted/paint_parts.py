#!/usr/bin/env python3
"""LPC 부품 전수 고품질 재스타일 — 부품 시트별 charstyle 배치(자산화).

부품 PNG는 각각 576×256 워크 시트(9열×4행 64px — 주민 시트와 동일 그리드)라
assetgen charstyle이 그대로 적용된다. 산출물은 assets/painted-parts/<id>/<png>.png
(2304×1024) — compose_painted.py 가 조합해 주민 시트를 만든다.

- 순서: 8주민이 사용하는 부품부터(게임 적용 조기 확보) → 나머지 풀 전수.
- 재개: 부품별 마커 JSON이 있으면 건너뜀(게이트 FAIL도 마커에 기록 — 조합 시
  픽셀 부품 ×4 폴백).
- 게이트 실패 시 디노이즈를 낮춰(0.42→0.36) 1회 자동 재시도 — 가느다란 부품
  (안경·눈썹)은 IoU가 민감하므로 구조 보존 우선.

사용:
  python3 tools/painted/paint_parts.py [--only-used] [--force] [--limit N]
"""
import argparse
import json
import os
import random
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
ASSETGEN = os.path.join(os.path.dirname(ROOT), 'assetgen')
PARTS = os.path.join(ROOT, 'assets', 'lpc', 'parts')
OUT = os.path.join(ROOT, 'assets', 'painted-parts')
PY = os.path.join(ASSETGEN, '.venv', 'bin', 'python')

sys.path.insert(0, HERE)
import gen_proto2d_assets as g  # noqa: E402

BUCKETS = ['male', 'male', 'male', 'female', 'female', 'male', 'female', 'teen']
SEED_BASE = 525252  # 주민(424242+)과 겹치지 않는 부품용 시드 영역


def sheet_list(lpc):
    """풀 전체 (id, png, category) — 실존 파일만. 사용 부품을 앞에."""
    seen, items = set(), []
    for it in lpc['POOL_ITEMS']:
        for bt, ents in it.get('byBt', {}).items():
            for ent in ents:
                key = (it['id'], ent['png'])
                if key in seen:
                    continue
                seen.add(key)
                if os.path.exists(os.path.join(PARTS, it['id'], ent['png'] + '.png')):
                    items.append((it['id'], ent['png'], it['cat']))

    # 8주민 사용 부품(결정론 pick_layers 재현) 선순위
    rng = random.Random(g.SEED)
    used = set()
    for b in BUCKETS:
        for _z, pid, png, _p in g.pick_layers(rng, lpc, b):
            used.add((pid, png))
    items.sort(key=lambda t: (t[:2] not in used, t[2], t[0], t[1]))
    return items, used


def run_charstyle(base_png, out_png, seed, denoise):
    r = subprocess.run(
        [PY, '-m', 'core.charstyle', '--in', base_png, '--out', out_png,
         '--seed', str(seed), '--denoise', str(denoise), '--sweep', '2',
         '--kind', 'character-part'],
        cwd=ASSETGEN, capture_output=True, text=True)
    tail = (r.stdout or '').strip().splitlines()[-3:] if r.stdout else []
    return r.returncode, tail, (r.stderr or '').strip().splitlines()[-1:]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--only-used', action='store_true', help='8주민 사용 부품만')
    ap.add_argument('--force', action='store_true')
    ap.add_argument('--limit', type=int, default=0, help='최대 처리 수(테스트용)')
    a = ap.parse_args()

    lpc = g.load_tf_data()['LPC']
    items, used = sheet_list(lpc)
    if a.only_used:
        items = [t for t in items if t[:2] in used]
    if a.limit:
        items = items[:a.limit]
    print(f'대상 부품 시트 {len(items)}장 (사용 부품 {len(used)} 중 우선)', flush=True)

    tmp = tempfile.mkdtemp(prefix='tf-partpaint-')
    fails, done = [], 0
    for i, (pid, png, cat) in enumerate(items):
        marker = os.path.join(OUT, pid, f'{png}.painted.json')
        out_png = os.path.join(OUT, pid, f'{png}.png')
        if os.path.exists(marker) and not a.force:
            continue
        os.makedirs(os.path.join(OUT, pid), exist_ok=True)
        base = Image_open(pid, png)
        base.save(os.path.join(tmp, 'base.png'))
        seed = SEED_BASE + i
        used_seed = seed
        rc, tail, err = run_charstyle(os.path.join(tmp, 'base.png'), out_png, seed, 0.34)
        if rc != 0:  # 구조 보존 우선 재시도 — 부품은 위치·스케일이 최우선
            used_seed = seed + 100000
            rc, tail, err = run_charstyle(os.path.join(tmp, 'base.png'),
                                          out_png, used_seed, 0.28)
        # 마커 시드는 실제 채택된 실행의 시드 — 재시도본을 원본 시드로
        # 기록하면 재현·리파인이 다른 이미지를 뽑는다(2026-10-20 리뷰).
        note = {'part': pid, 'png': png, 'cat': cat, 'seed': used_seed,
                'ok': rc == 0, 'log': tail}
        json.dump(note, open(marker, 'w'), ensure_ascii=False)
        done += 1
        state = 'PASS' if rc == 0 else 'FAIL'
        print(f'[{i + 1}/{len(items)}] {pid}/{png} ({cat}) {state}', flush=True)
        if rc != 0:
            fails.append((pid, png, tail))
            if err:
                print('    stderr:', err, flush=True)

    print(f'완료: {done} 처리, 실패 {len(fails)}', flush=True)
    for f in fails:
        print('  FAIL:', f[0], f[1], f[2][-1:] if f[2] else '', flush=True)


def Image_open(pid, png):
    from PIL import Image
    return Image.open(os.path.join(PARTS, pid, png + '.png')).convert('RGBA')


if __name__ == '__main__':
    main()
