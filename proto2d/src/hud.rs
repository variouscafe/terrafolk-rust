//! HUD — 좌상단 조작 힌트, 좌하단 시계(게임 시각).
use bevy::prelude::*;
use bevy::text::{FontSize, FontSource};
use crate::daynight::GameTime;

#[derive(Component)]
pub struct ClockText;

pub fn spawn(mut commands: Commands, assets: Res<AssetServer>) {
    let font = assets.load("fonts/NotoSansKR-Regular.otf");
    let panel = |top: Val, bottom: Val| {
        (
            Node {
                position_type: PositionType::Absolute,
                top,
                bottom,
                left: Val::Px(10.0),
                padding: UiRect::all(Val::Px(8.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.06, 0.05, 0.03, 0.42)),
        )
    };
    let text = |s: &str, size: f32| {
        (
            Text::new(s),
            TextFont {
                font: FontSource::Handle(font.clone()),
                font_size: FontSize::Px(size),
                ..default()
            },
            TextColor(Color::srgb(0.96, 0.93, 0.84)),
        )
    };

    commands
        .spawn(panel(Val::Px(10.0), Val::Auto))
        .with_child(text(
            "Terrafolk 2D proto — 드래그 팬 · 휠 줌 · 주민 클릭 선택/추적",
            15.0,
        ));
    commands
        .spawn(panel(Val::Auto, Val::Px(10.0)))
        .with_child((
            ClockText,
            Text::new("…"),
            TextFont {
                font: FontSource::Handle(font),
                font_size: FontSize::Px(17.0),
                ..default()
            },
            TextColor(Color::srgb(1.0, 0.96, 0.82)),
        ));
}

pub fn clock_update(
    gt: Res<GameTime>,
    net: Option<Res<crate::net::NetState>>,
    mut q: Query<&mut Text, With<ClockText>>,
) {
    let net_changed = net.as_ref().map(|n| n.is_changed()).unwrap_or(false);
    if !gt.is_changed() && !net_changed {
        return;
    }
    let minutes = (gt.t * 24.0 * 60.0) as u32;
    let (hh, mm) = (minutes / 60, minutes % 60);
    // 접속 배지 — 라이브면 ● LIVE, 접속 중이면 ◌, 데모면 데모 표기
    let badge = match net.as_ref().map(|n| n.status) {
        Some(crate::net::NetStatus::Connected) => " · ● LIVE",
        Some(crate::net::NetStatus::Connecting) => " · ◌ 접속중",
        Some(crate::net::NetStatus::Reconnecting) => " · ◌ 재접속",
        _ => " · 데모",
    };
    if let Ok(mut t) = q.single_mut() {
        **t = format!("{}일차 {:02}:{:02}{}", gt.day + 1, hh, mm, badge).into();
    }
}
