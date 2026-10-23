//! 발바닥 수평 고정 — Walk 외 애님에서 Foot 본의 월드 회전을 바인드값으로
//! 잠근다(2026-10-23 7차 '뒤꿈치 들림' 근본 수정 이식).
//!
//! 원인(실측): Idle·Interact·Wave 클립은 다리 본만 애니메이션하고 Foot 본
//! 채널이 없어 바인드에 방치되는데, 발 메시 가중치의 상당수가 정강이 본에
//! 묶여 다리 스윙이 부츠를 발목 앵커 축으로 피치시킨다(발끝 접지·뒤꿈치
//! 부유). Walk는 클립이 Foot 회전을 직접 구동하므로 해당 없음.
//! Foot 회전을 안 쓰는 클립에서는 애님 플레이어와 쓰기 충돌이 없다.
//! P2에서 leg_ik가 상위 호환으로 대체한다.

use bevy::prelude::*;

use super::anim::{AnimKind, AnimState};
use super::rig::{FOOT_L_BIND, FOOT_R_BIND};

/// 비보행 애님에서 발바닥 수평 유지 — Foot 본 로컬 회전을
/// `parent_world⁻¹ ∘ (액터요 ∘ bind)`로 매 프레임 재작성.
/// ⚠️ 액터 회전은 GlobalTransform으로 읽는다 — Transform을 &mut(본)와
/// 함께 물면 B0001 쿼리 충돌 panic(2026-10-23 wasm 부팅 사망 실측).
pub fn foot_lock(
    q: Query<(Entity, &AnimState)>,
    children_q: Query<&Children>,
    names: Query<&Name>,
    parents: Query<&ChildOf>,
    gts: Query<&GlobalTransform>,
    mut bones: Query<&mut Transform>,
) {
    for (root, state) in &q {
        if state.0 == AnimKind::Walk {
            continue; // 클립이 직접 구동 — 간섭 금지
        }
        let Ok(vgt) = gts.get(root) else { continue };
        let yaw = vgt.compute_transform().rotation;
        for d in children_q.iter_descendants(root) {
            let Ok(name) = names.get(d) else { continue };
            let want = match name.as_str() {
                "Foot.L" => yaw * FOOT_L_BIND,
                "Foot.R" => yaw * FOOT_R_BIND,
                _ => continue,
            };
            let Ok(parent) = parents.get(d) else { continue };
            let Ok(pgt) = gts.get(parent.0) else { continue };
            let local = pgt.compute_transform().rotation.inverse() * want;
            if let Ok(mut bone) = bones.get_mut(d) {
                bone.rotation = local;
            }
        }
    }
}
