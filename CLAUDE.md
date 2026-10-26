# Terrafolk Rust

스스로 생각하고 살아가는 AI 주민들의 원시 세계 시뮬레이션 게임 Terrafolk의 Rust/Bevy 클라이언트.

**목표 — "AI 시뮬레이션 게임 1위"**([`../terrafolk` 북극성과 동일](../terrafolk/CLAUDE.md)): 이 클라이언트의 모든 작업(그래픽·UX·성능)은 AI 주민의 생명감과 관전 몰입을 최우선으로 판단한다.

## 그래픽 지향점 — 레퍼런스 이미지(2026-10-19)

**룩의 단일 기준**: https://assets.suseona.com/c3946b40-dd41-4311-861e-4cf86cf2f0a6/1a4ab00f-3a5a-43a3-a425-a3daeb611f1b.png
— 핸드페인트 cozy 아이소(팜심 시뮬 재질감). **모든 그래픽 작업(맵·주민·소품·
발명품·에셋 제너레이터)은 이 레퍼런스를 목표로 방향을 맞춘다**: 부드러운
핸드페인트 마크·아웃라인 없음·좌상단 웜키 광원(측벽은 좌밝/우어둠)·유기적
바이옴 블렌딩·웜 그린/샌드/2톤 물(#4FC3D9↔#2E9FC4 + 포말 에지). 지오메트리는
레퍼런스(평평한 2:1)와 현재 채택 재료(mewki 블록 0.7656)가 다를 수 있되,
**채도·명도·광원·재질 감각은 항상 레퍼런스가 우선**이다. 신규 에셋은
별도 프로젝트 **에셋 제너레이터 서비스**(워크스페이스 `assetgen/` →
https://assetgen.pages.dev)가 이 기준으로 생성한다(2026-10-19 결정 — 이 repo가
직접 그리지 않는다). 지형 water·sand도 이 서비스 생성물이며, 발명품 스프라이트는
브레인의 외형 서술(`design`)이 잡 큐를 거쳐 스프라이트로 되돌아오는 연동(계약
§17.86)으로 생성된다. 배포·운영은 `assetgen/CLAUDE.md` 참조. **생성 인프라는
2026-10-26 llmserver(mac studio)로 완전 이관** — ComfyUI·워커·3D meshgen 전부
llmserver에서 `https://llm-asset.suseona.com`(cloudflared 터널)로 동작하고
개발 맥은 순수 클라이언트다(상세·운용 레시피는 assetgen/CLAUDE.md).

## 코드 규칙

- **파일은 기능별로 분리하고 가능한 500줄을 넘지 않는다.** 신규·확장 파일은
  물론 기존 파일이 그 이상으로 자라면 기능 단위로 쪼갠다(예: `net::apply` →
  `net::resources`, `tf-world-ui.js` → `tf-ambience.js`·`tf-minimap.js`).
  JS는 로드 순서 계약(index.html 주석)을 지키면서 분리한다.
- **컴포넌트화 우선**: 새 기능은 독립 모듈·컴포넌트로 만든다(bevy 컴포넌트·
  플러그인 단위, JS는 파일 단위) — 한 파일이 여러 기능을 담당하게 쓰지
  않는다. 리팩토링 시점: 기능 추가로 파일이 500줄을 넘거나 서로 다른
  관심사가 한 파일에 섞이는 순간이 그 적기다 — 미루지 말고 그때 쪼갠다.
  예: `world/models.rs`(스폰+핏+변형) → 스폰/핏/연출 분리 가능 형태.

## 역할 분담

- **이 repo = client 담당**: Rust/Bevy 클라이언트 — **웹 3D(terrafolk-3d.pages.dev,
  루트 terrafolk_bevy)가 우선 타겟**(2026-10-21 전환 — 사용자 지시). 안드로이드
  앱은 후순위 유지(빌드 레시피·adb 검증 루프는 유효) — 웹과 충돌하면 **웹을
  먼저** 맞춘다. iOS·PC(데스크톱) 네이티브도 웹 이후 고려.
- **backend·룸·브레인은 `../terrafolk` repo 담당** — API·권위적 월드 틱·LLM 호출은 그쪽에서 구현하며, 데이터 계약도 그쪽 문서(`../terrafolk/docs/data-contract.md`, §17.82) 기준을 따른다.
- 작업 중 워크스페이스의 repo를 오가면 **현재 repo의 역할에 맞게** 구현할 것.

## 설계 원칙 — 재사용(2026-10-18, 2026-10-24 강화)

다른 게임을 만들 때도 컴포넌트를 그대로 재사용할 수 있게 계층을 나눈다.

- **`src/platform/` = 게임 무관 런타임 계층**(어떤 Bevy 게임에나 복사해서
  쓴다): 렌더 에러 정책(로그 남기고 생존), 스왑체인 포맷 진단, 부팅 프로브
  (창·카메라·가시 메시 — 파란/검은 화면 3분법), 인앱 스크린샷(안드로이드
  쓰기 가능 경로 자동 해석 + adb pull 회수). `PlatformPlugin` 하나로 구성.
- **`src/character/` = 게임 무관 캐릭터 프레임워크**(2026-10-24 지정 —
  `docs/character-module-redesign.md` 참조, 이행 진행 중): 리그 디스크립터·
  비율(치비)·접지(표면장+FootLift+그림자)·다리 IK·행동→모션 매핑·도구
  부착·애님 상태머신·**공간 중력(fall.rs — SpaceGravity 리소스가 세계
  단위 소유, 이 게임=지구 9.81·Fall 적분기·절벽 이력현상)**·**지시 행동
  (action.rs — ActiveAction 상태머신: 토스 캐치·전방 투척 반발·점프·
  둘이서 캐치볼 탄도 해)**. 게임 측은 `CharacterTag`·표면장·activity 코드만
  제공한다. 기존 chibi.rs·ground.rs·foot_lock.rs·wear 앵커는 이 모듈로
  흡수 중 — **캐릭터 기능 추가는 character/에 게임 무관 형태로 먼저**.
- **`src/world/` = 이 게임 콘텐츠**(지형·주민·후처리 카메라 등). 플랫폼
  가드가 필요하면 platform으로, 재질 가드처럼 게임 고유면 world에 둔다.
  캐릭터의 게임 고유 부분(주민 AI·넷 동기·선택)만 world에 둔다.
- **채팅 지시 → 물리 행동(2026-10-24~25)**: 웹 채팅 원문이 ui.js에서
  `tf.chatAction(agentId, text)` 브리지로 들어와 `world/chat_action.rs`가
  한국어 어간표로 파싱(던졌다/던져/멀리/점프/캐치볼…) → 대상 주민에
  `ActiveAction` 부착(수면·수행 중은 명시 거부). **시나리오 추가 = 파서
  표 1행 + ActionKind 변체**. 게이트 `tools/e2e/action_chat.cjs`
  (헤디드, 깨어 있는 앵커 우선 픽 — 이동 주민은 앵커 위치 재판독 필수).
- **아키텍처 게이트(chartcraft 벤치마크, 2026-10-24)**: 계층 의존 방향
  (platform·character ← world만 허용)은 관례가 아니라 점진적으로 빌드
  게이트로 강제한다(cargo xtask layers 패턴). 채택 관행 8종 전체 목록은
  `docs/character-module-redesign.md` §6.
- **UI 이중 구현 금지**: 안드로이드·iOS·PC는 **bevy_ui 단일 구현**으로
  공유한다(플랫폼별 분기 금지 — 해상도·세이프에어리어만 반응형 대응).
  웹만 기존 DOM UI(tf-*.js)를 유지한다. 신규 화면은 native UI 모듈에
  feature 단위 파일로 추가. 현재 `src/ui/`: theme(토큰)·hud(상단 바)·
  menu(☰·설정)·chars(주민 목록·선택 패널). 버튼은 Pointer<Click> 옵저버로
  (Interaction 폴링은 빠른 탭 유실 — 상세 native-ui-bevyui-recipe 메모).
  **예외 — 웹 DOM→bevy_ui 단계 통일(2026-10-25 개시, 사용자 지시)**:
  **미니맵**은 `MinimapPlugin`(minimap.rs·minimap_draw.rs)이 웹(wasm)과
  네이티브 양측에 등록되는 단일 구현이다(tf-minimap.js 폐지 — 웹은 문명
  레이어도 얻음). wasm은 `ui` 모듈의 공용 서브셋(minimap·minimap_draw·
  theme)만 컴파일하고 나머지 네이티브 모듈은 cfg 게이트. 이후 컴포넌트
  이식 때마다 게이트 한 줄씩 해제하는 방식으로 확장.
- **⚠️ GPU 전처리 클램프 금지**: `GpuPreprocessingSupport.max_supported_mode`를
  매 프레임 Prepare에서 None으로 뒤집으면 배칭이 깨져 텍스처드 메시가 통째로
  안 그려진다(2026-10-18 실측 — '파란 화면만 보임'의 원인, a23804b에서
  도입됐다가 제거). CPU 배칭이 꼭 필요하면 렌더러 초기화 전 1회만 설정.

## 개발·배포 절차

실행·구조·배포 상세는 `README.md`를 참조할 것.

**안드로이드 앱(우선)**: 빌드는 `./mobile/build-apk.sh` 원스텝(xbuild+에셋
주입+서명). **산출물은 버전 규칙 `terrafolk-x.y.z.apk`** — 버전의 단일
원천은 루트 `Cargo.toml`의 `[package] version`(출시마다 bump + manifest
`version_code` 증가). 실기 전달은 file-download 스킬(마커
`CODE_SSH_DOWNLOAD v1`). 실물 검증은 에뮬레이터 adb 루프 — 설치·실행·
screencap·logcat(RustStdoutStderr 라인)으로 매번 확인한다. **재설치 검증은
  force-stop 필수(2026-10-23 확정)** — `adb install -r` 후 `am start`만 하면
  구 APK 프로세스가 포어그라운드로 살아돌아와 '지형 단색' 오판을 일으킨다
  (llvmpipe 에뮬 신규 시작은 텍스처 지형 정상). 네이티브 앱은
데모 월드(레거시 FE와 동일 맵, 시드 20261011)로 동작 — 라이브 접속은 웹
전용. **네이티브도 라이브 전제(2026-10-23 데모 제거, PR #83)** — 네이티브 데모
  월드(레거시 맵 임베드)는 용량 부담으로 제거됐다. 네이티브 앱의 기능 목표는
  레거시 FE 패리티(주민 표시·선택·follow·메뉴 — bevy_ui) 그대로.

**웹 테스트 URL**: 자동화 테스트(헤드리스·Playwright 등)는 https://terrafolk-rust.pages.dev 로 접속할 것. 커스텀 도메인(terrafolk.suseona.com)은 Cloudflare 봇 방어가 자동화 접속을 챌린지(403)로 차단한다 — 사람 브라우저는 통과, 자동 검증은 pages.dev 사용. 단 suseona.com 존의 API(tf-api)도 헤드리스 지문을 차단할 수 있어 라이브 백엔드 연동 E2E가 필요하면 headed 모드(`headless: false` + `--disable-blink-features=AutomationControlled` + 실제 UA)로 실행할 것.

**proto2d 웹 배포(terrafolk-2d.pages.dev)**: wasm이 25MiB를 넘으므로 Pages
직행이 안 된다 — `./tools/deploy-proto2d-r2.sh` 원스텝(2026-10-21 확립).
wasm은 R2 버킷 `terrafolk-wasm`(공개 r2.dev 도메인 + CORS `*`)에 올리고,
Pages는 HTML/JS만: 스크립트가 index.html의 `module_or_path`·preload href를
R2 절대 URL로 재작성 + SRI 재계산 후, wasm을 dist 밖으로 대피시켜(하위
디렉터리도 업로드 트리에 포함되므로) `wrangler pages deploy`, 끝나면 복원.
**금지 사항 2가지**: ① **wasm-opt/압축 금지** — 사용자 지시(2026-10-21).
bevy 0.20 그래픽 회귀는 opt 무관하게 0.20 자체 결함으로 판명(0.19.1 유지)
— 빌드 플래그를 바꾸지 말 것. ② wrangler 비대화면 배포는 Pages 프로젝트
바인딩을 빈값으로 덮어쓴다 — R2 바인딩·함수 프록시 방식은 쓰지 말고 절대
URL 방식 유지. 로컬 dist 검증은 trunk가 dist를 지우므로
`./tools/ui2-local.sh`(wasm 복사 후 상대 경로)로.
**배포 전 회귀 게이트 필수(2026-10-21, #63 사고 후)**: 로컬 스택
(backend 8892·room 8893·dist 8095 -c-1)을 띄운 뒤
`NODE_PATH=/opt/homebrew/lib/node_modules TF_MOCK_ECO=1 node
tools/e2e/proto2d_regress.cjs`가 **allPass: true**일 때만 배포한다 —
기능 하나만 검증하면 다른 표면 회귀(#63: 월드 스프라이트가 플로팅
패널 전멸)를 놓친다.

**3D 웹 배포(terrafolk-3d.pages.dev, 2026-10-21)**: 루트 3D 클라이언트
(bevy 0.20.0)의 웹 판 — `./tools/deploy-3d-r2.sh` 원스텝. **Pages 직행
2모드(2026-10-23 개편 — 사용자 지시: R2보다 Pages CDN이 우위)**: wasm-opt
(`-Os --strip-debug --all-features`, 사용자 허용 2026-10-23 — `-Oz`는 UBO
손상 전례로 금지, `--all-features` 필수: rustc 1.97 GC 명령) 후 raw ≤25MiB면
dist 그대로 직행, 초과면 `.wasm.gz` 탑재 + index.html `<body>` 최상단에
DecompressionStream fetch 래퍼 주입(스트리밍 해제→컴파일, 로컬 raw 폴백).
구 R2+glue 레시피는 `TF_DEPLOY_R2=1` 비상 폴백으로 잔존(r2.dev는 엣지
캐시 부재·레이트리밋 — 실측 2026-10-23). `assets/models` GLB 64종 필수(PR #69로
복구 — `cc2895d^`에서 `git checkout`으로 되돌릴 수 있음).
DOM UI 패널 모듈(2026-10-22, 500줄 규칙): `tf-civ.js`(문명)·
`tf-chars.js`(캐릭터)·`tf-reslog.js`(자원·기록)가 **tf-ui.js보다 먼저**
로드(create 팩토리를 tf-ui 모듈 스코프가 소비). 패널 분리 시 누락 클로저
심볼은 패널 오픈 시에만 발화 — **전 패널 오픈 스윕**으로 검증할 것. **웹(wasm)은
데모 월드가 컴파일 아웃**(src/world/mod.rs — 라이브 접속→프라임 직행,
로딩 해제=tf.uiPrime)이므로 **라이브 백엔드 없으면 지형이 안 뜬다**:
tf-api CORS 허용목록(../terrafolk `backend/src/terrafolk_api/config.py`)
에 `https://terrafolk-3d.pages.dev` 등록돼 있다(미등록 오리진은 영구
스피너). 게이트 `tools/e2e/root3d_smoke.cjs` — **headed 필수**(기본:
시스템 Chrome 채널; `HEADLESS=1`은 swiftshader 부팅 검사만 — 지형
프라임 안 됨). 루트 클라이언트는 **bevy 0.19.1 핀**(cba92d4) — 0.20은
문서화된 wasm 텍스처 회귀(proto2d 0.19.1 핀과 같은 원인)로 보수적 회피.
**원근 카메라(2026-10-23, PR #84)**: 정사영→원근(FOV 40°) 전환 — 줌=거리
  매핑(`cam_dist`)으로 줌 체감·미니맵·줌투커서 공식 불변. 요 0° 맵 변 정렬.

**줌아웃 글로브 전환(2026-10-25, docs/globe-map-review.md 안 A 구현)**: 줌
  4.55→6.0(=ZOOM_MAX) 구간에서 월드가 구면 행성으로 휘고 카메라는 궤도로 —
  시뮬레이션·계약은 평면 그대로(착시 곡률). 지형·물 = 정점 셰이더 워프
  (`terrain_globe_vertex.wgsl`·`water.wgsl` 버텍스 — globe.rs와 수식 계약),
  월드 엔티티 = `world/globe.rs`의 PreUpdate 복원/PostUpdate 워프 쌍
  (`Warped` 태그로 평면 원본 보존). 판정·투영·픽킹은 워프된 GlobalTransform
  공간에서 자동 정합, `ground_at`만 구면 교차 분기. 함정 3종(모듈 임포트
  무음 실패→인라인·Plane3d 분할 0·θ>π 이중 피복 z-fight)은 검토서 참조.
  게이트 `tools/e2e/root3d_globe.cjs`(?tfdbg=globe 부팅 + 줌 복귀 회귀).

**외형 편집 3D 반영(2026-10-23, PR #78)**: 레거시 lookJson parts(피부·머리색·눈·옷색)가
  3D 주민 재질 틴트로 반영된다 — `world/look_tint.rs`(LPC 팔레트·모델×재질 슬롯표,
  악세서리 미염색) + toonify 색 매칭 역추적(Gltf named_materials) + look 변경 재스폰.
  검증 게이트 `?tfdbg=looktest`(주민별 진한 팔레트 순환). **지오메트리 축도
  1차 완료(2026-10-23)**: 헤어 GLB qubase 2종(#86, 경량화 c2a139d 2048→512
  텍스처 6.6→1.1MB)·수염 7종(#90)·안경 4종(#91·#93) 부착 + 봇 기본 외형
  uuid 분배(#92). 남은 것: 헤어 스타일 N종 확장(LPC 카드 각도 문제 유보),
  handlebar 수염(실루엣 3px 미생성).

**하얀 맵 사고 정정(2026-10-21)**: 단색 지형의 주원인은 bevy 버전이 아니라
**cc2895d가 지운 `assets/tiles/terrain_atlas.*`·`assets/shaders/` 4종 부재**
였다(GLB 64종과 함께 삭제됐는데 src 참조가 남음 — 렌더 에러 생존 정책이
조용한 백지를 만듦). 전부 `cc2895d^`에서 복구했고, 재발 방지로
**`tools/check-assets.sh`(src 참조↔assets/ 실재 정합)를 deploy-3d-r2.sh·
build-apk.sh 빌드 전에 강제**한다. 복구 후 실측: 프라임 3초·낮 지형
초록 68%·panic 0.

