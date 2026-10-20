//! 메뉴(☰) — 설정: 눈 스킨 강제·패널 상태·정보. 레거시 FE 메뉴 패리티(최소).
//! 스킨 강제는 SkinOverride(Option)로 — 계절 자동(sync_daynight)보다 우선.

use bevy::prelude::*;
use bevy_egui::egui;

use super::i18n::{Lang, Tr, tr};
use super::ctx;
use crate::ground::GroundSkin;

/// 메뉴 스킨 강제 — None이면 계절·날씨 자동. sync_daynight가 참조.
#[derive(Resource, Default)]
pub struct SkinOverride(pub Option<GroundSkin>);

pub fn panel(
    mut contexts: bevy_egui::EguiContexts,
    mut override_skin: ResMut<SkinOverride>,
    mut skin: ResMut<GroundSkin>,
    mut ui_state: ResMut<crate::ui::UiState>,
    mut show: Local<bool>,
) {
    let Some(c) = ctx(&mut contexts) else { return };
    egui::Area::new(egui::Id::new("menu_btn"))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(8.0, 34.0))
        .show(&c, |ui| {
            if ui.button("≡").clicked() {
                *show = !*show;
            }
        });
    if !*show {
        return;
    }
    egui::Area::new(egui::Id::new("menu_panel"))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(8.0, 68.0))
        .show(&c, |ui| {
            super::widgets::card().show(ui, |ui| {
                ui.set_min_width(230.0);
                let lang = ui_state.lang;
                ui.label(egui::RichText::new(format!("≡ {}", tr(lang, Tr::Menu)))
                    .size(13.0).strong().color(super::theme::TEXT_WARM));
                // 언어 토글 — 한/EN 즉시 전환
                ui.horizontal(|ui| {
                    if ui.selectable_label(lang == Lang::Ko, "한국어").clicked() {
                        ui_state.lang = Lang::Ko;
                    }
                    if ui.selectable_label(lang == Lang::En, "English").clicked() {
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

                ui.separator();
                ui.weak(tr(lang, Tr::MenuHint1));
                ui.weak(tr(lang, Tr::MenuHint2));
                if ui.button(tr(lang, Tr::MenuClose)).clicked() {
                    *show = false;
                }
            });
        });
}
