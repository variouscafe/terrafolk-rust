//! 외형 편집 팝업 뷰 — 셸·행 스폰(look.rs가 상태·옵저버 소유, 이 파일은
//! 렌더 조립만). 컴포넌트 마커·재구성 트리거는 look.rs에 있다.

use bevy::prelude::*;

use super::look::{EditorData, LookSlide, LookSwatch, RowTarget, Tab};
use super::look::{on_slide_click, on_swatch_click, part_options, wear_options};
use super::look_data;
use super::theme::{self, UiTheme};

/// 탭별 행 스폰 — 옵션 행(◀ 이름 ▶)·색 점 행.
pub(super) fn build_rows(rows: &mut ChildSpawnerCommands, theme: &UiTheme, d: &EditorData) {
    match d.tab {
        Tab::Clothes => {
            for (slot, label) in [("top", "상의"), ("bottom", "하의"), ("feet", "신발")] {
                let opts = wear_options(d, slot);
                let cur = d.wears.get(slot).cloned().unwrap_or("auto".into());
                spawn_opt_row(
                    rows,
                    theme,
                    label,
                    &opts,
                    &cur,
                    RowTarget::Wear(slot),
                    d,
                    None,
                );
            }
        }
        Tab::Hair => {
            let hairs = d.hairs().to_vec();
            let cur = d.part("hair", &hairs);
            let opts: Vec<(String, bool)> =
                hairs.iter().map(|k| ((*k).to_owned(), false)).collect();
            spawn_opt_row(
                rows,
                theme,
                "헤어스타일",
                &opts,
                &cur,
                RowTarget::Part("hair"),
                d,
                None,
            );
            spawn_swatch_row(rows, theme, "머리색", "hairColor", d);
        }
        Tab::Face => {
            spawn_swatch_row(rows, theme, "피부", "skin", d);
            spawn_swatch_row(rows, theme, "눈", "eye", d);
            let beards = d.beards().to_vec();
            let cur = d.part("beard", &beards);
            let opts: Vec<(String, bool)> =
                beards.iter().map(|k| ((*k).to_owned(), false)).collect();
            spawn_opt_row(
                rows,
                theme,
                "수염",
                &opts,
                &cur,
                RowTarget::Part("beard"),
                d,
                None,
            );
            let glasses = d.glasses().to_vec();
            let cur = d.part("glasses", &glasses);
            let opts: Vec<(String, bool)> =
                glasses.iter().map(|k| ((*k).to_owned(), false)).collect();
            spawn_opt_row(
                rows,
                theme,
                "안경",
                &opts,
                &cur,
                RowTarget::Part("glasses"),
                d,
                None,
            );
        }
    }
}

/// 옵션 행 — (키, 잠김) 목록에서 현재값 카드 + 좌우 슬라이드.
#[allow(clippy::too_many_arguments)]
fn spawn_opt_row(
    rows: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    label: &str,
    opts: &[(String, bool)],
    current: &str,
    target: RowTarget,
    d: &EditorData,
    swatch_kind: Option<&str>,
) {
    let idx = opts.iter().position(|(k, _)| k == current).unwrap_or(0);
    let (cur_key, locked) = opts
        .get(idx)
        .map(|(k, l)| (k.clone(), *l))
        .unwrap_or_else(|| ("none".to_owned(), false));
    let name = if cur_key == "auto" {
        "자동".to_owned()
    } else if cur_key == "none" {
        "없음".to_owned()
    } else {
        d.pretty(&cur_key)
    };
    rows.spawn(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Px(6.0),
        ..default()
    })
    .with_children(|r| {
        r.spawn((
            Text::new(label),
            theme.label(10.0),
            Node {
                width: Val::Px(58.0),
                ..default()
            },
        ));
        slide_btn(r, theme, target.clone(), -1);
        r.spawn((
            Node {
                flex_direction: FlexDirection::Row,
                align_items: AlignItems::Center,
                column_gap: Val::Px(6.0),
                flex_grow: 1.0,
                padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                ..default()
            },
            BackgroundColor(theme::INK800),
        ))
        .with_children(|c| {
            if let Some(kind) = swatch_kind
                && let Some((_, _, hex)) = look_data::SWATCH
                    .iter()
                    .find(|(k, key, _)| *k == kind && *key == cur_key)
            {
                c.spawn((
                    Node {
                        width: Val::Px(10.0),
                        height: Val::Px(10.0),
                        ..default()
                    },
                    BackgroundColor(hex_color(hex, 1.0).unwrap_or(theme::ACCENT)),
                ));
            }
            c.spawn((
                Text::new(if locked { format!("🔒 {name}") } else { name }),
                theme.label(10.0),
            ));
            c.spawn((
                Text::new(format!("{}/{}", idx + 1, opts.len())),
                theme.text(8.0, theme::INK400),
            ));
        });
        slide_btn(r, theme, target, 1);
    });
}

fn slide_btn(r: &mut ChildSpawnerCommands, theme: &UiTheme, target: RowTarget, dir: i8) {
    r.spawn((
        LookSlide(target, dir),
        Button,
        Node {
            padding: UiRect::axes(Val::Px(8.0), Val::Px(4.0)),
            ..default()
        },
        BackgroundColor(theme::INK700),
        Interaction::default(),
    ))
    .observe(on_slide_click)
    .with_children(|b| {
        b.spawn((
            Text::new(if dir < 0 { "◀" } else { "▶" }),
            theme.label(10.0),
        ));
    });
}

/// 색 점 행 — 스와치가 있는 색만(웹 swatchRowHtml `if (!hex) return ''` 패리티).
fn spawn_swatch_row(
    rows: &mut ChildSpawnerCommands,
    theme: &UiTheme,
    label: &str,
    key: &'static str,
    d: &EditorData,
) {
    let cur = d.part(key, &part_options(d, key));
    rows.spawn(Node {
        flex_direction: FlexDirection::Row,
        align_items: AlignItems::Center,
        column_gap: Val::Px(6.0),
        ..default()
    })
    .with_children(|r| {
        r.spawn((
            Text::new(label),
            theme.label(10.0),
            Node {
                width: Val::Px(58.0),
                ..default()
            },
        ));
        let opts = part_options(d, key);
        for k in opts {
            let Some((_, _, hex)) = look_data::SWATCH
                .iter()
                .find(|(kd, key2, _)| *kd == key && *key2 == k)
            else {
                continue;
            };
            let on = *k == cur;
            r.spawn((
                LookSwatch(key, (*k).to_owned()),
                Button,
                Node {
                    width: Val::Px(if on { 22.0 } else { 18.0 }),
                    height: Val::Px(if on { 22.0 } else { 18.0 }),
                    ..default()
                },
                BackgroundColor(
                    hex_color(hex, if on { 1.0 } else { 0.55 }).unwrap_or(theme::ACCENT),
                ),
                Interaction::default(),
            ))
            .observe(on_swatch_click);
        }
    });
}

/// "#RRGGBB" → Color(알파 지정). 3D 틴트 팔레트 hex(look_data SWATCH) 전용.
fn hex_color(hex: &str, alpha: f32) -> Option<Color> {
    let h = hex.strip_prefix('#')?;
    if h.len() != 6 {
        return None;
    }
    let byte = |i: usize| u8::from_str_radix(h.get(i..i + 2)?, 16).ok();
    Some(Color::srgba(
        byte(0)? as f32 / 255.0,
        byte(2)? as f32 / 255.0,
        byte(4)? as f32 / 255.0,
        alpha,
    ))
}
