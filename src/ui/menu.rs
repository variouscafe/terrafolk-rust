//! ☰ 게임 메뉴 — 레거시 TopBar 드롭다운 직역(주민·설정. 나머지 항목은
//! 라이브 서버 연동 기능이 구현될 때 추가) + 설정 팝업(시간 배속).

use bevy::input::keyboard::KeyboardInput;
use bevy::prelude::*;

use super::theme::{self, UiTheme};
use super::{CloseButton, Popup, UiState};
use crate::world::daynight::DayTime;

#[derive(Component)]
pub struct MenuButton;

/// 드롭다운 루트(열림/닫힘 = Visibility)
#[derive(Component)]
pub struct MenuDropdown;

/// 메뉴 항목이 여는 팝업
#[derive(Component)]
pub struct MenuOpenPopup(Popup);

/// 설정 팝업 루트
#[derive(Component)]
pub struct SettingsPopup;

/// 시간 배속 토글 버튼·라벨
#[derive(Component)]
pub struct TimeScaleButton;
#[derive(Component)]
pub struct TimeScaleLabel;

/// HUD 행 안에 ☰ 버튼 스폰(hud.rs가 자리를 내준다)
pub fn spawn_menu_button(p: &mut ChildSpawnerCommands, theme: &UiTheme) {
    p.spawn((
        MenuButton,
        Button,
        Node {
            padding: UiRect::axes(Val::Px(7.0), Val::Px(3.0)),
            ..default()
        },
        BackgroundColor(theme::INK800),
        Interaction::default(),
    ))
    .with_children(|b| {
        b.spawn((Text::new("☰"), theme.title(12.0)));
    });
}

/// 드롭다운·설정 팝업 스폰(부팅 1회, 닫힘 상태)
pub fn spawn_menu(
    mut commands: Commands,
    theme: Res<UiTheme>,
    safe: Res<crate::platform::safe_area::SafeArea>,
) {
    // ── ☰ 드롭다운 — HUD 우측 아래로 열린다
    commands
        .spawn((
            MenuDropdown,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(safe.top + 30.0),
                right: Val::Px(6.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(4.0)),
                row_gap: Val::Px(2.0),
                ..default()
            },
            BackgroundColor(theme::INK900),
            Visibility::Hidden,
        ))
        .with_children(|d| {
            menu_item(d, &theme, "주민", Popup::Chars);
            menu_item(d, &theme, "설정", Popup::Settings);
        });
    // ── 설정 팝업(중앙 모달 셸) — 백드롭 탭으로도 닫힌다
    commands
        .spawn((
            SettingsPopup,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            Visibility::Hidden,
        ))
        .observe(super::close_on_backdrop)
        .with_children(|m| {
            m.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(14.0)),
                    row_gap: Val::Px(10.0),
                    min_width: Val::Px(190.0),
                    ..default()
                },
                BackgroundColor(theme::SURFACE),
            ))
            .with_children(|card| {
                card.spawn((Text::new("설정"), theme.title(13.0)));
                card.spawn((
                    TimeScaleButton,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(theme::INK800),
                    Interaction::default(),
                ))
                .with_children(|b| {
                    b.spawn((TimeScaleLabel, Text::new("시간 배속 ×1"), theme.label(11.0)));
                });
                card.spawn((
                    CloseButton,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(theme::INK700),
                    Interaction::default(),
                ))
                .with_children(|b| {
                    b.spawn((Text::new("닫기"), theme.label(11.0)));
                });
            });
        });
}

fn menu_item(d: &mut ChildSpawnerCommands, theme: &UiTheme, label: &str, popup: Popup) {
    d.spawn((
        MenuOpenPopup(popup),
        Button,
        Node {
            padding: UiRect::axes(Val::Px(12.0), Val::Px(7.0)),
            ..default()
        },
        BackgroundColor(Color::NONE),
        Interaction::default(),
    ))
    .with_children(|b| {
        b.spawn((Text::new(label), theme.label(12.0)));
    });
}

/// 메뉴 상호작용 — ☰ 토글, 항목 → 팝업, 닫기, 시간 배속 토글.
/// 버튼 종류별 쿼리 대신 하나의 통합 쿼리(충돌 필터 지옥 회피).
#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)] // Bevy 시스템 파라미터 — 입력·시간·UI 상태를 한 프레임 안에서 순서 보정
pub fn interact(
    mut state: ResMut<UiState>,
    mut day: ResMut<DayTime>,
    keys: Res<ButtonInput<KeyCode>>,
    mut key_msgs: MessageReader<KeyboardInput>,
    q_btns: Query<
        (
            &Interaction,
            Option<&MenuOpenPopup>,
            Has<CloseButton>,
            Has<TimeScaleButton>,
            Has<MenuButton>,
        ),
        With<Button>,
    >,
    mut q_dropdown: Query<&mut Visibility, (With<MenuDropdown>, Without<SettingsPopup>)>,
    mut q_settings: Query<&mut Visibility, (With<SettingsPopup>, Without<MenuDropdown>)>,
    mut q_label: Query<&mut Text, With<TimeScaleLabel>>,
) {
    // Escape/안드로이드 뒤로가기 — 열린 UI부터 닫는다(표준: 팝업→메뉴→선택 해제).
    if crate::platform::back_pressed(&keys) || crate::platform::back_in_messages(&mut key_msgs) {
        if state.popup.take().is_some() {
            // 팝업만 닫고 끝
        } else {
            state.menu_open = false;
        }
    }
    for (i, popup, close, timescale, menu_btn) in q_btns.iter() {
        if *i != Interaction::Pressed {
            continue;
        }
        if menu_btn {
            state.menu_open = !state.menu_open;
        } else if let Some(popup) = popup {
            state.popup = Some(popup.0);
            state.menu_open = false;
        } else if close {
            state.popup = None;
        } else if timescale {
            // 시간 배속 ×1 ↔ ×40(레거시 T키와 동일)
            day.time_scale = if day.time_scale == 1.0 { 40.0 } else { 1.0 };
        }
    }
    if let Ok(mut label) = q_label.single_mut() {
        **label = if day.time_scale == 1.0 {
            "시간 배속 ×1".into()
        } else {
            "시간 배속 ×40".into()
        };
    }

    // 표시 상태 반영
    if let Ok(mut v) = q_dropdown.single_mut() {
        *v = if state.menu_open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if let Ok(mut v) = q_settings.single_mut() {
        *v = if state.popup == Some(Popup::Settings) {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
