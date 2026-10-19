//! 관계 패널 — 주민 간 온기·대화·교역(REST /relations, 네이티브 폴러).
//! wasm은 폴러가 없어 안내 문구로 대체(라이브 프레임만 소비하는 플랫폼).

use bevy::prelude::*;
use bevy_egui::egui;

use super::i18n::{Tr, tr};
use super::{UiState, ctx};
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

    egui::Window::new(tr(ui_state.lang, Tr::Relations))
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-240.0, 34.0))
        .default_width(300.0)
        .show(&c, |ui| {
            if rows.is_empty() && !connected {
                ui.weak(tr(ui_state.lang, Tr::RelWaitLive));
            } else if rows.is_empty() && cfg!(target_arch = "wasm32") {
                ui.weak(tr(ui_state.lang, Tr::RelNativeOnly));
            } else if rows.is_empty() {
                ui.weak(tr(ui_state.lang, Tr::RelWaitData));
            }
            egui::ScrollArea::vertical().max_height(300.0).show(ui, |ui| {
                for r in &rows {
                    let a = name_of(&r.a_id, frame_agents);
                    let b = name_of(&r.b_id, frame_agents);
                    ui.label(format!("{} ↔ {}", a, b));
                    ui.horizontal(|ui| {
                        // 온기 게이지 — 0..1 가정(서버 스케일은 표시 시각화로 흡수)
                        let warmth = r.warmth.clamp(0.0, 1.0);
                        let bars = (warmth * 10.0) as usize;
                        ui.label(
                            egui::RichText::new(format!("{} {:.2}", "♥".repeat(bars.max(1)), warmth))
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
