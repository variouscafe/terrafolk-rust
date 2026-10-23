#!/usr/bin/env bash
# 3D 클라이언트(terrafolk_bevy) R2 배포 — proto2d 레시피 동일 적용(2026-10-21).
# wasm은 R2(terrafolk-wasm 버킷, pub-…r2.dev 공개 도메인+CORS *)에서,
# 나머지 정적(GLB 에셋·DOM UI 포함)은 Pages(terrafolk-3d 프로젝트)에서.
# ⚠️ 빌드 플래그 변경 금지(사용자 지시 2026-10-21) — 루트 Trunk.toml 그대로.
set -euo pipefail
R="$(cd "$(dirname "$0")/.." && pwd)"
cd "$R"
R2BASE="https://pub-04c1d3aebd34468d8c97384d2b60a289.r2.dev"
# 게임 버전(단일 원천 Cargo.toml — 하단 배지와 동일 출처) — 배포 보고용.
V="$(grep -m1 '^version' Cargo.toml | sed 's/version *= *"\(.*\)"/\1/')"

"$R/tools/check-assets.sh"   # 하얀 맵 사고(2026-10-21) 재발 방지 — 빌드 전 정합
trunk build --release
W=$(basename "$(ls dist/*_bg.wasm | head -1)")

# ① R2 업로드 — gzip + 원문 캐시(해시 파일명=불변). proto2d와 동일 버킷·규칙.
gzip -9 -c "dist/$W" > "$W.gz"
npx wrangler r2 object put "terrafolk-wasm/$W" --file "$W.gz" --remote \
  --content-type application/wasm --content-encoding gzip \
  --cache-control "public, max-age=31536000, immutable"
rm -f "$W.gz"

# ② glue — index.html의 wasm 참조를 R2 절대 URL로
# (원본 백업 — ⑥에서 복원. 백업은 dist 밖 — dist 안에 두면 업로드 트리에 끼어든다)
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

# ④ 버전 지문 — 부팅 버전 meta 주입 + /version.json(tf-update.js 폴링용).
#    낡은 wasm 캐시로 고장나는 사고(2026-10-22 '하늘색만 보임')를 감지·갱신
#    가이드로 예방한다. wasm 해시=배포마다 단조 변화.
GITV=$(git -C "$R" rev-parse --short HEAD)
python3 - "$W" "$GITV" <<'PY'
import sys, time, json, re
w, gitv = sys.argv[1], sys.argv[2]
src = open('dist/index.html').read()
v = gitv + '-' + w.replace('_bg.wasm', '').split('-')[-1]
if 'name="tf-version"' not in src:
    src = src.replace('<head>', '<head>\n    <meta name="tf-version" content="%s">' % v, 1)
else:
    src = re.sub(r'content="[^"]*"([^>]*name="tf-version")', r'content="%s"\g<1>' % v, src)
    src = re.sub(r'(name="tf-version"[^>]*content=")[^"]*', r'\g<1>%s' % v, src)
open('dist/index.html', 'w').write(src)
open('dist/version.json', 'w').write(json.dumps({'v': v, 't': int(time.time())}))
print('version →', v)
PY

# ⑤ wasm은 dist 밖으로(하위 디렉터리도 업로드 트리에 포함된다 — /tmp 대피)
TMPD=$(mktemp -d)
[ -d dist/wasm ] && mv dist/wasm "$TMPD/wasm_old"
mv "dist/$W" "$TMPD/"

# ⑥ Pages 배포(HTML/JS/GLB 에셋) — 명시적 프로젝트 지정
# (루트 wrangler.jsonc name=terrafolk-rust 를 따라가지 않게 CLI로 덮는다)
npx wrangler pages deploy dist --project-name terrafolk-3d \
  --branch main --commit-dirty=true

# ⑦ 대피 복구(로컬 8080 서빙용) — glue 원문도 복원(상대 경로 복귀)
mv "$TMPD/$W" dist/ 2>/dev/null || true
[ -d "$TMPD/wasm_old" ] && mv "$TMPD/wasm_old" dist/wasm
[ -f "$GLUE_BAK" ] && mv "$GLUE_BAK" dist/index.html
rm -rf "$TMPD"

echo "배포 완료 → https://terrafolk-3d.pages.dev — 게임 버전 v$V (wasm: R2/$W)"
