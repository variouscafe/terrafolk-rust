//! 맵 소품 툰화(2026-10-30 7차) — 건물·동물·유물 등 GLB 소품의
//! StandardMaterial을 3밴드 셀 램프(toon.wgsl)로 교체한다. 지형 루마
//! 밴딩(terrain_splat.wgsl)과 짝을 이뤄 화면 전체를 '카툰을 보는 듯한'
//! 감광으로 통일한다. 구 주민 toonify(villager toon)의 스왑 패턴 계승 —
//! 씬 자손 순회 후 메시 단위 멱등 스왑(ToonSwapped 마커).
//!
//! **나무·초본은 제외**(2026-10-30 사용자 보고 '나무에 선 추가됨') — 잎
//! 덩어리의 셀 램프 경계가 윤곽선으로 읽힌다. 유기 형태는 지형과 동일하게
//! 매끈한 PBR + 페인티드 블롭 섀도(8차 원칙: 가치대비는 형태 간 대비).
//!
//! 섀도 수신은 포기(구 캐릭터와 동일 계약) — 커스텀 프래그먼트는 bevy
//! 섀도 샘플링에 묶이지 않는다. 투사(cast)는 뎁스 패스가 재질 무관이라
//! 유지되고, 지면 그림자(6차 페인티드 블롭)가 장면의 그림자 축을 담당한다.

use super::data::PropKind;
use bevy::asset::AssetId;
use bevy::pbr::StandardMaterial;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;

/// 셀 램프 재질 — baseColorFactor 틴트 + (있다면) 원본 텍스처.
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct ToonMaterial {
    #[texture(0)]
    #[sampler(1)]
    pub base: Handle<Image>,
    /// rgb=baseColorFactor(지붕 잠금 적용본) · a=alpha
    #[uniform(2)]
    pub tint: Vec4,
}

impl Material for ToonMaterial {
    fn fragment_shader() -> bevy::shader::ShaderRef {
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
                bevy::render::render_resource::Extent3d {
                    width: 1,
                    height: 1,
                    depth_or_array_layers: 1,
                },
                bevy::render::render_resource::TextureDimension::D2,
                &[255, 255, 255, 255],
                bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                bevy::asset::RenderAssetUsages::default(),
            ))
        })
        .clone()
}

/// 소품 씬 자손의 표준 재질을 셀 램프로 교체 — 매 프레임 멱등 스캔
/// (GLB 재질이 비동기 삽입되는 어떤 시점에도 다음 프레임 안에 잡는다).
/// 라이브 자원 리스폰(상태 변화 재생성)도 자동 추적된다.
#[allow(clippy::type_complexity)] // Bevy 시스템 파라미터 — 씬 순회·재질 캐시
pub fn toonify_props(
    mut commands: Commands,
    roots: Query<
        (Entity, Option<&super::models::PropRoot>),
        Or<(With<super::models::PropRoot>, With<super::prop_fit::DynProp>)>,
    >,
    children: Query<&Children>,
    std_mats: Res<Assets<StandardMaterial>>,
    mut toon_mats: ResMut<Assets<ToonMaterial>>,
    mut images: ResMut<Assets<Image>>,
    q_mat: Query<(&MeshMaterial3d<StandardMaterial>, Option<&ToonSwapped>)>,
    // 원본 재질 → 툰 핸들 캐시 — 소품은 인스턴스 간 재질 공유(인스턴싱 유지).
    mut cache: Local<HashMap<AssetId<StandardMaterial>, Handle<ToonMaterial>>>,
    mut white: Local<Option<Handle<Image>>>,
) {
    if roots.is_empty() {
        return;
    }
    for (root, pr) in roots.iter() {
        // 나무·초본(유기 형태)은 스왑 제외 — 셀 램프 경계가 잎 덩어리에
        // 윤곽선으로 읽힌다(모듈 doc 참조). 매끈한 PBR + 블롭 섀도 유지.
        if let Some(pr) = pr {
            if matches!(
                pr.kind,
                PropKind::TreeBroadleaf
                    | PropKind::TreePine
                    | PropKind::TreeSapling
                    | PropKind::TreeDead
                    | PropKind::BerryBush
                    | PropKind::GrassClump
                    | PropKind::FlowerClump
            ) {
                continue;
            }
        }
        let mut stack = vec![root];
        while let Some(e) = stack.pop() {
            let Ok(kids) = children.get(e) else {
                continue;
            };
            for &k in kids {
                stack.push(k);
                let Ok((mat, None)) = q_mat.get(k) else {
                    continue;
                };
                let handle = cache.entry(mat.id()).or_insert_with(|| {
                    let (tex, tint) = match std_mats.get(&mat.0) {
                        Some(m) => (
                            m.base_color_texture.clone(),
                            super::materials_guard::lock_roof_color(m.base_color.to_linear()),
                        ),
                        None => (None, bevy::color::LinearRgba::WHITE),
                    };
                    let base = tex.unwrap_or_else(|| white_fallback(&mut images, &mut white));
                    toon_mats.add(ToonMaterial {
                        base,
                        tint: tint.to_vec4(),
                    })
                });
                commands
                    .entity(k)
                    .remove::<MeshMaterial3d<StandardMaterial>>()
                    .insert((MeshMaterial3d(handle.clone()), ToonSwapped));
            }
        }
    }
}

pub fn plugin(app: &mut App) {
    app.init_asset::<ToonMaterial>()
        .add_plugins(bevy::pbr::MaterialPlugin::<ToonMaterial>::default())
        .add_systems(Update, toonify_props);
}
