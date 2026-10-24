//! 발바닥 수평 고정 — Foot 본의 월드 회전을 바인드값으로 잠근다
//! (2026-10-23 7차 '뒤꿈치 들림' 근본 수정 + 2026-10-24 v2 Walk 확장).
//!
//! 원인(실측): Idle·Interact·Wave 클립은 Foot 본 채널이 없어 바인드에
//! 방치되는데, 발 메시 가중치의 상당수가 정강이 본에 묶여 다리 스윙이
//! 부츠를 발목 앵커 축으로 피치시킨다(발끝 접지·뒤꿈치 부유).
//! Walk는 클립이 Foot 회전을 구동 — 지면 근접(스탠스)일 때만 바인드로
//! 블렌드해 푸시오프의 과장된 뒤꿈치 들림·외반을 눌러 편다("발은 지면에").

use bevy::prelude::*;

use super::anim::{AnimKind, AnimState};
use super::ground::SurfaceField;
use super::rig::{FOOT_L_BIND, FOOT_R_BIND};

/// 스탠스 판정 갭(m) — 앵커가 지면에서 이 값 이내면 접지(접지 시 갭 ≈
/// −SOLE_CLEAR).
const STANCE_EPS: f32 = 0.03;

/// Foot 본 로컬 회전을 `parent_world⁻¹ ∘ (액터요 ∘ bind)`로 재작성.
/// 잠금 강도 w: 비보행=1(완전 잠금), Walk=지면 근접도(스탠스 가중).
/// ⚠️ 액터 회전은 GlobalTransform으로 읽는다 — Transform을 &mut(본)와
/// 함께 물면 B0001 쿼리 충돌 panic(2026-10-23 wasm 부팅 사망 실측).
pub fn foot_lock<F: Resource + SurfaceField>(
    world: Res<F>,
    q: Query<(Entity, &AnimState)>,
    children_q: Query<&Children>,
    names: Query<&Name>,
    parents: Query<&ChildOf>,
    gts: Query<&GlobalTransform>,
    mut bones: Query<&mut Transform>,
) {
    for (root, state) in &q {
        let Ok(vgt) = gts.get(root) else { continue };
        let yaw = vgt.compute_transform().rotation;
        for d in children_q.iter_descendants(root) {
            let Ok(name) = names.get(d) else { continue };
            let want = match name.as_str() {
                "Foot.L" => yaw * FOOT_L_BIND,
                "Foot.R" => yaw * FOOT_R_BIND,
                _ => continue,
            };
            let Ok(gt) = gts.get(d) else { continue };
            let w = if state.0 == AnimKind::Walk {
                let gy = world
                    .surface_height(gt.translation().x, gt.translation().z)
                    .max(world.stand_min_y());
                ((STANCE_EPS - (gt.translation().y - gy)) / STANCE_EPS).clamp(0.0, 1.0)
            } else {
                1.0
            };
            if w <= 0.0 {
                continue;
            }
            let Ok(parent) = parents.get(d) else { continue };
            let Ok(pgt) = gts.get(parent.0) else { continue };
            let local = pgt.compute_transform().rotation.inverse() * want;
            if let Ok(mut bone) = bones.get_mut(d) {
                bone.rotation = bone.rotation.slerp(local, w);
            }
        }
    }
}
