//! egui UI — 레거시 FE 관전 기능 패리티(전 플랫폼 단일 구현).
#![allow(deprecated)] // egui 0.34 최상위 Panel::show 폐기 예정 — 동작 유지
//! bevy_ui HUD를 대체한다(2026-10-20 사용자 결정 — CLAUDE.md UI 규칙 개정).
//! 셸(상단바·주민 패널)은 shell.rs, 채팅은 chat.rs — 500줄 규칙 분리.

pub mod chat;
pub mod civ;
pub mod daydial;
pub mod fonts;
pub mod i18n;
pub mod menu;
pub mod minimap;
pub mod relations;
pub mod shell;
pub mod theme;
pub mod toasts;
pub mod tooltip;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

/// 채팅 로그(말풍선과 별개 UI 패널 소스) — 최근 60행
#[derive(Resource, Default)]
pub struct ChatLog {
    pub rows: Vec<ChatLogRow>,
}

pub struct ChatLogRow {
    /// 도착 시각(초) — 타임스탬프 표시 예약(패리티 확장 시)
    #[allow(dead_code)]
    pub time: f32,
    /// 화자 주민 id — 근접 말풍선 정합 예약
    #[allow(dead_code)]
    pub agent_id: String,
    pub speaker: String,
    pub text: String,
}

/// 라이브 이벤트 로그(최근 100행) — 관전 타임라인
#[derive(Resource, Default)]
pub struct EventLog {
    pub rows: Vec<String>,
}

/// UI 상태 — 패널 표시 토글
#[derive(Resource)]
pub struct UiState {
    pub show_chat: bool,
    pub show_villagers: bool,
    pub show_events: bool,
    pub show_minimap: bool,
    pub show_civ: bool,
    pub show_relations: bool,
    pub lang: i18n::Lang,
}

impl Default for UiState {
    fn default() -> Self {
        UiState {
            show_chat: true,
            show_villagers: true,
            show_events: false,
            show_minimap: true,
            show_civ: false,
            show_relations: false,
            lang: i18n::Lang::default(),
        }
    }
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChatLog>()
            .init_resource::<EventLog>()
            .init_resource::<UiState>()
            .init_resource::<menu::SkinOverride>()
            .init_resource::<toasts::ToastQ>()
            .init_resource::<minimap::MinimapBake>()
            .add_plugins(bevy_egui::EguiPlugin::default())
            // 폰트 설치는 Update — 에셋이 비동기라 Startup 1회로는 절대 로드 전
            // 완료 못 함(웹 전체 두부 원인, 2026-10-20 실측). done 플래그로
            // 설치 후엔 매 프레임 조기 반환.
            .add_systems(Startup, (theme::apply, responsive_defaults))
            .add_systems(Update, fonts::install_korean_font)
            .add_systems(
                EguiPrimaryContextPass,
                (
                    shell::top_bar,
                    shell::villagers_panel,
                    chat::panel,
                    chat::events_panel,
                    minimap::panel,
                    daydial::dial,
                    menu::panel,
                    toasts::show,
                    civ::panel,
                    relations::panel,
                    tooltip::show,
                ),
            )
            .add_systems(Update, toasts::collect);
    }
}

/// egui가 포인터/키보드를 점유 중인가 — 월드 입력(클릭 선택·탭·WASD 판)은
/// 이때 스킵해야 UI 조작이 세지 않는다(패널 클릭 → 즉시 선택 해제 사고 방지).
pub fn egui_owns_input(contexts: &mut bevy_egui::EguiContexts) -> bool {
    contexts
        .ctx_mut()
        .ok()
        .map(|c| c.egui_wants_pointer_input() || c.wants_keyboard_input())
        .unwrap_or(false)
}

/// egui 컨텍스트 헬퍼 — 멀티패스 프라이머리 컨텍스트 획득
pub(crate) fn ctx(contexts: &mut EguiContexts) -> Option<egui::Context> {
    contexts.ctx_mut().ok().cloned()
}



/// 모바일 반응형 기본값 — 좁은 화면(폰)은 사이드 패널·미니맵 접기 시작.
/// 상단바 토글로 언제든 전개 가능(기능 손실 없음).
fn responsive_defaults(mut ui_state: ResMut<UiState>, windows: Query<&Window, With<bevy::window::PrimaryWindow>>) {
    let Ok(w) = windows.single() else { return };
    if w.width() < 700.0 {
        ui_state.show_villagers = false;
        ui_state.show_minimap = false;
        ui_state.show_chat = false;
        info!("[ui] 모바일 레이아웃 — 패널 접힘 기본");
    }
}
