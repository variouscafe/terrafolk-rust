//! egui 코지 테마 — 레퍼런스(핸드페인트 cozy 아이소) 팔레트 정렬.
//! 웜 다크 그린 패널·샌드 골드 액센트·둥근 모서리. 폰트 설치 직후 1회 적용.
//! 상용 게임 크롨(v2, 2026-10-20): 카드 프레임·섹션 헤더·바 위젯은 widgets.rs.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

// ── 팔레트 — CLAUDE.md 레퍼런스 이미지 기준(웜 그린/샌드/2톤 물 대비) ──
pub const PANEL_BG: egui::Color32 = egui::Color32::from_rgb(26, 32, 21); // 깊은 웜 그린
pub const PANEL_BG_HI: egui::Color32 = egui::Color32::from_rgb(38, 46, 29);
pub const CARD_BG: egui::Color32 = egui::Color32::from_rgb(35, 43, 28); // 카드 1단 상승
pub const CARD_BG_HI: egui::Color32 = egui::Color32::from_rgb(46, 56, 35);
pub const TEXT_WARM: egui::Color32 = egui::Color32::from_rgb(236, 230, 208); // 웜 아이보리
pub const TEXT_DIM: egui::Color32 = egui::Color32::from_rgb(163, 173, 141);
pub const ACCENT_SAND: egui::Color32 = egui::Color32::from_rgb(233, 203, 125); // 샌드 골드
pub const LIVE_GREEN: egui::Color32 = egui::Color32::from_rgb(96, 226, 126);
pub const WATER_BLUE: egui::Color32 = egui::Color32::from_rgb(79, 195, 217);
pub const DANGER: egui::Color32 = egui::Color32::from_rgb(240, 108, 98);
pub const WARN: egui::Color32 = egui::Color32::from_rgb(240, 190, 90);

/// 욕구 바 4색 — 레거시 FE NEEDS 팔레트 준거
pub const NEED_HUNGER: egui::Color32 = egui::Color32::from_rgb(235, 160, 90);
pub const NEED_THIRST: egui::Color32 = egui::Color32::from_rgb(96, 190, 230);
pub const NEED_FATIGUE: egui::Color32 = egui::Color32::from_rgb(178, 150, 235);
pub const NEED_WARMTH: egui::Color32 = egui::Color32::from_rgb(238, 130, 118);

/// 주민 상태색 — 레거시 FE STATUS_COLOR 직역
pub const ST_IDLE: egui::Color32 = egui::Color32::from_rgb(158, 168, 134);
pub const ST_MOVING: egui::Color32 = egui::Color32::from_rgb(125, 188, 255);
pub const ST_ACTING: egui::Color32 = egui::Color32::from_rgb(250, 208, 120);
pub const ST_THINKING: egui::Color32 = egui::Color32::from_rgb(196, 160, 255);
pub const ST_SLEEPING: egui::Color32 = egui::Color32::from_rgb(130, 150, 235);

/// 관계 온기 등급 라벨(레거시 REL_BUCKET 직역: <30/<45/<65/<80/≥80) — ko/en
pub fn warmth_tier(lang: crate::ui::i18n::Lang, w: f32) -> &'static str {
    use crate::ui::i18n::{Tr, tr};
    let key = if w >= 80.0 {
        Tr::WarmBest
    } else if w >= 65.0 {
        Tr::WarmGood
    } else if w >= 45.0 {
        Tr::WarmNormal
    } else if w >= 30.0 {
        Tr::WarmAwkward
    } else {
        Tr::WarmBad
    };
    tr(lang, key)
}

/// 주민별 결정론 색 — id 해시 8색 팔레트(레거시 agent_color 직역)
pub fn agent_color(id: &str) -> egui::Color32 {
    const PALETTE: [(u8, u8, u8); 8] = [
        (255, 210, 138),
        (154, 208, 255),
        (165, 232, 176),
        (240, 168, 200),
        (216, 194, 255),
        (255, 176, 160),
        (160, 232, 224),
        (232, 224, 154),
    ];
    let mut h: u32 = 0;
    for c in id.chars() {
        h = h.wrapping_mul(31).wrapping_add(c as u32);
    }
    let (r, g, b) = PALETTE[(h % 8) as usize];
    egui::Color32::from_rgb(r, g, b)
}

/// 계절 라벨+색 — 레거시 FE season chip 직역
pub fn season_mark(lang: crate::ui::i18n::Lang, season: &str) -> (&'static str, egui::Color32) {
    use crate::ui::i18n::{Tr, tr};
    match season {
        "spring" => (tr(lang, Tr::SeasonSpring), egui::Color32::from_rgb(160, 224, 150)),
        "summer" => (tr(lang, Tr::SeasonSummer), egui::Color32::from_rgb(250, 216, 110)),
        "autumn" => (tr(lang, Tr::SeasonAutumn), egui::Color32::from_rgb(235, 160, 96)),
        "winter" => (tr(lang, Tr::SeasonWinter), egui::Color32::from_rgb(190, 214, 240)),
        _ => ("", TEXT_DIM),
    }
}

/// 날씨 라벨 — 레거시 FE weather chip 직역(clear/overcast/rain/storm/snow/fog)
pub fn weather_label(lang: crate::ui::i18n::Lang, weather: &str) -> &'static str {
    use crate::ui::i18n::{Tr, tr};
    match weather {
        "clear" => tr(lang, Tr::WeatherClear),
        "overcast" => tr(lang, Tr::WeatherOvercast),
        "rain" => tr(lang, Tr::WeatherRain),
        "storm" => tr(lang, Tr::WeatherStorm),
        "snow" => tr(lang, Tr::WeatherSnow),
        "fog" => tr(lang, Tr::WeatherFog),
        _ => "",
    }
}

/// 게임 일 → (연, 월, 일, 계절) — 레거시 FE 달력 직역(2월 시작 365일년)
pub fn date_of(day: u32) -> (u32, u32, u32) {
    const YEAR_DAYS: u32 = 365;
    const MONTH_CUM: [u32; 12] = [31, 61, 92, 122, 153, 184, 214, 245, 275, 306, 337, 365];
    let doy = day % YEAR_DAYS + 1;
    let year = day / YEAR_DAYS + 1;
    let mut month = 2;
    let mut day_of_month = 28;
    for (i, &cum) in MONTH_CUM.iter().enumerate() {
        if doy <= cum {
            month = ((i + 2) % 12 + 1) as u32;
            day_of_month = doy - if i > 0 { MONTH_CUM[i - 1] } else { 0 };
            break;
        }
    }
    (year, month, day_of_month)
}

/// 폰트 설치 후 시각(Visuals) 적용 — fonts.rs의 install과 같은 시점에 1회.
pub fn apply(mut contexts: EguiContexts, mut done: Local<bool>) {
    if *done {
        return;
    }
    let Some(ctx) = contexts.ctx_mut().ok().cloned() else {
        return;
    };
    let mut vis = egui::Visuals::dark();
    vis.panel_fill = PANEL_BG;
    vis.window_fill = PANEL_BG;
    vis.extreme_bg_color = PANEL_BG_HI;
    vis.faint_bg_color = CARD_BG;
    vis.override_text_color = Some(TEXT_WARM);
    vis.selection.bg_fill = egui::Color32::from_rgb(58, 66, 40);
    vis.selection.stroke.color = ACCENT_SAND;
    vis.hyperlink_color = WATER_BLUE;
    vis.widgets.noninteractive.bg_fill = PANEL_BG;
    vis.widgets.inactive.bg_fill = CARD_BG_HI;
    vis.widgets.hovered.bg_fill = egui::Color32::from_rgb(52, 62, 38);
    vis.widgets.active.bg_fill = egui::Color32::from_rgb(64, 76, 46);
    vis.widgets.open.bg_fill = CARD_BG_HI;
    let round = egui::CornerRadius::same(6);
    vis.widgets.noninteractive.corner_radius = round;
    vis.widgets.inactive.corner_radius = round;
    vis.widgets.hovered.corner_radius = round;
    vis.widgets.active.corner_radius = round;
    vis.window_corner_radius = egui::CornerRadius::same(10);
    vis.menu_corner_radius = egui::CornerRadius::same(8);
    // 라인은 은은한 웜 — 아웃라인 없는 레퍼런스 룩에 맞춰 약하게
    vis.widgets.noninteractive.fg_stroke.color = TEXT_DIM;
    vis.widgets.inactive.fg_stroke.color = TEXT_WARM;
    vis.widgets.hovered.fg_stroke.color = ACCENT_SAND;
    vis.widgets.active.fg_stroke.color = ACCENT_SAND;
    ctx.set_visuals(vis);
    *done = true;
    info!("[ui] egui 코지 테마 적용");
}
