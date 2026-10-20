//! 자원 패널 — 마을 주변 채집 가능 자원 요약(레거시 FE 자원 팝업 직역).
//! kind별 총수·채집 가능 수(resUsable 미러) + 종 칩. campfire·hut은
//! 건축물이라 제외(레거시와 동일 규칙 — 문명 패널·월드가 담당).

use bevy::prelude::*;
use bevy_egui::egui;

use super::i18n::{Tr, tr};
use super::theme;
use super::vocab::Vocab;
use super::widgets::card;
use super::{UiState, ctx};
use crate::net::NetState;

/// 채집 가능 판정(레거시 resUsable 미러) — tree 성숙 2+, 금맥 잔량,
/// 식물 available+계절 게이팅, 동물 alive. stateJson 파싱 최소형.
fn usable(kind: &str, state: &serde_json::Value, season: &str) -> bool {
    match kind {
        "tree" => state.get("maturity").and_then(|v| v.as_u64()).unwrap_or(0) >= 2,
        "gold_vein" => state.get("amount").and_then(|v| v.as_f64()).unwrap_or(0.0) > 0.0,
        "plant" => {
            let available = state.get("available").and_then(|v| v.as_bool()).unwrap_or(true);
            let seasons = state
                .get("seasons")
                .and_then(|v| v.as_array())
                .map(|a| a.iter().filter_map(|x| x.as_str().to_owned()).collect::<Vec<_>>())
                .unwrap_or_default();
            available && (seasons.is_empty() || seasons.iter().any(|s| *s == season))
        }
        "animal" => state.get("alive").and_then(|v| v.as_bool()).unwrap_or(true),
        _ => true,
    }
}

/// 자원 stateJson 파싱(최소)
fn parse_state(raw: Option<&str>) -> serde_json::Value {
    raw.and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(serde_json::Value::Null)
}

pub fn panel(
    mut contexts: bevy_egui::EguiContexts,
    ui_state: Res<UiState>,
    net: Option<Res<NetState>>,
    vocab: Res<Vocab>,
) {
    if !ui_state.show_resources {
        return;
    }
    let Some(c) = ctx(&mut contexts) else { return };
    let lang = ui_state.lang;
    let Some(frame) = net.as_ref().and_then(|n| n.frame.as_ref()) else {
        return; // 라이브 자원 없음 — 데모는 소품일 뿐(정직 생략)
    };
    // kind별 집계 — 총수·채집 가능·종별 {n, usable}
    let mut order: Vec<&str> = Vec::new();
    let mut stats: std::collections::BTreeMap<String, (u32, u32, std::collections::BTreeMap<String, (u32, u32)>)> = Default::default();
    for r in &frame.resources {
        if matches!(r.kind.as_str(), "campfire" | "hut") {
            continue; // 건축물 — 문명·월드 영역
        }
        let st = parse_state(r.state.as_deref());
        let ok = usable(&r.kind, &st, &frame.season);
        let e = stats.entry(r.kind.clone()).or_insert((0, 0, Default::default()));
        e.0 += 1;
        e.1 += ok as u32;
        if !r.species.is_empty() {
            let s = e.2.entry(r.species.clone()).or_insert((0, 0));
            s.0 += 1;
            s.1 += ok as u32;
        }
        if !order.contains(&r.kind.as_str()) {
            order.push(&r.kind);
        }
    }
    // 총수 내림차순(레거시 정렬)
    order.sort_by_key(|k| std::cmp::Reverse(stats.get(*k).map(|e| e.0).unwrap_or(0)));

    egui::Window::new(tr(lang, Tr::ToggleResources))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -20.0))
        .default_width((c.screen_rect().width() - 24.0).min(300.0))
        .frame(card())
        .title_bar(false)
        .show(&c, |ui| {
            ui.set_min_width((c.screen_rect().width() - 24.0).min(300.0).max(240.0));
            ui.label(
                egui::RichText::new(format!("木 {}", tr(lang, Tr::ToggleResources)))
                    .size(13.0)
                    .strong()
                    .color(theme::TEXT_WARM),
            );
            ui.add_space(2.0);
            if stats.is_empty() {
                ui.label(
                    egui::RichText::new(tr(lang, Tr::ResEmpty))
                        .size(10.5)
                        .color(theme::TEXT_DIM),
                );
                return;
            }
            egui::ScrollArea::vertical()
                .id_salt("reslist")
                .max_height(360.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for kind in &order {
                        let Some((total, ok, species)) = stats.get(*kind) else { continue };
                        let icon = match *kind {
                            "tree" => "木",
                            "plant" => "草",
                            "gold_vein" => "金",
                            "animal" => "獸",
                            "grave" => "墓",
                            _ => "·",
                        };
                        egui::Frame::new()
                            .fill(theme::CREAM_3)
                            .corner_radius(7.0)
                            .inner_margin(egui::Margin::symmetric(8, 6))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.label(
                                        egui::RichText::new(icon)
                                            .size(12.0)
                                            .color(theme::ACCENT_SAND),
                                    );
                                    ui.label(
                                        egui::RichText::new(vocab.kind(lang, kind))
                                            .size(12.0)
                                            .strong()
                                            .color(theme::TEXT_WARM),
                                    );
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(
                                                    tr(lang, Tr::ResUsable)
                                                        .replacen("{}", &ok.to_string(), 1)
                                                        .replacen("{}", &total.to_string(), 1),
                                                )
                                                .size(10.0)
                                                .color(theme::TEXT_DIM),
                                            );
                                        },
                                    );
                                });
                                // 종 칩 — 채집 가능 수 툴팁
                                if !species.is_empty() {
                                    ui.horizontal_wrapped(|ui| {
                                        for (sp, (n, ok)) in species {
                                            let resp = egui::Frame::new()
                                                .fill(theme::CREAM_3)
                                                .corner_radius(6.0)
                                                .inner_margin(egui::Margin::symmetric(6, 2))
                                                .show(ui, |ui| {
                                                    ui.label(
                                                        egui::RichText::new(format!("{} ×{n}", vocab.sp(lang, sp)))
                                                            .size(10.0)
                                                            .color(theme::TEXT_DIM),
                                                    );
                                                })
                                                .response;
                                            let _ = ok;
                                            resp.on_hover_text(
                                                tr(lang, Tr::ResUsable)
                                                    .replacen("{}", &ok.to_string(), 1)
                                                    .replacen("{}", &n.to_string(), 1),
                                            );
                                        }
                                    });
                                }
                            });
                        ui.add_space(2.0);
                    }
                });
        });
}
