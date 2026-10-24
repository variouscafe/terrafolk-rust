# 3D 웹/앱 패리티 감사 (2026-10-24)

웹(terrafolk-3d DOM UI)과 네이티브 APK(bevy_ui)의 기능 대조 — 웹 우선
원칙(CLAUDE.md)에서 앱은 레거시 FE 패리티가 목표. 코드 레벨 전수 대조 결과.

## 동등 (차이 없음)

| 기능 | 웹 | 앱 |
|---|---|---|
| 주민 목록·선택·follow | tf-chars.js | ui/chars.rs |
| 주민 상세(욕구·목표·챗) | tf-chars.js | ui/chars.rs(SelPanel·ChatBar) |
| 문명 팝업·연대기·**유물 카드** | tf-civ.js | ui/civ.rs(relic.card.*) |
| 자원·기록 패널 | tf-reslog.js | ui/res.rs·log.rs |
| 미니맵 | tf-minimap.js | ui/minimap.rs |
| 레일(활동) | tf-rail.js | ui/rail.rs |
| HUD(시계·날씨·인구·넷) | tf-world-ui.js | ui/hud.rs |
| 설정·계정 로그인·i18n | tf-set.js | ui/menu.rs(AccountPopup) |
| 발명품 3D 프롭(§17.86) | 원격 GLB | net/inventions.rs(데스크톱·안드로이드 우선) |
| 계절 앰비언스 연출 | tf-ambience.js(DOM 보정) | 엔진 sky/daynight로 별도 구현(동등) |

## 격차 1건 — 외형 편집(look editor) 앱 부재

웹(tf-ui.js lookEdit + world/look_edit.rs 브리지)은 계정 주민의 lookJson
parts(피부·머리색·눈·옷색)·wears를 편집·저장. 앱은 렌더 반영(look_tint.rs
재스폰 체인, PR #78)만 있고 **편집 UI·저장 경로가 없다**.

### 구현 설계(착수 예정 — Phase 나눠 진행)

1. **Phase 1 — 진입·뼈대**: 룸 프레임에 `mine` 플래그 부재(웹은 tf-ui가
   계정 정보로 계산, worldsim.js에도 없음 → 앱은 계정 API에서 내 주민
   agentId를 받아 LiveAgentTag와 대조). SelPanel에 내 주민일 때만
   "외형 편집" 버튼 → `ui/look.rs` 신규(500줄 규칙) 팝업 뼈대·탭 3종
   (피부·머리·눈 — 옷은 wears 관장이라 수동 parts 금지, FE 규약).
2. **Phase 2 — 팔레트·미리보기**: look_tint.rs LPC 팔레트 재사용(단일
   원천), 선택 시 기존 look 변경 재스폰 체인으로 즉시 반영.
3. **Phase 3 — 저장**: `PATCH /api/worldsim/agents/{id}/look`
   `{parts, wears}`(tf-ui.js lookSave 계약 그대로) — 네이티브 인증
   토큰(auth.json)으로 http_poll 패턴 전용 스레드.

## 감아야 할 함정

- 옷 탭 저장 시 parts의 top/bottom/feet* 키는 **삭제 후** wears만 송신
  (소지 파생 우선 — FE LookEditor 규약, tf-ui.js 941-953).
- 헤어 스타일 N종은 미완(지오메트리 1차만 — look-tint-3d-pipeline 메모).
- 스펙 문서: ../terrafolk docs/data-contract.md §17.32(lookJson parts).
