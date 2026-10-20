#!/usr/bin/env python3
"""페인티드 부품 오프라인 재판정 — 기존 PNG을 현재 게이트로 다시 채점.

charstyle 게이트(style·IoU·drift)는 전부 CPU 계산이라, 게이트 개정 후
기존 산출물을 GPU 재생성 없이 재검할 수 있다. 구 게이트 오탐(빈 프로브
프레임·표본 하한)으로 FAIL 마커만 남은 산출은 ok:true로 뒤집고, 진짜
위반은 FAIL로 남겨 재생성 대상을 가려낸다(2026-10-20 부품 자산화 배치
실측 — regate로 nose_big 등을 GPU 없이 통과 처리).

주의: 이 스크립트의 판정은 charstyle restyle_sheet의 게이트와 같은
함수·기준을 쓴다(charstyle·stylelock 직접 import). assetgen 게이트가
바뀌면 이 도구의 판정도 같이 바뀐다 — 마커 재판정은 게이트 커밋과
함께 돌리는 것이 원칙.

사용:
  python3 tools/painted/regate_parts.py            # FAIL 마커 전수
  python3 tools/painted/regate_parts.py <part>/<png> …   # 지정 부품
"""
import glob
import json
import os
import statistics
import sys

HERE = os.path.dirname(os.path.abspath(__file__))
ROOT = os.path.dirname(os.path.dirname(os.path.dirname(os.path.abspath(__file__))))
ASSETGEN = os.path.join(os.path.dirname(ROOT), 'assetgen')
PARTS = os.path.join(ROOT, 'assets', 'lpc', 'parts')
PAINTED = os.path.join(ROOT, 'assets', 'painted-parts')

sys.path.insert(0, ASSETGEN)
sys.path.insert(0, HERE)
from PIL import Image  # noqa: E402

from core import stylelock  # noqa: E402
from core.charstyle import (PART_DRIFT_MEAN_MAX, PART_DRIFT_SIGMA_MAX,  # noqa: E402
                            PART_IOU_MIN, PART_IOU_TINY_MIN, alpha_iou,
                            densest_frame, mean_rgb, tiny_part)

COLS, ROWS, IN_F, OUT_F = 9, 4, 64, 256


def gate(part, png):
    """restyle_sheet 게이트의 오프라인 재현 — 산출 PNG vs 원본 시트."""
    base = Image.open(os.path.join(PARTS, part, png + '.png')).convert('RGBA')
    out = Image.open(os.path.join(PAINTED, part, png + '.png')).convert('RGBA')
    frame, _rc = densest_frame(base, COLS, ROWS, IN_F)
    probe_ref = frame  # 원본 해상도 그대로 — charstyle 게이트와 동일 기준
    probe, probe_n = None, -1
    ious, row_sigmas, abs_drifts = [], [], []
    for r in range(ROWS):
        drifts = []
        for c in range(COLS):
            fr = base.crop((c * IN_F, r * IN_F, (c + 1) * IN_F, (r + 1) * IN_F))
            pf = out.crop((c * OUT_F, r * OUT_F,
                           (c + 1) * OUT_F, (r + 1) * OUT_F))
            n = len(stylelock._opaque_px(pf))
            if n > probe_n:
                probe, probe_n = pf, n
            ious.append(alpha_iou(fr, pf, dilate=2))
            drifts.append(tuple(p - q for p, q in zip(mean_rgb(pf), mean_rgb(fr))))
            abs_drifts.append(sum(abs(x) for x in drifts[-1]) / 3)
        row_sigmas.append(max(statistics.pstdev([d[i] for d in drifts])
                              for i in range(3)))
    sigma, adrift = max(row_sigmas), sum(abs_drifts) / len(abs_drifts)
    issues, _ = stylelock.style_issues(probe, 'character-part', ref=probe_ref)
    iou_min = PART_IOU_TINY_MIN if tiny_part(probe_n) else PART_IOU_MIN
    if min(ious) < iou_min:
        issues.append(f'알파 IoU 최저 {min(ious):.3f} <{iou_min:g}')
    if sigma > PART_DRIFT_SIGMA_MAX:
        issues.append(f'행별 drift σ 최대 {sigma:.1f} >{PART_DRIFT_SIGMA_MAX:g}')
    if adrift > PART_DRIFT_MEAN_MAX:
        issues.append(f'평균 |drift| {adrift:.1f} >{PART_DRIFT_MEAN_MAX:g}')
    metrics = {'iou_min': round(min(ious), 3), 'drift_sigma_max': round(sigma, 2),
               'drift_mean': round(adrift, 1), 'frames': len(ious)}
    return issues, metrics


def main():
    targets = sys.argv[1:]
    if not targets:
        for mp in sorted(glob.glob(os.path.join(PAINTED, '*', '*.painted.json'))):
            try:
                if not json.load(open(mp)).get('ok'):
                    targets.append(mp)
            except json.JSONDecodeError:
                pass
    for spec in targets:
        if spec.endswith('.json'):
            mp = spec
            rel = os.path.relpath(mp, PAINTED).replace('.painted.json', '')
        else:
            rel = spec
            mp = os.path.join(PAINTED, rel + '.painted.json')
        part, png = rel.split('/')
        try:
            issues, metrics = gate(part, png)
        except (OSError, ValueError) as e:
            print(f'{rel}: 채점 불가({e})')
            continue
        m = json.load(open(mp))
        state = 'PASS' if not issues else 'FAIL ' + '; '.join(issues[:3])
        if not issues and not m.get('ok'):
            m['ok'] = True
            m['log'] = ['regate: 신 게이트 재판정 통과(오프라인)'] + m.get('log', [])
            m['metrics'] = metrics
            json.dump(m, open(mp, 'w'), ensure_ascii=False)
        elif issues and m.get('ok'):
            print(f'{rel}: ⚠ 통과 마커인데 재판정 FAIL — 수동 확인: {state}')
            continue
        print(f'{rel}: {state} {metrics}')


if __name__ == '__main__':
    main()
