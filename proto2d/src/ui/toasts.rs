//! 토스트 — 하이라이트 이벤트 알림(우하단 스택·6초 페이드).
//! 레거시 FE 하이라이트 피드 직역: is_highlight 통과 이벤트만, 아이콘+종색.

use bevy::prelude::*;
use bevy_egui::egui;

use super::i18n::{ev_color, ev_kind_icon};
use super::theme;
use super::vocab::Vocab;
use super::{EventLog, UiState, ctx, is_highlight};

#[derive(Default)]
pub struct Toast {
    pub icon: &'static str,
    pub color: egui::Color32,
    pub text: String,
    pub ttl: f32,
}

#[derive(Resource, Default)]
pub struct ToastQ {
    pub items: Vec<Toast>,
}

const TOAST_SECS: f32 = 6.0;
const MAX_TOASTS: usize = 4;

/// EventLog 신규 행 → 하이라이트만 토스트 적재(렌더 1회 — 언어 고정)
pub fn collect(
    mut toasts: ResMut<ToastQ>,
    log: Res<EventLog>,
    vocab: Res<Vocab>,
    ui_state: Res<UiState>,
    mut seen: Local<usize>,
) {
    // 로그가 잘린 경우(400 상한 드레인) 시점 재동기
    if *seen > log.rows.len() {
        *seen = log.rows.len();
    }
    while *seen < log.rows.len() {
        let e = &log.rows[*seen];
        if is_highlight(&e.kind, &e.payload) {
            let text = vocab.event_text(ui_state.lang, &e.payload);
            if !text.is_empty() {
                toasts.items.push(Toast {
                    icon: ev_kind_icon(&e.kind),
                    color: ev_color(&e.kind),
                    text,
                    ttl: TOAST_SECS,
                });
            }
        }
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
        .anchor(egui::Align2::RIGHT_BOTTOM, egui::vec2(-10.0, -44.0))
        .show(&c, |ui| {
            for t in &mut toasts.items {
                let alpha = (t.ttl.min(1.0) * 0.92).clamp(0.0, 1.0);
                egui::Frame::new()
                    .fill(egui::Color32::from_rgba_unmultiplied(
                        31,
                        42,
                        36,
                        (alpha * 235.0) as u8,
                    ))
                    .stroke(egui::Stroke::new(
                        1.4_f32,
                        egui::Color32::from_rgba_unmultiplied(88, 106, 80, (alpha * 230.0) as u8),
                    ))
                    .inner_margin(egui::Margin::symmetric(10, 6))
                    .corner_radius(12.0)
                    .shadow(egui::Shadow {
                        offset: [0, 3],
                        blur: 10,
                        spread: 0,
                        color: egui::Color32::from_black_alpha((alpha * 110.0) as u8),
                    })
                    .show(ui, |ui| {
                        ui.set_max_width(320.0);
                        ui.horizontal(|ui| {
                            ui.label(
                                egui::RichText::new(t.icon)
                                    .size(12.0)
                                    .color(mul_alpha(t.color, alpha)),
                            );
                            ui.label(
                                egui::RichText::new(&t.text)
                                    .size(10.5)
                                    .color(mul_alpha(theme::INK, alpha)),
                            );
                        });
                    });
            }
        });
}

fn mul_alpha(c: egui::Color32, a: f32) -> egui::Color32 {
    let [r, g, b, ca] = c.to_array();
    egui::Color32::from_rgba_unmultiplied(r, g, b, (ca as f32 * a) as u8)
}
