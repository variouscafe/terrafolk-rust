//! 채팅 패널 — 관전 채팅 로그 + 송신(선택 주민 대화)·이벤트 로그 패널.
//! 송신 경로: 네이티브=native_conn::send_chat(룸 핸들), 웹=terrafolk.ui.sendChat.

use bevy::prelude::*;
use bevy_egui::egui;

use super::{ChatLog, EventLog, UiState, ctx};
use crate::agents::Agent;
use crate::net::apply::LiveAgentTag;
use crate::net::NetStatus;

/// 채팅 패널 — 하단 로그 + 입력(선택 주민에게 말걸기). 전송 중임을 표시.
#[allow(clippy::type_complexity)] // Bevy 파라미터 — 로그·선택·네트 동시 참조
pub fn panel(
    mut contexts: bevy_egui::EguiContexts,
    time: Res<Time>,
    ui_state: Res<UiState>,
    net: Option<Res<crate::net::NetState>>,
    log: ResMut<ChatLog>,
    agents_q: Query<(&Agent, &LiveAgentTag), With<crate::agents::Selected>>,
    mut draft: Local<String>,
) {
    if !ui_state.show_chat {
        return;
    }
    let Some(c) = ctx(&mut contexts) else { return };
    let connected = net.as_ref().map(|n| n.status) == Some(NetStatus::Connected);
    egui::Panel::bottom("chat")
        .default_size(150.0)
        .show(&c, |ui| {
            ui.heading("💬 채팅");
            egui::ScrollArea::vertical()
                .max_height(90.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    if log.rows.is_empty() {
                        ui.weak("(대기 중… 주민들이 대화하면 여기에 표시됩니다)");
                    }
                    for r in &log.rows {
                        ui.label(
                            egui::RichText::new(format!("{}: {}", r.speaker, r.text))
                                .color(egui::Color32::from_rgb(225, 220, 200)),
                        );
                    }
                });
            ui.horizontal(|ui| {
                let target = agents_q.iter().next().map(|(a, _)| a.name.clone());
                let enabled = connected && target.is_some();
                ui.add_enabled(
                    enabled,
                    egui::TextEdit::singleline(&mut *draft)
                        .hint_text("선택한 주민에게 말걸기…")
                        .desired_width(ui.available_width() - 70.0),
                );
                if ui
                    .add_enabled(enabled, egui::Button::new("보내기"))
                    .clicked()
                    || (enabled && ui.input(|i| i.key_pressed(egui::Key::Enter)))
                {
                    let text = draft.trim().to_owned();
                    if !text.is_empty()
                        && let Some((_, tag)) = agents_q.iter().next()
                    {
                        send_chat(&tag.0, &text);
                        *draft = String::new();
                    }
                }
            });
        });
    let _ = time;
}

/// 채팅 송신 — 플랫폼 분기(네이티브 룸 핸들 / 웹 worldsim 브리지)
fn send_chat(agent_id: &str, text: &str) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        crate::net::native_conn::send_chat(agent_id, text);
    }
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        if let Some(window) = web_sys::window()
            && let Ok(tf) = js_sys::Reflect::get(&window, &"terrafolk".into())
            && let Ok(ui) = js_sys::Reflect::get(&tf, &"ui".into())
            && let Ok(f) = js_sys::Reflect::get(&ui, &"sendChat".into())
            && let Ok(f) = f.dyn_into::<js_sys::Function>()
        {
            let _ = f.call3(
                &ui,
                &wasm_bindgen::JsValue::from_str(agent_id),
                &wasm_bindgen::JsValue::from_str(text),
                &wasm_bindgen::JsValue::from_str("ko"),
            );
        }
    }
}

/// 이벤트 로그 패널 — 관전 타임라인(생성·채집·관계 등 최근 이벤트 문자열)
pub fn events_panel(
    mut contexts: bevy_egui::EguiContexts,
    ui_state: Res<UiState>,
    log: ResMut<EventLog>,
) {
    if !ui_state.show_events {
        return;
    }
    let Some(c) = ctx(&mut contexts) else { return };
    egui::Window::new("📜 이벤트")
        .default_width(340.0)
        .show(&c, |ui| {
            if log.rows.is_empty() {
                ui.weak("(이벤트 없음)");
            }
            egui::ScrollArea::vertical()
                .max_height(260.0)
                .stick_to_bottom(true)
                .show(ui, |ui| {
                    for r in &log.rows {
                        ui.label(
                            egui::RichText::new(r).color(egui::Color32::from_rgb(200, 210, 190)),
                        );
                    }
                });
        });
}
