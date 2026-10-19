//! egui UI — 레거시 FE 관전 기능 패리티(전 플랫폼 단일 구현).
#![allow(deprecated)] // egui 0.34 최상위 Panel::show 폐기 예정 — 동작 유지
//! bevy_ui HUD를 대체한다(2026-10-20 사용자 결정 — CLAUDE.md UI 규칙 개정).
//! 셸(상단바·주민 패널)은 shell.rs, 채팅은 chat.rs — 500줄 규칙 분리.

pub mod chat;
pub mod civ;
pub mod daydial;
pub mod fonts;
pub mod menu;
pub mod minimap;
pub mod relations;
pub mod shell;
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
            .add_systems(Startup, fonts::install_korean_font)
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

/// egui 컨텍스트 헬퍼 — 멀티패스 프라이머리 컨텍스트 획득
pub(crate) fn ctx(contexts: &mut EguiContexts) -> Option<egui::Context> {
    contexts.ctx_mut().ok().cloned()
}

/// 시계 문자열(공통) — GameTime → "N일차 HH:MM"
pub(crate) fn clock_text(day: u32, t: f32) -> String {
    let minutes = (t * 24.0 * 60.0) as u32;
    format!("{}일차 {:02}:{:02}", day + 1, minutes / 60, minutes % 60)
}
