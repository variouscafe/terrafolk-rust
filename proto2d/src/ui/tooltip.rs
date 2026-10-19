//! 자원 툴팁 — 마우스 hovered 자원 위에 상태 정보(stateJson) 표시.
//! bevy 커서 → 월드 좌표 → 근접 라이브 자원 탐색 → egui Area(포인터 위치).

use bevy::prelude::*;
use bevy_egui::egui;

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
    owner_name: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    epitaph: Option<String>,
    #[serde(default)]
    purpose: Option<String>,
    #[serde(default)]
    available: Option<bool>,
}

#[allow(clippy::type_complexity)] // Bevy 파라미터 — 창·카메라·자원 동시 참조
pub fn show(
    mut contexts: bevy_egui::EguiContexts,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    res_q: Query<(&Transform, &LiveResourceTag), Without<Camera2d>>,
    state_cache: bevy::ecs::system::Res<crate::net::NetState>,
) {
    let Some(c) = ctx(&mut contexts) else { return };
    // egui가 마우스를 쓰는 중(패널 hover)이면 스킵 — 월드 툴팁만
    if c.is_pointer_over_area() {
        return;
    }
    let Some(pointer) = c.pointer_hover_pos() else {
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
    let kind_label = match r.kind.as_str() {
        "tree" => "나무",
        "plant" => "식물",
        "gold_vein" => "금맥",
        "animal" => "동물",
        "campfire" => "모닥불",
        "hut" => "건축물",
        "grave" => "무덤",
        "stash" => "저장고",
        "relic" => "유물",
        other => other,
    };
    egui::Area::new(egui::Id::new("res_tooltip"))
        .order(egui::Order::Tooltip)
        .fixed_pos(pointer + egui::vec2(12.0, 12.0))
        .interactable(false)
        .show(&c, |ui| {
            egui::Frame::popup(ui.style()).show(ui, |ui| {
                ui.set_min_width(150.0);
                ui.strong(format!("{kind_label} #{}", r.id));
                if let Some(n) = &st.name {
                    ui.label(format!("이름: {n}"));
                }
                if let Some(o) = &st.owner_name {
                    ui.label(format!("주인: {o}"));
                }
                if let Some(m) = st.maturity {
                    ui.label(format!("성장: {}/3", m.min(3)));
                }
                if let Some(a) = st.available {
                    ui.label(if a { "수확 가능" } else { "재생 중" });
                }
                if let Some(alive) = st.alive {
                    ui.label(if alive { "생명: 살아있음" } else { "생명: 죽음" });
                }
                if let Some(s) = &st.stage {
                    let mut line = format!("공사: {s}");
                    if let Some(t) = &st.bld_type {
                        line.push_str(&format!(" ({t})"));
                    }
                    ui.label(line);
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
