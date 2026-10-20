//! 채팅 패널 — 하단 로그 + 송신(선택 주민 대화).
//! 레거시 FE ChatBar 직역: 시각 스탬프(gmin→D{n} HH:MM), 화자색(사람=골드,
//! 주민=주민색), 선택 해제(X). 송신 경로: 네이티브=native_conn(룸 핸들),
//! 웹=terrafolk.ui.sendChat.

use bevy::prelude::*;
use bevy_egui::egui;

use super::i18n::{Tr, tr};
use super::theme;
use super::{ChatLog, UiState, ctx, fmt_hhmm};
use crate::agents::Agent;
use crate::net::apply::LiveAgentTag;
use crate::net::{NetState, NetStatus};

/// 채팅 패널 — 하단 로그 + 입력(선택 주민에게 말걸기). 전송 중임을 표시.
#[allow(clippy::type_complexity)] // Bevy 파라미터 — 로그·선택·네트 동시 참조
pub fn panel(
    mut contexts: bevy_egui::EguiContexts,
    ui_state: Res<UiState>,
    net: Option<Res<NetState>>,
    log: Res<ChatLog>,
    agents_q: Query<(&Agent, &LiveAgentTag), With<crate::agents::Selected>>,
    mut commands: Commands,
    mut draft: Local<String>,
) {
    if !ui_state.show_chat {
        return;
    }
    let Some(c) = ctx(&mut contexts) else { return };
    let connected = net.as_ref().map(|n| n.status) == Some(NetStatus::Connected);
    let frame_agents = net.as_ref().and_then(|n| n.frame.as_ref()).map(|f| f.agents.as_slice());
    // 모바일 — 컴팩트 바(레거시 ChatBar 직역: 로그 2-3행 + 입력행, 헤더 생략)
    let narrow = c.screen_rect().width() < 760.0;
    let (panel_h, log_h) = if narrow { (94.0, 38.0) } else { (148.0, 78.0) };
    egui::Panel::bottom("chat")
        .exact_height(panel_h)
        .frame(super::widgets::dock())
        .show(&c, |ui| {
            ui.add_space(3.0);
            if !narrow {
                super::widgets::title_bar(ui, "話", tr(ui_state.lang, Tr::Chat));
            }
            egui::ScrollArea::vertical()
                .max_height(log_h)
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if log.rows.is_empty() {
                        ui.label(egui::RichText::new(tr(ui_state.lang, Tr::ChatWaiting)).size(10.5).color(theme::TEXT_DIM));
                    }
                    for r in &log.rows {
                        // 화자 판정 — 사람(관전자 발언)은 speaker가 주민명과 다르거나 @ 포함
                        let agent_name = frame_agents
                            .and_then(|a| a.iter().find(|la| la.id == r.agent_id))
                            .map(|la| la.name.clone());
                        // 레거시 isHumanRow — 주민 목록에 있는 화자와 이름이 다를 때만 사람
                        let is_human = r.speaker.contains('@')
                            || agent_name.as_deref().map(|n| n != r.speaker.as_str()).unwrap_or(false);
                        let stamp = r
                            .gmin
                            .map(super::villagers::fmt_gmin)
                            .or_else(|| r.ts.as_deref().map(|t| fmt_hhmm(Some(t))))
                            .unwrap_or_default();
                        ui.horizontal(|ui| {
                            if !stamp.is_empty() {
                                ui.label(egui::RichText::new(&stamp).size(9.0).color(theme::TEXT_DIM));
                            }
                            ui.label(
                                egui::RichText::new(format!("{}:", r.speaker))
                                    .size(10.5)
                                    .strong()
                                    .color(if is_human {
                                        theme::GOLD_DEEP
                                    } else {
                                        theme::agent_ink(&r.agent_id)
                                    }),
                            );
                            ui.label(
                                egui::RichText::new(&r.text)
                                    .size(10.5)
                                    .color(theme::TEXT_WARM),
                            );
                        });
                    }
                });
            ui.add_space(2.0);
            ui.horizontal(|ui| {
                let target = agents_q.iter().next().map(|(a, _)| a.name.clone());
                let enabled = connected && target.is_some();
                let hint = match &target {
                    Some(n) => tr(ui_state.lang, Tr::ChatHintMine).replace("{}", n),
                    None => tr(ui_state.lang, Tr::ChatHint).to_owned(),
                };
                // Enter 송신은 입력창 포커스 기준으로만 — 전역 키로 걸면
                // 다른 버튼·맵 조작의 Enter가 초안을 몰래 전송한다
                // (2026-10-20 리뷰). singleline Enter는 포커스를 반납하므로
                // lost_focus로 판정하고, 송신 후 다시 포커스를 돌려준다.
                let edit = ui.add_enabled(
                    enabled,
                    egui::TextEdit::singleline(&mut *draft)
                        .hint_text(hint)
                        .desired_width(ui.available_width() - 130.0),
                );
                let enter_in_edit =
                    edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if ui
                    .add_enabled(enabled, egui::Button::new(tr(ui_state.lang, Tr::ChatSend)).small())
                    .clicked()
                    || (enabled && enter_in_edit)
                {
                    let text = draft.trim().to_owned();
                    if !text.is_empty()
                        && let Some((_, tag)) = agents_q.iter().next()
                    {
                        send_chat(&tag.0, &text);
                        *draft = String::new();
                        edit.request_focus();
                    }
                }
                if target.is_some()
                    && ui
                        .button(egui::RichText::new("×").size(10.0).color(theme::TEXT_DIM))
                        .on_hover_text(tr(ui_state.lang, Tr::Deselect))
                        .clicked()
                {
                    deselect_all(&mut commands);
                }
            });
        });
}

/// 선택 전면 해제 — agents 쿼리를 직접 못 얻는 자리에서 Commands로
fn deselect_all(commands: &mut Commands) {
    commands.queue(|world: &mut World| {
        let selected: Vec<Entity> = world
            .query_filtered::<Entity, With<crate::agents::Selected>>()
            .iter(world)
            .collect();
        for e in selected {
            world.entity_mut(e).remove::<crate::agents::Selected>();
        }
    });
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
