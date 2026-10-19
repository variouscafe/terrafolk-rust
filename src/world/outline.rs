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

/// 메시 법선 방향 부풀리기(미터, 메시 공간 — 레퍼런스는 가는 라인이라 얇게).
/// 2026-10-19 실측 교정: 0.010은 헤어·눈썹 카드 등 얇은 지오메트리에서
/// 파편처럼 튀어나와 보였다 — 0.004로 축소.
const INFLATE: f32 = 0.004;

/// 이 최대 치수(미터, 메시 공간)보다 작은 메시는 헐 생략 — 눈·눈썹·헤어
/// 카드 등 얇은 소형 지오메트리의 윤곽은 오히려 지저분해진다(실측).
const MIN_HULL_SIZE: f32 = 0.25;

/// 부풀린 메시 생성 — POSITION을 NORMAL 방향으로 밀고 나머지 속성(스킨
/// weight·UV·인덱스)은 그대로 복제. 토폴로지·법선·크기 부적합은 None.
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
    // 소형 메시 스킵 — AABB 최대 변으로 판정.
    let (mn, mx) = (
        pos.iter().fold([f32::MAX; 3], |a, p| {
            [a[0].min(p[0]), a[1].min(p[1]), a[2].min(p[2])]
        }),
        pos.iter().fold([f32::MIN; 3], |a, p| {
            [a[0].max(p[0]), a[1].max(p[1]), a[2].max(p[2])]
        }),
    );
    let extent = (mx[0] - mn[0]).max(mx[1] - mn[1]).max(mx[2] - mn[2]);
    if extent < MIN_HULL_SIZE {
        return None;
    }
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
    mut cache: Local<HashMap<bevy::asset::AssetId<Mesh>, Option<Handle<Mesh>>>>,
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
            let inflated = match cache.entry(mesh.id()).or_insert_with(|| {
                meshes
                    .get(mesh)
                    .and_then(inflate_mesh)
                    .map(|m| meshes.add(m))
            }) {
                Some(h) => h.clone(),
                // 부적합 메시(소형·법선 없음)는 헐 없음 — 얇은 카드의 뒷면
                // 렌더가 파편처럼 보이는 폴백은 제거(2026-10-19 실측).
                None => continue,
            };
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
