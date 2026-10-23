//! 리그 디스크립터 — 본 이름표·바인드 상수의 단일 원천.
//! 이중 리그(Quaternius CharacterArmature · UE5 마네킹) 지원.
//! ⚠️ 근거 실측(2026-10-23, 7차 뒤꿈치 조사 — 상세는 메모리
//! qchar-foot-rig-anatomy): PT.L/R은 스킨 가중치 0인 보조본,
//! Foot.X는 Root 직속(다리 체인과 형제), Idle/Interact/Wave 클립엔
//! Foot/PT 채널이 없다. 앵커 스캔은 qchar 이름만 — 헤어 GLB가 ubase
//! 리그 뼈(foot_l 등, 지면 2.7m 아래 스폰)를 계층에 섞어오기 때문.

/// 발 앵커 측정 목록 — 발목(Foot)·발볼(PT)·말단 리프(_end).
pub const FOOT_ANCHORS: [&str; 8] = [
    "Foot.L",
    "Foot.R",
    "PT.L",
    "PT.R",
    "Foot.L_end",
    "Foot.R_end",
    "PT.L_end",
    "PT.R_end",
];

/// Foot 본 바인드 월드 회전(qchar 6종 공통 — 발바닥이 수평인 자세,
/// 2026-10-23 오프라인 검증). foot_lock이 소비한다.
pub const FOOT_L_BIND: bevy::math::Quat =
    bevy::math::Quat::from_xyzw(0.61441, 0.34804, -0.34615, 0.61769);
pub const FOOT_R_BIND: bevy::math::Quat =
    bevy::math::Quat::from_xyzw(0.70403, 0.04097, -0.04079, 0.70781);

/// 치비화 배율 — 레퍼런스 룩(2.5두신 카툰) 근사. 본 이름별 매항표는
/// proportion::bone_scale.
pub const HEAD_SCALE: f32 = 2.05;
pub const SPINE_SCALE: f32 = 0.70;
pub const LEG_SCALE: f32 = 0.58;
pub const ARM_SCALE: f32 = 0.78;
/// 발 — 균등 강축소(회전된 본 로컬 축에 비균등은 shear 왜곡 — 4차 보고).
pub const FOOT_SCALE: f32 = 0.34;

/// 측정 앵커(발목 본 원점)가 메시 밑창보다 위인 오프셋 보정(미터) —
/// 바인드 δ≈0.023(에셋 단위) × 핏 스케일 ≈0.74 × 발 축소 0.34.
/// FootLift에 더해 **메시 밑창**(앵커 아님)이 지면에 닿게 한다.
pub const SOLE_CLEAR: f32 = 0.006;

/// 본 이름별 치비 배율 — 이중 리그. ⚠️ 체인 최상단 본만 매칭(로컬 스케일은
/// 자손 전파 — 모든 뼈에 곱하면 0.82³ 누적 왜곡, 2026-10-18 실측).
/// qchar는 Foot.X·PT.X가 Root 직속이라 다리 축소 미전파 — 명시 매칭 필수
/// ('패들 발' 사고). UE5(toe_l)는 calf_l 자손이라 전파됨 — qchar만 매칭.
pub fn bone_scale(name: &str) -> Option<bevy::math::Vec3> {
    use bevy::math::Vec3;
    match name {
        "Head" => Some(Vec3::splat(HEAD_SCALE)),
        "Abdomen" | "spine_01" => Some(Vec3::splat(SPINE_SCALE)),
        "UpperLeg.L" | "UpperLeg.R" | "thigh_l" | "thigh_r" => Some(Vec3::splat(LEG_SCALE)),
        "Foot.L" | "Foot.R" | "PT.L" | "PT.R" => Some(Vec3::splat(FOOT_SCALE)),
        "UpperArm.L" | "UpperArm.R" | "upperarm_l" | "upperarm_r" => Some(Vec3::splat(ARM_SCALE)),
        _ => None,
    }
}
