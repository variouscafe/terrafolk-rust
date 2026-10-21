#!/bin/bash
# Terrafolk 안드로이드 APK 원스텝 빌드 — xbuild + 에셋 주입 + 정렬 + 서명.
# 산출물: terrafolk-<버전>.apk (버전은 Cargo.toml [package] version 단일 원천).
# 함정 메모: 시스템 gradle 9는 xbuild 플러그인과 불호환(gradle@8 사용),
# NDK llvm-ar 경로 주입 필수, manifest의 assets는 gradle 모드에서 무시됨.
set -euo pipefail
cd "$(dirname "$0")"
ROOT=$(cd .. && pwd)
export PATH="/opt/homebrew/opt/gradle@8/bin:$HOME/.cargo/bin:$PATH"
export ANDROID_HOME=${ANDROID_HOME:-/opt/homebrew/share/android-commandlinetools}
export ANDROID_NDK_HOME=${ANDROID_NDK_HOME:-$ANDROID_HOME/ndk/28.2.13676358}
export NDK_HOME=$ANDROID_NDK_HOME
export PATH="$ANDROID_NDK_HOME/toolchains/llvm/prebuilt/darwin-x86_64/bin:$PATH"

# 버전 규칙 terrafolk-x.y.z.apk — Cargo.toml이 단일 원천(수정은 여기서만)
VERSION=$(grep -m1 '^version' "$ROOT/Cargo.toml" | sed 's/version = "\(.*\)"/\1/')
OUT="$ROOT/terrafolk-$VERSION.apk"

# 에셋 참조 정합 — src가 참조하는 에셋이 지워졌으면 빌드 전 중단
# (2026-10-21 하얀 맵 사고: cc2895d 에셋 삭제를 아무 게이트도 못 잡음).
"$ROOT/tools/check-assets.sh"

echo '[1/4] xbuild (rust+gradle apk) — 추가 플래그: $*'
x build --platform android --arch arm64 --release --format apk "$@" 
APK=$ROOT/target/x/release/android/mobile.apk

echo '[2/4] 에셋 주입 (manifest assets는 gradle에서 무시되므로 직접)'
cp "$APK" /tmp/terrafolk_inject.apk
(cd "$ROOT" && zip -qr /tmp/terrafolk_inject.apk assets)

BT=$ANDROID_HOME/build-tools/35.0.0
echo '[3/4] zipalign'
$BT/zipalign -f 4 /tmp/terrafolk_inject.apk /tmp/terrafolk_aligned.apk

echo "[4/4] debug 서명 → $OUT (배포 서명은 release keystore로 교체할 것)"
$BT/apksigner sign --ks ~/.android/debug.keystore --ks-pass pass:android \
  --key-pass pass:android --out "$OUT" /tmp/terrafolk_aligned.apk

# 이전 버전 산출물 정리 — 새 APK/AAB가 생기면 나머지 버전 파일은 모두
# 제거(누적 방지 · 2026-10-18 사용자 규칙). 현재 버전만 남긴다.
find "$ROOT" -maxdepth 1 -type f \( -name 'terrafolk-*.apk' -o -name 'terrafolk-*.aab' \) \
  ! -name "terrafolk-$VERSION.apk" -print -delete
rm -f /tmp/terrafolk_inject.apk /tmp/terrafolk_aligned.apk

echo "완료: $OUT ($(du -h "$OUT" | cut -f1)) — adb: adb install -r $OUT"
