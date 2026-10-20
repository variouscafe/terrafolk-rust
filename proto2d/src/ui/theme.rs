//! egui 코지 테마 v3 — 레퍼런스 이미지 UI 추출 정착(2026-10-20).
//! 크림 패널(#F5F2EA/#EDE7D8)·탄색 스트로크(#B9A88C)·골드 포인트(#F2B33D)·
//! 필 바·12~14px 라운드 — 다크 그린 v2에서 코지 팜심 크롬으로 전환.
//! 본문 잉크는 웜 브라운(#3E3A34) — 크림 위 가독.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

// ── 패널 표면(레퍼런스 핫바·토스트 추출) ──
pub const CREAM: egui::Color32 = egui::Color32::from_rgb(245, 242, 234); // 메인 카드 #F5F2EA
pub const CREAM_2: egui::Color32 = egui::Color32::from_rgb(237, 231, 216); // 2단·도킹 바 #EDE7D8
pub const CREAM_3: egui::Color32 = egui::Color32::from_rgb(228, 219, 198); // 호버·트랙 #E4DBC6
pub const STROKE_TAN: egui::Color32 = egui::Color32::from_rgb(185, 168, 140); // 외곽선 #B9A88C
pub const INK: egui::Color32 = egui::Color32::from_rgb(62, 58, 52); // 본문 잉크 #3E3A34
pub const INK_DIM: egui::Color32 = egui::Color32::from_rgb(122, 111, 96); // 보조 #7A6F60
#[allow(dead_code)] // 골드 포인트 — 배지·포인트용 예비
pub const GOLD: egui::Color32 = egui::Color32::from_rgb(242, 179, 61); // 골드 포인트 #F2B33D
pub const GOLD_DEEP: egui::Color32 = egui::Color32::from_rgb(191, 136, 27); // 골드 텍스트용
pub const SEL_FILL: egui::Color32 = egui::Color32::from_rgb(242, 227, 184); // 선택행 밝은 골드

// ── 의미색(크림 위 가독로 진하게) ──
pub const LIVE_GREEN: egui::Color32 = egui::Color32::from_rgb(46, 148, 67);
pub const WATER_BLUE: egui::Color32 = egui::Color32::from_rgb(38, 122, 160);
pub const DANGER: egui::Color32 = egui::Color32::from_rgb(196, 62, 50);
pub const WARN: egui::Color32 = egui::Color32::from_rgb(198, 136, 28);

/// 욕구 바 4색(레거시 FE NEEDS 준거 — 크림 위 진톤)
pub const NEED_HUNGER: egui::Color32 = egui::Color32::from_rgb(201, 123, 58);
pub const NEED_THIRST: egui::Color32 = egui::Color32::from_rgb(59, 135, 184);
pub const NEED_FATIGUE: egui::Color32 = egui::Color32::from_rgb(123, 98, 184);
pub const NEED_WARMTH: egui::Color32 = egui::Color32::from_rgb(194, 76, 66);

/// 주민 상태색(진톤 — 크림 위 라벨 가독)
pub const ST_IDLE: egui::Color32 = egui::Color32::from_rgb(133, 122, 104);
pub const ST_MOVING: egui::Color32 = egui::Color32::from_rgb(64, 122, 178);
pub const ST_ACTING: egui::Color32 = egui::Color32::from_rgb(184, 134, 34);
pub const ST_THINKING: egui::Color32 = egui::Color32::from_rgb(134, 98, 182);
pub const ST_SLEEPING: egui::Color32 = egui::Color32::from_rgb(88, 104, 168);

// ── 호환 재수출(v2 이름 유지 — 크롬 일괄 전환 전 참조 파일들이 쓴다) ──
pub const TEXT_WARM: egui::Color32 = INK;
pub const TEXT_DIM: egui::Color32 = INK_DIM;
pub const ACCENT_SAND: egui::Color32 = GOLD_DEEP;

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

fn hash8(id: &str) -> usize {
    let mut h: u32 = 0;
    for c in id.chars() {
        h = h.wrapping_mul(31).wrapping_add(c as u32);
    }
    h as usize
}

/// 주민별 결정론 색 — id 해시 8색 팔레트(레거시 agent_color 직역).
/// 팔레트 원색은 점·아이콘용.
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
    let (r, g, b) = PALETTE[hash8(id) % 8];
    egui::Color32::from_rgb(r, g, b)
}

/// 크림 배경 위 주민 텍스트색(같은 색상성, 명도 낮춘 잉크 변형)
pub fn agent_ink(id: &str) -> egui::Color32 {
    const PALETTE: [(u8, u8, u8); 8] = [
        (158, 112, 44),
        (52, 118, 172),
        (74, 142, 88),
        (178, 96, 132),
        (128, 96, 184),
        (186, 96, 78),
        (58, 148, 138),
        (160, 148, 62),
    ];
    let (r, g, b) = PALETTE[hash8(id) % 8];
    egui::Color32::from_rgb(r, g, b)
}

/// 계절 라벨+색 — 레거시 FE season chip 직역(크림 위 진톤)
pub fn season_mark(lang: crate::ui::i18n::Lang, season: &str) -> (&'static str, egui::Color32) {
    use crate::ui::i18n::{Tr, tr};
    match season {
        "spring" => (tr(lang, Tr::SeasonSpring), egui::Color32::from_rgb(97, 156, 74)),
        "summer" => (tr(lang, Tr::SeasonSummer), egui::Color32::from_rgb(190, 138, 26)),
        "autumn" => (tr(lang, Tr::SeasonAutumn), egui::Color32::from_rgb(186, 108, 44)),
        "winter" => (tr(lang, Tr::SeasonWinter), egui::Color32::from_rgb(96, 126, 168)),
        _ => ("", INK_DIM),
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

/// 게임 일 → (연, 월, 일) — 레거시 FE 달력 직역(2월 시작 365일년)
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
/// 라이트 베이스 + 크림 표면 — 위젯 기본값도 라이트로 따라온다.
pub fn apply(mut contexts: EguiContexts, mut done: Local<bool>) {
    if *done {
        return;
    }
    let Some(ctx) = contexts.ctx_mut().ok().cloned() else {
        return;
    };
    let mut vis = egui::Visuals::light();
    vis.panel_fill = CREAM_2;
    vis.window_fill = CREAM;
    vis.extreme_bg_color = CREAM_2;
    vis.faint_bg_color = CREAM_3;
    vis.override_text_color = Some(INK);
    vis.weak_text_color = Some(INK_DIM);
    vis.hyperlink_color = WATER_BLUE;
    // 선택 — 골드
    vis.selection.bg_fill = egui::Color32::from_rgb(242, 214, 138);
    vis.selection.stroke.color = INK;
    // 위젯 — 크림 계열
    vis.widgets.noninteractive.bg_fill = CREAM_2;
    vis.widgets.inactive.bg_fill = CREAM;
    vis.widgets.hovered.bg_fill = CREAM_3;
    vis.widgets.active.bg_fill = CREAM_3;
    vis.widgets.open.bg_fill = CREAM;
    let round = egui::CornerRadius::same(8);
    vis.widgets.noninteractive.corner_radius = round;
    vis.widgets.inactive.corner_radius = round;
    vis.widgets.hovered.corner_radius = round;
    vis.widgets.active.corner_radius = round;
    vis.window_corner_radius = egui::CornerRadius::same(14);
    vis.menu_corner_radius = egui::CornerRadius::same(12);
    let shadow = egui::Shadow {
        offset: [0, 4],
        blur: 14,
        spread: 0,
        color: egui::Color32::from_black_alpha(70),
    };
    vis.popup_shadow = shadow;
    vis.window_shadow = shadow;
    // 스트로크 — 탄색(은은한 웜), 호버 골드
    vis.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, STROKE_TAN);
    vis.widgets.inactive.fg_stroke = egui::Stroke::new(1.2_f32, STROKE_TAN);
    vis.widgets.hovered.fg_stroke = egui::Stroke::new(1.4_f32, GOLD_DEEP);
    vis.widgets.active.fg_stroke = egui::Stroke::new(1.6_f32, GOLD_DEEP);
    ctx.set_visuals(vis);
    *done = true;
    info!("[ui] egui 코지 크림 테마 적용");
}
