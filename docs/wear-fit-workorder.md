# 작업 지시서: 주민 착용품 위치 수정 (머리 위 가발, 가슴 앞 안경) — 2026-11-06

- 실행 모델: sonnet(medium). 순서와 게이트는 이 문서대로 따른다. 판단이 갈리면 **멈추고 보고**한다.
- 먼저 읽을 것: `CLAUDE.md`("개발·배포 절차", "탐색 지름길"), 선행 지시서 `docs/globe-wig-perf-workorder.md`.
- 기준 버전: main `c9de7f6`, 라이브 v0.8.82.

## 0. 공통 규칙

- **브랜치**: `fix/wear-fit`(main에서 분기). 단계마다 커밋하고, 커밋 전에 `git status`를 확인한다.
- **금지**: push, PR, 머지. 크레이트(`../3d-chibi-character`) 수정. 로컬 5101 serve 상시 구동.
- **배포**: `tools/deploy-3d-r2.sh`로 배포한 뒤 terrafolk-3d.pages.dev에서 검증한다. 배포할 때마다 버전을 올리고 기록한다. **상한은 4회**이다.
- **추측성 배포 금지**: 위치 보정값은 배포 전에 아래 A1의 해석 계산으로 먼저 정한다.
- rustfmt는 리프 파일만 지정한다(`mod.rs` 금지). 파일 500줄 규칙을 지킨다.
- 커밋 메시지 끝에 `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`를 넣는다.

## 1. 증상 (사용자 스크린샷, v0.8.80 이후 라이브)

- 주민 '테린'의 머리 **위 허공**에 회색 헤어 캡(가발)이 떠 있다. 머리 자체에는 이미 갈색 헤어가 있다.
- **가슴 앞**에 안경처럼 보이는 납작한 판이 있다.
- 배경: v0.8.79까지는 이 착용품들이 ×100으로 거대해서 화면 밖에 있었다. 그래서 사실상 안 보였다. v0.8.80에서 `ToolAnchor`로 크기를 바로잡자 **보정되지 않은 위치**가 드러났다.

## 2. 원인 (설계자가 오프라인으로 확정함)

### 2-1. 헤어·눈썹·입체 수염(qubase) — 머리 위로 뜬다

- 이 GLB들의 정점은 ubase **전신 공간**(발 원점)에 있다. 바인드 포즈 기준으로 헤어는 y 1.50~1.84, 눈썹은 1.65~1.73, beard-full은 1.55~1.69이다.
- 스킨 메시는 바인드 포즈에서 `wear 엔티티 원점 + 정점 좌표`로 그려진다. wear 원점은 Head 본에 오프셋 y −0.10을 더한 위치이므로 헤어가 **약 1.4 단위 × 앵커 스케일만큼 머리 위로** 뜬다.
- wear.rs 주석의 "ubase Head 본(목 뿌리) 기준 모델링"은 **거짓**이다.

### 2-2. qchar 본체 머리에 이미 헤어·눈썹·콧수염이 있다

이전 서브에이전트의 "qchar에 헤어 메시 없음"은 메시 이름만 본 오판이다. Head 메시의 재질은 다음과 같다.

| 모델 | Head 메시 재질 |
|---|---|
| f-adventurer | Hair_Brown, Brown, Skin |
| f-worker | Worker_Yellow(헬멧), DarkBrown, Brown, Skin |
| m-adventurer | Hair, Eyebrows, Eye, Skin |
| m-casual-character | Hair, Eyebrows, Eye, Skin, Skin_Darker |
| m-farmer | Beige, Red(모자), Eyebrows, Eye, Skin |
| m-worker | Moustache, Eyebrows, Worker_Yellow, Eye, Skin |

결론: 전 모델이 헤어(또는 모자·헬멧)와 눈썹을 내장하고 있다. 따라서 wear 헤어·눈썹은 **중복**이고, 모자·헬멧과도 충돌한다.

### 2-3. 안경·LPC 수염 — 가슴 앞에 온다

- LPC 절출 GLB는 원점이 발 앵커이다. 실루엣은 y 0~0.5, 폭 ~1.5의 납작한 판이다.
- 오프셋 `Glasses (0,−0.22,0.12)`, `Beard (0,−0.32,0.10)`과 스케일 0.28/0.30은 앵커가 없던 옛 체인에서 정한 값이다. 앵커 공간(월드 스케일 = `ToolScale`)에서는 너무 낮아서 가슴 높이에 온다.

## 3. 수정

### F1. 헤어·눈썹 기본 착용 제거

- `wear_ids_from_look`에서 기본 헤어(DEFAULT_HAIRS)와 눈썹 추가를 **제거**한다.
- `parts.hair`·`parts.brows` 편집값도 qchar 내장 헤어와 충돌하므로 착용하지 않는다.
- 레지스트리의 `hair-*`, `brows-*` 항목과 GLB 파일은 남겨 둔다. 재사용 여지를 두기 위해서이다.
- 대신 `wear_ids_from_look`에 한 줄 주석으로 이유를 적는다: "qchar Head에 헤어·눈썹 내장 — 2026-11-06".
- `?tfdbg=wearhair` 게이트는 이제 의미가 없으므로 제거한다. CLAUDE.md의 해당 줄도 같이 고친다.

### F2. 수염은 내장 콧수염 모델에서 생략, 나머지는 위치 보정

- m-worker처럼 Head에 `Moustache` 재질이 있는 모델에는 수염 wear를 붙이지 않는다.
  - 판정 방법은 모델 경로를 하드코딩하지 말고, 씬 하위 재질 이름 또는 메시 이름을 검사한다. 재사용 원칙을 따르기 위해서이다.
  - 판정 시점은 `apply_wears`가 이미 씬 완결 후에 돌기 때문에 거기서 하면 된다.
- 위치·크기 보정은 F3의 방법으로 한다.
  - beard-full(Beard3d)은 전신 공간 정점이다. 오프셋 y에 정점 밴드 높이(~1.6 × slot.scale)를 빼는 보정이 필요하다.
  - LPC 수염과 안경은 납작한 판 원점 보정이 필요하다.

### F3. 해석 기반 보정 (배포 전 계산)

1. 진단 프로브 `?tfdbg=wearfit`를 신규로 만든다(`src/world/wear_fit_probe.rs`, 게이트 off면 `run_if`로 비용 0). 주민마다 한 번 로그를 남긴다. 로그 항목:
   - Head 본 월드 위치와 앵커 월드 스케일
   - qchar **Head 메시 월드 AABB**: 정수리, 턱, 눈높이(추정 = AABB 높이의 약 45%), 얼굴 정면 z
   - 각 wear 메시의 **월드 AABB**(스킨이면 조인트 GT 기준)
   - 판정: wear AABB 중심이 Head AABB를 20% 확장한 박스 **안**에 있는지
2. 첫 배포(F1 + 프로브 + 강제 게이트)에서 실측값을 수집한다.
   - 강제 게이트 `?tfdbg=wearglasses`를 신규로 만든다(전 주민 facial_glasses 강제).
   - 기존 `wearbeard`도 같이 쓴다.
3. 실측값으로 `offset()`·`scale()`을 계산한다.
   - 안경: 렌즈 중심을 눈높이·얼굴 정면 바로 앞에 두고, 폭은 머리 폭의 약 90%로 한다.
   - 수염: 턱 높이에 두고, 폭은 머리 폭의 약 60%로 한다.
4. 계산식과 입력값을 커밋 메시지와 코드 주석에 남긴다.
5. 두 번째 배포로 확인한다.

## 4. 검증 게이트 (전부 통과해야 완료)

- **G1 (wearfit 자동 판정)**:
  - 조건: `?tfdbg=wearfit,wearglasses,wearbeard`, 기본 줌.
  - 로그 상 모든 wear 메시가 Head 확장 박스 안에 있어야 한다(위반 0).
  - 헤어·눈썹 wear 엔티티 수는 0이어야 한다.
  - 2회 실행한다.
- **G2 (근접 컷 육안)**: `tools/e2e/wear_closeup.cjs`(필요하면 정면·측면 각도를 추가).
  - 안경: 눈 위치·얼굴 앞에 있어야 한다.
  - 수염: 턱에 있어야 한다.
  - 머리 위에 떠 있는 물체가 없어야 한다.
  - 이미지를 직접 Read로 보고 판정 근거를 적는다. 남녀 각 1명 이상이 필요하다.
  - 가림 때문에 컷을 못 얻으면 카메라 대상이나 각도를 바꿔 재시도하고, "미판정"으로 남기지 않는다.
- **G3 (회귀)**:
  - `globe_artifact.cjs` base 정확색 0
  - `bigmesh.cjs` wear 3m 초과 0
  - `root3d_smoke`, `root3d_globe`(URL 인자로 pages.dev) allPass
  - `cargo test` 통과, clippy 경고 수가 main보다 늘지 않을 것

## 5. 보고 (한국어, 간결하게)

- wearfit 실측값과 보정 계산식
- 변경 파일과 커밋 해시
- 게이트별 결과(실행 횟수 포함)
- 근접 컷 경로와 판정
- 배포 버전 목록
- 보류 항목과 그 이유
