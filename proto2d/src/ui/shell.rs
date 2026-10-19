//! UI 셸 — 상단 바(시계·접속 배지·패널 토글)·주민 패널(목록+상세).
//! 레거시 FE chars 패리티: 클릭 선택 → 월드 마커·카메라 추적 연동.

use bevy::prelude::*;
use bevy_egui::egui;

use super::i18n::{Tr, tr, trf, status_label};
use super::{UiState, ctx};
use crate::agents::{Agent, Selected};
use crate::cam::ManualUntil;
use crate::daynight::GameTime;
use crate::net::apply::LiveAgentTag;
use crate::net::{NetState, NetStatus};

pub fn top_bar(
    mut contexts: bevy_egui::EguiContexts,
    gt: Res<GameTime>,
    net: Option<Res<NetState>>,
    mut ui_state: ResMut<UiState>,
) {
    let Some(c) = ctx(&mut contexts) else { return };
    let lang = ui_state.lang;
    egui::Panel::top("topbar").show(&c, |ui| {
        ui.horizontal(|ui| {
            ui.strong("Terrafolk 2D");
            ui.separator();
            let minutes = (gt.t * 24.0 * 60.0) as u32;
            ui.label(format!(
                "{} {:02}:{:02}",
                trf(lang, Tr::DayN, &(gt.day + 1).to_string()),
                minutes / 60,
                minutes % 60
            ));
            // 접속 배지 — 라이브 ● / 접속중 ◌ / 데모
            let (badge, color) = match net.as_ref().map(|n| n.status) {
                Some(NetStatus::Connected) => ("● LIVE", super::theme::LIVE_GREEN),
                Some(NetStatus::Connecting) => (tr(lang, Tr::Connecting), egui::Color32::from_rgb(230, 190, 80)),
                Some(NetStatus::Reconnecting) => (tr(lang, Tr::Reconnecting), egui::Color32::from_rgb(230, 190, 80)),
                _ => (tr(lang, Tr::Demo), egui::Color32::from_rgb(160, 160, 160)),
            };
            ui.colored_label(color, badge);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.toggle_value(&mut ui_state.show_relations, tr(lang, Tr::ToggleRelations));
                ui.toggle_value(&mut ui_state.show_civ, tr(lang, Tr::ToggleCiv));
                ui.toggle_value(&mut ui_state.show_minimap, tr(lang, Tr::ToggleMinimap));
                ui.toggle_value(&mut ui_state.show_events, tr(lang, Tr::ToggleEvents));
                ui.toggle_value(&mut ui_state.show_chat, tr(lang, Tr::ToggleChat));
                ui.toggle_value(&mut ui_state.show_villagers, tr(lang, Tr::ToggleVillagers));
            });
        });
    });
}

/// 주민 패널 — 우측 목록(이름·상태) + 선택 시 상세(활동·목표·욕구).
/// 행 클릭 = 월드 클릭과 동일 선택(Selected 삽입 → 마커·추적 재사용).
#[allow(clippy::type_complexity)] // Bevy 파라미터 — 선택·라이브 프레임·시간 동시 참조
pub fn villagers_panel(
    mut contexts: bevy_egui::EguiContexts,
    time: Res<Time>,
    net: Option<Res<NetState>>,
    mut commands: Commands,
    agents_q: Query<(Entity, &Agent, Option<&LiveAgentTag>, Option<&Selected>)>,
    mut manual: ResMut<ManualUntil>,
    polls: Res<crate::net::PollQueuesRes>,
    ui_state: Res<UiState>,
) {
    if !ui_state.show_villagers {
        return;
    }
    let Some(c) = ctx(&mut contexts) else { return };
    let lang = ui_state.lang;
    egui::Panel::right("villagers")
        .default_width(230.0)
        .show(&c, |ui| {
            ui.heading(tr(lang, Tr::Villagers));
            let frame = net.as_ref().and_then(|n| n.frame.as_ref());
            let mut selected_entity: Option<Entity> = None;
            for (e, _a, tag, sel) in &agents_q {
                if sel.is_some() {
                    selected_entity = Some(e);
                }
                let raw_status = frame
                    .and_then(|f| tag.as_ref().and_then(|t| f.agents.iter().find(|la| la.id == t.0)))
                    .map(|la| la.status.clone())
                    .unwrap_or_else(|| {
                        if _a.rest > 0.0 { "idle".into() } else { "moving".into() }
                    });
                let localized = status_label(lang, &raw_status);
                let status = if localized.is_empty() { raw_status } else { localized.to_owned() };
                let is_sel = sel.is_some();
                let label = format!("{} · {}", _a.name, status);
                if ui.selectable_label(is_sel, label).clicked() {
                    // 선택 토글 — 기존 해제 후 이 엔티티 선택(월드 클릭과 동일 흐름)
                    for (e2, _, _, _) in &agents_q {
                        commands.entity(e2).remove::<Selected>();
                    }
                    if !is_sel {
                        commands.entity(e).insert(Selected);
                        manual.0 = time.elapsed_secs(); // 추적 즉시 재개
                    }
                }
            }
            // 상세 — 선택 주민의 라이브 상태(활동·목표·욕구)
            if let Some(e) = selected_entity
                && let Some((_, _, Some(tag), _)) = agents_q.iter().find(|(e2, _, _, _)| *e2 == e)
                && let Some(f) = frame
                && let Some(la) = f.agents.iter().find(|la| la.id == tag.0)
            {
                ui.separator();
                ui.strong(&la.name);
                if let Some(act) = la.act_type.as_deref().filter(|s| !s.is_empty()) {
                    ui.label(format!("{}: {act}", tr(lang, Tr::Act)));
                }
                if let Some(goal) = la
                    .goal_text
                    .as_deref()
                    .or(la.goal_key.as_deref())
                    .filter(|s| !s.is_empty())
                {
                    ui.label(format!("{}: {goal}", tr(lang, Tr::Goal)));
                }
                let mut need = |label: &str, v: Option<f32>| {
                    if let Some(v) = v {
                        let v = v.clamp(0.0, 100.0);
                        ui.label(format!("{label} {}", "●".repeat((v / 12.5) as usize)));
                    }
                };
                need(tr(lang, Tr::Hunger), la.hunger);
                need(tr(lang, Tr::Thirst), la.thirst);
                need(tr(lang, Tr::Fatigue), la.fatigue);
                need(tr(lang, Tr::Warmth), la.warmth);
                // 생활기록(선택 주민) — 네이티브 폴러, 최근 8행 역순
                let hist = polls.history.lock().map(|q| q.clone()).unwrap_or_default();
                if !hist.is_empty() {
                    ui.separator();
                    ui.strong(tr(lang, Tr::History));
                    for h in hist.iter().rev().take(8) {
                        ui.small(format!("[{}] {}", h.type_, h.text));
                    }
                }
            }
        });
}
