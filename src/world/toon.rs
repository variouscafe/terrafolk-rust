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
#[allow(clippy::type_complexity)]
pub fn toonify(
    mut commands: Commands,
    villagers: Query<Entity, With<Villager>>,
    children: Query<&Children>,
    mut std_mats: ResMut<Assets<bevy::pbr::StandardMaterial>>,
    mut toon_mats: ResMut<Assets<ToonMaterial>>,
    mut images: ResMut<Assets<Image>>,
    mut q_mat: Query<
        (&MeshMaterial3d<bevy::pbr::StandardMaterial>, Option<&ToonSwapped>),
        Without<super::outline::OutlineHull>,
    >,
    mut white: Local<Option<Handle<Image>>>,
    // 원본 재질 → 툰 핸들 캐시(같은 GLB 재질을 주민이 공유)
    mut cache: Local<HashMap<bevy::asset::AssetId<bevy::pbr::StandardMaterial>, Handle<ToonMaterial>>>,
) {
    if villagers.is_empty() {
        return;
    }
    for root in &villagers {
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
            // 언릿·비불투명 PBR 재질은 툰 스킵 — 링·연기 등 연출 의도 보존
            if src.unlit || src.alpha_mode != AlphaMode::Opaque {
                commands.entity(d).insert(ToonSwapped);
                continue;
            }
            let toon = cache
                .entry(mat.0.id())
                .or_insert_with(|| {
                    toon_mats.add(ToonMaterial {
                        base: src
                            .base_color_texture
                            .clone()
                            .unwrap_or_else(|| white_fallback(&mut images, &mut white)),
                        tint: {
                            let c = src.base_color.to_srgba();
                            Vec4::new(c.red, c.green, c.blue, c.alpha)
                        },
                    })
                })
                .clone();
            commands
                .entity(d)
                .remove::<MeshMaterial3d<bevy::pbr::StandardMaterial>>()
                .insert((MeshMaterial3d(toon), ToonSwapped));
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_plugins(bevy::pbr::MaterialPlugin::<ToonMaterial>::default());
    app.init_asset::<ToonMaterial>();
    app.add_systems(Update, toonify);
}
