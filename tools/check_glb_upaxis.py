#!/usr/bin/env python3
"""GLB up축 정합 게이트 — Z-up 메시가 노드 변환 없이 실려 '옆으로 누운' GLB를 차단.

유래: fetch_village_glbs.py merge()가 원본 노드의 -90°X 변환을 버려 CottageHouses.glb가
Z-up으로 렌더(완공 집이 누움, 2026-11-05). bevy는 Y-up.
대상: 스킨 없는 *.glb (스킨 GLB는 ×100 아마추어가 정상이라 제외).
계산: scene 루트부터 matrix 또는 TRS를 4x4로 합성해 월드 AABB.
Z-up 시그니처: |min.z| < 0.02*size.z 이고 min.y < -0.2*size.y  → 경로 출력 + exit 1.
용법: check_glb_upaxis.py [assets/models 디렉터리]
"""
import json
import struct
import sys
from pathlib import Path


def node_matrix(n):
    if "matrix" in n:
        m = n["matrix"]  # column-major
        return [[m[c * 4 + r] for c in range(4)] for r in range(4)]
    tx, ty, tz = n.get("translation", [0, 0, 0])
    x, y, z, w = n.get("rotation", [0, 0, 0, 1])
    sx, sy, sz = n.get("scale", [1, 1, 1])
    r = [
        [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
        [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
        [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)],
    ]
    return [
        [r[0][0] * sx, r[0][1] * sy, r[0][2] * sz, tx],
        [r[1][0] * sx, r[1][1] * sy, r[1][2] * sz, ty],
        [r[2][0] * sx, r[2][1] * sy, r[2][2] * sz, tz],
        [0, 0, 0, 1],
    ]


def mm(a, b):
    return [[sum(a[i][k] * b[k][j] for k in range(4)) for j in range(4)] for i in range(4)]


def load_json(path):
    f = path.read_bytes()
    n = struct.unpack("<I", f[12:16])[0]
    return json.loads(f[20 : 20 + n])


def world_aabb(j):
    """[(name, min, max)] — 메시 노드별 월드 AABB는 아니고 전체 합 AABB와 노드별 둘 다 필요해 노드별 반환."""
    nodes = j["nodes"]
    out = []

    def walk(i, acc):
        n = nodes[i]
        m = mm(acc, node_matrix(n))
        if "mesh" in n:
            mn = [1e18] * 3
            mx = [-1e18] * 3
            for p in j["meshes"][n["mesh"]]["primitives"]:
                a = j["accessors"][p["attributes"]["POSITION"]]
                if "min" not in a or "max" not in a:
                    continue
                for cx in (a["min"][0], a["max"][0]):
                    for cy in (a["min"][1], a["max"][1]):
                        for cz in (a["min"][2], a["max"][2]):
                            for k in range(3):
                                v = m[k][0] * cx + m[k][1] * cy + m[k][2] * cz + m[k][3]
                                mn[k] = min(mn[k], v)
                                mx[k] = max(mx[k], v)
            if mn[0] <= mx[0]:
                out.append((n.get("name", f"node{i}"), mn, mx))
        for c in n.get("children", []):
            walk(c, m)

    ident = [[1.0 if r == c else 0.0 for c in range(4)] for r in range(4)]
    for r in j["scenes"][j.get("scene", 0)]["nodes"]:
        walk(r, ident)
    return out


def is_zup(mn, mx):
    size = [b - a for a, b in zip(mn, mx)]
    if min(size) <= 0:
        return False
    return abs(mn[2]) < 0.02 * size[2] and mn[1] < -0.2 * size[1]


def main():
    root = Path(sys.argv[1] if len(sys.argv) > 1 else "assets/models")
    bad, checked = [], 0
    for path in sorted(root.rglob("*.glb")):
        j = load_json(path)
        if j.get("skins"):
            continue
        checked += 1
        for name, mn, mx in world_aabb(j):
            if is_zup(mn, mx):
                size = [round(b - a, 3) for a, b in zip(mn, mx)]
                bad.append(f"{path} [{name}] size={size} minY={mn[1]:.3f} minZ={mn[2]:.3f}")
    if bad:
        print("✗ Z-up 시그니처 GLB (노드 변환 소실 의심 — 옆으로 누워 렌더됨):", file=sys.stderr)
        for b in bad:
            print("  - " + b, file=sys.stderr)
        sys.exit(1)
    print(f"✓ GLB up축 정합 — {checked}개 검사")


if __name__ == "__main__":
    main()
