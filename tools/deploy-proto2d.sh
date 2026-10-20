#!/usr/bin/env bash
# proto2d 웹 배포 — 확립 레시피 캡슐화(2026-10-20, assetgen 재생성 싱크가 호출).
# 절차: trunk release 빌드 → wasm-opt(-Os+strip — ⚠️ -Oz는 wasm 손상 사고로 금지)
#       → SRI integrity 재계산 → terrafolk-2d 배포.
# 사용: tools/deploy-proto2d.sh [--skip-build]  (빌드 생략=에셋만 다시 올릴 때)
set -euo pipefail
cd "$(dirname "$0")/../proto2d"

if [[ "${1:-}" != "--skip-build" ]]; then
  trunk build --release
fi

W=$(basename "$(ls dist/terrafolk_proto2d-*_bg.wasm)")   # index.html href는 './파일명' — 경로 접두 제거
wasm-opt -Os --enable-bulk-memory-opt --enable-nontrapping-float-to-int \
  --strip-debug --strip-producers "dist/$W" -o /tmp/p2d_opt.wasm
mv /tmp/p2d_opt.wasm "dist/$W"

# ⚠️ integrity는 문서에 2개(wasm preload + JS modulepreload) — wasm 링크만
# 갱신하고 검증도 wasm href의 값으로(레시피: JS 해시는 건드리지 않는다).
H=$(openssl dgst -sha384 -binary "dist/$W" | openssl base64 -A)
python3 - "$W" "$H" <<'EOF'
import re, sys
w, h = sys.argv[1], sys.argv[2]
src = open('dist/index.html').read()
new, n = re.subn(r'(href="[^"]*%s"[^>]*integrity=")sha384-[^"]+' % re.escape(w),
                 r'\g<1>sha384-' + h, src)
assert n == 1, f'integrity 갱신 대상 {n}건(1이어야)'
open('dist/index.html', 'w').write(new)
EOF

npx wrangler pages deploy dist --project-name terrafolk-2d \
  --branch main --commit-dirty=true
echo "배포 완료 → https://terrafolk-2d.pages.dev"
