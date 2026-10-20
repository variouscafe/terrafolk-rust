//! 설정 창 — ☰ 메뉴 드롭다운(shell.rs)에서 연다. 언어(한/EN)·눈 스킨
//! 강제·조작 힌트. 스킨 강제는 SkinOverride로 — 계절 자동(sync_daynight)보다 우선.

use bevy::prelude::*;
use bevy_egui::egui;

use super::i18n::{Lang, Tr, tr};
use super::theme;
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
        .default_width((c.screen_rect().width() - 24.0).min(250.0))
        .frame(super::widgets::card())
        .title_bar(false)
        .show(&c, |ui| {
            ui.label(
                egui::RichText::new(format!("⚙ {}", tr(lang, Tr::Menu)))
                    .size(13.0)
                    .strong()
                    .color(theme::TEXT_WARM),
            );
            ui.add_space(3.0);
            // 언어 토글 — 한/EN 즉시 전환
            ui.horizontal(|ui| {
                let ko = egui::Button::new(
                    egui::RichText::new("한국어")
                        .size(11.0)
                        .color(if lang == Lang::Ko { theme::TEXT_WARM } else { theme::TEXT_DIM })
                        .strong(),
                )
                .fill(if lang == Lang::Ko {
                    theme::SEL_FILL
                } else {
                    theme::CREAM_3
                })
                .corner_radius(999.0);
                if ui.add(ko).clicked() {
                    ui_state.lang = Lang::Ko;
                }
                let en = egui::Button::new(
                    egui::RichText::new("English")
                        .size(11.0)
                        .color(if lang == Lang::En { theme::TEXT_WARM } else { theme::TEXT_DIM })
                        .strong(),
                )
                .fill(if lang == Lang::En {
                    theme::SEL_FILL
                } else {
                    theme::CREAM_3
                })
                .corner_radius(999.0);
                if ui.add(en).clicked() {
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
