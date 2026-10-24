//! 발 정합 — Root 직속 발 본(Foot/PT)의 위치·회전을 치비 다리에 맞춘다.
//!
//! 원인(2026-10-23 7차 + 2026-10-24 GLB 실측): qchar 리그에서 Foot/PT는
//! 다리 체인과 형제(Root 직속)라 LEG_SCALE 치비 축소가 전파되지 않는다.
//! Walk/Punch 계열 클립은 Foot translation을 **원본 다리 길이 공간의 저작값**으로
//! 매 프레임 재작성 — 발목 본이 치비 정강이 끝보다 0.46 에셋m 아래에 남아
//! 다리가 발목에 못 미치는 채 스윙한다(발 부유·발목 무꺾임·과보폭).
//!
//! 담당 2가지:
//! 1. **위치** — map_foot_local로 저작값(구동 애님) 또는 bind(그 외)을
//!    고관절 피벗 중심 LEG_SCALE 수축 사상. 피벗은 매 프레임 고관절 GT로
//!    갱신(Body 본 보행 밥 추종).
//! 2. **회전** — 발바닥 수평. Idle 계열 클립엔 Foot 채널이 없어 바인드에
//!    방치되는데 발 메시 가중치 일부가 정강이에 묶여 스윙이 부츠를 피치
//!    시키므로 바인드로 완전 잠근다. Walk는 클립이 회전을 구동 — 지면
//!    근접(스탠스)일 때만 바인드로 블렌드해 푸시오프 과장을 눌러 편다.

use bevy::prelude::*;

use super::anim::{AnimKind, AnimState};
use super::ground::SurfaceField;
use super::proportion::FootRig;
use super::rig::{FOOT_L_BIND, FOOT_R_BIND, map_foot_local};

/// 스탠스 판정 갭(m) — 앵커가 지면에서 이 값 이내면 접지(접지 시 갭 ≈
/// −SOLE_CLEAR).
const STANCE_EPS: f32 = 0.03;

/// 발 본 위치 사상 + 발바닥 수평 잠금.
/// ⚠️ 액터 회전은 GlobalTransform으로 읽는다 — Transform을 &mut(본)와
/// 함께 물면 B0001 쿼리 충돌 panic(2026-10-23 wasm 부팅 사망 실측).
pub fn foot_lock<F: Resource + SurfaceField>(
    world: Res<F>,
    q: Query<(&AnimState, &FootRig, &GlobalTransform)>,
    parents: Query<&ChildOf>,
    gts: Query<&GlobalTransform>,
    mut bones: Query<&mut Transform>,
) {
    for (state, rig, root_gt) in &q {
        let yaw = root_gt.compute_transform().rotation;
        let driven = state.0.drives_foot_bones();
        for side in &rig.sides {
            let (Ok(hip_gt), Ok(parent)) = (gts.get(side.hip), parents.get(side.foot)) else {
                continue;
            };
            let Ok(root_bone) = gts.get(parent.0) else {
                continue;
            };
            // 고관절 피벗(발 본 부모 = Root 본 로컬) — Body 밥 반영 매 프레임 갱신.
            let pivot = root_bone
                .affine()
                .inverse()
                .transform_point3(hip_gt.translation());
            // 위치 — 구동 애님은 클립 저작값을, 그 외엔 bind를 사상해 쓴다.
            for (bone, bind) in [(side.foot, side.bind[0]), (side.pt, side.bind[1])] {
                if let Ok(mut tr) = bones.get_mut(bone) {
                    let authored = if driven { tr.translation } else { bind };
                    tr.translation = map_foot_local(authored, pivot);
                }
            }
            // 회전 — 발바닥 수평(PT는 헬퍼라 잠금 불필요).
            let Ok(gt) = gts.get(side.foot) else { continue };
            let want = yaw * if side.left { FOOT_L_BIND } else { FOOT_R_BIND };
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
            let local = root_bone.compute_transform().rotation.inverse() * want;
            if let Ok(mut bone) = bones.get_mut(side.foot) {
                bone.rotation = bone.rotation.slerp(local, w);
            }
        }
    }
}
