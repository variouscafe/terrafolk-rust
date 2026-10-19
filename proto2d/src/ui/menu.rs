//! 메뉴(☰) — 설정: 눈 스킨 강제·패널 상태·정보. 레거시 FE 메뉴 패리티(최소).
//! 스킨 강제는 SkinOverride(Option)로 — 계절 자동(sync_daynight)보다 우선.

use bevy::prelude::*;
use bevy_egui::egui;

use super::ctx;
use crate::ground::GroundSkin;

/// 메뉴 스킨 강제 — None이면 계절·날씨 자동. sync_daynight가 참조.
#[derive(Resource, Default)]
pub struct SkinOverride(pub Option<GroundSkin>);

pub fn panel(
    mut contexts: bevy_egui::EguiContexts,
    mut override_skin: ResMut<SkinOverride>,
    mut skin: ResMut<GroundSkin>,
    mut show: Local<bool>,
) {
    let Some(c) = ctx(&mut contexts) else { return };
    egui::Area::new(egui::Id::new("menu_btn"))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(8.0, 34.0))
        .show(&c, |ui| {
            if ui.button("☰").clicked() {
                *show = !*show;
            }
        });
    if !*show {
        return;
    }
    egui::Area::new(egui::Id::new("menu_panel"))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(8.0, 68.0))
        .show(&c, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(230.0);
                ui.heading("메뉴");
                // 눈 스킨 강제 — 체크 해제하면 계절 자동 복귀
                let mut snow = override_skin.0 == Some(GroundSkin::Snow);
                if ui.checkbox(&mut snow, "❄ 눈 스킨(강제)").changed() {
                    override_skin.0 = snow.then_some(GroundSkin::Snow);
                    // 데모·오프라인에선 sync_daynight가 안 돌아도 즉시 반영
                    *skin = override_skin.0.unwrap_or(GroundSkin::Normal);
                }
                ui.separator();
                ui.weak(format!(
                    "패널: 주민·채팅·이벤트·미니맵 — 상단바 토글로 제어"
                ));
                ui.separator();
                ui.weak("Terrafolk 2D proto — 라이브 관전");
                ui.weak("드래그 팬 · 휠 줌 · 주민 클릭 선택/추적");
                if ui.button("닫기").clicked() {
                    *show = false;
                }
            });
        });
}
