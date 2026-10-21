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
    mut ui_state: ResMut<UiState>,
    net: Option<Res<NetState>>,
    vocab: Res<Vocab>,
    mut mt: ResMut<super::mt::MtCache>,
    mut mt_icon: ResMut<super::icon::IconCache>,
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
            let close = super::widgets::title_bar_close(ui, "木", tr(lang, Tr::ToggleResources));
            if close {
                ui_state.show_resources = false;
            }
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
                        // Phosphor 아이콘(레거시 lucide TreePine 대응 — 한자보다 시인성)
                        let icon = match *kind {
                            "tree" => egui_phosphor::regular::TREE,
                            "plant" => egui_phosphor::regular::PLANT,
                            "gold_vein" => egui_phosphor::regular::DIAMOND,
                            "animal" => egui_phosphor::regular::PAW_PRINT,
                            "grave" => egui_phosphor::regular::MARTINI, // 묘비 대체 없음 — 비석 실루엣
                            _ => egui_phosphor::regular::DOTS_NINE,
                        };
                        egui::Frame::new()
                            .fill(theme::glass_hi())
                            .stroke(egui::Stroke::new(1.0_f32, theme::STROKE_OLIVE))
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
                                                .fill(egui::Color32::from_rgba_unmultiplied(20, 26, 18, 190))
                                                .stroke(egui::Stroke::new(1.0_f32, theme::STROKE_OLIVE))
                                                .corner_radius(6.0)
                                                .inner_margin(egui::Margin::symmetric(6, 2))
                                                .show(ui, |ui| {
                                                    ui.horizontal(|ui| {
                                                        ui.label(
                                                            egui::RichText::new(vocab.sp(lang, sp))
                                                                .size(10.0)
                                                                .color(theme::TEXT_DIM),
                                                        );
                                                        // 개수 배지 — 골드 틴트(레거시 ×n 칩 패리티)
                                                        egui::Frame::new()
                                                            .fill(egui::Color32::from_rgba_unmultiplied(232, 184, 64, 36))
                                                            .corner_radius(5.0)
                                                            .inner_margin(egui::Margin::symmetric(4, 1))
                                                            .show(ui, |ui| {
                                                                ui.label(
                                                                    egui::RichText::new(format!("{n}"))
                                                                        .size(9.0)
                                                                        .color(theme::GOLD_SOFT),
                                                                );
                                                            });
                                                    });
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
                    inv_chips(ui, lang, &mut mt, &mut mt_icon, frame);
                });
        });
}

/// 마을의 발명 칩(레거시 res 팝업 '마을의 발명' 섹션 직역) — 이름 + 개선형 ↑,
/// 호버에 발명가·목적. 아이콘 합성(assetgen)은 미지원 — 텍스트 칩.
fn inv_chips(
    ui: &mut egui::Ui,
    lang: super::i18n::Lang,
    mt: &mut super::mt::MtCache,
    mt_icon: &mut super::icon::IconCache,
    frame: &crate::net::LiveFrame,
) {
    super::widgets::section(ui, "⚒", &tr(lang, Tr::ResInventions));
    let rows: Vec<super::civ::InventionRow> = frame
        .inventions
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    if rows.is_empty() {
        ui.label(
            egui::RichText::new(tr(lang, Tr::ResInvEmpty))
                .size(10.0)
                .color(theme::TEXT_DIM),
        );
        return;
    }
    ui.horizontal_wrapped(|ui| {
        for r in rows {
            let label = if r.based_on.as_deref().unwrap_or("").is_empty() {
                r.name.clone()
            } else {
                format!("{} ↑", r.name)
            };
            let resp = egui::Frame::new()
                .fill(theme::glass_hi())
                .corner_radius(6.0)
                .inner_margin(egui::Margin::symmetric(6, 3))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        // 합성 아이콘(§17.36) — 도착 전엔 텍스트만
                        if let Some(spec) = r.icon.as_ref()
                            && let Some(bytes) = mt_icon.get(spec)
                        {
                            ui.add(
                                egui::Image::from_bytes("inv-icon.png", bytes.to_vec())
                                    .fit_to_exact_size(egui::vec2(14.0, 14.0)),
                            );
                        }
                        ui.label(egui::RichText::new(label).size(10.0).color(theme::TEXT_WARM));
                    });
                })
                .response;
            let inv = r.inventor.as_deref().unwrap_or("");
            let purpose = r.purpose.as_deref().unwrap_or("");
            let purpose = &mt.get(purpose, lang);
            let tip = match (inv.is_empty(), purpose.is_empty()) {
                (false, false) => format!("{inv} — {purpose}"),
                (false, true) => inv.to_owned(),
                (true, false) => purpose.to_owned(),
                (true, true) => String::new(),
            };
            if !tip.is_empty() {
                resp.on_hover_text(tip);
            }
        }
    });
}
