#!/usr/bin/env python3
"""Feet 메시 밑창 가중치 리밸런스 — 정강이 본(LowerLeg)이 밑창을 들어올리는
스키닝 결함 수정(2026-10-23, 7차 '뒤꿈치 들림' 잔여 근본).

원인: qchar *Feet 메시의 밑창 정점 가중치 ~31%가 LowerLeg에 묶여 있다.
Idle/Interact처럼 Foot 본 채널 없이 다리만 애니메이션하는 클립에서 무릎
굽힘이 밑창을 발목 핀보다 위로 끌어올린다(발끝 접지·뒤꿈치 부유).
Walk에서도 정강이 스윙이 부츠를 늘어뜨린다('스키 발' 역사 증상).

치료: 렌더 공간 y가 낮은 정점(밑창 영역, y<0.010m 전량 소거 → 0.022m
스무스 페이드)의 LowerLeg.L/R 가중치를 감쇠하고 4영향을 재정규화한다.
부츠 통(sha 검 높이)은 정강이 가중을 유지해 무릎 추종·이음새를 보존.

사용: python3 tools/fix_feet_weights.py assets/models/qchar/*.glb
멱등성: 재실행 시 이미 0에 수렴한 가중치를 다시 0으로 곱한다(사실상 불변).
"""

import json
import math
import struct
import sys

JSON_CHUNK = 0x4E4F534A
BIN_CHUNK = 0x004E4942
# 밑창 영역 임계(렌더 공간, 미터) — 바인드 실측: 밑창 y≈0, 부츠 통 시작 y≈0.029.
FADE_LO = 0.010
FADE_HI = 0.022


def read_glb(path):
    with open(path, "rb") as f:
        data = f.read()
    magic, _v, _l = struct.unpack_from("<III", data[:12])
    if magic != 0x46546C67:
        raise SystemExit(f"{path}: GLB가 아님")
    off, jb, bb = 12, None, None
    while off < len(data):
        clen, ctype = struct.unpack_from("<II", data, off)
        chunk = data[off + 8 : off + 8 + clen]
        if ctype == JSON_CHUNK:
            jb = chunk
        elif ctype == BIN_CHUNK:
            bb = bytearray(chunk)
        off += 8 + clen
    return json.loads(jb), bb


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


def smoothstep(lo, hi, x):
    t = max(0.0, min(1.0, (x - lo) / (hi - lo)))
    return t * t * (3.0 - 2.0 * t)


def node_matrix(n):
    """TRS → 4x4(행우선, 열벡터 관례). qchar Feet 노드는 Rx(-90°)·scale100."""
    t = n.get("translation", [0, 0, 0])
    q = n.get("rotation", [0, 0, 0, 1])
    s = n.get("scale", [1, 1, 1])
    x, y, z, w = q
    r = [
        1 - 2 * (y * y + z * z), 2 * (x * y - z * w), 2 * (x * z + y * w),
        2 * (x * y + z * w), 1 - 2 * (x * x + z * z), 2 * (y * z - x * w),
        2 * (x * z - y * w), 2 * (y * z + x * w), 1 - 2 * (x * x + y * y),
    ]
    m = [0.0] * 16
    for rr in range(3):
        for c in range(3):
            m[rr * 4 + c] = r[rr * 3 + c] * s[c]
        m[rr * 4 + 3] = t[rr]
    m[15] = 1.0
    return m


def fix(path):
    meta, bb = read_glb(path)
    nodes = meta["nodes"]
    accessors = meta["accessors"]
    buffer_views = meta["bufferViews"]
    changed = 0
    for ni, node in enumerate(nodes):
        mesh_name = ""
        if node.get("mesh") is not None:
            mesh_name = meta["meshes"][node["mesh"]].get("name", "")
        if "Feet" not in mesh_name:
            continue
        m = node_matrix(node)
        mesh = meta["meshes"][node["mesh"]]
        for prim in mesh["primitives"]:
            attrs = prim["attributes"]
            if "JOINTS_0" not in attrs or "WEIGHTS_0" not in attrs:
                continue
            skin = meta["skins"][prim.get("skin", 0)]
            joint_names = [nodes[j].get("name", "") for j in skin["joints"]]
            shin_idx = {
                i
                for i, nm in enumerate(joint_names)
                if nm in ("LowerLeg.L", "LowerLeg.R")
            }
            # 정점 위치(밑창 판정용)
            pa = accessors[attrs["POSITION"]]
            pbv = buffer_views[pa["bufferView"]]
            pbase = pa.get("byteOffset", 0) + pbv.get("byteOffset", 0)
            pstride = pbv.get("byteStride", 0) or 12
            # 가중치 버퍼
            wa = accessors[attrs["WEIGHTS_0"]]
            wbv = buffer_views[wa["bufferView"]]
            wbase = wa.get("byteOffset", 0) + wbv.get("byteOffset", 0)
            wstride = wbv.get("byteStride", 0) or 16
            ja = accessors[attrs["JOINTS_0"]]
            jbv = buffer_views[ja["bufferView"]]
            jbase = ja.get("byteOffset", 0) + jbv.get("byteOffset", 0)
            jstride = jbv.get("byteStride", 0) or 8
            n = wa["count"]
            for k in range(n):
                px, py, pz = struct.unpack_from("<fff", bb, pbase + k * pstride)
                wy = m[4] * px + m[5] * py + m[6] * pz + m[7]
                t = smoothstep(FADE_LO, FADE_HI, wy)
                if t >= 1.0:
                    continue
                idxs = struct.unpack_from("<HHHH", bb, jbase + k * jstride)
                wts = list(struct.unpack_from("<ffff", bb, wbase + k * wstride))
                hit = False
                for ii in range(4):
                    if idxs[ii] in shin_idx:
                        wts[ii] *= t
                        hit = True
                if not hit:
                    continue
                total = sum(wts)
                if total < 1e-6:
                    continue
                wts = [w / total for w in wts]
                struct.pack_into(
                    "<ffff", bb, wbase + k * wstride, *wts
                )
                changed += 1
    write_glb(path, meta, bb)
    print(f"{path}: 밑창 정강이 가중치 감쇠 {changed}정점")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    for p in sys.argv[1:]:
        fix(p)
