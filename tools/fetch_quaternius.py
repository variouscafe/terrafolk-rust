#!/usr/bin/env python3
"""Quaternius 에셋 폴러 — poly.pizza 번들 API에서 모델 목록을 받아 GLB를 내려받는다.

원천(모두 CC0 · Quaternius):
  Stylized Nature MegaKit  — 번들 T34GZFA0fm(맵 자연 9종 사용 중)
  Animated Animal Pack     — 번들 ILAPXeUYiS(사슴 계열 2종 사용 중)

사용법:
  python3 tools/fetch_quaternius.py            # 번들 JSON 새로고침 + 전량 받기
  python3 tools/fetch_quaternius.py --refresh  # JSON만 갱신

받은 파일은 tools/quaternius-pool/<kit>/ 에 쌓인다. 맵에 실제로 쓰는 변형만
assets/models/{qnature,qanimal}/ 로 복사해 사용한다(APK는 assets/ 전체가
주입되므로 미사용분은 pool에 둔다). urllib은 403 차단이라 curl 서브프로세스
경유 — 환경 의존 최소화.
"""
import json
import os
import re
import subprocess
import sys
import time

HERE = os.path.dirname(os.path.abspath(__file__))
POOL = os.path.join(HERE, "quaternius-pool")
KITS = {
    "qnature": "T34GZFA0fm",   # Stylized Nature MegaKit
    "qanimal": "ILAPXeUYiS",   # Animated Animal Pack
}
UA = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36"


def curl(url, out=None, timeout=30):
    cmd = ["curl", "-s", "-L", "--max-time", str(timeout), "-A", UA]
    if out:
        cmd += ["-o", out]
    else:
        cmd += [url]
        r = subprocess.run(cmd, capture_output=True, text=True, timeout=timeout + 10)
        return r.stdout
    cmd.append(url)
    subprocess.run(cmd, timeout=timeout + 10, check=True)


def fetch(kit, bundle_id, refresh):
    os.makedirs(os.path.join(POOL, kit), exist_ok=True)
    meta = os.path.join(POOL, f"{kit}.json")
    if refresh or not os.path.exists(meta):
        curl(f"https://poly.pizza/api/list/{bundle_id}", out=meta)
    models = json.load(open(meta))["Models"]
    seen, ok = {}, 0
    for m in models:
        pid, title = m["publicID"], m["title"]
        n = seen.get(title, 0)
        seen[title] = n + 1
        safe = re.sub(r"[^A-Za-z0-9]+", "-", title).strip("-").lower()
        out = os.path.join(POOL, kit, f"{safe}-{n}.glb" if kit == "qnature" else f"{safe}.glb")
        if os.path.exists(out) and os.path.getsize(out) > 1000:
            ok += 1
            continue
        page = curl(f"https://poly.pizza/m/{pid}", timeout=15)
        urls = re.findall(r"https://static\.poly\.pizza/[a-f0-9-]+\.glb", page)
        if not urls:
            print("MISS", title, pid)
            continue
        curl(urls[0], out=out)
        ok += 1
        time.sleep(0.2)
    print(f"[{kit}] {ok}/{len(models)} 확보")


if __name__ == "__main__":
    refresh = "--refresh" in sys.argv
    for kit, bid in KITS.items():
        fetch(kit, bid, refresh)
