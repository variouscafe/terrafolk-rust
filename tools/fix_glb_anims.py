#!/usr/bin/env python3
"""GLB 애니메이션 translation 채널 0화 — 스틱 변형 수리(2026-10-18).

배경: normalize_glb_units.py가 스켈레톤/애님 translation 계약을 어긋나게
만들어 재생 중 관절이 날아가 스틱처럼 뭉개졌다. 주민 이동은 클라이언트
보간이 담당하므로 캐릭터 애님의 translation(대부분 루트 모션)은 0으로
두어도 관전에 지장이 없다 — 회전 채널만으로 포즈가 구성된다.

각 animation의 path=='translation' 채널이 참조하는 sampler 출력 accessor를
0으로 채운다(레이아웃 불변). rotation/scale 채널 무변경. 멱등.

사용: python3 tools/fix_glb_anims.py assets/models/VillagerMan.glb ...
"""

import json
import struct
import sys

JSON_CHUNK = 0x4E4F534A
BIN_CHUNK = 0x004E4942


def read_glb(path):
    with open(path, "rb") as f:
        data = f.read()
    magic, _v, _l = struct.unpack("<III", data[:12])
    if magic != 0x46546C67:
        raise SystemExit(f"{path}: GLB가 아님")
    off, jb, bb = 12, None, None
    while off < len(data):
        clen, ctype = struct.unpack("<II", data[off : off + 8])
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


def fix(path):
    meta, bin_bytes = read_glb(path)
    zeroed = 0
    for anim in meta.get("animations", []):
        for ch in anim.get("channels", []):
            if ch.get("target", {}).get("path") != "translation":
                continue
            sampler = anim["samplers"][ch["sampler"]]
            acc_idx = sampler["output"]
            acc = meta["accessors"][acc_idx]
            bv = meta["bufferViews"][acc["bufferView"]]
            base = acc.get("byteOffset", 0) + bv.get("byteOffset", 0)
            stride = bv.get("byteStride", 0) or 12
            n = acc["count"]
            for k in range(n):
                off = base + k * stride
                bin_bytes[off : off + 12] = b"\x00" * 12
            zeroed += 1
    write_glb(path, meta, bin_bytes)
    print(f"{path}: translation 채널 {zeroed}개 0화 완료")


if __name__ == "__main__":
    if len(sys.argv) < 2:
        raise SystemExit(__doc__)
    for p in sys.argv[1:]:
        fix(p)
