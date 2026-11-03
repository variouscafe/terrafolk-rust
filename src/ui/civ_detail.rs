//! 발명 상세 카드 — civ.rs에서 분리(500줄 규칙·기능별 분리, 2026-11-03).
//! 발명 탭 진입 시 목록 대신 그려지는 카드(범주 심볼·재료 목록·태양력
//! 날짜·개선형 배지·§17.86 스프라이트/생김새 서술)의 조립을 소유한다.
//! 목록·요약·연대기는 civ.rs.

#[derive(Component)]
pub(crate) struct TextLine;

/// 상세 카드 닫기(✕) 버튼.
#[derive(Component)]
pub(super) struct CivDetailClose;

fn on_detail_close(_: On<Pointer<Click>>, mut detail: ResMut<CivDetail>) {
    detail.0 = None;
}


use bevy::prelude::*;

use super::civ::{CivDetail, InventionRow};
use super::i18n;
use super::theme::{self, UiTheme};

/// 발명 상세 카드 — 아이콘 대신 범주 심볼, 재료 목록·만든 날짜(태양력)·설명.
pub(super) fn spawn_detail(
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
