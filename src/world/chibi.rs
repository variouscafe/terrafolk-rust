//! 치비화 — Quaternius 표준 리그(≈6두신)를 레퍼런스 룩(2.5두신 카툰)으로
//! 런타임 변형. Blender 개조 없이 본 엔티티 스케일만 조정한다: 머리 확대·
//! 몸통 압축·팔다리 단축. 스폰 완료(VillagerAnims 부착) 직후 1회 적용
//! (ChibiDone 마커). 이후 프레임 오토펏 재측정 경로는 오토폯이 이미 끝난
//! 뒤라 루트 스케일에 반영되지 않는다(높이 +α 감수 — PoC).

use bevy::prelude::*;

use super::villager::VillagerAnims;

/// 변형 배율 — 레퍼런스(2.5두신 카툰: 큰 머리·통통한 몸·짧은 팔다리) 근사치.
/// 2026-10-19 사용자 요청으로 팔 비율 추가·전체 강화(3.5→2.5두신 방향).
const HEAD_SCALE: f32 = 2.05;
const SPINE_SCALE: f32 = 0.70;
const LEG_SCALE: f32 = 0.58;
const ARM_SCALE: f32 = 0.78;

/// 치비화 적용 완료 마커.
#[derive(Component)]
pub struct ChibiDone;

/// 본 이름별 배율 — 이중 리그(Quaternius CharacterArmature · UE5 마네킹).
/// ⚠️ 체인 최상단 본만 매칭: 로컬 스케일은 자손 전체에 전파되므로 체인의
/// 모든 뼈에 곱하면 0.82³처럼 누적 왜곡된다(2026-10-18 실측).
///   - 머리: Head(양쪽 동일) · 몸통: Abdomen(q)/spine_01(u) — 다리는 형제라
///     영향 없음 · 다리: UpperLeg.X(q)/thigh_x(u) — 종아리는 자손이라 함께 단축 ·
///     팔: UpperArm.X(q)/upperarm_x(u) — 손은 자손이라 함께 단축(2026-10-19).
pub fn bone_scale(name: &str) -> Option<f32> {
    match name {
        "Head" => Some(HEAD_SCALE),
        "Abdomen" | "spine_01" => Some(SPINE_SCALE),
        "UpperLeg.L" | "UpperLeg.R" | "thigh_l" | "thigh_r" => Some(LEG_SCALE),
        "UpperArm.L" | "UpperArm.R" | "upperarm_l" | "upperarm_r" => Some(ARM_SCALE),
        _ => None,
    }
}

/// 스폰 완료 주민(애님 부착됨) 치비화 1회 적용 — 머리·몸통·팔·다리 재비율.
/// 착용 앵커는 손·머리 본을 추적하므로 축소된 위치를 그대로 따라간다.
pub fn chibiify(
    mut commands: Commands,
    q: Query<Entity, (With<VillagerAnims>, Without<ChibiDone>)>,
    children_q: Query<&Children>,
    mut bones: Query<(&Name, &mut Transform), Without<ChibiDone>>,
) {
    for entity in q.iter() {
        for descendant in children_q.iter_descendants(entity) {
            if let Ok((name, mut tr)) = bones.get_mut(descendant)
                && let Some(scale) = bone_scale(&name.to_string())
            {
                tr.scale *= scale;
            }
        }
        commands.entity(entity).insert(ChibiDone);
    }
}
