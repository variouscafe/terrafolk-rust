//! 카툰 아웃라인 — 역방향 헐(inverted hull). 주민 메시를 법선 방향으로
//! 부풀린 복제 메시를 뒷면만 렌더(전면 컬링)해 윤곽을 깎는 2.5D 카툰 기법.
//! 언릿 단색이라 조명 비용 없고, 부풀린 메시는 원본 핸들별 캐시(주민 공유).
//! 스킨 애니메이션은 SkinnedMesh 컴포넌트를 원본 엔티티에서 복제해 추종.

use bevy::asset::RenderAssetUsages;
use bevy::pbr::MeshMaterial3d;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::mesh::{Mesh, Mesh3d, VertexAttributeValues};
use bevy::render::render_resource::PrimitiveTopology;

use super::villager::{Villager, VillagerAnims};

/// 윤곽 헐 마커(자기 복제 방지).
#[derive(Component)]
pub struct OutlineHull;

/// 주민별 적용 완료 마커 — 없으면 매 프레임 헐이 무한增生한다(2026-10-19
/// 실측: 메시 2천→2.5만 폭증으로 SwiftShader DeviceLost 유발).
#[derive(Component)]
pub struct OutlineDone;

/// 메시 법선 방향 부풀리기(미터, 메시 공간 — 루트 스케일 ~0.85 반영해 실제 ~9mm).
const INFLATE: f32 = 0.010;

/// 부풀린 메시 생성 — POSITION을 NORMAL 방향으로 밀고 나머지 속성(스킨
/// weight·UV·인덱스)은 그대로 복제. 토폴로지가 씬 메시(삼각형 리스트)가
/// 아니면 None(윤곽 생략).
fn inflate_mesh(src: &Mesh) -> Option<Mesh> {
    if src.primitive_topology() != PrimitiveTopology::TriangleList {
        return None;
    }
    let pos = match src.attribute(Mesh::ATTRIBUTE_POSITION)? {
        VertexAttributeValues::Float32x3(v) => v.clone(),
        _ => return None,
    };
    let nrm = match src.attribute(Mesh::ATTRIBUTE_NORMAL)? {
        VertexAttributeValues::Float32x3(v) => v.clone(),
        _ => return None,
    };
    let mut out = Mesh::new(src.primitive_topology(), RenderAssetUsages::default());
    for (id, data) in src.attributes() {
        if *id == Mesh::ATTRIBUTE_POSITION {
            let moved: Vec<[f32; 3]> = pos
                .iter()
                .zip(nrm.iter())
                .map(|(p, n)| [p[0] + n[0] * INFLATE, p[1] + n[1] * INFLATE, p[2] + n[2] * INFLATE])
                .collect();
            out.insert_attribute(*id, moved);
        } else {
            // 다른 속성은 원본 버퍼 그대로 재사용(스킨·UV 보존).
            out.insert_attribute(*id, data.clone());
        }
    }
    if let Some(idx) = src.indices() {
        out.insert_indices(idx.clone());
    }
    Some(out)
}

/// 주민 자손 메시에 윤곽 헐 부착 — VillagerAnims 완결 후 1회.
#[allow(clippy::type_complexity)] // Bevy 시스템 파라미터
pub fn attach_outlines(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<bevy::pbr::StandardMaterial>>,
    villagers: Query<Entity, (With<Villager>, With<VillagerAnims>, Without<OutlineDone>)>,
    children: Query<&Children>,
    meshes_q: Query<(&Mesh3d, Option<&bevy::mesh::skinning::SkinnedMesh>), Without<OutlineHull>>,
    mut cache: Local<HashMap<bevy::asset::AssetId<Mesh>, Handle<Mesh>>>,
) {
    if villagers.is_empty() {
        return;
    }
    let mut hull_mat: Option<Handle<bevy::pbr::StandardMaterial>> = None;
    for root in &villagers {
        for d in children.iter_descendants(root) {
            let Ok((mesh, skin)) = meshes_q.get(d) else {
                continue;
            };
            let inflated = cache
                .entry(mesh.id())
                .or_insert_with(|| match meshes.get(mesh).and_then(inflate_mesh) {
                    Some(m) => meshes.add(m),
                    // 폴백: 부풀리기 실패 시 원형(뒷면은 앞면에 가려 무해)
                    None => mesh.0.clone(),
                })
                .clone();
            let mat = hull_mat.get_or_insert_with(|| {
                mats.add(bevy::pbr::StandardMaterial {
                    // 레퍼런스 윤곽색 — 검정 아닌 웜 다크브라운(#4A3520)
                    base_color: Color::srgb(0.29, 0.21, 0.13),
                    unlit: true,
                    // 전면 컬링 — 뒷면만 그려 헐 뒤 윤곽이 보이는 원리
                    cull_mode: Some(bevy::render::render_resource::Face::Front),
                    ..default()
                })
            });
            // 스킨 메시는 SkinnedMesh 복제로 본 추종 — 아니면 정적 헐.
            let hull = if let Some(skin) = skin {
                commands
                    .spawn((
                        OutlineHull,
                        Mesh3d(inflated),
                        MeshMaterial3d(mat.clone()),
                        skin.clone(),
                    ))
                    .id()
            } else {
                commands
                    .spawn((OutlineHull, Mesh3d(inflated), MeshMaterial3d(mat.clone())))
                    .id()
            };
            commands.entity(d).add_child(hull);
        }
        // 루트에 완료 마커 — 다음 프레임부터 재부착 없음(자손 헐은
        // Without<OutlineHull>로 걸러지지만 원본 메시는 걸러지지 않는다).
        commands.entity(root).insert(OutlineDone);
    }
}
