//! Terrafolk Bevy 웹 클라이언트 — Phase 1(2.5D 아이소 렌더 프로토타입).
//! 데이터 계약: ../terrafolk/docs/data-contract.md(§17.82). 월드 데이터는 데모
//! 결정론 생성(시드 순수 함수) — Phase 2에서 Colyseus 어댑터로 교체.

pub mod net;
pub mod platform;
// ui 모듈 자체는 양 플랫폼 컴파일(미니맵 웹 공용화) — 등록만 네이티브 UiPlugin
// / 웹 MinimapPlugin으로 분기(아래 GamePlugin).
pub mod ui;
pub mod world;

use bevy::prelude::*;

pub struct GamePlugin;

impl Plugin for GamePlugin {
    fn build(&self, app: &mut App) {
        // 플랫폼 런타임(렌더 가드·진단)을 월드보다 먼저 — 게임 콘텐츠와
        // 무관하게 어떤 게임에도 재사용되는 계층(다른 게임도 이 계약으로 시작).
        app.add_plugins(platform::PlatformPlugin);
        app.add_plugins(world::WorldPlugin);
        app.add_plugins(net::NetPlugin);
        // 네이티브 UI(bevy_ui) — 웹은 DOM UI(tf-*.js)가 있으므로 이중 구현 금지.
        #[cfg(all(not(target_arch = "wasm32"), not(feature = "no-ui")))]
        app.add_plugins(ui::UiPlugin);
        // 미니맵은 예외 — 웹·네이티브 공용 bevy_ui 단일 구현(2026-10-25,
        // tf-minimap.js 폐지. DOM 상단 바 아래 같은 슬롯에 렌더).
        #[cfg(all(not(feature = "no-ui"), not(feature = "no-minimap")))]
        app.add_plugins(ui::minimap::MinimapPlugin);
        // HUD 정보바도 공용(통일 2호) — 웹 DOM 상단바는 ☰ 메뉴만 남고
        // 정보(시계·해시계·날씨·인구·링크)는 이쪽으로.
        #[cfg(all(not(feature = "no-ui"), not(feature = "no-hud")))]
        app.add_plugins(ui::hud::HudPlugin);
    }
}
