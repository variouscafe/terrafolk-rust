//! 리그 디스크립터 — 본 이름표·바인드 상수의 단일 원천.
//! 이중 리그(Quaternius CharacterArmature · UE5 마네킹) 지원.
//! ⚠️ 근거 실측(2026-10-23, 7차 뒤꿈치 조사 — 상세는 메모리
//! qchar-foot-rig-anatomy): PT.L/R은 스킨 가중치 0인 보조본,
//! Foot.X는 Root 직속(다리 체인과 형제), Idle/Interact/Wave 클립엔
//! Foot/PT 채널이 없다. 앵커 스캔은 qchar 이름만 — 헤어 GLB가 ubase
//! 리그 뼈(foot_l 등, 지면 2.7m 아래 스폰)를 계층에 섞어오기 때문.

use bevy::prelude::*;

/// 발 앵커 측정 목록 — 발목(Foot)·폴 타깃 헬퍼(PT, 스킨 가중치 0)·말단 리프.
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

/// 다리 체인 본(leg_ik 2본 솔브) — 고관절(상퇴)·무릎(하퇴)·발목(Foot 원점,
/// 바인드에서 LowerLeg_end와 일치 — 2026-10-24 GLB 실측). 좌[0]·우[1].
pub const HIP_BONES: [&str; 2] = ["UpperLeg.L", "UpperLeg.R"];
pub const KNEE_BONES: [&str; 2] = ["LowerLeg.L", "LowerLeg.R"];
pub const ANKLE_BONES: [&str; 2] = ["Foot.L", "Foot.R"];

/// 고관절 횡간 수렴 배율은 프로파일(hip_narrow)로 — 뼈 스케일은 본 위치를
/// 못 옮기므로 치비화가 다리만 줄이면 고관절이 원본 폭으로 남아 허벅지가
/// 벌어진다(사람처럼 = 다리 굵기 대비 좁은 골반, 2026-10-24 사용자 보고).
/// 고관절 translation.x(chibiify)와 발 본 위치 사상(map_foot_local)의
/// 횡축이 소비한다.

/// 횡간 수렴 대상 본 — 고관절(상퇴)만. Foot/PT의 횡간은 map_foot_local이
/// 고관절 피벗 사상에 통합 관할한다(별도 x-narrow 중복 제거, 2026-10-24).
pub fn narrow_bone(name: &str) -> bool {
    HIP_BONES.contains(&name)
}

/// 치비 발 위치 사상 — Root 직속 발 본(Foot/PT)의 저작 translation을 현재
/// 프로파일의 다리에 정합한다: 고관절 피벗 중심 leg 수축 + 횡간 hip_narrow
/// 수렴. 프로파일이 REAL(전부 1)이면 항등 사상(원본 저작값 그대로).
///
/// 근거(2026-10-24 GLB 실측): 원본 리그는 발목 본 == 다리 체인 끝(바인드
/// 편차 0)이 성립하지만 Foot/PT는 Root 직속이라 leg 축소가 전파되지
/// 않는다 — 축소 후 발목 본이 정강이 끝보다 0.46 에셋m 아래에 남는다
/// (다리가 발목에 못 미치는 채 스윙 = 발 부유·발목 무꺾임·과보폭의 원인,
/// 0.8.20 캡처 보고). 피벗 사상 하나로 바인드·Walk/Punch 클립 저작값이
/// 전부 변형 체인과 정합하고, 스킨 밑창이 최저 앵커와 0.07~1.6mm 이내로
/// 붙는다(6종 LBS 검증).
pub fn map_foot_local(
    authored: bevy::math::Vec3,
    hip_local: bevy::math::Vec3,
    p: &ProportionProfile,
) -> bevy::math::Vec3 {
    bevy::math::Vec3::new(
        hip_local.x * p.hip_narrow + p.leg * (authored.x - hip_local.x),
        hip_local.y + p.leg * (authored.y - hip_local.y),
        hip_local.z + p.leg * (authored.z - hip_local.z),
    )
}

/// Foot 본 바인드 월드 회전(qchar 6종 공통 — 발바닥이 수평인 자세,
/// 2026-10-23 오프라인 검증). foot_lock이 소비한다.
pub const FOOT_L_BIND: bevy::math::Quat =
    bevy::math::Quat::from_xyzw(0.61441, 0.34804, -0.34615, 0.61769);
pub const FOOT_R_BIND: bevy::math::Quat =
    bevy::math::Quat::from_xyzw(0.70403, 0.04097, -0.04079, 0.70781);

/// 비율 프로파일 — 치비/리얼 등 캐릭터 룩을 게임이 선택한다(프레임워크는
/// 배율 집합만 제공. 캐릭터 모듈을 통째로 재사용하는 다른 게임이 동일 룩을
/// 보장하는 단일 스펙 단위).
/// 발 배율은 규칙상 다리(leg)와 동일 — 균등 축소(회전된 본 로컬 축에 비균등은
/// shear 왜곡, 4차 보고)이고 Foot 본은 Root 직속이라 다리 축소가 전파되지
/// 않아 같은 배율이어야 원본 발:다리 비율이 보존된다(2026-10-24 — 0.34는
/// 발을 다리 대비 59%로 왜곡해 이상 보행의 한 축).
#[derive(Resource, Clone, Copy)]
pub struct ProportionProfile {
    /// 머리 배율.
    pub head: f32,
    /// 몸통(복부) 배율.
    pub spine: f32,
    /// 다리 배율(발 포함 — 위 규칙).
    pub leg: f32,
    /// 팔 배율.
    pub arm: f32,
    /// 고관절 횡간 수렴 배율(1 = 원본 폭 유지).
    pub hip_narrow: f32,
}

/// **치비 완성 스펙 v1(2026-10-24 전신 감사 확정)** — 두신 ≈2.1 · 다리
/// 25%H · 손끝 허벅지 중간 · 어깨≈머리 폭. (감사 전 이탈: 팔 무릎까지
/// 0.78·몸통 과장 0.70·발:다리 왜곡 0.34)
pub const CHIBI_V1: ProportionProfile = ProportionProfile {
    head: 2.05,
    spine: 0.64,
    leg: 0.50,
    arm: 0.62,
    hip_narrow: 0.6,
};

/// 원본 리그 비율 그대로(≈6두신) — 모든 배율 항등. 세계 물체(나무·건물)
/// 대비 사람 비율이 정합돼야 하는 게임의 선택(2026-10-26 이 게임 채택).
pub const REAL: ProportionProfile = ProportionProfile {
    head: 1.0,
    spine: 1.0,
    leg: 1.0,
    arm: 1.0,
    hip_narrow: 1.0,
};

/// 측정 앵커(발목 본 원점)와 스킨 밑창의 잔차 보정(미터) — 발 위치 사상
/// 적용 후 잔차는 6종 실측 0.07~1.6mm(발 본을 치비 체인 끝에 옮기면 LBS가
/// 밑창을 함께 내린다). 접지 확정감을 위한 소량 클리어런스만 남긴다.
pub const SOLE_CLEAR: f32 = 0.002;

/// 본 이름별 비율 배율 — 이중 리그. ⚠️ 체인 최상단 본만 매칭(로컬 스케일은
/// 자손 전파 — 모든 뼈에 곱하면 0.82³ 누적 왜곡, 2026-10-18 실측).
/// qchar는 Foot.X·PT.X가 Root 직속이라 다리 축소 미전파 — 명시 매칭 필수
/// ('패들 발' 사고). UE5(toe_l)는 calf_l 자손이라 전파됨 — qchar만 매칭.
/// 프로파일이 REAL이면 전부 항등이라 사실상 무동작(원본 리그 그대로).
pub fn bone_scale(name: &str, p: &ProportionProfile) -> Option<bevy::math::Vec3> {
    use bevy::math::Vec3;
    match name {
        "Head" => Some(Vec3::splat(p.head)),
        "Abdomen" | "spine_01" => Some(Vec3::splat(p.spine)),
        "UpperLeg.L" | "UpperLeg.R" | "thigh_l" | "thigh_r" => Some(Vec3::splat(p.leg)),
        "Foot.L" | "Foot.R" | "PT.L" | "PT.R" => Some(Vec3::splat(p.leg)),
        "UpperArm.L" | "UpperArm.R" | "upperarm_l" | "upperarm_r" => Some(Vec3::splat(p.arm)),
        _ => None,
    }
}
