//! egui 코지 테마 — 레퍼런스(핸드페인트 cozy 아이소) 팔레트 정렬.
//! 웜 다크 그린 패널·샌드 액센트·둥근 모서리. 폰트 설치 직후 1회 적용.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

/// 팔레트 — CLAUDE.md 레퍼런스 이미지 기준(웜 그린/샌드/2톤 물 대비)
pub const PANEL_BG: egui::Color32 = egui::Color32::from_rgb(30, 36, 24); // 깊은 웜 그린
pub const PANEL_BG_HI: egui::Color32 = egui::Color32::from_rgb(40, 48, 31);
pub const TEXT_WARM: egui::Color32 = egui::Color32::from_rgb(233, 226, 201); // 웜 아이보리
pub const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(158, 168, 134);
pub const ACCENT_SAND: egui::Color32 = egui::Color32::from_rgb(232, 201, 122); // 샌드 골드
pub const LIVE_GREEN: egui::Color32 = egui::Color32::from_rgb(90, 220, 120);

/// 폰트 설치 후 시각(Visuals) 적용 — fonts.rs의 install과 같은 시점에 1회.
pub fn apply(mut contexts: EguiContexts, mut done: Local<bool>) {
    if *done {
        return;
    }
    let Some(ctx) = contexts.ctx_mut().ok().cloned() else {
        return;
    };
    let mut vis = egui::Visuals::dark();
    vis.panel_fill = PANEL_BG;
    vis.window_fill = PANEL_BG;
    vis.extreme_bg_color = PANEL_BG_HI;
    vis.faint_bg_color = PANEL_BG_HI;
    vis.override_text_color = Some(TEXT_WARM);
    vis.selection.bg_fill = egui::Color32::from_rgb(58, 66, 40);
    vis.selection.stroke.color = ACCENT_SAND;
    vis.widgets.noninteractive.bg_fill = PANEL_BG;
    vis.widgets.inactive.bg_fill = PANEL_BG_HI;
    vis.widgets.hovered.bg_fill = egui::Color32::from_rgb(52, 62, 38);
    vis.widgets.active.bg_fill = egui::Color32::from_rgb(64, 76, 46);
    vis.widgets.open.bg_fill = PANEL_BG_HI;
    let round = egui::CornerRadius::same(6);
    vis.widgets.noninteractive.corner_radius = round;
    vis.widgets.inactive.corner_radius = round;
    vis.widgets.hovered.corner_radius = round;
    vis.widgets.active.corner_radius = round;
    vis.window_corner_radius = egui::CornerRadius::same(10);
    vis.menu_corner_radius = egui::CornerRadius::same(8);
    // 라인은 은은한 웜 — 아웃라인 없는 레퍼런스 룩에 맞춰 약하게
    vis.widgets.noninteractive.fg_stroke.color = TEXT_DIM;
    vis.widgets.inactive.fg_stroke.color = TEXT_WARM;
    vis.widgets.hovered.fg_stroke.color = ACCENT_SAND;
    vis.widgets.active.fg_stroke.color = ACCENT_SAND;
    ctx.set_visuals(vis);
    *done = true;
    info!("[ui] egui 코지 테마 적용");
}
