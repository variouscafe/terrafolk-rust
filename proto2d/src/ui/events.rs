//! 기록 패널 — 관전 타임라인(구조화 이벤트)을 탭으로 분류.
//! 레거시 FE LogFeed 직역: 전체/하이라이트/사회·인연/생활·교역/지식·기술/
//! 생애/세계 — 각 탭 행 수 표시, 최신이 위, 아이콘+주민명+문장(vocab 렌더).

use bevy::prelude::*;
use bevy_egui::egui;

use super::i18n::{Lang, Tr, ev_cat_of, ev_color, ev_kind_icon, tr};
use super::theme;
use super::vocab::Vocab;
use super::widgets::card;
use super::{EventLog, EventRow, UiState, ctx, fmt_hhmm, is_highlight};
use crate::net::NetState;

/// 탭 정의 — (내부 키, 라벨 Tr)
const TABS: [(&str, Tr); 7] = [
    ("all", Tr::EvTabAll),
    ("highlight", Tr::EvTabHighlight),
    ("social", Tr::EvTabSocial),
    ("living", Tr::EvTabLiving),
    ("knowledge", Tr::EvTabKnowledge),
    ("life", Tr::EvTabLife),
    ("world", Tr::EvTabWorld),
];

pub fn panel(
    mut contexts: bevy_egui::EguiContexts,
    ui_state: Res<UiState>,
    log: Res<EventLog>,
    vocab: Res<Vocab>,
    net: Option<Res<NetState>>,
    mut tab: Local<&'static str>,
) {
    if !ui_state.show_events {
        return;
    }
    if TABS.iter().all(|(k, _)| *k != *tab) {
        *tab = "all";
    }
    let Some(c) = ctx(&mut contexts) else { return };
    let lang = ui_state.lang;
    let name_of = |id: &str| -> Option<String> {
        if id.is_empty() {
            return None;
        }
        net.as_ref()
            .and_then(|n| n.frame.as_ref())
            .and_then(|f| f.agents.iter().find(|a| a.id == id))
            .map(|a| a.name.clone())
            .or_else(|| Some(id.chars().take(8).collect()))
    };
    egui::Window::new(tr(lang, Tr::Events))
        .anchor(egui::Align2::LEFT_TOP, egui::vec2(10.0, 242.0))
        .default_width((c.screen_rect().width() - 20.0).min(360.0))
        .frame(card())
        .title_bar(false)
        .show(&c, |ui| {
            super::widgets::title_bar(ui, "史", tr(lang, Tr::Events));
            ui.add_space(2.0);
            // 탭 행 — 카운트 포함
            ui.horizontal_wrapped(|ui| {
                for (key, label) in TABS {
                    let n = match key {
                        "all" => log.rows.len(),
                        "highlight" => log.rows.iter().filter(|e| is_highlight(&e.kind, &e.payload)).count(),
                        cat => log.rows.iter().filter(|e| ev_cat_of(&e.kind) == cat).count(),
                    };
                    let active = *tab == key;
                    let text = egui::RichText::new(format!("{} {n}", tr(lang, label)))
                        .size(9.5)
                        .color(if active { theme::INK } else { theme::INK_DIM })
                        .strong();
                    let btn = egui::Button::new(text)
                        .fill(if active {
                            theme::sel_pill()
                        } else {
                            theme::glass_hi()
                        })
                        .stroke(if active {
                            egui::Stroke::new(1.0_f32, theme::GOLD)
                        } else {
                            egui::Stroke::new(1.0_f32, theme::STROKE_OLIVE)
                        })
                        .corner_radius(999.0);
                    if ui.add(btn).clicked() {
                        *tab = key;
                    }
                }
            });
            ui.add_space(2.0);
            if log.rows.is_empty() {
                ui.label(egui::RichText::new(tr(lang, Tr::EventsEmpty)).size(10.5).color(theme::TEXT_DIM));
                return;
            }
            egui::ScrollArea::vertical()
                .id_salt("evlist")
                .max_height(320.0)
                .stick_to_bottom(false)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    // 최신이 위(레거시 reverse)
                    for e in log.rows.iter().rev() {
                        if !row_in_tab(*tab, e) {
                            continue;
                        }
                        row_ui(ui, lang, &vocab, e, &name_of);
                    }
                });
        });
}

fn row_in_tab(tab: &str, e: &EventRow) -> bool {
    match tab {
        "all" => true,
        "highlight" => is_highlight(&e.kind, &e.payload),
        cat => ev_cat_of(&e.kind) == cat,
    }
}

/// 이벤트 행 — 아이콘(한자·종색) + 주민명(강조) + 문장 + 시각
fn row_ui(
    ui: &mut egui::Ui,
    lang: Lang,
    vocab: &Vocab,
    e: &EventRow,
    name_of: &dyn Fn(&str) -> Option<String>,
) {
    // 문장 없는 행(동기 resource 등)은 스킵 — 레거시와 동일 규칙
    let text = vocab.event_text(lang, &e.payload);
    if text.is_empty() {
        return;
    }
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(ev_kind_icon(&e.kind))
                .size(11.0)
                .color(ev_color(&e.kind)),
        );
        ui.vertical(|ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                if let Some(n) = name_of(&e.agent_id) {
                    ui.label(
                        egui::RichText::new(n)
                            .size(10.0)
                            .strong()
                            .color(theme::TEXT_WARM),
                    );
                }
                ui.label(
                    egui::RichText::new(&text)
                        .size(10.0)
                        .color(ev_color(&e.kind)),
                );
            });
        });
    })
    .response
    .on_hover_text(fmt_hhmm(e.ts.as_deref()));
}
