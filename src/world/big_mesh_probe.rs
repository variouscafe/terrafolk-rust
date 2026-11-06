//! 거대 메시 프로브 — `?tfdbg=bigmesh` (평시 비용 0). 글로브 파란 조각 범인 특정용.
//! PostUpdate(전파 이후) 120프레임마다(최대 30회) Mesh3d 전수의 월드 범위를 재서
//! extent>8 | joint_spread>3 | joint_reach>4 인 상위 10개를 `[bigmesh] {json}` 한 줄로 출력.

use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::{Mesh, VertexAttributeValues};
use bevy::prelude::*;

use super::globe::GlobeExempt;
use super::prop_fit::{MeshAabbCache, mesh_aabb};
use super::terrain::TerrainMesh;
use super::toon::ToonMaterial;
use super::villager::Villager;
use super::water::{WaterMaterial, WaterMesh};
use crate::character::ink::InkMaterial;

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn big_mesh_probe(
    mut frame: Local<u32>,
    mut shots: Local<u32>,
    meshes: Res<Assets<Mesh>>,
    ibms: Res<Assets<SkinnedMeshInverseBindposes>>,
    mut cache: ResMut<MeshAabbCache>,
    std_mats: Res<Assets<StandardMaterial>>,
    toon_mats: Res<Assets<ToonMaterial>>,
    q: Query<
        (
            Entity,
            &Mesh3d,
            &GlobalTransform,
            Option<&SkinnedMesh>,
            Option<&MeshMaterial3d<StandardMaterial>>,
            Option<&MeshMaterial3d<ToonMaterial>>,
            Option<&MeshMaterial3d<InkMaterial>>,
            Option<&MeshMaterial3d<WaterMaterial>>,
            Option<&Visibility>,
            Option<&ViewVisibility>,
            Option<&InheritedVisibility>,
        ),
        (Without<TerrainMesh>, Without<WaterMesh>),
    >,
    names: Query<&Name>,
    parents: Query<&ChildOf>,
    gts: Query<&GlobalTransform>,
    exempt: Query<(), With<GlobeExempt>>,
    villagers: Query<(), With<Villager>>,
) {
    if !crate::net::dbg_gate::on("bigmesh") {
        return;
    }
    *frame += 1;
    if *shots >= 30 || !(*frame).is_multiple_of(120) {
        return;
    }
    *shots += 1;
    let mut rows: Vec<(f32, String)> = Vec::new();
    for (e, mh, gt, skin, std_m, toon_m, ink_m, water_m, vis, view_vis, inh_vis) in &q {
        // 조상 체인(루트까지, 최대 8) + 하늘 제외
        let mut chain = vec![
            names
                .get(e)
                .map(|n| n.as_str().to_string())
                .unwrap_or("?".into()),
        ];
        let mut cur = e;
        let mut root = e;
        let mut sky = exempt.get(e).is_ok();
        let mut has_vill = villagers.get(e).is_ok();
        while let Ok(p) = parents.get(cur) {
            cur = p.parent();
            root = cur;
            sky |= exempt.get(cur).is_ok();
            has_vill |= villagers.get(cur).is_ok();
            if chain.len() < 8 {
                chain.push(
                    names
                        .get(cur)
                        .map(|n| n.as_str().to_string())
                        .unwrap_or("?".into()),
                );
            }
        }
        if sky {
            continue;
        }
        let Some(mesh) = meshes.get(&mh.0) else {
            continue;
        };
        let (mn, mx) = match cache.0.get(&mh.id()) {
            Some(c) => *c,
            None => {
                let Some(a) = mesh_aabb(mesh) else { continue };
                cache.0.insert(mh.id(), a);
                a
            }
        };
        let m = gt.to_matrix();
        let mut pts = Vec::with_capacity(8);
        for x in [mn.x, mx.x] {
            for y in [mn.y, mx.y] {
                for z in [mn.z, mx.z] {
                    pts.push(m.transform_point3(Vec3::new(x, y, z)));
                }
            }
        }
        let mut extent = 0.0f32;
        for a in &pts {
            for b in &pts {
                extent = extent.max(a.distance(*b));
            }
        }
        // 스킨 메시의 실제 월드 범위 — 렌더는 Σ w·(조인트GT·IBM)·p (메시 자체 GT 무시).
        // 스킨 경로 진단 — 'ok'면 조인트 행렬 수식 적용, 아니면 실패 원인(ibm/pos/ji/jw)
        let skin_path = match skin {
            None => "unskinned",
            Some(sk) if ibms.get(&sk.inverse_bindposes).is_none() => "no-ibm",
            Some(_)
                if !matches!(
                    mesh.attribute(Mesh::ATTRIBUTE_POSITION),
                    Some(VertexAttributeValues::Float32x3(_))
                ) =>
            {
                "pos-fmt"
            }
            Some(_)
                if !matches!(
                    mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX),
                    Some(VertexAttributeValues::Uint16x4(_))
                ) =>
            {
                "ji-fmt"
            }
            Some(_)
                if !matches!(
                    mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT),
                    Some(VertexAttributeValues::Float32x4(_))
                ) =>
            {
                "jw-fmt"
            }
            Some(_) => "ok",
        };
        if let Some(sk) = skin
            && let Some(ib) = ibms.get(&sk.inverse_bindposes)
            && let Some(VertexAttributeValues::Float32x3(pos)) =
                mesh.attribute(Mesh::ATTRIBUTE_POSITION)
            && let Some(VertexAttributeValues::Uint16x4(ji)) =
                mesh.attribute(Mesh::ATTRIBUTE_JOINT_INDEX)
            && let Some(VertexAttributeValues::Float32x4(jw)) =
                mesh.attribute(Mesh::ATTRIBUTE_JOINT_WEIGHT)
        {
            let jm: Vec<Mat4> = sk
                .joints
                .iter()
                .enumerate()
                .map(|(i, j)| {
                    gts.get(*j).map(|g| g.to_matrix()).unwrap_or(Mat4::IDENTITY)
                        * ib.get(i).copied().unwrap_or(Mat4::IDENTITY)
                })
                .collect();
            let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for ((p, j), w) in pos.iter().zip(ji.iter()).zip(jw.iter()) {
                let p4 = Vec3::from(*p).extend(1.0);
                let mut acc = Vec4::ZERO;
                for k in 0..4 {
                    if w[k] > 0.0
                        && let Some(m) = jm.get(j[k] as usize)
                    {
                        acc += w[k] * (*m * p4);
                    }
                }
                let v = acc.truncate();
                lo = lo.min(v);
                hi = hi.max(v);
            }
            if lo.x <= hi.x {
                extent = lo.distance(hi);
            }
        }
        let (mut spread, mut reach) = (0.0f32, 0.0f32);
        if let Some(s) = skin {
            let js: Vec<Vec3> = s
                .joints
                .iter()
                .filter_map(|j| gts.get(*j).ok().map(|g| g.translation()))
                .collect();
            let rt = gts.get(root).map(|g| g.translation()).unwrap_or(Vec3::ZERO);
            for a in &js {
                reach = reach.max(a.distance(rt));
                for b in &js {
                    spread = spread.max(a.distance(*b));
                }
            }
        }
        if ink_m.is_some() {
            continue; // 잉크 헐은 본체 메시와 동일 스켈레톤 — 본체 행만 본다
        }
        if extent <= 8.0 {
            continue;
        }
        let (kind, rgb, unlit, alpha) = if let Some(h) = std_m {
            let s = std_mats.get(&h.0);
            let c = s.map(|s| s.base_color.to_srgba()).unwrap_or(Srgba::NONE);
            (
                "Standard",
                [c.red, c.green, c.blue],
                s.map(|s| s.unlit).unwrap_or(false),
                s.map(|s| format!("{:?}", s.alpha_mode)).unwrap_or_default(),
            )
        } else if let Some(h) = toon_m {
            let t = toon_mats.get(&h.0).map(|t| t.tint).unwrap_or(Vec4::ZERO);
            ("Toon", [t.x, t.y, t.z], false, "Opaque".to_string())
        } else if ink_m.is_some() {
            ("Ink", [0.0; 3], true, "Opaque".to_string())
        } else if water_m.is_some() {
            ("Water", [0.0; 3], false, "Opaque".to_string())
        } else {
            ("Other", [0.0; 3], false, String::new())
        };
        let visible = !matches!(vis, Some(Visibility::Hidden));
        let in_view = view_vis.is_some_and(|v| v.get());
        let inherited = inh_vis.is_some_and(|v| v.get());
        rows.push((
            extent,
            format!(
                "{{\"name\":{:?},\"ancestors\":{:?},\"root_has_villager\":{has_vill},\"extent\":{extent:.2},\"joint_spread\":{spread:.2},\"joint_reach\":{reach:.2},\"skinned\":{},\"skin_path\":\"{skin_path}\",\"mat\":{{\"kind\":\"{kind}\",\"base_srgb\":[{:.3},{:.3},{:.3}],\"unlit\":{unlit},\"alpha_mode\":{alpha:?}}},\"visible\":{visible},\"inherited_vis\":{inherited},\"view_vis\":{in_view}}}",
                chain[0],
                chain,
                skin.is_some(),
                rgb[0],
                rgb[1],
                rgb[2]
            ),
        ));
    }
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    let top: Vec<String> = rows.into_iter().take(10).map(|r| r.1).collect();
    info!(
        "[bigmesh] {{\"shot\":{},\"n\":{},\"top\":[{}]}}",
        *shots,
        top.len(),
        top.join(",")
    );
}
