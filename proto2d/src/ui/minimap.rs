//! 미니맵 — GID 색 그리드를 egui ColorImage로(맵+스킨별 1회 베이크).
//! 뷰포트 사각형·주민 점 표시. 레거시 FE tf-minimap.js 패리티.

use bevy::prelude::*;
use bevy_egui::egui;

use super::i18n::{Tr, tr};
use super::{UiState, ctx};
use crate::agents::Agent;
use crate::cam::{ManualUntil, TapPos};
use crate::ground::GroundSkin;
use crate::mapdata;
use crate::net::apply::LiveAgentTag;

/// 베이크된 미니맵 텍스처(맵·스킨 변경 시 재베이크)
#[derive(Resource, Default)]
pub struct MinimapBake {
    pub texture: Option<egui::TextureHandle>,
    pub baked_key: (usize, usize, GroundSkin),
}

fn gid_color(g: u8, skin: GroundSkin) -> egui::Color32 {
    use egui::Color32;
    if skin == GroundSkin::Snow && matches!(g, 2..=6) {
        return Color32::from_rgb(228, 236, 243);
    }
    match g {
        1 => Color32::from_rgb(46, 140, 178),  // water
        2 => Color32::from_rgb(216, 190, 138), // sand
        3 => Color32::from_rgb(112, 165, 92),  // grass
        4 => Color32::from_rgb(64, 112, 62),   // forest
        5 => Color32::from_rgb(140, 138, 132), // rock
        6 => Color32::from_rgb(150, 116, 84),  // dirt
        _ => Color32::from_rgb(90, 90, 90),
    }
}

#[allow(clippy::type_complexity)] // Bevy 파라미터 — 카메라·주민·스킨 동시 참조
pub fn panel(
    mut contexts: bevy_egui::EguiContexts,
    ui_state: Res<UiState>,
    skin: Res<GroundSkin>,
    mut bake: ResMut<MinimapBake>,
    mut camera_q: Query<&mut Transform, With<Camera2d>>,
    agents_q: Query<(&Agent, Option<&LiveAgentTag>)>,
    mut tap: ResMut<TapPos>,
    mut manual: ResMut<ManualUntil>,
    time: Res<Time>,
) {
    if !ui_state.show_minimap {
        return;
    }
    let Some(c) = ctx(&mut contexts) else { return };
    let (mw, mh) = mapdata::dims();
    let key = (mw as usize, mh as usize, *skin);
    if bake.baked_key != key {
        drop(bake.texture.take()); // 이전 텍스처 해제(재베이크 누수 방지)
        let px: Vec<egui::Color32> = (0..mw * mh)
            .map(|i| gid_color(mapdata::gid(i % mw, i / mw), *skin))
            .collect();
        let image = egui::ColorImage {
            size: [mw as usize, mh as usize],
            source_size: egui::vec2(mw as f32, mh as f32),
            pixels: px,
        };
        // 128×128 1회 베이크 — 맵·스킨 변경 시에만 재로드
        bake.texture =
            Some(c.load_texture("minimap", image, egui::TextureOptions::NEAREST));
        bake.baked_key = key;
    }
    let Some(tex) = bake.texture.as_ref() else { return };

    egui::Window::new(tr(ui_state.lang, Tr::Minimap))
        .anchor(egui::Align2::LEFT_BOTTOM, egui::vec2(8.0, -40.0))
        .collapsible(true)
        .resizable(false)
        .show(&c, |ui| {
            let size = egui::vec2(176.0, 176.0);
            let resp = ui.image((tex.id(), size));
            // 클릭 → 해당 타일로 카메라 점프(레거시 미니맵 패리티)
            if resp.clicked()
                && let Some(pos) = resp.interact_pointer_pos()
            {
                let p = (pos - resp.rect.min) / resp.rect.size();
                let (gx, gy) = (p.x * mw as f32, p.y * mh as f32);
                let (px, py) = mapdata::project(gx, gy);
                for mut tr in &mut camera_q {
                    tr.translation.x = px;
                    tr.translation.y = -py;
                }
                tap.0 = None; // 탭 선택 오발동 방지
                // 추적 재개 유예 — 즉시 now를 주면 follow가 카메라를 되납치
                manual.0 = time.elapsed_secs() + 1.5;
            }
            // 뷰포트 사각형 + 주민 점
            if let Ok(cam) = camera_q.single() {
                // 역투영: 카메라 중심 → 그리드 근사(뷰포트는 크게 잡아 표시)
                let scale = 620.0; // 줌 0.8 가시 월드 폭 근사
                let (cx, cy) = (cam.translation.x, -cam.translation.y);
                let gx = (cx / (mapdata::TW * 0.5) + cy / (mapdata::TH * 0.5)) / 2.0;
                let gy = (cy / (mapdata::TH * 0.5) - cx / (mapdata::TW * 0.5)) / 2.0;
                let to_px = |gx: f32, gy: f32| {
                    egui::pos2(
                        resp.rect.min.x + gx / mw as f32 * resp.rect.width(),
                        resp.rect.min.y + gy / mh as f32 * resp.rect.height(),
                    )
                };
                let vp_half = 3.0f32; // 타일 단위 뷰포트 반폭(근사 표시)
                let _ = scale;
                let v0 = to_px(gx - vp_half, gy - vp_half);
                let v1 = to_px(gx + vp_half, gy + vp_half);
                ui.painter().rect_stroke(
                    egui::Rect::from_two_pos(v0, v1),
                    1.0,
                    egui::Stroke::new(1.5_f32, egui::Color32::from_white_alpha(200)),
                    egui::StrokeKind::Middle,
                );
                for (a, _) in &agents_q {
                    let p = to_px(a.gx, a.gy);
                    ui.painter().circle_filled(p, 2.5, egui::Color32::from_rgb(255, 210, 90));
                }
            }
        });
}
