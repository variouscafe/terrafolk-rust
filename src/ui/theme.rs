//! UI 테마 — 레거시 FE 디자인 토큰 직역(index.css 팔레트, fe-parity 규칙).
//! 다른 게임에 재사용할 때 이 파일의 상수만 교체하면 UI 전체 색이 따라온다.

use bevy::prelude::*;

/// 배경·표면 계열(deep-ink 다크)
pub const INK950: Color = Color::srgb(0.043, 0.055, 0.078); // #0b0e14
pub const INK900: Color = Color::srgb(0.067, 0.082, 0.114); // #11151d
pub const INK800: Color = Color::srgb(0.094, 0.114, 0.157); // #181d28
pub const INK700: Color = Color::srgb(0.129, 0.157, 0.212); // #212836
pub const INK600: Color = Color::srgb(0.173, 0.208, 0.278); // #2c3547
/// 본문 텍스트
pub const INK200: Color = Color::srgb(0.682, 0.718, 0.776); // #aeb7c6
pub const INK50: Color = Color::srgb(0.933, 0.945, 0.965); // #eef1f6
pub const INK400: Color = Color::srgb(0.420, 0.463, 0.537); // #6b7689
/// 액센트(선택·강조)
pub const ACCENT: Color = Color::srgb(0.486, 0.612, 1.0); // #7c9cff
/// 언릿 앰버(선택 링과 동일 계열)
pub const AMBER: Color = Color::srgb(1.0, 0.78, 0.28);

/// 폰트 핸들 + 공통 스타일 헬퍼 — 모든 UI 텍스트는 이 리소스에서 폰트를 받는다.
#[derive(Resource)]
pub struct UiTheme {
    pub font: Handle<Font>,
}

impl UiTheme {
    /// 텍스트 스팬 공통 번들 (TextFont + TextColor) — 색은 TextColor가 담당.
    pub fn text(&self, size: f32, color: Color) -> (TextFont, TextColor) {
        (
            TextFont {
                font: bevy::text::FontSource::Handle(self.font.clone()),
                font_size: size.into(),
                ..default()
            },
            TextColor(color),
        )
    }
    /// 본문 라벨(ink-200)
    pub fn label(&self, size: f32) -> (TextFont, TextColor) {
        self.text(size, INK200)
    }
    /// 제목(ink-50)
    pub fn title(&self, size: f32) -> (TextFont, TextColor) {
        self.text(size, INK50)
    }
}

/// 반투명 표면(레거시 HUD의 bg-ink-950/75 대응)
pub const HUD_BG: Color = Color::srgba(0.043, 0.055, 0.078, 0.78);
/// 팝업 표면(surface-raised 대응)
pub const SURFACE: Color = INK900;

/// 날씨 아이콘(레거시 WEATHER_ICON 직역) — 이모지 폰트가 없어 텍스트 심볼로.
/// 종류는 precip::WeatherState.kind(clear·overcast·rain·storm·snow·fog).
pub fn weather_icon(weather: &str) -> &'static str {
    match weather {
        "clear" => "☀",
        "overcast" => "☁",
        "rain" => "☂",
        "snow" => "❄",
        "storm" => "⚡",
        "fog" => "≈",
        _ => "",
    }
}

/// 날씨 한글명(레거시 i18n 직역)
pub fn weather_label(weather: &str) -> &'static str {
    match weather {
        "clear" => "맑음",
        "overcast" => "흐림",
        "rain" => "비",
        "snow" => "눈",
        "storm" => "폭풍",
        "fog" => "안개",
        _ => "",
    }
}

/// 계절 한글명
pub fn season_label(season: &str) -> &'static str {
    match season {
        "spring" => "봄",
        "summer" => "여름",
        "autumn" => "가을",
        "winter" => "겨울",
        _ => "",
    }
}

/// 서버 상태 코드 → 한글 라벨(레거시 status.* 직역, 알 수 없으면 원문)
pub fn status_label(status: &str) -> String {
    match status {
        "idle" => "휴식 중".into(),
        "walk" | "wander" => "이동 중".into(),
        "chop" => "벌목 중".into(),
        "gather" => "채집 중".into(),
        "build" => "건설 중".into(),
        "fish" => "낚시 중".into(),
        "drink" => "취수 중".into(),
        "eat" => "식사 중".into(),
        "sleep" => "취침 중".into(),
        "talk" => "대화 중".into(),
        other => other.to_string(),
    }
}

/// 편의 — 투명 배경 버튼용
pub const TRANSPARENT: Color = Color::NONE;
