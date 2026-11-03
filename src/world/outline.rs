//! 카툰 잉크 아웃라인 — 화면공간 일정 두께 헐(inverted hull). 주민 메시를
//! 법선 방향으로 부풀린 복제를 뒷면만 렌더(전면 컬링)해 윤곽을 깎는 카툰
//! 기법. 팽창은 버텍스 셰이더(shaders/outline_ink.wgsl)에서 뷰 깊이에
//! 비례 — 줌·거리와 무관하게 일정 픽셀 두께(구판은 CPU 상수 부풀림이라
//! 줌 구간마다 굵기가 20배 변해 기본 줌에서 소멸했다, 2026-11-03 실측).
//! 원본 메시 핸들을 그대로 재사용(부풀림 메시 복제 없음)하고 스킨 애니메이션은
//! SkinnedMesh 컴포넌트를 원본 엔티티에서 복제해 추종.

use bevy::asset::Asset;
use bevy::light::NotShadowCaster;
use bevy::pbr::{Material, MaterialPipeline, MaterialPipelineKey, MeshMaterial3d};
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::mesh::{Mesh, Mesh3d, MeshVertexBufferLayoutRef, VertexAttributeValues};
use bevy::render::render_resource::{
    AsBindGroup, Face, RenderPipelineDescriptor, SpecializedMeshPipelineError,
};
use bevy::shader::ShaderRef;

use super::villager::{Villager, VillagerAnims};

/// 윤곽 헐 마커(자기 복제 방지).
#[derive(Component)]
pub struct OutlineHull;

/// 주민별 적용 완료 마커 — 없으면 매 프레임 헐이 무한增殖한다(2026-10-19
/// 실측: 메시 2천→2.5만 폭증으로 SwiftShader DeviceLost 유발).
#[derive(Component)]
pub struct OutlineDone;

/// 목표 라인 두께 — 800px 뷰포트 기준 픽셀(비율이라 해상도 비례 확대).
/// 4px: 히어로캠에서 셀 애니 잉크 감각, 기본 줌(주민 25~70px)에서도
/// 명확히 읽히는 굵기(2026-11-03 사용자 "안 보여" 보고 후 결정).
/// 게이트 `?tfdbg=ink{px}` — 재빌드 없이 두께 A/B(예: ink3, ink6,
/// ink0=아웃라인 끔).
const INK_PX_DEFAULT: f32 = 4.0;

/// 이 세션의 라인 두께(px) — 게이트 우선. ink0/파싱 실패 0은 끔.
fn ink_px_now() -> Option<f32> {
    match crate::net::dbg_gate::value("ink").and_then(|v| v.parse::<f32>().ok()) {
        Some(px) if px > 0.0 => Some(px),
        Some(_) => None,
        None => Some(INK_PX_DEFAULT),
    }
}

/// 이 최대 치수(미터, 메시 공간)보다 작은 메시는 헐 생략 — 눈·눈썹·헤어
/// 카드 등 얇은 소형 지오메트리는 원거리 상한 팽창(120mm)에 튀어나가
/// 파편처럼 보인다(2026-10-19 실측, 화면공간 개편 후에도 유효).
const MIN_HULL_SIZE: f32 = 0.25;

/// 잉크 헐 재질 — params: rgb=잉크색(#291F17) · a=두께(화면 세로 비율).
/// 단일 f32 유니폼은 wasm 정합 붕괴(0.9.12 사고) — vec4 슬롯 고정.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct OutlineMaterial {
    #[uniform(0)]
    pub params: Vec4,
}

impl Material for OutlineMaterial {
    fn vertex_shader() -> ShaderRef {
        "shaders/outline_ink.wgsl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "shaders/outline_ink.wgsl".into()
    }
    /// 전면 컬링 — 뒷면만 그려 헐 뒤 윤곽이 보이는 원리(구판 cull_mode).
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut RenderPipelineDescriptor,
        _layout: &MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        descriptor.primitive.cull_mode = Some(Face::Front);
        Ok(())
    }
}

/// 메시 헐 적합성 — AABB 최대 변이 MIN_HULL_SIZE 이상인가(법선 없음도 부적합).
fn hull_suitable(mesh: Option<&Mesh>) -> bool {
    let Some(src) = mesh else {
        return false;
    };
    let Some(VertexAttributeValues::Float32x3(pos)) = src.attribute(Mesh::ATTRIBUTE_POSITION)
    else {
        return false;
    };
    let mut mn = [f32::MAX; 3];
    let mut mx = [f32::MIN; 3];
    for p in pos {
        for i in 0..3 {
            mn[i] = mn[i].min(p[i]);
            mx[i] = mx[i].max(p[i]);
        }
    }
    (mx[0] - mn[0]).max(mx[1] - mn[1]).max(mx[2] - mn[2]) >= MIN_HULL_SIZE
}

/// 주민 자손 메시에 잉크 헐 부착 — VillagerAnims 완결 후 1회. 헐은 원본
/// 메시 핸들 공유 + OutlineMaterial(팽창은 셰이더).
#[allow(clippy::type_complexity)] // Bevy 시스템 파라미터
pub fn attach_outlines(
    mut commands: Commands,
    mut mats: ResMut<Assets<OutlineMaterial>>,
    villagers: Query<Entity, (With<Villager>, With<VillagerAnims>, Without<OutlineDone>)>,
    children: Query<&Children>,
    meshes_q: Query<(&Mesh3d, Option<&bevy::mesh::skinning::SkinnedMesh>), Without<OutlineHull>>,
    meshes: Res<Assets<Mesh>>,
    // 메시 적합성 캐시(핸들별 1회 스캔).
    mut cache: Local<HashMap<bevy::asset::AssetId<Mesh>, bool>>,
) {
    if villagers.is_empty() {
        return;
    }
    // 세션 라인 두께(게이트 ink{px} 우선) — ink0이면 아웃라인 없음.
    let Some(px) = ink_px_now() else {
        return;
    };
    let mat = mats.add(OutlineMaterial {
        params: Vec4::new(0.16, 0.12, 0.09, px / 800.0),
    });
    for root in &villagers {
        for d in children.iter_descendants(root) {
            let Ok((mesh, skin)) = meshes_q.get(d) else {
                continue;
            };
            let suitable = *cache
                .entry(mesh.id())
                .or_insert_with(|| hull_suitable(meshes.get(&mesh.0)));
            if !suitable {
                continue;
            }
            // 스킨 메시는 SkinnedMesh 복제로 본 추종 — 아니면 정적 헐.
            // NotShadowCaster: 팽창 헐이 지면 그림자를 더 어둡게 못 박게.
            let hull = if let Some(skin) = skin {
                commands
                    .spawn((
                        OutlineHull,
                        Mesh3d(mesh.0.clone()),
                        MeshMaterial3d(mat.clone()),
                        skin.clone(),
                        NotShadowCaster,
                    ))
                    .id()
            } else {
                commands
                    .spawn((
                        OutlineHull,
                        Mesh3d(mesh.0.clone()),
                        MeshMaterial3d(mat.clone()),
                        NotShadowCaster,
                    ))
                    .id()
            };
            commands.entity(d).add_child(hull);
        }
        // 루트에 완료 마커 — 다음 프레임부터 재부착 없음.
        commands.entity(root).insert(OutlineDone);
    }
}

pub fn plugin(app: &mut App) {
    app.add_plugins(bevy::pbr::MaterialPlugin::<OutlineMaterial>::default());
    app.init_asset::<OutlineMaterial>();
    app.add_systems(Update, attach_outlines);
}
