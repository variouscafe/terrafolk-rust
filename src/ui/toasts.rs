//! 하이라이트 토스트 — 우하단, 최대 4개·6s 후 소멸(FE pushToast·tf-world-ui
//! pushToast 직역). 이벤트는 룸 WS 실시간분만(HTTP 백로그 시딩은 안 뜬다 —
//! FE와 동일 규약: 접속 시딩은 setEvents, pushHighlight는 onEvent 전용).
//! highlight 판정은 i18n::highlight_of(20조건 직역)를 name_of 주입으로 공유.

use bevy::picking::Pickable;
use bevy::prelude::*;

use super::i18n;
use super::theme::{self, UiTheme};
use crate::net::apply::LiveAgentTag;
use crate::net::feed::{EventLog, ToastQ};
use crate::world::villager::Villager;

const TOAST_LIFE: f32 = 6.0;
const TOAST_FADE: f32 = 0.5;
const TOAST_MAX: usize = 4;

#[derive(Component)]
pub struct ToastHost;

/// 토스트 행 — text는 외동 Text 자식 엔티티(페이드 시 색도 함께 연산).
#[derive(Component)]
pub struct ToastRow {
    life: f32,
    text: Entity,
}

pub fn spawn_toasts(mut commands: Commands) {
    commands.spawn((
        ToastHost,
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            bottom: Val::Px(12.0),
            right: Val::Px(12.0),
            flex_direction: FlexDirection::Column,
            row_gap: Val::Px(4.0),
            align_items: AlignItems::FlexEnd,
            max_width: Val::Px(300.0),
            ..default()
        },
    ));
}

/// 새 이벤트 → 하이라이트 판정 → 토스트 스폰. 수명 감산·페이드·초과분 제거.
#[allow(clippy::type_complexity)]
pub fn update_toasts(
    time: Res<Time>,
    theme: Res<UiTheme>,
    mut evlog: ResMut<EventLog>,
    mut toastq: ResMut<ToastQ>,
    villagers: Query<(&LiveAgentTag, &Villager)>,
    mut commands: Commands,
    q_host: Query<Entity, (With<ToastHost>, Without<ToastRow>)>,
    mut q_rows: Query<(Entity, &mut ToastRow, &mut BackgroundColor)>,
    mut q_texts: Query<&mut TextColor, Without<ToastRow>>,
) {
    // 이름 조회 — 주민 id → 표시명(하이라이트 who).
    let name_of = |id: &str| -> String {
        villagers
            .iter()
            .find(|(t, _)| t.0 == id)
            .map(|(_, v)| v.name.clone())
            .unwrap_or_default()
    };
    if evlog.seen < evlog.rows.len() {
        let fresh: Vec<_> = evlog.rows.iter().skip(evlog.seen).cloned().collect();
        evlog.seen = evlog.rows.len();
        for ev in &fresh {
            if let Some(toast) = i18n::highlight_of(ev, &name_of) {
                toastq.pending.push(toast);
            }
        }
    }
    // 스폰
    if !toastq.pending.is_empty() && let Ok(host) = q_host.single() {
        for t in toastq.pending.drain(..) {
            let body = if t.who.is_empty() {
                t.text.clone()
            } else {
                format!("{} — {}", t.who, t.text)
            };
            let text_e = commands
                .spawn((
                    Pickable::IGNORE,
                    Text::new(format!("{} {}", t.icon, body)),
                    theme.text(9.5, theme::INK200),
                ))
                .id();
            let row = commands
                .spawn((
                    ToastRow {
                        life: TOAST_LIFE,
                        text: text_e,
                    },
                    Pickable::IGNORE,
                    Node {
                        padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                        max_width: Val::Px(300.0),
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.043, 0.055, 0.078, 0.92)),
                ))
                .add_child(text_e)
                .id();
            commands.entity(host).add_child(row);
        }
        // 초과분 — 가장 오래된(첫) 행부터 제거(FE removeChild(firstChild))
        let count = q_rows.iter().count();
        if count + 1 > TOAST_MAX {
            let excess = count + 1 - TOAST_MAX;
            let stale: Vec<Entity> = q_rows.iter().take(excess).map(|(e, _, _)| e).collect();
            for e in stale {
                commands.entity(e).despawn();
            }
        }
    }
    // 수명·페이드
    let dt = time.delta_secs();
    for (e, mut row, mut bg) in &mut q_rows {
        row.life -= dt;
        if row.life <= 0.0 {
            commands.entity(e).despawn();
            continue;
        }
        if row.life < TOAST_FADE {
            let a = (row.life / TOAST_FADE).clamp(0.0, 1.0);
            bg.0 = Color::srgba(0.043, 0.055, 0.078, 0.92 * a);
            if let Ok(mut color) = q_texts.get_mut(row.text) {
                color.0.set_alpha(a * 1.0);
            }
        }
    }
}
