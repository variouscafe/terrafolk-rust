//! 자원 툴팁 — 마우스 hovered 자원 위에 상태 정보(stateJson) 표시.
//! bevy 커서 → 월드 좌표 → 근접 라이브 자원 탐색 → egui Area(포인터 위치).

use bevy::prelude::*;
use bevy_egui::egui;

use super::i18n::{Tr, tr, trf};
use super::theme;
use super::vocab::Vocab;
use super::ctx;
use crate::net::live_props::LiveResourceTag;

/// 자원 stateJson 표시 필드(렌더용 최소)
#[derive(Default, serde::Deserialize)]
struct TipState {
    #[serde(default)]
    maturity: Option<u8>,
    #[serde(default)]
    alive: Option<bool>,
    #[serde(default)]
    stage: Option<String>,
    #[serde(default, rename = "type")]
    bld_type: Option<String>,
    #[serde(default)]
    by: Option<String>,
    #[serde(default)]
    progress: Option<f32>,
    #[serde(default)]
    lit: Option<bool>,
    #[serde(default)]
    owner_name: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    age: Option<u32>,
    #[serde(default)]
    epitaph: Option<String>,
    #[serde(default)]
    purpose: Option<String>,
    #[serde(default)]
    available: Option<bool>,
}

#[allow(clippy::type_complexity)] // Bevy 파라미터 — 창·카메라·자원·사전 동시 참조
pub fn show(
    mut contexts: bevy_egui::EguiContexts,
    ui_state: bevy::ecs::system::Res<crate::ui::UiState>,
    vocab: bevy::ecs::system::Res<Vocab>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    res_q: Query<(&Transform, &LiveResourceTag), Without<Camera2d>>,
    state_cache: bevy::ecs::system::Res<crate::net::NetState>,
) {
    let Some(c) = ctx(&mut contexts) else { return };
    let Some(pointer) = c.pointer_hover_pos() else {
        return;
    };
    // egui 계층 위(패널·창)면 스킵 — 월드 툴팁만(배경 계층=월드)
    if c
        .layer_id_at(pointer)
        .is_some_and(|l| l.order != egui::Order::Background)
    {
        return;
    };
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Ok((camera, cam_tf)) = camera_q.single() else { return };
    let Ok(world) = camera.viewport_to_world_2d(cam_tf, cursor) else { return };
    // 가장 가까운 자원(반경 90 월드 이내)
    let mut best: Option<(f32, &Transform, &LiveResourceTag)> = None;
    for (tr, tag) in &res_q {
        let d = tr.translation.truncate().distance(world);
        if d < 90.0 && best.as_ref().map(|(bd, _, _)| d < *bd).unwrap_or(true) {
            best = Some((d, tr, tag));
        }
    }
    let Some((_, _, tag)) = best else { return };
    // stateJson은 프레임 resources에서 해당 id 조회
    let Some(frame) = state_cache.frame.as_ref() else { return };
    let Some(r) = frame.resources.iter().find(|r| r.id == tag.0) else {
        return;
    };
    let st: TipState = r
        .state
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default();
    let lang = ui_state.lang;
    let kind_label = match r.kind.as_str() {
        "tree" => tr(lang, Tr::TipTree),
        "plant" => tr(lang, Tr::TipPlant),
        "gold_vein" => tr(lang, Tr::TipGoldVein),
        "animal" => tr(lang, Tr::TipAnimal),
        "campfire" => tr(lang, Tr::TipCampfire),
        "hut" => tr(lang, Tr::TipHut),
        "grave" => tr(lang, Tr::TipGrave),
        "stash" => tr(lang, Tr::TipStash),
        "relic" => tr(lang, Tr::TipRelic),
        other => other,
    };
    let icon = super::i18n::ev_kind_icon(match r.kind.as_str() {
        "tree" | "plant" => "resource",
        "animal" => "spawn",
        "grave" => "death",
        "relic" => "invention",
        _ => "result",
    });
    egui::Area::new(egui::Id::new("res_tooltip"))
        .order(egui::Order::Tooltip)
        .fixed_pos(pointer + egui::vec2(12.0, 12.0))
        .interactable(false)
        .show(&c, |ui| {
            super::widgets::card().show(ui, |ui| {
                ui.set_min_width(150.0);
                ui.horizontal(|ui| {
                    ui.label(
                        bevy_egui::egui::RichText::new(icon)
                            .size(11.0)
                            .color(theme::ACCENT_SAND),
                    );
                    ui.strong(format!("{kind_label} #{}", r.id));
                });
                // 종(species) — vocab 라벨
                if !r.species.is_empty() {
                    ui.label(bevy_egui::egui::RichText::new(vocab.sp(lang, &r.species)).size(10.5));
                }
                if let Some(n) = &st.name {
                    ui.label(trf(lang, Tr::TipName, n));
                }
                if let Some(by) = st.by.as_deref().filter(|s| !s.is_empty()) {
                    ui.label(trf(lang, Tr::TipOwner, by));
                }
                if let Some(o) = &st.owner_name {
                    ui.label(trf(lang, Tr::TipOwner, o));
                }
                if let Some(m) = st.maturity {
                    ui.label(trf(lang, Tr::TipMaturity, &m.min(3).to_string()));
                }
                if let Some(a) = st.available {
                    ui.label(if a { tr(lang, Tr::TipHarvest) } else { tr(lang, Tr::TipRegrow) });
                }
                if let Some(alive) = st.alive {
                    ui.label(if alive { tr(lang, Tr::TipAlive) } else { tr(lang, Tr::TipDead) });
                }
                if let Some(lit) = st.lit {
                    let mark = if lit { "火" } else { "·" };
                    ui.label(bevy_egui::egui::RichText::new(format!(
                        "{mark} {}",
                        if lit { tr(lang, Tr::TipHarvest) } else { tr(lang, Tr::TipRegrow) }
                    ))
                    .size(10.5));
                }
                // 건축물 — 종 라벨 + 단계(vocab bldstage) + 진행률
                if let Some(s) = &st.stage {
                    let bld = st.bld_type.as_deref().unwrap_or("HUT");
                    let stage_lbl = vocab.bldstage(lang, bld, s);
                    let pct = st.progress.map(|p| format!(" {}%", (p * 100.0) as u32)).unwrap_or_default();
                    let line = trf(lang, Tr::TipBuild, stage_lbl) + &pct;
                    ui.label(line);
                    ui.label(bevy_egui::egui::RichText::new(vocab.building(lang, bld)).size(10.0));
                }
                if let Some(a) = st.age {
                    ui.label(trf(lang, Tr::TipGraveAge, &a.to_string()));
                }
                if let Some(e) = &st.epitaph {
                    ui.weak(format!("“{e}”"));
                }
                if let Some(p) = &st.purpose {
                    ui.weak(p);
                }
            });
        });
}
