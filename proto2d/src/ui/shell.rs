//! UI 셸 — 상단 바(시계·접속 배지·메뉴 토글)·주민 패널(목록+상세).
//! 레거시 FE chars 패리티: 클릭 선택 → 월드 마커·카메라 추적 연동.

use bevy::prelude::*;
use bevy_egui::egui;

use super::{UiState, clock_text, ctx};
use crate::agents::{Agent, Selected};
use crate::cam::ManualUntil;
use crate::daynight::GameTime;
use crate::net::apply::LiveAgentTag;
use crate::net::{NetState, NetStatus};

pub fn top_bar(
    mut contexts: bevy_egui::EguiContexts,
    time: Res<Time>,
    gt: Res<GameTime>,
    net: Option<Res<NetState>>,
    mut ui_state: ResMut<UiState>,
) {
    let Some(c) = ctx(&mut contexts) else { return };
    egui::Panel::top("topbar").show(&c, |ui| {
        ui.horizontal(|ui| {
            ui.strong("Terrafolk 2D");
            ui.separator();
            ui.label(clock_text(gt.day, gt.t));
            // 접속 배지 — 라이브 ● / 접속중 ◌ / 데모
            let (badge, color) = match net.as_ref().map(|n| n.status) {
                Some(NetStatus::Connected) => ("● LIVE", egui::Color32::from_rgb(90, 220, 120)),
                Some(NetStatus::Connecting) | Some(NetStatus::Reconnecting) => {
                    ("◌ 접속중", egui::Color32::from_rgb(230, 190, 80))
                }
                _ => ("데모", egui::Color32::from_rgb(160, 160, 160)),
            };
            ui.colored_label(color, badge);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.toggle_value(&mut ui_state.show_events, "📜 이벤트");
                ui.toggle_value(&mut ui_state.show_minimap, "🗺 미니맵");
                ui.toggle_value(&mut ui_state.show_chat, "💬 채팅");
                ui.toggle_value(&mut ui_state.show_villagers, "👥 주민");
            });
        });
    });
    let _ = time;
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
    ui_state: Res<UiState>,
) {
    if !ui_state.show_villagers {
        return;
    }
    let Some(c) = ctx(&mut contexts) else { return };
    egui::Panel::right("villagers")
        .default_width(230.0)
        .show(&c, |ui| {
            ui.heading("주민");
            let frame = net.as_ref().and_then(|n| n.frame.as_ref());
            let mut selected_entity: Option<Entity> = None;
            for (e, _a, tag, sel) in &agents_q {
                if sel.is_some() {
                    selected_entity = Some(e);
                }
                let status = frame
                    .and_then(|f| tag.as_ref().and_then(|t| f.agents.iter().find(|la| la.id == t.0)))
                    .map(|la| la.status.clone())
                    .unwrap_or_else(|| {
                        if _a.rest > 0.0 { "idle".into() } else { "moving".into() }
                    });
                let is_sel = sel.is_some();
                let label = format!("{} · {}", _a.name, status);
                if ui
                    .selectable_label(is_sel, label)
                    .clicked()
                {
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
                    ui.label(format!("활동: {act}"));
                }
                if let Some(goal) = la
                    .goal_text
                    .as_deref()
                    .or(la.goal_key.as_deref())
                    .filter(|s| !s.is_empty())
                {
                    ui.label(format!("목표: {goal}"));
                }
                let mut need = |label: &str, v: Option<f32>| {
                    if let Some(v) = v {
                        let v = v.clamp(0.0, 100.0);
                        ui.label(format!("{label} {}", "●".repeat((v / 12.5) as usize)));
                    }
                };
                need("허기", la.hunger);
                need("갈증", la.thirst);
                need("피로", la.fatigue);
                need("체온", la.warmth);
            }
        });
}
