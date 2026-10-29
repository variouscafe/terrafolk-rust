//! 월드 앵커 오버레이(네이티브) — tf-world-ui.js와 동형 아키텍처의 bevy_ui 판.
//! 주민 머리 앵커(지면+1.85m)를 world_to_viewport로 매 프레임 투영해 절대 노드를
//! 배치한다: 이름 라벨(저줌<선택 외 숨김) · 활동 배지(진행 링+아이콘) · 수면 zZz ·
//! 욕구 경고 ❗ · 말풍선(4.1s). 전 노드 Pickable::IGNORE — 월드 탭을 훔치지 않는다.
//! 하위 슬롯은 SlotMark 열거형 하나로 식별 — 시스템마다 동일 컴포넌트(&mut
//! Visibility·&mut Text)를 보는 쿼리가 여럿이면 B0001 충돌이라 단일 쿼리로 통일.

use bevy::picking::Pickable;
use bevy::prelude::*;

use super::theme::UiTheme;
use super::world_overlay_badges::BadgeCanvas;
use crate::net::apply::LiveAgentTag;
use crate::net::feed::ChatLog;
use crate::world::camera_rig::CameraRig;
use crate::world::pick::Selection;
use crate::world::agent::Villager;

/// 주민 상단 앵커 높이(m) — overlay.rs ANCHOR_H와 동일값(네이티브 재정의).
const ANCHOR_H: f32 = 1.85;
/// 이 줌보다 멀면 이름 숨김(tf-world-ui LABEL_ZOOM_MAX — FE zoom<0.85 상당).
const LABEL_ZOOM_MAX: f32 = 1.9;
/// 화면 밖 판정 마진(px) — tf-world-ui와 동일.
const MARGIN_X: f32 = 80.0;
const MARGIN_Y: f32 = 60.0;
/// 버블 수명 3.4s + 페이드 0.7s(FE 트윈 동일).
const BUBBLE_LIFE: f32 = 3.4;
const BUBBLE_FADE: f32 = 0.7;
/// 컬럼 폭 — 버블 max-width와 동일.
const COL_W: f32 = 190.0;
/// 라벨 텍스트색(크림) / 선택·사용자 발화 강조(금색) — tf-ui.css 패리티.
pub(crate) const NAME_CREAM: Color = Color::srgb(0.961, 0.949, 0.910);
pub(crate) const NAME_GOLD: Color = Color::srgb(1.0, 0.914, 0.690);

#[derive(Component)]
pub struct OverlayRoot;

/// 컬럼 하위 슬롯 종류 — 쿼리 통일용.
#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Slot {
    Name,
    Row,
    Zzz,
    Badge,
    Alert,
    Bubble,
    Canvas,
    NameText,
    IconText,
    BubbleText,
}

#[derive(Component)]
pub struct SlotMark(pub(crate) Slot);

/// 주민별 앵커 컬럼 — 하위 슬롯 엔티티를 직접 들어 Children 순회를 없앤다.
#[derive(Component)]
pub struct AnchorCol {
    pub id: String,
    pub target: Entity,
    pub(crate) name: Entity,
    pub(crate) row: Entity,
    bubble: Entity,
    bubble_text: Entity,
    pub(crate) canvas: Entity,
}


/// 버블 수명·본문(중복 스킵 판정).
#[derive(Component)]
pub struct BubbleNode {
    text: String,
    life: f32,
}

pub fn spawn_overlay(mut commands: Commands) {
    commands.spawn((
        OverlayRoot,
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
    ));
}

/// 주민 증감 동기 — 떠난 주민 컬럼 철거, 신규 스폰(라벨은 데모 주민도).
#[allow(clippy::type_complexity)]
pub fn sync_overlay_nodes(
    mut commands: Commands,
    theme: Res<UiTheme>,
    q_root: Query<Entity, (With<OverlayRoot>, Without<AnchorCol>)>,
    villagers: Query<(Entity, &Villager, Option<&LiveAgentTag>), Without<AnchorCol>>,
    mut q_cols: Query<(Entity, &AnchorCol)>,
) {
    let Ok(root) = q_root.single() else {
        return;
    };
    for (e, col) in &mut q_cols {
        if villagers.get(col.target).is_err() {
            commands.entity(e).despawn();
        }
    }
    for (e, v, tag) in &villagers {
        let id = tag
            .map(|t| t.0.clone())
            .unwrap_or_else(|| format!("demo:{}", e.index()));
        // 버블 슬롯(첫 자리) — 비었으면 숨김
        let bubble_text = commands
            .spawn((
                SlotMark(Slot::BubbleText),
                Pickable::IGNORE,
                Text::new(""),
                theme.text(9.0, NAME_CREAM),
            ))
            .id();
        let bubble = commands
            .spawn((
                SlotMark(Slot::Bubble),
                BubbleNode {
                    text: String::new(),
                    life: 0.0,
                },
                Pickable::IGNORE,
                Node {
                    max_width: Val::Px(COL_W),
                    padding: UiRect::axes(Val::Px(5.0), Val::Px(3.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8)),
                Visibility::Hidden,
            ))
            .add_child(bubble_text)
            .id();
        // 이름 라벨
        let name_text = commands
            .spawn((
                SlotMark(Slot::NameText),
                Pickable::IGNORE,
                Text::new(v.name.clone()),
                theme.text(9.0, NAME_CREAM),
            ))
            .id();
        let name = commands
            .spawn((
                SlotMark(Slot::Name),
                Pickable::IGNORE,
                Node {
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(1.0)),
                    ..default()
                },
                BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
            ))
            .add_child(name_text)
            .id();
        // 행 — zZz · 배지(캔버스+아이콘) · ❗
        let zzz = commands
            .spawn((
                SlotMark(Slot::Zzz),
                Pickable::IGNORE,
                Text::new("z"),
                theme.text(12.0, Color::srgb(0.812, 0.890, 1.0)),
                Visibility::Hidden,
            ))
            .id();
        let canvas = commands
            .spawn((
                SlotMark(Slot::Canvas),
                BadgeCanvas {
                    handle: None,
                    last_width: u32::MAX,
                },
                Pickable::IGNORE,
                ImageNode::default(),
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(30.0),
                    height: Val::Px(30.0),
                    ..default()
                },
            ))
            .id();
        let icon = commands
            .spawn((
                SlotMark(Slot::IconText),
                Pickable::IGNORE,
                Text::new(""),
                theme.text(11.0, NAME_CREAM),
            ))
            .id();
        let badge = commands
            .spawn((
                SlotMark(Slot::Badge),
                Pickable::IGNORE,
                Node {
                    width: Val::Px(30.0),
                    height: Val::Px(30.0),
                    ..default()
                },
                Visibility::Hidden,
            ))
            .add_child(canvas)
            .add_child(icon)
            .id();
        let alert = commands
            .spawn((
                SlotMark(Slot::Alert),
                Pickable::IGNORE,
                Text::new("!"),
                theme.text(13.0, Color::srgb(1.0, 0.69, 0.627)),
                Visibility::Hidden,
            ))
            .id();
        let row = commands
            .spawn((
                SlotMark(Slot::Row),
                Pickable::IGNORE,
                Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(3.0),
                    ..default()
                },
            ))
            .add_child(zzz)
            .add_child(badge)
            .add_child(alert)
            .id();
        let col_entity = commands
            .spawn((
                AnchorCol {
                    id,
                    target: e,
                    name,
                    row,
                    bubble,
                    bubble_text,
                    canvas,
                },
                Pickable::IGNORE,
                Node {
                    position_type: PositionType::Absolute,
                    width: Val::Px(COL_W),
                    flex_direction: FlexDirection::Column,
                    align_items: AlignItems::Center,
                    row_gap: Val::Px(2.0),
                    ..default()
                },
            ))
            .add_child(bubble)
            .add_child(name)
            .add_child(row)
            .id();
        commands.entity(root).add_child(col_entity);
    }
}

/// 말풍선 — ChatLog watermark 뒤 새 행만. 60자, 사용자 발화 ⟪이름⟫ 전체 금색,
/// 동일 텍스트 재수신 스킵(에코 확정 재방송 방지). 수명 3.4s + 페이드 0.7s.
#[allow(clippy::type_complexity)]
pub fn update_bubbles(
    time: Res<Time>,
    mut chat: ResMut<ChatLog>,
    mut q_cols: Query<&mut AnchorCol>,
    villagers: Query<&Villager>,
    mut q_bubble: Query<(&mut BubbleNode, &mut Visibility, &mut BackgroundColor), Without<AnchorCol>>,
    mut q_bubble_text: Query<(&mut Text, &mut TextColor), Without<AnchorCol>>,
) {
    if chat.seen < chat.rows.len() {
        let fresh: Vec<_> = chat.rows.iter().skip(chat.seen).cloned().collect();
        chat.seen = chat.rows.len();
        for row in fresh {
            for col in &mut q_cols {
                if col.id != row.agent_id {
                    continue;
                }
                // 화자 판정 — '@' 포함·'user'·주민명 불일치(tf-world-ui 동일 기준)
                let is_user = villagers.get(col.target).is_ok_and(|v| {
                    row.speaker.contains('@')
                        || row.speaker == "user"
                        || row.speaker != v.name
                });
                let tag = if is_user {
                    format!("⟪{}⟫ ", row.speaker.split('@').next().unwrap_or(""))
                } else {
                    String::new()
                };
                let body: String = if row.text.is_empty() {
                    row.text_en.clone().unwrap_or_default()
                } else {
                    row.text.clone()
                };
                let text = format!("{tag}{}", body.chars().take(60).collect::<String>());
                if let Ok((mut bubble, mut vis, mut bg)) = q_bubble.get_mut(col.bubble) {
                    if bubble.text == text {
                        break; // 무변화 — 다음 행으로
                    }
                    bubble.text = text.clone();
                    bubble.life = BUBBLE_LIFE + BUBBLE_FADE;
                    *vis = Visibility::Visible;
                    *bg = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.8));
                    if let Ok((mut t, mut color)) = q_bubble_text.get_mut(col.bubble_text) {
                        **t = text;
                        color.0 = if is_user { NAME_GOLD } else { NAME_CREAM };
                    }
                }
                break;
            }
        }
    }
    // 수명 감산 + 마지막 0.7s 페이드
    let dt = time.delta_secs();
    for col in &mut q_cols {
        if let Ok((mut bubble, mut vis, mut bg)) = q_bubble.get_mut(col.bubble)
            && bubble.life > 0.0
        {
            bubble.life -= dt;
            if bubble.life <= 0.0 {
                *vis = Visibility::Hidden;
                bubble.text.clear();
            } else if bubble.life < BUBBLE_FADE {
                let a = (bubble.life / BUBBLE_FADE).clamp(0.0, 1.0) * 0.8;
                *bg = BackgroundColor(Color::srgba(0.0, 0.0, 0.0, a));
            }
        }
    }
}

/// 투영 배치 — 앵커(머리+1.85m)를 화면 좌표로, 컬럼을 위로 쌓는다.
/// 저줌 라벨 숨김·화면 밖 마진·밀집 스태킹(17px×4단)은 tf-world-ui 직역.
#[allow(clippy::type_complexity)]
pub fn place_anchors(
    q_cam: Single<(&Camera, &GlobalTransform), With<Camera3d>>,
    window: Single<&Window>,
    rig: Res<CameraRig>,
    sel: Res<Selection>,
    mut q_cols: Query<(&AnchorCol, &mut Node, &mut Visibility)>,
    targets: Query<&Transform, With<Villager>>,
    mut q_slots: Query<(&SlotMark, &mut Visibility), Without<AnchorCol>>,
) {
    let (cam, gt) = q_cam.into_inner();
    let show_labels = rig.zoom <= LABEL_ZOOM_MAX;
    let (ww, wh) = (window.width(), window.height());
    struct Placed {
        target: Entity,
        x: f32,
        y: f32,
        height: f32,
        offscreen: bool,
        label_on: bool,
    }
    let mut placed: Vec<Placed> = vec![];
    for (col, _, mut vis) in &mut q_cols {
        let Ok(tr) = targets.get(col.target) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let anchor = tr.translation + Vec3::Y * ANCHOR_H;
        let Ok(p) = cam.world_to_viewport(gt, anchor) else {
            *vis = Visibility::Hidden;
            continue;
        };
        let offscreen = p.x < -MARGIN_X
            || p.y < -MARGIN_Y
            || p.x > ww + MARGIN_X
            || p.y > wh + MARGIN_Y;
        let selected = sel.id.as_deref() == Some(col.id.as_str());
        let label_on = show_labels || selected;
        // 컬럼별 슬롯 상태(버블·행) — 높이 근사에 반영
        let bubble_on = q_slots
            .get(col.bubble)
            .is_ok_and(|(_, v)| *v == Visibility::Visible);
        let row_on = q_slots
            .get(col.row)
            .is_ok_and(|(_, v)| *v == Visibility::Visible);
        let mut height = 2.0;
        if label_on {
            height += 15.0;
        }
        if row_on {
            height += 30.0;
        }
        if bubble_on {
            height += 32.0;
        }
        placed.push(Placed {
            target: col.target,
            x: p.x,
            y: p.y,
            height,
            offscreen,
            label_on,
        });
    }
    // 이름 스태킹 — rect(110×16) 겹침 → 위로 17px 시프트(최대 4단)
    for i in 0..placed.len() {
        if !placed[i].label_on {
            continue;
        }
        let mut shift = 0.0f32;
        for _ in 0..4 {
            let collided = placed.iter().enumerate().any(|(j, p)| {
                j != i
                    && p.label_on
                    && (placed[i].x - p.x).abs() < 110.0
                    && ((placed[i].y - shift) - p.y).abs() < 16.0
            });
            if !collided {
                break;
            }
            shift += 17.0;
        }
        placed[i].y -= shift;
    }
    for (col, mut node, mut vis) in &mut q_cols {
        let Some(p) = placed.iter().find(|p| p.target == col.target) else {
            continue;
        };
        *vis = if p.offscreen {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
        // 앵커가 컬럼 바닥 — 쌓인 높이만큼 위로
        node.left = Val::Px(p.x - COL_W * 0.5);
        node.top = Val::Px(p.y - p.height);
        // 라벨 저줌 숨김(선택 주민 유지)
        if let Ok((mark, mut nv)) = q_slots.get_mut(col.name)
            && mark.0 == Slot::Name
        {
            *nv = if p.label_on {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}
