//! UI 셸 — 상단 바(브랜드·시계·날씨·인원·접속 배지·패널 토글).
//! 레거시 FE TopBar 직역: 시계는 "{계절} {연}년 {월}월 {일}일 {시}:{분}"
//! (태양력 달력 — 3월 시작 365일년). 주민 패널은 villagers.rs로 분리.

use bevy::prelude::*;
use bevy_egui::egui;

use super::i18n::{Tr, tr, trf};
use super::theme;
use super::{UiState, ctx};
use crate::daynight::GameTime;
use crate::net::{NetState, NetStatus};

pub fn top_bar(
    mut contexts: bevy_egui::EguiContexts,
    gt: Res<GameTime>,
    net: Option<Res<NetState>>,
    mut ui_state: ResMut<UiState>,
) {
    let Some(c) = ctx(&mut contexts) else { return };
    let lang = ui_state.lang;
    let frame = net.as_ref().and_then(|n| n.frame.as_ref());
    // 반응형 계층 — 폰 좁은 폭에서 정보 칩부터 줄인다(레거시 mobile과 동일 정보 유지)
    let sw = c.screen_rect().width();
    let (show_brand, compact_clock, icon_only_weather) =
        if sw >= 560.0 { (true, false, false) } else if sw >= 420.0 { (false, false, true) } else { (false, true, true) };
    egui::Panel::top("topbar")
        .exact_height(38.0)
        .frame(egui::Frame::new().fill(egui::Color32::from_rgba_unmultiplied(237, 231, 216, 248)).stroke(egui::Stroke::new(1.0_f32, theme::STROKE_TAN)))
        .show(&c, |ui| {
            ui.horizontal_centered(|ui| {
                ui.add_space(6.0);
                // 브랜드 — 골드 강조(좁은 화면 생략)
                if show_brand {
                    ui.label(
                        egui::RichText::new("TERRAFOLK")
                            .size(12.0)
                            .strong()
                            .color(theme::ACCENT_SAND)
                            .background_color(egui::Color32::TRANSPARENT),
                    );
                    ui.separator();
                }
                let dial_resp = mini_dial(ui, &gt);
                // 서버 정본 일출/일몰 hover(분 → HH:MM)
                if let Some((sr, ss)) = net
                    .as_ref()
                    .and_then(|n| n.frame.as_ref())
                    .and_then(|f| f.sunrise.zip(f.sunset))
                {
                    let fmt = |m: u16| format!("{:02}:{:02}", m / 60, m % 60);
                    let text = tr(lang, Tr::SunTimes)
                        .replacen("{}", &fmt(sr), 1)
                        .replacen("{}", &fmt(ss), 1);
                    dial_resp.on_hover_text(text);
                }
                // 시계 — 계절+태양력 날짜(레거시 ko 서식 직역). 좁으면 연도 생략
                let (y, m, d) = theme::date_of(gt.day as u32);
                let season = frame.map(|f| f.season.as_str()).unwrap_or("spring");
                let season_lbl = theme::season_mark(lang, season).0;
                let minutes = (gt.t * 24.0 * 60.0) as u32;
                let clock = if compact_clock {
                    format!("{season_lbl} {m:02}/{d:02} {:02}:{:02}", minutes / 60, minutes % 60)
                } else {
                    match lang {
                        super::i18n::Lang::Ko => format!(
                            "{season_lbl} {y}년 {m:02}월 {d:02}일 {:02}:{:02}",
                            minutes / 60,
                            minutes % 60
                        ),
                        super::i18n::Lang::En => format!(
                            "{season_lbl} Y{y}·{m}/{d} {:02}:{:02}",
                            minutes / 60,
                            minutes % 60
                        ),
                    }
                };
                ui.label(egui::RichText::new(clock).size(11.5).color(theme::TEXT_WARM));
                // 날씨 칩 — 프레임 weather(좁으면 아이콘만)
                if let Some(w) = frame.and_then(|f| f.weather.as_deref()) {
                    let icon = match w {
                        "clear" => "☀",
                        "overcast" | "fog" => "☁",
                        "rain" => "雨",
                        "storm" => "雷",
                        "snow" => "雪",
                        _ => "·",
                    };
                    let text = if icon_only_weather {
                        icon.to_owned()
                    } else {
                        format!("{icon} {}", theme::weather_label(lang, w))
                    };
                    chip(ui, &text);
                }
                // 인원 칩
                let pop = frame.map(|f| f.agents.len()).unwrap_or(0);
                if pop > 0 {
                    chip(ui, &trf(lang, Tr::Population, &pop.to_string()));
                }
                // 접속 배지 — 라이브 ● / 접속중 ◌ / 데모
                let (badge, color) = match net.as_ref().map(|n| n.status) {
                    Some(NetStatus::Connected) => ("● LIVE", theme::LIVE_GREEN),
                    Some(NetStatus::Connecting) => (tr(lang, Tr::Connecting), theme::WARN),
                    Some(NetStatus::Reconnecting) => (tr(lang, Tr::Reconnecting), theme::WARN),
                    _ => (tr(lang, Tr::Demo), egui::Color32::from_rgb(122, 118, 104)),
                };
                ui.label(
                    egui::RichText::new(badge)
                        .size(10.5)
                        .strong()
                        .color(color),
                );
                // ☰ 메뉴 — 레거시 FE TopBar 드롭다운 직역(우측 끝)
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_space(4.0);
                    let menu_text = egui::RichText::new("≡").size(14.0).color(theme::INK);
                    egui::menu::menu_button(ui, menu_text, |ui| {
                        ui.set_min_width(172.0);
                        let items = [
                            (Tr::ToggleVillagers, "人"),
                            (Tr::ToggleChat, "話"),
                            (Tr::ToggleResources, "木"),
                            (Tr::ToggleEvents, "史"),
                            (Tr::ToggleCiv, "文"),
                            (Tr::ToggleRelations, "緣"),
                            (Tr::ToggleEconomy, "貨"),
                            (Tr::ToggleMinimap, "圖"),
                            (Tr::ToggleSettings, "⚙"),
                        ];
                        for (key, icon) in items {
                            if key == Tr::ToggleSettings {
                                ui.separator();
                            }
                            let on = menu_toggle_on(&ui_state, key);
                            let label = if on {
                                egui::RichText::new(format!("{icon} {}  ●", tr(lang, key)))
                                    .size(12.0)
                                    .color(theme::TEXT_WARM)
                                    .strong()
                            } else {
                                egui::RichText::new(format!("{icon} {}", tr(lang, key)))
                                    .size(12.0)
                                    .color(theme::TEXT_DIM)
                            };
                            // 기본 버튼(호버 배경 유지) — 항목 클릭으로만 닫힌다
                            if ui.add(egui::Button::new(label)).clicked() {
                                menu_toggle(&mut ui_state, key, sw);
                                ui.close_menu();
                            }
                        }
                    });
                });
            });
        });
}

/// 메뉴 항목 → 현재 열림 여부
fn menu_toggle_on(s: &UiState, key: Tr) -> bool {
    match key {
        Tr::ToggleVillagers => s.show_villagers,
        Tr::ToggleChat => s.show_chat,
        Tr::ToggleResources => s.show_resources,
        Tr::ToggleEvents => s.show_events,
        Tr::ToggleCiv => s.show_civ,
        Tr::ToggleRelations => s.show_relations,
        Tr::ToggleEconomy => s.show_economy,
        Tr::ToggleMinimap => s.show_minimap,
        _ => s.show_settings,
    }
}

/// 메뉴 항목 토글 — 좁은 화면(폰)에서는 플로팅 패널을 한 번에 하나만
/// (레거시 모달 팝업과 같은 단일 창 정책). 도킹 패널(주민·채팅)은 복수 허용.
fn menu_toggle(s: &mut UiState, key: Tr, screen_w: f32) {
    s.user_adjusted = true; // 반응형 와치독 정지 — 사용자 의도 우선
    let narrow = screen_w < 760.0;
    // 단일 창 정책 대상 — 캐릭터는 모바일 모달일 때만(데스크톱은 도킹 패널)
    let is_float = if key == Tr::ToggleVillagers {
        narrow
    } else {
        matches!(
            key,
            Tr::ToggleResources
                | Tr::ToggleEvents
                | Tr::ToggleCiv
                | Tr::ToggleRelations
                | Tr::ToggleEconomy
                | Tr::ToggleSettings
        )
    };
    if narrow && is_float {
        s.show_resources = false;
        s.show_events = false;
        s.show_civ = false;
        s.show_relations = false;
        s.show_economy = false;
        s.show_settings = false;
        s.show_villagers = false;
    }
    match key {
        Tr::ToggleVillagers => s.show_villagers = !s.show_villagers,
        Tr::ToggleChat => s.show_chat = !s.show_chat,
        Tr::ToggleResources => s.show_resources = !s.show_resources,
        Tr::ToggleEvents => s.show_events = !s.show_events,
        Tr::ToggleCiv => s.show_civ = !s.show_civ,
        Tr::ToggleRelations => s.show_relations = !s.show_relations,
        Tr::ToggleEconomy => s.show_economy = !s.show_economy,
        Tr::ToggleMinimap => s.show_minimap = !s.show_minimap,
        _ => s.show_settings = !s.show_settings,
    }
}

/// 미니 주야 다이얼(레거시 DayDial 직역 — 해/달 궤적 + 지평선)
fn mini_dial(ui: &mut egui::Ui, gt: &GameTime) -> egui::Response {
    let (w, h) = (64.0, 18.0);
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(w, h), egui::Sense::hover());
    let p = ui.painter();
    // 지평선·궤적
    p.line_segment(
        [egui::pos2(rect.left(), rect.bottom() - 3.0), egui::pos2(rect.right(), rect.bottom() - 3.0)],
        egui::Stroke::new(1.0_f32, egui::Color32::from_white_alpha(28)),
    );
    let t = gt.t.clamp(0.0, 1.0);
    let x = rect.left() + 3.0 + t * (w - 6.0);
    let y = rect.bottom() - 3.0 - (t * std::f32::consts::PI).sin() * 10.0;
    let is_day = (0.25..0.75).contains(&gt.t);
    let (col, r) = if is_day {
        (egui::Color32::from_rgb(255, 209, 102), 3.4)
    } else {
        (egui::Color32::from_rgb(219, 228, 240), 3.0)
    };
    p.circle_filled(egui::pos2(x, y), r, col);
    if is_day {
        for (dx, dy) in [(1.0, 0.0), (-1.0, 0.0), (0.0, 1.0), (0.0, -1.0)] {
            p.line_segment(
                [
                    egui::pos2(x + dx * (r + 1.5), y + dy * (r + 1.5)),
                    egui::pos2(x + dx * (r + 4.6), y + dy * (r + 4.6)),
                ],
                egui::Stroke::new(1.2_f32, col),
            );
        }
    }
    resp
}

/// 작은 정보 칩(상단 바 통계) — 크림 필 + 탄색 스트로크
fn chip(ui: &mut egui::Ui, text: &str) {
    egui::Frame::new()
        .fill(theme::CREAM)
        .stroke(egui::Stroke::new(1.0_f32, theme::STROKE_TAN))
        .corner_radius(999.0)
        .inner_margin(egui::Margin::symmetric(7, 2))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(10.5).color(theme::INK_DIM));
        });
}
