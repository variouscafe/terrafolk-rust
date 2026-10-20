//! 미니맵 — GID 색 그리드 + 자원 오버레이를 egui ColorImage로 베이크(1회).
//! 뷰포트 이중 사각형·주민 점(선택=흰색). 레거시 FE WorldMiniMap 직역.
//! 클릭 → 카메라 점프.

use bevy::prelude::*;
use bevy_egui::egui;

use super::i18n::{Tr, tr};
use super::theme;
use super::{UiState, ctx};
use crate::agents::{Agent, Selected};
use crate::cam::{ManualUntil, TapPos};
use crate::ground::GroundSkin;
use crate::mapdata;
use crate::net::apply::LiveAgentTag;
use crate::net::NetState;

/// 베이크된 미니맵 텍스처(맵·스킨·자원 시그니처 변경 시 재베이크)
#[derive(Resource, Default)]
pub struct MinimapBake {
    pub texture: Option<egui::TextureHandle>,
    /// (맵폭, 맵높이, 스킨, 자원 서명) — 서명은 id·좌표 접지라 동숫자
    /// 이동(수확→새싹)에도 재베이크된다
    pub baked_key: (usize, usize, GroundSkin, u64),
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

/// 자원 종류 → 오버레이 색(레거시 미니맵 자원 색 직역)
fn resource_color(kind: &str) -> Option<egui::Color32> {
    use egui::Color32;
    match kind {
        "tree" => Some(Color32::from_rgb(47, 93, 42)),
        "gold_vein" => Some(Color32::from_rgb(240, 192, 46)),
        "plant" => Some(Color32::from_rgb(184, 233, 134)),
        _ => None,
    }
}

#[allow(clippy::type_complexity)] // Bevy 파라미터 — 카메라·주민·스킨 동시 참조
pub fn panel(
    mut contexts: bevy_egui::EguiContexts,
    ui_state: Res<UiState>,
    skin: Res<GroundSkin>,
    mut bake: ResMut<MinimapBake>,
    mut camera_q: Query<&mut Transform, With<Camera2d>>,
    agents_q: Query<(&Agent, Option<&LiveAgentTag>, Option<&Selected>)>,
    mut tap: ResMut<TapPos>,
    mut manual: ResMut<ManualUntil>,
    time: Res<Time>,
    net: Option<Res<NetState>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
) {
    if !ui_state.show_minimap {
        return;
    }
    let Some(c) = ctx(&mut contexts) else { return };
    let (mw, mh) = mapdata::dims();
    let resources: Vec<&crate::net::LiveResource> = net
        .as_ref()
        .and_then(|n| n.frame.as_ref())
        .map(|f| f.resources.iter().collect())
        .unwrap_or_default();
    let res_sig = resources
        .iter()
        .map(|r| (r.id as u64).wrapping_mul(1_000_003) ^ (r.x as u64) << 20 ^ (r.y as u64) << 8)
        .fold(0u64, |a, b| a.wrapping_mul(31).wrapping_add(b));
    let key = (mw as usize, mh as usize, *skin, res_sig);
    if bake.baked_key != key {
        drop(bake.texture.take()); // 이전 텍스처 해제(재베이크 누수 방지)
        let mut px: Vec<egui::Color32> = (0..mw * mh)
            .map(|i| gid_color(mapdata::gid(i % mw, i / mw), *skin))
            .collect();
        // 자원 오버레이 — 수종·채집 가능 자원만(건축물·무덤은 월드에서 본다)
        for r in &resources {
            if let Some(col) = resource_color(&r.kind)
                && (r.x as usize) < mw as usize
                && (r.y as usize) < mh as usize
            {
                px[r.y as usize * mw as usize + r.x as usize] = col;
            }
        }
        let image = egui::ColorImage {
            size: [mw as usize, mh as usize],
            source_size: egui::vec2(mw as f32, mh as f32),
            pixels: px,
        };
        bake.texture = Some(c.load_texture("minimap", image, egui::TextureOptions::NEAREST));
        bake.baked_key = key;
    }
    let Some(tex) = bake.texture.as_ref() else { return };
    let mobile = windows.single().map(|w| w.width() < 640.0).unwrap_or(false);
    let size_px = if mobile { 112.0 } else { 168.0 };

    egui::Area::new(egui::Id::new("minimap"))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 46.0))
        .show(&c, |ui| {
            egui::Frame::new()
                .fill(theme::glass())
                .stroke(egui::Stroke::new(2.0_f32, theme::STROKE_OLIVE))
                .corner_radius(12.0)
                .inner_margin(egui::Margin::same(4))
                .shadow(egui::Shadow {
                    offset: [0, 4],
                    blur: 14,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(110),
                })
                .show(ui, |ui| {
                    let size = egui::vec2(size_px, size_px);
                    // egui Image 기본 Sense::hover — clicked()가 영원히
                    // false라 점프가 동작하지 않았다(2026-10-21 발견).
                    let resp = ui.add(
                        egui::Image::new((tex.id(), size)).sense(egui::Sense::click()),
                    );
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
                    // 뷰포트 사각형(이중 스트로크) + 주민 점(선택=흰색)
                    if let Ok(cam) = camera_q.single() {
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
                        let v0 = to_px(gx - vp_half, gy - vp_half);
                        let v1 = to_px(gx + vp_half, gy + vp_half);
                        let vr = egui::Rect::from_two_pos(v0, v1);
                        let p = ui.painter_at(resp.rect);
                        p.rect_stroke(
                            vr,
                            1.5,
                            egui::Stroke::new(
                                3.0_f32,
                                egui::Color32::from_rgba_unmultiplied(20, 24, 16, 165),
                            ),
                            egui::StrokeKind::Middle,
                        );
                        p.rect_stroke(
                            vr,
                            1.5,
                            egui::Stroke::new(
                                1.5_f32,
                                egui::Color32::from_rgba_unmultiplied(255, 224, 138, 245),
                            ),
                            egui::StrokeKind::Middle,
                        );
                        for (a, _, sel) in &agents_q {
                            let pt = to_px(a.gx, a.gy);
                            let (col, rad) = if sel.is_some() {
                                (egui::Color32::WHITE, 3.5)
                            } else {
                                (egui::Color32::from_rgb(255, 107, 107), 2.5)
                            };
                            p.circle_stroke(pt, rad + 0.8, egui::Stroke::new(1.5_f32, egui::Color32::from_rgba_unmultiplied(16, 22, 15, 180)));
                            p.circle_filled(pt, rad, col);
                        }
                        // 라이브 테두리 — 미묘한 그린
                        if net.as_ref().map(|n| n.status) == Some(crate::net::NetStatus::Connected) {
                            p.rect_stroke(
                                resp.rect,
                                4.0,
                                egui::Stroke::new(1.2_f32, theme::LIVE_GREEN),
                                egui::StrokeKind::Middle,
                            );
                        }
                    }
                    let _ = tr(ui_state.lang, Tr::Minimap);
                });
        });
}
