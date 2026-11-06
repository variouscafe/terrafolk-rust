# 작업 지시서 — 글로브 정적 소품 워프 캐시 (CPU 성능) — 2026-11-06

실행 모델: sonnet(medium). 순서와 게이트를 그대로 따르고, 판단이 갈리면 **멈추고 보고**한다.

먼저 읽을 것:
- `CLAUDE.md`("개발·배포 절차", "탐색 지름길")
- `src/world/globe.rs` 머리 주석(워프 계약 W1·W2)
- `src/world/mod.rs`의 글로브 스케줄 블록(restore_flat, W2 재전파, apply_warp)
- `docs/globe-small-prop-cull-workorder.md`(직전 작업)

기준: main `9afb324`, 라이브 v0.8.86.

사용자 승인: "진행해줘". 대상은 지구본 화면에서 정적 소품의 매 프레임 위치 재계산을 생략하는 것이다.

## 0. 배경

- 직전 작업(globe_cull)에서 줌아웃 가시 메시를 −68% 줄였지만 프레임 p50은 −3%에 그쳤다. 병목은 CPU로 추정한다.
- 현재 글로브 활성(LastWarp=Some) 동안 매 프레임 흐름:
  1. PreUpdate `restore_flat`이 `Warped` 루트 **전부**(소품 수천 개)의 Transform을 평면으로 되돌리고 `Warped`를 제거한다(커맨드 수천 건).
  2. PreUpdate W2 재전파(mark_dirty_trees → propagate_parent_transforms → sync_simple_transforms)가 바뀐 루트의 하위 트리 **전부**를 다시 계산한다.
  3. PostUpdate `apply_warp`이 루트 전부를 다시 휘게 하고 `Warped`를 삽입한다(커맨드 수천 건).
  4. PostUpdate Propagate가 또 하위 트리 전부를 다시 계산한다.
- 소품(나무·바위·건물 등)은 움직이지 않는데 매 프레임 "평면→구면" 왕복 쓰기 때문에 변경 감지가 켜져 전파 2회와 커맨드 2×N건을 낸다.
- bevy 0.19.1 전파는 `TransformTreeChanged`와 `StaticTransformOptimizations`로 **바뀌지 않은 트리를 건너뛴다**. 즉 정적 루트의 Transform을 건드리지 않으면 그 하위 트리 전파 비용이 사라진다.
- 측정 도구:
  - `?tfdbg=perf`(`src/world/perf_probe.rs`): 구간 ms를 낸다. pre에 W2가 포함되고, post_pre와 post가 있다.
  - `tools/e2e/perf.cjs`의 `ab` 모드(교차 5쌍).
  - Playwright는 `NODE_PATH=/Users/openclaw/workspace/apt/node_modules`로 실행한다.

## 0-1. 공통 규칙

- 브랜치는 `perf/globe-static-warp`(main에서 분기)이다. 단계별로 커밋하고, 커밋 전에 `git status`를 본다.
- 커밋 끝에 `Co-Authored-By: Claude Opus 5.5 <noreply@anthropic.com>`를 붙인다.
- **금지 사항**:
  - push·PR·머지
  - 크레이트 수정
  - 5101 상시 serve
  - **빌드·배포 중 브랜치 전환**
- 배포는 `tools/deploy-3d-r2.sh`로 한다.
  - 배포 전에 `Cargo.toml` 버전을 올리고 diff로 확인한다. 직전 작업에서 sed 실패로 bump가 누락될 뻔했다.
  - 상한은 **3회**이며 0.8.87부터 시작한다.
- rustfmt는 리프 파일만 지정한다(mod.rs 금지). 500줄 규칙을 지킨다(`globe.rs`는 220줄이다. 넘치면 신규 파일 `globe_static.rs`로 분리한다).

## A. 사전 감사 (코드 수정 전 — 결과를 보고서에 표로)

정적 소품의 Transform은 Update 구간에서도 **구면 값으로 남게 된다**. 그래서 W2 계약의 예외가 생긴다. 먼저 아래를 grep으로 전수 조사한다.

1. **쓰기 감사**: First~PostUpdate(apply_warp 이전)에서 `PropRoot` 루트의 `Transform`을 쓰는 시스템을 찾는다.
   - 조사 대상: `models.rs`, `prop_fit.rs`, `net/apply.rs`(라이브 자원 이동: `LiveResourceTag`의 `tr.translation +=` 경로가 이미 확인됨), `wind.rs`, `animal_anim.rs`, `occlusion.rs`, 건물/집 공사, 발명품, 유물, 무덤, 모닥불 `flicker_campfires` 등. `Query<.*&mut Transform` 전체를 대상으로 한다.
   - 루트를 쓰는지, 자식만 쓰는지 구분한다. 자식 로컬만 쓰는 건 무관하다(W1).
2. **읽기 감사**: Update 구간에서 `PropRoot` 루트 또는 그 하위의 `Transform`/`GlobalTransform`을 **평면 좌표로 가정하고 읽는** 시스템을 찾는다.
   - 확인된 곳: `occlusion.rs`의 `q_root`와 `q_hide`(Transform), `net/apply.rs` `debug_goto_animal`, `map_audit.rs`.
   - 그 밖에 픽킹, hover_res, 충돌/회피, 주민 FSM의 목표 위치 계산 등이 소품 엔티티 위치를 읽는지 확인한다.
3. **정적 집합 정의**: 1의 결과로 "Update에서 루트 Transform을 쓰지 않는 PropRoot"를 정적 집합으로 정한다.
   - 움직이는 라이브 동물(`LiveResourceTag`로 이동하는 것)은 **제외**한다.
   - 판정은 종류 하드코딩보다 마커나 쿼리 조건으로 한다. 예: `Without<LiveResourceTag>`인 `PropRoot`. 다만 감사 결과에 맞춘다.
   - 건물·공사 발판 등 PropRoot가 아닌 정적 루트는 **이번 범위 밖**이다. 감사에서 수가 많으면 보고만 한다.
4. 정적 소품이 Update에서 구면 Transform을 갖게 되면 평면을 가정한 소비자가 깨진다. 이 소비자가 **수정 불가하거나 많으면(5곳 초과) 멈추고 보고한다.**

## B. 구현

### B1. 평면 원본 보관 — `GlobeStatic`

```rust
#[derive(Component)]
pub struct GlobeStatic {
    pub flat: Transform,           // 평면 원본(정본)
    pub applied: Option<WarpKey>,  // 현재 Transform에 반영된 워프(None=평면)
}
```

- `WarpKey`는 `WarpFrame`을 비교 가능하게 만든 값이다(center, r_eff를 `to_bits()`로 묶는 등). 프레임이 **비트 단위로 같을 때만** 재쓰기를 생략한다. 줌 애니메이션 중에는 매 프레임 달라지므로 기존과 같은 비용이 든다. 이건 정상이다.
- 삽입 시점: 소품이 `Fitted`(prop_fit 완료) 된 뒤에 넣는다. 이때 Transform이 평면 최종값이어야 한다.
  - 줌아웃 상태에서 새로 스폰되는 경우(라이브 `net::apply` 경로)도 포함한다.
  - 삽입 시스템은 Update 끝(또는 PostUpdate의 apply_warp 직전)에서 `Added<Fitted>` 등으로 처리한다.
  - **삽입 프레임에는 Transform이 평면이어야 한다**. restore 이후, warp 이전 구간이어야 한다.

### B2. 워프 경로 분리

- `apply_warp` 쿼리에 `Without<GlobeStatic>`를 추가한다. 동적 엔티티(주민, 동물, 새 등)는 기존 그대로 처리한다.
- `restore_flat`은 `With<Warped>`만 다룬다. 정적 소품은 `Warped`를 달지 않으므로 자동 제외된다.
- 신규 `warp_static`(PostUpdate, apply_warp과 같은 위치: leg_ik·tool_anchor_fit 이후, Propagate 이전)은 정적 소품마다 다음을 한다.
  - `key = 현재 프레임 키 또는 None`
  - `key == applied`면 **아무것도 쓰지 않는다**. `Mut` 역참조도 하지 않는다. `bypass_change_detection`을 쓰거나 비교 후에만 `as_mut()`를 쓴다.
  - 다르면 `Transform = key.map(warp(flat)).unwrap_or(flat)`로 쓰고 `applied = key`로 둔다.
  - 줌인으로 평면에 복귀할 때(factor<EPS)는 key가 None이 되어 flat 원본으로 **정확히** 돌아간다. 역변환을 쓰지 않으므로 누적 오차도 없다.
- W2 PreUpdate 재전파는 그대로 둔다. 정적 트리는 변경이 없어 bevy가 건너뛴다. 이 점이 성능 이득의 핵심이며, B4 측정으로 확인한다.

### B3. 평면 소비자 보정

A2에서 찾은 소비자가 정적 소품 위치를 읽을 때는 `GlobeStatic.flat`을 쓰도록 바꾼다.

- 헬퍼를 둔다: `pub fn flat_pos(tr: &Transform, st: Option<&GlobeStatic>) -> Vec3`.
- `occlusion.rs`는 필수 대상이다.
- 쓰기 경로(A1)가 정적 소품 루트를 써야 한다면(예: 공사·성장 스케일 변경) 다음 둘 중 하나로 처리한다. 선택 근거는 보고한다.
  - 그 엔티티를 정적 집합에서 빼거나,
  - 쓰기를 `GlobeStatic.flat`에 하고 `applied=None`(재워프 강제)으로 무효화한다.

### B4. 킬스위치·측정

- `?tfdbg=no_globe_static`을 켜면 GlobeStatic 삽입을 하지 않는다. 그러면 기존 경로와 완전히 같다.
- `perf.cjs` ab 모드에서 B 쪽 플래그로 쓸 수 있게 한다. 기존 ab 모드가 플래그를 인자로 받는지 확인하고, 아니면 일반화한다.

## C. 검증 게이트

- **G1 (정확성·기본 줌 픽셀)**: 기본 줌에서 `no_globe_static`과 on의 스크린샷 소품 위치가 같아야 한다.
  - 시각과 날씨 차이를 피하려면 같은 페이지에서 플래그만 다르게 연속 캡처한다.
  - 줌아웃 → 기본 줌 복귀 후에도 소품이 원위치(평면)에 있어야 한다. 복귀 후 `map_audit`(`?tfdbg=mapaudit`) 위치 판정이 이전과 같아야 한다.
- **G2 (글로브 외형)**: 최대 줌아웃에서 on/off 스크린샷을 비교한다.
  - 소품이 지구본 표면에 앉아 있어야 한다(하늘 부유, 지면 파묻힘, 접선 이탈이 없어야 함).
  - **줌 애니메이션 도중**(4.55→6.0 진행 중) 캡처도 1장 이상 보고 소품이 주민·지형과 같이 휘는지 확인한다.
  - 이미지를 직접 Read로 보고 판정 근거를 적는다.
  - 땅이 보이는 구도로 찍는다. 직전 작업은 미니맵에 가렸으니 미니맵 아래 지면이 보이게 카메라 target이나 시점을 조정한다.
- **G3 (회귀)**:
  - `root3d_smoke`, `root3d_globe`(`?tfdbg=globe`, pages.dev URL) allPass
  - `globe_artifact` base 정확색 0, W 모드 통과
  - `globe_cull_gate.cjs` 통과
  - `cargo test` 통과
  - clippy 경고 수를 **main에서 직접 재고**(worktree 또는 브랜치 생성 전 측정) 대비 증가 0
- **G4 (선택 주민 주변 풀숲 투명화)**: occlusion이 글로브 줌 상태에서도 동작해야 한다. 기본 줌에서 주민을 선택했을 때 가림 소품이 숨는지 1회 확인한다(기존 동작 유지).
- **G5 (성능)**: `perf.cjs ab`로 최대 줌아웃 정착 상태를 5쌍(A=no_globe_static, B=on) 측정한다.
  - 쌍별 rAF p50/p95와 `[perf]` 구간 ms(pre·post)를 보고한다.
  - 판정은 직전과 같다. 4/5쌍 같은 부호 + 중앙값 ≥5% 개선이면 "개선 입증"이고, 아니면 "미입증"이다.
  - `[perf]` pre/post 구간 ms 감소는 1차 지표로 수치를 보고한다.
  - 줌 애니메이션 중에는 이득이 없는 것이 정상이며 보고에 명시한다.
  - 개선이 미입증이고 구간 ms 감소도 노이즈 수준이면, 기능을 되돌릴지 판단하지 말고 **보고만** 한다. 결정은 설계자가 한다.

## D. 보고 (한국어, 간결)

- A 감사표: 쓰기·읽기 지점, 정적 집합 정의, 엔티티 수(정적/동적)
- 구현 요지와 변경 파일
- G1~G5 결과(실행 횟수, PNG 경로)
- 교차 A/B 표
- 배포 버전과 커밋 해시
- 보류 항목
- CLAUDE.md "탐색 지름길"에 `no_globe_static` 한 줄 추가
- `globe.rs` 머리 주석에 "W2 예외: GlobeStatic 소품은 Update 구간에도 구면 Transform이며 평면 정본은 GlobeStatic.flat" 계약을 추가
