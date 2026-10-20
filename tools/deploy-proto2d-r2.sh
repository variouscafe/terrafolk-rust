#!/usr/bin/env bash
# proto2d R2 배포 — Pages 25MiB 한도 우회(2026-10-21).
# wasm은 R2(terrafolk-wasm 버킷, pub-…r2.dev 공개 도메인+CORS *)에서,
# 나머지 정적은 Pages에서. wrangler 페이지 배포 바인딩이 비대화면에서
# 빈값으로 덮어쓰는 함정 때문에 함수 프록시 대신 r2.dev 직접 참조 방식.
# ⚠️ wasm-opt 미적용(사용자 지시 — 0.20 그래픽 회귀 원인 가설 검증 중).
set -euo pipefail
P="$(cd "$(dirname "$0")/../proto2d" && pwd)"
cd "$P"
R2BASE="https://pub-04c1d3aebd34468d8c97384d2b60a289.r2.dev"

trunk build --release
W=$(basename "$(ls dist/*_bg.wasm | head -1)")

# ① R2 업로드
npx wrangler r2 object put "terrafolk-wasm/$W" --file "dist/$W" --remote

# ② glue — index.html의 wasm 참조를 R2 절대 URL로
# (원본 백업 — ⑥에서 복원. 못 복원하면 로컬 8095가 R2 wasm=프로덕션을
# 물고 있어 "로컬 테스트"가 프로덕션 검증으로 둔갑한다 — 2026-10-21 실측.
# 백업은 dist 밖 — dist 안에 두면 Pages 업로드 트리에 끼어든다)
GLUE_BAK=$(mktemp)
cp dist/index.html "$GLUE_BAK"
python3 - "$W" "$R2BASE" <<'PY'
import sys, re, hashlib, base64
w, r2 = sys.argv[1], sys.argv[2]
src = open('dist/index.html').read()
new = re.sub(r"module_or_path: '[^']*'", f"module_or_path: '{r2}/{w}'", src)
new = re.sub(r'href="[^"]*_bg\.wasm"( crossorigin="anonymous")?', f'href="{r2}/{w}" crossorigin="anonymous"', new)
# ③ SRI — R2 본문 해시로 재계산(wasm preload 무결성)
h = base64.b64encode(hashlib.sha384(open(f'dist/{w}','rb').read()).digest()).decode()
new2, n = re.subn(r'(href="[^"]*' + re.escape(w) + r'"[^>]*integrity=")sha384-[^"]+', r'\g<1>sha384-' + h, new)
assert n == 1, f'SRI 대상 {n}건'
open('dist/index.html', 'w').write(new2)
print('glue →', f'{r2}/{w}'[:80])
PY

# ④ wasm은 dist 밖으로(하위 디렉터리도 업로드 트리에 포함된다 — /tmp 대피)
TMPD=$(mktemp -d)
[ -d dist/wasm ] && mv dist/wasm "$TMPD/wasm_old"
mv "dist/$W" "$TMPD/"

# ⑤ Pages 배포(HTML/JS/에셋)
npx wrangler pages deploy dist --project-name terrafolk-2d \
  --branch main --commit-dirty=true

# ⑥ 대피 복구(로컬 8095 서빙용) — glue 원문도 복원(상대 경로 복귀)
mv "$TMPD/$W" dist/ 2>/dev/null || true
[ -d "$TMPD/wasm_old" ] && mv "$TMPD/wasm_old" dist/wasm
[ -f "$GLUE_BAK" ] && mv "$GLUE_BAK" dist/index.html
rm -rf "$TMPD"

echo "배포 완료 → https://terrafolk-2d.pages.dev (wasm: R2/$W)"
