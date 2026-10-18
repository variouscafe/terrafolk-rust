//! 자원 팝업 — civ.rs에서 분리(500줄 규칙). 라이브 frame.resources 종별 집계,
//! 데모는 WorldData 장식 프롭 집계. 서명 변화 시 본문 재구성.

use bevy::prelude::*;

use super::civ::{CivSigs, TextLine, clear_children};
use super::{Popup, UiState};
use crate::net::NetState;
use crate::world::data::WorldData;

#[derive(Component)]
pub struct ResPopupRoot;
#[derive(Component)]
pub struct ResBody;

pub fn spawn(mut commands: Commands, theme: Res<super::theme::UiTheme>) {
    commands
        .spawn((
            ResPopupRoot,
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
                ResBody,
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(14.0)),
                    row_gap: Val::Px(6.0),
                    min_width: Val::Px(230.0),
                    max_height: Val::Px(360.0),
                    overflow: Overflow::clip_y(),
                    ..default()
                },
                BackgroundColor(super::theme::SURFACE),
            ))
            .with_children(|card| {
                card.spawn((Text::new("자원"), theme.title(13.0)));
            });
        });
}

/// 자원 종류 → 한국어 라벨(레거시 res.kind.* 직역).
pub fn res_kind_label(kind: &str) -> &str {
    match kind {
        "tree" => "나무",
        "plant" => "식물",
        "gold_vein" => "광맥",
        "animal" => "동물",
        "water" => "물",
        "hut" => "오두막",
        "campfire" => "모닥불",
        "grave" => "무덤",
        "relic" => "발명 흔적",
        other => other,
    }
}

/// 자원 집계 — (종류, 총수, "종 ×n" 목록) 정렬된 행.
fn res_rows(net: &NetState, world: &WorldData) -> Vec<(String, usize, String)> {
    use std::collections::BTreeMap;
    let mut groups: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    let mut feed = |kind: String, species: String| {
        let g = groups.entry(kind).or_default();
        *g.entry(species).or_insert(0) += 1;
    };
    if let Some(frame) = net.frame.as_ref() {
        for r in &frame.resources {
            feed(r.kind.clone(), r.species.clone());
        }
    } else {
        for p in world.decorative_props() {
            let (kind, species) = prop_kind_str(p.kind);
            feed(kind.into(), species.into());
        }
    }
    groups
        .into_iter()
        .map(|(kind, species)| {
            let total: usize = species.values().sum();
            let chips = species
                .iter()
                .map(|(sp, n)| format!("{sp} ×{n}"))
                .collect::<Vec<_>>()
                .join(" · ");
            (kind, total, chips)
        })
        .collect()
}

fn prop_kind_str(p: crate::world::data::PropKind) -> (&'static str, &'static str) {
    use crate::world::data::PropKind::*;
    match p {
        TreeBroadleaf => ("tree", "활엽수"),
        TreePine => ("tree", "침엽수"),
        TreePalm => ("tree", "야자"),
        TreeSapling => ("tree", "묘목"),
        TreeDead => ("tree", "고사목"),
        BerryBush => ("plant", "열매 덤불"),
        Boulder => ("plant", "바위"),
        GoldVein => ("gold_vein", "금"),
        FlowerClump => ("plant", "꽃"),
        GrassClump => ("plant", "풀"),
        Hut => ("hut", "오두막"),
        StorageHut => ("hut", "저장고"),
        Tent => ("hut", "천막"),
        Windbreak => ("hut", "바람막이"),
        Campfire => ("campfire", "모닥불"),
        Grave => ("grave", "무덤"),
        Stash | WoodLog => ("plant", "저장 더미"),
        Relic => ("relic", "발명 흔적"),
        AnimalBoar => ("animal", "멧돼지"),
        AnimalDeer => ("animal", "사슴"),
        AnimalWaterDeer => ("animal", "고라니"),
        AnimalSquirrel => ("animal", "다람쥐"),
        AnimalPheasant => ("animal", "꿩"),
    }
}

/// 자원 팝업 — 표시 + 서명 변화 시 본문 재구성.
#[allow(clippy::type_complexity, clippy::too_many_arguments)] // Bevy 시스템 파라미터 — 프로젝트 관례
pub(crate) fn refresh_res(
    state: Res<UiState>,
    theme: Res<super::theme::UiTheme>,
    net: Res<NetState>,
    world: Res<WorldData>,
    mut q_root: Query<&mut Visibility, (With<ResPopupRoot>, Without<super::civ::CivPopupRoot>)>,
    q_body: Query<(Entity, Option<&Children>), (With<ResBody>, Without<ResPopupRoot>)>,
    mut commands: Commands,
    mut sigs: ResMut<CivSigs>,
) {
    let visible = state.popup == Some(Popup::Res);
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
    let rows = res_rows(&net, &world);
    let sig = format!("{:?}", rows);
    if sig == sigs.res {
        return;
    }
    sigs.res = sig;
    let Ok((body_e, children)) = q_body.single() else {
        return;
    };
    clear_children(children, &mut commands);
    if rows.is_empty() {
        commands.entity(body_e).with_child((
            TextLine,
            Text::new("아직 이 월드에서 자원을 찾지 못했다"),
            theme.label(11.0),
        ));
        return;
    }
    for (kind, total, chips) in rows {
        commands.entity(body_e).with_child((
            TextLine,
            Text::new(format!("{} {total} — {chips}", res_kind_label(&kind))),
            theme.label(11.0),
        ));
    }
}
