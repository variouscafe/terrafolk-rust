//! 상단 HUD — 레거시 TopBar 직역(시계·날씨 칩·인원·관전 칩 + ☰ 자리).
//! ☰ 버튼 본체는 menu.rs가 소유(드롭다운과 한 루트).

use bevy::prelude::*;

use super::theme::{self, UiTheme};
use crate::net::{NetState, NetStatus};
use crate::world::daynight::DayTime;
use crate::world::precip::WeatherState;
use crate::world::agent::Villager;

/// HUD 플러그인 — 웹·네이티브 공용(2026-10-25 DOM→bevy_ui 통일 2호).
/// 정보바(제목·시계·해시계·날씨·인구·링크)는 단일 구현. ☰ 메뉴 버튼은
/// 네이티브 전용(웹은 DOM 메뉴·팝업 체인을 그대로 둔다 — menu.rs 게이트).
pub struct HudPlugin;

impl Plugin for HudPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<super::daydial::DayDialState>()
            .add_systems(Startup, (super::ensure_theme, spawn_hud).chain())
            .add_systems(
                Update,
                (update_hud, update_notice, super::daydial::update_daydial).chain(),
            )
            // 진단 — ?tfdbg=uidbg: 모든 bevy_ui 노드의 화면좌표를 1회 덤프
            .add_systems(Update, uidbg_dump);
    }
}

/// HUD 루트 마커 — 세이프에어리어 패치 대상
#[derive(Component)]
pub struct HudRoot;

/// 갱신 대상 텍스트 마커들
#[derive(Component)]
pub struct HudClock;
#[derive(Component)]
pub struct HudWeather;
#[derive(Component)]
pub struct HudPop;
#[derive(Component)]
pub struct HudNet;
/// 재접속 공지 칩(레거시 notice — 좌측 노란 토스트 근사)
#[derive(Component)]
pub struct HudNotice;

/// 이전 프레임 문자열 캐시 — 변화 시에만 Text 재할당(리렌더 최소화)
#[derive(Resource, Default)]
pub struct LastHud {
    clock: String,
    weather: String,
    pop: String,
    net: String,
}

/// 재접속 공지 — 링크 단계 에지에서 문구 갱신, 복구 2s 후 소멸(FE notice 근사).
pub fn update_notice(
    net: Res<crate::net::NetState>,
    time: Res<Time>,
    mut q_chip: Query<&mut Visibility, With<HudNotice>>,
    mut q_text: Query<&mut Text, Without<HudNotice>>,
    children_q: Query<&Children, With<HudNotice>>,
    mut last: Local<Option<crate::net::NetStatus>>,
    mut clear_at: Local<Option<f32>>,
) {
    if *last != Some(net.status) {
        *last = Some(net.status);
        *clear_at = match net.status {
            crate::net::NetStatus::Connecting => None,
            crate::net::NetStatus::Reconnecting => None,
            crate::net::NetStatus::Connected => Some(time.elapsed_secs() + 2.0),
            crate::net::NetStatus::Idle => None,
        };
        if let Ok(mut v) = q_chip.single_mut() {
            *v = match net.status {
                crate::net::NetStatus::Connecting | crate::net::NetStatus::Reconnecting => {
                    Visibility::Visible
                }
                _ => Visibility::Hidden,
            };
        }
        if let Ok(children) = children_q.single()
            && let Some(&first) = children.first()
            && let Ok(mut t) = q_text.get_mut(first)
        {
            **t = match net.status {
                crate::net::NetStatus::Connecting => "연결 중…".into(),
                crate::net::NetStatus::Reconnecting => "재접속 중…".into(),
                _ => String::new(),
            };
        }
    }
    if let Some(at) = *clear_at
        && time.elapsed_secs() >= at
    {
        *clear_at = None;
        if let Ok(mut v) = q_chip.single_mut() {
            *v = Visibility::Hidden;
        }
    }
}

/// 칩 공통 노드(레거시 rounded bg-ink-800 칩)
fn chip() -> Node {
    Node {
        padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)),
        ..default()
    }
}

pub fn spawn_hud(
    mut commands: Commands,
    theme: Res<UiTheme>,
    safe: Res<crate::platform::safe_area::SafeArea>,
    window: Single<&Window>,
) {
    // 협소 화면 — DayDial 축소·관전 칩 생략(☰가 밀려나지 않게)
    let narrow = window.width() < 360.0;
    commands
        .spawn((
            HudRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(safe.top),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Px(30.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(5.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(theme::HUD_BG),
        ))
        .with_children(|p| {
            p.spawn((Text::new("Terrafolk"), theme.title(11.0)));
            p.spawn((HudClock, Text::new(""), theme.label(10.0)));
            super::daydial::mount(p, &theme, narrow);
            p.spawn((
                HudWeather,
                Text::new(""),
                theme.label(10.0),
                chip(),
                BackgroundColor(theme::INK800),
            ));
            p.spawn((
                HudPop,
                Text::new(""),
                theme.label(10.0),
                chip(),
                BackgroundColor(theme::INK800),
            ));
            p.spawn((
                HudNet,
                Text::new(""),
                theme.text(9.0, theme::ACCENT),
                chip(),
                BackgroundColor(theme::INK800),
            ));
            if !narrow {
                p.spawn((
                    Text::new("관전"),
                    theme.text(9.0, theme::INK400),
                    chip(),
                    BackgroundColor(theme::INK800),
                ));
            }
            // flex spacer + ☰ 메뉴 버튼 — 네이티브 전용(웹은 DOM 메뉴).
            #[cfg(not(target_arch = "wasm32"))]
            {
                p.spawn(Node {
                    flex_grow: 1.0,
                    ..default()
                });
                super::menu::spawn_menu_button(p, &theme);
            }
        });
    // 공지 칩 — HUD 아래 좌측(절대), 평소 숨김
    commands.spawn((
        HudNotice,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(safe.top + 32.0),
            left: Val::Px(8.0),
            padding: UiRect::axes(Val::Px(7.0), Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(Color::srgb(0.988, 0.816, 0.325)),
        Visibility::Hidden,
    ))
    .with_children(|n| {
        n.spawn((
            Text::new(""),
            theme.text(9.0, theme::INK950),
        ));
    });
    commands.init_resource::<LastHud>();
}

/// 태양력 달력(§17.37 직역) — 절대 일수 → (연, 월, 일). 1년 365일, 3월 시작.
pub fn date_of(day: u32) -> (u32, u32, u32, &'static str) {
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
    let season = if doy <= 92 {
        "spring"
    } else if doy <= 184 {
        "summer"
    } else if doy <= 275 {
        "autumn"
    } else {
        "winter"
    };
    (year, month, day_of_month, season)
}

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)] // Bevy 시스템 파라미터(텍스트 마커별 분리 쿼리)
pub fn update_hud(
    day: Res<DayTime>,
    weather: Res<WeatherState>,
    net: Res<NetState>,
    villagers: Query<(), With<Villager>>,
    mut q_clock: Query<
        &mut Text,
        (
            With<HudClock>,
            Without<HudWeather>,
            Without<HudPop>,
            Without<HudNet>,
        ),
    >,
    mut q_weather: Query<
        &mut Text,
        (
            With<HudWeather>,
            Without<HudClock>,
            Without<HudPop>,
            Without<HudNet>,
        ),
    >,
    mut q_pop: Query<
        &mut Text,
        (
            With<HudPop>,
            Without<HudClock>,
            Without<HudWeather>,
            Without<HudNet>,
        ),
    >,
    mut q_net: Query<
        &mut Text,
        (
            With<HudNet>,
            Without<HudClock>,
            Without<HudWeather>,
            Without<HudPop>,
        ),
    >,
    mut last: ResMut<LastHud>,
) {
    let (y, m, d, season) = date_of(day.day);
    let hh = day.hour.floor() as u32;
    let mm = ((day.hour - hh as f32) * 60.0).floor() as u32;
    let clock = format!(
        "{} {}년 {}월 {}일 {:02}:{:02}",
        theme::season_label(season),
        y,
        m,
        d,
        hh,
        mm
    );
    if clock != last.clock {
        last.clock = clock.clone();
        if let Ok(mut t) = q_clock.single_mut() {
            **t = clock;
        }
    }

    let weather_txt = format!(
        "{} {}",
        theme::weather_icon(&weather.kind),
        theme::weather_label(&weather.kind)
    );
    if weather_txt != last.weather {
        last.weather = weather_txt.clone();
        if let Ok(mut t) = q_weather.single_mut() {
            **t = weather_txt;
        }
    }

    let pop = format!("{}명", villagers.iter().count());
    if pop != last.pop {
        last.pop = pop.clone();
        if let Ok(mut t) = q_pop.single_mut() {
            **t = pop;
        }
    }

    // 링크 배지 — 라이브(네이티브 접속 스레드)·데모 구분(웹 worldsim 배지 대응)
    let net_txt = match net.status {
        NetStatus::Connected => "● LIVE".to_owned(),
        NetStatus::Connecting => "…연결".to_owned(),
        NetStatus::Reconnecting => "↻ 재접속".to_owned(),
        NetStatus::Idle => "○ 데모".to_owned(),
    };
    if net_txt != last.net {
        last.net = net_txt.clone();
        if let Ok(mut t) = q_net.single_mut() {
            **t = net_txt;
        }
    }
}

/// UI 노드 화면좌표 덤프(진단) — ?tfdbg=uidbg. bevy 0.19 bevy_ui는 전용
/// 트랜스폼 스택(UiGlobalTransform)을 쓴다 — GlobalTransform 쿼리는 0매치
/// (2026-10-25 "하늘 가발" 조사 중 실측).
#[allow(clippy::type_complexity)]
pub fn uidbg_dump(
    mut done: Local<bool>,
    q: Query<
        (
            &bevy::ui::UiGlobalTransform,
            Option<&bevy::ui::ComputedNode>,
            Option<&Text>,
        ),
        With<Node>,
    >,
) {
    if *done || !crate::net::dbg_gate::on("uidbg") {
        return;
    }
    *done = true;
    #[cfg(target_arch = "wasm32")]
    for (gt, cn, text) in &q {
        let p = gt.transform_point2(Vec2::ZERO);
        let (w, h) = cn.map(|c| (c.size.x, c.size.y)).unwrap_or((0.0, 0.0));
        let txt: String = text.map(|t| t.0.chars().take(14).collect()).unwrap_or_default();
        web_sys::console::log_1(
            &format!("UIDBG pos=({:.0},{:.0}) size=({:.0}x{:.0}) text={}", p.x, p.y, w, h, txt).into(),
        );
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = q;
}
