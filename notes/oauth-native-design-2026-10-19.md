# F2 네이티브 OAuth 설계 — 2026-10-19 조사 기준

## 현황(코드 실증)
- **앱(관전)**: `native_conn.rs`가 `GET /worldsim/spectate-ticket`(무인증) → matchmake
  Bearer → 룸 `onAuth` → `kind='spectator'`(채팅은 agentId 지정 converse만)
- **웹 OAuth**: `functions/auth/google.js` — self-verifying state(`<rnd>.<b64(next)>.<ts>.<HMAC>`)
  로 쿠키 유실(앱 핸드오프) 내성 설계已完成. callback은 코드 교환 후 **토큰을 URL
  fragment에 실어 SPA 루트로 302** — 네이티브가 가로챌 수 없는 웹 경로.
- **백엔드 티켓**: `spectate-ticket`(무인증) · `player-ticket`(require_user) — 인증 티켓은
  `kind='user'`(소유 캐릭터 지시·chat directive·my_agents) 스코프.

## 전환 설계(안)
1. **브라우저 오픈(앱→웹)**: `platform/open_url` — Android Intent JNI
   (`ACTION_VIEW`, NativeActivity vm 경유). 게임 무관 platform 계층.
2. **콜백→앱 복귀(웹→앱)**: `../terrafolk` repo의 callback.js에 `?native=1`(또는 state
   세그먼트 플래그)일 때 `terrafolk://auth#access=..&refresh=..`로 리다이렉트 —
   **백엔드 1커밋 과업**(양 repo 협업 지점, state 자기검증 덕에 앱 핸드오프 이미 고려됨).
3. **딥링크 수신**: manifest에 `terrafolk` 스킴 intent-filter 추가(xbuild manifest.yaml
   지원 여부 확인 → 미지원 시 build-apk.sh 후처리로 XML 주입). 런치 인텐트 데이터는
   부팅 시 1회 판독(JNI `getIntent().getData()`)으로 충분(토큰 수령은 1회 이벤트).
4. **토큰 저장·사용**: 앱 전용 저장소(Bevy 데이터 디렉터리 파일) → 매 부팅 갱신:
   `player-ticket`(Bearer 리프레시) 실패 시 spectate 폴백 — 관전은 항상 동작 보장.
5. **UI**: Popup::Account(로그인 안내 + "브라우저로 로그인" 버튼 = open_url) · 로그인됨
   표시(이메일) · 로그아웃. ☰ 메뉴에 계정 항목 추가.

## 구현 순서(리스크 순)
1. platform/open_url(JNI) — 단독 검증 가능(에뮬: 버튼→브라우저)
2. 토큰 저장·player-ticket 폴백 — spectate 유지 회귀 무결
3. 백엔드 native 리다이렉트(별도 repo PR) + 딥링크 인텐트 → 수신 판독
4. 계정 팝업 UI 마무리

## 리스크
- xbuild manifest intent-filter 지원 불확실 → gradle 후처리 확정 필요
- winit/android-activity의 인텐트 데이터 접근 API 부재 → JNI 직접 구현
  (NativeActivity 환경에서 vm 포인터 획득 경로 사전 검증 요망)
- Google OAuth 클라이언트에 커스텀 스킴 redirect 미등록 → 웹 origin 경유
  (callback이 스킴으로 최종 302)라 등록 불필요(웹 redirect_uri 그대로 사용)

## 당분간의 실용 대안(구현 전)
웹에서 로그인 후 발급되는 토큰을 앱 설정에 수동 붙여넣기 — 개발 검증 경로로는
즉시 사용 가능(계정 팝업에 임시 토큰 입력란). 1차 출시에는 불필요(관전이 메인).
