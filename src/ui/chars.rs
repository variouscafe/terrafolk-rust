//! 주민 팝업(목록) + 선택 패널 — 레거시 CharsPopup 직역(모바일 간결판).
//! 목록 탭 → 기존 pick::Selection 체인 재사용(링+카메라 추적 자동).

use bevy::prelude::*;

use super::theme::{self, UiTheme};
use super::{CloseButton, Popup, UiState};
use crate::net::apply::LiveAgentTag;
use crate::world::cam_follow::CamFollow;
use crate::world::pick::Selection;
use crate::world::villager::Villager;

#[derive(Component)]
pub struct CharsPopupRoot;

/// 목록 컨테이너(항목 스폰·철거 대상)
#[derive(Component)]
pub struct CharsList;

/// 목록 항목 — (주민 엔티티, 선택 체인용 id)
#[derive(Component)]
pub struct CharRow(Entity, String);

/// 하단 선택 패널 마커들
#[derive(Component)]
pub struct SelPanel;
#[derive(Component)]
pub struct SelName;
#[derive(Component)]
pub struct SelStatus;
#[derive(Component)]
pub struct SelFollowChip;
/// 선택 해제 버튼
#[derive(Component)]
pub struct SelClearButton;

/// 목록 재구성 판정 서명(이름·상태·수를 문자열 하나로)
#[derive(Resource, Default)]
pub struct ListSig(String);

pub fn spawn(
    mut commands: Commands,
    theme: Res<UiTheme>,
    safe: Res<crate::platform::safe_area::SafeArea>,
) {
    // ── 주민 팝업(모달)
    commands
        .spawn((
            CharsPopupRoot,
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
                    padding: UiRect::all(Val::Px(12.0)),
                    row_gap: Val::Px(8.0),
                    min_width: Val::Px(230.0),
                    max_height: Val::Px(340.0),
                    ..default()
                },
                BackgroundColor(theme::SURFACE),
            ))
            .with_children(|card| {
                card.spawn((Text::new("주민"), theme.title(13.0)));
                card.spawn((
                    CharsList,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(4.0),
                        ..default()
                    },
                ));
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
    // ── 하단 선택 패널(선택 시 등장)
    commands
        .spawn((
            SelPanel,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(safe.bottom + 8.0),
                left: Val::Px(8.0),
                right: Val::Px(8.0),
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(8.0),
                padding: UiRect::all(Val::Px(10.0)),
                ..default()
            },
            BackgroundColor(theme::HUD_BG),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            p.spawn((Node {
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(2.0),
                flex_grow: 1.0,
                ..default()
            },))
                .with_children(|info| {
                    info.spawn((SelName, Text::new(""), theme.title(12.0)));
                    info.spawn((SelStatus, Text::new(""), theme.label(10.0)));
                });
            p.spawn((
                SelFollowChip,
                Text::new("추적 중"),
                theme.text(9.0, theme::INK950),
                Node {
                    padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(theme::ACCENT),
            ));
            p.spawn((
                SelClearButton,
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(9.0), Val::Px(5.0)),
                    ..default()
                },
                BackgroundColor(theme::INK700),
                Interaction::default(),
            ))
            .observe(on_clear_click) // 폴링 아닌 확정 클릭(빠른 탭 유실 방지)
            .with_children(|b| {
                b.spawn((Text::new("해제"), theme.label(10.0)));
            });
        });
    commands.init_resource::<ListSig>();
}

/// 목록 갱신 — 서명(이름·상태·수)이 바뀔 때만 항목 재구성(멱등).
/// 주의: 목록 컨테이너는 자식이 없으면 Children 컴포넌트가 없다 —
/// 쿼리는 Entity로만 잡고 자식 조작은 despawn_descendants/add_child로.
#[allow(clippy::type_complexity)]
pub fn refresh_list(
    state: Res<UiState>,
    theme: Res<UiTheme>,
    villagers: Query<(Entity, &Villager, Option<&LiveAgentTag>)>,
    mut q_root: Query<&mut Visibility, (With<CharsPopupRoot>, Without<SelPanel>)>,
    q_list: Query<(Entity, Option<&Children>), (With<CharsList>, Without<CharsPopupRoot>)>,
    mut commands: Commands,
    mut sig: ResMut<ListSig>,
) {
    let visible = state.popup == Some(Popup::Chars);
    if let Ok(mut v) = q_root.single_mut() {
        *v = if visible {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if !visible {
        return;
    }

    // 서명 계산 + 행 데이터 수집(id·이름·상태 — 이름순 정렬)
    let mut sig_new = String::new();
    let mut rows: Vec<(Entity, String, String, String)> = villagers
        .iter()
        .map(|(e, v, tag)| {
            let id = tag
                .map(|t| t.0.clone())
                .unwrap_or_else(|| format!("demo:{}", e.index()));
            sig_new.push_str(&id);
            sig_new.push('|');
            sig_new.push_str(&v.name);
            sig_new.push('|');
            sig_new.push_str(&v.status);
            sig_new.push('\n');
            (e, id, v.name.clone(), v.status.clone())
        })
        .collect();
    rows.sort_by(|a, b| a.2.cmp(&b.2));
    if sig_new == sig.0 {
        return; // 내용 동일 — 재구성 생략
    }
    sig.0 = sig_new;

    let Ok((list_e, children)) = q_list.single() else {
        return;
    };
    // 기존 행 철거 — despawn은 자손(행 텍스트)까지 재귀 제거(0.19 계약)
    let stale: Vec<Entity> = children.map(|c| c.to_vec()).unwrap_or_default();
    for c in stale {
        commands.entity(c).despawn();
    }
    for (e, id, name, status) in rows {
        let row = commands
            .spawn((
                CharRow(e, id),
                Button,
                Node {
                    padding: UiRect::axes(Val::Px(9.0), Val::Px(6.0)),
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Baseline,
                    ..default()
                },
                BackgroundColor(theme::INK800),
                Interaction::default(),
            ))
            .with_children(|r| {
                r.spawn((Text::new(name), theme.label(11.0)));
                r.spawn((Text::new(status), theme.text(9.0, theme::INK400)));
            })
            .observe(on_row_click)
            .id();
        commands.entity(list_e).add_child(row);
    }
}

/// 행 클릭 옵저버 — Interaction 폴링은 빠른 탭에서 Pressed 창을 놓친다
/// (에뮬레이터 tap=눌림+해제 동일 프레임). Pointer<Click>은 확정 이벤트.
fn on_row_click(
    trigger: On<Pointer<Click>>,
    mut sel: ResMut<Selection>,
    mut state: ResMut<UiState>,
    rows: Query<&CharRow>,
) {
    let Ok(row) = rows.get(trigger.entity) else {
        return;
    };
    sel.id = Some(row.1.clone());
    sel.entity = Some(row.0);
    sel.epoch = sel.epoch.wrapping_add(1);
    state.popup = None; // 탭 즉시 닫고 카메라가 주민에게
}

/// 하단 패널 해제 칩 — 선택 해제. 행 선택과 동일하게 확정 클릭 이벤트.
fn on_clear_click(_: On<Pointer<Click>>, mut sel: ResMut<Selection>) {
    sel.id = None;
    sel.entity = None;
}

/// 하단 선택 패널 갱신 — 이름·상태·추적 칩 표시(선택 해제는 위 시스템이).
#[allow(clippy::type_complexity)]
pub fn refresh_sel_panel(
    sel: Res<Selection>,
    follow: Res<CamFollow>,
    villagers: Query<&Villager>,
    mut q_panel: Query<
        &mut Visibility,
        (
            With<SelPanel>,
            Without<CharsPopupRoot>,
            Without<SelFollowChip>,
        ),
    >,
    mut q_name: Query<&mut Text, (With<SelName>, Without<SelStatus>)>,
    mut q_status: Query<&mut Text, (With<SelStatus>, Without<SelName>)>,
    mut q_chip: Query<&mut Visibility, (With<SelFollowChip>, Without<SelPanel>)>,
) {
    let Some(entity) = sel.entity.filter(|e| villagers.get(*e).is_ok()) else {
        if let Ok(mut v) = q_panel.single_mut() {
            *v = Visibility::Hidden;
        }
        return;
    };
    let v = villagers.get(entity).expect("방금 확인함");
    if let Ok(mut p) = q_panel.single_mut() {
        *p = Visibility::Visible;
    }
    if let Ok(mut t) = q_name.single_mut()
        && **t != v.name
    {
        **t = v.name.clone();
    }
    if let Ok(mut t) = q_status.single_mut() {
        let label = theme::status_label(&v.status);
        if **t != label {
            **t = label;
        }
    }
    if let Ok(mut c) = q_chip.single_mut() {
        *c = if follow.active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
