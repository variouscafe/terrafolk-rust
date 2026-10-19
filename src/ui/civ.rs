//! 문명 팝업 — 레거시 FE CivPopup 패리티. 시대 요약·6대분류 진척(도트 8)·
//! 만든 이들·살아있는 전통(§17.60 거행 통계)·마을 연대기(chronicle)·
//! 발명 목록(탭 → 상세 카드: 재료·만든 날짜·설명). 자원 팝업은 res.rs로 분리.

use bevy::prelude::*;
use bevy::ui_widgets::Button; // 0.20: prelude의 Button은 deprecated alias

use super::i18n;
use super::theme::{self, UiTheme};
use super::{Popup, UiState};
use crate::net::NetState;
use crate::net::feed::EventLog;

#[derive(Component)]
pub struct CivPopupRoot;
#[derive(Component)]
pub struct CivBody;

/// 발명 목록 행(탭 → 상세 카드) — 이름으로 식별.
#[derive(Component)]
struct CivRow(String);
/// 상세 카드 닫기(✕) 버튼.
#[derive(Component)]
struct CivDetailClose;

/// 각 팝업 재구성 서명(자원은 res.rs가 사용) — 변화 시에만 자식 재구성.
#[derive(Resource, Default)]
pub struct CivSigs {
    pub res: String,
    pub civ: String,
}

/// 발명 상세 — 열린 발명 이름(None이면 목록 뷰).
#[derive(Resource, Default)]
pub struct CivDetail(pub Option<String>);

/// inventionsJson 행 — docs §2.4(서버 worldsim serializeInventions).
#[derive(serde::Deserialize, Clone)]
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
    #[serde(default)]
    materials: Option<std::collections::BTreeMap<String, u32>>,
    #[serde(default)]
    performs: Option<u32>,
    #[serde(default)]
    last_perform_day: Option<u32>,
}

pub fn spawn(mut commands: Commands, theme: Res<UiTheme>) {
    commands
        .spawn((
            CivPopupRoot,
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
                CivBody,
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(14.0)),
                    row_gap: Val::Px(6.0),
                    min_width: Val::Px(240.0),
                    width: Val::Px(320.0),
                    max_height: Val::Px(400.0),
                    overflow: Overflow::clip_y(),
                    ..default()
                },
                BackgroundColor(super::theme::SURFACE),
            ))
            .with_children(|card| {
                card.spawn((Text::new("문명"), theme.title(13.0)));
            });
        });
    commands.init_resource::<CivSigs>();
    commands.init_resource::<CivDetail>();
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

fn civ_summary(rows: &[InventionRow]) -> (u32, &'static str) {
    let mut groups: std::collections::BTreeSet<&str> = Default::default();
    for r in rows {
        if let Some(c) = r.category.as_deref() {
            groups.insert(i18n::inv_group_of(c));
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

/// 문명 팝업 — inventionsJson 파싱(원문 변화 시만) + 연대기(EventLog).
#[allow(clippy::type_complexity, clippy::too_many_arguments)] // Bevy 시스템 파라미터 — 프로젝트 관례
pub(crate) fn refresh_civ(
    state: Res<UiState>,
    theme: Res<UiTheme>,
    net: Res<NetState>,
    evlog: Res<EventLog>,
    mut detail: ResMut<CivDetail>,
    mut q_root: Query<&mut Visibility, (With<CivPopupRoot>, Without<super::res::ResPopupRoot>)>,
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
        detail.0 = None; // 닫힐 때 상세 뷰 초기화
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
    let chronicle: Vec<_> = evlog
        .rows
        .iter()
        .filter(|e| e.kind == "chronicle")
        .collect();
    let sig = format!(
        "{}:{:?}|{:?}|{}",
        rows.len(),
        rows.first().map(|r| &r.name),
        detail.0,
        chronicle.len(),
    );
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
    // ── 상세 카드 뷰(발명 탭 시) — 목록 대신
    if let Some(name) = detail.0.clone()
        && let Some(r) = rows.iter().find(|r| r.name == name)
    {
        spawn_detail(&mut commands, body_e, &theme, r);
        return;
    }
    let (score, era) = civ_summary(rows);
    commands.entity(body_e).with_child((
        SummaryLine,
        Text::new(format!("{era} · 발명 {} · 문명 점수 {score}", rows.len())),
        theme.text(12.0, theme::ACCENT),
    ));
    // 6대분류 진척 — 도트 8칸 스케일(FE GroupDots 직역)
    for group in [
        "living",
        "production",
        "infrastructure",
        "culture",
        "knowledge",
        "institutions",
    ] {
        let total = rows
            .iter()
            .filter(|r| r.category.as_deref().is_some_and(|c| i18n::inv_group_of(c) == group))
            .count();
        let dots = format!(
            "{}{}",
            "●".repeat(total.min(8)),
            "○".repeat(8 - total.min(8))
        );
        commands.entity(body_e).with_child((
            TextLine,
            Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(6.0),
                ..default()
            },
        )).with_children(|g| {
            g.spawn((Text::new(i18n::inv_group_label(group)), theme.label(9.0)));
            g.spawn((
                Text::new(dots),
                theme.text(9.0, if total > 0 {
                    Color::srgb(0.984, 0.749, 0.141)
                } else {
                    theme::INK600
                }),
            ));
            g.spawn((Text::new(format!("{total}")), theme.text(9.0, theme::INK400)));
        });
    }
    // 만든 이들 top4(레거시 civ.makers)
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
            Text::new(format!("{} — {}", i18n::t_or("civ.makers", "만든 이들"), line)),
            theme.label(10.0),
        ));
    }
    // 살아있는 전통(§17.60) — 문화 계열 발명의 거행 통계. 관습은 행해질수록 깊어진다.
    let held: Vec<&InventionRow> = rows
        .iter()
        .filter(|r| {
            r.category.as_deref().is_some_and(|c| i18n::inv_group_of(c) == "culture")
                && r.performs.unwrap_or(0) > 0
        })
        .collect();
    let culture_total = rows
        .iter()
        .filter(|r| r.category.as_deref().is_some_and(|c| i18n::inv_group_of(c) == "culture"))
        .count();
    if culture_total > 0 {
        commands.entity(body_e).with_child((
            TextLine,
            Text::new(i18n::t_or("civ.living", "살아있는 전통")),
            theme.title(11.0),
        ));
        if held.is_empty() {
            commands.entity(body_e).with_child((
                TextLine,
                Text::new(i18n::t_or("civ.living.empty", "아직 거행된 전통이 없다")),
                theme.text(9.5, theme::INK400),
            ));
        } else {
            let line = held
                .iter()
                .map(|r| {
                    let n = r.performs.unwrap_or(0);
                    let d = (n / 4).min(3);
                    format!(
                        "{} {}",
                        r.name,
                        i18n::t_fmt("civ.living.holds", &[("n", &n.to_string()), ("d", &d.to_string())])
                    )
                })
                .collect::<Vec<_>>()
                .join(" · ");
            commands.entity(body_e).with_child((
                TextLine,
                Text::new(line),
                theme.text(9.5, theme::INK200),
            ));
        }
    }
    // 마을 연대기(§17.60) — 이벤트 스트림 chronicle 행 최근 12(최신 하단)
    if !chronicle.is_empty() {
        commands.entity(body_e).with_child((
            TextLine,
            Text::new(i18n::t_or("civ.story", "마을 연대기")),
            theme.title(11.0),
        ));
        let start = chronicle.len().saturating_sub(12);
        for ev in &chronicle[start..] {
            let text = i18n::event_text(&ev.payload);
            if text.is_empty() {
                continue;
            }
            commands.entity(body_e).with_child((
                TextLine,
                Text::new(format!("{} {}", i18n::ev_kind_icon("chronicle"), text)),
                theme.text(9.0, i18n::event_color("chronicle")),
            ));
        }
    }
    // 발명 목록(최신순 30) — 행 탭 → 상세 카드
    for r in rows.iter().rev().take(30) {
        let day = r.day.map(|d| format!(" · D+{d}")).unwrap_or_default();
        let by = r.inventor.as_deref().unwrap_or("");
        let row = commands
            .spawn((
                CivRow(r.name.clone()),
                Button,
                TextLine,
                Node {
                    padding: UiRect::axes(Val::Px(4.0), Val::Px(2.0)),
                    ..default()
                },
                BackgroundColor(Color::NONE),
                Interaction::default(),
            ))
            .observe(on_inv_click)
            .with_children(|t| {
                t.spawn((
                    Text::new(format!("{} — {}{day}", r.name, by)),
                    theme.label(10.5),
                ));
            })
            .id();
        commands.entity(body_e).add_child(row);
    }
}

/// 발명 상세 카드 — 아이콘 대신 범주 심볼, 재료 목록·만든 날짜(태양력)·설명.
fn spawn_detail(
    commands: &mut Commands,
    body: Entity,
    theme: &UiTheme,
    r: &InventionRow,
) {
    let amber = Color::srgb(0.988, 0.816, 0.325);
    commands.entity(body).with_child((
        TextLine,
        Node {
            flex_direction: FlexDirection::Row,
            align_items: AlignItems::Baseline,
            column_gap: Val::Px(6.0),
            ..default()
        },
    )).with_children(|h| {
        h.spawn((Text::new(r.name.clone()), theme.title(12.0)));
        if let Some(base) = r.based_on.as_deref().filter(|b| !b.is_empty()) {
            h.spawn((
                Text::new(i18n::t_fmt("relic.card.improved", &[("x", base)])),
                theme.text(8.5, amber),
            ));
        }
        h.spawn(Node { flex_grow: 1.0, ..default() });
        // ✕ — 목록으로 돌아가기
        h.spawn((
            CivDetailClose,
            Button,
            Text::new("✕"),
            theme.label(11.0),
            Node {
                padding: UiRect::axes(Val::Px(6.0), Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(theme::INK700),
            Interaction::default(),
        ))
        .observe(on_detail_close);
    });
    // 만든 이 · 날짜(태양력) · 범주
    let (y, m, d, _) = super::hud::date_of(r.day.unwrap_or(0));
    let cat = r.category.as_deref().unwrap_or("");
    let meta = format!(
        "{} · {}   {}   {}",
        i18n::t_or("relic.card.by", "만든 이"),
        r.inventor.as_deref().unwrap_or("—"),
        i18n::t_fmt("civ.inv.date", &[("y", &y.to_string()), ("m", &m.to_string()), ("d", &d.to_string())]),
        i18n::vocab_label("invcat", cat),
    );
    commands.entity(body).with_child((
        TextLine,
        Text::new(meta),
        theme.text(9.0, theme::INK400),
    ));
    // 재료 목록 — "라벨 ×수량 · ..."
    if let Some(mats) = &r.materials
        && !mats.is_empty()
    {
        let line = mats
            .iter()
            .map(|(it, q)| format!("{} ×{q}", i18n::vocab_label("item", it)))
            .collect::<Vec<_>>()
            .join(" · ");
        commands.entity(body).with_child((
            TextLine,
            Text::new(format!("{} · {}", i18n::t_or("inv.materials", "재료"), line)),
            theme.text(9.0, theme::INK200),
        ));
    }
    // 발명가의 설명(자유문 — 원문 표시)
    let desc = r
        .purpose
        .as_deref()
        .filter(|p| !p.trim().is_empty())
        .map(str::to_owned)
        .unwrap_or_else(|| i18n::t_or("relic.card.desc.empty", "설명이 남지 않았다"));
    commands.entity(body).with_child((
        TextLine,
        Text::new(i18n::fx(&desc)),
        theme.text(9.5, theme::INK200),
    ));
}

fn on_inv_click(trigger: On<PointerClick>, rows: Query<&CivRow>, mut detail: ResMut<CivDetail>) {
    if let Ok(row) = rows.get(trigger.entity) {
        detail.0 = Some(row.0.clone());
    }
}

fn on_detail_close(_: On<PointerClick>, mut detail: ResMut<CivDetail>) {
    detail.0 = None;
}

/// 재구성 대행 표식 — 본문 텍스트 행(스타일 공통, res.rs도 사용).
#[derive(Component)]
pub(crate) struct TextLine;
#[derive(Component)]
struct SummaryLine;

/// 본문 기존 자식 철거 — despawn은 자손까지 재귀 제거(0.19 계약, chars.rs 동일).
pub(crate) fn clear_children(children: Option<&Children>, commands: &mut Commands) {
    if let Some(children) = children {
        for c in children.iter() {
            commands.entity(c).despawn();
        }
    }
}
