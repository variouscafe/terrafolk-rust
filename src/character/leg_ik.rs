//! 2본 다리 IK — Idle 계열 클립의 다리 스윙을 발목에 정합시킨다
//! (2026-10-24 P2 — foot_lock의 완성형. docs/character-module-redesign.md §2).
//!
//! 근거 리그 실태(2026-10-23 7차 조사 + 2026-10-24 GLB 실측):
//! - 다리 체인 `UpperLeg→LowerLeg→_end`의 _end(발목)는 Foot 본 원점과
//!   **바인드에서 정확히 일치**(편차 0.0). Foot 본은 Root 직속이라 Idle 계열
//!   클립(다리 회전만 구동)이 발목을 못 움직인다 → 클립 무릎 굽힘이 정강이
//!   끝을 발목에서 떼어 밑창-정강이 간극·부츠 들림으로 보인다.
//! - 무릎 자연 굽음 방향 = 모델 +Z 전방(바인드 오프셋 z +0.07~0.11 실측).
//!
//! 동작: chibi_settle이 치비 확정 포즈에서 바인드(체인 길이·본 월드 회전·
//! 방향)를 캡처(LegIkBind). 매 프레임 Walk 외 애님에서 해석적 2본 솔브로
//! 무릎 위치를 정하고 상퇴·하퇴 월드 회전을 `Δ·캡처회전`으로 재작성 —
//! Δ는 캡처 시 방향→현재 방향의 최단호 회전이라 액터 요 회전도 자동 반영.
//! Foot 본 회전은 foot_lock이 계열 담당(여기선 위치만 타깃으로 읽는다).

use bevy::prelude::*;

use super::anim::{AnimKind, AnimState};
use super::rig::{ANKLE_BONES, HIP_BONES, KNEE_BONES};

/// 캡처된 다리 바인드 — settle 시 1회(치비 스케일·핏 반영 완료 포즈).
#[derive(Component)]
pub struct LegIkBind {
    pub sides: [Side; 2],
}

/// 한쪽 다리 바인드 — 엔티티 직접 참조(이름 스캔 무매 프레임). 캡처 후 불변.
#[derive(Clone)]
pub struct Side {
    pub hip: Entity,
    pub knee: Entity,
    pub ankle: Entity,
    /// 대퇴 길이 |무릎−고관절|(치비 반영).
    pub l1: f32,
    /// 하퇴 길이 |발목−무릎|(치비 반영).
    pub l2: f32,
    /// 캡처 시 본 월드 회전(=IK가 되돌리는 기준 자세).
    pub bind_rot_hip: Quat,
    pub bind_rot_knee: Quat,
    /// 캡처 시 체인 방향(월드) — Δ 회전의 원본.
    pub dir_hip_knee: Vec3,
    pub dir_knee_ankle: Vec3,
}

/// settle에서 LegIkBind 캡처 — proportion::chibi_settle이 이름·GT 스캔을
/// 이미 하므로 같은 패스에서 호출한다(프로브 완료 시점 = GT 전파 확정).
pub fn capture_side(
    hip_e: Entity,
    knee_e: Entity,
    ankle_e: Entity,
    gts: &Query<&GlobalTransform>,
) -> Option<Side> {
    let (hip, knee, ankle) = (gts.get(hip_e).ok()?, gts.get(knee_e).ok()?, gts.get(ankle_e).ok()?);
    let (h, k, a) = (hip.translation(), knee.translation(), ankle.translation());
    let l1 = k.distance(h);
    let l2 = a.distance(k);
    if l1 < 1e-4 || l2 < 1e-4 {
        return None;
    }
    Some(Side {
        hip: hip_e,
        knee: knee_e,
        ankle: ankle_e,
        l1,
        l2,
        bind_rot_hip: hip.rotation(),
        bind_rot_knee: knee.rotation(),
        dir_hip_knee: (k - h).normalize(),
        dir_knee_ankle: (a - k).normalize(),
    })
}

/// LegIkBind 부착 가능한 리그 본 인덱스 — 이름으로 해석(L/R별 엔티티).
pub fn leg_entities(root: Entity, children_q: &Query<&Children>, names: &Query<&Name>) -> LegRefs {
    let mut refs = LegRefs::default();
    for d in children_q.iter_descendants(root) {
        let Ok(name) = names.get(d) else { continue };
        let n = name.as_str();
        for i in 0..2 {
            if n == HIP_BONES[i] {
                refs.upper[i] = Some(d);
            } else if n == KNEE_BONES[i] {
                refs.lower[i] = Some(d);
            } else if n == ANKLE_BONES[i] {
                refs.foot[i] = Some(d);
            }
        }
    }
    refs
}

/// 다리 스켈레톤 참조(캡처 직전 1회 사용).
#[derive(Default)]
pub struct LegRefs {
    pub upper: [Option<Entity>; 2],
    pub lower: [Option<Entity>; 2],
    pub foot: [Option<Entity>; 2],
}

/// 매 프레임 2본 IK — Walk 외 애님에서 정강이 끝을 발목에 정합.
/// ⚠️ foot_lock과 동일 계약: 본 Transform &mut와 루트 Transform을 같은
/// 쿼리에 물지 않는다(B0001 충돌 — 2026-10-23 wasm 부팅 사망 실측).
pub fn leg_ik(
    q: Query<(&AnimState, &LegIkBind, &GlobalTransform)>,
    gts: Query<&GlobalTransform>,
    parents: Query<&ChildOf>,
    mut bones: Query<&mut Transform>,
) {
    for (state, bind, root_gt) in &q {
        if state.0 == AnimKind::Walk {
            continue; // Walk 클립이 Foot 채널을 직접 구동 — 간섭 금지
        }
        let actor_rot = root_gt.rotation(); // 요만 있으나 회전 전체 반영 무해
        // 무릎 굽음 방향(극) — 모델 +Z 전방을 액터 회전으로 월드화.
        let fwd = actor_rot * Vec3::Z;
        for side in &bind.sides {
            let (Ok(hip_gt), Ok(ankle_gt)) = (gts.get(side.hip), gts.get(side.ankle)) else {
                continue;
            };
            let h = hip_gt.translation();
            let t = ankle_gt.translation();
            let mut span = h.distance(t);
            let reach = side.l1 + side.l2 - 1e-4;
            let floor_ = (side.l1 - side.l2).abs() + 1e-4;
            if span > reach {
                span = reach; // 닿지 않으면 다리를 펴서 최대한 근접
            }
            if span < floor_ {
                span = floor_;
            }
            let u = if h.distance(t) > 1e-5 {
                (t - h).normalize()
            } else {
                continue;
            };
            // 해석적 무릎 위치: u에서 극 방향으로 α만큼 벌어진 대퇴 방향.
            let cos_a = ((side.l1 * side.l1 + span * span - side.l2 * side.l2)
                / (2.0 * side.l1 * span))
                .clamp(-1.0, 1.0);
            let sin_a = (1.0 - cos_a * cos_a).max(0.0).sqrt();
            let mut pole = fwd - u * fwd.dot(u);
            if pole.length_squared() < 1e-6 {
                // 다리가 전방을 거의 따르는 자세 — 측면으로 탈출.
                let lat = actor_rot * Vec3::X;
                pole = lat - u * lat.dot(u);
            }
            let pole = pole.normalize_or_zero();
            let thigh_dir = (u * cos_a + pole * sin_a).normalize_or_zero();
            let knee_pos = h + thigh_dir * side.l1;
            let shin_dir = (t - knee_pos).normalize_or_zero();
            // Δ = 캡처 방향 → 현재 방향 최단호. 원하는 월드 회전 = Δ·캡처회전.
            let d_thigh = Quat::from_rotation_arc(side.dir_hip_knee, thigh_dir);
            let d_shin = Quat::from_rotation_arc(side.dir_knee_ankle, shin_dir);
            write_rot(&side.hip, d_thigh * side.bind_rot_hip, &parents, &gts, &mut bones);
            write_rot(&side.knee, d_shin * side.bind_rot_knee, &parents, &gts, &mut bones);
        }
    }
}

fn write_rot(
    entity: &Entity,
    want_world: Quat,
    parents: &Query<&ChildOf>,
    gts: &Query<&GlobalTransform>,
    bones: &mut Query<&mut Transform>,
) {
    let Ok(parent) = parents.get(*entity) else {
        return;
    };
    let local = match gts.get(parent.0) {
        Ok(pgt) => pgt.rotation().inverse() * want_world,
        Err(_) => want_world,
    };
    if let Ok(mut bone) = bones.get_mut(*entity) {
        bone.rotation = local;
    }
}
