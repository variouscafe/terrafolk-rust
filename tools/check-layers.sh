#!/bin/bash
# 계층 의존 방향 검사 — chartcraft xtask layers 패턴의 경량 이식(2026-10-24).
# 규칙(CLAUDE.md 설계 원칙):
#   platform, character = 게임 무관 하위 계층 → 상위(world·net·ui) import 금지
#                         ※ 재사용 계층의 순수성이 이 게이트의 핵심 목적.
#   world               = 게임 콘텐츠 → character·platform·net 허용(net의
#                         라이브 태그·시스템을 소비하는 기존 방향), ui 금지.
#   net                 = 서버 어댑터 → world 허용(스폰·적용).
# 위반 시 즉시 실패(빌드 전 게이트 — deploy-3d-r2.sh·build-apk.sh에서 호출).
set -euo pipefail
cd "$(dirname "$0")/.."
FAIL=0

deny() { # $1=검사 대상 dir, $2..=금지 prefix
  local dir="$1"; shift
  local hits
  hits=$(grep -rn "crate::" "$dir" --include="*.rs" 2>/dev/null \
    | grep -E "crate::($1)" || true)
  if [ -n "$hits" ]; then
    echo "✗ 계층 위반 [$dir] → $1:"
    echo "$hits" | head -5
    FAIL=1
  fi
}

# 하위 계층은 상위를 모름
deny src/platform 'world|net|ui|character'
# 캐릭터 프레임워크는 오픈소스 레포로 이관(2026-10-27) — 여기는 셤만
# 남았고, 순수성 게이트는 ../3d-chibi-character의 tools/check-purity.sh가 담당.
deny src/character.rs 'world|net|ui'
# 게임 콘텐츠는 UI를 모름(웹은 DOM, 네이티브 UI는 최상위)
deny src/world 'ui'

if [ "$FAIL" -eq 1 ]; then
  echo "계층 검사 실패 — docs/character-module-redesign.md §6·CLAUDE.md 설계 원칙 참고."
  exit 1
fi

# never-crash 벨트(#123·#126) — clippy 전용 린트라 cargo build는 못 잡는다.
# unwrap/expect 발화가 있으면(프로덕션 크레이트) 게이트 실패.
if cargo clippy --workspace --quiet 2>&1 | grep -qE "unwrap_used|expect_used"; then
  echo "✗ never-crash 벨트 위반 — unwrap/expect는 Result 전파로:"
  cargo clippy --workspace --quiet 2>&1 | grep -A3 "unwrap_used\|expect_used" | grep "\-\->" | head -5
  exit 1
fi

echo "✓ 계층 의존 검사 통과 (platform·character ← world ↔ net) + never-crash 벨트"
