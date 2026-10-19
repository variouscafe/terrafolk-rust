//! 주야 다이얼 — 해/달 위치로 시각 직관 표시(레거시 FE 시계 다이얼 패리티).
//! 상단바 우측 원형 캔버스: 하루 각도 = t·2π, 낮 반원엔 태양·밤 반원엔 달.

use bevy::prelude::*;
use bevy_egui::egui;

use super::ctx;
use crate::daynight::GameTime;

pub fn dial(
    mut contexts: bevy_egui::EguiContexts,
    gt: Res<GameTime>,
    net: Option<Res<crate::net::NetState>>,
) {
    let Some(c) = ctx(&mut contexts) else { return };
    // 접속 배지 옆 작은 다이얼 — 상단바와 겹치지 않는 우상단 고정
    egui::Area::new(egui::Id::new("daydial"))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-8.0, 34.0))
        .show(&c, |ui| {
            let r = 26.0;
            let (rect, _) = ui.allocate_exact_size(egui::vec2(r * 2.0, r * 2.0), egui::Sense::hover());
            let center = rect.center();
            let p = ui.painter();
            // 배경 원 — 낮(상)/밤(하) 반원
            p.circle_filled(center, r, egui::Color32::from_black_alpha(110));
            p.circle_stroke(
                center,
                r,
                egui::Stroke::new(2.0_f32, egui::Color32::from_white_alpha(70)),
            );
            // 시침 — t(0=자정 아래) 각도. 자정=아래(-π/2 기준 위가 정오)
            let ang = gt.t * 2.0 * std::f32::consts::PI - std::f32::consts::FRAC_PI_2;
            let tip = center + egui::vec2(ang.cos(), ang.sin()) * (r - 4.0);
            let is_day = (0.25..0.75).contains(&gt.t);
            let knob = if is_day {
                egui::Color32::from_rgb(255, 214, 90)
            } else {
                egui::Color32::from_rgb(190, 205, 235)
            };
            p.line_segment(
                [center, tip],
                egui::Stroke::new(2.0_f32, egui::Color32::from_white_alpha(160)),
            );
            p.circle_filled(tip, 4.5, knob);
            // 라이브면 테두리 강조
            if net.as_ref().map(|n| n.status) == Some(crate::net::NetStatus::Connected) {
                p.circle_stroke(center, r + 2.0_f32, egui::Stroke::new(1.0_f32, egui::Color32::from_rgb(90, 220, 120)));
            }
        });
}
