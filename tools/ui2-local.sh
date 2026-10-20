#!/usr/bin/env bash
# ui2 로컬 검증 사이클 — trunk 빌드 후 dist/wasm 복사+glue 로컬화(8095 서빙용).
set -euo pipefail
P="$(cd "$(dirname "$0")/../proto2d" && pwd)"
cd "$P"
trunk build --release
mkdir -p dist/wasm
W=$(basename "$(ls dist/*_bg.wasm | head -1)")
cp "dist/$W" "dist/wasm/$W"
python3 - "$W" <<'PY'
import sys, re
w = sys.argv[1]
src = open('dist/index.html').read()
new = re.sub(r"module_or_path: '[^']*'", f"module_or_path: './wasm/{w}'", src)
new = re.sub(r'href="[^"]*_bg\.wasm"( crossorigin="anonymous")?', f'href="./wasm/{w}"', new)
open('dist/index.html', 'w').write(new)
print('local glue →', f'./wasm/{w}'[:64])
PY
