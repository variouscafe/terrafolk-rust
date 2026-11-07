//! 맵 소품 툰화 — 건물·암석·유물·동물 등 GLB 소품의 StandardMaterial을
//! 캐릭터와 동일한 일본 2.5D 애니 셀(toon.wgsl, toon::ToonMaterial)로
//! 교체한다(2026-11-07 사용자 요청 — "캐릭터처럼 카툰 풍으로". 구 folk
//! 라인 0.8.7x toon_props의 스왑 패턴 계승, 셰이더는 2026-11-03 주민용
//! 2.5D 셀과 공유 — 소품·캐릭터가 같은 감광·틴트로 한 화면을 이룬다).
//! 씬 자손 순회 후 메시 단위 멱등 스왑(ToonSwapped 마커) — GLB 재질이
//! 비동기 삽입되는 어떤 시점에도 다음 프레임 안에 잡고, 라이브 자원
//! 리스폰(상태 변화 재생성)도 자동 추적된다.
//!
//! **나무·초본은 제외**(구 라인 9차 사용자 보고 '나무에 선 추가됨') —
//! 잎 덩어리의 셀 경계가 윤곽선으로 읽힌다. 유기 형태는 지형과 동일하게
//! 매끈한 PBR(가치대비는 형태 간 대비 원칙). 현재 라인의 나무는 전종
//! Quaternius 3D(카드 리페인트는 구 라인)이므로 나무 전종 제외.
//! 모닥불 불꽃(CampfireFlame)·포인트 라이트는 PropRoot의 형제 엔티티라
//! 순회 대상이 아니다 — 발광 연출은 그대로.
//!
//! 섀도 수신은 포기(캐릭터와 동일 계약) — 커스텀 프래그먼트는 bevy
//! 섀도 샘플링에 묶이지 않는다. 투사(cast)는 뎁스 패스가 재질 무관이라
//! 유지되고, 지면 그림자가 장면의 그림자 축을 담당한다.
//! 킬스위치 `?tfdbg=no_toon_props`(원본 PBR로 즉시 복귀 — A/B 비교).

use super::data::PropKind;
use bevy::asset::AssetId;
use bevy::image::Image;
use bevy::pbr::StandardMaterial;
use bevy::platform::collections::HashMap;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::toon::{ToonMaterial, ToonSwapped};

/// 스왑 제외 — 유기 형태(셀 램프 경계가 윤곽선으로 읽히는 잎 덩어리류).
fn organic(kind: PropKind) -> bool {
    matches!(
        kind,
        PropKind::TreeBroadleaf
            | PropKind::TreePine
            | PropKind::TreePalm
            | PropKind::TreeSapling
            | PropKind::TreeDead
            | PropKind::BerryBush
            | PropKind::FlowerClump
            | PropKind::GrassClump
    )
}

/// 텍스처 없는 단색 재질용 1×1 흰 텍스처(스왑당 1회 생성 — toon.rs 패턴).
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
                bevy::asset::RenderAssetUsages::default(),
            ))
        })
        .clone()
}

/// 소품 씬 자손의 표준 재질을 애니 셀로 교체 — 매 프레임 멱등 스캔.
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
    if crate::net::dbg_gate::off("no_toon_props") {
        return;
    }
    if roots.is_empty() {
        return;
    }
    for (root, pr) in roots.iter() {
        if let Some(pr) = pr {
            if organic(pr.kind) {
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
                    // 텍스처는 원본 재질에서 그대로 물려받는다(디자인 유지,
                    // 조명 응답만 카툰). 지붕 파랑은 테라코타로 잠금.
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
    app.add_systems(Update, toonify_props);
}
