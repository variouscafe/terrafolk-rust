# 레거시 FE 패리티 백로그 (2026-10-25 차분 · 2026-11-03 2차 갱신)

레거시 FE(`../terrafolk-2d-legacy/src/`, React — 2026-10-27 동결 스냅숏) 대비
3D 클라이언트(assets/web/) 미반영 차분.
1차 전수 대조(Explore 에이전트, 2026-10-25) 결과 — 팝업 6종·탑바·미니맵·채팅독·
우측 레일·크레딧은 전량 포트 완료, 아래 정밀 갭만 잔여.

## A. 구현 대상 (v0.8.30 배치)

| # | 항목 | 레거시 근거 | 3D 현황 |
|---|---|---|---|
| 1 | 채팅 플레이스홀더 내 캐릭 변형(`chat.placeholder.own`) | ChatBar.tsx:101-105 — sel.mine이면 own | ui.js:192 항상 talk (키는 tf-data에 존재) |
| 2 | 레일 상태 패널 목표 문장 MT | ChatPanels.tsx:186-190 `mt(sel.goalText)` | tf-rail.js:219-223 원문 무번역 (chars 팝업은 적용됨) |
| 3 | 레일 기억 패널 문장 MT | ChatPanels.tsx:243 `mt(m.text)` | tf-rail.js:261 원문 무번역 |
| 4 | 미니맵 툴팁 다국어(`minimap.title`) | WorldMiniMap.tsx:165 | tf-minimap.js:105 'Minimap' 하드코딩 |
| 5 | 로그인 브랜딩·관전 안내(`login.desc`·`login.spectate`) | LoginPage.tsx:16-24,48-50 | tf-set.js 계정 팝업에 버튼만 — 안내문 추가 |
| 7 | 채팅 입력창 Esc = 선택 해제 | ChatBar.tsx:107-110 | ui.js:301 blur만 (캔버스 Esc는 해제) |
| 8 | 토스트 자유문 MT 패치 | useWorldConnection.ts:76-81 | tf-world-ui.js:314-333 무번역 (템플릿 문은 양쪽 동일) |
| 9 | 전송 버튼 빈 입력 비활성 | ChatBar.tsx:112-115 | ui.js 상시 활성 |

## B. 의도적 미반영 (판단 기록)

| # | 항목 | 판단 |
|---|---|---|
| 6 | OAuth `next` 리다이렉트 파라미터 | 레거시는 로그인 '페이지'(LoginPage)의 URL next를 검증·복귀. 3D엔 페이지가 없고 계정 팝업(UI)에서 직행하므로 `next=/` 고정이 정합 — 페이지 대응물이 생기면 그때 이식. |

## 완료 기록

- v0.8.30 (2026-10-25): A표 8건 전량 구현 — 커밋 메시지 참조.
- v0.8.52 (2026-11-03): 2차 차분(레거시 동결분 10-26~27) 전량 구현 —
  ① 문명 점수 **정착 실적** 반영(완공 오두막×2≤20 + 인구≤15, 웹 tf-civ.js·
  네이티브 ui/civ.rs 양측 — 레거시 civ.ts·룸과 동일식). ② tf-data.js 재생성 —
  gen-web-data.mjs 원천을 `../terrafolk-2d-legacy`로 재지정(구 ../terrafolk
  웹 소스 제거로 스크립트 단선 상태였음). goal.build_resume·goal.chop_wood
  2종 + 거래/화폐 이벤트 체계(ev.sell·ev.buy 24종·ev.money 3종·
  ev.act.retarget) + gold_dust 어휘 반영. 라이브 실측: 발명 141·점수 177
  (구식 최대 153 초과 = 정착 실적 반영 확인).
  유보 확인: §17.80 물 리터 개정은 룸 상수(맨손 4·물통 12)라 FE 반영분 없음.
