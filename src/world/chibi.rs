//! 치비화 — Quaternius 표준 리그(≈6두신)를 타겟 룩(3.5~4두신, Memoji풍)
//! 로 런타임 변형. Blender 개조 없이 본 엔티티 스케일만 조정한다:
//! 머리 확대·몸통 압축·하반부 단축. 스폰 완료(VillagerAnims 부착) 직후
//! 1회 적용(ChibiDone 마커). 이후 프레임 오토펏 재측정 경로는 오토폯이
//! 이미 끝난 뒤라 루트 스케일에 반영되지 않는다(높이 +α 감수 — PoC).

use bevy::prelude::*;

use super::villager::VillagerAnims;

/// 변형 배율 — 타겟 이미지 분석(2026-10-18) 기준 근사치.
const HEAD_SCALE: f32 = 1.65;
const SPINE_SCALE: f32 = 0.82;
const LEG_SCALE: f32 = 0.72;

/// 치비화 적용 완료 마커.
#[derive(Component)]
pub struct ChibiDone;

/// 본 이름별 배율 — Quaternius CharacterArmature 표준 본명.
fn bone_scale(name: &str) -> Option<f32> {
    match name {
        "Head" | "Head_end" => Some(HEAD_SCALE),
        "Spine" | "Spine1" | "Spine2" => Some(SPINE_SCALE),
        "UpperLeg.L" | "UpperLeg.R" | "LowerLeg.L" | "LowerLeg.R" => Some(LEG_SCALE),
        _ => None,
    }
}

/// 스폰 완료 주민(애님 부착됨) 치비화 1회 적용. 팔·발은 유지(도구·착용
/// 앵커·착지 호환) — 머리·몸통·다리만 재비율한다.
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
