# 레거시 FE 패리티 백로그 (2026-10-25 차분)

레거시 FE(`../terrafolk/src/`, React) 대비 3D 클라이언트(assets/web/) 미반영 차분.
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
