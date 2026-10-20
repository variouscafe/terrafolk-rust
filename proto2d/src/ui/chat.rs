//! 채팅 — 하단 도킹 바(전체 로그) + 선택 주민 대화창(컴포저).
//! 캐릭터 클릭(월드·목록) → 중앙 하단 대화창 카드: 헤더(아바타 칩·이름·
//! 상태 칩·✕) + 이 주민과의 로그 + 상용급 컴포저. 레거시 FE ChatBar 직역
//! (패널이 입력 바 위에 뜨는 구조) + 코지 글래스 크롬(theme v4).
//! 시각 스탬프(gmin→D{n} HH:MM), 화자색(사람=골드, 주민=주민색).

use bevy::prelude::*;
use bevy_egui::egui;

use super::i18n::{Tr, status_color, status_label, tr};
use super::theme;
use super::widgets;
use super::{ChatLog, ChatLogRow, UiState, ctx, fmt_hhmm};
use crate::agents::Agent;
use crate::net::apply::LiveAgentTag;
use crate::net::{LiveAgent, NetState, NetStatus};

/// 하단 도킹 바 — 전체 로그 뷰어. 입력은 선택 시 뜨는 대화창 컴포저가
/// 담당(레거시 FE처럼 패널이 바 위) — 바의 컴포저는 항상 비활성 안내.
#[allow(clippy::type_complexity)] // Bevy 파라미터 — 로그·네트 동시 참조
pub fn panel(
    mut contexts: bevy_egui::EguiContexts,
    ui_state: Res<UiState>,
    log: Res<ChatLog>,
    net: Option<Res<NetState>>,
    mut draft: Local<String>,
    mut focus: Local<bool>,
) {
    if !ui_state.show_chat {
        return;
    }
    let Some(c) = ctx(&mut contexts) else { return };
    let agents = live_agents(&net);
    // 모바일 — 컴팩트 바(레거시 ChatBar 직역: 로그 2-3행 + 입력행, 헤더 생략)
    let narrow = c.screen_rect().width() < 760.0;
    let (panel_h, log_h) = if narrow { (94.0, 38.0) } else { (148.0, 78.0) };
    egui::Panel::bottom("chat")
        .exact_height(panel_h)
        .frame(widgets::dock())
        .show(&c, |ui| {
            ui.add_space(3.0);
            if !narrow {
                widgets::title_bar(ui, "話", tr(ui_state.lang, Tr::Chat));
            }
            egui::ScrollArea::vertical()
                .max_height(log_h)
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if log.rows.is_empty() {
                        ui.label(
                            egui::RichText::new(tr(ui_state.lang, Tr::ChatWaiting))
                                .size(10.5)
                                .color(theme::TEXT_DIM),
                        );
                    }
                    for r in &log.rows {
                        log_row(ui, r, agents, 10.5);
                    }
                });
            ui.add_space(2.0);
            // 비활성 컴포저 — "주민을 선택하면 대화" 안내(입력은 대화창)
            composer(
                ui,
                &mut draft,
                &mut focus,
                false,
                tr(ui_state.lang, Tr::ChatHint),
                None,
                |_| {},
            );
        });
}

/// 대화창 — 선택 주민과의 1:1 카드(중앙 하단, 레거시 ChatBar 위치).
/// 도킹 바가 보일 땐 그 위에 뜬다(한 번에 한 대화 — 선택은 단일).
#[allow(clippy::type_complexity, clippy::too_many_arguments)] // Bevy 파라미터 — 선택·로그·네트 동시 참조
pub fn conversation(
    mut contexts: bevy_egui::EguiContexts,
    ui_state: Res<UiState>,
    net: Option<Res<NetState>>,
    log: Res<ChatLog>,
    agents_q: Query<(&Agent, Option<&LiveAgentTag>), With<crate::agents::Selected>>,
    mut commands: Commands,
    mut draft: Local<String>,
    mut focus: Local<bool>,
) {
    // 선택 없으면(또는 해제 순간) 대화창도 없다 — 패널이 사라진다
    let Some((agent, tag)) = agents_q.iter().next() else {
        return;
    };
    let Some(c) = ctx(&mut contexts) else { return };
    let lang = ui_state.lang;
    let agents = live_agents(&net);
    let connected = net.as_ref().map(|n| n.status) == Some(NetStatus::Connected);
    let id = tag.map(|t| t.0.as_str()).unwrap_or("");
    let la = agents.iter().find(|a| a.id == id);
    let color = if id.is_empty() {
        theme::INK_DIM
    } else {
        theme::agent_color(id)
    };
    let narrow = c.screen_rect().width() < 760.0;
    // 남은 맵 영역의 중앙 하단에 배치 — 도킹 크기는 알려진 상수로 계산
    // (available_rect는 멀티패스 프레임에서 이전 패스 패널 할당이 애매해
    // 실측보다 결정론이 안전하다).
    let screen = c.screen_rect();
    let right = if ui_state.show_villagers && !narrow {
        272.0
    } else {
        0.0
    };
    let bottom = if ui_state.show_chat {
        if narrow { 94.0 } else { 148.0 }
    } else {
        0.0
    };
    let free_w = screen.width() - right;
    let off = egui::vec2(free_w * 0.5 - screen.center().x, -(bottom + 10.0));
    let width = (free_w - 16.0).min(560.0);
    // 레이어 주의 — 원시 Area+Order::Middle은 패널 층과 입력 라우팅이
    // 꼬인다(클릭이 카드 위에서 유실, 2026-10-20 실측). 메뉴와 같은
    // 앵커 Window 패턴이 이 앱에서 검증된 클릭 경로다.
    egui::Window::new("chat_conv")
        .anchor(egui::Align2::CENTER_BOTTOM, off)
        .default_width(width)
        .frame(widgets::card())
        .title_bar(false)
        .show(&c, |ui| {
            ui.set_min_width(width);
            ui.set_width(width);
            // ── 헤더 — 아바타 칩 + 이름 + 상태 칩 + ✕ ──
            ui.horizontal(|ui| {
                let initial = agent
                    .name
                    .chars()
                    .next()
                    .map(String::from)
                    .unwrap_or_default();
                widgets::avatar_chip(ui, &initial, color, 26.0);
                ui.label(
                    egui::RichText::new(&agent.name)
                        .size(13.5)
                        .strong()
                        .color(theme::TEXT_WARM),
                );
                if let Some(la) = la {
                    let lbl = status_label(lang, &la.status);
                    let st = if lbl.is_empty() {
                        la.status.as_str()
                    } else {
                        lbl
                    };
                    widgets::chip(
                        ui,
                        st,
                        status_color(&la.status),
                        egui::Color32::from_rgba_unmultiplied(16, 22, 15, 190),
                    );
                    if la.kind == "user" {
                        widgets::chip(
                            ui,
                            tr(lang, Tr::Participating),
                            theme::ACCENT_SAND,
                            egui::Color32::from_rgb(52, 44, 20),
                        );
                    }
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(egui::RichText::new("×").size(12.0).color(theme::TEXT_DIM))
                        .on_hover_text(tr(lang, Tr::Deselect))
                        .clicked()
                    {
                        deselect_all(&mut commands);
                    }
                });
            });
            ui.add_space(3.0);
            // ── 이 주민과의 로그 — 시간순, 하단 고정 ──
            let rows: Vec<&ChatLogRow> = log.rows.iter().filter(|r| r.agent_id == id).collect();
            egui::ScrollArea::vertical()
                .id_salt("conv_log")
                .max_height(if narrow { 130.0 } else { 190.0 })
                .stick_to_bottom(true)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    if rows.is_empty() {
                        ui.label(
                            egui::RichText::new(tr(lang, Tr::ChatEmpty))
                                .size(10.5)
                                .color(theme::TEXT_DIM),
                        );
                    }
                    for r in rows {
                        log_row(ui, r, agents, 11.0);
                    }
                });
            ui.add_space(4.0);
            // ── 컴포저 — 라이브 접속 + 라이브 주민일 때만 활성 ──
            let enabled = connected && !id.is_empty();
            let hint = if !connected {
                tr(lang, Tr::ChatOffline).to_owned()
            } else if la.map(|l| l.kind == "user").unwrap_or(false) {
                tr(lang, Tr::ChatHintMine).replace("{}", &agent.name)
            } else {
                tr(lang, Tr::ChatTalk).replace("{}", &agent.name)
            };
            let initial = agent
                .name
                .chars()
                .next()
                .map(String::from)
                .unwrap_or_default();
            let send_id = id.to_owned();
            composer(
                ui,
                &mut draft,
                &mut focus,
                enabled,
                &hint,
                Some((initial.as_str(), color)),
                move |text| send_chat(&send_id, text),
            );
        });
}

/// 공용 로그 행 — 스탬프 + 화자(사람=골드/주민=주민색) + 본문.
fn log_row(ui: &mut egui::Ui, r: &ChatLogRow, agents: &[LiveAgent], size: f32) {
    // 레거시 isHumanRow — 주민 목록에 있는 화자와 이름이 다를 때만 사람
    let agent_name = agents
        .iter()
        .find(|la| la.id == r.agent_id)
        .map(|la| la.name.as_str());
    let is_human =
        r.speaker.contains('@') || agent_name.map(|n| n != r.speaker.as_str()).unwrap_or(false);
    let stamp = r
        .gmin
        .map(super::villagers::fmt_gmin)
        .or_else(|| r.ts.as_deref().map(|t| fmt_hhmm(Some(t))))
        .unwrap_or_default();
    ui.horizontal(|ui| {
        if !stamp.is_empty() {
            ui.label(
                egui::RichText::new(&stamp)
                    .size(size - 1.5)
                    .color(theme::TEXT_DIM),
            );
        }
        ui.label(
            egui::RichText::new(format!("{}:", r.speaker))
                .size(size)
                .strong()
                .color(if is_human {
                    theme::GOLD_DEEP
                } else {
                    theme::agent_ink(&r.agent_id)
                }),
        );
        ui.label(
            egui::RichText::new(&r.text)
                .size(size)
                .color(theme::TEXT_WARM),
        );
    });
}

/// 상용급 컴포저 — 인셋 필 입력란(포커스 골드 글로우) + 대상 미니 아바타 +
/// 원형 골드 송신 버튼(베벨 돔). Enter 송신은 입력창 포커스 기준으로만 —
/// 전역 키로 걸면 다른 버튼·맵 조작의 Enter가 초안을 몰래 전송한다
/// (2026-10-20 리뷰). singleline Enter는 포커스를 반납하므로 lost_focus로
/// 판정하고, 송신 후 다시 포커스를 돌려준다.
fn composer(
    ui: &mut egui::Ui,
    draft: &mut String,
    focus: &mut bool,
    enabled: bool,
    hint: &str,
    avatar: Option<(&str, egui::Color32)>,
    mut on_send: impl FnMut(&str),
) {
    let mut edit_id = None;
    let outer = egui::Frame::new()
        .fill(theme::track())
        .stroke(egui::Stroke::new(
            1.2_f32,
            if *focus && enabled {
                theme::GOLD
            } else {
                theme::STROKE_OLIVE
            },
        ))
        .corner_radius(19.0)
        .inner_margin(egui::Margin::symmetric(6, 3))
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                // 대상 아바타 미니 칩 — 없으면(비활성) 딤 話 오너먼트
                match avatar {
                    Some((initial, color)) => widgets::avatar_chip(ui, initial, color, 22.0),
                    None => mute_chip(ui),
                }
                let edit = ui.add_enabled(
                    enabled,
                    egui::TextEdit::singleline(draft)
                        .frame(egui::Frame::NONE)
                        .hint_text(hint)
                        .desired_width((ui.available_width() - 42.0).max(40.0)),
                );
                let enter_in_edit =
                    edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                let can_send = enabled && !draft.trim().is_empty();
                if enabled && enter_in_edit && can_send {
                    on_send(draft.trim());
                    draft.clear();
                    edit.request_focus();
                }
                *focus = edit.has_focus() && enabled;
                edit_id = Some(edit.id);
                // ── 원형 골드 송신 버튼 — 하이라이트 베벨 돔 ──
                send_button(ui, can_send, || {
                    let text = draft.trim().to_owned();
                    if !text.is_empty() {
                        on_send(&text);
                        draft.clear();
                        edit.request_focus();
                    }
                });
            });
        });
    // 필 어디를 눌러도 입력창 포커스 — 상용 채팅 바 동작(패딩·아바타 옆
    // 빈 공간 클릭이 무시돼 '클릭 먹힘'처럼 보이는 사고 방지)
    if enabled
        && outer.response.clicked()
        && let Some(id) = edit_id
    {
        ui.ctx().memory_mut(|m| m.request_focus(id));
    }
    // 포커스 글로우 — 프레임 바깥 소프트 골드 링(같은 프레임 즉시 반영)
    if *focus && enabled {
        ui.painter().rect_stroke(
            outer.response.rect.expand(2.5),
            21.0,
            egui::Stroke::new(
                3.0_f32,
                egui::Color32::from_rgba_unmultiplied(232, 184, 64, 46),
            ),
            egui::StrokeKind::Middle,
        );
    }
}

/// 비활성 컴포저의 딤 오너먼트 — 어두운 원 + 話(대화 없음 표시)
fn mute_chip(ui: &mut egui::Ui) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::hover());
    let p = ui.painter();
    let c = rect.center();
    p.circle_filled(c, 10.0, theme::track());
    p.circle_stroke(c, 9.5, egui::Stroke::new(1.0_f32, theme::STROKE_OLIVE));
    p.text(
        c,
        egui::Align2::CENTER_CENTER,
        "話",
        egui::FontId::proportional(10.0),
        theme::TEXT_DIM,
    );
}

/// 원형 송신 버튼 — 골드 돔(상단 하이라이트)+ 어두운 화살표. 비활성은
/// 올리브 링 딤. 클릭 시 on_send(호버 커서는 재바인딩 — self 소비 함정).
fn send_button(ui: &mut egui::Ui, can: bool, on_send: impl FnOnce()) {
    let (rect, btn) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::click());
    let p = ui.painter();
    let c = rect.center();
    if can {
        let btn = btn.on_hover_cursor(egui::CursorIcon::PointingHand);
        if btn.clicked() {
            on_send();
        }
        let pressed = btn.is_pointer_button_down_on();
        // 베벨 — 어두운 밑동 + 밝은 돔(눌림=가라앉음, 호버=밝게)
        p.circle_filled(c, 15.0, egui::Color32::from_rgb(138, 100, 32));
        let (dome_c, dome_fill) = if pressed {
            (
                c + egui::vec2(0.0, 1.0),
                egui::Color32::from_rgb(201, 155, 46),
            )
        } else if btn.hovered() {
            (c + egui::vec2(0.0, -0.8), theme::GOLD_SOFT)
        } else {
            (c + egui::vec2(0.0, -0.8), theme::GOLD)
        };
        p.circle_filled(dome_c, 13.5, dome_fill);
        // 글로시 하이라이트 호
        p.circle_stroke(
            c + egui::vec2(0.0, -4.0),
            8.0,
            egui::Stroke::new(2.0_f32, egui::Color32::from_white_alpha(56)),
        );
        p.text(
            c,
            egui::Align2::CENTER_CENTER,
            "→",
            egui::FontId::proportional(15.0),
            egui::Color32::from_rgb(58, 48, 24),
        );
    } else {
        p.circle_filled(c, 14.0, theme::track());
        p.circle_stroke(c, 13.5, egui::Stroke::new(1.2_f32, theme::STROKE_OLIVE));
        p.text(
            c,
            egui::Align2::CENTER_CENTER,
            "→",
            egui::FontId::proportional(15.0),
            theme::TEXT_DIM,
        );
    }
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

/// 라이브 프레임 주민 슬라이스 — 폴백 빈 조각(화자 판정·이름 조회 공용)
fn live_agents<'a>(net: &'a Option<Res<'a, NetState>>) -> &'a [LiveAgent] {
    net.as_ref()
        .and_then(|n| n.frame.as_ref())
        .map(|f| f.agents.as_slice())
        .unwrap_or(&[])
}
