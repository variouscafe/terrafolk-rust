#!/usr/bin/env bash
# 3D 클라이언트(terrafolk_bevy) 웹 배포 — Pages 직행 2모드 + R2 폴백(2026-10-23 개편).
#   기본: wasm-opt(-Os+strip) 후
#     · raw ≤25MiB  → dist 그대로 Pages 직행(상대 경로·preload 무변경)
#     · raw >25MiB  → .wasm.gz 탑재 + index.html에 DecompressionStream fetch
#                     래퍼 주입(스트리밍 해제→컴파일, 로컬 raw 폴백 내장)
#     · TF_DEPLOY_R2=1 → 구 R2+glue 레시피(비상 폴백)
# ⚠️ wasm-opt는 사용자 허용(2026-10-23, 이전 금지 해제) — 플래그 고정:
#    -Os+strip+--all-features. -Oz는 UBO 손상 전례(proto2d 사고)로 금지.
set -euo pipefail
R="$(cd "$(dirname "$0")/.." && pwd)"
cd "$R"
R2BASE="https://pub-04c1d3aebd34468d8c97384d2b60a289.r2.dev"
# 게임 버전(단일 원천 Cargo.toml — 하단 배지와 동일 출처) — 배포 보고용.
V="$(grep -m1 '^version' Cargo.toml | sed 's/version *= *"\(.*\)"/\1/')"

"$R/tools/check-assets.sh"   # 하얀 맵 사고(2026-10-21) 재발 방지 — 빌드 전 정합
trunk build --release
W=$(basename "$(ls dist/*_bg.wasm | head -1)")

# ⓞ wasm-opt **미적용** — 2026-10-23 사고: --all-features로만 binaryen 검증이
#    통과(rustc 1.97 GC/참조형)되는데, all-features는 **compact-import
#    인코딩으로 재인코딩**해 현재 Chrome(V8 기본 비활성)이 'Invalid import
#    kind 127'로 컴파일 거부 — 라이브 다운. gz 루트에서 opt 이득은 전송량
#    8.6→8.4MiB뿐(무의미)이라 위험만 제거한다. 재도입 시: 최소 feature
#    플래그 이분 탐색 + 브라우저 부팅 게이트 필수.
WSIZE=$(stat -f %z "dist/$W")
echo "wasm: $((WSIZE / 1048576))MiB (rustc LTO 그대로)"

USE_R2="${TF_DEPLOY_R2:-0}"
GLUE_BAK=""
ROUTE=""

if [ "$USE_R2" = "1" ]; then
  ROUTE="r2"
  echo "→ R2 레시피(TF_DEPLOY_R2=1 폴백)"
  # ① R2 업로드 — gzip + 원문 캐시(해시 파일명=불변)
  gzip -9 -c "dist/$W" > "$W.gz"
  npx wrangler r2 object put "terrafolk-wasm/$W" --file "$W.gz" --remote \
    --content-type application/wasm --content-encoding gzip \
    --cache-control "public, max-age=31536000, immutable"
  rm -f "$W.gz"
  # ② glue — index.html wasm 참조를 R2 절대 URL로(백업은 dist 밖)
  GLUE_BAK=$(mktemp)
  cp dist/index.html "$GLUE_BAK"
  python3 - "$W" "$R2BASE" <<'PY'
import sys, re, hashlib, base64
w, r2 = sys.argv[1], sys.argv[2]
src = open('dist/index.html').read()
new = re.sub(r"module_or_path: '[^']*'", f"module_or_path: '{r2}/{w}'", src)
new = re.sub(r'href="[^"]*_bg\.wasm"( crossorigin="anonymous")?', f'href="{r2}/{w}" crossorigin="anonymous"', new)
h = base64.b64encode(hashlib.sha384(open(f'dist/{w}','rb').read()).digest()).decode()
new2, n = re.subn(r'(href="[^"]*' + re.escape(w) + r'"[^>]*integrity=")sha384-[^"]+', r'\g<1>sha384-' + h, new)
assert n == 1, f'SRI 대상 {n}건'
open('dist/index.html', 'w').write(new2)
print('glue →', f'{r2}/{w}'[:80])
PY
elif [ "$WSIZE" -le $((25 * 1024 * 1024)) ]; then
  ROUTE="raw"
  echo "→ Pages 직행(raw ≤25MiB — 래퍼·glue 불필요)"
else
  ROUTE="gz"
  echo "→ Pages 직행(gzip) — .wasm.gz + DecompressionStream 래퍼"
  gzip -9 -c "dist/$W" > "dist/$W.gz"
  GLUE_BAK=$(mktemp)
  cp dist/index.html "$GLUE_BAK"
  # ③ gz-래퍼 glue — ① wasm preload 링크 제거(.wasm은 탑재 안 함 — .gz만),
  #    ② <body> 최상단에 fetch 래퍼 주입(클래식 스크립트 = 모듈 로더보다 선행).
  #    래퍼: _bg.wasm fetch를 .wasm.gz로 우회해 스트리밍 해제→application/wasm
  #    Response 반환(스트리밍 컴파일 유지). .gz 부재(로컬 dist)면 raw 폴백.
  python3 - "$W" <<'PY'
import sys, re
w = sys.argv[1]
src = open('dist/index.html').read()
src2, n = re.subn(r'<link[^>]*href="[^"]*' + re.escape(w) + r'"[^>]*/?>\s*', '', src)
print('preload 제거:', n)
wrapper = """<script>
(function () {
  var orig = window.fetch;
  window.fetch = function (input, init) {
    var u = typeof input === 'string' ? input : (input && input.url) || '';
    if (/_bg\\.wasm(\\?|$)/.test(u) && typeof DecompressionStream !== 'undefined') {
      var gz = u.replace(/(_bg\\.wasm)(\\?|$)/, '$1.gz$2');
      return orig(gz, init).then(function (r) {
        if (r.ok) return new Response(
          r.body.pipeThrough(new DecompressionStream('gzip')),
          { headers: { 'content-type': 'application/wasm' } });
        return orig(u, init); // .gz 미탑재(로컬 등) — raw 폴백
      });
    }
    return orig(input, init);
  };
})();
</script>"""
i = src2.find('<body')
j = src2.find('>', i) if i >= 0 else -1
assert i >= 0 and j > i, f'<body> 태그 미발견(i={i},j={j})'
src3 = src2[: j + 1] + '\n' + wrapper + src2[j + 1 :]
open('dist/index.html', 'w').write(src3)
print('gz 래퍼 주입 완료')
PY
fi

# ④ 버전 지문 — 부팅 버전 meta 주입 + /version.json(tf-update.js 폴링용).
#    낡은 wasm 캐시로 고장나는 사고(2026-10-22 '하늘색만 보임') 감지·갱신 가이드.
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

# ⑤ 업로드 트리 정리 + Pages 배포 — 명시적 프로젝트 지정(루트 wrangler.jsonc
#    name=terrafolk-rust 를 따라가지 않게 CLI로 덮는다). 하위 디렉터리도
#    업로드 트리에 포함되므로 wasm 원본(dist/wasm 등)은 /tmp 대피.
# 해시 파일명 wasm(·.gz)은 불변 — Pages 기본(max-age=0 재검증) 대신 1년 캐시.
printf '/*.wasm\n  Cache-Control: public, max-age=31536000, immutable\n\n/*.wasm.gz\n  Cache-Control: public, max-age=31536000, immutable\n' > dist/_headers
TMPD=$(mktemp -d)
[ -d dist/wasm ] && mv dist/wasm "$TMPD/wasm_old"
if [ "$ROUTE" = "gz" ] || [ "$ROUTE" = "r2" ]; then
  mv "dist/$W" "$TMPD/"   # gz: .gz만 탑재 / r2: 절대 URL 참조라 원본 불필요
fi
npx wrangler pages deploy dist --project-name terrafolk-3d \
  --branch main --commit-dirty=true

# ⑥ 대피 복구(로컬 서빙용) — glue 원문도 복원
mv "$TMPD/$W" dist/ 2>/dev/null || true
rm -f "dist/$W.gz"
[ -d "$TMPD/wasm_old" ] && mv "$TMPD/wasm_old" dist/wasm
[ -f "$GLUE_BAK" ] && mv "$GLUE_BAK" dist/index.html
rm -rf "$TMPD"

echo "배포 완료 → https://terrafolk-3d.pages.dev — 게임 버전 v$V · 경로 $ROUTE (wasm: $W)"
