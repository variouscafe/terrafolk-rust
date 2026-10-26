//! 월드 툴팁(§17.78) — 주민(호버/탭)·무덤(tip.grave)·건물(tip.hut.*) 문구.
//! 데스크톱은 커서 호버로 상시 갱신, 모바일은 탭 시 2.5s 고정(FE bldRender
//! 터치 규약). 무덤·건물 판정은 NetState.frame.resources(권위 프레임)에서
//! 직접 — 발판 rect 포함, 무덤은 중심 1.1타일 근접(hover_res와 동일 채도).

use bevy::picking::Pickable;
use bevy::prelude::*;

use super::i18n;
use super::theme::{self, UiTheme};
use crate::net::NetState;
use crate::net::agent_hud::AgentHud;
use crate::net::apply::LiveAgentTag;
use crate::net::visual_map::{ResState, bld_size};
use crate::world::pick::{GroundTap, Selection};
use crate::world::villager::Villager;

/// 탭 툴팁 노출 시간(FE 터치 고정 2.5s).
const TAP_SECS: f32 = 2.5;
/// 호버 리프레시(커서 이탈 후 잔상).
const HOVER_LINGER: f32 = 0.15;
const TORSO_H: f32 = 0.8;
const PICK_RADIUS: f32 = 0.8;

#[derive(Component)]
pub struct TooltipNode {
    text: Entity,
    until: f32,
}

pub fn spawn_tooltip(mut commands: Commands, theme: Res<UiTheme>) {
    let text = commands
        .spawn((
            Pickable::IGNORE,
            Text::new(""),
            theme.text(10.0, Color::srgb(0.961, 0.949, 0.910)),
        ))
        .id();
    commands
        .spawn((
            TooltipNode {
                text,
                until: 0.0,
            },
            Pickable::IGNORE,
            Node {
                position_type: PositionType::Absolute,
                max_width: Val::Px(220.0),
                padding: UiRect::all(Val::Px(6.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.85)),
            Visibility::Hidden,
        ))
        .add_child(text);
}

/// 툴팁 갱신 — (1) 탭: Selection.epoch(주민)·GroundTap.epoch(무덤·건물) 에지에서
/// 2.5s 고정. (2) 데스크톱 호버: 커서 광선으로 주민·지면 판정, linger 갱신.
#[allow(clippy::type_complexity)]
pub fn update_tooltip(
    time: Res<Time>,
    window: Single<&Window>,
    rig: Res<crate::world::camera_rig::CameraRig>,
    world: Res<crate::world::data::WorldData>,
    q_cam: Query<(&Camera, &GlobalTransform), With<Camera3d>>,
    sel: Res<Selection>,
    tap: Res<GroundTap>,
    ui_hover: Res<crate::platform::UiHover>,
    net: Res<NetState>,
    villagers: Query<(&GlobalTransform, &Villager, Option<&LiveAgentTag>, Option<&AgentHud>)>,
    mut q_node: Query<(&mut TooltipNode, &mut Node, &mut Visibility)>,
    mut q_text: Query<&mut Text, Without<TooltipNode>>,
    mut last_sel: Local<u32>,
    mut last_tap: Local<u32>,
) {
    let t = time.elapsed_secs();
    let Ok((mut node, mut nstyle, mut vis)) = q_node.single_mut() else {
        return;
    };
    let mut show: Option<(String, Vec2)> = None;
    // (1) 탭 — 주민 선택 에지: 머리 위 투영 위치에 2.5s
    if sel.epoch != *last_sel {
        *last_sel = sel.epoch;
        if let Some(entity) = sel.entity
            && let Ok((gtr, v, _tag, hud)) = villagers.get(entity)
            && let Ok((cam, gt)) = q_cam.single()
            // 워프 공간 투영(글로브 정합 — 평면에서는 기존과 동일)
            && let Ok(p) = cam.world_to_viewport(gt, gtr.translation() + gtr.up() * 2.0)
        {
            let act = hud
                .map(|h| i18n::act_label(&h.act_type))
                .filter(|a| !a.is_empty())
                .unwrap_or_else(|| theme::status_label(&v.status));
            let prog = hud
                .filter(|h| h.act_progress > 0.0)
                .map(|h| format!(" ({:.0}%)", h.act_progress * 100.0))
                .unwrap_or_default();
            show = Some((format!("{}\n{}{}", v.name, act, prog), Vec2::new(p.x, p.y)));
            node.until = t + TAP_SECS;
        }
    }
    // (1) 탭 — 빈 땅 에지: 무덤·건물 판정(지면점 투영 위치)
    if tap.epoch != *last_tap {
        *last_tap = tap.epoch;
        if show.is_none()
            && let Ok((cam, gt)) = q_cam.single()
            && let Some(g) = crate::world::camera_input::ground_at(
                cam, gt, tap.px, 0.0,
                crate::world::globe::WarpFrame::active(&world, rig.zoom).as_ref(),
            )
            && let Some(text) = res_tip(&net, g.x, g.z)
            && let Ok(p) = cam.world_to_viewport(gt, g)
        {
            show = Some((text, Vec2::new(p.x, p.y)));
            node.until = t + TAP_SECS;
        }
    }
    // (2) 데스크톱 호버 — 커서 있을 때만(모바일 cursor None). 주민 우선, 없으면 지면.
    if let Some(cursor) = window.cursor_position().filter(|_| !ui_hover.0)
        && let Ok((cam, gt)) = q_cam.single()
    {
        let ray = cam.viewport_to_world(gt, cursor).ok();
        let mut vill: Option<(f32, String)> = None;
        if let Some(ray) = ray.as_ref() {
            for (gtr, v, _tag, hud) in &villagers {
                // 워프 공간 판정(글로브 정합 — 평면에서는 기존과 동일)
                let center = gtr.translation() + gtr.up() * TORSO_H;
                let dv = center - ray.origin;
                let along = dv.dot(*ray.direction);
                if along <= 0.0 {
                    continue;
                }
                let d = (dv - *ray.direction * along).length();
                if d > PICK_RADIUS {
                    continue;
                }
                let act = hud
                    .map(|h| i18n::act_label(&h.act_type))
                    .filter(|a| !a.is_empty())
                    .unwrap_or_else(|| theme::status_label(&v.status));
                let prog = hud
                    .filter(|h| h.act_progress > 0.0)
                    .map(|h| format!(" ({:.0}%)", h.act_progress * 100.0))
                    .unwrap_or_default();
                if vill.as_ref().is_none_or(|(bd, _)| d < *bd) {
                    vill = Some((d, format!("{}\n{}{}", v.name, act, prog)));
                }
            }
        }
        if let Some((_, text)) = vill {
            show = Some((text, cursor));
            node.until = t + HOVER_LINGER;
        } else if let Some(g) = crate::world::camera_input::ground_at(
            cam, gt, cursor, 0.0,
            crate::world::globe::WarpFrame::active(&world, rig.zoom).as_ref(),
        )
            && let Some(text) = res_tip(&net, g.x, g.z)
        {
            show = Some((text, cursor));
            node.until = t + HOVER_LINGER;
        }
    }
    if let Some((text, at)) = show {
        if let Ok(mut txt) = q_text.get_mut(node.text) {
            **txt = text;
        }
        place(&mut nstyle, &window, at);
        *vis = Visibility::Visible;
    } else if node.until < t {
        *vis = Visibility::Hidden;
    }
}

fn place(node: &mut Node, window: &Window, at: Vec2) {
    let x = at.x.clamp(4.0, (window.width() - 226.0).max(4.0));
    let y = at.y.clamp(4.0, (window.height() - 80.0).max(4.0));
    node.left = Val::Px(x + 10.0);
    node.top = Val::Px(y - 8.0);
}

/// 무덤·건물 툴팁 문구 — 프레임 resources에서 판정(발판 rect 포함,
/// 무덤 1.1타일 근접 — hover_res 규약 동일).
fn res_tip(net: &NetState, gx: f32, gz: f32) -> Option<String> {
    let frame = net.frame.as_ref()?;
    let (tx, tz) = (gx.floor() as i32, gz.floor() as i32);
    let mut hit: Option<(&crate::net::LiveResource, f32)> = None;
    for r in &frame.resources {
        match r.kind.as_str() {
            "hut" => {
                let st: ResState = parse(r);
                let (w, h) = bld_size(&st, st.bld_type.as_deref().unwrap_or("HUT"));
                let inside = (tx >= r.x as i32 && tx < r.x as i32 + w as i32)
                    && (tz >= r.y as i32 && tz < r.y as i32 + h as i32);
                if inside {
                    hit = Some((r, 0.0));
                    break;
                }
            }
            // 무덤·유물 — 중심 1.1타일 근접(hover_res 규약 동일)
            "grave" | "relic" => {
                let d = (r.x as f32 - gx).powi(2) + (r.y as f32 - gz).powi(2);
                if d < 1.1 * 1.1 && hit.as_ref().is_none_or(|(_, bd)| d < *bd) {
                    hit = Some((r, d));
                }
            }
            _ => {}
        }
    }
    let (r, _) = hit?;
    let st: ResState = parse(r);
    match r.kind.as_str() {
        "grave" => {
            let name = st
                .name
                .clone()
                .unwrap_or_else(|| i18n::t_or("tip.grave.someone", "누군가"));
            let mut s = i18n::t_fmt(
                "tip.grave",
                &[
                    ("name", name.as_str()),
                    ("age", &st.age.unwrap_or(0).to_string()),
                ],
            );
            if let Some(ep) = st.epitaph.as_deref().filter(|e| !e.is_empty()) {
                s.push_str(&format!("\n「{ep}」"));
            }
            Some(s)
        }
        "relic" => {
            // 발명품 — 이름(species) + 발명가 + 설명(§17.38 FE 툴팁 직역)
            let mut s = format!(
                "{}\n{}",
                r.species,
                i18n::t_fmt("tip.relic", &[("by", st.by.as_deref().unwrap_or("?"))])
            );
            if let Some(p) = st.purpose.as_deref().filter(|p| !p.is_empty()) {
                s.push_str(&format!("\n{}", i18n::fx(p)));
            }
            Some(s)
        }
        "hut" => {
            let bld = i18n::vocab_label("bld", st.bld_type.as_deref().unwrap_or("HUT"));
            let xy = [("x", r.x.to_string()), ("y", r.y.to_string())];
            let _ = &xy;
            if st.stage.as_deref().unwrap_or("COMPLETED") != "COMPLETED" {
                Some(i18n::t_fmt(
                    "tip.hut.site",
                    &[
                        ("name", st.by.as_deref().unwrap_or("?")),
                        ("bld", bld.as_str()),
                        ("p", &(st.progress.unwrap_or(0.0) * 100.0).round().to_string()),
                    ],
                ))
            } else if let Some(owner) = st.owner.as_deref().filter(|o| !o.is_empty()) {
                let shown = st
                    .owner_name
                    .as_deref()
                    .filter(|n| !n.is_empty())
                    .unwrap_or(owner);
                Some(i18n::t_fmt(
                    "tip.hut.home",
                    &[
                        ("name", shown),
                        ("bld", bld.as_str()),
                        ("x", &r.x.to_string()),
                        ("y", &r.y.to_string()),
                    ],
                ))
            } else {
                Some(i18n::t_fmt(
                    "tip.hut.vacant",
                    &[
                        ("bld", bld.as_str()),
                        ("x", &r.x.to_string()),
                        ("y", &r.y.to_string()),
                    ],
                ))
            }
        }
        _ => None,
    }
}

fn parse(r: &crate::net::LiveResource) -> ResState {
    serde_json::from_str(r.state.as_deref().unwrap_or("{}")).unwrap_or_default()
}
