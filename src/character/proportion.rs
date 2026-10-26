//! 치비화·부양량 측정 — 런타임 비율 변형(P1: 기존 world/chibi.rs 이식,
//! 동작 보존). 원본 리그(≈6두신)를 카툰 비율로 본 엔티티 스케일만 조정.
//! P4에서 오프라인 베이크로 대체 예정(docs §2).

use bevy::prelude::*;

use super::rig::{FOOT_ANCHORS, SOLE_CLEAR, bone_scale, map_foot_local};

/// 치비화 적용 대상 마커 — 게임이 '리그+애님 준비 완료' 시점에 부착.
#[derive(Component)]
pub struct ChibiTarget;

/// 치비화 적용 완료 마커.
#[derive(Component)]
pub struct ChibiDone;

/// 치비 확정 프로브(4프레임 2단계) — 2: 스케일 GT 전파 후 발 본 재배치,
/// 0: 재배치 GT 전파 후 FootLift·앵커·다리 IK 바인드 측정.
#[derive(Component)]
pub struct SettleProbe(pub u8);

/// 치비 부양량(발바닥이 루트 원점에서 뜬 높이) — ground::ground_snap 소비.
#[derive(Component)]
pub struct FootLift(pub f32);

/// 발 앵커 엔티티 목록 + 바인드 FootLift — ground::ground_snap의 동적
/// 추적(발 기준 중력 v2)이 소비. settle 시 1회 캡처.
#[derive(Component)]
pub struct FootAnchors {
    pub entities: Vec<Entity>,
    pub bind_lift: f32,
}

/// 치비 발 리그 참조 — foot_lock이 Root 직속 발 본(Foot/PT) 위치를 치비
/// 다리에 사상하는 데 쓰는 엔티티·바인드. settle 1단계에서 캡처 후 불변.
#[derive(Component)]
pub struct FootRig {
    pub sides: [FootSide; 2],
}

/// 한쪽 발 — 발목·폴타깃 본, 피벗 고관절, 사상 전 바인드 translation.
pub struct FootSide {
    pub left: bool,
    pub foot: Entity,
    /// IK 폴 타깃 헬퍼 본(PT) — 스킨 가중치 0이나 저작 채널이 있어 같이 사상.
    pub pt: Entity,
    pub hip: Entity,
    /// [Foot, PT] 바인드 translation(발 본의 부모 로컬) — 비구동 애님 복원값.
    pub bind: [Vec3; 2],
}

/// 치비화 1회 적용 — ChibiTarget 보유 액터의 자손 본 스케일을 현재
/// 프로파일(rig::ProportionProfile)로 조정 후 2프레임 측정 프로브 예약.
/// REAL 프로파일에서는 항등(원본 리그 그대로) — 발 측정·IK 바인드 절차는
/// 프로파일 무관하게 그대로 수행된다.
pub fn chibiify(
    mut commands: Commands,
    profile: Res<super::rig::ProportionProfile>,
    q: Query<Entity, (With<ChibiTarget>, Without<ChibiDone>)>,
    children_q: Query<&Children>,
    mut bones: Query<(&Name, &mut Transform), Without<ChibiDone>>,
) {
    for entity in &q {
        for descendant in children_q.iter_descendants(entity) {
            if let Ok((name, mut tr)) = bones.get_mut(descendant) {
                if let Some(scale) = bone_scale(name.as_str(), &profile) {
                    tr.scale *= scale;
                }
                // 고관절·발 앵커 횡간 수렴 — 스케일이 못 옮기는 본 위치 보정.
                if super::rig::narrow_bone(name.as_str()) {
                    tr.translation.x *= profile.hip_narrow;
                }
            }
        }
        commands.entity(entity).insert((ChibiDone, SettleProbe(4)));
    }
}

/// settle 1단계 — Root 직속 발 본을 치비 다리 길이에 재배치(FootRig 캡처).
/// Foot/PT의 부모(Root 본) 로컬에서 고관절 피벗을 잡고, 바인드 translation
/// 에 map_foot_local을 적용해 되쓴다. 사상 전 저작값은 bind로 저장(비구동
/// 애님에서의 복원값). 어느 쪽 요소가 없으면 리그가 맞지 않는 것 — 프로브만
/// 조용히 제거하고 발 사상은 비활성(foot_lock이 FootRig 없으면 스킵).
fn capture_foot_rig(
    root: Entity,
    children_q: &Query<&Children>,
    names: &Query<&Name>,
    parents: &Query<&ChildOf>,
    gts: &Query<&GlobalTransform>,
    bones: &mut Query<&mut Transform>,
    profile: &super::rig::ProportionProfile,
) -> Option<FootRig> {
    // [좌우][발목, 폴타깃, 고관절] — 정확한 이름 매칭(헤어 GLB 뼈 오염 제외).
    let mut found: [[Option<Entity>; 3]; 2] = [[None; 3], [None; 3]];
    for d in children_q.iter_descendants(root) {
        let Ok(name) = names.get(d) else { continue };
        let (side, slot) = match name.as_str() {
            "Foot.L" => (0, 0),
            "PT.L" => (0, 1),
            "UpperLeg.L" => (0, 2),
            "Foot.R" => (1, 0),
            "PT.R" => (1, 1),
            "UpperLeg.R" => (1, 2),
            _ => continue,
        };
        if found[side][slot].is_none() {
            found[side][slot] = Some(d);
        }
    }
    let mut sides: [Option<FootSide>; 2] = [None, None];
    for (i, foot_pt_hip) in found.into_iter().enumerate() {
        let [Some(foot), Some(pt), Some(hip)] = foot_pt_hip else {
            continue;
        };
        let (Ok(hip_gt), Ok(parent)) = (gts.get(hip), parents.get(foot)) else {
            continue;
        };
        let Ok(root_gt) = gts.get(parent.0) else {
            continue;
        };
        // 고관절 피벗 — 발 본 부모(Root 본) 로컬 좌표(에셋 단위).
        let pivot = root_gt
            .affine()
            .inverse()
            .transform_point3(hip_gt.translation());
        let mut bind = [Vec3::ZERO; 2];
        for (slot, bone) in [foot, pt].into_iter().enumerate() {
            if let Ok(mut tr) = bones.get_mut(bone) {
                bind[slot] = tr.translation;
                tr.translation = map_foot_local(tr.translation, pivot, profile);
            }
        }
        sides[i] = Some(FootSide {
            left: i == 0,
            foot,
            pt,
            hip,
            bind,
        });
    }
    let [Some(a), Some(b)] = sides else {
        return None;
    };
    Some(FootRig { sides: [a, b] })
}

/// 발바닥 부양량 측정 — 발 하위 본(양쪽·_end 리프 포함) 월드 y 최소에서
/// 루트 y를 뺀 값 + SOLE_CLEAR를 FootLift로 저장. 루트 y는 상쇄되므로
/// 스폰 클램프 오차 무관. 하강 적용은 ground_snap이 다음 프레임부터 담당.
#[allow(clippy::type_complexity)]
pub fn chibi_settle(
    mut commands: Commands,
    profile: Res<super::rig::ProportionProfile>,
    mut q: Query<(Entity, &mut SettleProbe)>,
    children_q: Query<&Children>,
    names: Query<&Name>,
    parents: Query<&ChildOf>,
    gts: Query<&GlobalTransform>,
    mut bones: Query<&mut Transform>,
) {
    for (root, mut probe) in &mut q {
        probe.0 = probe.0.saturating_sub(1);
        if probe.0 == 2 {
            // 1단계 — 치비 스케일 GT 전파 완료. 발 본을 재배치해 두면
            // 2단계(0) 측정이 치비 발 위치 기준으로 잡힌다.
            if let Some(rig) = capture_foot_rig(
                root,
                &children_q,
                &names,
                &parents,
                &gts,
                &mut bones,
                &profile,
            ) {
                commands.entity(root).insert(rig);
            } else {
                commands.entity(root).remove::<SettleProbe>();
            }
            continue;
        }
        if probe.0 > 0 {
            continue; // 재배치 GT 전파 대기
        }
        let Ok(root_gt) = gts.get(root) else {
            commands.entity(root).remove::<SettleProbe>();
            continue;
        };
        let root_y = root_gt.translation().y;
        let mut foot_y: Option<f32> = None;
        let mut anchor_entities = Vec::new();
        for d in children_q.iter_descendants(root) {
            let Ok(name) = names.get(d) else { continue };
            if FOOT_ANCHORS.contains(&name.as_str())
                && let Ok(gt) = gts.get(d)
            {
                anchor_entities.push(d);
                foot_y =
                    Some(foot_y.map_or(gt.translation().y, |m: f32| m.min(gt.translation().y)));
            }
        }
        let drop = foot_y.map_or(0.0, |fy| (fy - root_y).max(0.0));
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
                && let Some(side) = super::leg_ik::capture_side(*hip, *knee, *ankle, &gts)
            {
                sides[i] = Some(side);
            }
        }
        let mut cmd = commands.entity(root);
        cmd.insert(FootLift(drop + SOLE_CLEAR))
            .remove::<SettleProbe>();
        cmd.insert(FootAnchors {
            entities: anchor_entities,
            bind_lift: drop + SOLE_CLEAR,
        });
        if let [Some(a), Some(b)] = sides {
            cmd.insert(super::leg_ik::LegIkBind { sides: [a, b] });
        }
    }
}
