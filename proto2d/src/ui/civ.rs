//! 문명 패널 — 발명 레지스트리(inventionsJson) 목록·요약. 레거시 FE 발명
//! 팝업 패리티(원문 변화 시만 재파싱 — root ui/civ.rs 캐시 패턴).

use bevy::prelude::*;
use bevy_egui::egui;

use super::{UiState, ctx};
use crate::net::NetState;

/// inventionsJson 행 — docs §2.4(서버 serializeInventions)
#[derive(serde::Deserialize, Clone)]
#[allow(dead_code)] // 표시에 골라 쓴다 — 필드는 계약 정본 그대로
pub(crate) struct InventionRow {
    name: String,
    #[serde(default)]
    purpose: Option<String>,
    #[serde(default)]
    inventor: Option<String>,
    #[serde(default)]
    based_on: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    day: Option<u32>,
}

pub fn panel(
    mut contexts: bevy_egui::EguiContexts,
    ui_state: Res<UiState>,
    net: Option<Res<NetState>>,
    mut cache: Local<Option<(String, Vec<InventionRow>)>>,
) {
    if !ui_state.show_civ {
        return;
    }
    let Some(c) = ctx(&mut contexts) else { return };
    // 원문 파싱 캐시 — 프레임마다 와도 원문 같으면 재파싱 없음
    let raw = net
        .as_ref()
        .and_then(|n| n.frame.as_ref())
        .and_then(|f| f.inventions.as_deref())
        .map(str::to_owned);
    let changed = cache.as_ref().map(|(s, _)| s != raw.as_deref().unwrap_or(""));
    if changed != Some(false) {
        *cache = raw.map(|s| {
            let rows = serde_json::from_str::<Vec<InventionRow>>(&s).unwrap_or_default();
            (s, rows)
        });
    }
    let rows: &[InventionRow] = cache.as_ref().map(|(_, r)| r.as_slice()).unwrap_or(&[]);

    egui::Window::new("🏛 문명")
        .anchor(egui::Align2::RIGHT_TOP, egui::vec2(-240.0, 34.0))
        .default_width(300.0)
        .show(&c, |ui| {
            ui.label(format!("발명 {}건", rows.len()));
            ui.separator();
            if rows.is_empty() {
                ui.weak("(아직 발명이 없다 — 주민들이 연구 중…)");
            }
            egui::ScrollArea::vertical().max_height(320.0).show(ui, |ui| {
                for r in rows.iter().rev() {
                    ui.strong(&r.name);
                    let mut meta = Vec::new();
                    if let Some(d) = r.day {
                        meta.push(format!("{d}일차"));
                    }
                    if let Some(inv) = &r.inventor {
                        meta.push(format!("by {inv}"));
                    }
                    if let Some(cat) = &r.category {
                        meta.push(cat.clone());
                    }
                    if !meta.is_empty() {
                        ui.weak(meta.join(" · "));
                    }
                    if let Some(p) = &r.purpose {
                        ui.small(p).on_hover_text(p);
                    }
                    ui.separator();
                }
            });
        });
}
