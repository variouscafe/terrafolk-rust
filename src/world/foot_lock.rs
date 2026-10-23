//! 발바닥 수평 고정 — Walk 외 애님에서 Foot 본의 월드 회전을 바인드값으로
//! 잠근다(2026-10-23 7차 '뒤꿈치 들림' 근본 수정).
//!
//! 원인(실측): Idle·Interact·Wave 클립은 다리 본(Upper/LowerLeg)만 애니메이션
//! 하고 Foot 본 채널이 없어 바인드에 방치되는데, 발 메시 가중치의 상당수가
//! 정강이 본에 묶여 있다(m-farmer Feet 프리미티브 기준 정강이 31%·발 19%).
//! 다리 스윙이 부츠를 발목 앵커 축으로 피치시켜 **발끝만 접지하고 뒤꿈치가
//! 뜬다**(배포 0.8.8 feet_check 확대 판독). Walk는 클립이 Foot 회전을 직접
//! 구동하므로 해당 없음.
//!
//! 잠금이 안전한 이유: Foot 회전을 건드리지 않는 클립에서는 애님 플레이어와
//! 쓰기 충돌이 없다(치비화 bone scale과 같은 원리). Walk 전환 순간부터는
//! 클립 값이 매 프레임 덮어쓴다.
//!
//! 기준 회전: qchar 6종 전부 동일한 바인드 월드 회전(오프라인 검증
//! 2026-10-23 — 발바닥이 수평으로 놓인 자세)을 상수로 쓰고, 주민 요(회전)는
//! 런타임에 합성한다. PT 본은 메시 가중치 0(리그 보조본)이라 다루지 않는다.

use bevy::prelude::*;

use super::villager::{AnimKind, VillagerAnims};

/// Foot.L 바인드 월드 회전(qchar 공통 — 에셋 아마추어 스페이스).
const FOOT_L_BIND: Quat = Quat::from_xyzw(0.61441, 0.34804, -0.34615, 0.61769);
/// Foot.R 바인드 월드 회전.
const FOOT_R_BIND: Quat = Quat::from_xyzw(0.70403, 0.04097, -0.04079, 0.70781);

/// 비보행 애니에서 발바닥 수평 유지 — Foot 본 로컬 회전을
/// `parent_world⁻¹ ∘ (주민요 ∘ bind)`로 매 프레임 재작성.
/// ⚠️ 주민 회전은 GlobalTransform으로 읽는다 — Transform을 &mut(본)와
/// 함께 물면 B0001 쿼리 충돌 panic(실측 2026-10-23 wasm 부팅 사망).
pub fn foot_lock(
    q: Query<(Entity, &VillagerAnims)>,
    children_q: Query<&Children>,
    names: Query<&Name>,
    parents: Query<&ChildOf>,
    gts: Query<&GlobalTransform>,
    mut bones: Query<&mut Transform>,
) {
    for (root, anims) in &q {
        if anims.current == AnimKind::Walk {
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

pub fn plugin(app: &mut App) {
    app.add_systems(Update, foot_lock);
}
