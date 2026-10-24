//! 치비화·부양량 측정 — 런타임 비율 변형(P1: 기존 world/chibi.rs 이식,
//! 동작 보존). 원본 리그(≈6두신)를 카툰 비율로 본 엔티티 스케일만 조정.
//! P4에서 오프라인 베이크로 대체 예정(docs §2).

use bevy::prelude::*;

use super::rig::{bone_scale, FOOT_ANCHORS, SOLE_CLEAR};

/// 치비화 적용 대상 마커 — 게임이 '리그+애님 준비 완료' 시점에 부착.
#[derive(Component)]
pub struct ChibiTarget;

/// 치비화 적용 완료 마커.
#[derive(Component)]
pub struct ChibiDone;

/// 발바닥 스냅 측정 프로브(2프레임 — 스케일 GlobalTransform 전파 대기).
#[derive(Component)]
pub struct SettleProbe(pub u8);

/// 치비 부양량(발바닥이 루트 원점에서 뜬 높이) — ground::ground_snap 소비.
#[derive(Component)]
pub struct FootLift(pub f32);

/// 치비화 1회 적용 — ChibiTarget 보유 액터의 자손 본 스케일 조정 후
/// 2프레임 측정 프로브 예약.
pub fn chibiify(
    mut commands: Commands,
    q: Query<Entity, (With<ChibiTarget>, Without<ChibiDone>)>,
    children_q: Query<&Children>,
    mut bones: Query<(&Name, &mut Transform), Without<ChibiDone>>,
) {
    for entity in &q {
        for descendant in children_q.iter_descendants(entity) {
            if let Ok((name, mut tr)) = bones.get_mut(descendant)
                && let Some(scale) = bone_scale(name.as_str())
            {
                tr.scale *= scale;
            }
        }
        commands.entity(entity).insert((ChibiDone, SettleProbe(2)));
    }
}

/// 발바닥 부양량 측정 — 발 하위 본(양쪽·_end 리프 포함) 월드 y 최소에서
/// 루트 y를 뺀 값 + SOLE_CLEAR를 FootLift로 저장. 루트 y는 상쇄되므로
/// 스폰 클램프 오차 무관. 2프레임 대기(전파 확정 후). 하강 적용은
/// ground_snap이 다음 프레임부터 담당.
#[allow(clippy::type_complexity)]
pub fn chibi_settle(
    mut commands: Commands,
    mut q: Query<(Entity, &mut SettleProbe)>,
    roots: Query<&Transform>,
    children_q: Query<&Children>,
    names: Query<&Name>,
    feet: Query<&GlobalTransform>,
) {
    for (root, mut probe) in &mut q {
        probe.0 = probe.0.saturating_sub(1);
        if probe.0 > 0 {
            continue; // GT 전파 대기
        }
        let Ok(tr) = roots.get(root) else {
            commands.entity(root).remove::<SettleProbe>();
            continue;
        };
        let mut foot_y: Option<f32> = None;
        for d in children_q.iter_descendants(root) {
            let Ok(name) = names.get(d) else { continue };
            if FOOT_ANCHORS.contains(&name.as_str())
                && let Ok(gt) = feet.get(d)
            {
                foot_y = Some(foot_y.map_or(gt.translation().y, |m: f32| {
                    m.min(gt.translation().y)
                }));
            }
        }
        let drop = foot_y.map_or(0.0, |fy| (fy - tr.translation.y).max(0.0));
        // 다리 IK 바인드 캡처 — 같은 프로브 시점(GT 전파 확정)에 체인 길이·
        // 회전·방향을 저장(leg_ik의 Δ 기준 포즈). 양쪽 체인이 온전해야 부착.
        let refs = super::leg_ik::leg_entities(root, &children_q, &names);
        let mut sides: [Option<super::leg_ik::Side>; 2] = [None, None];
        for (i, ((upper, lower), foot)) in refs
            .upper
            .iter()
            .zip(&refs.lower)
            .zip(&refs.foot)
            .enumerate()
        {
            if let (Some(hip), Some(knee), Some(ankle)) = (upper, lower, foot)
                && let Some(side) = super::leg_ik::capture_side(*hip, *knee, *ankle, &feet)
            {
                sides[i] = Some(side);
            }
        }
        let mut cmd = commands.entity(root);
        cmd.insert(FootLift(drop + SOLE_CLEAR)).remove::<SettleProbe>();
        if let [Some(a), Some(b)] = sides {
            cmd.insert(super::leg_ik::LegIkBind { sides: [a, b] });
        }
    }
}
