#!/usr/bin/env python3
"""주민 시트 고화질 재스타일 배치 — 베이스 합성(이 repo) → charstyle(assetgen).

계약(CLAUDE.md — 이 repo는 직접 그리지 않는다): LPC 부품 합성·리컬러는 여기서
결정론적으로 하고, 페인티드 재스타일은 assetgen `core/charstyle.py`(Tier 2
img2img + 게이트)에 위임한다. 산출물은 proto2d/assets/lpc/villager_N.png 에
그대로 드롭인(해상만 2304×1024로 동일 — villagers.json 불변).

사용:
  python3 tools/painted/paint_villagers.py [--only 0,3] [--force] [--seed 424242]

- 재개 지원: 출력이 이미 있으면 건너뛴다(--force로 재생성).
- 게이트 FAIL이어도 산출물은 기록되고 배치는 계속 — 요약에서 표시.
"""
import argparse
import json
import os
import subprocess
import sys
import tempfile

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
ASSETGEN = os.path.join(os.path.dirname(ROOT), 'assetgen')
LPC_OUT = os.path.join(ROOT, 'proto2d', 'assets', 'lpc')

sys.path.insert(0, HERE)
import gen_proto2d_assets as g  # noqa: E402

BUCKETS = ['male', 'male', 'male', 'female', 'female', 'male', 'female', 'teen']
PY = os.path.join(ASSETGEN, '.venv', 'bin', 'python')


def compose_bases():
    """gen_proto2d_assets 와 동일 결정론 시퀀스로 베이스 시트 8장(576×256)."""
    import random
    rng = random.Random(g.SEED)
    lpc = g.load_tf_data()['LPC']
    sheets = []
    for name, bucket in zip(g.NAMES, BUCKETS):
        sheets.append(g.compose_sheet(g.pick_layers(rng, lpc, bucket)))
    return sheets


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument('--only', default='', help='대상 인덱스 쉼표 나열(예: 0,3)')
    ap.add_argument('--force', action='store_true', help='있어도 재생성')
    ap.add_argument('--seed', type=int, default=424242)
    a = ap.parse_args()
    only = {int(x) for x in a.only.split(',') if x.strip()} if a.only else None

    tmp = tempfile.mkdtemp(prefix='tf-villpaint-')
    bases = compose_bases()
    fails = []
    for i, base in enumerate(bases):
        if only is not None and i not in only:
            continue
        out = os.path.join(LPC_OUT, f'villager_{i}.png')
        marker = os.path.join(LPC_OUT, f'villager_{i}.painted.json')
        # 재개 판정: 출력 png 존재만으론 안 된다(구 저해상 시트도 존재) —
        # 이 파이프라인이 성공 표시로 남긴 마커 사이드카가 있어야 건너뜀.
        if os.path.exists(marker) and not a.force:
            print(f'[{i}] {g.NAMES[i]} — 페인티드 마커 있음, 건너뜀')
            continue
        base_png = os.path.join(tmp, f'base_{i}.png')
        base.save(base_png)
        print(f'[{i}] {g.NAMES[i]} 재스타일 중… (seed {a.seed + i})', flush=True)
        r = subprocess.run(
            [PY, '-m', 'core.charstyle', '--in', base_png, '--out', out,
             '--seed', str(a.seed + i)],
            cwd=ASSETGEN, capture_output=True, text=True)
        tail = (r.stdout or '').strip().splitlines()[-3:] if r.stdout else []
        for line in tail:
            print('   ', line, flush=True)
        if r.returncode == 0:
            metrics = next((l for l in tail if l.strip().startswith('측정')), '')
            open(marker, 'w').write(json.dumps(
                {'villager': i, 'name': g.NAMES[i], 'seed': a.seed + i,
                 'metrics': metrics.strip()}, ensure_ascii=False))
        else:
            fails.append((i, g.NAMES[i], tail))
            if r.stderr:
                print('    stderr:', r.stderr.strip().splitlines()[-1])

    if fails:
        print('게이트 실패:', fails)
        raise SystemExit(1)
    print('전원 완료 →', LPC_OUT)


if __name__ == '__main__':
    main()
