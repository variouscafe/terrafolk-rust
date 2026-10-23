// disable console on windows for release builds
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use bevy::DefaultPlugins;
use bevy::asset::AssetMetaCheck;
use bevy::ecs::system::NonSendMarker;
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy::winit::WINIT_WINDOWS;
use std::io::Cursor;
use terrafolk_bevy::GamePlugin;
#[cfg(not(target_arch = "wasm32"))]
use terrafolk_bevy::net::remote_assets;
use winit::window::Icon;

fn main() {
    #[cfg(target_arch = "wasm32")]
    console_error_panic_hook::set_once();
    // 하단 버전 배지(tf-version.js) — Cargo.toml 단일 원천을 컴파일 타임 상수로
    // DOM에 전달. 로컬·배포 동일 경로(2026-10-23 사용자 요청).
    #[cfg(target_arch = "wasm32")]
    announce_version();
    let mut app = App::new();
    // 원격 에셋 소스 — AssetPlugin 확정 전 등록(발명품 3D 모델 §17.86).
    #[cfg(not(target_arch = "wasm32"))]
    remote_assets::register_source(&mut app);
    #[cfg(target_arch = "wasm32")]
    terrafolk_bevy::net::remote_assets::mem::register_source(&mut app);
    app.insert_resource(ClearColor(Color::linear_rgb(0.4, 0.4, 0.4)))
        .add_plugins(
            DefaultPlugins
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Terrafolk — A Living World of AI Villagers".to_string(),
                        // Bind to canvas included in `index.html`
                        canvas: Some("#bevy".to_owned()),
                        fit_canvas_to_parent: true,
                        // Tells wasm not to override default event handling, like F5 and Ctrl+R
                        prevent_default_event_handling: false,
                        ..default()
                    }),
                    ..default()
                })
                .set(AssetPlugin {
                    meta_check: AssetMetaCheck::Never,
                    ..default()
                }),
        )
        .add_plugins(GamePlugin)
        .add_systems(Startup, set_window_icon)
        .run();
}

/// wasm 부팅 버전 통보 — terrafolk.setVersion(env!) → tf-version.js 배지.
/// notify_prime(apply.rs)와 동일 리플렉션 패턴.
#[cfg(target_arch = "wasm32")]
fn announce_version() {
    use wasm_bindgen::prelude::*;
    let Some(window) = web_sys::window() else {
        return;
    };
    let Ok(tf) = js_sys::Reflect::get(&window.into(), &"terrafolk".into()) else {
        return;
    };
    let Ok(f) = js_sys::Reflect::get(&tf, &"setVersion".into()) else {
        return;
    };
    if let Ok(f) = f.dyn_into::<js_sys::Function>() {
        let _ = f.call1(&tf, &env!("CARGO_PKG_VERSION").into());
    }
}

// Sets the icon on windows and X11
fn set_window_icon(
    primary_window: Single<Entity, With<PrimaryWindow>>,
    _non_send_marker: NonSendMarker,
) -> Result {
    WINIT_WINDOWS.with_borrow(|windows| {
        let Some(primary) = windows.get_window(*primary_window) else {
            return Err(BevyError::from("No primary window!"));
        };
        let icon_buf = Cursor::new(include_bytes!(
            "../build/macos/AppIcon.iconset/icon_256x256.png"
        ));
        if let Ok(image) = image::load(icon_buf, image::ImageFormat::Png) {
            let image = image.into_rgba8();
            let (width, height) = image.dimensions();
            let rgba = image.into_raw();
            // 부팅 1회 — 정상 에셋(정합 게이트가 지킴) 전제의 아이콘 로드.
            #[allow(clippy::unwrap_used)]
            let icon = Icon::from_rgba(rgba, width, height).unwrap();
            primary.set_window_icon(Some(icon));
        };

        Ok(())
    })
}
