//! 설정 창 — ☰ 메뉴 드롭다운(shell.rs)에서 연다. 언어(한/EN)·눈 스킨
//! 강제·조작 힌트. 스킨 강제는 SkinOverride로 — 계절 자동(sync_daynight)보다 우선.

use bevy::prelude::*;
use bevy_egui::egui;

use super::i18n::{Lang, Tr, tr};
use super::theme;
use egui_phosphor::regular;
use super::{UiState, ctx};
use crate::ground::GroundSkin;

/// 메뉴 스킨 강제 — None이면 계절·날씨 자동. sync_daynight가 참조.
#[derive(Resource, Default)]
pub struct SkinOverride(pub Option<GroundSkin>);

pub fn panel(
    mut contexts: bevy_egui::EguiContexts,
    mut override_skin: ResMut<SkinOverride>,
    mut skin: ResMut<GroundSkin>,
    mut ui_state: ResMut<UiState>,
) {
    if !ui_state.show_settings {
        return;
    }
    let Some(c) = ctx(&mut contexts) else { return };
    let lang = ui_state.lang;
    egui::Window::new(tr(lang, Tr::ToggleSettings))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -20.0))
        .default_width((c.viewport_rect().width() - 24.0).min(250.0))
        .frame(super::widgets::card())
        .title_bar(false)
        .show(&c, |ui| {
            super::widgets::title_bar(ui, regular::GEAR, tr(lang, Tr::Menu));
            ui.add_space(3.0);
            // 언어 토글 — 한/EN 즉시 전환(활성=골드 스트로크 필)
            ui.horizontal(|ui| {
                let lang_btn = |label: &str, active: bool| {
                    let text = egui::RichText::new(label)
                        .size(11.0)
                        .color(if active { theme::INK } else { theme::INK_DIM })
                        .strong();
                    egui::Button::new(text)
                        .fill(if active {
                            theme::sel_pill()
                        } else {
                            theme::glass_hi()
                        })
                        .stroke(if active {
                            egui::Stroke::new(1.0_f32, theme::GOLD)
                        } else {
                            egui::Stroke::new(1.0_f32, theme::STROKE_OLIVE)
                        })
                        .corner_radius(999.0)
                };
                if ui.add(lang_btn("한국어", lang == Lang::Ko)).clicked() {
                    ui_state.lang = Lang::Ko;
                }
                if ui.add(lang_btn("English", lang == Lang::En)).clicked() {
                    ui_state.lang = Lang::En;
                }
            });
            // 눈 스킨 강제 — 체크 해제하면 계절 자동 복귀
            let mut snow = override_skin.0 == Some(GroundSkin::Snow);
            if ui.checkbox(&mut snow, tr(lang, Tr::MenuSnow)).changed() {
                override_skin.0 = snow.then_some(GroundSkin::Snow);
                // 데모·오프라인에선 sync_daynight가 안 돌아도 즉시 반영
                *skin = override_skin.0.unwrap_or(GroundSkin::Normal);
            }
            ui.separator();
            ui.label(egui::RichText::new(tr(lang, Tr::MenuHint1)).size(10.0).color(theme::TEXT_DIM));
            ui.label(egui::RichText::new(tr(lang, Tr::MenuHint2)).size(10.0).color(theme::TEXT_DIM));
            ui.add_space(3.0);
            if ui
                .button(egui::RichText::new(tr(lang, Tr::MenuClose)).size(11.0).color(theme::ACCENT_SAND))
                .clicked()
            {
                ui_state.show_settings = false;
            }
        });
}
