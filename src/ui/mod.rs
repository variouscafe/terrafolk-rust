//! 네이티브 UI — 레거시 FE 기능 패리티(bevy_ui, 안드로이드·iOS·PC 공용).
//! 웹(wasm)은 DOM UI(tf-*.js)를 쓰므로 이 모듈은 네이티브에서만 등록한다
//! (UI 이중 구현 금지 원칙 — CLAUDE.md 설계 원칙).
//!
//! 구성(기능별 파일, 500줄 규칙): theme(디자인 토큰) · hud(상단 바) ·
//! menu(☰ 드롭다운·팝업 셸·설정) · chars(주민 목록·선택 패널).

pub mod chars;
pub mod hud;
pub mod menu;
pub mod minimap;
pub mod theme;

use bevy::prelude::*;

/// 팝업 종류 — 레거시 PopupKey 중 네이티브 데모에서 실동작하는 것부터.
/// (res·inventions·log·account는 라이브 서버 연동 기능 — 연결 시 추가)
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Popup {
    Chars,
    Settings,
}

/// UI 상태(메뉴·팝업) — 한 번에 열리는 팝업은 1개(레거시와 동일).
#[derive(Resource, Default)]
pub struct UiState {
    pub menu_open: bool,
    pub popup: Option<Popup>,
}

/// 포인터가 UI 위에 있는지 — 월드 입력(팬·핀치·탭 픽킹)이 이걸 보고 양보한다.
#[derive(Resource, Default)]
pub struct PointerOverUi(pub bool);

/// 팝업 공용 닫기 버튼 마커 — 각 팝업 카드가 하나씩 단다.
#[derive(Component)]
pub struct CloseButton;

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UiState>()
            .init_resource::<PointerOverUi>()
            .init_resource::<minimap::MinimapState>()
            .add_systems(
                Startup,
                (
                    load_theme,
                    crate::platform::safe_area::refresh, // 스폰 직전 인셋 2차 확인
                    (
                        hud::spawn_hud,
                        menu::spawn_menu,
                        chars::spawn,
                        minimap::spawn_minimap,
                    ),
                )
                    .chain(),
            );
        if !cfg!(feature = "ui-min") {
            app.add_systems(
                Update,
                (
                    update_pointer_over_ui,
                    hud::update_hud,
                    menu::interact,
                    chars::refresh_list,
                    chars::refresh_sel_panel,
                    minimap::update_minimap,
                    apply_safe_area,
                )
                    .chain(),
            );
        }
    }
}

/// 인셋이 늦게 확정되면(0.5초 리프레시) 이미 스폰된 노드 위치를 패치.
/// HUD 상단·드롭다운 상단·선택 패널 하단을 SafeArea로 다시 쓴다.
#[allow(clippy::type_complexity)]
fn apply_safe_area(
    safe: Res<crate::platform::safe_area::SafeArea>,
    mut q_hud: Query<&mut Node, With<hud::HudRoot>>,
    mut q_drop: Query<&mut Node, (With<menu::MenuDropdown>, Without<hud::HudRoot>)>,
    mut q_panel: Query<
        &mut Node,
        (
            With<chars::SelPanel>,
            Without<hud::HudRoot>,
            Without<menu::MenuDropdown>,
        ),
    >,
    mut q_mini: Query<
        &mut Node,
        (
            With<minimap::MinimapRoot>,
            Without<hud::HudRoot>,
            Without<menu::MenuDropdown>,
            Without<chars::SelPanel>,
        ),
    >,
    mut applied: Local<crate::platform::safe_area::SafeArea>,
) {
    if (safe.top - applied.top).abs() < 0.5 && (safe.bottom - applied.bottom).abs() < 0.5 {
        return;
    }
    *applied = *safe;
    if let Ok(mut n) = q_hud.single_mut() {
        n.top = Val::Px(safe.top);
    }
    if let Ok(mut n) = q_drop.single_mut() {
        n.top = Val::Px(safe.top + 30.0);
    }
    if let Ok(mut n) = q_panel.single_mut() {
        n.bottom = Val::Px(safe.bottom + 8.0);
    }
    if let Ok(mut n) = q_mini.single_mut() {
        n.top = Val::Px(safe.top + minimap::SAFE_TOP_OFFSET);
    }
}

/// 팝업 백드롭 탭 → 팝업 닫기(모바일 표준 UX). 각 팝업 루트에 observe로 부착.
pub fn close_on_backdrop(_: On<Pointer<Click>>, mut state: ResMut<UiState>) {
    state.popup = None;
}

/// 폰트 로드 → 테마 리소스. HUD·팝업 스폰이 이 뒤에 오므로 스폰 시점에
/// 핸들이 존재한다(에셋 로드는 비동기하지만 핸들 자체는 즉시 생성).
fn load_theme(mut commands: Commands, assets: Res<AssetServer>) {
    commands.insert_resource(theme::UiTheme {
        font: assets.load("fonts/NotoSansKR-Regular.otf"),
    });
}

/// 호버된 버튼이 하나라도 있으면 포인터는 UI 위 — 월드 입력 차단 근거.
fn update_pointer_over_ui(
    interactions: Query<&Interaction>,
    state: Res<UiState>,
    mut over: ResMut<PointerOverUi>,
    mut hover: ResMut<crate::platform::UiHover>,
    mut modal: ResMut<crate::platform::UiModal>,
) {
    over.0 = interactions
        .iter()
        .any(|i| *i == Interaction::Hovered || *i == Interaction::Pressed);
    hover.0 = over.0;
    modal.0 = state.menu_open || state.popup.is_some();
}
