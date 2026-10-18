#!/usr/bin/env python3
"""glTF(+bin+png) → 단일 GLB 변환기 — Quaternius Universal Base Characters용.

Standard 팩은 .gltf+.bin+png로 배포된다. 에셋 파이프라인(assets/ 주입)은
단일 파일이 관리가 쉬우므로 버퍼·이미지를 전부 임베드한다.

사용법:
  python3 tools/gltf_to_glb.py <src.gltf> <out.glb>

주의: 이미지가 bufferView로 이미 임베드된 경우 그대로 유지하고, 외부
uri(png/bin)만 읽어 chunk로 추가한다. 노드·애님·스킨은 건드리지 않는다.
"""
import json
import struct
import sys
from pathlib import Path


def pad4(data: bytes) -> bytes:
    return data + b"\x00" * ((4 - len(data) % 4) % 4)


def convert(src: Path, out: Path) -> None:
    doc = json.loads(src.read_text())
    base = src.parent

    chunks: list[bytes] = []
    # 1) 버퍼 — 최초 외부 .bin을 GLB binary chunk로 흡수
    bin_data = b""
    for buf in doc.get("buffers", []):
        uri = buf.pop("uri", None)
        if uri and uri.endswith(".bin"):
            bin_data = (base / uri).read_bytes()
        buf["byteLength"] = len(bin_data)
    chunks.append(bin_data)

    # 2) 이미지 — 외부 png를 binary chunk 뒤에 append, bufferView로 지칭.
    #    byteOffset은 binary chunk 내 위치 — bin을 4정렬 패딩 후부터 시작.
    offset = (len(bin_data) + 3) & ~3
    for img in doc.get("images", []):
        uri = img.pop("uri", None)
        if not uri or not uri.lower().endswith((".png", ".jpg", ".jpeg")):
            continue  # 이미 bufferView(임베드) — 그대로
        p = base / uri
        if not p.exists():
            # Godot 변형명 폴백 — T_*_png.png ↔ T_*.png (팩 내 부재 케이스)
            alt = base / uri.replace("_png.png", ".png")
            if alt.exists():
                p = alt
            else:
                raise SystemExit(f"누락 텍스처: {p}")
        chunks.append(p.read_bytes())
        doc["bufferViews"].append(
            {"buffer": 0, "byteOffset": offset, "byteLength": len(chunks[-1])}
        )
        img["bufferView"] = len(doc["bufferViews"]) - 1
        img["mimeType"] = "image/png"
        offset += (len(chunks[-1]) + 3) & ~3

    # 3) GLB 조립 — JSON chunk(0x20 패딩) + binary chunk(0x00 패딩)
    bin_total = b"".join(
        c + b"\x00" * ((4 - len(c) % 4) % 4) for c in chunks
    )
    if not bin_total:
        bin_total = b"\x00\x00\x00\x00"
    doc["buffers"][0]["byteLength"] = len(bin_total)
    jdoc = json.dumps(doc, separators=(",", ":")).encode()
    jdoc = jdoc + b" " * ((4 - len(jdoc) % 4) % 4)
    total = 12 + 8 + len(jdoc) + 8 + len(bin_total)
    out.write_bytes(
        struct.pack("<III", 0x46546C67, 2, total)
        + struct.pack("<II", len(jdoc), 0x4E4F534A)
        + jdoc
        + struct.pack("<II", len(bin_total), 0x004E4942)
        + bin_total
    )
    print(f"{out.name}: {out.stat().st_size/1e6:.1f}MB")


if __name__ == "__main__":
    convert(Path(sys.argv[1]), Path(sys.argv[2]))
