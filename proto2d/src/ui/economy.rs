//! 경제 패널 — §17.84 경제 관측(REST /economy, 폴러가 채움).
//! 시세 참조(금 가루 g)·발현 KPI·최근 성사. price_g는 0.1g 실수 —
//! '명령 가격'이 아니라 지난 성사의 통계라는 점을 라벨로 명시.

use bevy::prelude::*;
use bevy_egui::egui;

use super::i18n::{Tr, tr, trf};
use super::theme;
use super::vocab::Vocab;
use super::widgets::{card, chip, section};
use super::{UiState, ctx};
use crate::net::{EcoQueue, NetState};

pub fn panel(
    mut contexts: bevy_egui::EguiContexts,
    ui_state: Res<UiState>,
    net: Option<Res<NetState>>,
    vocab: Res<Vocab>,
    eco: Res<EcoQueue>,
) {
    if !ui_state.show_economy {
        return;
    }
    let Some(c) = ctx(&mut contexts) else { return };
    let lang = ui_state.lang;
    let data = eco.0.lock().ok().and_then(|g| g.clone());
    let name_of = |id: &str| -> String {
        net.as_ref()
            .and_then(|n| n.frame.as_ref())
            .and_then(|f| f.agents.iter().find(|a| a.id == id))
            .map(|a| a.name.clone())
            .unwrap_or_else(|| id.chars().take(8).collect())
    };
    egui::Window::new(tr(lang, Tr::Economy))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(196.0, 46.0))
        .default_width((c.screen_rect().width() - 24.0).min(300.0))
        .frame(card())
        .title_bar(false)
        .show(&c, |ui| {
            ui.label(
                egui::RichText::new(format!("貨 {}", tr(lang, Tr::Economy)))
                    .size(13.0)
                    .strong()
                    .color(theme::TEXT_WARM),
            );
            ui.add_space(2.0);
            let Some(d) = data else {
                ui.label(
                    egui::RichText::new(tr(lang, Tr::EcoWait))
                        .size(10.5)
                        .color(theme::TEXT_DIM),
                );
                return;
            };
            // KPI 칩 행
            if let Some(k) = &d.kpi {
                ui.horizontal(|ui| {
                    if let Some(gs) = k.gold_share {
                        chip(ui, &tr(lang, Tr::EcoGoldShare).replace("{}", &format!("{:.0}", gs * 100.0)), theme::WARN, theme::CREAM);
                    }
                    if let Some(v) = k.volume {
                        chip(ui, &tr(lang, Tr::EcoVolume).replace("{}", &v.to_string()), theme::TEXT_WARM, theme::CREAM_3);
                    }
                    if let Some(g) = k.gold_deals {
                        chip(ui, &tr(lang, Tr::EcoGoldDeals).replace("{}", &g.to_string()), theme::TEXT_WARM, theme::CREAM_3);
                    }
                });
                if let Some(day) = k.day {
                    ui.label(
                        egui::RichText::new(trf(lang, Tr::EcoUpdated, &day.to_string()))
                            .size(9.5)
                            .color(theme::TEXT_DIM),
                    );
                }
                ui.add_space(2.0);
            }
            // 시세 — item | 금가격(g) | 성사수 (상위 12, 가격 내림차순 옴)
            section(ui, "秤", tr(lang, Tr::EcoPrices));
            if d.prices.is_empty() {
                ui.label(egui::RichText::new(tr(lang, Tr::EcoWait)).size(10.0).color(theme::TEXT_DIM));
            } else {
                egui::Grid::new("prices")
                    .num_columns(3)
                    .spacing(egui::vec2(10.0, 3.0))
                    .show(ui, |ui| {
                        for p in d.prices.iter().take(12) {
                            ui.label(
                                egui::RichText::new(vocab.item(lang, &p.item))
                                    .size(10.5)
                                    .color(theme::TEXT_WARM),
                            );
                            ui.label(
                                egui::RichText::new(format!("{:.1}g", p.price_g))
                                    .size(10.5)
                                    .color(theme::ACCENT_SAND),
                            );
                            ui.label(
                                egui::RichText::new(format!("×{}", p.trades_seen.unwrap_or(0)))
                                    .size(9.5)
                                    .color(theme::TEXT_DIM),
                            );
                            ui.end_row();
                        }
                    });
            }
            // 최근 성사 — A → B: 나무×3 ⇄ 열매×2
            if !d.deals.is_empty() {
                section(ui, "換", tr(lang, Tr::EcoDeals));
                egui::ScrollArea::vertical()
                    .id_salt("deals")
                    .max_height(130.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        for deal in d.deals.iter().rev().take(10) {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} → {} · {}×{} ⇄ {}×{}",
                                    name_of(&deal.a_id),
                                    name_of(&deal.b_id),
                                    vocab.item(lang, &deal.give_item),
                                    deal.give_qty.unwrap_or(0),
                                    vocab.item(lang, &deal.get_item),
                                    deal.get_qty.unwrap_or(0),
                                ))
                                .size(10.0)
                                .color(theme::TEXT_DIM),
                            );
                        }
                    });
            }
        });
}
