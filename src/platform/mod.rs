//! 런타임 플랫폼 계층 — 게임 콘텐츠와 무관한, 어떤 Bevy 게임에도 재사용되는
//! 실행 환경 대응 모음. `world/`(게임 고유)와 달리 여기는 범용 계약만 다룬다:
//!
//! - **렌더 가드**(`gpu`): 약한 GPU 대응 — GPU 전처리 클램프·렌더 에러 정책.
//!   게임이 wgpu를 잘못 써도 앱이 죽지 않고(관전 앱 관점) 로그로 원인을 남긴다.
//! - **부팅 진단**(`diagnostics`): "화면이 왜 안 나오지?"를 기기에서 바로
//!   잡는 도구 — 창/카메라/가시 메시 프로브 + 인앱 스크린샷(플랫폼별 쓰기
//!   가능 경로 자동 해석). 문제 재현 → 원인 → 수정 루프를 adb logcat으로.
//!
//! 사용: `app.add_plugins(PlatformPlugin)` — GamePlugin이 월드보다 먼저 넣는다.
//! 게임 고유 재질 가드(예: WebGL2 BLEND 금지)는 world/materials_guard 참조.

pub mod diagnostics;
pub mod open_url;
pub mod gpu;
pub mod safe_area;

use bevy::prelude::*;

/// UI가 포인터를 점유 중(버튼 호버·팝업) — 월드 입력(팬·핀치·탭)이 양보.
/// 네이티브 UI 모듈이 매 프레임 갱신. 웹은 DOM UI가 포인터를 아예 흡수하므로
/// 항상 false.
#[derive(Resource, Default)]
pub struct UiHover(pub bool);

/// 모달 UI(팝업·메뉴 드롭다운) 열림 여부 — Escape/뒤로가기 소유권 판정에
/// 월드 입력(pick의 선택 해제)이 참조. 웹은 DOM(tf.modalEsc) 경로.
#[derive(Resource, Default)]
pub struct UiModal(pub bool);

/// '뒤로 가기' 계열 판정(물리 키 경로) — 데스크톱 Escape 등.
pub fn back_pressed(keys: &bevy::input::ButtonInput<bevy::input::keyboard::KeyCode>) -> bool {
    keys.just_pressed(bevy::input::keyboard::KeyCode::Escape)
        || keys.just_pressed(bevy::input::keyboard::KeyCode::BrowserBack)
}

/// '뒤로 가기' 계열 판정(논리 키 경로) — 안드로이드 Back은 winit이 물리 키를
/// 매핑하지 않고 논리 키(Key::Named(BrowserBack))로만 전달하므로 메시지로 감지.
/// 각 시스템이 자기 MessageReader로 호출한다(프레임 내 1회 소진).
pub fn back_in_messages(
    reader: &mut bevy::ecs::message::MessageReader<bevy::input::keyboard::KeyboardInput>,
) -> bool {
    reader.read().any(|e| {
        matches!(
            e.logical_key,
            bevy::input::keyboard::Key::BrowserBack | bevy::input::keyboard::Key::Escape
        )
    })
}

/// 플랫폼 런타임 가드·진단 번들 — 게임 로직과 독립적, 어느 게임에나 적용.
pub struct PlatformPlugin;

impl Plugin for PlatformPlugin {
    fn build(&self, app: &mut App) {
        gpu::setup(app);
        safe_area::setup(app);
        app.add_systems(Startup, safe_area::refresh)
            .add_systems(Update, safe_area::refresh);
        app.init_resource::<UiHover>();
        app.init_resource::<UiModal>();
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(
            Update,
            (diagnostics::boot_probe, diagnostics::debug_screenshot),
        );
    }
}
