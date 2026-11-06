//! 재질 색칠 판별 — `?tfdbg=matcolor[,matname]` (평시 비용 0, 진단 전용·복원 없음).
//! 파란 조각이 보이는 스크린샷 한 장으로 재질 계열을 특정한다. 계열별 고정 언릿 색:
//! Standard unlit=마젠타, Standard lit=노랑, 툰=초록, 잉크=빨강, 바람(ExtendedMaterial)=시안.
//! Terrain·Water는 교체하지 않는다(기준 배경). `matname`이면 (계열,이름) 해시로 12색
//! 팔레트를 배정하고 색→이름 표를 `[matname] {...}`로 600프레임마다(첫 300) 출력한다.

use std::collections::{BTreeMap, BTreeSet};

use bevy::prelude::*;

use super::toon::ToonMaterial;
use super::wind::WindMaterial;
use crate::character::ink::InkMaterial;

/// 교체 전 원본이 있었다는 표식(재처리 방지).
#[derive(Component)]
pub struct OrigMat;

const PALETTE: [(f32, f32, f32); 12] = [
    (1.0, 0.0, 0.0),
    (0.0, 1.0, 0.0),
    (0.0, 0.0, 1.0),
    (1.0, 1.0, 0.0),
    (1.0, 0.0, 1.0),
    (0.0, 1.0, 1.0),
    (1.0, 0.5, 0.0),
    (0.5, 0.0, 1.0),
    (1.0, 1.0, 1.0),
    (0.0, 0.0, 0.0),
    (0.5, 0.5, 0.0),
    (1.0, 0.5, 0.7),
];

fn fnv(s: &str) -> usize {
    let mut h: u32 = 0x811c9dc5;
    for b in s.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    h as usize
}

fn solid(c: (f32, f32, f32)) -> StandardMaterial {
    StandardMaterial {
        base_color: Color::srgb(c.0, c.1, c.2),
        unlit: true,
        ..default()
    }
}

#[derive(Default)]
pub struct State {
    frame: u32,
    class: Vec<Handle<StandardMaterial>>, // 0 unlit 1 lit 2 toon 3 ink 4 wind
    pal: Vec<Handle<StandardMaterial>>,
    table: BTreeMap<usize, BTreeSet<String>>,
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)]
pub fn mat_color_probe(
    mut st: Local<State>,
    mut commands: Commands,
    mut mats: ResMut<Assets<StandardMaterial>>,
    names: Query<&Name>,
    parents: Query<&ChildOf>,
    q_std: Query<(Entity, &MeshMaterial3d<StandardMaterial>), (With<Mesh3d>, Without<OrigMat>)>,
    q_toon: Query<Entity, With<MeshMaterial3d<ToonMaterial>>>,
    q_ink: Query<Entity, With<MeshMaterial3d<InkMaterial>>>,
    q_wind: Query<Entity, With<MeshMaterial3d<WindMaterial>>>,
) {
    if !crate::net::dbg_gate::on("matcolor") {
        return;
    }
    let by_name = crate::net::dbg_gate::on("matname");
    if st.class.is_empty() {
        let cols = [
            (1.0, 0.0, 1.0),
            (1.0, 1.0, 0.0),
            (0.0, 1.0, 0.0),
            (1.0, 0.0, 0.0),
            (0.0, 1.0, 1.0),
        ];
        st.class = cols.iter().map(|c| mats.add(solid(*c))).collect();
        st.pal = PALETTE.iter().map(|c| mats.add(solid(*c))).collect();
    }
    st.frame += 1;
    // 가장 가까운 Name(자신 또는 조상)
    let name_of = |e: Entity| -> String {
        let mut cur = e;
        loop {
            if let Ok(n) = names.get(cur) {
                return n.as_str().to_string();
            }
            match parents.get(cur) {
                Ok(p) => cur = p.parent(),
                Err(_) => return "?".into(),
            }
        }
    };
    let mut pick = |st: &mut State, class: usize, label: &str, e: Entity| {
        if !by_name {
            return st.class[class].clone();
        }
        let n = name_of(e);
        let idx = fnv(&format!("{label}:{n}")) % PALETTE.len();
        st.table
            .entry(idx)
            .or_default()
            .insert(format!("{label}:{n}"));
        st.pal[idx].clone()
    };
    // 교체 대상 열거 — 터레인·수면은 이 쿼리에 안 걸린다(StandardMaterial 아님/Water 타입).
    for (e, h) in &q_std {
        let unlit = mats.get(&h.0).map(|m| m.unlit).unwrap_or(false);
        let (c, label) = if unlit {
            (0, "std-unlit")
        } else {
            (1, "std-lit")
        };
        let new = pick(&mut st, c, label, e);
        commands
            .entity(e)
            .try_insert((OrigMat, MeshMaterial3d(new)));
    }
    for e in &q_toon {
        let new = pick(&mut st, 2, "toon", e);
        commands
            .entity(e)
            .try_remove::<MeshMaterial3d<ToonMaterial>>()
            .try_insert((OrigMat, MeshMaterial3d(new)));
    }
    for e in &q_ink {
        let new = pick(&mut st, 3, "ink", e);
        commands
            .entity(e)
            .try_remove::<MeshMaterial3d<InkMaterial>>()
            .try_insert((OrigMat, MeshMaterial3d(new)));
    }
    for e in &q_wind {
        let new = pick(&mut st, 4, "wind", e);
        commands
            .entity(e)
            .try_remove::<MeshMaterial3d<WindMaterial>>()
            .try_insert((OrigMat, MeshMaterial3d(new)));
    }
    if by_name && st.frame >= 300 && (st.frame - 300).is_multiple_of(600) {
        let rows: Vec<String> = st
            .table
            .iter()
            .map(|(i, v)| {
                let c = PALETTE[*i];
                let list: Vec<&String> = v.iter().take(12).collect();
                format!("\"{},{},{}\":{:?}", c.0, c.1, c.2, list)
            })
            .collect();
        info!("[matname] {{{}}}", rows.join(","));
    }
}
