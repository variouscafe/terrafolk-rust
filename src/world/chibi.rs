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
/// 발 — 균등 강축소(회전된 본 로컬 축에 비균등은 shear 왜곡 — 4차 보고).
const FOOT_SCALE: f32 = 0.34;

/// 치비화 적용 완료 마커.
#[derive(Component)]
pub struct ChibiDone;

/// 발바닥 스냅 측정 프로브(2프레임 카운트 — 스케일 GlobalTransform 전파 대기).
#[derive(Component)]
pub struct SettleProbe(pub u8);

/// 치비 부양량(발바닥이 루트 원점에서 뜬 높이) — 루트 y의 단일 소유자
/// `ground.rs`(매 프레임 surface_height − FootLift)가 소비한다.
#[derive(Component)]
pub struct FootLift(pub f32);

/// 밑창 클리어런스 — 측정 앵커(발목 본 원점)는 메시 밑창보다 위다. 바인드
/// 실측 앵커↔밑창 δ≈0.023(에셋 단위) × 핏 스케일 ≈0.74 × 발 축소 0.34 ≈
/// **0.006m**. 이 만큼을 FootLift에 더해 루트를 내려야 **메시 밑창**(앵커
/// 아님)이 지면에 닿는다 — 미보정 시 전신이 δ만큼 가라앉아 발끝은 파묻히고
/// 뒤꿈치(밑창이 위로 굽는 부츠 형상)만 떠 보인다(7차 '뒤꿈치 들림' 잔여).
const SOLE_CLEAR: f32 = 0.006;

/// 발바닥 부양량 측정(2026-10-23 '뒤꿈치 들고 걷기' 보고 → 동일 재보고로
/// 재설계) — 다리·발 축소로 발바닥이 원본보다 올라가는데 루트 원점은
/// 원본 발바닥 레벨(지면) 그대로라 발이 지면에서 뜬다(치비화는 오토폿
/// 완료 후라 루트 스케일에 반영되지 않는다). 발끝 본(PT/Toe, Foot 본보다
/// 아래)을 포함한 양측 하위 본 월드 y 최소에서 루트 y를 뺀 값을
/// **FootLift로 저장**한다(루트 y는 상쇄 — 지면 클램프 오차 무관).
/// 하강 적용은 ground_snap이 다음 프레임부터 담당(스폰 즉시 하강
/// DropMark 경로는 y 작성 다중화 폐지로 제거).
/// 측정은 2프레임 대기(치비 스케일의 GlobalTransform 전파 확정 후).
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
        // 발 하위 본(양쪽) 월드 y 최소 — 발목(Foot)·발볼(PT)에 **말단 리프 본
        // (_end, 발끝·뒤꿈치 끝단)** 까지 잡아 밑창 표면을 근사한다(6차 보고
        // '뒤꿈치 재부유' — 뼈 원점 앵커는 축소 후 밑창까지 남은 두께만큼
        // 발을 들게 한다. _end는 지오메트리 끝단까지 뻗은 리프라 최저점 근사).
        // ⚠️ qchar(CharacterArmature) 이름만 매칭 — 소문자 UE5명(foot_l 등)은
        // 헤어 GLB가 싣고 온 ubase 리그 뼈가 주민 계층에 섞여 발밑 2.7m 아래
        // 에 스폰돼 측정을 오염시킨다(2026-10-23 footdbg 실측). 주민 스폰은
        // qchar 단일 리그다.
        let mut foot_y: Option<f32> = None;
        for d in children_q.iter_descendants(root) {
            let Ok(name) = names.get(d) else { continue };
            let n = name.as_str();
            if matches!(
                n,
                "Foot.L"
                    | "Foot.R"
                    | "PT.L"
                    | "PT.R"
                    | "Foot.L_end"
                    | "Foot.R_end"
                    | "PT.L_end"
                    | "PT.R_end"
            ) {
                if let Ok(gt) = feet.get(d) {
                    foot_y = Some(foot_y.map_or(gt.translation().y, |m: f32| {
                        m.min(gt.translation().y)
                    }));
                }
            }
        }
        let drop = foot_y.map_or(0.0, |fy| (fy - tr.translation.y).max(0.0));
        commands
            .entity(root)
            .insert(FootLift(drop + SOLE_CLEAR))
            .remove::<SettleProbe>();
    }
}

/// 본 이름별 배율 — 이중 리그(Quaternius CharacterArmature · UE5 마네킹).
/// ⚠️ 체인 최상단 본만 매칭: 로컬 스케일은 자손 전체에 전파되므로 체인의
/// 모든 뼈에 곱하면 0.82³처럼 누적 왜곡된다(2026-10-18 실측).
///   - 머리: Head(양쪽 동일) · 몸통: Abdomen(q)/spine_01(u) — 다리는 형제라
///     영향 없음 · 다리: UpperLeg.X(q)/thigh_x(u) — 종아리는 자손이라 함께 단축 ·
///     팔: UpperArm.X(q)/upperarm_x(u) — 손은 자손이라 함께 단축(2026-10-19).
///   - 발: qchar 리그는 Foot.X가 **Root 직속**(다리 체인과 형제)이라 다리 축소가
///     전파되지 않는다 — 명시 매칭 없이 두면 원본 크기 발이 짧아진 다리에 붙어
///     스키처럼 앞으로 뻗는다(2026-10-22 직접 플레이 QA '패들 발').
///     **PT.X(발가락)도 Root 직속**(2026-10-23 5차 보고 '발 꺽임·늘어남' 실측
///     정정 — 예전 주석의 'PT는 Foot 자손'은 ubase 리그 기준 오류): 발 메시가
///     Foot+PT 양쪽에 스키닝돼 PT를 빼면 앞꿈치만 원본 크기로 남아 메시가
///     늘어나고(보행) 비틀린다(정지). UE5(foot_l)는 calf_l 자손이라 전파됨 —
///     toe_l을 여기 넣으면 이중 축소되므로 qchar 이름만 매칭한다.
///   - 발(2026-10-23 4차 보고 '고운 꺾임·테린 한쪽 소실'): ⚠️ **본 스케일은
///     반드시 균등** — Foot 본은 원본 리그에서 ~104° 회전돼 로컬 축이 세계축과
///     어긋나, 비균등(0.45/0.38/0.30)을 걸면 발 메시가 shear로 비틀린다
///     (포즈·각도별 꺾임·소실). 균등 강축소 0.34로 뭉툭한 부츠를 낸다.
pub fn bone_scale(name: &str) -> Option<Vec3> {
    match name {
        "Head" => Some(Vec3::splat(HEAD_SCALE)),
        "Abdomen" | "spine_01" => Some(Vec3::splat(SPINE_SCALE)),
        "UpperLeg.L" | "UpperLeg.R" | "thigh_l" | "thigh_r" => Some(Vec3::splat(LEG_SCALE)),
        "Foot.L" | "Foot.R" | "PT.L" | "PT.R" => Some(Vec3::splat(FOOT_SCALE)),
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
        commands.entity(entity).insert((ChibiDone, SettleProbe(2)));
    }
}
