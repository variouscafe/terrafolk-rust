# Terrafolk Rust

스스로 생각하고 살아가는 AI 주민들의 원시 세계 시뮬레이션 게임 Terrafolk의 Rust/Bevy 클라이언트.

**목표 — "AI 시뮬레이션 게임 1위"**([`../terrafolk` 북극성과 동일](../terrafolk/CLAUDE.md)): 이 클라이언트의 모든 작업(그래픽·UX·성능)은 AI 주민의 생명감과 관전 몰입을 최우선으로 판단한다.

## 그래픽 지향점 — 레퍼런스 이미지(2026-10-29 교체, 사용자 지시)

**룩의 단일 기준(정본)**:
https://assets.suseona.com/c3946b40-dd41-4311-861e-4cf86cf2f0a6/e2521c15-799c-4066-9d50-6514c355bdb7.jpg
— 핸드페인트 **애니/지브리풍 해안 마을 일러스트**(사용자 지시 2026-10-29:
"이 이미지와 같은 게임 그래픽을 구현"). **모든 그래픽 작업(맵·물·주민·소품·
발명품·에셋 제너레이터)의 방향은 이 이미지가 우선**한다. 실측 핵심 규칙:
- **무아웃라인·가치대비 조형** — 형태는 선이 아니라 명암·색 전환으로.
- **잠긴 팔레트** — 네이비→시안 2톤 바다(#1A5FA8↔#7FC8E8·포말은 밝은
  웜화이트 뭉치 블롭)·테라코타 지붕(#D95F2B)·크림 석회 벽·올리브그린 초목.
- **광원 = 우상단 웜키(림) + 쿨 앰비언트** — 그림자는 경계 뭉개진 페인티드
  블롭(구 레퍼런스의 좌상단 광원 규칙은 폐지).
- **구도 = 원근 카메라·계단식 해안 절벽 위 조밀 스택 마을**.
- 캐릭터 = **~5두신 청소년**(파란 재킷+러스트 오렌지 백팩) — 캐릭터
  시스템은 2026-10-29 전면 철퇴(아래 '캐릭터 시스템 철퇴')로 현재 몸체가
  없다. 새 캐릭터는 이 이미지 캐릭터(비율·의상 팔레트)를 기준으로 처음부터
  설계·제작한다(구 치비 농부 레퍼런스·바이블은 철퇴와 함께 폐기).

신규 에셋은
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

- **캐릭터 시스템 전면 철퇴(2026-10-29, 사용자 지시)**: KayKit 잔재
  정화를 위해 캐릭터 코드·레퍼런스·캐릭터 키트를 모두 제거했다 —
  `src/character`(셤)·world의 villager/wear/look/outfit/toon/outline 등
  14모듈, Kit GLB(KayKit 마을세트·Villagers, Kenney 캐릭터·aid, qubase,
  lpc, folk) 51파일, 관련 툴·E2E 게이트. 주민은 **데이터 엔티티**
  (`world/agent.rs` — 명·상태·활동·이동 보간)로만 존재하고 UI(목록·채팅·
  미니맵·선택·외형 편집 데이터)는 그대로 동작한다. 구 프레임워크 레포
  `../3d-chibi-character`는 GitHub에 보존(archive)후 워크스페이스에서
  삭제 — 이 repo 빌드는 이제 sibling 없이 자립한다. 새 캐릭터 시스템은
  해안 마을 레퍼런스 캐릭터로 **처음부터 재설계**가 별도 과제.
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
- **`src/world/` = 이 게임 콘텐츠**(지형·주민 데이터·후처리 카메라 등).
  플랫폼 가드가 필요하면 platform으로, 재질 가드처럼 게임 고유면 world에
  둔다. 주민은 현재 `world/agent.rs` 데이터 컴포넌트만 존재(캐릭터 철퇴).
- **아키텍처 게이트(chartcraft 벤치마크, 2026-10-24)**: 계층 의존 방향
  (platform ← world만 허용)은 관례가 아니라 빌드 게이트로 강제한다
  (`tools/check-layers.sh` — cargo xtask layers 패턴).
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

**로컬 무스택 개발(2026-10-27)**: 로컬 postgres 제거로 로컬 백엔드 상시
구동을 폐지했다 — 클라이언트는 로컬 8892가 죽으면 자동으로 aiserver 라이브로
폴백한다(worldsim.js `probeLocalBackend` 700ms no-cors /health · 네이티브
`pick_bases` 600ms TCP — `?tfenv=prod|local`(웹)·`TF_ENV=`(네이티브)로 강제
가능). 로컬 웹 서빙은 CORS 허용 포트 **5174**로 할 것(tf-api 허용목록의 유일
localhost 오리진 — 8095/8096은 CORS 400). 인증 액션은 배포 사이트 로그인 후
localStorage `readAuth` 토큰을 로컬 탭에 복사. proto2d는 별도 레포로
분리돼 있으며(위 proto2d 절 참조) 이 repo 소관이 아니다.

**터널 테스트 루프(2026-10-30, 사용자 지시 — 테스트 시 배포 금지)**: 그래픽
반복 작업은 **Cloudflare Pages 배포 없이** 로컬 dist를 사용자 확인 URL로
노출한다 — `dist/`를 로컬 3000번대에 서빙(`python3 -m http.server 3000
--bind 127.0.0.1`)하고 리버스 터널 `ssh -N -f -p 315 -o ExitOnForwardFailure
=yes root@221.143.21.29 -R 0.0.0.0:3000:localhost:3000` → 사용자는
**http://221.143.21.29:3000** 에서 확인. 포트는 3000~3020(업스트림 방화벽이
**3000만** 외부 개방 — 3001+는 서버 내부에선 열려도 외부에서 안 열림, 실측
2026-10-30). tf-api CORS에는 `http://221.143.21.29:3000~3020`을 등록했다
(aiserver `~/terrafolk/backend/.env` CORS_ORIGINS — 코드 기본값 아님).
백엔드 재시작 레시피: launchd가 아니라 terin 소유 uvicorn 프로세스 —
`cd ~/terrafolk/backend && pkill -f "uvicorn terrafolk_api"; sleep 2;
PYTHONPATH=src nohup .venv/bin/uvicorn terrafolk_api.main:app --host 0.0.0.0
--port 8892 --workers 1 >> ~/terrafolk/logs/backend.log 2>&1 &`(PYTHONPATH
누락 시 ModuleNotFoundError, 로그 경로는 ~/terrafolk/logs 절대경로). 캡처
검증도 터널 URL로 직접(오리진이 CORS 목록에 있음) — 프라임 신호는
**body.tf-loading 클래스 해제**(root3d_smoke 계약, `waitForFunction`).

**proto2d 완전 분리(2026-10-28)**: proto2d는 개발 종료(2026-10-27, 사용자
지시)로 sibling 레포 **`../terrafolk-2d-rust-proto`**(github
variouscafe/terrafolk-2d-rust-proto — 코드·tools·notes 전부 이관,
crates/ws-link·colyseus-lite 밴딩)로 옮겼다. 이 repo에는 proto2d 흔적이
없다. 배포(terrafolk-2d.pages.dev + R2 wasm 우회)·E2E 게이트·wasm-opt
금지 레시피는 그쪽 README.md·CLAUDE.md가 단일 기준. ws-link·colyseus-lite는
양쪽이 계보 공유 — 한쪽 수정 시 반대쪽 반영 검토.

**캐릭터 시스템 철퇴(2026-10-29, 사용자 지시)**: "kaykit 잔재" 정화를 위해
코드·레퍼런스·캐릭터 키트를 전부 제거했다(위 '역할 분담' 항 참조). 이제
이 repo 빌드에 sibling 클론이 필요 없다. 구 /character·/character2 뷰어
페이지 탑재 단계(deploy ⑤.5·⑤.6)도 폐지 — 기존 배포본은 Pages에 남지만
재배포 시 사라진다. 웹 채팅의 물리 행동 브리지(tf.chatAction)·외형 편집
3D 미리보기(tf.setLocalLook·setLookFocus)도 이 철퇴로 폐지(JS 호출부는
가드돼 있어 no-op). 주민 관전 UI(목록·선택·follow·채팅·미니맵 점)는
데이터 엔티티 기반으로 계속 동작.

**3D 웹 배포(terrafolk-3d.pages.dev, 2026-10-21)**: 루트 3D 클라이언트
(bevy 0.20.0)의 웹 판 — `./tools/deploy-3d-r2.sh` 원스텝. **Pages 직행
2모드(2026-10-23 개편 — 사용자 지시: R2보다 Pages CDN이 우위)**: wasm-opt
(`-Os --strip-debug --all-features`, 사용자 허용 2026-10-23 — `-Oz`는 UBO
손상 전례로 금지, `--all-features` 필수: rustc 1.97 GC 명령) 후 raw ≤25MiB면
dist 그대로 직행, 초과면 `.wasm.gz` 탑재 + index.html `<body>` 최상단에
DecompressionStream fetch 래퍼 주입(스트리밍 해제→컴파일, 로컬 raw 폴백).
구 R2+glue 레시피는 `TF_DEPLOY_R2=1` 비상 폴백으로 잔존(r2.dev는 엣지
캐시 부재·레이트리밋 — 실측 2026-10-23). `assets/models` GLB는 Kit 철퇴
(2026-10-29)로 축소됐다 — 현행 세트는 tools/check-assets.sh가 정본
(참조↔실재 정합 게이트).
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

**외형 편집(데이터 축)**: lookJson 편집·저장(웹 tf-ui.js loadMine·lookSave /
  네이티브 ui/look.rs + net/look_sync.rs)은 서버 계약 그대로 동작한다. 3D
  틴트·지오메트리 반영(look_tint·qubase 헤어 부착)은 캐릭터 철퇴(2026-10-29)로
  제거 — 새 캐릭터 시스템이 look 파트를 다시 소비한다.

**하얀 맵 사고 정정(2026-10-21)**: 단색 지형의 주원인은 bevy 버전이 아니라
**cc2895d가 지운 `assets/tiles/terrain_atlas.*`·`assets/shaders/` 4종 부재**
였다(GLB 64종과 함께 삭제됐는데 src 참조가 남음 — 렌더 에러 생존 정책이
조용한 백지를 만듦). 전부 `cc2895d^`에서 복구했고, 재발 방지로
**`tools/check-assets.sh`(src 참조↔assets/ 실재 정합)를 deploy-3d-r2.sh·
build-apk.sh 빌드 전에 강제**한다. 복구 후 실측: 프라임 3초·낮 지형
초록 68%·panic 0.

