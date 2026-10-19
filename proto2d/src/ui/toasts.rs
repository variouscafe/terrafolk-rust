//! 토스트 — 새 이벤트 알림(우하단 스택·4초 페이드). EventLog 신규 행에서 공급.

use bevy::prelude::*;
use bevy_egui::egui;

use super::ctx;
use super::EventLog;

#[derive(Default)]
pub struct Toast {
    pub text: String,
    pub ttl: f32,
}

#[derive(Resource, Default)]
pub struct ToastQ {
    pub items: Vec<Toast>,
}

const TOAST_SECS: f32 = 4.0;
const MAX_TOASTS: usize = 4;

/// EventLog 신규 행 → 토스트 적재
pub fn collect(
    mut toasts: ResMut<ToastQ>,
    log: Res<EventLog>,
    mut seen: Local<usize>,
) {
    // 로그가 잘린 경우(100 상한 드레인) 시점 재동기
    if *seen > log.rows.len() {
        *seen = log.rows.len();
    }
    while *seen < log.rows.len() {
        let text = log.rows[*seen].clone();
        toasts.items.push(Toast {
            text,
            ttl: TOAST_SECS,
        });
        *seen += 1;
    }
    if toasts.items.len() > MAX_TOASTS {
        let cut = toasts.items.len() - MAX_TOASTS;
        toasts.items.drain(0..cut);
    }
}

/// 토스트 렌더 — ttl 감소·마지막 1초 페이드
pub fn show(mut contexts: bevy_egui::EguiContexts, time: Res<Time>, mut toasts: ResMut<ToastQ>) {
    if toasts.items.is_empty() {
        return;
    }
    let Some(c) = ctx(&mut contexts) else { return };
    for t in &mut toasts.items {
        t.ttl -= time.delta_secs();
    }
    toasts.items.retain(|t| t.ttl > 0.0);
    if toasts.items.is_empty() {
        return;
    }
    egui::Area::new(egui::Id::new("toasts"))
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-8.0, -40.0))
        .show(&c, |ui| {
            for t in &toasts.items {
                let alpha = (t.ttl.min(1.0) * 0.88).clamp(0.0, 1.0);
                egui::Frame::new()
                    .fill(egui::Color32::from_rgba_unmultiplied(18, 24, 16, (alpha * 255.0) as u8))
                    .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_white_alpha((alpha * 60.0) as u8)))
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .corner_radius(6.0)
                    .show(ui, |ui| {
                        ui.set_max_width(340.0);
                        ui.label(
                            egui::RichText::new(&t.text)
                                .small()
                                .color(egui::Color32::from_white_alpha((alpha * 235.0) as u8)),
                        );
                    });
            }
        });
}
