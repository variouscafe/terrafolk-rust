//! 의상 색 다양화 — 레퍼런스 팔레트 계열(테라코타·슬레이트블루·세이지·
//! 머스터드·플럼·웜브라운)로 주민별 스냅. Pants/Feet 메시의 평색 재질을
//! 복제·재색한다 — Body/Legs 메시는 Skin 재질이 섞여 있어 제외(2026-10-19
//! GLB 실측). 신체(Skin/Eye/Eyebrows) 재질은 건드리지 않는다. 애니메이션·
//! 본과 무관한 재질 교체라 VillagerAnims 완결 후 적용(1회 마커).

use bevy::pbr::MeshMaterial3d;
use bevy::prelude::*;
use bevy::asset::Assets;
use bevy::render::mesh::Mesh3d;

use super::villager::{Villager, VillagerAnims};

/// 적용 완료 마커(1회).
#[derive(Component)]
pub struct OutfitTinted;

/// 레퍼런스 의상 팔레트 계열 — 테라코타·슬레이트블루·세이지·머스터드·
/// 플럼·웜브라운(원색 근사). 주민 해시로 1택(같은 주민은 항상 같은 계열).
/// 원본 명도는 보존해 옷감 음영은 살리고 색조만 계열로 스냅.
const OUTFIT_FAMILIES: [Hsla; 6] = [
    Hsla { hue: 18.0, saturation: 0.55, lightness: 0.5, alpha: 1.0 }, // 테라코타
    Hsla { hue: 210.0, saturation: 0.35, lightness: 0.5, alpha: 1.0 }, // 슬레이트블루
    Hsla { hue: 90.0, saturation: 0.30, lightness: 0.5, alpha: 1.0 }, // 세이지
    Hsla { hue: 45.0, saturation: 0.60, lightness: 0.5, alpha: 1.0 }, // 머스터드
    Hsla { hue: 330.0, saturation: 0.30, lightness: 0.5, alpha: 1.0 }, // 플럼
    Hsla { hue: 25.0, saturation: 0.40, lightness: 0.4, alpha: 1.0 }, // 웜브라운
];

fn family_of(name: &str) -> Hsla {
    let h = name.bytes().fold(0u64, |a, b| a.wrapping_mul(31) + b as u64);
    OUTFIT_FAMILIES[(h % OUTFIT_FAMILIES.len() as u64) as usize]
}

/// 재색: 계열 색조로 스냅(명도 보존), 채도는 원본과 계열의 중간.
fn snap_family(orig: Hsla, fam: Hsla) -> Hsla {
    Hsla {
        hue: fam.hue,
        saturation: orig.saturation * 0.5 + fam.saturation * 0.5,
        lightness: orig.lightness,
        alpha: 1.0,
    }
}

/// 주민별 Pants/Feet 재질 팔레트 스냅 — 마을 의상 다양성(생명감).
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
        let fam = family_of(&villager.name);
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
            new_mat.base_color = snap_family(Hsla::from(new_mat.base_color), fam).into();
            commands
                .entity(entity)
                .insert(MeshMaterial3d(assets.add(new_mat)));
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
                    // 레퍼런스 그림자 — 검정 아닌 웜 올리브(#5F7A3C 계열)
                    base_color: Color::srgba(0.26, 0.32, 0.16, 0.32),
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
