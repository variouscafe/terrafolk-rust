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
pub mod gpu;

use bevy::prelude::*;

/// 플랫폼 런타임 가드·진단 번들 — 게임 로직과 독립적, 어느 게임에나 적용.
pub struct PlatformPlugin;

impl Plugin for PlatformPlugin {
    fn build(&self, app: &mut App) {
        gpu::setup(app);
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(
            Update,
            (
                diagnostics::boot_probe,
                diagnostics::debug_screenshot,
            ),
        );
    }
}
