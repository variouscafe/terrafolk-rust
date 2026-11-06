# 작업 지시서 — 글로브 줌아웃 소형 소품 숨김 (성능) — 2026-11-06

실행 모델: sonnet(medium). 순서·게이트를 그대로 따르고, 판단이 갈리면 **멈추고 보고**한다.
먼저 읽을 것: `CLAUDE.md`("개발·배포 절차"·"탐색 지름길"), `docs/globe-wig-perf-workorder.md` Part B와 그 보고 결과(아래 0절 요약).
기준: main `6d47bce`, 라이브 v0.8.85. 사용자 승인: "작은 것부터 숨겨 보자" — 줌아웃 외형 변화(소형 소품 숨김) 허용됨.

## 0. 배경 (이전 측정, v0.8.80, CPU 4x, 412×732@2.625)

| 장면 | rAF p50 | 가시 메시 |
|---|---|---|
| 기본 줌 | ~98ms | 240 |
| 최대 줌아웃(글로브) | ~134ms | **4756** |

- 글로브가 느린 이유는 가시 메시 수(렌더 비용)이다.
- 데스크톱 측정 노이즈가 ±20%라서 단발 A/B로는 판정하지 못했다. **이번엔 측정법부터 강화한다**(B절).
- 도구: `?tfdbg=perf`(`src/world/perf_probe.rs`)와 `tools/e2e/perf.cjs`.
  - Playwright는 `NODE_PATH=/Users/openclaw/workspace/apt/node_modules`로 실행한다.
  - `root3d_globe`는 URL 인자로 pages.dev를 넘긴다.

## 0-1. 공통 규칙

- 브랜치는 `perf/globe-small-cull`(main에서 분기)이다. 단계별로 커밋하고, 커밋 전에 `git status`로 확인한다.
- 커밋 메시지 끝에 `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`를 붙인다.
- **금지**:
  - push, PR, 머지
  - 크레이트 수정
  - 5101 상시 serve
  - 빌드·배포 중 브랜치 전환 (이전 사고가 있었다)
- 배포는 `tools/deploy-3d-r2.sh`로 한다. 배포마다 버전을 올리고 기록한다. **상한은 3회**이다.
- rustfmt는 리프 파일만 지정한다(mod.rs 금지). 500줄 규칙을 지킨다.

## A. 구현

### A1. 대상 분류 — `PropKind`의 `target_h`(models.rs) 기준 2단계

| 단계 | 대상(target_h) | 숨김 시작 줌(진입/복귀 히스테리시스) |
|---|---|---|
| S(소) | FlowerClump 0.6, GrassClump 0.55, AnimalSquirrel 0.38, AnimalPheasant 0.55 | zoom ≥ 5.0 숨김 / < 4.9 복귀 |
| M(중) | WoodLog 0.9, AnimalBoar 0.85, GoldVein 1.05, Windbreak 1.05, Stash 1.0, BerryBush 1.15 | zoom ≥ 5.5 숨김 / < 5.4 복귀 |

- 나무·바위·건물·텐트·모닥불(Campfire, 불꽃·라이트 포함)·유물(Relic)·무덤·사슴류·주민은 **숨기지 않는다**.
- 규칙은 `PropKind` 열거로 하드코딩하지 않는다. **`target_h` 임계 상수 2개**(예: `CULL_S_MAX_H = 0.7`, `CULL_M_MAX_H = 1.2`)와 제외 목록(Campfire, Relic)으로 판정한다. 재사용 원칙에 따른 것이다.
- 기준 줌 상수: `globe::ZOOM_FLAT = 4.55`, `ZOOM_FULL = 6.0`(=ZOOM_MAX).

### A2. 메커니즘 — 신규 `src/world/globe_cull.rs`

- 현재 단계(0/1/2)를 리소스로 둔다. **단계가 바뀐 프레임에만** 대상 `PropRoot` 루트의 `Visibility`를 Hidden 또는 Inherited로 바꾼다. 매 프레임 전수 쓰기는 금지한다.
- 새로 스폰되는 소품도 처리한다. 라이브 `net::apply` 경로까지 포함해서, `Added<PropRoot>`에 현재 단계를 즉시 적용한다.
- **충돌 확인을 먼저 한다**: 다른 시스템이 `PropRoot` 루트의 `Visibility`를 쓰는지 grep한다. 대상은 prop_fit, toon, 동물 애님, net/apply, occlusion 등이다.
  - 쓰는 곳이 있으면 덮어쓰기 경합이 생긴다. 이 경우 루트 대신 별도 방식을 쓴다(예: 숨김 전용 래퍼 자식). 방식을 바꿀 때는 보고서에 근거를 적는다.
  - 판단이 어려우면 멈추고 보고한다.
- 숨긴 소품도 픽킹·호버 툴팁 대상에서 빠지는지 확인한다(`pick.rs`, `overlay::hover_res`). 보이지 않는 걸 탭해서 선택되면 안 된다. 빠지지 않으면 제외 처리를 추가한다.
- 킬스위치 `?tfdbg=no_globe_cull`을 둔다. 켜면 기능 전체가 비활성화된다. A/B 측정에 쓴다.
- 시스템 등록은 Update에 한다(카메라 리그 줌 갱신 이후). 렌더 워프와 무관하다.

## B. 측정 (핵심 — 노이즈 대책)

1. `perf.cjs`에 **교차 A/B 모드**를 추가한다.
   - 같은 빌드에서 `?tfdbg=perf,globe` 실행과 `?tfdbg=perf,globe,no_globe_cull` 실행을 **번갈아 5쌍** 돌린다(ABABAB…).
   - 장면은 최대 줌아웃 하나, 각 15초이며 앞 3초는 버린다.
   - 지표는 쌍별 rAF p50·p95와 `[perf]`의 가시 메시 수이다. **중앙값과 쌍별 차이의 부호 일관성**을 보고한다.
2. 판정:
   - **1차 지표(결정적)**: 가시 메시 수 감소율.
   - **2차 지표**: 프레임 시간. 쌍별 차이가 5쌍 중 4쌍 이상 같은 부호이고 중앙값 개선이 ≥5%이면 "개선 입증", 아니면 "미입증"으로 정직하게 보고한다.
3. S만 켠 경우와 S+M을 켠 경우를 구분해서 잰다. 디버그 게이트 `?tfdbg=globe_cull_s_only`를 쓰면 같은 배포에서 잴 수 있다.

## C. 검증 게이트

- **G1 (외형)**: 최대 줌아웃 스크린샷을 cull on/off로 나란히 비교한다. 이미지를 직접 Read로 보고 판정한다.
  - 나무·바위·건물·주민·배지·모닥불이 그대로여야 한다.
  - 숨김 대상만 사라져야 하고, 지형 구멍이나 이상 색이 없어야 한다.
  - 비교 PNG 경로를 보고한다.
- **G2 (복귀)**: 최대 줌아웃 → 기본 줌으로 복귀했을 때 모든 소품이 다시 보여야 한다. `[perf]` 가시 메시 수가 원래 수준이어야 하고, 기본 줌 스크린샷이 cull off와 동일해야 한다. 줌 경계(4.9~5.0, 5.4~5.5)에서 깜빡임이 없는지도 본다. 히스테리시스로 확인하고, 가능하면 경계 줌을 고정해 3초간 단계 변화 로그가 0인지 본다.
- **G3 (회귀)**:
  - `root3d_smoke`, `root3d_globe` allPass
  - `globe_artifact` base 정확색 0
  - `cargo test` 통과
  - clippy 경고 수가 main 대비 늘지 않을 것
- **G4 (성능)**: B절 결과표. 가시 메시 감소율은 반드시 수치로 보고한다. 프레임 시간이 미입증이어도 G1~G3를 통과하면 기능은 유지한다. 메시 감소 자체가 실기 GPU 부하를 줄이기 때문이다. 단, 보고서에 "미입증"을 명시한다.

## D. 마무리·보고 (한국어, 간결)

- CLAUDE.md "탐색 지름길"에 `no_globe_cull`, `globe_cull_s_only`, perf A/B 모드를 각각 한 줄씩 추가한다.
- 보고에 넣을 것:
  - 충돌 grep 결과와 선택한 메커니즘
  - 대상 종류별 엔티티 수
  - 가시 메시 수(off / S / S+M)
  - 프레임 시간 교차 A/B 표
  - G1~G4 결과
  - 배포 버전
  - 커밋 해시
  - 보류 항목
