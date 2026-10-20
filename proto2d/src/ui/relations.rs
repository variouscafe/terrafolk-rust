//! 관계 패널 — 주민 간 온기·대화·교역(REST /relations, 폴러가 채움).
//! 레거시 FE 관계 패널 직역: 선택 주민 있으면 그 주민 인연 상위 8,
//! 없으면 마을 전체 상위 12. 온기는 0..100 정수(바 hsl, 등급 30/45/65/80).

use bevy::prelude::*;
use bevy_egui::egui;

use super::i18n::{Tr, tr};
use super::theme;
use super::vocab::warmth_hsl;
use super::widgets::card;
use super::{UiState, ctx};
use crate::agents::Selected;
use crate::net::apply::LiveAgentTag;
use crate::net::{NetState, NetStatus, PollQueuesRes, RelRow};

/// id → 이름 매핑 — 라이브 프레임 에이전트 기준(관계는 라이브 전용 데이터라
/// 프레임에 없는 id는 잘라 표기)
fn name_of(id: &str, frame_agents: &[crate::net::LiveAgent]) -> String {
    frame_agents
        .iter()
        .find(|a| a.id == id)
        .map(|a| a.name.clone())
        .unwrap_or_else(|| id.chars().take(8).collect())
}

pub fn panel(
    mut contexts: bevy_egui::EguiContexts,
    ui_state: Res<UiState>,
    net: Option<Res<NetState>>,
    polls: Res<PollQueuesRes>,
    sel_q: Query<&LiveAgentTag, With<Selected>>,
) {
    if !ui_state.show_relations {
        return;
    }
    let Some(c) = ctx(&mut contexts) else { return };
    let lang = ui_state.lang;
    let connected = net.as_ref().map(|n| n.status) == Some(NetStatus::Connected);
    let frame_agents: &[crate::net::LiveAgent] = super::chat::live_agents(&net);
    let mut rows: Vec<RelRow> = polls
        .relations
        .lock()
        .map(|q| q.clone())
        .unwrap_or_default();
    let selected_id = sel_q.iter().next().map(|t| t.0.clone());
    egui::Window::new(tr(lang, Tr::Relations))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-282.0, 44.0))
        .default_width((c.screen_rect().width() - 20.0).min(320.0))
        .frame(card())
        .title_bar(false)
        .show(&c, |ui| {
            // 헤더 — 선택 주민 인연 / 마을 전체
            let title = match &selected_id {
                Some(id) => tr(lang, Tr::RelFor).replacen("{}", &name_of(id, frame_agents), 1),
                None => tr(lang, Tr::RelAll).to_owned(),
            };
            super::widgets::title_bar(ui, "緣", &title);
            ui.add_space(2.0);
            if rows.is_empty() && !connected {
                ui.label(egui::RichText::new(tr(lang, Tr::RelWaitLive)).size(10.5).color(theme::TEXT_DIM));
                return;
            }
            if rows.is_empty() {
                ui.label(egui::RichText::new(tr(lang, Tr::RelWaitData)).size(10.5).color(theme::TEXT_DIM));
                return;
            }
            // 정렬·필터 — 선택 주민이면 그 주민 것만, 전체면 최상위만
            rows.sort_by(|a, b| b.warmth.partial_cmp(&a.warmth).unwrap_or(std::cmp::Ordering::Equal));
            let take = if selected_id.is_some() { 8 } else { 12 };
            let shown: Vec<&RelRow> = match &selected_id {
                Some(id) => rows
                    .iter()
                    .filter(|r| &r.a_id == id || &r.b_id == id)
                    .take(take)
                    .collect(),
                None => rows.iter().take(take).collect(),
            };
            egui::ScrollArea::vertical()
                .id_salt("rellist")
                .max_height(340.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    for r in shown {
                        let a = name_of(&r.a_id, frame_agents);
                        let b = name_of(&r.b_id, frame_agents);
                        let warmth = r.warmth.clamp(0.0, 100.0);
                        let col = warmth_hsl(warmth);
                        let resp = egui::Frame::new()
                            .fill(theme::glass_hi())
                            .corner_radius(7.0)
                            .inner_margin(egui::Margin::symmetric(7, 5))
                            .show(ui, |ui| {
                                ui.set_width(ui.available_width());
                                ui.horizontal(|ui| {
                                    ui.label(egui::RichText::new(a.clone()).size(11.0).color(theme::agent_ink(&r.a_id)));
                                    ui.label(egui::RichText::new("↔").size(10.0).color(theme::TEXT_DIM));
                                    ui.label(egui::RichText::new(b.clone()).size(11.0).color(theme::agent_ink(&r.b_id)));
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            ui.label(
                                                egui::RichText::new(format!(
                                                    "{} {:.0}",
                                                    theme::warmth_tier(lang, warmth),
                                                    warmth
                                                ))
                                                .size(9.5)
                                                .color(col),
                                            );
                                        },
                                    );
                                });
                                // 온기 바(hsl) + 대화·거래 수
                                ui.horizontal(|ui| {
                                    let (rect, _) = ui.allocate_exact_size(
                                        egui::vec2((ui.available_width() - 110.0).max(50.0), 6.0),
                                        egui::Sense::hover(),
                                    );
                                    let p = ui.painter();
                                    p.rect_filled(rect, 3.0, theme::track());
                                    let mut fr = rect;
                                    fr.set_width(rect.width() * warmth / 100.0);
                                    p.rect_filled(fr, 3.0, col);
                                    ui.label(
                                        egui::RichText::new(format!("話{} 換{}", r.chats, r.trades))
                                            .size(9.5)
                                            .color(theme::TEXT_DIM),
                                    );
                                });
                            })
                            .response;
                        let tip = tr(lang, Tr::RelChatsTrades)
                            .replacen("{}", &r.chats.to_string(), 1)
                            .replacen("{}", &r.trades.to_string(), 1);
                        resp.on_hover_text(tip);
                    }
                });
        });
}
