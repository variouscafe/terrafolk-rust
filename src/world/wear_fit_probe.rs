//! 착용품 위치 진단 — `?tfdbg=wearfit` (꺼짐=비용 0). 주민마다 한 번 `[wearfit] {json}` 한 줄.
//! 좌표는 "액터 프레임"(원점=액터 GT 위치, 회전=액터 회전, 스케일 제거 — 월드 단위, y 위·z 정면).
//! 항목: Head 본·앵커 위치/월드 스케일/축, qchar Head 메시 AABB(정수리·턱·눈높이·정면 z·폭),
//! 각 wear 메시의 AABB(스킨이면 조인트 GT·IBM 기준)와 "Head 20% 확장 박스 안" 판정.

use std::collections::HashMap;

use bevy::gltf::{GltfMaterialName, GltfMeshName};
use bevy::mesh::skinning::{SkinnedMesh, SkinnedMeshInverseBindposes};
use bevy::mesh::{Mesh, VertexAttributeValues};
use bevy::prelude::*;

use super::prop_fit::mesh_aabb;
use super::villager::Villager;
use crate::character::ink::InkMaterial;
use crate::character::tool::ToolAnchor;

type Aabb = (Vec3, Vec3);

/// 메시 월드 AABB(월드 좌표) — 스킨이면 Σ w·(조인트GT·IBM)·p, 아니면 로컬 AABB 8모서리.
fn world_aabb(
    mesh: &Mesh,
    gt: &GlobalTransform,
    skin: Option<(&SkinnedMesh, &SkinnedMeshInverseBindposes)>,
    gts: &Query<&GlobalTransform>,
) -> Option<Aabb> {
    let (mut lo, mut hi) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
    if let Some((sk, ib)) = skin
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
    } else {
        let (mn, mx) = mesh_aabb(mesh)?;
        let m = gt.to_matrix();
        for x in [mn.x, mx.x] {
            for y in [mn.y, mx.y] {
                for z in [mn.z, mx.z] {
                    let v = m.transform_point3(Vec3::new(x, y, z));
                    lo = lo.min(v);
                    hi = hi.max(v);
                }
            }
        }
    }
    (lo.x <= hi.x).then_some((lo, hi))
}

fn v3(v: Vec3) -> String {
    format!("[{:.3},{:.3},{:.3}]", v.x, v.y, v.z)
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
pub fn wear_fit_probe(
    mut frame: Local<u32>,
    mut ready: Local<HashMap<Entity, u32>>,
    mut done: Local<Vec<Entity>>,
    meshes: Res<Assets<Mesh>>,
    ibms: Res<Assets<SkinnedMeshInverseBindposes>>,
    villagers: Query<(Entity, &GlobalTransform), (With<Villager>, With<super::wear::WearApplied>)>,
    wears: Query<&super::wear::Wears>,
    children: Query<&Children>,
    names: Query<&Name>,
    mesh_q: Query<(
        &Mesh3d,
        &GlobalTransform,
        Option<&SkinnedMesh>,
        Option<&GltfMeshName>,
        Option<&GltfMaterialName>,
        Has<MeshMaterial3d<InkMaterial>>,
    )>,
    parents: Query<&ChildOf>,
    anchors: Query<&ToolAnchor>,
    gts: Query<&GlobalTransform>,
    scales: Query<&crate::character::tool::ToolScale>,
) {
    if !crate::net::dbg_gate::on("wearfit") {
        return;
    }
    *frame += 1;
    if !(*frame).is_multiple_of(30) {
        return;
    }
    for (actor, agt) in &villagers {
        if done.contains(&actor) {
            continue;
        }
        // 첫 관측 후 300프레임(≈5초) 대기 — wear GLB 비동기 로드·툰 스왑·앵커 refit 안정화.
        let n = ready.entry(actor).or_insert(0);
        *n += 30;
        if *n < 300 {
            continue;
        }
        let (_, rot, org) = agt.to_scale_rotation_translation();
        let inv = rot.inverse();
        let to_actor = |p: Vec3| inv * (p - org);
        let aabb_actor = |(lo, hi): Aabb| {
            let (mut a, mut b) = (Vec3::splat(f32::MAX), Vec3::splat(f32::MIN));
            for x in [lo.x, hi.x] {
                for y in [lo.y, hi.y] {
                    for z in [lo.z, hi.z] {
                        let p = to_actor(Vec3::new(x, y, z));
                        a = a.min(p);
                        b = b.max(p);
                    }
                }
            }
            (a, b)
        };
        // 본체 Head 메시 + wear 메시 수집
        let mut head: Option<Aabb> = None;
        let mut wear_rows: Vec<(String, Aabb, Aabb)> = Vec::new();
        let mut moustache = false;
        let mut bone_pos = None;
        let mut anchor_rows: Vec<String> = Vec::new();
        for e in children.iter_descendants(actor) {
            let nm = names.get(e).map(|n| n.as_str()).unwrap_or("");
            if nm == "Head" && bone_pos.is_none() {
                bone_pos = gts.get(e).ok().map(|g| to_actor(g.translation()));
            }
            if anchors.get(e).is_ok()
                && let Ok(g) = gts.get(e)
            {
                let (s, r, t) = g.to_scale_rotation_translation();
                let rr = inv * r;
                anchor_rows.push(format!(
                    "{{\"name\":{nm:?},\"pos\":{},\"world_scale\":{},\"x\":{},\"y\":{},\"z\":{}}}",
                    v3(to_actor(t)),
                    v3(s),
                    v3(rr * Vec3::X),
                    v3(rr * Vec3::Y),
                    v3(rr * Vec3::Z)
                ));
            }
            let Ok((mh, gt, skin, mname, matname, is_ink)) = mesh_q.get(e) else {
                continue;
            };
            if is_ink {
                continue;
            }
            if let Some(GltfMaterialName(m)) = matname
                && m.to_lowercase().contains("moustache")
            {
                moustache = true;
            }
            let Some(mesh) = meshes.get(&mh.0) else {
                continue;
            };
            let sk = skin.and_then(|s| ibms.get(&s.inverse_bindposes).map(|i| (s, i)));
            let Some(w) = world_aabb(mesh, gt, sk, &gts) else {
                continue;
            };
            // wear-* 하위 여부(조상 체인 이름)
            let mut wear_name = None;
            let mut cur = e;
            while let Ok(p) = parents.get(cur) {
                cur = p.parent();
                if let Ok(n) = names.get(cur)
                    && n.starts_with("wear-")
                    && !n.starts_with("wear-anchor")
                {
                    wear_name = Some(n.as_str().to_string());
                    break;
                }
            }
            if let Some(wn) = wear_name {
                let local = mesh_aabb(mesh).unwrap_or((Vec3::ZERO, Vec3::ZERO));
                wear_rows.push((wn, aabb_actor(w), local));
            } else if mname.is_some_and(|m| m.0.ends_with("Head")) || nm.ends_with("Head") {
                let a = aabb_actor(w);
                head = Some(match head {
                    Some((l, h)) => (l.min(a.0), h.max(a.1)),
                    None => a,
                });
            }
        }
        let Some((hlo, hhi)) = head else { continue };
        done.push(actor);
        let size = hhi - hlo;
        let (elo, ehi) = (hlo - size * 0.2, hhi + size * 0.2);
        let eye_y = hlo.y + size.y * 0.45;
        let ts = scales.get(actor).map_or(1.0, |t| t.0);
        let wear_count = wears.get(actor).map_or(0, |w| w.0.len());
        let mut rows = Vec::new();
        let mut violations = 0;
        for (wn, (lo, hi), (llo, lhi)) in &wear_rows {
            let c = (*lo + *hi) * 0.5;
            let inside = c.cmpge(elo).all() && c.cmple(ehi).all();
            if !inside {
                violations += 1;
            }
            rows.push(format!(
                "{{\"wear\":{wn:?},\"min\":{},\"max\":{},\"center\":{},\"local_min\":{},\"local_max\":{},\"inside\":{inside}}}",
                v3(*lo), v3(*hi), v3(c), v3(*llo), v3(*lhi)
            ));
        }
        let hair_wears = wear_rows
            .iter()
            .filter(|(n, _, _)| n.starts_with("wear-hair") || n.starts_with("wear-brows"))
            .count();
        info!(
            "[wearfit] {{\"actor\":{:?},\"tool_scale\":{ts:.3},\"wears\":{wear_count},\"head_bone\":{},\"anchors\":[{}],\"head\":{{\"min\":{},\"max\":{},\"top\":{:.3},\"chin\":{:.3},\"eye_y\":{eye_y:.3},\"face_z\":{:.3},\"width\":{:.3}}},\"builtin_moustache\":{moustache},\"hair_brow_wear_meshes\":{hair_wears},\"violations\":{violations},\"rows\":[{}]}}",
            names.get(actor).map(|n| n.as_str()).unwrap_or("?"),
            bone_pos.map_or("null".into(), v3),
            anchor_rows.join(","),
            v3(hlo),
            v3(hhi),
            hhi.y,
            hlo.y,
            hhi.z,
            size.x,
            rows.join(",")
        );
    }
}
