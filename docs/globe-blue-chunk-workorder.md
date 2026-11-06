# 작업 지시서 — 글로브 줌아웃 '파란 조각' + 맵↔객체 관계 감사 (2026-11-06)

대상 구현자: Sonnet(medium) 서브에이전트. **이 문서의 순서·규칙을 그대로 따를 것.**
추측으로 수정하지 말고, 각 단계의 판정 결과(숫자·스크린샷 경로)를 최종 보고에 남긴다.

---

## 0. 배경 — 설계자 진단(2026-11-06)

사용자 실기(세로 폰, 1080×1764) 스크린샷: 줌아웃(글로브 전환 구간)에서 섬 한가운데
**길쭉한 직선 다각형의 파란 판**이 지형 위에 그려진다. 원본:
`/Users/openclaw/.code-ssh-attach/20261106-155335.jpg`

설계자 실측(PIL):

| 영역 | 평균 RGB | σ |
|---|---|---|
| 파란 조각 내부 (580,1180)-(640,1400) | (117,165,213) | **0.0 / 0.0 / 0.0** |
| 바다(우하단) | (153,215,226) | ~2 |
| 섬 안 연못(정상 수면) | (145,190,190) | 17~58 |

- 조각 내부는 **가장자리까지 완전 단색**이다(가로 스캔 y=1100/1300/1500에서 경계 직전까지
  (117,165,213) 고정). 수면 셰이더(water.wgsl)는 노이즈·프레넬·해안 얕은수 혼합이
  있어 σ>0이고 해안 근처가 밝아지므로 **정상 수면 출력이 아닐 가능성이 높다**.
- 경계는 **긴 직선 + 얇은 슬리버(삼각형 가장자리)** — 지형(하프타일 0.5 격자) 교차가
  아니라 **큰 삼각형 단위로 통째 보이는 지오메트리**.
- 툰 셰이더(toon.wgsl)는 밴드 양자화로 단색을 낸다(그림자색 = albedo×(0.66,0.70,0.86)).
  청색 재질 후보: `models/CottageHouses.glb`의 `RoofTiles`(sRGB≈150,187,212)·`Windows`.
  CottageHouses는 노드에 ×100 + Z-up 행렬이 있고 v0.8.67에서 막 수정됨(누움 사고).
- 반대 가설: 수면 평면(3×size, 128분할 → 한 칸 3타일, 지형보다 24배 거침)이 글로브 워프
  수치 오차(`R·(cosθ−1)` 상쇄 오차, 모바일 GPU sin/cos 정밀도)로 육지 위로 솟는 경우.
  기하학적으로(정확 연산) 수면 현(chord)은 지형보다 항상 아래라 정상 산술로는 뚫지 못한다.

**결론: 원인 후보가 둘 이상 — 반드시 1단계 이분 탐색으로 확정한 뒤 해당 분기만 수정한다.**

추가 관찰(2단계 감사 항목으로 편입):
- aiserver DB의 공유 월드 맵(`worldsim_worlds.map_data`, 128×128)은 **가장자리까지 육지인
  대륙형**(좌측에 가장자리에 닿는 큰 호수)인데, 사용자 스크린샷 미니맵은 **바다로 둘러싸인
  둥근 섬**이다. 클라이언트가 보는 맵 ≠ DB 맵일 수 있다(다른 월드 id·캐시·미니맵 별도 원천).

---

## 1. 공통 규칙 (위반 금지)

1. 브랜치: `git checkout -b fix/globe-blue-chunk` (main에서). 커밋 전 **매번 `git status`로
   스테이지 확인**(병렬 세션 공유 레포). 내가 만든 파일 외 변경은 스테이지 금지.
2. 버전: `Cargo.toml [package] version` 0.8.67 → **0.8.68**(배포 1회당 +1 패치). `Cargo.lock`
   동반. 배포·커밋·PR·최종 보고에 `vX.Y.Z`를 명시.
3. 검증은 **배포 후 라이브**: `./tools/deploy-3d-r2.sh` → `https://terrafolk-3d.pages.dev/?...&cb=<epoch>`.
   - 5101 `trunk serve` 상시 구동 금지. 배포 전 `pgrep -fl "trunk serve"`가 있으면 **중지하지 말고
     보고**(다른 세션 소유일 수 있음) — 없을 때만 배포.
   - 배포 직후 수 분은 엣지 전파 지연 — 결과가 이상하면 캐시버스터 바꿔 재실행.
   - E2E는 **headed 필수**(시스템 Chrome 채널, `--disable-blink-features=AutomationControlled`,
     실 UA). 프라임 신호 = `!document.body.classList.contains('tf-loading')`, 대기 **180s**.
   - 공용 유틸 `tools/e2e/lib.cjs`(PNG 디코더·휘도 통계·헤디드 런처·프라임 대기) 재사용.
     실행: `NODE_PATH=/opt/homebrew/lib/node_modules node tools/e2e/<script>.cjs [url]`
4. 코드 규칙: 파일 500줄 이하(새 기능은 새 파일), rustfmt는 **리프 파일만 지정**(mod.rs 금지),
   `cargo check` + `cargo check --target wasm32-unknown-unknown` 통과, `cargo test` 통과.
5. 글로브 워프 수식은 **3곳 계약**(`src/world/globe.rs`·`assets/shaders/terrain_globe_vertex.wgsl`·
   `assets/shaders/water.wgsl`) — 하나를 바꾸면 셋 다 같은 수식으로.
6. WebGL2 유니폼 예산: 수면 재질에 **유니폼·텍스처 바인딩 추가 금지**(13개 초과 사고 전례).
   `alpha_mode` Opaque 유지. 지형 확장도 바인딩 추가 금지.
7. chibi-character 크레이트·주민 몸체 코드는 건드리지 않는다.
8. 줌인(평면, factor<0.003) 화면은 **픽셀 동일**이 원칙 — 평면 경로 변경 시 전/후 스크린샷 비교.
9. 막히면(분기 판정 불가·배포 실패 2회) 추측 수정하지 말고 그 시점까지의 증거로 보고 후 종료.

---

## 1단계 — 재현·이분 탐색 (코드 수정 없음, 현재 라이브 v0.8.67 대상)

### 1-1. 재현 스크립트 `tools/e2e/globe_artifact.cjs` 신규

- 뷰포트 **세로 폰**: `412×732`, `deviceScaleFactor: 2.625`, `isMobile: false`(휠 사용 위해).
  보조로 데스크톱 `1280×800` 1회.
- URL: `https://terrafolk-3d.pages.dev/?tfdbg=globe<,추가플래그>&cb=<epoch>` (인자로 받음).
  `tfdbg=globe`는 부팅 시 글로브 줌으로 시작(camera_rig.rs:125).
- 프라임 대기 후 5초 정착 → 스크린샷 ①(완전 글로브). 이어서 `mouse.wheel(0, 120)`을 1회씩
  주며 4단계 추가 스크린샷(②~⑤ — 전환 중간 곡률 구간 포함; root3d_globe.cjs 53행 방식).
  각 단계 1.5s 정착. 화면 중앙 드래그(마우스 다운→+150px 이동→업)로 팬 1회 후 ⑥.
- **검출기**: 각 스크린샷에서 "청색 단색 블롭" 탐지
  - 픽셀 조건: `B > R + 60 && B > G + 25`
  - 4-연결 성분 중 **면적 ≥ 1500px** 이고 성분 내 RGB 표준편차 합 **< 3**(단색)
  - 출력: `{shot, blobs:[{area, bbox, meanRGB, std}]}`
  - 바다/정상 연못은 σ가 커서 걸러져야 한다 — 기준 스크린샷에서 오탐이 있으면 임계를
    조정하고 조정값을 보고에 기록.
- 결과 JSON을 `/tmp/globe_artifact_<tag>.json`, 스크린샷 `/tmp/globe_artifact_<tag>_N.png`.

### 1-2. 이분 행렬 (각각 실행)

| tag | tfdbg 추가 플래그 |
|---|---|
| base | (없음) |
| nowater | `no_water` |
| nohuts | `no_huts` |
| noscaf | `no_scaffold` |
| noanim | `no_animals` |
| novill | `no_villagers` |

판정: base에서 블롭 ≥1 이 재현돼야 한다. **블롭이 사라지는 플래그가 원인 분기**.
- base에서 재현 실패 시: 사용자 폰 해상도(1080×1764 = 412×673 @2.625)로 재시도, 그래도
  실패면 APK가 아니라 웹에서도 재현 불가 — 2분기(W) 예방 수정 + 6단계 회귀 테스트만
  진행하고 "실기 재현 불가"를 명시해 보고.
- 블롭 스크린샷은 Read로 직접 열어 시각 확인(이미지 판독).

---

## 2단계 — 원인 분기별 수정 (1단계 판정된 분기만)

### 분기 H — `no_huts`(또는 `no_scaffold`)에서 사라짐: 건물 메시 이상

1. `?tfdbg=globe,hutprobe` 로 콘솔 로그(`[net] hutprobe ...`)와 `FIT 클램프` 로그 수집
   (`tools/e2e/console_all.cjs` 패턴 재사용).
2. 조사 순서
   - `src/world/prop_fit.rs fit_props`: Hut(CottageHouses, target_h 4.2, 변형 House_1/2/4)의
     측정 size·스케일 s·클램프 발생 여부. 변형별 실제 노드 행렬(×100, Z-up)이
     `chain_matrix`에 반영되는지.
   - `tools/check_glb_upaxis.py`(v0.8.67 게이트)로 현재 GLB 재검사.
   - 글로브 워프 W1 계약(globe.rs 머리말): 건물 서브트리 안에 **ChildOf 없는 별도 루트**
     (지붕·창 등)가 있으면 그것만 따로 워프돼 접선 방향으로 튄다. `apply_warp` 쿼리 대상
     루트 목록을 hutprobe에 덤프(이름·translation·scale)해 확인.
   - 변형 파기(`collect_meshes` 비매칭 despawn) 후 남는 메시가 다른 변형 노드 행렬을
     잃어 원점·×100 그대로 남는 경우.
3. 원인 확정 후 최소 수정. 재발 게이트: `tools/check_glb_upaxis.py`에 **변형별 노드 행렬
   적용 후 높이 범위 [1.5m, 8m] 검사** 추가(현재 check-assets.sh가 호출).

### 분기 W — `no_water`에서 사라짐: 수면 평면이 육지를 뚫음

두 수정을 **모두** 적용(서로 독립, 어느 쪽이 결정적이든 방어 겸용):

W-1. **수치 안정 워프** — 3곳의 `R*(cosθ − 1)`을 `−2·R·sin²(θ/2)`로 교체.
   - globe.rs `warp_point`: `let h = (0.5*theta).sin(); base.y = -2.0 * self.r_eff * h * h;`
     (θ 클램프 동일 적용). `unwarp_point`은 기존대로(역변환 정확성 단위테스트로 확인).
   - terrain_globe_vertex.wgsl·water.wgsl `globe_warp_pos`: 동일 수식.
   - 수식 주석에 "상쇄 오차 회피(2026-11-06)" 남기고 세 곳 머리말 계약 문구 유지.

W-2. **육지 아래 수면 정점 가라앉히기(CPU, water.rs)** — Plane3d 빌드 후 positions를 후처리:
   - 정점 평면좌표(엔티티 translation 포함 월드 x,z)에서 **반경 4타일**(한 칸 3타일 + 여유 1) 안의
     모든 타일이 맵 안 + `terrain_at != "water"`이면 로컬 y = **−1.0**(세계 −1.18).
   - 이 조건이면 그 정점에 닿는 모든 수면 삼각형은 완전히 내륙 육지(지표 ≥ 0) 아래 →
     **평면 줌에서 픽셀 동일**, 글로브에서 오차 여유 1m 확보.
   - 노멀은 사용 안 하지만(셰이더 무광) 그대로 둔다. 정점 수 불변(유니폼·바인딩 불변).
   - 함수 분리: `fn sink_under_land(mesh: &mut Mesh, world: &WorldData, origin: Vec3)` +
     단위테스트(내륙 정점 −1.0, 해안 4타일 이내 정점 0.0).

W-3. 회귀 단위테스트 `src/world/globe.rs` 테스트 모듈(또는 신규 `globe_tests.rs`):
   - 데모 `WorldData::generate(20261011)` 기준, factor ∈ {0.05, 0.3, 0.6, 1.0}.
   - 육지 타일 중심 500개 샘플: 해당 평면점이 속한 수면 삼각형(128분할 격자 재현 + W-2 적용
     높이)의 세 정점을 `warp_point`한 뒤 무게중심 보간한 점의 **구 중심 반경** vs 지표점
     (`surface_height`)을 warp한 반경 → `water_r < terrain_r − 0.05` 단언.
   - `warp_point`/`unwarp_point` 왕복 오차 < 1e-3(θ 0.01~3.0).

### 분기 O — 어느 플래그로도 안 사라짐

`no_terrain`·`no_sky`·`no_wind` 추가 실행 + 블롭 색과 동일한 재질을 `src/` 전역·GLB에서 탐색
(설계자 목록: `CottageHouses RoofTiles/Windows`만 청색). 원인 확정 실패 시 **수정하지 말고**
증거(행렬 결과·스크린샷 경로)로 보고 후 종료.

---

## 3단계 — 맵↔객체 관계 감사 프로브 `?tfdbg=mapaudit` (신규)

신규 파일 `src/world/map_audit.rs`(≤300줄), `world/mod.rs`에 시스템 등록(Update, 진단 게이트
`crate::net::dbg_gate::on("mapaudit")`일 때만 동작 — 운영 무해). 프라임 후 첫 1회 + 600프레임마다
wasm 콘솔(`web_sys::console::log_1`)·네이티브 `info!`로 한 줄 JSON 출력 `[mapaudit] {...}`.
**평면 좌표로 판정**(PreUpdate restore 이후 Update 단계라 Transform은 평면).

| id | 항목 | 판정 |
|---|---|---|
| A0 | 맵 정체성 | `size`, 지형 GID 해시(FNV-1a 32bit, terrain Vec<u8> 전체), water 비율 |
| A1 | 물 위 객체 | 루트 엔티티(PropRoot·건물·자원·동물 — 쿼리 가능한 마커 기준) 중 타일이 water인 것 수 + 샘플 5개(종류·x,z). 수중 동물·물고기(fish.rs)·WaterDeer 제외 |
| A2 | 부유/매몰 | 루트의 평면 y − `surface_height(x,z).max(WATER_STAND_Y)` 가 +0.6 초과(부유) 또는 −0.6 미만(매몰) 수·샘플. 새(sky.rs)·달/별·`GlobeExempt`·주민(점프 중일 수 있음 — 별도 카운트만) 제외 |
| A3 | 맵 밖 | x,z ∉ [0,size] 수·샘플 |
| A4 | 핏 이상 | FitTransform.scale 이 0.02 또는 6.0(클램프 경계)인 프롭 수·종류 |
| A5 | 건물 겹침 | 완공 건물 루트 반경 1.6m 안에 있는 장식/자원 프롭 수(집 관통) |

DB 대조: `ssh aiserver` 후
`cd ~/terrafolk/backend && PYTHONPATH=src .venv/bin/python -c "from terrafolk_api import db; ..."`로
`worldsim_worlds` 각 행(id, map_data) 의 동일 FNV-1a 해시 계산(Tiled `layers[0].data`, GID를
`clamp(1,6)` 후 바이트 — 클라이언트 `WorldData::from_map`과 동일 전처리, n=min(w,h,256)) →
A0 해시와 비교. **불일치면 어느 월드 id를 클라이언트가 받는지**(룸 접속 파라미터·
`src/net` 접속 코드) 추적해 보고(수정은 서버 소관이면 하지 않는다 — ../terrafolk 세션 소관).

---

## 4단계 — 감사 결과 기반 수정 (최대 3건)

v0.8.68 배포 → `globe_artifact.cjs`(base) + `?tfdbg=mapaudit` 콘솔 수집(새 스크립트
`tools/e2e/map_audit.cjs` — 프라임 후 20s 콘솔 수집, `[mapaudit]` 라인 파싱해 JSON 저장).
- 카운트 > 0 이고 **원인이 코드에서 확인되는 항목만** 수정, 최대 3건. 우선순위 A1 > A2 > A5 > A4 > A3.
- 각 수정마다 전/후 카운트를 표로 기록. 원인 불명·서버 데이터 원인(예: 서버가 물 타일에
  자원 배치)은 수정하지 말고 보고 목록에.
- 수정이 있으면 버전 0.8.69로 재배포 후 재측정.

---

## 5단계 — 최종 검증 (마지막 배포 버전 기준)

1. `globe_artifact.cjs` 6개 행렬 중 base: **청색 단색 블롭 0** (세로·데스크톱 둘 다).
2. `tools/e2e/root3d_globe.cjs https://terrafolk-3d.pages.dev/?tfdbg=globe&cb=…` → allPass.
3. `tools/e2e/root3d_smoke.cjs <pages.dev url>` → 통과(줌인 평면 회귀 없음).
4. 평면 줌 픽셀 동일성(분기 W 적용 시): 수정 전 v0.8.67 기본 줌 스크린샷(1단계에서 `?cb=`로
   미리 1장 저장해 둘 것)과 수정 후 동일 카메라 — 카메라가 라이브 주민을 따라가 비결정이면
   수면 영역만 PIL로 평균색 비교(차이 < 2)로 대체하고 그 사실 기록.
5. `cargo test` 전체 통과 수 기록.

## 6단계 — 커밋·PR

- 커밋 메시지 예: `fix: 글로브 줌아웃 파란 조각 — <확정 원인 한 줄> + mapaudit 프로브 (v0.8.68)`
  마지막 줄: `Co-Authored-By: Claude Sonnet 5.5 <noreply@anthropic.com>`
- `git push -u origin fix/globe-blue-chunk` → `gh pr create`(본문: 원인·증거표·감사 결과·
  버전·검증 결과, 끝줄 `🤖 Generated with [Claude Code](https://claude.com/claude-code)`).
  **머지는 하지 않는다**(설계자 검토 후).

## 최종 보고 형식 (서브에이전트 → 설계자)

```
버전: v0.8.xx (배포 URL)
1단계 판정표: tag | 블롭수 | 최대면적 | 색
확정 원인: ... (근거 파일:줄)
수정 목록: 파일 — 요지
감사 A0~A5 전/후 표 + DB 해시 대조 결과
검증: globe_artifact / root3d_globe / root3d_smoke / cargo test 결과
미해결·보류 항목
PR 링크
```

---

## 추가 지시 A1 (2026-11-06, 1단계 결과 반영) — 분기 V: 주민 다리 늘어남

1단계 판정: `no_villagers`에서만 전 구간 0 → **분기 V 확정**. 설계자 코드 확인:
- 크레이트 `../3d-chibi-character/crates/chibi-character/src/lib.rs:132~` 가
  `(pose::gait_blend, foot_lock::foot_lock::<F>, leg_ik::leg_ik).chain()`을
  `PostUpdate .after(AnimationSystems).before(TransformSystems::Propagate)`에 등록.
- 게임 `src/world/mod.rs:102~` 는 `globe::apply_warp.before(Propagate)`만 지정 — **두 쪽 사이 순서가
  없다(모호성)**. apply_warp가 먼저 돌면 루트 Transform이 이미 구면 좌표인데 foot_lock/leg_ik는
  `fresh_affine`(루트 로컬 Transform 합성)·`surface_height(x,z)`(평면장)로 발 목표를 계산 →
  발/정강이 본이 엉뚱한 곳으로 당겨져 스킨 메시(데님 바지 #6B84C4 틴트 → 툰 단색)가 거대한
  판으로 늘어난다.

수정(게임 측만 — 크레이트 수정 금지, 규칙 7 위반 아님):
1. `src/world/mod.rs`의 apply_warp 등록에 `.after(chibi_character::leg_ik::leg_ik)` 추가
   (chain이라 gait_blend·foot_lock도 앞섬). `tool::tool_anchor_fit`(PostUpdate, 전파 전)도
   `.after(chibi_character::tool::tool_anchor_fit)`로 함께 정렬. 주석으로 이유(워프는 평면 포즈
   계산이 모두 끝난 뒤 마지막) 기록.
2. 게임·크레이트의 **나머지 PostUpdate(전파 이전) 시스템** 중 루트/본 Transform을 평면으로
   가정하는 것을 grep(`add_systems(\s*PostUpdate` 전수)으로 찾아 표로 보고하고, 해당되면 동일하게
   apply_warp를 그 뒤로 정렬.
3. 회귀 방지: `src/world/globe.rs` 테스트 또는 앱 빌드 테스트로 스케줄 순서를 검증하기 어렵다면,
   `?tfdbg=globe` 라이브에서 1단계 카운터 base 6장 모두 0을 게이트로 삼는다(globe_artifact.cjs에
   정확색 카운트 판정 내장 — 밤/낮 무관 (117,165,212)±1 픽셀 < 200 이면 PASS, 아니면 FAIL).
   주민 데님 틴트 색은 조명에 따라 다르므로, 추가로 "화면 높이 30% 초과 세로 길이의 단색 블롭"
   형태 판정도 넣는다.
4. 이후 3~6단계(mapaudit·배포 v0.8.68·검증·PR)를 지시서대로 진행. 1단계 측정은 재사용.

---

## 추가 지시 A2 (2026-11-06, v0.8.68 FAIL 반영) — 추측 중단, 개체 직접 특정

A1(워프↔발 체인 순서)은 **유지**(정당한 순서 계약, 무해). 단 원인이 아니었다 — v0.8.68에서도
(117,165,212) 단색 다각형이 **수면 위·물 마시는 주민 근처**에 남음(`/tmp/globe_artifact_fixphone_4.png`).
조명 불변 단색 → 언릿 계열. 설계자 배제 목록(색 불일치): 블롭 그림자(올리브 블렌드)·선택 링(앰버)·
night_glow(웜)·잉크 헐(base_mix=1 갈색 단색)·occlusion(Visibility 은닉).

### A2-1. 거대 메시 프로브 `?tfdbg=bigmesh` (신규 `src/world/big_mesh_probe.rs`, ≤250줄)

- 등록: `PostUpdate`, `.after(bevy::transform::TransformSystems::Propagate)`, 게이트
  `crate::net::dbg_gate::on("bigmesh")`, 프라임 후 120프레임마다 1회, 최대 30회.
- 대상: `Mesh3d`를 가진 **모든** 엔티티. 제외: `TerrainMesh`·`WaterMesh` 마커, 하늘(`GlobeExempt` 조상).
- 각 엔티티의 월드 범위: 메시 정점 AABB(메시 에셋당 캐시 — prop_fit의 `mesh_aabb`/`MeshAabbCache`
  재사용)의 8코너를 `GlobalTransform`으로 변환한 최대 변 길이 = `extent`.
  `SkinnedMesh`가 있으면 추가로 joints의 `GlobalTransform.translation()` 집합의 최대 쌍거리 =
  `joint_spread`와, 가장 가까운 조상 루트(ChildOf 없는 엔티티)와 joints 최대 거리 = `joint_reach`.
- 판정: `extent > 8.0` 또는 `joint_spread > 3.0` 또는 `joint_reach > 4.0` 인 것만 로그.
  extent 내림차순 상위 10개를 한 줄 JSON `[bigmesh] {...}`:
  `{name, ancestors:[이름 최대 8개, 루트까지], root_has_villager:bool, extent, joint_spread,
    joint_reach, skinned, mat:{kind: "Standard"|"Toon"|"Ink"|"Water"|"Other", base_srgb, unlit,
    alpha_mode}, visible:bool}`
  재질 종류는 해당 `MeshMaterial3d<T>` 컴포넌트 존재로 판별(게임 toon.rs 재질·크레이트
  `ink` 재질 타입·StandardMaterial). 이름은 `Name` 없으면 `"?"`.
- 수집 스크립트 `tools/e2e/bigmesh.cjs`: `?tfdbg=globe,bigmesh` 세로 폰, 프라임 후 휠 줌아웃 2단계
  (globe_artifact ③ 상태)에서 40초 콘솔 수집 → `/tmp/bigmesh.json`. `%c` 서식 잘림 문제는
  `msg.args()`를 `jsonValue()`로 읽거나 `[bigmesh] ` 이후 전체 문자열을 정규식으로 잡아 해결.
  **비교 실행**: `?tfdbg=bigmesh`(평면 기본 줌) 1회 — 평면에서도 거대한지(글로브 고유인지) 판별.

### A2-2. 판정 → 수정

- 상위 항목 중 base_srgb 또는 화면 색이 청색 계열이고 villager 루트 소속인 것 = 범인.
- 범인의 소유가 **게임 코드**면 원인(스케일·부모 누락·워프 이중 적용·스킨 조인트 부모 불일치 등)을
  파일:줄로 확정 후 최소 수정.
- 범인의 소유가 **크레이트(../3d-chibi-character)**면 **수정하지 말고** 범인 JSON·원인 추정 파일:줄로
  보고 후 종료(설계자 결정).
- 수정 후 v0.8.69 배포 → globe_artifact(세로·데스크톱) (117,165,212)±1 < 200 이면 PASS.
  색이 조명에 따라 달라질 수 있으니 bigmesh 재수집에서 해당 항목 0건도 함께 PASS 조건.

### A2-3. 감사 항목 원인 조사(수정은 코드 원인 확정 시에만, 최대 2건)

- **A4 다람쥐 핏 클램프**(16건 대부분 AnimalSquirrel): `models.rs` 다람쥐 ModelSpec의 GLB·target_h와
  실제 AABB(노드 행렬 포함)를 python으로 측정 — 필요 스케일이 6.0 초과(작은 단위 GLB)면
  클램프 때문에 다람쥐가 의도보다 작게 렌더된다. 해결: 해당 종만 ModelSpec에 스케일 상한 예외를
  두지 말고, `prop_fit` 클램프 상한을 종별로 넓히는 대신 **GLB 단위 정규화가 원천**이면
  check_glb_upaxis.py류 게이트에 "변형별 필요 스케일 0.05~6" 검사를 추가하고 보고(에셋 수정은 하지 않음).
- **A2 멧돼지 부유 +0.9m**(12건): AnimalBoar 루트 y 산출 경로(`src/net/apply.rs` 동물 스폰·
  `animal_anim.rs`)에서 지면 대비 오프셋 원인 확인. 점프/이동 애님의 루트 모션이면 정상(보고만),
  ground_y 계산 버그면 수정.
- A5(건물 반경 장식 6건)는 보고만.

최종 보고는 기존 형식 + `bigmesh 상위 10 표`.

---

## 추가 지시 A3 (2026-11-06, v0.8.70 결과 반영) — 재질 색칠 판별 + 동물 클램프 수정

설계자 추가 배제: JS 캔버스 오버레이 없음(assets/web 2d 컨텍스트는 tf-look 편집기뿐)·카메라 1대뿐·
AgentHud는 데이터 컴포넌트. 크롭(`/tmp/crop.png`)상 블롭은 **톱니 외곽선(한쪽 어두운 점선·한쪽 밝은 점선)을
가진 큰 볼록/오목 다각형**이 수면 위에 그려진 것 — 3D 지오메트리로 본다.

### A3-1. `?tfdbg=matcolor` (신규 `src/world/mat_color_probe.rs`, ≤200줄) — 결정적 판별

간헐성과 무관하게 **블롭이 보이는 스크린샷 1장으로 재질 계열을 특정**한다.
- `Update` 매 프레임(게이트 on일 때만): 아래 계열의 메시 엔티티 재질 핸들을 **계열별 고정 언릿 색**
  재질(1회 생성 캐시)로 교체. 원래 핸들은 교체 전 `OrigMat` 컴포넌트에 보존(진단 전용이라 복원 불필요).
  - StandardMaterial `unlit=true` → 마젠타 (1,0,1)
  - StandardMaterial lit(그 외) → 노랑 (1,1,0)
  - 게임 툰 재질(`world/toon.rs`의 Material 타입) → 초록 (0,1,0)
  - 크레이트 잉크 재질 → 빨강 (1,0,0)
  - TerrainMaterial·WaterMaterial → **교체하지 않음**(기준 배경)
  - 그 밖의 `MeshMaterial3d<T>` 타입이 있으면 grep으로 전부 열거해 각각 고유색(시안·주황·흰색…) 배정,
    표를 보고서에 기록.
  - 툰 재질처럼 교체가 타입을 바꿔야 하는 경우 `remove::<MeshMaterial3d<Toon>>()` +
    `insert(MeshMaterial3d<StandardMaterial>)`.
- `globe_artifact.cjs`에 tag 인자 `matcolor` 추가 실행(`?tfdbg=globe,matcolor`). 블롭 위치(기존 실행에서
  블롭이 나온 단계 ③~⑥)의 색을 PIL/JS로 읽어 계열 판정. **블롭이 원래 청색 그대로 남으면** 위 어느
  계열도 아님 → "메시가 아닌 것(수면 셰이더 자체·구멍)"으로 판정하고 A3-3으로.
- 계열이 정해지면 `?tfdbg=matcolor,matname` 2차: 그 계열 안에서 엔티티 `Name`(없으면 가장 가까운 Name 조상)
  해시로 색을 12색 팔레트에서 배정하고, 색→이름 표를 콘솔 `[matname] {...}`로 1회 출력 → 블롭 색으로 이름 특정.

### A3-2. 범인 확정 후

- 게임 코드 소유면 최소 수정 → 배포 → globe_artifact(세로·데스크톱) 정확색 < 200 + matcolor 재실행에서
  해당 색 블롭 0.
- 크레이트 소유면 수정 금지, 이름·계열·추정 원인(파일:줄)으로 보고 후 종료.

### A3-3. 블롭이 matcolor에서도 청색 그대로일 때 (메시 아님)

`no_water,matcolor`로 재실행(수면 제거 시 블롭 색이 무엇이 되는지) + `?tfdbg=globe,no_water` 스크린샷에서
같은 위치 색을 보고. 수정하지 말고 보고.

### A3-4. 동물 핏 클램프 하한 수정 (설계자 승인 — 이 지시로 확정)

`src/world/prop_fit.rs` `s.clamp(0.02, 6.0)` → **`s.clamp(0.002, 6.0)`**. 근거: AnimalBoar(필요 0.009)·
AnimalSquirrel(필요 0.010)이 하한에 걸려 목표 대비 약 2배 크게 렌더되고, 멧돼지 루트가 +0.9m 떠 보이는
A2 감사값도 같은 원인. 수정 후 mapaudit 재수집에서 A4의 Boar·Squirrel 0, A2 부유 0(또는 동물 점프 외 0)
확인. 주석에 근거 숫자 기록. 같은 하한에 걸리는 다른 종이 있는지 A4 표로 재확인.

### 마무리

A3 수정(+A3-4)을 한 번의 배포(v0.8.71)로 묶을 것. 진단 프로브(map_audit·big_mesh_probe·mat_color_probe)는
게이트 뒤라 운영 무해 — 유지. 검증 통과 시 6단계(push·PR, 머지 금지). 블롭 미해결이어도 A3-4와 진단
도구는 PR로 올리되 PR 제목/본문에 "파란 조각 미해결 — 진단 결과" 명시.

---

## 추가 지시 A4 (2026-11-06, 사용자 승인: 1번 진행 + **크레이트 수정 허용**)

### 설계자 가설(유력) — "Update 구간 GlobalTransform 오염"

- 글로브 활성 중 Update 단계에서 `Transform`은 평면(restore_flat)이지만 **`GlobalTransform`은 지난
  프레임 워프값(구면)** 그대로다. GT와 Transform을 섞는 Update 시스템은 글로브에서 틀린 값을 낸다.
- 크레이트 해당 지점: `proportion.rs chibi_settle → capture_foot_rig(gts)`(스폰 직후 **1회 캡처 → 영구 보존**),
  `ground.rs ground_snap`(`gt.translation().y - tr.translation.y`), `action.rs face_targets`, `lod.rs assign_lod` 등.
- 시나리오: 글로브 줌 중 주민이 **스폰/재스폰**(재연결·룩 변경·신규 접속)되면 settle이 구면 GT로
  FootRig 피벗을 잡음 → foot_lock/leg_ik가 매 프레임 발을 엉뚱한 로컬로 사상 → 바지(LightBlue std-lit)·
  잉크 헐이 거대 판으로 늘어남. **사용자 스샷에 "다시 연결되었습니다" 토스트** = 재연결 전원 재스폰과 정합.
  간헐성(스폰 시점이 글로브였는지)도 설명.

### A4-1. 결정적 재현으로 가설 검증 (수정 전, 현재 라이브 v0.8.73)

`tools/e2e/globe_artifact.cjs`에 모드 2개 추가, 각각 3회 반복:
- **S(글로브 중 스폰)**: `?tfdbg=globe` 부팅(주민이 글로브 상태에서 settle) → 프라임 후 20s 대기 → 휠로 한 단계
  줌인했다가 다시 줌아웃 → 스샷 3장.
- **F(평면 스폰 후 줌아웃)**: `?tfdbg=` 없이 부팅 → 프라임 후 20s(평면에서 settle 완료) → 휠 줌아웃으로 글로브
  진입 → 스샷 3장.
- 추가: S 모드 후 **줌인(평면)** 상태 스샷 1장 — 가설이 맞으면 평면에서도 늘어난 다리가 보일 수 있음(FootRig 영구 오염).
가설 지지 = S에서 정확색 블롭 다수, F에서 0. 결과 표를 보고에 기록. 지지되지 않으면 A4-3로.

### A4-2. 근본 수정 (게임 측, 계열 전체 해결)

`src/world/globe.rs`·`mod.rs`: **restore_flat 직후 같은 PreUpdate에서 GlobalTransform을 평면으로 재전파**.
- bevy 0.19.1의 `bevy::transform::systems`에서 공개 시스템(`mark_dirty_trees`·`propagate_parent_transforms`·
  `sync_simple_transforms` 등 — **실제 이름은 `~/.cargo/registry` 해당 버전 소스를 grep해 확인**, 추측 금지)을
  `PreUpdate`에 `.after(globe::restore_flat)` 체인으로 추가하고, `run_if`로 **직전 프레임 워프가 있었을 때만**
  실행(LastWarp가 Some → restore가 실제로 되돌린 프레임). 평면 줌에선 비용 0.
- 이렇게 하면 Update 전 구간이 평면 GT를 보므로 크레이트 수정 없이 settle·ground_snap·face_targets·lod 모두 정상.
- 주석: "글로브 계약 W2 — Update 구간 GT는 평면(PreUpdate 재전파). 렌더 전용 워프는 PostUpdate apply_warp".
- `src/world/globe.rs` 머리말 계약 문구에 W2 한 줄 추가. CLAUDE.md는 수정하지 않는다(설계자가 함).

### A4-3. 크레이트 방어 (허용됨 — 필요 시에만)

A4-2만으로 S 모드 블롭이 0이 되면 크레이트는 건드리지 않는다. 남으면:
- `../3d-chibi-character` 별도 git 레포. 브랜치 `fix/settle-fresh-affine` 생성, `capture_foot_rig`·`ground_snap`의 GT
  사용을 `pose::fresh_affine`(로컬 체인 합성, 이미 foot_lock이 쓰는 방식)으로 교체. 크레이트 `cargo test` 통과,
  크레이트 버전 패치 bump, 커밋(Co-Authored-By 동일)·push·PR 생성(머지 금지). 게임은 path 의존이라 그대로 빌드됨.
- 크레이트 CLAUDE.md/README의 관련 계약 문서가 있으면 같은 PR에서 한 줄 갱신.

### A4-4. 배포·검증·PR

- v0.8.74 배포 1회(필요 시 크레이트 포함). 검증: S·F 모드 각 3회 정확색 < 200(세로 폰), 데스크톱 S 1회,
  `root3d_globe`·`root3d_smoke` 통과, `cargo test` 재실행 수치 기록.
- 기존 PR #158 브랜치에 커밋 추가·push(새 PR 만들지 말 것). 해결되면 PR 제목에서 "미해결" 제거·본문 갱신
  (원인 = Update 구간 GT 오염, 수정 = PreUpdate 재전파). 머지 금지.
- 미해결이면 S/F 표·matname 결과로 보고 후 종료.

---

## 추가 지시 A5 (2026-11-06) — **원인 확정: 머리 착용품(wear) ×100 거대화**

설계자 무배포 조사(라이브 v0.8.76):
- qchar 주민 GLB(f-worker·m-farmer·m-adventurer)의 `CharacterArmature` 노드 **scale ×100** → 그 아래
  `Head` 본의 월드 스케일 ≈ ×100.
- `src/world/wear.rs:283` 이 헤어·수염·안경·눈썹 GLB 씬을 `Head` 본 직속 자식으로 붙이며 스케일은
  `slot.scale()`(0.28~0.85)뿐 — **본 ×100을 상쇄하지 않음**. v0.8.66 손 도구 거대화(크레이트 ToolAnchor로
  도구만 해결)와 동일 계열이 착용품엔 미적용.
- bigmesh 실측: wear-hair/glasses/beard extent 35~115m(평면에서도 동일 — "아티팩트"가 아니라 실값).
- 결정적 재현: `?tfdbg=globe,wearbeard`(남성 전원 수염 강제) 1회 실행에서 정확색 27,834px, 밤(19:25)에도
  (117,165,212) 고정 — 거대 착용품이 대부분 지형 아래 묻혀 있고 **낮은 곳(호수·바다)에서만 수면 위로
  솟아** 직선 다각형으로 보임(해안선에서 잘림). 간헐성 = 착용 주민의 위치·카메라 방향.
- 스샷: `/private/tmp/claude-502/-Users-openclaw-workspace-terrafolk-rust/dce9a32b-14d5-456f-b81a-23a039614503/scratchpad/wb_2.png`

### A5-1. 수정 (게임 `src/world/wear.rs`)

- 크레이트의 `chibi_character::tool::ToolAnchor { actor }` 재사용(계약 T1: 앵커 월드 스케일 = 액터 `ToolScale`,
  `tool_anchor_fit`가 PostUpdate 전파 전 매 프레임 refit, 첫 보정 후 Hidden→Inherited). 주민엔 이미
  `ToolScale(villager_scale())`가 붙어 있음(villager_spawn.rs).
- 부착 구조 변경: `Head`(등 슬롯 본) → **앵커 엔티티**(`Name("wear-anchor-{slot}")`, `ToolAnchor{actor: 주민}`,
  `Transform::default()`, `Visibility::Hidden`) → 기존 wear 씬 엔티티(offset·scale 그대로 시작).
  슬롯당 앵커 1개(같은 슬롯 여러 아이템은 같은 앵커 공유). 손 슬롯(HandRight/Left)도 동일 적용.
- 진단 프로브의 ToolPartOf 의존 등 도구 전용 로직이 앵커를 오인하지 않는지 grep 확인(`ToolAnchor` 쿼리 사용처).
- 캘리브레이션: 수정 후 앵커 월드 스케일 = villager_scale(≈1.6)이 되므로 `WearSlot::offset()`·`scale()`
  의 기준 단위가 바뀐다. **근접 컷**으로 판정 후 필요 시 값 조정:
  `?tfdbg=wearbeard` + 주민 근접(카메라 팔로우 또는 휠 줌인) 스샷에서 헤어가 머리에, 수염이 턱에,
  안경이 눈높이에 오는지 이미지로 확인(Read로 직접 열람). 조정값과 근거를 커밋 메시지에 기록.

### A5-2. 회귀 게이트

- `tools/e2e/bigmesh.cjs` 결과로 **flat·globe 모두 `wear-*` 조상 항목 extent ≤ 3.0m**(0건) 판정 추가.
- `globe_artifact.cjs`: base 3회 + `globe,wearbeard` 3회 + S 모드 2회 — 정확색 (117,165,212)±1 < 200 전부 PASS.
- `root3d_globe`·`root3d_smoke` 통과, `cargo test` 수치.

### A5-3. 부수 버그 (같은 PR, 원인 명확 시만)

- 미니맵 패널 안에서 지도 이미지가 오른쪽으로 밀려 **왼쪽에 청색 띠**가 생기는 경우 관측
  (`/private/tmp/claude-502/-Users-openclaw-workspace-terrafolk-rust/dce9a32b-14d5-456f-b81a-23a039614503/scratchpad/sbm1_0.png`).
  `src/ui/minimap.rs`·`minimap_draw.rs`에서 원인 확인 시 수정, 불명확하면 보고만.

### A5-4. 배포·PR

- v0.8.77 배포 1회(필요하면 캘리브레이션 재배포 v0.8.78까지 허용).
- PR #158 브랜치에 커밋·**push**(사용자가 push 허용함). PR 제목에서 "미해결" 제거, 본문 원인 섹션을
  "머리 착용품 ×100 거대화(Head 본 아마추어 스케일 미상쇄) — ToolAnchor 재사용으로 상쇄"로 갱신.
  A1(W 순서)·W2(GT 재전파)·크레이트 0.2.11은 원인은 아니었지만 정당한 계약 정리로 유지한다고 명시. 머지 금지.
- 크레이트 수정은 이번엔 불필요(ToolAnchor 공개 API 재사용). 필요해지면 크레이트 PR #2 브랜치에 추가.

---

## 추가 지시 A6 (2026-11-06, 사용자 승인: 후속 1~3 진행)

PR #158(v0.8.77)에 이어서 같은 브랜치 `fix/globe-blue-chunk`에 커밋. 크레이트 PR #2는 설계자가 이미
push·생성함(https://github.com/variouscafe/3d-chibi-character/pull/2) — 건드리지 말 것.

### A6-1. 수염·안경 캘리브레이션 (결정적 근접 컷)

근접 컷 장치 — 라이브 JS 브리지(`src/world/look_edit.rs`·`pick.rs` register_bridge):
- `window.terrafolk.selectAgent(id)` → 주민 선택·카메라 팔로우
- `window.terrafolk.setLookFocus(true)` → 편집 줌(EDIT_ZOOM 0.34) 근접 고정 + 정오 조명
- `window.terrafolk.setLocalLook(id, lookJson)` → 그 주민 외형 즉시 오버라이드(재스폰). parts 키는
  `src/world/wear.rs`의 `wear_ids_from_look`이 읽는 키(beard·glasses·hair 등)를 grep으로 확인해 사용.
- 주민 id: 라이브 프레임에서 얻기 — 콘솔 로그·`/api/worldsim/...`(tf-api)·또는 `window.terrafolk`/worldsim.js 상태
  중 확실한 경로를 찾아 사용. 남성 1명 + 여성 1명.

스크립트 `tools/e2e/wear_closeup.cjs`(신규): 각 주민에 대해 look을 ① beard-full ② glasses(안경 슬롯 아이템
1종) ③ beard+glasses 로 순차 오버라이드 → 각각 5s 대기 → 스크린샷. 정면이 안 나오면 카메라 회전 입력
(camera_input.rs 확인 — 우클릭 드래그/키) 또는 주민 회전 대기로 2~3각도 확보. 출력
`/tmp/wear_closeup_<agent>_<case>_<n>.png`.

판정(이미지 Read로 직접 확인): 수염 = 턱/입 아래, 폭이 얼굴 폭 이하. 안경 = 눈높이, 폭 ≈ 얼굴 폭.
벗어나면 `WearSlot::offset()`·`scale()`(정적 슬롯: Beard·Glasses·Eyes·HandRight·HandLeft만 — 이제 단위 =
주민 월드 미터, 앵커 월드 스케일 = ToolScale)을 조정. 조정마다 재배포가 필요하므로 **계산으로 먼저 맞춘 뒤**
1회 배포로 확인, 최대 2회 배포(v0.8.78~79). 손 슬롯 아이템도 컷 1장으로 확인.

### A6-2. bigmesh 스킨 extent 계산 보정

- `src/world/big_mesh_probe.rs`의 스킨 메시 extent를 Bevy 렌더와 동일 수식으로: 정점 월드 위치 =
  Σ_i w_i · (joint_GT_i.affine() · IBM_i) · p_local. **메시 엔티티 자신의 GlobalTransform을 곱하지 않는다**
  (Bevy 스키닝은 joint 행렬만 사용 — 현 오탐 원인 후보). joint_spread/joint_reach는 참고치로만 남기고
  판정에서 제외.
- 보정 후 평면 컷에서 헤어·눈썹 extent가 0.2~1.5m 범위인지 확인. 그래도 큰 값이면 근접 컷과 대조해
  어느 쪽이 맞는지 보고(코드 근거 포함).
- `tools/e2e/bigmesh.cjs` wear 게이트(wear-* 조상 extent ≤ 3.0m, 0건) PASS를 globe·flat 모두 달성.

### A6-3. 미니맵 좌측 청색 띠

- 재현: 기본 URL(평면) 세로 폰 스샷에서 미니맵 패널 내부 지도 이미지 좌측 띠 유무(전 실행 기록 중
  `/private/tmp/claude-502/-Users-openclaw-workspace-terrafolk-rust/dce9a32b-14d5-456f-b81a-23a039614503/scratchpad/sbm1_0.png`에 관측).
  3회 중 몇 회 나오는지 기록.
- `src/ui/minimap.rs`·`minimap_draw.rs`: 텍스처 크기 변경(데모 56→라이브 128) 시 `Image` 재생성 대신
  `texture_descriptor.size`만 바꾸고 data를 갈아끼우는 경로, 혹은 ImageNode 크기/UV(rect) 고정값 여부 확인.
  원인 확정 시 수정(크기 변경 시 새 Image 핸들로 교체 등), 불명확하면 보고만.

### A6-4. 배포·검증·PR

- 배포 v0.8.78(필요 시 v0.8.79까지). 회귀: globe_artifact base 2회 + `globe,wearbeard` 2회 정확색 < 200,
  `root3d_globe`·`root3d_smoke` PASS, `cargo test`.
- PR #158에 push, 본문에 캘리브레이션 전후 컷 경로·조정값·bigmesh 보정 결과·미니맵 결과 추가. 머지 금지.
- 최종 보고: 기존 형식 + 근접 컷 판정표(주민×케이스×결과).
