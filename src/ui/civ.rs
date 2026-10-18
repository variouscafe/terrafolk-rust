//! 자원·문명 팝업 — 레거시 FE(tf-ui.js resHtml·civHtml) 패리티 첫 슬라이스.
//! 라이브: NetState.frame.resources(종별 집계) + inventionsJson(시대 사다리·
//! 발명 목록). 데모: WorldData 장식 프롭 집계, 문명은 미개(정직 표시).
//! 내용 재구성은 서명(집계 요약) 변화 시만 — 매 프레임 멱등.

use bevy::prelude::*;

use super::{Popup, UiState};
use crate::net::NetState;
use crate::world::data::WorldData;

#[derive(Component)]
pub struct ResPopupRoot;
#[derive(Component)]
pub struct ResBody;
#[derive(Component)]
pub struct CivPopupRoot;
#[derive(Component)]
pub struct CivBody;

/// 각 팝업 재구성 서명(자원/문면 집계 요약) — 변화 시에만 자식 재구성.
#[derive(Resource, Default)]
pub struct CivSigs {
    pub res: String,
    pub civ: String,
}

/// inventionsJson 행 — docs §2.4(서버 worldboot serializeInventions).
#[derive(serde::Deserialize)]
#[allow(dead_code)] // 표시에 골라 쓴다 — 필드는 계약 정본 그대로
pub(crate) struct InventionRow {
    name: String,
    #[serde(default)]
    purpose: Option<String>,
    #[serde(default)]
    inventor: Option<String>,
    #[serde(default)]
    based_on: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    day: Option<u32>,
}

pub fn spawn(mut commands: Commands, theme: Res<super::theme::UiTheme>) {
    /// 팝업 카드 셸 공통(백드롭 닫기·타이틀·본문 라인들)
    macro_rules! popup_shell {
        ($root:ident, $body:ident, $title:expr) => {
            commands
                .spawn((
                    $root,
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
                        $body,
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
                        card.spawn((Text::new($title), theme.title(13.0)));
                    });
                });
        };
    }
    popup_shell!(ResPopupRoot, ResBody, "자원");
    popup_shell!(CivPopupRoot, CivBody, "문명");
    commands.init_resource::<CivSigs>();
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

/// ── 문명 — 시대 사다리(레거시 ERA_STEPS·civScore 직역) ──
const ERA_STEPS: [(&str, u32); 9] = [
    ("band", 0),
    ("camp", 6),
    ("village", 12),
    ("large_village", 20),
    ("town", 32),
    ("city", 48),
    ("metropolis", 70),
    ("megacity", 100),
    ("beyond", 140),
];

fn era_label(key: &str) -> &'static str {
    match key {
        "band" => "무리",
        "camp" => "캠프",
        "village" => "마을",
        "large_village" => "큰 마을",
        "town" => "읍",
        "city" => "도시",
        "metropolis" => "대도시",
        "megacity" => "거대 도시",
        _ => "문명의 경계",
    }
}

/// 발명 category → 6계열 그룹(레거시 INVCAT_GROUPS 직역).
fn inv_group(category: &str) -> Option<&'static str> {
    Some(match category.trim().to_ascii_lowercase().as_str() {
        "clothing" | "food" | "housing" | "furniture" => "living",
        "tools" | "tool" | "materials" | "machinery" => "production",
        "buildings" | "building" | "transport" => "infrastructure",
        "arts" | "entertainment" | "traditions" => "culture",
        "language" | "science" | "techniques" => "knowledge",
        "economy" | "governance" | "laws" => "institutions",
        _ => return None,
    })
}

fn civ_summary(rows: &[InventionRow]) -> (u32, &'static str) {
    let mut groups: std::collections::BTreeSet<&str> = Default::default();
    for r in rows {
        if let Some(c) = r.category.as_deref().and_then(inv_group) {
            groups.insert(c);
        }
    }
    let score = rows.len() as u32 + groups.len() as u32 * 2;
    let mut era = "band";
    for (key, at) in ERA_STEPS {
        if score >= at {
            era = key;
        }
    }
    (score, era_label(era))
}

/// 자원 팝업 — 표시 + 서명 변화 시 본문 재구성.
#[allow(clippy::type_complexity, clippy::too_many_arguments)] // Bevy 시스템 파라미터 — 프로젝트 관례
pub(crate) fn refresh_res(
    state: Res<UiState>,
    theme: Res<super::theme::UiTheme>,
    net: Res<NetState>,
    world: Res<WorldData>,
    mut q_root: Query<&mut Visibility, (With<ResPopupRoot>, Without<CivPopupRoot>)>,
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

/// 문명 팝업 — inventionsJson 파싱(원문 변화 시만)·시대 요약·목록.
#[allow(clippy::type_complexity, clippy::too_many_arguments)] // Bevy 시스템 파라미터 — 프로젝트 관례
pub(crate) fn refresh_civ(
    state: Res<UiState>,
    theme: Res<super::theme::UiTheme>,
    net: Res<NetState>,
    mut q_root: Query<&mut Visibility, (With<CivPopupRoot>, Without<ResPopupRoot>)>,
    q_body: Query<(Entity, Option<&Children>), (With<CivBody>, Without<CivPopupRoot>)>,
    mut commands: Commands,
    mut sigs: ResMut<CivSigs>,
    mut cache: Local<Option<(String, Vec<InventionRow>)>>,
) {
    let visible = state.popup == Some(Popup::Inventions);
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
    // 원문 파싱 캐시 — JSON은 프레임마다 오지만 원문이 같으면 재파싱 않는다
    let raw = net
        .frame
        .as_ref()
        .and_then(|f| f.inventions.as_deref())
        .map(str::to_owned);
    if cache
        .as_ref()
        .map(|(s, _)| s != raw.as_deref().unwrap_or(""))
        != Some(true)
    {
        *cache = raw.map(|s| {
            let rows = serde_json::from_str::<Vec<InventionRow>>(&s).unwrap_or_default();
            (s, rows)
        });
    }
    let rows = cache
        .as_ref()
        .map(|(_, r)| r.as_slice())
        .unwrap_or_default();
    let sig = format!("{}:{:?}", rows.len(), rows.first().map(|r| &r.name));
    if sig == sigs.civ {
        return;
    }
    sigs.civ = sig;
    let Ok((body_e, children)) = q_body.single() else {
        return;
    };
    clear_children(children, &mut commands);
    if rows.is_empty() {
        commands.entity(body_e).with_child((
            TextLine,
            Text::new(if net.frame.is_some() {
                "아직 고안된 것이 없다" // 라이브 — 발명 0
            } else {
                "데모 마을 — 라이브 접속 시 문명이 표시된다"
            }),
            theme.label(11.0),
        ));
        return;
    }
    let (score, era) = civ_summary(rows);
    commands.entity(body_e).with_child((
        SummaryLine,
        Text::new(format!("{era} · 발명 {} · 문명 점수 {score}", rows.len())),
        theme.text(12.0, super::theme::ACCENT),
    ));
    // 만든 이들 top4(레거시 civ.makers 근사)
    let mut makers: std::collections::BTreeMap<&str, usize> = Default::default();
    for r in rows {
        if let Some(w) = r.inventor.as_deref().filter(|w| !w.trim().is_empty()) {
            *makers.entry(w).or_insert(0) += 1;
        }
    }
    let mut top: Vec<_> = makers.into_iter().collect();
    top.sort_unstable_by_key(|(_, n)| std::cmp::Reverse(*n));
    if !top.is_empty() {
        let line = top
            .iter()
            .take(4)
            .map(|(w, n)| format!("{w} ×{n}"))
            .collect::<Vec<_>>()
            .join(" · ");
        commands.entity(body_e).with_child((
            TextLine,
            Text::new(format!("만든 이들 — {line}")),
            theme.label(10.0),
        ));
    }
    for r in rows.iter().rev().take(30) {
        let day = r.day.map(|d| format!(" · D+{d}")).unwrap_or_default();
        let by = r.inventor.as_deref().unwrap_or("");
        commands.entity(body_e).with_child((
            TextLine,
            Text::new(format!("{} — {}{day}", r.name, by)),
            theme.label(11.0),
        ));
    }
}

/// 재구성 대행 표식 — 본문 텍스트 행(스타일 공통).
#[derive(Component)]
struct TextLine;
#[derive(Component)]
struct SummaryLine;

/// 본문 기존 자식 철거 — despawn은 자손까지 재귀 제거(0.19 계약, chars.rs 동일).
fn clear_children(children: Option<&Children>, commands: &mut Commands) {
    if let Some(children) = children {
        for c in children.iter() {
            commands.entity(c).despawn();
        }
    }
}
