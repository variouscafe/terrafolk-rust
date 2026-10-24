# 3D 웹/앱 패리티 감사 (2026-10-24, 2026-10-25 갱신)

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
| **외형 편집(look editor)** | tf-ui.js lookEdit + world/look_edit.rs | ui/look.rs·look_view.rs + net/look_sync.rs |
| 발명품 3D 프롭(§17.86) | 원격 GLB | net/inventions.rs(데스크톱·안드로이드 우선) |
| 계절 앰비언스 연출 | tf-ambience.js(DOM 보정) | 엔진 sky/daynight로 별도 구현(동등) |

## 완료 기록

- **외형 편집기 앱 이식(2026-10-24, 9e36535 → 0.8.22 출시)** — 구 격차
  1건 해소. 계정 주민 판별 → SelPanel 진입 → 팝업 탭(피부·머리·눈) →
  `PATCH /api/worldsim/agents/{id}/look` 저장까지 전 경로(ui/look.rs).
- 유지보수 유의 규약: 옷 탭 저장 시 parts의 top/bottom/feet* 키는
  **삭제 후** wears만 송신(소지 파생 우선 — FE LookEditor 규약,
  tf-ui.js 941-953). 스펙 문서: ../terrafolk docs/data-contract.md
  §17.32(lookJson parts).
