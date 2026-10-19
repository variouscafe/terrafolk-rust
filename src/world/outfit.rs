//! 의상 색 다양화 — Quaternius 변형 6종은 같은 계열 색이라 마을이 단조롭다.
//! Pants/Feet 메시의 평색 재질(baseColor만 있는 GLB 재질)을 주민별 색조로
//! 복제·회전(hue rotate)한다. 신체(Skin/Eye/Eyebrows) 재질은 건드리지
//! 않는다 — 대상은 메시 이름(Pants·Feet)으로 한정. 애니메이션·본과 무관한
//! 재질 교체라 VillagerAnims 완결 후 적용(1회 마커).

use bevy::pbr::MeshMaterial3d;
use bevy::prelude::*;
use bevy::asset::Assets;
use bevy::render::mesh::Mesh3d;

use super::villager::{Villager, VillagerAnims};

/// 적용 완료 마커(1회).
#[derive(Component)]
pub struct OutfitTinted;

/// 이름 해시 → 색조 회전량(도). 12단계 — 같은 주민은 항상 같은 색.
fn hue_shift(name: &str) -> f32 {
    let h = name.bytes().fold(0u64, |a, b| a.wrapping_mul(31) + b as u64);
    (h % 12) as f32 * 30.0
}

/// 주민별 Pants/Feet 재질 색조 회전 — 마을 의상 다양성(생명감).
#[allow(clippy::type_complexity)] // Bevy 시스템 파라미터
pub fn vary_outfits(
    mut commands: Commands,
    meshes_q: Query<(Entity, &Name), With<Mesh3d>>,
    mats: Query<&MeshMaterial3d<StandardMaterial>>,
    villagers: Query<(Entity, &Villager), (With<VillagerAnims>, Without<OutfitTinted>)>,
    children: Query<&Children>,
    mut assets: ResMut<Assets<StandardMaterial>>,
) {
    for (root, villager) in &villagers {
        let shift = hue_shift(&villager.name);
        if shift != 0.0 {
            for d in children.iter_descendants(root) {
                let Ok((entity, mesh_name)) = meshes_q.get(d) else {
                    continue;
                };
                let lower = mesh_name.to_ascii_lowercase();
                if !(lower.contains("pants") || lower.contains("feet")) {
                    continue;
                }
                let Ok(handle) = mats.get(entity) else {
                    continue;
                };
                let Some(mat) = assets.get(handle) else {
                    continue;
                };
                let mut new_mat = mat.clone();
                let hsla = Hsla::from(new_mat.base_color);
                new_mat.base_color = Hsla {
                    hue: (hsla.hue + shift) % 360.0,
                    ..hsla
                }
                .into();
                commands
                    .entity(entity)
                    .insert(MeshMaterial3d(assets.add(new_mat)));
            }
        }
        // 원색 유지(1/12) 또는 대상 메시 부재 모델도 마커 — 재시도 방지.
        commands.entity(root).insert(OutfitTinted);
    }
}

/// 발밑 블롭 그림자 적용 마커.
#[derive(Component)]
pub struct BlobShadow;

/// 블롭 그림자 — 그림자 맵 폐지(2026-10-19 아트 방향)의 대체 연출.
/// 언릿 반투명 원형 쿼드를 주민 루트 자식으로 — 이동을 따라가고 지면 y는
/// 루트가 이미 추적한다. 라이트와 무관해 밤에도 일정(타겟 스타일의
/// 부드러운 블롭 근사).
#[allow(clippy::type_complexity)] // Bevy 시스템 파라미터
pub fn spawn_blob_shadows(
    mut commands: Commands,
    q: Query<(Entity, &Villager), (With<VillagerAnims>, Without<BlobShadow>)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    for (root, _) in &q {
        let blob = commands
            .spawn((
                Name::new("blob-shadow"),
                Mesh3d(meshes.add(Circle::new(0.22).mesh().build())),
                MeshMaterial3d(mats.add(StandardMaterial {
                    base_color: Color::srgba(0.08, 0.07, 0.10, 0.28),
                    unlit: true,
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                })),
                Transform::from_rotation(Quat::from_rotation_x(
                    -std::f32::consts::FRAC_PI_2,
                )),
            ))
            .id();
        commands.entity(root).add_child(blob);
        commands.entity(root).insert(BlobShadow);
    }
}
