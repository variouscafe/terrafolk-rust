//! egui 코지 테마 v4 — 레퍼런스 이미지 UI 픽셀 실측 정착(2026-10-20).
//! 다크 올리브-차콜 글래스 패널(#222F28@85%) + 라이트 올리브 스트로크
//! (#586A50) + 크림 잉크(#F4E3BE) + 골드 포인트(#E8B840) + 비비드 필 바
//! (HP #59C971·스태미나 #40BCF7 실측). v3 크림 크롬은 레퍼런스 오독
//! (핫바 슬롯 테두리만 크림)이라 폐기 — 실측 스펙이 우선.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

// ── 패널 표면(레퍼런스 실측: 글래스 #222F28@85%·스트로크 #586A50) ──
// 반투명색은 from_rgba_unmultiplied가 비const라 함수로 제공한다.
/// 카드·플로팅 패널 — 반투명 글래스(지형이 은은히 비침).
/// 렌더러가 선형 블렌딩이라 같은 알파도 밝게 뜬다 — 실측 보정치.
pub fn glass() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(30, 40, 34, 240)
}
/// 도킹 바(상단·하단·우측 패널) — 거의 불투명(가독 우선)
pub fn glass_dock() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(25, 34, 29, 248)
}
/// 인셋 표면 — 행·칩·탭 밝은 쪽
pub fn glass_hi() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(48, 60, 50, 190)
}
/// 바 트랙(레퍼런스 HP 바 트랙 #20261F)
pub fn track() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(18, 24, 20, 200)
}
/// 선택행 하이라이트 — 올리브
pub fn sel_fill() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(96, 116, 84, 110)
}
/// 선택 필 버튼(활성 탭·언어 토글) — sel_fill보다 진한 같은 올리브
pub fn sel_pill() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(96, 116, 84, 170)
}
/// 핫바 슬롯식 퍼치먼트 테두리(실측 #D8C9A0) — 소지품 칩
pub fn parchment() -> egui::Color32 {
    egui::Color32::from_rgba_unmultiplied(216, 201, 160, 150)
}

// ── 라인·잉크 ──
/// 패널 스트로크 — 라이트 올리브(실측 #586A50)
pub const STROKE_OLIVE: egui::Color32 = egui::Color32::from_rgb(88, 106, 80);
/// 본문 잉크 — 크림(실측 #F4E3BE)
pub const INK: egui::Color32 = egui::Color32::from_rgb(244, 227, 190);
/// 보조 잉크 — 세이지
pub const INK_DIM: egui::Color32 = egui::Color32::from_rgb(169, 183, 158);
/// 골드 포인트(실측 #E8B840) — 아이콘·배지
pub const GOLD: egui::Color32 = egui::Color32::from_rgb(232, 184, 64);
/// 밝은 골드 — 강조 텍스트
pub const GOLD_SOFT: egui::Color32 = egui::Color32::from_rgb(242, 206, 107);

// ── 의미색(다크 글래스 위 비비드 — 레퍼런스 HP/스태미나 실측 톤) ──
pub const LIVE_GREEN: egui::Color32 = egui::Color32::from_rgb(89, 201, 113); // 실측 HP #59C971
pub const WATER_BLUE: egui::Color32 = egui::Color32::from_rgb(64, 188, 247); // 실측 스태미나
pub const DANGER: egui::Color32 = egui::Color32::from_rgb(242, 105, 92);
pub const WARN: egui::Color32 = egui::Color32::from_rgb(240, 190, 90);

/// 욕구 바 4색(레거시 FE NEEDS 준거 — 다크 위 채도 원복)
pub const NEED_HUNGER: egui::Color32 = egui::Color32::from_rgb(235, 160, 94);
pub const NEED_THIRST: egui::Color32 = egui::Color32::from_rgb(96, 190, 230);
pub const NEED_FATIGUE: egui::Color32 = egui::Color32::from_rgb(178, 150, 235);
pub const NEED_WARMTH: egui::Color32 = egui::Color32::from_rgb(238, 130, 118);

/// 주민 상태색(레거시 FE STATUS_COLOR 직역 — 다크 위 비비드)
pub const ST_IDLE: egui::Color32 = egui::Color32::from_rgb(158, 168, 134);
pub const ST_MOVING: egui::Color32 = egui::Color32::from_rgb(125, 188, 255);
pub const ST_ACTING: egui::Color32 = egui::Color32::from_rgb(250, 208, 120);
pub const ST_THINKING: egui::Color32 = egui::Color32::from_rgb(196, 160, 255);
pub const ST_SLEEPING: egui::Color32 = egui::Color32::from_rgb(130, 150, 235);

// ── 호환 재수출(크롬 일괄 전환 전 참조 파일들이 쓴다 — 값은 v4) ──
pub const TEXT_WARM: egui::Color32 = INK;
pub const TEXT_DIM: egui::Color32 = INK_DIM;
pub const ACCENT_SAND: egui::Color32 = GOLD_SOFT;
pub const GOLD_DEEP: egui::Color32 = GOLD; // 다크 위에선 골드 단일계

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
/// 밝은 파스텔 — 다크 글래스 위 점·아이콘·텍스트 공용.
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

/// 다크 표면 위 주민 텍스트색 — v4부로 팔레트 자체가 밝아 같은 색.
pub fn agent_ink(id: &str) -> egui::Color32 {
    agent_color(id)
}

/// 세미볼드 폰트 — 타이틀·헤더용(fonts.rs가 "semi" 패밀리 등장)
pub fn font_semi(size: f32) -> egui::FontId {
    egui::FontId::new(size, egui::FontFamily::Name("semi".into()))
}

/// 계절 라벨+색 — 레거시 FE season chip 직역(다크 위 비비드)
pub fn season_mark(lang: crate::ui::i18n::Lang, season: &str) -> (&'static str, egui::Color32) {
    use crate::ui::i18n::{Tr, tr};
    match season {
        "spring" => (tr(lang, Tr::SeasonSpring), egui::Color32::from_rgb(160, 224, 150)),
        "summer" => (tr(lang, Tr::SeasonSummer), egui::Color32::from_rgb(250, 216, 110)),
        "autumn" => (tr(lang, Tr::SeasonAutumn), egui::Color32::from_rgb(235, 160, 96)),
        "winter" => (tr(lang, Tr::SeasonWinter), egui::Color32::from_rgb(190, 214, 240)),
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
/// 다크 베이스 + 올리브 글래스 — 위젯 기본(메뉴·스크롤·텍스트입력)도 동일 톤.
pub fn apply(mut contexts: EguiContexts, mut done: Local<bool>) {
    if *done {
        return;
    }
    let Some(ctx) = contexts.ctx_mut().ok().cloned() else {
        return;
    };
    // "semi" 패밀리 선바인딩 — 폰트 에셋 로드 전 1프레임에도 레이아웃이
    // 돌아야 한다(epaint는 미바인딩 패밀리 참조 시 패닉). 임시로 기본 폰트를
    // 물리고, 로드 후 fonts.rs가 진짜 SemiBold로 갈아끼운다.
    let mut defs = egui::FontDefinitions::default();
    defs.families.insert(
        egui::FontFamily::Name("semi".into()),
        vec!["Ubuntu-Light".into()],
    );
    ctx.set_fonts(defs);
    let mut vis = egui::Visuals::dark();
    vis.panel_fill = glass_dock();
    vis.window_fill = glass();
    vis.extreme_bg_color = track();
    vis.faint_bg_color = glass_hi();
    vis.override_text_color = Some(INK);
    vis.weak_text_color = Some(INK_DIM);
    vis.hyperlink_color = WATER_BLUE;
    // 선택 — 골드
    vis.selection.bg_fill = egui::Color32::from_rgba_unmultiplied(232, 184, 64, 70);
    vis.selection.stroke.color = INK;
    // 위젯 — 올리브 글래스 계열
    vis.widgets.noninteractive.bg_fill = glass_dock();
    vis.widgets.inactive.bg_fill = glass_hi();
    vis.widgets.hovered.bg_fill = egui::Color32::from_rgba_unmultiplied(64, 78, 62, 215);
    vis.widgets.active.bg_fill = egui::Color32::from_rgba_unmultiplied(80, 96, 70, 225);
    vis.widgets.open.bg_fill = glass_hi();
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
        color: egui::Color32::from_black_alpha(120),
    };
    vis.popup_shadow = shadow;
    vis.window_shadow = shadow;
    // 스트로크 — 올리브(레퍼런스 글래스 가장자리), 호버 골드
    vis.widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0_f32, STROKE_OLIVE);
    vis.widgets.inactive.fg_stroke = egui::Stroke::new(1.2_f32, STROKE_OLIVE);
    vis.widgets.hovered.fg_stroke = egui::Stroke::new(1.4_f32, GOLD);
    vis.widgets.active.fg_stroke = egui::Stroke::new(1.6_f32, GOLD);
    ctx.set_visuals(vis);
    *done = true;
    info!("[ui] egui 코지 글래스 테마 v4 적용(레퍼런스 실측)");
}
