# 작업 지시서 — 주민(캐릭터) 크기 상향 (2026-11-04)

## 0. 배경·판단 (검토 완료 — 재검토 불필요)

사용자 스크린샷: 주민이 나무·덤불·멧돼지에 비해 너무 작아 보인다.

현재 세계 물체 목표 높이(`src/world/models.rs`, 미터):

| 물체 | target_h |
|---|---|
| 활엽수 / 소나무 / 고사목 | 7.2 / 8.0 / 6.4 |
| 꽃덤불(BerryBush) | 1.15 |
| 바위 | 1.5 |
| 멧돼지 / 사슴 / 고라니 | 0.85 / 1.7 / 1.1 |
| 오두막 | 4.2 |
| **주민(가시 신장)** | **1.72** (fit 상수 `VILLAGER_H = 1.79`) |

세계 물체끼리는 실측 비율로 서로 맞는다. 주민도 수치상 실측(1.72m)이지만
① 치비 비율(2.6두신·짧은 다리)이라 몸통이 작게 읽히고 ② 위에서 내려다보는
카메라가 세로 길이를 크게 줄여 덤불(1.15m, 옆으로 넓음)보다 작아 보인다.
코지 팜심 장르 관례(나무:캐릭터 ≈ 2.5~3:1)에 맞추려면 **주민만 키우면 된다.**
나무·동물·건물은 **절대 건드리지 않는다.**

**결정: 주민 배율 ×1.6**(초안 ×1.4 → 사용자 A/B 후 1.6 확정) → 가시 신장 1.72 → 약 2.75m, fit 1.79 → 2.864.
(나무:주민 = 7.2/2.75 ≈ 2.6). URL 게이트로 A/B·미세 조정 가능하게 만든다.

## 1. 구현 (정확히 이 범위만)

### 1-1. 배율 상수 + 게이트 — `src/world/villager.rs`

`pub const VILLAGER_H: f32 = 1.79;` 블록(15~21행 근처)을 아래처럼 바꾼다.
기존 doc 주석 이력은 유지하고 마지막에 한 줄 추가.

```rust
/// (기존 주석 유지) …
/// 2026-11-04: 세계 물체(나무 7~8m·덤불 1.15m) 대비 주민이 작아 보여
/// 주민만 ×VILLAGER_SCALE 상향(docs/villager-scale-up-spec.md).
const VILLAGER_H_BASE: f32 = 1.79;

/// 주민 크기 배율 기본값 — 가시 신장 1.72m × 1.4 ≈ 2.41m.
pub const VILLAGER_SCALE_DEFAULT: f32 = 1.4;

/// 주민 크기 배율 — `?tfdbg=vscale{f}`(예: vscale1.0·vscale1.5)로 A/B.
/// 0.5~3.0 밖·파싱 실패는 기본값. 1회 계산 후 캐시.
pub fn villager_scale() -> f32 {
    static S: std::sync::OnceLock<f32> = std::sync::OnceLock::new();
    *S.get_or_init(|| {
        crate::net::dbg_gate::value("vscale")
            .and_then(|v| v.parse::<f32>().ok())
            .filter(|v| (0.5..=3.0).contains(v))
            .unwrap_or(VILLAGER_SCALE_DEFAULT)
    })
}

/// 주민 fit 목표(미터, 치비화 전 raw AABB 기준) = 기본 × 배율.
pub fn villager_h() -> f32 {
    VILLAGER_H_BASE * villager_scale()
}
```

`pub const VILLAGER_H`는 **삭제**하고, 쓰던 곳을 전부 `villager_h()`로 바꾼다.
현재 사용처(반드시 `grep -rn "VILLAGER_H" src` 로 재확인):
- `src/net/apply.rs` ~393행: `target_h: crate::world::villager::VILLAGER_H,`
  → `target_h: crate::world::villager::villager_h(),`
- `src/world/mod.rs` ~122행 주석의 `VILLAGER_H 1.79` → `villager_h()(기본 1.79×1.4)`로 갱신.
- 그 외 남은 참조가 있으면 동일하게 교체(`src/world/toon.rs`·`villager_spawn.rs`
  등에서 `Villager {` 생성 시 target_h를 상수로 넣는지 확인).

`crate::net::dbg_gate`가 네이티브(non-wasm)에서도 컴파일되는지 확인 —
`value()`는 cfg 분기로 빈 목록을 반환하므로 그대로 쓰면 된다.

### 1-2. 키에 묶인 높이 상수 — 같은 배율 적용

아래 상수는 "주민 키" 기준 절대 미터라 그대로 두면 라벨이 머리에 박히고
선택 판정이 몸통 아래로 쏠린다. **상수 이름은 그대로 두고 `*_BASE`로 바꾼 뒤
사용 지점에서 `* crate::world::villager::villager_scale()`을 곱한다.**

| 파일 | 상수 | 현재 값 | 처리 |
|---|---|---|---|
| `src/world/overlay.rs` ~22 | `ANCHOR_H` | 1.85 | ×scale (머리 위 라벨/말풍선 앵커) |
| `src/ui/world_overlay.rs` ~20 | `ANCHOR_H` | 1.85 | ×scale (네이티브판, 위와 동일값 유지) |
| `src/world/pick.rs` ~22 | `TORSO_H` | 0.8 | ×scale (선택 판정 중심) |
| `src/world/pick.rs` ~20 | `PICK_RADIUS` | 0.8 | ×scale (선택 반경) |
| `src/ui/tooltip.rs` ~22 | `TORSO_H` | 0.8 | ×scale (툴팁 앵커) |
| `src/world/night_glow.rs` ~28 | `radius: 0.5` | 0.5 | ×scale (발밑 미광 원 크기) |

`overlay.rs` 19행 주석의 "주민 상단(VILLAGER_H 1.72)" 문구도 배율 반영으로 갱신.

예시(overlay.rs):
```rust
const ANCHOR_H_BASE: f32 = 1.85;
// 사용 지점
let anchor = gtr.translation() + gtr.up() * (ANCHOR_H_BASE * crate::world::villager::villager_scale());
```
`ui/world_overlay.rs`가 네이티브 전용 모듈이라 `crate::world::villager` 접근이
막혀 있으면(가시성/cfg) 같은 값을 쓰는 방법을 택하되 컴파일되게만 할 것.

### 1-3. 건드리지 말 것 (명시적 금지)

- `src/world/models.rs` 모든 target_h (나무·동물·건물·소품).
- `src/world/bld.rs` 골조 보 1.85m(건축물 치수 — 주민 무관).
- `src/net/inventions.rs` 2.2, `bld_spawn.rs` 수치.
- `../3d-chibi-character` 크레이트(외부 레포) — 일절 수정 금지.
- 카메라(`camera_rig.rs VIEW_H` 등)·잉크 두께(`outline.rs`).
- 걸음 속도·애님 재생속도 — 우선 그대로. 검증에서 발 미끄러짐이 **눈에 띄게**
  보이면 보고만 하고 고치지 말 것.

자식 메시(옷·모자·도구 `wear.rs`/`villager_tools.rs`)는 주민 루트 Transform
scale을 상속하므로 별도 처리 불필요 — 단 검증에서 도구/모자가 따로 노는지 확인.

## 2. 빌드 검증

1. `cargo check` (네이티브) 통과.
2. `cargo check --target wasm32-unknown-unknown` 통과 (wasm 타깃이 설치돼 있으면).
3. `cargo test` 통과 — 실패 시 내 변경 관련인지 분리 보고.
4. **rustfmt는 리프 파일만** 지정(`rustfmt --edition 2021 src/world/villager.rs …`).
   `mod.rs`에 rustfmt 금지(자식 전체 재포맷 캐스케이드). 혹은 rustfmt 생략.
5. `git status`/`git diff --stat`로 의도한 파일만 바뀌었는지 확인.

## 3. 라이브 시각 검증

**이미 `trunk serve --port 5101`이 다른 세션 소유로 떠 있다(`pgrep -fl "trunk serve"`).
절대 kill/재시작하지 말 것.** HMR이 소스 변경을 감지해 자동 재빌드한다.
재빌드 완료는 `curl -s http://127.0.0.1:5101/ | grep -o '_bg.wasm[^"]*'`의 해시
변화나 수십 초 대기로 판단. (5101이 떠 있지 않을 때만 CLAUDE.md의 개발 모드 터널
절차로 직접 띄운다.)

- 라이브 URL: `http://221.143.21.29:5101` (tf-api CORS 등록 오리진. 127.0.0.1:5101은 미등록이라
  주민이 안 뜬다). 터널 ~2MB/s라 wasm 도착에 70~80초 — **waitPrimed 180s 기본값 유지,
  30초 만에 "부팅 실패" 판정 금지.**
- 회귀 스크립트:
  ```
  NODE_PATH=/opt/homebrew/lib/node_modules \
  PNG_WORLD=<scratch>/after_world.png PNG_CIV=<scratch>/after_civ.png \
  node tools/e2e/root3d_live_villagers.cjs http://221.143.21.29:5101
  ```
  판정 항목(부팅·휘도·오버레이 앵커 ≥1·문명 팝업·panic 0) 모두 PASS여야 한다.
- A/B 비교 — 같은 스크립트를 URL만 바꿔 두 번 더:
  - `http://221.143.21.29:5101/?tfdbg=vscale1.0` → `<scratch>/before_world.png` (구 크기)
  - `http://221.143.21.29:5101/` → `<scratch>/after_world.png` (×1.4)
  (스크립트가 쿼리 포함 URL을 그대로 goto하는지 확인; 아니면 소폭 수정 없이
  playwright 원라이너로 캡처.)
- 스크린샷은 **Read 도구로 직접 열어 육안 판독**하고 아래를 확인·보고:
  1. 주민이 덤불보다 뚜렷이 크고, 나무보다는 확실히 작은가(나무:주민 ≈ 3:1 근처).
  2. 이름 라벨/말풍선이 머리 위에 떠 있고 머리에 박히지 않는가.
  3. 발이 지면에 붙어 있는가(공중 부양·파묻힘 없음).
  4. 모자·도구 등 부착물이 몸과 함께 커졌는가.
  5. 잉크 아웃라인이 유지되는가.
- 브라우저 콘솔 panic/에러 0.

## 4. 금지·주의

- 커밋·푸시·배포 금지(사용자 승인 후 별도). 버전 범프도 하지 않는다.
- 다른 세션이 같은 레포를 쓴다 — 내가 바꾸지 않은 파일은 되돌리거나 스테이징하지 말 것.
- 임시 파일은 스크래치패드에만.

## 5. 최종 보고 형식

- 변경 파일 목록 + 각 파일 변경 요지(1줄씩)
- cargo check / wasm check / test 결과(명령·통과 여부·실패 원문)
- E2E 판정 결과 표(before/after)
- 스크린샷 절대경로 2장 + 위 1~5 체크 결과
- 남은 이슈(발 미끄러짐·문 높이 대비 등) — 수정하지 않고 관찰만
