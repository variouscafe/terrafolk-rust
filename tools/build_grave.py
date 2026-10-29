#!/usr/bin/env python3
"""무덤(Grave) 절차 생성기 — KayKit 자산 완전 철퇴(2026-10-29 사용자 지시).

구 Grave.glb(서드파티 CC0)를 자체 생성 묘비로 교체한다. 룩 기준 =
현행 그래픽 레퍼런스(2026-10-29 지브리풍 해안 마을): 웜크림 석재·
테라코타 포인트·올리브 그린 잔디·부드러운 둥근 아치. 무텍스처 플랫
컬러 + 페이스 노멀 — folk(build_folk.py)와 동일한 렌더 계약.

사용: python3 tools/build_grave.py
출력: assets/models/Grave.glb (모델 target_h 1.3 — models.rs 오토폿 기준)
"""

import json
import math
import os
import struct

def vsub(a, b): return [a[0]-b[0], a[1]-b[1], a[2]-b[2]]
def vdot(a, b): return a[0]*b[0]+a[1]*b[1]+a[2]*b[2]
def vcross(a, b):
    return [a[1]*b[2]-a[2]*b[1], a[2]*b[0]-a[0]*b[2], a[0]*b[1]-a[1]*b[0]]
def vnorm(a):
    l = math.sqrt(vdot(a, a)) or 1.0
    return [a[0]/l, a[1]/l, a[2]/l]

class Mesh:
    """삼각형 단위 정점 복제 + 페이스 노멀(플랫 패싯) 메시."""
    def __init__(self):
        self.pos = []; self.nrm = []; self.idx = []

    def _emit(self, verts, tris):
        base = len(self.pos)
        for (a, b, c) in tris:
            p0, p1, p2 = verts[a], verts[b], verts[c]
            n = vcross(vsub(p1, p0), vsub(p2, p0))
            if vdot(n, n) < 1e-14:
                continue
            n = vnorm(n)
            i0 = len(self.pos)
            self.pos.extend([p0, p1, p2])
            self.nrm.extend([n, n, n])
            self.idx.append((i0, i0+1, i0+2))

    def box(self, cx, cy, cz, w, h, d, tilt_z=0.0):
        """중심·크기(z축 미세 기울임 — 손으로 세운 느낌)."""
        x0, x1 = cx-w/2, cx+w/2
        y0, y1 = cy-h/2, cy+h/2
        z0, z1 = cz-d/2, cz+d/2
        V = [[x0,y0,z0],[x1,y0,z0],[x1,y1,z0],[x0,y1,z0],
             [x0,y0,z1],[x1,y0,z1],[x1,y1,z1],[x0,y1,z1]]
        if tilt_z:
            t = math.radians(tilt_z)
            for p in V:
                x, y = p[0]-cx, p[1]-cy
                p[0] = cx + x*math.cos(t) - y*math.sin(t)
                p[1] = cy + x*math.sin(t) + y*math.cos(t)
        T = [(0,2,1),(0,3,2),(4,5,6),(4,6,7),(0,1,5),(0,5,4),
             (3,6,2),(3,7,6),(1,2,6),(1,6,5),(0,4,7),(0,7,3)]
        self._emit(V, T)

    def arch(self, cx, cy, cz, r, depth, seg=10):
        """반원 아치 캡 — XZ 단면 반원을 Z로 돌출(석판 상단 라운드)."""
        verts, tris = [], []
        for s in range(2):          # 0=앞면(z+), 1=뒷면(z-)
            z = cz + (depth/2 if s == 0 else -depth/2)
            for i in range(seg+1):  # 0..180°
                t = math.pi * i / seg
                verts.append([cx + r*math.cos(t), cy + r*math.sin(t), z])
        for i in range(seg):        # 측면(원주형 스트립)
            a, b = i, i+1
            c, d = (seg+1)+i, (seg+1)+i+1
            tris += [(a, c, d), (a, d, b)]
        # 상단 캡(평면 부채꼴 — 위에서 보이는 면)
        for i in range(1, seg):
            tris.append((0, i+1, i))
        self._emit(verts, tris)

    def blob(self, cx, cy, cz, r, sy=1.0, seg=8):
        """납작 구 근사(잔디 언덕·꽃) — 팔면 파이 절두."""
        verts = []
        for j, yy in enumerate((-0.5, -0.18, 0.22, 0.5)):
            rr = r * math.sqrt(max(0.0, 1 - (2*yy)**2))
            for i in range(seg):
                t = 2*math.pi*i/seg
                verts.append([cx + rr*math.cos(t), cy + yy*r*2*sy, cz + rr*math.sin(t)])
        tris = []
        for j in range(3):
            r0, r1 = j*seg, (j+1)*seg
            for i in range(seg):
                a, b = r0+i, r0+(i+1) % seg
                c, d = r1+i, r1+(i+1) % seg
                tris += [(a, c, d), (a, d, b)]
        top = (len(verts)-seg + len(verts)) // 2
        self._emit(verts, tris)

# ── 팔레트(현행 레퍼런스 — 웜크림 석재·테라코타·올리브) ───────────────
MATS = {
    "stone":  [0.914, 0.871, 0.784],   # 웜크림 석판 #E9DFC7
    "stone_d":[0.780, 0.724, 0.639],   # 밑단 어두운 크림
    "terra":  [0.851, 0.373, 0.169],   # 테라코타 꽃 #D95F2B
    "grass":  [0.478, 0.561, 0.310],   # 올리브 잔디
    "stem":   [0.333, 0.447, 0.220],   # 줄기 어두운 올리브
}

def build_parts():
    """(재질키, Mesh) 목록 — 바닥 y0 기준."""
    P = []
    # 석판 — 아치 상단 + 몸통 박스(살짝 기울임 = 손으로 세운 정겨움).
    slab = Mesh()
    slab.arch(0.0, 0.98, 0.0, r=0.30, depth=0.13)
    slab.box(0.0, 0.68, 0.0, 0.60, 0.62, 0.13, tilt_z=2.5)
    P.append(("stone", slab))
    # 밑단 받침 — 넓고 낮은 어두운 크림.
    base = Mesh()
    base.box(0.0, 0.09, 0.0, 0.78, 0.18, 0.30, tilt_z=2.5)
    P.append(("stone_d", base))
    # 잔디 언덕 — 받침을 감싸는 올리브 납작 blob 2.
    for sx in (-0.42, 0.40):
        g = Mesh()
        g.blob(sx, 0.10, 0.06, r=0.16, sy=0.55)
        P.append(("grass", g))
    # 꽃 1 — 테라코타 점 + 줄기(석판 옆).
    P.append(("stem", Mesh()))
    P[-1][1].box(0.30, 0.34, 0.05, 0.025, 0.22, 0.025)
    fl = Mesh()
    fl.blob(0.30, 0.47, 0.05, r=0.055, sy=0.8)
    P.append(("terra", fl))
    return P

def main():
    parts = build_parts()
    buf = bytearray()
    views, accessors, meshes, nodes = [], [], [], []
    for mi, (mat, mesh) in enumerate(parts):
        def push(data):
            while len(buf) % 4:
                buf.append(0)
            off = len(buf)
            buf.extend(data)
            views.append({"buffer": 0, "byteOffset": off, "byteLength": len(data)})
            return len(views) - 1
        pv = push(b"".join(struct.pack("<3f", *p) for p in mesh.pos))
        comps = [c for p in mesh.pos for c in p]
        pos_a = len(accessors)
        accessors.append({"bufferView": pv, "type": "VEC3", "count": len(mesh.pos),
                          "componentType": 5126,
                          "min": [min(comps[0::3]), min(comps[1::3]), min(comps[2::3])],
                          "max": [max(comps[0::3]), max(comps[1::3]), max(comps[2::3])]})
        nv = push(b"".join(struct.pack("<3f", *n) for n in mesh.nrm))
        nrm_a = len(accessors)
        accessors.append({"bufferView": nv, "type": "VEC3", "count": len(mesh.nrm),
                          "componentType": 5126})
        iv = push(b"".join(struct.pack("<I", i) for t in mesh.idx for i in t))
        idx_a = len(accessors)
        accessors.append({"bufferView": iv, "type": "SCALAR", "count": len(mesh.idx)*3,
                          "componentType": 5125})
        meshes.append({"name": f"grave_{mat}", "primitives": [{
            "attributes": {"POSITION": pos_a, "NORMAL": nrm_a},
            "indices": idx_a, "material": list(MATS).index(mat)}]})
        nodes.append({"name": f"grave_{mat}", "mesh": mi})
    materials = []
    for key, c in MATS.items():
        materials.append({"name": f"grave_{key}",
                          "pbrMetallicRoughness": {
                              "baseColorFactor": [pow(x, 2.2) for x in c] + [1.0],
                              "metallicFactor": 0.0, "roughnessFactor": 1.0}})
    out = {
        "asset": {"version": "2.0", "generator": "build_grave.py"},
        "scene": 0,
        "scenes": [{"name": "Scene", "nodes": list(range(len(nodes)))}],
        "nodes": nodes,
        "meshes": meshes,
        "materials": materials,
        "accessors": accessors,
        "bufferViews": views,
        "buffers": [{"byteLength": len(buf)}],
    }
    os.makedirs("assets/models", exist_ok=True)
    jb = json.dumps(out, separators=(",", ":")).encode()
    jb += b" " * ((4 - len(jb) % 4) % 4)
    with open("assets/models/Grave.glb", "wb") as f:
        f.write(b"glTF")
        f.write(struct.pack("<II", 2, 12 + 8 + len(jb) + 8 + len(buf)))
        f.write(struct.pack("<I", len(jb)))
        f.write(b"JSON")
        f.write(jb)
        f.write(struct.pack("<I", len(buf)))
        f.write(b"BIN\0")
        f.write(buf)
    print(f"✓ assets/models/Grave.glb — meshes {len(meshes)} · {len(buf)/1024:.0f}KiB")

if __name__ == "__main__":
    main()
