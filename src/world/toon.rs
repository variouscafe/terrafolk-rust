//! 캐릭터 툰화 — 주민 메시 재질을 PBR(StandardMaterial)에서 3밴드 툰
//! 셰이딩(shaders/toon.wgsl)으로 교체한다. 맵이 핸드페인트 텍스처 기반이라
//! 주민만 세미리얼 PBR이면 이질감이 크다(2026-10-22 사용자 요청 — "맵
//! 그래픽처럼 카툰 풍"). 텍스처·틴트는 원본 GLB 재질에서 그대로 물려받아
//! 디자인은 유지하고 조명 응답만 카툰으로 바꾼다.

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::pbr::Material;
use bevy::render::render_resource::AsBindGroup;
use bevy::render::render_resource::{
    Extent3d, TextureDimension, TextureFormat,
};
use bevy::shader::ShaderRef;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;

use super::villager::Villager;

/// 툰 재질 — 텍스처 + 원본 baseColorFactor.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ToonMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub base: Handle<Image>,
    /// rgb=baseColorFactor · a=alpha(전부 불투명 GLB지만 보존)
    #[uniform(2)]
    pub tint: Vec4,
}

impl Material for ToonMaterial {
    fn fragment_shader() -> ShaderRef {
        "shaders/toon.wgsl".into()
    }
}

/// 툰 교체 완료 마커(메시 단위) — 재스왑 방지.
#[derive(Component)]
pub struct ToonSwapped;

/// 텍스처 없는 단색 재질용 1×1 흰 텍스처(스왑당 1회 생성).
fn white_fallback(
    images: &mut Assets<Image>,
    cache: &mut Local<Option<Handle<Image>>>,
) -> Handle<Image> {
    cache
        .get_or_insert_with(|| {
            images.add(Image::new_fill(
                Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                &[255, 255, 255, 255],
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            ))
        })
        .clone()
}

/// 주민 자손 PBR 메시를 툰으로 교체 — 매 프레임 스윕(마커 스킵이라 상시
/// 비용은 미미). 착용(wear)·후속 부착 메시도 자동 편입된다.
/// ⚠️ 교체 대상은 불투명+라이트 기준 재질만: 언릿(선택 링)·알파(연기·글로우)
/// 재질은 PBR 의도(SemiTransparent)라 건드리지 않는다.
/// 외형 틴트(2026-10-23) — lookJson parts의 skin/hairColor/eye/옷색을 재질
/// 슬롯에 반영한다. GLB 스폰 재질은 이름이 없어 Gltf.named_materials의
/// base_color 메타데이터와 색 매칭(ε=0.01)해 재질명을 역추적한다.
#[allow(clippy::type_complexity)] // Bevy 시스템 파라미터 — 툰 교체·틴트 전 경로
pub fn toonify(
    mut commands: Commands,
    villagers: Query<(Entity, &super::villager::VillagerModel), With<Villager>>,
    children: Query<&Children>,
    gltfs: Res<Assets<bevy::gltf::Gltf>>,
    gltf_mats: Res<Assets<bevy::gltf::GltfMaterial>>,
    std_mats: Res<Assets<bevy::pbr::StandardMaterial>>,
    mut toon_mats: ResMut<Assets<ToonMaterial>>,
    mut images: ResMut<Assets<Image>>,
    q_mat: Query<
        (&MeshMaterial3d<bevy::pbr::StandardMaterial>, Option<&ToonSwapped>),
        Without<super::outline::OutlineHull>,
    >,
    mut white: Local<Option<Handle<Image>>>,
    // (원본 재질, look 지문) → 툰 핸들 캐시 — look별 틴트가 다르므로 키에 포함
    mut cache: Local<
        HashMap<
            (bevy::asset::AssetId<bevy::pbr::StandardMaterial>, u64),
            Handle<ToonMaterial>,
        >,
    >,
    // GLB → (재질 linear색, 이름) 후보표 — 첫 주민 스캔 시 1회 구축
    mut meta: Local<HashMap<bevy::asset::AssetId<bevy::gltf::Gltf>, Vec<([f32; 4], String)>>>,
) {
    if villagers.is_empty() {
        return;
    }
    for (root, model) in &villagers {
        let look = model.1.clone().unwrap_or_default();
        let tints = if look.is_empty() {
            None
        } else {
            Some(super::look_tint::parse(&look))
        };
        let model_stem = model
            .0
            .path()
            .and_then(|p| {
                p.path()
                    .file_stem()
                    .and_then(|s| s.to_str().map(|s| s.to_string()))
            })
            .unwrap_or_default();
        // GLB 메타 — named_materials의 base_color(linear)·이름 쌍
        let candidates = meta.entry(model.0.id()).or_insert_with(|| {
            gltfs
                .get(&model.0)
                .map(|g| {
                    g.named_materials
                        .iter()
                        .filter_map(|(name, h)| {
                            gltf_mats.get(h).map(|gm| {
                                let c = gm.base_color.to_linear();
                                (
                                    [c.red, c.green, c.blue, c.alpha],
                                    name.to_string(),
                                )
                            })
                        })
                        .collect()
                })
                .unwrap_or_default()
        });
        // look 지문 — 캐시 키(틴트 조합별 재질 분리)
        let look_hash = look.bytes().fold(0u64, |a, b| a.wrapping_mul(31) + b as u64);
        // 미스왑(아직 PBR) 메시 잔여 — 0이 되는 프레임에 루트를 표시한다
        // (스폰 직후 PBR 프레임이 화면에 끼는 것을 막는다).
        let mut pending = 0usize;
        for d in children.iter_descendants(root) {
            let Ok((mat, swapped)) = q_mat.get(d) else {
                continue;
            };
            if swapped.is_some() {
                continue;
            }
            let Some(src) = std_mats.get(&mat.0) else {
                continue;
            };
            pending += 1;
            // 언릿·비불투명 PBR 재질은 툰 스킵 — 링·연기 등 연출 의도 보존.
            // Visible 동봉: wear 부착물(스폰 시 Hidden)도 스왑 확정 프레임에 표시.
            if src.unlit || src.alpha_mode != AlphaMode::Opaque {
                commands
                    .entity(d)
                    .insert((ToonSwapped, Visibility::Visible));
                continue;
            }
            // 틴트 — 재질명 역추적(색 매칭) → 슬롯 → 팔레트색. 실패 시 원본색.
            let src_v4 = {
                let c = src.base_color.to_srgba();
                Vec4::new(c.red, c.green, c.blue, c.alpha)
            };
            let tint = 'tint: {
                let Some(t) = tints.as_ref() else {
                    break 'tint src_v4;
                };
                let sc = src.base_color.to_linear();
                let Some((_, name)) = candidates.iter().find(|(c, _)| {
                    (c[0] - sc.red).abs() < 0.01
                        && (c[1] - sc.green).abs() < 0.01
                        && (c[2] - sc.blue).abs() < 0.01
                }) else {
                    break 'tint src_v4;
                };
                match super::look_tint::slot_for(&model_stem, name)
                    .and_then(|(slot, gain)| super::look_tint::tint_color(t, slot, gain))
                {
                    Some(col) => {
                        let c = col.to_srgba();
                        Vec4::new(c.red, c.green, c.blue, 1.0)
                    }
                    None => src_v4,
                }
            };
            let tinted = tint != src_v4;
            let key = (mat.0.id(), if tinted { look_hash } else { 0 });
            let toon = cache
                .entry(key)
                .or_insert_with(|| {
                    toon_mats.add(ToonMaterial {
                        base: src
                            .base_color_texture
                            .clone()
                            .unwrap_or_else(|| white_fallback(&mut images, &mut white)),
                        tint,
                    })
                })
                .clone();
            commands
                .entity(d)
                .remove::<MeshMaterial3d<bevy::pbr::StandardMaterial>>()
                .insert((MeshMaterial3d(toon), ToonSwapped, Visibility::Visible));
        }
        // 표시 게이트 — 미스왑 메시가 없는 프레임(스왑 확정)에 루트 표시.
        // 씬 로드 전(메시 0개)도 통과하지만 그릴 게 없어 PBR 노출이 없다.
        if pending == 0 {
            commands.entity(root).insert(Visibility::Visible);
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_plugins(bevy::pbr::MaterialPlugin::<ToonMaterial>::default());
    app.init_asset::<ToonMaterial>();
    app.add_systems(Update, toonify);
}
