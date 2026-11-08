//! 셀 카드 진단 프로브 — `?tfdbg=celprobe`(2026-11-07 소품 카툰 통일).
//! 팔로우 중인 주민 주변 2×2 격자로 celprops 4종(hut·boulder·gold_vein·
//! bonfire)을 강제 스폰한다 — 카메라가 주민을 따르므로 프레임 보장이 되고,
//! 프레임 헌팅 없이 정면·알파·스케일을 한 컷에서 판정한다.
//! 스폰은 PropRoot 정규 경로(요 0 카드 계약·툰·잉크 전부 실경로).

use bevy::prelude::*;

use super::data::PropKind;
use super::models::{prop_scene, PropRoot};
use super::villager::Villager;

fn celprobe(
    mut commands: Commands,
    world: Res<super::data::WorldData>,
    assets: Res<AssetServer>,
    sel: Res<super::pick::Selection>,
    q_sel: Query<&Transform, With<Villager>>,
    mut spawned: Local<bool>,
) {
    if !crate::net::dbg_gate::on("celprobe") || *spawned {
        return;
    }
    // 카메라 초점 = 선택 주민(pick::Selection.entity — cam_follow와 동일
    // 원천). 미선택이면 스킵(다음 프레임 재시도).
    let Some(entity) = sel.entity else {
        return;
    };
    let Ok(tr) = q_sel.get(entity) else {
        return;
    };
    let pos = tr.translation;
    let kinds = [
        (PropKind::Hut, -4i32, -1),
        (PropKind::Boulder, 1, -1),
        (PropKind::GoldVein, -4, 2),
        (PropKind::Campfire, 1, 2),
    ];
    for (kind, dx, dz) in kinds {
        let (x, z) = (pos.x as i32 + dx, pos.z as i32 + dz);
        let ground = world.height_at(x, z);
        let spec = kind.model();
        commands.spawn((
            PropRoot {
                kind,
                target_h: spec.target_h,
                ground_y: ground,
                rot_y: super::models::CARD_YAW,
                scale_jitter: 1.0,
                variants: spec.variants,
            },
            bevy::world_serialization::WorldAssetRoot(prop_scene(kind, &assets)),
            Transform::from_translation(Vec3::new(
                x as f32 + 0.5,
                ground,
                z as f32 + 0.5,
            )),
        ));
    }
    info!("[celprobe] 셀 카드 4종 스폰 @({:.0},{:.0})", pos.x, pos.z);
    *spawned = true;
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, celprobe);
}
