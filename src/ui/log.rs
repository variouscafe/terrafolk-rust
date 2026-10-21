//! 기록 팝업 — 레거시 FE LogFeed(팝업 7탭) 패리티. EventLog(룸 WS 실시간 +
//! HTTP 백로그)을 분류 탭으로 걸러 시각·종류색·렌더 문장으로 보여준다.
//! bevy_ui에 스크롤이 없어 20행/페이지 페이징(최신 페이지 기본).
//! 서명(탭·페이지·행 수·마지막 행 신원) 변화 시에만 재구성.

use bevy::prelude::*;

use super::i18n;
use super::theme::{self, UiTheme};
use super::{CloseButton, Popup, UiState};
use crate::net::feed::EventLog;
use crate::net::{NetState, NetStatus};

const PER_PAGE: usize = 20;

#[derive(Component)]
pub struct LogPopupRoot;
/// 탭 버튼(인덱스 0..=6)
#[derive(Component)]
pub struct LogTabBtn(u8);
/// 목록 컨테이너
#[derive(Component)]
pub struct LogList;
#[derive(Component)]
pub struct LogPageLabel;
#[derive(Component)]
pub struct LogPrevBtn;
#[derive(Component)]
pub struct LogNextBtn;
/// 데모(기록 없음) 안내 문구
#[derive(Component)]
pub struct LogEmptyHint;

/// 탭·페이징 상태 — 0 전체 · 1 하이라이트 · 2..6 5분류.
#[derive(Resource)]
pub struct LogUi {
    pub tab: u8,
    pub page: usize,
    sig: String,
}

impl Default for LogUi {
    fn default() -> Self {
        LogUi {
            tab: 0,
            page: 0,
            sig: String::new(),
        }
    }
}

fn tab_keys() -> [&'static str; 7] {
    [
        "panel.log.tab.all",
        "panel.log.tab.highlight",
        "panel.log.tab.social",
        "panel.log.tab.living",
        "panel.log.tab.knowledge",
        "panel.log.tab.life",
        "panel.log.tab.world",
    ]
}

pub fn spawn(mut commands: Commands, theme: Res<UiTheme>) {
    commands
        .spawn((
            LogPopupRoot,
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
                    row_gap: Val::Px(8.0),
                    width: Val::Px(340.0),
                    max_height: Val::Px(400.0),
                    ..default()
                },
                BackgroundColor(theme::SURFACE),
            ))
            .with_children(|card| {
                card.spawn((Text::new("기록"), theme.title(13.0)));
                // 탭 행 — FE LogFeed 7분류(전체/하이라이트/사회/생활/지식/생애/세계)
                card.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    flex_wrap: FlexWrap::Wrap,
                    column_gap: Val::Px(4.0),
                    row_gap: Val::Px(4.0),
                    ..default()
                })
                .with_children(|tabs| {
                    for (i, key) in tab_keys().into_iter().enumerate() {
                        tabs.spawn((
                            LogTabBtn(i as u8),
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(7.0), Val::Px(4.0)),
                                ..default()
                            },
                            BackgroundColor(theme::INK800),
                            Interaction::default(),
                        ))
                        .observe(on_tab_click)
                        .with_children(|b| {
                            b.spawn((Text::new(i18n::t_or(key, key)), theme.text(9.0, theme::INK200)));
                        });
                    }
                });
                card.spawn((LogEmptyHint, Text::new("라이브 접속 시 기록이 쌓인다"), theme.text(10.0, theme::INK400), Visibility::Hidden));
                card.spawn((
                    LogList,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(3.0),
                        ..default()
                    },
                ));
                // 페이징 푸터
                card.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(8.0),
                    align_items: AlignItems::Center,
                    justify_content: JustifyContent::Center,
                    ..default()
                })
                .with_children(|f| {
                    page_btn(f, &theme, LogPrevBtn, "◀");
                    f.spawn((LogPageLabel, Text::new(""), theme.text(9.0, theme::INK400)));
                    page_btn(f, &theme, LogNextBtn, "▶");
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
                .observe(on_close_click)
                .with_children(|b| {
                    b.spawn((Text::new("닫기"), theme.label(11.0)));
                });
            });
        });
    commands.init_resource::<LogUi>();
}

fn page_btn(f: &mut ChildSpawnerCommands, theme: &UiTheme, marker: impl Component, label: &str) {
    f.spawn((
        marker,
        Button,
        Node {
            padding: UiRect::axes(Val::Px(9.0), Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(theme::INK800),
        Interaction::default(),
    ))
    .observe(on_page_click)
    .with_children(|b| {
        b.spawn((Text::new(label), theme.label(10.0)));
    });
}

fn on_tab_click(trigger: On<Pointer<Click>>, tabs: Query<&LogTabBtn>, mut ui: ResMut<LogUi>) {
    if let Ok(btn) = tabs.get(trigger.entity) {
        ui.tab = btn.0;
        ui.page = 0;
    }
}

fn on_page_click(
    trigger: On<Pointer<Click>>,
    prev: Query<(), With<LogPrevBtn>>,
    mut ui: ResMut<LogUi>,
) {
    if prev.contains(trigger.entity) {
        ui.page = ui.page.saturating_sub(1);
    } else {
        ui.page += 1;
    }
}

fn on_close_click(_: On<Pointer<Click>>, mut state: ResMut<UiState>) {
    state.popup = None;
}

/// 탭별 필터 — 1 하이라이트는 highlight_of 판정 공유(토스트와 동일 규약).
fn tab_filter(tab: u8, ev: &crate::net::feed::LiveEvent) -> bool {
    match tab {
        0 => true,
        1 => i18n::highlight_of(ev, &|_| String::new()).is_some(),
        t => i18n::ev_cat_of(&ev.kind) == i18n::EV_CATS[(t - 2) as usize],
    }
}

#[allow(clippy::type_complexity)]
pub fn update_log(
    state: Res<UiState>,
    theme: Res<UiTheme>,
    net: Res<NetState>,
    evlog: Res<EventLog>,
    mut ui: ResMut<LogUi>,
    mut commands: Commands,
    mut q_root: Query<
        &mut Visibility,
        (With<LogPopupRoot>, Without<LogList>, Without<LogEmptyHint>),
    >,
    mut q_hint: Query<
        &mut Visibility,
        (With<LogEmptyHint>, Without<LogPopupRoot>, Without<LogList>),
    >,
    q_list: Query<(Entity, Option<&Children>), (With<LogList>, Without<LogPopupRoot>)>,
    mut q_tabs: Query<(&LogTabBtn, &mut BackgroundColor), Without<LogList>>,
    mut q_page: Query<&mut Text, (With<LogPageLabel>, Without<LogList>)>,
) {
    let visible = state.popup == Some(Popup::Log);
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
    // 활성 탭 하이라이트
    for (btn, mut bg) in &mut q_tabs {
        *bg = BackgroundColor(if btn.0 == ui.tab {
            theme::INK600
        } else {
            theme::INK800
        });
    }

    let live = net.status == NetStatus::Connected;
    let filtered: Vec<_> = evlog.rows.iter().filter(|e| tab_filter(ui.tab, e)).collect();
    let total = filtered.len();
    let pages = total.div_ceil(PER_PAGE).max(1);
    ui.page = ui.page.min(pages - 1);

    // 서명 — 탭·페이지·수·마지막 행 신원(id 없으면 tick+kind).
    let last_id = filtered
        .last()
        .map(|e| e.id.map(|i| i.to_string()).unwrap_or_else(|| format!("{}{}", e.tick, e.kind)))
        .unwrap_or_default();
    let sig = format!("{}|{}|{total}|{last_id}", ui.tab, ui.page);
    if sig == ui.sig {
        return;
    }
    ui.sig = sig;

    if let Ok(mut h) = q_hint.single_mut() {
        *h = if live && total == 0 {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if let Ok(mut t) = q_page.single_mut() {
        let label = format!("{total}건 · {}/{}", ui.page + 1, pages);
        if **t != label {
            **t = label;
        }
    }
    let Ok((list_e, children)) = q_list.single() else {
        return;
    };
    let stale: Vec<Entity> = children.map(|c| c.to_vec()).unwrap_or_default();
    for c in stale {
        commands.entity(c).despawn();
    }
    if !live || total == 0 {
        return;
    }
    // 최신 페이지 기본 — 창은 연대순(오래된→최신)으로 보여준다(FE 로그 방향).
    let end = total - ui.page * PER_PAGE;
    let start = end.saturating_sub(PER_PAGE);
    for ev in &filtered[start..end] {
        let text = i18n::event_text(&ev.payload);
        let color = i18n::event_color(&ev.kind);
        let row = commands
            .spawn(Node {
                flex_direction: FlexDirection::Row,
                column_gap: Val::Px(6.0),
                align_items: AlignItems::Baseline,
                ..default()
            })
            .with_children(|r| {
                r.spawn((Text::new(i18n::ev_kind_icon(&ev.kind)), theme.text(9.0, color)));
                if let Some(ts) = &ev.ts {
                    let hm = i18n::fmt_time(ts);
                    if !hm.is_empty() {
                        r.spawn((Text::new(hm), theme.text(8.0, theme::INK400)));
                    }
                }
                r.spawn((Text::new(text), theme.text(9.0, color)));
            })
            .id();
        commands.entity(list_e).add_child(row);
    }
}
