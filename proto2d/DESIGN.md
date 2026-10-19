# proto2d — 엔진·아키텍처 설계 (2026-10-19)

2D 스프라이트 클라이언트 프로토타입. **시스템·데이터는 원본 terrafolk 웹 FE
기반, 그래픽 목표는 레퍼런스 이미지(핸드페인트 cozy 아이소) 최대 반영.**
검증되면 `src/world2d`로 승격해 3D `src/world`를 대체한다.

## 1. 엔진 결정

| 항목 | 결정 | 근거 |
|---|---|---|
| 엔진 | Bevy 0.20.0 (repo 현행) | 메인 클라이언트·WESL 작업과 동일 버전, 자산·지식 공유 |
| 렌더 | `Camera2d` 직교 — **아이소 투상은 스프라이트 자체에 내장**(스폰 시 (x−y)/(x+y) 투영) | SCR·레퍼런스 방식. 진짜 기울임 카메라 아님 → 픽킹·좌표 단순 |
| 필터 | **linear** (페인티드 톤) | 원본 픽셀 소스는 사전 업스케일(LANCZOS+언샵)로 톤 통합 |
| 플랫폼 | 웹(wasm+trunk) 우선 검증 → 안드로이드(xbuild 래퍼 재사용) | 2D는 3D보다 GPU 부담↓ — 저사양 안드로이드에 유리 |
| 배포 | trunk release → wasm-opt **-Os+strip(검증 플래그 고정)** → integrity 갱신 → wrangler pages **별도 프로젝트(terrafolk-2d)** | 메인 pages.dev(terrafolk-rust)를 덮지 않는다 |

## 2. 크레이트 구조 (기능별 분리·500줄 규칙)

```
proto2d/src/
├─ main.rs        부트·플러그인 조립 (thin)
├─ mapdata.rs     맵 데이터(include map.txt=GID)·아이소 투영·워커빌리티·RNG
├─ ground.rs      타일 전개(GID×2변형 배칭)·물 2프레임·미광
├─ props.rs       소품 산포·마을 클러스터·모닥불 애님·그림자 블롭
├─ agents.rs      LPC 주민 — 시트 애님·배회·클릭 선택·마커
├─ cam.rs         팬·줌·추적(수동 조작 유예)
├─ daynight.rs    주야 그레이딩(풀스크린 UI 틴트)·게임 시각
└─ hud.rs         bevy_ui HUD(힌트·시계)
```

향후 승격 시: `mapdata→world2d/data`, `agents→world2d/agents`,
`net/apply`가 WorldState를 밀어넣는 기존 계약 그대로 부착.

## 3. 렌더 아키텍처

- **아이소 투영**: `screen = ((x−y)·64, (x+y)·32)`, 타일 = 128×64 다이아몬드
  PNG(투명 모서리). 지형은 평면(z=0), 오브젝트는 **y-sort**(`z = 10 + py·0.001`)
- **배칭**: GID별 타일 텍스처 2변형(해시 선택) → 같은 텍스처끼리 자동 배칭.
  128×128 맵 ≈ 16k 스프라이트 → 드로우콜 한 자릿수 예상
- **깊이 연출**: 발밑 소프트 그림자 블롭(자식 스프라이트) — 레퍼런스의
  정착감. 액터 z−0.2에 배치
- **그레이딩**: 상시 웜 틴트 + 주야 사이클(밤=딥블루, 새벽/해질=웜)

## 4. 아트 파이프라인 (2026-10-19 개편 — mewki 팩 + assetgen 서비스)

**지형 = mewki 블록 타일**(itch.io, 상업적 사용 허가·재배포 금지):
- 정준 규격 `tiles/BLOCK_SPEC.md` — 캔버스 **256×312**, 상판 다이아몬드
  **256×200**(비율 0.78125 — 팩 실측), 월드 **TW=128/TH=100**, 측벽 y-sort
  (z=py·0.001), 스폰 오프셋 (0, −20).
- grass/forest/rock/dirt: `tools/painted/apply_mewki_blocks.py`가 팩을 정규화.
- **water(4프레임)·sand(4변형): 별도 프로젝트 `~/workspace/assetgen` 서비스가
  생성**(레퍼런스 팔레트·동일 정준 규격) — 신규 에셋은 전부 이 서비스 경로.

| 단계 | 내용 |
|---|---|
| 지형(팩) | mewki 타일 → `apply_mewki_blocks.py` 정규화 |
| 지형(생성) | assetgen `core/blocktile.py`(결정론·시드 고정) |
| 주민 | assets/lpc/parts 부품 + tf-data.js 리컬러 표 → walk 시트 합성 → **×2 LANCZOS+언샵**(페인티드 톤) |
| 소품 | 원본 픽셀 프롭 → **×3 LANCZOS+언샵** |
| 부속 | 그림자 블롭·선택 링 |
| v1+ | 발명품 스프라이트 = assetgen(Tier 2 디퓨전 → Tier 1 후처리 게이트) |

결정론(시드 고정) — 재생성 시 동일 산출. **룩의 단일 기준 = CLAUDE.md 레퍼런스
이미지**(핸드페인트 cozy 아이소).

## 4.5 라이브 모드 (2026-10-19 추가)

- **접속**: root 클라이언트 net 계약 축소 미러(`src/net/` — FrameQueue→drain→적용).
  네이티브=colyseus-lite 스레드(관전 티켓·무인증), 웹=worldsim.js 포크
  (assets/web/ — chat sink 인젤로프·?live=0 스위치가 원본과의 전체 diff).
- **프라임**: mapJson(Tiled) 1회 → 데모 철거(GroundTile/DemoProp/DemoAgent)
  → `mapdata::set_live_map`(OnceLock — 이후 불변) → 라이브 지형 스폰.
- **주민**: 서버 목적지 등속 트윈 1.25타일/s·facing→LPC 행·AgentGrace 5s.
  **lookJson 한계**: uuid 해시로 기존 8시트 결정론 선택 — 부품 합성(원본 FE
  tf-look)은 미구현, 승격 시 과제.
- **자원**: kind/species/stateJson → 프롭 매핑(net/live_props.rs). 동물 배위는
  RoamTo 트윈(§17.64). 건축 LONGHOUSE·WAREHOUSE는 assetgen Tier2(ComfyUI)
  스프라이트 — `core/prop2d.py`가 하단 접지 크롭으로 납품.
- **계절·날씨**: winter/snow → GroundSkin::Snow(육지 GID 전부 iso_snow —
  assetgen 하이브리드). ?exp=w(TF_SKIN=snow)로 프리뷰.
- **채팅**: 수신 → 주민 머리 위 말풍선(6s·페이드). 송신 UI는 미구현.

## 5. 데이터 흐름

- **v0(프로토타입)**: `include_str!` 맵(GID)·villagers.json(생성Manifest) —
  에셋 로드는 AssetServer, 원격 의존 없음
- **v1(라이브)**: 기존 `net/apply` + colyseus-lite가 동일 좌표계(grid)를 밀면
  2D 렌더가 소비 — grid→screen 투영만으로 3D 클라이언트 데이터 계약 호환
- **AI 발명품**: 계약 `sprite_url` → AssetServer 원격 로드(캐시) → 프롭
  슬롯 스폰. 플레이스홀더 = 템플릿 아이콘(계약 현행 `icon{tpl,primary,accent}`)

## 6. 검증 게이트

1. `cargo check --target wasm32-unknown-unknown` 클린
2. 네이티브 `cargo run` — 부트·렌더·입력 육안
3. trunk release + wasm-opt → 로컬 서빙 headless 캔버스 검증(픽셀 분산)
4. 배포 후 pages.dev 헤드리스 E2E(부트 캔버스 크기·비공백)

## 7. 리스크·대응

- LPC(픽셀)↔페인티드 지형 톤 갭 → 업스케일 필터로 완화, 부족하면 v1 베이크로 교체
- 16k 스프라이트 웹 성능 → 배칭 확인, 필요시 청크 culling(`Visibility` 토글)
- 아이소 투상 좌표 혼동 → `mapdata::tile_center` 단일 진입점으로 봉쇄
