//! 관계 패널 — 주민 간 온기·대화·교역(REST /relations, 네이티브 폴러).
//! wasm은 폴러가 없어 안내 문구로 대체(라이브 프레임만 소비하는 플랫폼).

use bevy::prelude::*;
use bevy_egui::egui;

use super::{UiState, ctx};
use crate::agents::Agent;
use crate::net::{NetState, NetStatus, PollQueuesRes, RelRow};

/// id → 이름 매핑(프레임 에이전트 우선, 데모 주민 폴백)
fn name_of(
    id: &str,
    frame_agents: &[crate::net::LiveAgent],
    agents: &Query<&Agent>,
) -> String {
    if let Some(a) = frame_agents.iter().find(|a| a.id == id) {
        return a.name.clone();
    }
    if let Some(a) = agents.iter().find(|a| a.name == id) {
        return a.name.clone();
    }
    id.chars().take(8).collect()
}

#[allow(clippy::type_complexity)] // Bevy 파라미터 — 프레임·주민·큐 동시 참조
pub fn panel(
    mut contexts: bevy_egui::EguiContexts,
    ui_state: Res<UiState>,
    net: Option<Res<NetState>>,
    polls: Res<PollQueuesRes>,
    agents_q: Query<&Agent>,
) {
    if !ui_state.show_relations {
        return;
    }
    let Some(c) = ctx(&mut contexts) else { return };
    let connected = net.as_ref().map(|n| n.status) == Some(NetStatus::Connected);
    let frame_agents: &[crate::net::LiveAgent] = net
        .as_ref()
        .and_then(|n| n.frame.as_ref())
        .map(|f| f.agents.as_slice())
        .unwrap_or(&[]);
    let rows: Vec<RelRow> = polls
        .relations
        .lock()
        .map(|q| q.clone())
        .unwrap_or_default();

    egui::Window::new("🤝 관계")
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-240.0, 34.0))
        .default_width(300.0)
        .show(&c, |ui| {
            if !cfg!(target_arch = "wasm32") && connected && rows.is_empty() {
                ui.weak("(관계 집계 대기 중…)");
            } else if cfg!(target_arch = "wasm32") && rows.is_empty() {
                ui.weak("관계 상세는 네이티브 앱에서 제공(웹은 라이브 관전만)");
            } else if rows.is_empty() {
                ui.weak("(아직 관계 없음)");
            }
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for r in &rows {
                    let a = name_of(&r.a_id, frame_agents, &agents_q);
                    let b = name_of(&r.b_id, frame_agents, &agents_q);
                    ui.label(format!("{} ↔ {}", a, b));
                    ui.horizontal(|ui| {
                        // 온기 게이지 — 0..1 가정(서버 스케일은 표시 시각화로 흡수)
                        let warmth = r.warmth.clamp(0.0, 1.0);
                        let bars = (warmth * 10.0) as usize;
                        ui.label(
                            egui::RichText::new(format!("{} {:.2}", "❤".repeat(bars.max(1)), warmth))
                                .small()
                                .color(egui::Color32::from_rgb(235, 130, 120)),
                        );
                        ui.weak(format!("💬{} 🤝{}", r.chats, r.trades));
                    });
                    ui.separator();
                }
            });
        });
}
