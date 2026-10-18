#!/usr/bin/env python3
"""GLB inverseBindMatrix 재계산 — 스킨 바인드포즈 복구.

배경(2026-10-18 진단): normalize_glb_units.py의 IBM 갱신식이 정합을 깨뜨려
스킨 정점이 원점 부근 납작 팬케이크(~24cm×1cm)로 붕괴 — 주민이 화면에 안
보이는 실제 원인(재질·스폰·렌더 파이프라인은 모두 정상이었다).

수리: 각 skin의 모든 관절에 대해 IBM = inverse(bind_global(node))을
정의대로 재계산해 제자리 재작성(레이아웃 불변 — f32 MAT4 × 관절 수).
애니메이션·정점·노드는 무변경. 바인드포즈에서 스킨 결과 == 원본 정점이
되는 것이 수학적 보증.

사용: python3 tools/fix_glb_ibms.py assets/models/VillagerMan.glb ...
"""

import json
import struct
import sys

JSON_CHUNK = 0x4E4F534A  # "JSON"
BIN_CHUNK = 0x004E4942  # "BIN"


def read_glb(path):
    with open(path, "rb") as f:
        data = f.read()
    magic, _ver, _len = struct.unpack("<III", data[:12])
    if magic != 0x46546C67:
        raise SystemExit(f"{path}: GLB가 아님")
    off, json_bytes, bin_bytes = 12, None, None
    while off < len(data):
        clen, ctype = struct.unpack("<II", data[off : off + 8])
        chunk = data[off + 8 : off + 8 + clen]
        if ctype == JSON_CHUNK:
            json_bytes = chunk
        elif ctype == BIN_CHUNK:
            bin_bytes = bytearray(chunk)
        off += 8 + clen
    return json.loads(json_bytes), bin_bytes


def write_glb(path, meta, bin_bytes):
    j = json.dumps(meta, separators=(",", ":")).encode()
    if len(j) % 4:
        j += b" " * (4 - len(j) % 4)
    if len(bin_bytes) % 4:
        bin_bytes += b"\x00" * (4 - len(bin_bytes) % 4)
    out = struct.pack("<III", 0x46546C67, 2, 12 + 8 + len(j) + 8 + len(bin_bytes))
    out += struct.pack("<II", len(j), JSON_CHUNK) + j
    out += struct.pack("<II", len(bin_bytes), BIN_CHUNK) + bytes(bin_bytes)
    with open(path, "wb") as f:
        f.write(out)


def node_matrix(n):
    if "matrix" in n:
        # glTF는 열우선 저장 — numpy 행우선으로 전치해 반환
        return transpose4(n["matrix"])
    t = n.get("translation", [0, 0, 0])
    q = n.get("rotation", [0, 0, 0, 1])
    s = n.get("scale", [1, 1, 1])
    x, y, z, w = q
    R = [
        [1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w)],
        [2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w)],
        [2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y)],
    ]
    M = identity4()
    for i in range(3):
        for j in range(3):
            M[i][j] = R[i][j] * s[j]
    M[0][3], M[1][3], M[2][3] = t
    return M


def identity4():
    return [[1.0 if i == j else 0.0 for j in range(4)] for i in range(4)]


def transpose4(col_major):
    # glTF matrix 배열은 열우선 16개 — 행우선 4x4로: [c0 c1 c2 c3] 열들
    cols = [col_major[k * 4 : k * 4 + 4] for k in range(4)]
    return [[cols[j][i] for j in range(4)] for i in range(4)]


def mat_mul(A, B):
    return [
        [sum(A[i][k] * B[k][j] for k in range(4)) for j in range(4)] for i in range(4)
    ]


def invert4(M):
    # 가우스-조던(행우선 4x4)
    A = [row[:] + [1.0 if i == j else 0.0 for j in range(4)] for i, row in enumerate(M)]
    for col in range(4):
        pivot = max(range(col, 4), key=lambda r: abs(A[r][col]))
        if abs(A[pivot][col]) < 1e-12:
            raise SystemExit(f"퇴화 행렬 — 관절 scale 0? col={col}")
        A[col], A[pivot] = A[pivot], A[col]
        p = A[col][col]
        A[col] = [v / p for v in A[col]]
        for r in range(4):
            if r != col and A[r][col] != 0.0:
                f = A[r][col]
                A[r] = [v - f * w for v, w in zip(A[r], A[col])]
    inv = [row[4:] for row in A]
    # 행우선 → 열우선 저장
    return [inv[j][i] for i in range(4) for j in range(4)]


def global_matrix(nodes, i, cache):
    if i in cache:
        return cache[i]
    chain = []
    j = i
    while True:
        chain.append(j)
        j = nodes[j].get("parent")
        if j is None:
            break
    M = identity4()
    for k in reversed(chain):
        M = mat_mul(M, node_matrix(nodes[k]))
    for k in chain:
        cache[k] = M  # 체인 마디는 동일 글로벌(부분곱은 재계산 — 근사 캐시 아님)
    # 정확성: 체인 전체에 대해 순차 누적 저장
    acc = identity4()
    for k in reversed(chain):
        acc = mat_mul(acc, node_matrix(nodes[k]))
        cache[k] = acc
    return cache[i]


def fix(path):
    meta, bin_bytes = read_glb(path)
    nodes = meta["nodes"]
    cache = {}
    changed = 0
    for skin in meta.get("skins", []):
        acc_idx = skin["inverseBindMatrices"]
        acc = meta["accessors"][acc_idx]
        bv = meta["bufferViews"][acc["bufferView"]]
        base = acc.get("byteOffset", 0) + bv.get("byteOffset", 0)
        stride = bv.get("byteStride", 0) or 64
        n = len(skin["joints"])
        if acc["count"] < n:
            raise SystemExit(f"{path}: IBM 부족({acc['count']}<{n})")
        for k, joint in enumerate(skin["joints"]):
            G = global_matrix(nodes, joint, cache)
            ibm = invert4(G)
            off = base + k * stride
            bin_bytes[off : off + 64] = struct.pack("<16f", *ibm)
            changed += 1
        # accessor min/max 동기(선택 — 뷰어용)
    write_glb(path, meta, bin_bytes)
    print(f"{path}: IBM {changed}개 재계산 완료")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    for p in sys.argv[1:]:
        fix(p)
