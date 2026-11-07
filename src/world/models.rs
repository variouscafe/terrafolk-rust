//! 3D 소품 — Quaternius CC0 GLB 씬 스폰 + 자동 핏. 자연은 Stylized Nature
//! MEGAKIT(2026-10-18 교체), 사슴 계열은 Animated Animal Pack(스킨드·Idle
//! 애님 — animal_anim.rs). 나머지 건축·소품은 기존 poly/Kenney CC0·CC-BY.
//! 원천 스케일이 제각각(0.57m 오두막~25m 텐트)이므로 계층 AABB를 측정해
//! 목표 높이로 정규화한다(결정론 — 좌표 해시 회전·지터 포함).
//! 번들 GLB(나무 5종 나란히)는 메시명으로 변형 1개만 유지한다.

use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::world_serialization::WorldAssetRoot;
use std::collections::HashMap;

use super::data::{PropKind, WorldData};

/// 소품 → GLB 명세. variants: 번들 내 메시명(빈 배열=통째 사용).
pub struct ModelSpec {
    pub path: &'static str,
    pub target_h: f32,
    pub variants: &'static [&'static str],
    /// 절출 카드 프롭(assetgen meshgen 산출) — 정면(+Z)이 그림면이라
    /// 결정론 회전 대신 요 0 고정(카메라 요 0°와 정렬, 구 folk 라인
    /// 카드 나무 계약). 회전하면 빈 옆면이 보인다.
    pub card: bool,
}

impl PropKind {
    pub fn model(self) -> ModelSpec {
        match self {
            PropKind::ClayPot => ModelSpec {
                // meshgen(assetgen) 산출 — 스프라이트 절출 GLB, 1.4m 정규화
                path: "models/clay_pot.glb",
                target_h: 1.4,
                variants: &[],
                card: false,
            },
            PropKind::TreeBroadleaf => ModelSpec {
                path: "models/qnature/tree-0.glb",
                target_h: 7.2,
                variants: &[],
                card: false,
            },
            PropKind::TreePine => ModelSpec {
                path: "models/qnature/pine-1.glb",
                target_h: 8.0,
                variants: &[],
                card: false,
            },
            PropKind::TreePalm => ModelSpec {
                path: "models/PalmTrees.glb",
                target_h: 6.2,
                variants: &[
                    "PalmTree_1",
                    "PalmTree_2",
                    "PalmTree_3",
                    "PalmTree_4",
                    "PalmTree_5",
                ],
                card: false,
            },
            PropKind::TreeSapling => ModelSpec {
                path: "models/qnature/fern-0.glb",
                target_h: 1.4,
                variants: &[],
                card: false,
            },
            PropKind::TreeDead => ModelSpec {
                path: "models/qnature/dead-tree-0.glb",
                target_h: 6.4,
                variants: &[],
                card: false,
            },
            PropKind::BerryBush => ModelSpec {
                path: "models/qnature/bush-with-flowers-0.glb",
                target_h: 1.15,
                variants: &[],
                card: false,
            },
            PropKind::Boulder => ModelSpec {
                // 셀 카툰 카드(2026-11-07) — 구본 qnature/rock-medium-0
                path: "models/celprops/boulder.glb",
                target_h: 1.5,
                variants: &[],
                card: true,
            },
            PropKind::GoldVein => ModelSpec {
                // 셀 카툰 카드(2026-11-07) — 구본 qnature/rock-medium-2
                path: "models/celprops/gold_vein.glb",
                target_h: 1.05,
                variants: &[],
                card: true,
            },
            PropKind::FlowerClump => ModelSpec {
                path: "models/qnature/flower-group-0.glb",
                target_h: 0.6,
                variants: &[],
                card: false,
            },
            PropKind::GrassClump => ModelSpec {
                path: "models/qnature/grass-0.glb",
                target_h: 0.55,
                variants: &[],
                card: false,
            },
            PropKind::Hut => ModelSpec {
                // 셀 카툰 카드(2026-11-07 소품 카툰 통일 3단계) — Qwen Image 2.1
                // 셀 카툭 스프라이트(assetgen qwenprop·celprep 절출). 구본
                // CottageHouses 3변형은 git 히스토리 참조.
                path: "models/celprops/hut.glb",
                target_h: 4.2,
                variants: &[],
                card: true,
            },
            PropKind::StorageHut => ModelSpec {
                path: "models/Barn.glb",
                target_h: 3.4,
                variants: &[],
                card: false,
            },
            PropKind::Tent => ModelSpec {
                path: "models/Tent.glb",
                target_h: 2.5,
                variants: &[],
                card: false,
            },
            PropKind::Campfire => ModelSpec {
                // 셀 카툰 카드(2026-11-07) — 구본 Bonfire.glb. 불꽃·라이트는
                // 형제 엔티티(spawn_campfire_fire)라 카드와 무관하게 유지.
                path: "models/celprops/bonfire.glb",
                target_h: 1.15,
                variants: &[],
                card: true,
            },
            PropKind::Windbreak => ModelSpec {
                path: "models/Fence.glb",
                target_h: 1.05,
                variants: &[],
                card: false,
            },
            PropKind::Grave => ModelSpec {
                path: "models/Grave.glb",
                target_h: 1.3,
                variants: &[],
                card: false,
            },
            PropKind::Stash => ModelSpec {
                path: "models/CubeCrate.glb",
                target_h: 1.0,
                variants: &[],
                card: false,
            },
            PropKind::Relic => ModelSpec {
                // 임시 룩 — 자연 바위(RockLarge)와 구별되는 상자. 발명 GLB 보유
                // 시엔 DynProp로 대체된다(net/resources.rs §17.86).
                path: "models/CubeCrate.glb",
                target_h: 1.1,
                variants: &[],
                card: false,
            },
            PropKind::WoodLog => ModelSpec {
                path: "models/WoodLog.glb",
                target_h: 0.9,
                variants: &[],
                card: false,
            },
            // 동물 — Poly by Google CC-BY(stylized flat) · Kenta Imai(niwatori)
            PropKind::AnimalBoar => ModelSpec {
                path: "models/AnimalBoar.glb",
                target_h: 0.85,
                variants: &[],
                card: false,
            },
            PropKind::AnimalDeer => ModelSpec {
                path: "models/qanimal/deer.glb",
                target_h: 1.7,
                variants: &[],
                card: false,
            },
            PropKind::AnimalWaterDeer => ModelSpec {
                path: "models/qanimal/stag.glb",
                target_h: 1.1,
                variants: &[],
                card: false,
            },
            PropKind::AnimalSquirrel => ModelSpec {
                path: "models/AnimalSquirrel.glb",
                target_h: 0.38,
                variants: &[],
                card: false,
            },
            PropKind::AnimalPheasant => ModelSpec {
                path: "models/AnimalPheasant.glb",
                target_h: 0.55,
                variants: &[],
                card: false,
            },
        }
    }
}

/// 라이브 동기용 — 종별 씬 핸들(GLB Scene0).
pub fn prop_scene(
    kind: PropKind,
    assets: &AssetServer,
) -> Handle<bevy::world_serialization::WorldAsset> {
    let spec = kind.model();
    assets.load(GltfAssetLabel::Scene(0).from_asset(spec.path))
}

/// 라이브 동기용 — 종별 (목표높이, 변형목록)만 노출.
pub fn prop_model(kind: PropKind) -> (f32, &'static [&'static str]) {
    let spec = kind.model();
    (spec.target_h, spec.variants)
}

#[derive(Component)]
pub struct PropRoot {
    pub kind: PropKind,
    pub target_h: f32,
    pub ground_y: f32,
    pub rot_y: f32,
    pub scale_jitter: f32,
    pub variants: &'static [&'static str],
}

#[derive(Component)]
pub struct CampfireGlow;

/// 발광 불꽃 메시(블룸 소스 — PointLight만으론 화면에 번짐이 생기지 않음).
#[derive(Component)]
pub struct CampfireFlame;

pub(crate) fn pos_hash(x: usize, y: usize, salt: u64) -> u64 {
    let mut h = (x as u64).wrapping_mul(0x9E3779B97F4A7C15)
        ^ (y as u64).wrapping_mul(0xC2B2AE3D27D4EB4F)
        ^ salt.wrapping_mul(0x165667B19E3779F9);
    h ^= h >> 29;
    h = h.wrapping_mul(0xBF58476D1CE4E5B9);
    h ^= h >> 32;
    h
}

pub fn spawn_props(
    mut commands: Commands,
    world: Res<WorldData>,
    assets: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let mut scene_cache: HashMap<&'static str, _> = HashMap::new();
    for p in &world.props {
        spawn_one_prop(
            &mut commands,
            &world,
            &assets,
            &mut meshes,
            &mut mats,
            &mut scene_cache,
            p,
        );
    }
}

/// 소품 1개 스폰 — 씬 캐시 공유(동일 모델 인스턴싱 유도)·결정론 회전/지터.
#[allow(clippy::too_many_arguments)] // 씬 캐시·에셋 저장소 — 스폰 경로 공유(apply.rs)
pub fn spawn_one_prop(
    commands: &mut Commands,
    world: &WorldData,
    assets: &AssetServer,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    scene_cache: &mut HashMap<&'static str, Handle<bevy::world_serialization::WorldAsset>>,
    p: &super::data::PropSpec,
) {
    let spec = p.kind.model();
    let scene = scene_cache
        .entry(spec.path)
        .or_insert_with(|| assets.load(GltfAssetLabel::Scene(0).from_asset(spec.path)))
        .clone();
    let ground = world.height_at(p.x as i32, p.y as i32);
    let h1 = pos_hash(p.x, p.y, 1);
    let h2 = pos_hash(p.x, p.y, 2);
    // 카드 프롭은 요 0 고정(정면 +Z = 카메라 정면) — 회전하면 빈 옆면 노출.
    let rot_y = if spec.card {
        0.0
    } else {
        (h1 % 3600) as f32 / 3600.0 * std::f32::consts::TAU
    };
    let scale_jitter = 0.92 + (h2 % 1000) as f32 / 1000.0 * 0.16;
    commands.spawn((
        WorldAssetRoot(scene),
        PropRoot {
            kind: p.kind,
            target_h: spec.target_h,
            ground_y: ground,
            rot_y,
            scale_jitter,
            variants: spec.variants,
        },
        Transform::from_translation(Vec3::new(p.x as f32 + 0.5, ground, p.y as f32 + 0.5)),
    ));
    if p.kind == PropKind::Campfire {
        spawn_campfire_fire(
            commands,
            meshes,
            mats,
            p.x as f32 + 0.5,
            p.y as f32 + 0.5,
            ground,
        );
    }
}

/// 모닥불 연출 스폰 — 발광 불꽃(emissive ×5, 블룸 소스) + 포인트 라이트.
/// 데모(spawn_one_prop)·라이브(net::apply::sync_resources) 공통 경로.
/// 반환: (불꽃, 라이트) 엔티티 — 라이브는 여기에 동기화 태그를 붙인다.
/// 재질은 모닥불별 개별(위상 오프셋 플리커 — flicker_campfires 참조).
pub fn spawn_campfire_fire(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    x: f32,
    z: f32,
    ground: f32,
) -> (Entity, Entity) {
    let flame = commands
        .spawn((
            CampfireFlame,
            Mesh3d(meshes.add(Mesh::from(bevy::math::primitives::Sphere::new(0.20)))),
            MeshMaterial3d(mats.add(StandardMaterial {
                base_color: Color::BLACK,
                emissive: FLAME_COLOR * 5.0,
                ..default()
            })),
            Transform::from_translation(Vec3::new(x, ground + 0.55, z)),
        ))
        .id();
    let glow = commands
        .spawn((
            CampfireGlow,
            PointLight {
                color: Color::srgb(1.0, 0.66, 0.32),
                intensity: 120_000.0,
                range: 9.0,
                shadow_maps_enabled: false,
                ..default()
            },
            Transform::from_translation(Vec3::new(x, ground + 1.0, z)),
        ))
        .id();
    (flame, glow)
}

/// 메시 POSITION 속성에서 AABB(GPU 미추출 독립 — final_aabb는 추출 후에만 존재).
pub fn flicker_campfires(
    time: Res<Time>,
    mut lights: Query<(&Transform, &mut PointLight), With<CampfireGlow>>,
    mut flames: Query<(&Transform, &MeshMaterial3d<StandardMaterial>), With<CampfireFlame>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let t = time.elapsed_secs();
    for (tr, mut light) in &mut lights {
        light.intensity = 120_000.0 * flick(t, tr);
    }
    for (tr, mat) in &mut flames {
        if let Some(mut m) = mats.get_mut(&mat.0) {
            m.emissive = FLAME_COLOR * 5.0 * flick(t, tr);
        }
    }
}


/// 불꽃 발광색(선형 sRGB — 배수로 HDR>1 만들어 블룸 유발).
const FLAME_COLOR: LinearRgba = LinearRgba::new(1.0, 0.42, 0.10, 1.0);

/// 플리커 계수 — 모닥불 좌표로 위상 오프셋(전체 동시 점멸 방지).
fn flick(t: f32, tr: &Transform) -> f32 {
    let ph = tr.translation.x * 3.1 + tr.translation.z * 1.7;
    0.82 + 0.13 * (t * 9.0 + ph).sin()
        + 0.05 * (t * 23.0 + ph + 1.7).sin()
        + 0.04 * (t * 5.0 + ph * 2.0 + 4.1).cos()
}
