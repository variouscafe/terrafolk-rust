//! 우측 정보 레일 — 생활 기록(A4)·관계(A5) 패널(FE ChatSideRail·tf-rail.js
//! 동형). 우측 세로 pill 버튼 2개 + 옆 카드 패널, 재클릭으로 닫힌다.
//! 데이터는 HTTP 폴러(net/http_poll.rs)가 PollStore에 15s/20s로 채운다 —
//! 여기선 선택 주민 id를 want_history로 요청하고 서명 변화 시 재구성.
//! 라이브 전용(데모는 안내 문구).

use bevy::prelude::*;
use bevy::ui_widgets::Button; // 0.20: prelude의 Button은 deprecated alias

use super::i18n;
use super::theme::{self, UiTheme};
use crate::net::apply::LiveAgentTag;
use crate::net::feed::{ChatLog, PollStore};
use crate::world::pick::Selection;
use crate::world::villager::Villager;

#[derive(Component)]
pub struct RailRoot;
/// 레일 버튼(0=생활기록, 1=관계, 2=소지품, 3=대화)
#[derive(Component)]
pub struct RailBtn(u8);
#[derive(Component)]
pub struct RailPanel;
#[derive(Component)]
pub struct RailTitle;
/// 본문 목록 컨테이너
#[derive(Component)]
pub struct RailBody;

/// 레일 상태 — 열린 패널(없으면 닫힘).
#[derive(Resource, Default)]
pub struct RailState {
    pub open: Option<u8>,
    sig: String,
}

pub fn spawn(mut commands: Commands, theme: Res<UiTheme>, safe: Res<crate::platform::safe_area::SafeArea>) {
    // 우측 세로 레일 — HUD 아래. 모바일은 심볼만(웹 media query 직역 근사).
    commands
        .spawn((
            RailRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(safe.top + 56.0),
                right: Val::Px(12.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(6.0),
                ..default()
            },
        ))
        .with_children(|rail| {
            rail_btn(rail, &theme, 0, "史", "생활기록");
            rail_btn(rail, &theme, 1, "♥", "관계");
            rail_btn(rail, &theme, 2, "袋", "소지품");
            rail_btn(rail, &theme, 3, "言", "대화");
        });
    // 패널 카드 — 레일 버튼 왼쪽
    commands
        .spawn((
            RailPanel,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(safe.top + 56.0),
                right: Val::Px(58.0),
                width: Val::Px(300.0),
                max_width: Val::Percent(68.0),
                max_height: Val::Px(330.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(10.0)),
                row_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(Color::srgba(0.067, 0.082, 0.114, 0.92)),
            Visibility::Hidden,
        ))
        .with_children(|p| {
            p.spawn((RailTitle, Text::new(""), theme.title(12.0)));
            p.spawn((
                RailBody,
                Node {
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(3.0),
                    ..default()
                },
            ));
        });
    commands.init_resource::<RailState>();
}

fn rail_btn(rail: &mut ChildSpawnerCommands, theme: &UiTheme, idx: u8, sym: &str, label: &str) {
    rail.spawn((
        RailBtn(idx),
        Button,
        Node {
            padding: UiRect::axes(Val::Px(8.0), Val::Px(7.0)),
            ..default()
        },
        BackgroundColor(theme::INK800),
        Interaction::default(),
    ))
    .observe(on_rail_click)
    .with_children(|b| {
        b.spawn((Text::new(format!("{sym} {label}")), theme.label(10.0)));
    });
}

fn on_rail_click(trigger: On<PointerClick>, btns: Query<&RailBtn>, mut state: ResMut<RailState>) {
    if let Ok(btn) = btns.get(trigger.entity) {
        // 재클릭 닫힘(FE ChatSideRail 규약)
        state.open = if state.open == Some(btn.0) { None } else { Some(btn.0) };
    }
}

/// 레일 패널 갱신 — want_history 요청·본문 재구성(서명 변화 시만).
#[allow(clippy::type_complexity)]
pub fn refresh_rail(
    mut state: ResMut<RailState>,
    theme: Res<UiTheme>,
    sel: Res<Selection>,
    store: Res<PollStore>,
    chat: Res<ChatLog>,
    villagers: Query<(&LiveAgentTag, &Villager)>,
    net: Res<crate::net::NetState>,
    mut commands: Commands,
    mut q_panel: Query<&mut Visibility, (With<RailPanel>, Without<RailBody>)>,
    mut q_title: Query<&mut Text, (With<RailTitle>, Without<RailBody>)>,
    q_body: Query<(Entity, Option<&Children>), (With<RailBody>, Without<RailPanel>)>,
) {
    let visible = state.open.is_some();
    if let Ok(mut v) = q_panel.single_mut() {
        *v = if visible { Visibility::Visible } else { Visibility::Hidden };
    }
    // 생활기록 요청 — 패널 열림 + 선택 주민. 닫히거나 미선택이면 해제.
    {
        let mut want = store.want_history.lock().unwrap_or_else(|e| e.into_inner());
        *want = if state.open == Some(0) { sel.id.clone() } else { None };
    }
    if !visible {
        state.sig.clear();
        return;
    }
    let kind = state.open.unwrap_or(0);
    let name_of = |id: &str| -> String {
        villagers
            .iter()
            .find(|(t, _)| t.0 == id)
            .map(|(_, v)| v.name.clone())
            .unwrap_or_else(|| id.to_string())
    };
    let hist = store.history.lock().unwrap_or_else(|e| e.into_inner());
    let rels = store.relations.lock().unwrap_or_else(|e| e.into_inner());
    // 서명 — 패널 종류·선택·데이터 버전(행 수·끝 ts)
    let sig = format!(
        "{kind}|{}|{}|{}|{}|{}|{}",
        sel.id.clone().unwrap_or_default(),
        hist.len(),
        hist.last().map(|h| h.ts.clone()).unwrap_or_default(),
        rels.len(),
        chat.rows.len(),
        chat.rows.back().map(|c| (c.gmin.unwrap_or(0), c.text.clone())).map(|(g, t)| format!("{g}{t}")).unwrap_or_default(),
    );
    if sig == state.sig {
        return;
    }
    state.sig = sig;
    let Ok((body_e, children)) = q_body.single() else {
        return;
    };
    let stale: Vec<Entity> = children.map(|c| c.to_vec()).unwrap_or_default();
    for c in stale {
        commands.entity(c).despawn();
    }
    if let Ok(mut title) = q_title.single_mut() {
        **title = match kind {
            0 => "생활 기록".into(),
            1 => "관계".into(),
            2 => "소지품".into(),
            _ => "대화 기록".into(),
        };
    }
    if net.status != crate::net::NetStatus::Connected {
        spawn_hint(&mut commands, body_e, &theme, "라이브 접속 시 표시된다");
        return;
    }
    match kind {
        0 => {
            // A4 생활 기록 — 선택 주민 히스토리 25행(최신 하단, FE 로그 방향).
            if sel.id.is_none() {
                spawn_hint(&mut commands, body_e, &theme, "주민을 선택하면 생활 기록이 표시된다");
                return;
            }
            if hist.is_empty() {
                spawn_hint(&mut commands, body_e, &theme, "아직 기록이 없다");
                return;
            }
            let start = hist.len().saturating_sub(25);
            for h in &hist[start..] {
                let color = i18n::hist_color(&h.type_);
                let label = i18n::hist_label(&h.type_);
                let hm = i18n::fmt_time(&h.ts);
                let row = commands
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(5.0),
                        ..default()
                    })
                    .with_children(|r| {
                        if !hm.is_empty() {
                            r.spawn((Text::new(hm.clone()), theme.text(8.0, theme::INK400)));
                        }
                        r.spawn((Text::new(label.clone()), theme.text(8.5, color)));
                        r.spawn((Text::new(h.text.clone()), theme.text(8.5, theme::INK200)));
                    })
                    .id();
                commands.entity(body_e).add_child(row);
            }
        }
        1 => {
            // A5 관계 — 선택 주민 포함 쌍(미선택 시 마을 전체) warmth top8.
            let rows: Vec<_> = rels
                .iter()
                .filter(|r| match sel.id.as_deref() {
                    Some(id) => r.a_id == id || r.b_id == id,
                    None => true,
                })
                .map(|r| {
                    let other = if sel.id.as_deref() == Some(r.a_id.as_str()) {
                        r.b_id.clone()
                    } else {
                        r.a_id.clone()
                    };
                    (other, r)
                })
                .collect();
            let mut rows = rows;
            rows.sort_by(|a, b| b.1 .warmth.total_cmp(&a.1 .warmth));
            if rows.is_empty() {
                spawn_hint(&mut commands, body_e, &theme, "아직 관계가 형성되지 않았다");
                return;
            }
            for (other, r) in rows.iter().take(8) {
                let name = name_of(other);
                let color = i18n::agent_color(other);
                // FE hsl(warmth×1.2, 65%, 55%) — warmth 0..100 → 색조 0..120°
                let bar = Color::hsl(r.warmth * 1.2, 0.65, 0.55);
                let row = commands
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        align_items: AlignItems::Center,
                        column_gap: Val::Px(6.0),
                        ..default()
                    })
                    .with_children(|w| {
                        w.spawn((
                            Text::new(name.clone()),
                            theme.text(9.0, color),
                            Node {
                                width: Val::Px(64.0),
                                ..default()
                            },
                        ));
                        w.spawn((
                            Node {
                                width: Val::Px(90.0),
                                height: Val::Px(5.0),
                                ..default()
                            },
                            BackgroundColor(theme::INK800),
                        ))
                        .with_children(|track| {
                            track.spawn((
                                Node {
                                    width: Val::Percent(r.warmth.clamp(0.0, 100.0)),
                                    height: Val::Percent(100.0),
                                    ..default()
                                },
                                BackgroundColor(bar),
                            ));
                        });
                        w.spawn((
                            Text::new(format!(
                                "{} {} · 대화 {} · 교역 {}",
                                i18n::rel_label(r.warmth),
                                r.warmth.round() as i32,
                                r.chats,
                                r.trades
                            )),
                            theme.text(8.0, theme::INK400),
                        ));
                    })
                    .id();
                commands.entity(body_e).add_child(row);
            }
        }
        2 => {
            // A3 소지품 — 선택 주민 invJson(§17.81 착용 제외 무게 바 + 칩).
            let Some(id) = sel.id.as_deref() else {
                spawn_hint(&mut commands, body_e, &theme, "주민을 선택하면 소지품이 표시된다");
                return;
            };
            let Some(inv_src) = net
                .frame
                .as_ref()
                .and_then(|f| f.agents.iter().find(|a| a.id == id))
                .and_then(|a| a.inv_json.as_deref())
            else {
                spawn_hint(&mut commands, body_e, &theme, "소지품 정보가 없다");
                return;
            };
            let Ok(inv) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(inv_src)
            else {
                spawn_hint(&mut commands, body_e, &theme, "소지품을 읽을 수 없다");
                return;
            };
            if inv.is_empty() {
                spawn_hint(&mut commands, body_e, &theme, "빈손이다");
                return;
            }
            // 무게 바 — 85%+ 앰버 경고(FE CarryBar 직역)
            const CAP: f32 = 5000.0;
            let wt = i18n::carry_weight(&inv);
            let heavy = wt / CAP >= 0.85;
            let bar_row = commands
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    align_items: AlignItems::Center,
                    column_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|w| {
                    w.spawn((Text::new("袋"), theme.text(9.0, theme::INK400)));
                    w.spawn((
                        Node {
                            flex_grow: 1.0,
                            height: Val::Px(5.0),
                            ..default()
                        },
                        BackgroundColor(theme::INK800),
                    ))
                    .with_children(|track| {
                        track.spawn((
                            Node {
                                width: Val::Percent((wt / CAP * 100.0).min(100.0)),
                                height: Val::Percent(100.0),
                                ..default()
                            },
                            BackgroundColor(if heavy {
                                Color::srgb(0.984, 0.749, 0.141)
                            } else {
                                Color::srgb(0.063, 0.725, 0.506)
                            }),
                        ));
                    });
                    w.spawn((
                        Text::new(format!("{wt}/5000")),
                        theme.text(
                            8.0,
                            if heavy {
                                Color::srgb(0.988, 0.816, 0.325)
                            } else {
                                theme::INK400
                            },
                        ),
                    ));
                })
                .id();
            commands.entity(body_e).add_child(bar_row);
            // 칩 목록 — 라벨 ×수량(무게 내림)
            let mut items: Vec<(&String, &serde_json::Value)> = inv.iter().collect();
            items.sort_by(|a, b| {
                (b.1.as_f64().unwrap_or(0.0) * i18n::item_wt(b.0) as f64)
                    .total_cmp(&(a.1.as_f64().unwrap_or(0.0) * i18n::item_wt(a.0) as f64))
            });
            let chips = commands
                .spawn(Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(4.0),
                    row_gap: Val::Px(4.0),
                    ..default()
                })
                .id();
            for (id, qty) in items {
                let chip = commands
                    .spawn((
                        Node {
                            padding: UiRect::axes(Val::Px(5.0), Val::Px(2.0)),
                            ..default()
                        },
                        BackgroundColor(theme::INK800),
                    ))
                    .with_children(|c| {
                        c.spawn((
                            Text::new(format!(
                                "{} ×{}",
                                i18n::vocab_label("item", id),
                                qty.as_f64().map(|v| v as u32).unwrap_or(0)
                            )),
                            theme.text(8.5, theme::INK200),
                        ));
                    })
                    .id();
                commands.entity(chips).add_child(chip);
            }
            commands.entity(body_e).add_child(chips);
        }
        _ => {
            // A6 대화 기록 — 최근 25행(게임 시각·화자색). 사람 발화는 액센트.
            if chat.rows.is_empty() {
                spawn_hint(&mut commands, body_e, &theme, "아직 대화가 없다");
                return;
            }
            let start = chat.rows.len().saturating_sub(25);
            for row in chat.rows.iter().skip(start) {
                let is_human = row.speaker.contains('@') || row.speaker == "user";
                let color = if is_human {
                    theme::ACCENT
                } else {
                    i18n::agent_color(&row.agent_id)
                };
                let stamp = row.gmin.map(i18n::fmt_gmin).unwrap_or_default();
                let text = if row.text.is_empty() {
                    row.text_en.clone().unwrap_or_default()
                } else {
                    row.text.clone()
                };
                let line = commands
                    .spawn(Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(5.0),
                        ..default()
                    })
                    .with_children(|l| {
                        if !stamp.is_empty() {
                            l.spawn((Text::new(stamp), theme.text(7.5, theme::INK400)));
                        }
                        l.spawn((Text::new(row.speaker.clone()), theme.text(8.5, color)));
                        l.spawn((Text::new(text), theme.text(8.5, theme::INK200)));
                    })
                    .id();
                commands.entity(body_e).add_child(line);
            }
        }
    }
}

fn spawn_hint(commands: &mut Commands, body: Entity, theme: &UiTheme, text: &str) {
    let hint = commands
        .spawn((Text::new(text.to_string()), theme.text(9.0, theme::INK400)))
        .id();
    commands.entity(body).add_child(hint);
}
