# terrafolk-web 패리티 백로그 — 미반영 기능·설계

기준: terrafolk-web `04c885f`(2026-11-03, M4 완료) ↔ terrafolk-rust `aace763`
(0.8.59). terrafolk-web은 레거시 FE 2D 도트 계열 스택(Three.js 스프라이트),
terrafolk-rust 웹은 Bevy 3D 치비 — **기능·데이터 계약** 패리티가 기준이고
표현 양식(스프라이트 vs 3D)은 각자 유지한다.

## 1. 구현 목록 대조 (terrafolk-web 전체 ↔ rust 웹)

| terrafolk-web 기능 | rust 웹 현황 | 판정 |
|---|---|---|
| 관전 부트·재접속(티켓→룸→스냅샷) | worldsim.js 동일 구조 | ✅ 반영 |
| GID 지형 하이트필드(§17.79) | 동일 알고리즘(synthesize_relief 원본) | ✅ |
| 주민 보간·선택·follow·말풍선 | cam_follow·pick·tf-world-ui 말풍선 | ✅ |
| 미니맵 | MinimapPlugin(웹 공용) | ✅ |
| 낮밤 조명(게임시각 룩 보간) | daynight.rs(태양·달·컬러그레이딩 — 더 풍부) | ✅ |
| 날씨 6종 룩(하늘·물·포그) | precip.rs sky_mix 동일 6종 | ✅ |
| 비·눈 파티클 | 네이티브 precip_fx / 웹은 DOM 오버레이(tf-ambience) | ✅(표현 상이) |
| 물 수심·포말 | water.wgsl shore-distance 베이크 | ✅ |
| 문명 패널(시대 사다리·연대기·만든 이들) | tf-civ.js 동일 파생 | ✅ |
| 기록 패널 | tf-reslog.js(카테고리 5종 탭은 없음 — P3) | ✅(일부) |
| 유물 카드(발명 이야기) | tf-feed.js pickAt | ✅ |
| 발명품 relic 월드 표현 | FloatBob 부유 + **GLB 치환 코드 완성** | ⚠️ 웹 데이터 단절(P0-2) |
| OAuth 로그인·합류 폼·관계 패널 | tf-set·tf-chars(관계 warmth 포함) | ✅ |
| hut §17.92 집주소 필터(유령 48채 제거) | **없음** — 비개굴 hut 전부 렌더 | ❌ P1 |
| 건축 진행 연출(진행률 계산) | 발판·링·바는 있으나 **진행률 항상 0** | ❌ P1 |
| 주민 행동 바·진행률(머리 위) | DOM 링(ringSvg) 존재 — **Rust 수신 단절로 웹에서도 값 0 아님, Rust 연계만 죽음** | ⚠️ P0-1 |
| 주민 패널 행동 라벨(ACT_KO 한국어·수면) | actType 영문 원문 그대로 표시 | ❌ P1 |
| hut 성장 스케일(0.5→1.0)+비계 틴트 | rust는 단계별 절차 발판(골조·어도비) — 별도 표현 | ➖ 표현 상이(패리티 유지 불필요) |
| 동물 배위(§17.64) | RoamTo 등속 + 배회 | ✅ |
| 동물 이동 애님(Gallop) | Idle만 — "2차 과제" 주석 | ❌ P2 |
| 어류 유영(§17.43) | 물 타일 자원 영구 스킵 | ❌ P2 |
| 모닥불 동적 스폰/소멸(§17.25 이벤트) | 스냅숏 스폰만 — 'event' 메시지가 Rust로 안 흐름 | ❌ P2 |
| 자원 밀도 절제(tree 42%·plant 35%) | 나무만 15%(2026-10-26 사용자 지시 — 3D 자체 튜닝) | ➖ 의도적 상이(plant만 검토 후보 P3) |
| Capacitor APK | mobile/build-apk.sh 네이티브 APK | ➖ 플랫폼 상이 |

## 2. P0 — 웹 데이터 단결 수리 ✅ 완료(2026-11-04)

### P0-1. worldsim.js sink ↔ Rust LiveAgent 키 불일치
- **현상**: JS sink는 camelCase(`actType`·`actProgress`·`goalText`·`goalKey`·
  `needs{...}`·`inventory`)로 보내지만 Rust `LiveAgent`(src/net/mod.rs)는
  snake_case 평면 키(`act_type`·`act_progress`·`goal_text`·`goal_key`·
  `hunger`…`inv_json`)를 기대. `rename="actType"`의 `_act_type_r`는
  dead_code로 한 번도 안 읽힘 → **웹에서 Rust는 행동·목표·욕구·인벤을
  전부 못 받는다**(DOM UI는 JS 원본으로 동작해 겉으로 티 안 남).
- **설계**: sink 전용 에이전트 투영을 worldsim.js에 추가(snake_case 평면 +
  `home_id`) — DOM feed.frame(camelCase)은 무변경. Rust는 `_act_type_r`
  폐지 + `act_type`에 `alias = "actType"`, `home_id: Option<String>` 추가.
- **효과**: 웹에서 행동 애님(give/trade Wave)·AgentHud 연계가 산다.

### P0-2. sink에서 `inventions` 누락
- **현상**: worldsim.js가 inventionsJson을 파싱해 DOM에만 넘기고 sink
  페이로드에서 빠짐 → 웹에서 `LiveFrame.inventions` 항상 None →
  발명품 GLB 치환(bld_spawn.rs 원격 DynProp)이 절대 발동 안 함(정적
  relic 폴백 고정).
- **설계**: sink에 `inventions: st.inventionsJson`(원문 문자열 — Rust
  계약이 원문 파싱) 추가 한 줄.

## 3. P1 — 1차 파동 ✅ 완료(2026-11-04, 실측 완료)

- P0-1·P0-2 완료 — worldsim.js sink 전용 snake_case 투영(sinkAgents) +
  `inventions` 원문 캐시 편승. Rust `_act_type_r` 폐지·alias 통합.
- P1-1 완료 — `visual_map::hut_progress()`(STAGE_DUR 5단계 × w·h 규모).
  실측: `FRAME_READY w0 → 33%`(=(30+60)/270, web 동일식). PLANNED(미표
  단계)는 0%로 둔다 — 부지 미착공 표현. 미발견 단계 역시 0%(web과 동일
  안전 폴백).
- P1-2 완료 — §17.92 필터 3곳(3D 렌더 스폰/디스폰·미니맵·구조물 청소
  반경). **실측(2026-11-04 라이브)**: hut 행 209(개굴 50 별도) 중
  완공 182 → **유령 176 스킵·실거주 6만 렌더**·공사 27 전량 표시.
  `?tfdbg=hutprobe` 2.5s 주기 로그로 교차 검증(DOM resMap/homeId 기대치와
  정확히 일치). 유령 판정은 프레임마다 재평가(입주·사망 반영)하되
  스폰 예산은 소비하지 않는다(2026-10-26 프론티어 굶주림 사고 회피).
- P1-3 완료 — FE 정본 i18n.ts act.* 14종 추가(fetch·teach·perform 등
  26종 전거리화) + tf-chars actLabel(한국어 + 진행률 %). 실측:
  "길어오기 57%"·"음수 27%".

### P1-1. 공사 진행률 계산(hutProgress 포팅) — "집에 얼만큼 지어졌나"
- **현상**: rust는 `stateJson.progress`를 읍는데 서버는 그 필드를 안 보냄
  → 공사 바·링이 항상 0. terrafolk-web은 `stage`+`work`+`w`+`h`로
  클라 계산(resources.ts hutProgress).
- **설계**: `visual_map.rs` ResState에 `work` 추가 + `hut_progress(&st)`
  순수 함수(terrafolk-web과 동일식 — 단계표 `{SITE_PREPARED:30,
  FOUNDATION_READY:60, FRAME_READY:60, ENCLOSURE_READY:90, COMPLETED:30}`
  게임분, `scale=max(0.25,(w*h)/4)`, `req=max(1,round(dur*scale))`,
  `(완료단계 누적+min(work,현재 req))/total`). `st.progress`는 서버 직송
  예비 우선권. 적용처: bld_spawn 스폰 값 + resources.rs SiteMeta 갱신.
  res_sig는 정체성 필드만 쓰므로 work 변화→재스폰 폭풍 없음(기존 계약).

### P1-2. §17.92 hut 집주소 필터 — 유령 완공 hut 제거
- **설계**: 프레임 agents의 `home_id` 집합으로 가시 판정 —
  `stage==COMPLETED && !home_ids.contains(id)` → 렌더 스킵/디스폰.
  공사 중(stage≠COMPLETED)은 항상 표시(웹과 동일). skip_cache(영구)를
  쓰지 않고 프레임마다 판정 — 입주·사망으로 가시성이 역전되면 그 프레임에
  반영되야 한다. 레거시 무스테이지 완공 행(stage None)도 완공 취급.
- **주의**: 웹 소유자 기반 필터(owner 필드)는 유령 93채 사고로 폐기됨 —
  **집주소(homeId) 기반**이 정답(terrafolk-web resources.ts:166-174 주석).

### P1-3. ACT_KO 행동 라벨(tf-chars.js)
- **설계**: terrafolk-web popupParts ACT_KO 26종 표 이식 +
  `수면/이동 중/행동 중/대기` 조합 라벨(진행률 % 동봉). 주민 목록 행·
  히어로 카드·상세 패널 3곳에 동일 헬퍼 적용.

## 4. P2~P3 — 후속 파동

- **P2 모닥불 이벤트 ✅ 구현(2026-11-04)** — worldsim.js 'event' 중
  `payload.kind==='campfire'`를 `pendingCampfires` 큐로 → 다음 sink 프레임에
  `campfires` 편승 → Rust `LiveFrame.campfires` → `NetState.pending_campfires`
  1회 소비 → sync 행 병합(스냅숏 스폰 경로 재사용, refuel 무해 통과·
  remove는 서버 행이 남아도 차단). 라이브 발화(모닥불 점화 이벤트)는
  자연 발생 대기 — 체인 컴파일·프레임 파싱 무회귀는 게이트로 검증.
- **P2 어류**: 물 타일 스킵 예외 + fish 종 매핑 + 수면 근처 유영(웹: 반투명
  0.62·y−0.42·꼬리 스위시). 절차 소형 물고기 메시 신규 필요.
- **P2 동물 Gallop**: RoamTo 이동 중 Gallop 클립 전환(animal_anim 2차 과제).
- **P3 기록 카테고리 탭 5종**(사회/생활/지식/생명/세계 — tf-reslog 확장).
- **P3 plant 밀도**: 3D는 나무 15% 자체 튜닝 유지, plant(덤불)만 과밀 시
  절제 검토.
- **P3 날씨 강제 게이트**: `TF_FORCE_WEATHER`가 env var이라 wasm 무효 —
  `?tfdbg=weather:rain` 게이트로 웹 검증 경로 확보.

## 5. 검증 게이트 ✅

- 빌드: `cargo check`(네이티브+wasm 양측 통과).
- 회귀: `tools/e2e/root3d_live_villagers.cjs http://221.143.21.29:5101` —
  프라임·주민 앵커 6·문명 점수 정착·panic 0 (2026-11-04 실측 통과).
- 진단: `?tfdbg=hutprobe` — hut 행·완공(거주)·유령 스킵·공사·진행 샘플을
  2.5s 주기 info 로그. §17.92·진행률 회귀 시 1차 확인 지점.
