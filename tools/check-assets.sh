#!/usr/bin/env bash
# 에셋 참조 정합 게이트 — src/가 로드하는 에셋 경로 문자열이 assets/ 트리에
# 실재하는지 빌드 전에 검사.
# 유래: cc2895d가 GLB 64종·shaders 4종·지형 아틀라스를 지웠는데 src 참조가
# 남아, 렌더 에러 생존 정책 때문에 3D 클라이언트가 "하얀 맵"으로 조용히
# 살아남음(2026-10-21 사고 — 웹·APK 어디서도 못 잡음). deploy-3d-r2.sh·
# build-apk.sh가 빌드 전 호출한다.
set -euo pipefail
R="$(cd "$(dirname "$0")/.." && pwd)"
cd "$R"

python3 - <<'PY'
import re, pathlib, sys
pat = re.compile(r'["\']([A-Za-z0-9_\-./]+\.(?:glb|gltf|png|wgsl|json|ttf|otf|woff2|jpg|jpeg|ktx2|mp3|ogg|wav|svg))["\']')
refs = set()
for f in pathlib.Path('src').rglob('*.rs'):
    for m in pat.finditer(f.read_text()):
        refs.add(m.group(1))
# 제외: http(s) URL · `..`로 시작(compile-time include_str!·include_bytes! —
# 컴파일러가 이미 보증) · 슬래시 없는 단독 파일명(런타임 저장·캐시 파일 —
# game_shot.png·auth.json) · `files/`(앱 내부 캐시 디렉터리).
# 나머지는 assets/ 루트 상대 경로로 실재해야 한다(DOM UI assets/web도 같은
# 트리 — 런타임 소비지만 참조 정합에 포함해도 무해).
refs = {p for p in refs
        if '/' in p
        and not p.startswith(('http://', 'https://', '..', 'files/'))}
missing = sorted(p for p in refs if not (pathlib.Path('assets') / p).exists())
if missing:
    print(f'✗ src 참조 에셋이 assets/에 없음 ({len(refs)}건 중):', file=sys.stderr)
    for p in missing:
        print(f'  - {p}', file=sys.stderr)
    sys.exit(1)
print(f'✓ 에셋 참조 정합 — src 참조 {len(refs)}건 전부 assets/에 실재')
PY
