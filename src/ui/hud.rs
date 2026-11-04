//! 상단 HUD — 레거시 TopBar 직역(시계·날씨 칩·인원·관전 칩 + ☰ 자리).
//! ☰ 버튼 본체는 menu.rs가 소유(드롭다운과 한 루트).

use bevy::prelude::*;

use super::theme::{self, UiTheme};
use crate::net::{NetState, NetStatus};
use crate::world::daynight::DayTime;
use crate::world::precip::WeatherState;
use crate::world::villager::Villager;

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
        // 웹 — DOM 클러스터 예약 폭을 창 폭에 실시간 추종(회전·리사이즈)
        #[cfg(target_arch = "wasm32")]
        app.add_systems(Update, update_dom_reserve);
    }
}

/// HUD 레일 높이(px) — DOM 상단 컨트롤(웹 ☰ 클러스터)과 같은 스케일.
/// menu.rs 드롭다운·minimap.rs 상단 오프셋·mod.rs safe-area 패치가 참조.
pub const HUD_ROW_H: f32 = 38.0;

/// 웹 DOM 클러스터(모드 칩+☰) 예약 폭 — 모바일(≤600px, 칩 숨김)은 ☰만.
#[cfg(target_arch = "wasm32")]
fn dom_reserve_w(win: f32) -> f32 {
    if win <= 600.0 { 68.0 } else { 184.0 }
}

/// DOM 예약 스폰 마커 — update_dom_reserve가 창 폭 변화에 맞춘다(wasm).
#[derive(Component)]
#[cfg(target_arch = "wasm32")]
pub struct HudDomReserve;

/// 창 폭 → 예약 폭 재계산(wasm). CSS @media는 실시간으로 반응하므로
/// 스폰 시점 고정값은 회전·리사이즈에서 어긋난다(2026-11-04 리뷰 3).
#[cfg(target_arch = "wasm32")]
pub fn update_dom_reserve(
    window: Single<&Window>,
    mut q: Query<&mut Node, With<HudDomReserve>>,
) {
    let w = Val::Px(dom_reserve_w(window.width()));
    if let Ok(mut n) = q.single_mut()
        && n.width != w
    {
        n.width = w;
    }
}

/// 스폰 시점 화면 폭 기반 레이아웃 플래그 — 기존 narrow(<360) 관례 확장.
/// 칩 텍스트가 줄바꿈으로 찢어지던 것(2026-11-04 '관즐' 보고)을 폭 단계별
/// 정보 축약으로 해소한다(리사이즈 미추적 — narrow와 동일 계약).
#[derive(Resource)]
pub struct HudLayout {
    /// <420px — 타이틀·날씨 라벨 생략(우측 상태 칩 + ☰ 자리 확보)
    pub compact: bool,
    /// <340px — 날씨 칩 전체 생략
    pub ultra: bool,
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
    net_col: Color,
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

/// 칩 공통 노드(레거시 rounded bg-ink-800 칩) — 줄바꿈·압축 금지로 한 줄 유지
fn chip() -> Node {
    Node {
        padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
        flex_shrink: 0.0,
        border_radius: BorderRadius::all(Val::Px(5.0)),
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
    let compact = window.width() < 420.0;
    let ultra = window.width() < 340.0;
    commands.insert_resource(HudLayout { compact, ultra });
    commands
        .spawn((
            HudRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(safe.top),
                left: Val::Px(0.0),
                width: Val::Percent(100.0),
                height: Val::Px(HUD_ROW_H),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(5.0),
                padding: UiRect::horizontal(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(theme::HUD_BG),
        ))
        .with_children(|p| {
            if !compact {
                p.spawn((
                    Text::new("Terrafolk"),
                    theme.title(11.0),
                    Node {
                        flex_shrink: 0.0,
                        ..default()
                    },
                ));
            }
            p.spawn((HudClock, Text::new(""), theme.label(10.0), Node {
                flex_shrink: 0.0,
                ..default()
            }));
            super::daydial::mount(p, &theme, narrow || compact);
            // 좌측 정보군 ↔ 우측 상태 칩군 분리 — 상태(날씨·인구·링크)는
            // 우측 끝에 붙는 관전 HUD 관례(상용 시뮬 레이아웃).
            p.spawn(Node {
                flex_grow: 1.0,
                ..default()
            });
            if !ultra {
                p.spawn((
                    HudWeather,
                    Text::new(""),
                    theme.label(10.0),
                    chip(),
                    BackgroundColor(theme::INK800),
                ));
            }
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
            // 관전 칩은 네이티브 전용 — 웹은 DOM 모드 칩(관전/참여 실제
            // 상태 + 글래스 필)이 같은 정보를 주므로 중복 표시가 된다.
            #[cfg(not(target_arch = "wasm32"))]
            if !narrow {
                p.spawn((
                    Text::new("관전"),
                    theme.text(9.0, theme::INK400),
                    chip(),
                    BackgroundColor(theme::INK800),
                ));
            }
            // 웹 — 우측 DOM 클러스터(모드 칩+☰) 자리 예약. 칩이 ☰ 밑으로
            // 파고들어 '관즐'처럼 겹쳐 읽히던 것(2026-11-04)의 여백 확보.
            // 폭은 update_dom_reserve가 매 프레임 창 폭에 맞춘다(회전·
            // 리사이즈 — 스폰 시 1회면 CSS 미디어(≤600px 칩 숨김)와 어긋난다,
            // 리뷰 3). 데스크톱 184px는 영어 로케일 최장 칩('Participating')
            // 까지 포함한 실측 여유(리뷰 4).
            #[cfg(target_arch = "wasm32")]
            {
                p.spawn((
                    HudDomReserve,
                    Node {
                        width: Val::Px(dom_reserve_w(window.width())),
                        flex_shrink: 0.0,
                        ..default()
                    },
                ));
            }
            // ☰ 메뉴 버튼 — 네이티브 전용(웹은 DOM 메뉴).
            #[cfg(not(target_arch = "wasm32"))]
            {
                super::menu::spawn_menu_button(p, &theme);
            }
        });
    // 공지 칩 — 네이티브 전용(웹은 DOM 공지 바 tf-rest notifyBar + 모드 점이
    // 같은 정보를 준다 — bevy 칩이 웹 미니맵 좌상단과 겹치던 것, 리뷰 6).
    // HUD 아래 좌측(절대), 평소 숨김.
    #[cfg(not(target_arch = "wasm32"))]
    commands.spawn((
        HudNotice,
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(safe.top + HUD_ROW_H + 4.0),
            left: Val::Px(8.0),
            padding: UiRect::axes(Val::Px(7.0), Val::Px(4.0)),
            border_radius: BorderRadius::all(Val::Px(5.0)),
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
    layout: Res<HudLayout>,
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
        (&mut Text, &mut TextColor),
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
    // 협소 화면 시계 축약(리뷰 7) — 칩 전체가 no-shrink라 양보할 수 있는 건
    // 텍스트 길이뿐. compact: 계절+월일+시각(연도 생략), ultra: 시각만.
    // 레거시 FE도 모바일 축약 시계(tf-i18n clockCompact)를 썼다.
    let clock = if layout.ultra {
        format!("{:02}:{:02}", hh, mm)
    } else if layout.compact {
        format!(
            "{} {}월 {}일 {:02}:{:02}",
            theme::season_label(season),
            m,
            d,
            hh,
            mm
        )
    } else {
        format!(
            "{} {}년 {}월 {}일 {:02}:{:02}",
            theme::season_label(season),
            y,
            m,
            d,
            hh,
            mm
        )
    };
    if clock != last.clock {
        last.clock = clock.clone();
        if let Ok(mut t) = q_clock.single_mut() {
            **t = clock;
        }
    }

    // 협소 화면(compact) — 날씨는 아이콘만(칸 절약, 라벨 생략)
    let weather_txt = if layout.compact {
        theme::weather_icon(&weather.kind).to_owned()
    } else {
        format!(
            "{} {}",
            theme::weather_icon(&weather.kind),
            theme::weather_label(&weather.kind)
        )
    };
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

    // 링크 배지 — 라이브(네이티브 접속 스레드)·데모 구분(웹 worldsim 배지 대응).
    // 상태별 색: LIVE=그린(방송 관례), 연결/재접속=앰버/레드, 데모=뮤트.
    let (net_txt, net_col) = match net.status {
        NetStatus::Connected => ("● LIVE", Color::srgb(0.33, 0.85, 0.47)),
        NetStatus::Connecting => ("…연결", Color::srgb(1.0, 0.75, 0.28)),
        NetStatus::Reconnecting => ("↻ 재접속", Color::srgb(0.97, 0.44, 0.44)),
        NetStatus::Idle => ("○ 데모", theme::INK400),
    };
    if net_txt != last.net {
        last.net = net_txt.to_owned();
        if let Ok((mut t, _)) = q_net.single_mut() {
            **t = net_txt.to_owned();
        }
    }
    if net_col != last.net_col {
        last.net_col = net_col;
        if let Ok((_, mut c)) = q_net.single_mut() {
            c.0 = net_col;
        }
    }
}

/// UI 노드 화면좌표 덤프(진단) — ?tfdbg=uidbg. bevy 0.19 bevy_ui는 전용
/// 트랜스폼 스택(UiGlobalTransform)을 쓴다 — GlobalTransform 쿼리는 0매치
/// (2026-10-25 "하늘 가발" 조사 중 실측).
#[allow(clippy::type_complexity)]
pub fn uidbg_dump(
    mut done: Local<bool>,
    time: Res<Time>,
    q: Query<
        (
            &bevy::ui::UiGlobalTransform,
            Option<&bevy::ui::ComputedNode>,
            Option<&Text>,
        ),
        With<Node>,
    >,
) {
    // t>8s — 첫 프레임(레이아웃 전)에 덤프하면 전 노드 0×0으로 나와 무의미
    // 했다(2026-11-04 실측). 레이아웃 확정 후 1회 덤프.
    if *done || time.elapsed_secs() < 8.0 || !crate::net::dbg_gate::on("uidbg") {
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
