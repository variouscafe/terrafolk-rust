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
/// 발 폭(x) — 높이·전후는 bone_scale Foot 항목(비균등) 참조.
const FOOT_SCALE: f32 = 0.45;

/// 치비화 적용 완료 마커.
#[derive(Component)]
pub struct ChibiDone;

/// 발바닥 지면 스냅 완료 마커(1회).
#[derive(Component)]
pub struct SettleDone;

/// 루트 하강 지시(마커 지연 적용 — Transform 충돌 회피, ScaleMark 패턴).
#[derive(Component)]
pub struct DropMark(pub f32);

pub fn apply_drop_marks(
    mut commands: Commands,
    mut q: Query<(Entity, &DropMark, &mut Transform)>,
) {
    for (e, mark, mut tr) in &mut q {
        tr.translation.y -= mark.0;
        commands.entity(e).remove::<DropMark>();
    }
}

/// 발바닥 지면 스냅(2026-10-23 '뒤꿈치 들고 걷기' 보고) — 다리 축소로
/// 발바닥이 원본보다 올라가는데 루트 높이는 그대로라 발이 지면에서 뜬다
/// (치비화는 오토폿 완료 후라 루트 스케일에 반영되지 않는다 — 기존 PoC
/// 감수 사항을 상용 수준으로 닫는다). Foot 본 월드 y(≈발바닥)가 루트
/// y(지면 클램프)보다 높은 만큼 루트를 내려 붙인다. 애님으로 Foot가
/// 움직여도 스냅은 1회(스폰 시) — 걷기 중 발 들기는 애님 자연 동작.
pub fn chibi_settle(
    mut commands: Commands,
    q: Query<(Entity, &Transform), (With<ChibiDone>, Without<SettleDone>)>,
    children_q: Query<&Children>,
    names: Query<&Name>,
    feet: Query<&GlobalTransform>,
) {
    for (root, tr) in &q {
        // Foot 본(양쪽) 월드 y 최소 — 발바닥 근사
        let mut foot_y: Option<f32> = None;
        for d in children_q.iter_descendants(root) {
            let Ok(name) = names.get(d) else { continue };
            let n = name.as_str();
            if n == "Foot.L" || n == "Foot.R" || n == "foot_l" || n == "foot_r" {
                if let Ok(gt) = feet.get(d) {
                    foot_y = Some(foot_y.map_or(gt.translation().y, |m: f32| {
                        m.min(gt.translation().y)
                    }));
                }
            }
        }
        if let Some(fy) = foot_y {
            let drop = (fy - tr.translation.y).max(0.0);
            if drop > 0.005 {
                commands.entity(root).insert(DropMark(drop));
            }
        }
        commands.entity(root).insert(SettleDone);
    }
}

/// 본 이름별 배율 — 이중 리그(Quaternius CharacterArmature · UE5 마네킹).
/// ⚠️ 체인 최상단 본만 매칭: 로컬 스케일은 자손 전체에 전파되므로 체인의
/// 모든 뼈에 곱하면 0.82³처럼 누적 왜곡된다(2026-10-18 실측).
///   - 머리: Head(양쪽 동일) · 몸통: Abdomen(q)/spine_01(u) — 다리는 형제라
///     영향 없음 · 다리: UpperLeg.X(q)/thigh_x(u) — 종아리는 자손이라 함께 단축 ·
///     팔: UpperArm.X(q)/upperarm_x(u) — 손은 자손이라 함께 단축(2026-10-19).
///   - 발: qchar 리그는 Foot.X가 **Body 직속**(다리 체인과 형제)이라 다리 축소가
///     전파되지 않는다 — 명시 매칭 없이 두면 원본 크기 발이 짧아진 다리에 붙어
///     스키처럼 앞으로 뻗는다(2026-10-22 직접 플레이 QA '패들 발'). PT(발가락)는
///     Foot 자손이라 함께 축소된다. UE5(foot_l)는 calf_l 자손이라 전파됨.
///   - 발은 **비균등**(2026-10-23 3차 보고 '앞뒤로 길다'): 원본이 운동화형
///     대형이라 균등 0.45로도 전후 길이가 남는다 — 폭(x) 0.45·높이(y) 0.38
///     ·전후(z) 0.30으로 뭉툭한 부츠 비율.
pub fn bone_scale(name: &str) -> Option<Vec3> {
    match name {
        "Head" => Some(Vec3::splat(HEAD_SCALE)),
        "Abdomen" | "spine_01" => Some(Vec3::splat(SPINE_SCALE)),
        "UpperLeg.L" | "UpperLeg.R" | "thigh_l" | "thigh_r" => Some(Vec3::splat(LEG_SCALE)),
        "Foot.L" | "Foot.R" => Some(Vec3::new(FOOT_SCALE, 0.38, 0.30)),
        "UpperArm.L" | "UpperArm.R" | "upperarm_l" | "upperarm_r" => Some(Vec3::splat(ARM_SCALE)),
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
