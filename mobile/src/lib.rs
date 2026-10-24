use bevy::prelude::*;
use bevy::window::WindowMode;
use bevy::winit::WinitSettings;
use terrafolk_bevy::GamePlugin;

#[unsafe(no_mangle)]
unsafe extern "C" fn main_rs() {
    main();
}

// this macro is a no-op on ios and only needed for android since bevy 0.16
// see https://github.com/bevyengine/bevy/pull/14780
#[bevy_main]
fn main() {
    #[cfg(target_os = "ios")]
    unsafe {
        // Sets our audio session to Ambient mode to prevent background music from stopping.
        // The default for iOS apps is SoloAmbient, which stops background music.
        // See apple docs: https://developer.apple.com/documentation/avfaudio/avaudiosession/category-swift.struct/ambient
        if let Err(e) = objc2_avf_audio::AVAudioSession::sharedInstance()
            .setCategory_error(objc2_avf_audio::AVAudioSessionCategoryAmbient.unwrap())
        {
            println!("Error setting audio session category: {:?}", e);
        }
    }

    let mut app = App::new();
    // 원격 에셋 소스 — AssetPlugin 확정 전 등록(루트 main.rs와 동일).
    // 이것이 없으면 발명품 GLB 로드가 전부 실패한다("remote" 소스 부재 —
    // 모바일 엔트리 누락으로 2026-10-24까지 APK에서 발명품 실물이 안 나옴).
    terrafolk_bevy::net::remote_assets::register_source(&mut app);
    // 연속 렌더링 — mobile() 기본(React: 입력 시에만 프레임)은 살아있는
    // 세계 관전에 부적합(2026-10-17 실측: 무입력 시 정지 프레임, CPU 0%).
    // game 프리셋 = 포커스 중 Continuous + 백그라운드 60Hz 폴.
    app.insert_resource(WinitSettings::game())
        // 0.20 — 그룹 빌더와 플러그인 혼합 튜플 add_plugins 불가, 분리 등록.
        .add_plugins(DefaultPlugins.set(WindowPlugin {
            primary_window: Some(Window {
                resizable: false,
                mode: WindowMode::BorderlessFullscreen(MonitorSelection::Current),
                ..default()
            }),
            ..default()
        }))
        .add_plugins(GamePlugin)
        .run();
}
