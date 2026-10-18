//! 렌더 GPU 가드 — 약한 GPU·소프트웨어 렌더러(SwiftShader 에뮬레이터, 일부
//! 모바일 드라이버)에서도 앱이 살아 관전을 이어가게 하는 범용 방어.
//! 게임 콘텐츠와 무관 — 어떤 Bevy 게임에도 동일 적용.
//!
//! ⚠️ GPU 전처리 클램프 금지(2026-10-18 실측): 매 프레임 Prepare에서
//! `GpuPreprocessingSupport.max_supported_mode`를 None으로 뒤집으면 배칭
//! 상태가 깨져 텍스처드 메시가 화면에 안 남는다(파란 하늘만 보이는 원인).
//! CPU 배칭이 꼭 필요하면 렌더러 초기화 전(RenderStartup)에 1회 설정할 것.

use bevy::prelude::*;

/// 렌더 검증 오류를 무시하고 계속 — bevy가 에러를 이미 로그로 남긴 뒤
/// 정책만 물어보므로(0.19 error_handler 계약), Ignore는 "로그는 됐고 앱은
/// 생존"을 뜻한다. 관전 앱이 특정 파이프라인 실패(에뮬레이터·구형 GPU)로
/// 통째로 죽는 것보다 나은 트레이드오프. 원인은 로그에서 찾아 고친다.
pub fn setup(app: &mut App) {
    app.insert_resource(bevy::render::error_handler::RenderErrorHandler(
        |_err, _main, _render| bevy::render::error_handler::RenderErrorPolicy::Ignore,
    ));
    // 진단/회피 스위치 — TF_CPU_PREPROC=1이면 GPU 전처리를 RenderStartup에서
    // 1회 비활성화. 매 프레임 클램프 금지 규칙의 유일 예외 경로(부팅 1회).
    // 네이티브(Metal/Vulkan) GPU 전처리 경로에서 스키닝 메시가 조용히 안
    // 그려지는 회귀 조사용(웹 WebGL2=CPU 경로는 정상).
    if let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
        render_app.add_systems(
            bevy::render::RenderStartup,
            |mut support: ResMut<
                bevy::render::batching::gpu_preprocessing::GpuPreprocessingSupport,
            >| {
                if std::env::var("TF_CPU_PREPROC").is_ok() {
                    support.max_supported_mode = bevy::render::batching::gpu_preprocessing::GpuPreprocessingMode::None;
                    info!("[gpu] TF_CPU_PREPROC — GPU 전처리 비활성화(부팅 1회)");
                }
            },
        );
    }
    // 스왑체인 포맷 로그 — "화면이 비는데 에러가 없다" 진단의 첫 확인 포인트.
    #[cfg(target_os = "android")]
    if let Some(render_app) = app.get_sub_app_mut(bevy::render::RenderApp) {
        render_app.add_systems(
            bevy::render::Render,
            log_surface_format
                .run_if(after_half_second)
                .in_set(bevy::render::RenderSystems::Prepare),
        );
    }
}

/// 실제 획득된 스왑체인 텍스처 포맷 로그(최초 1회) — SwiftShader 등에서
/// bevy가 예상 밖 포맷을 골랐는지 즉시 확인.
#[cfg(target_os = "android")]
fn log_surface_format(
    windows: ResMut<bevy::render::view::window::ExtractedWindows>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    *done = true;
    for w in windows.windows.values() {
        match &w.swap_chain_texture_format {
            Some(fmt) => info!(
                "[diag] 스왑체인 포맷={fmt:?} {}x{}",
                w.physical_width, w.physical_height
            ),
            None => warn!("[diag] 스왑체인 텍스처 획득 실패(포맷 없음)"),
        }
    }
}

#[cfg(target_os = "android")]
fn after_half_second(time: Res<Time>) -> bool {
    time.elapsed_secs() > 0.5
}
