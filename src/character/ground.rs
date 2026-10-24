//! 접지 — 액터 루트 y의 단일 소유자(2026-10-23 재설계 원칙 이식).
//! FootLift가 붙은 GroundActor의 루트 y는 매 프레임 이 시스템만 쓴다:
//! 발밑 표면고도(SurfaceField)에서 물 상면 클램프 후 FootLift만큼 내린 값.
//! 이동 시스템은 수평(x/z)·회전만 담당하고 본 시스템은 양쪽 이후에 실행
//! (순서 조율은 게임 측 world가 after로 지정).
//!
//! **물리낙하(2026-10-24)**: `Fall` 컴포넌트 액터는 지지면 단차가 임계를
//! 넘으면(절벽·순간이동) 공중 전환해 공간 중력(`SpaceGravity`)을 적분한다
//! (fall.rs — 이 게임 맵은 지구 9.81). 미보유 액터는 종래 하드 스냅.
//!
//! **발 기준 중력(2026-10-24 v2)**: FootAnchors(발 앵커 엔티티·바인드 기준)
//! 이 있으면 FootLift를 매 프레임 **현재 포즈의 최저 발 앵커 상대높이**로
//! 추적한다(하강 민감·상승 완만 비대칭 스무딩, 바인드 클램프). 걷기 클립의
//! 발 높이 진동 동안 몸이 바인드 고정값 위에 떠 있던 보행 부유의 수정 —
//! 루트 y를 바꿔도 상대높이는 불변이라 되먹임 없이 안정하다.

use bevy::prelude::*;

use super::fall::{integrate, Fall, SpaceGravity};
use super::proportion::{FootAnchors, FootLift};
use super::rig::SOLE_CLEAR;

/// 접지 대상 액터 마커 — 게임이 액터 스폰 시 부착(예: Villager).
#[derive(Component)]
pub struct GroundActor;

/// 표면장 — 지형 고도의 연속 함수(렌더 메시와 공유하는 장을 게임이 제공).
pub trait SurfaceField {
    /// (x,z) 위치의 지표면 고도(연속 보간).
    fn surface_height(&self, x: f32, z: f32) -> f32;
    /// 서 있을 수 있는 최저 y(수면 클램프 등) — 지형 메시보다 아래여도 됨.
    fn stand_min_y(&self) -> f32;
}

/// FootLift 추적 게인(1/s) — 하강(착지 압축)은 민감하게, 상승(양발 이륙·
/// 스트라이드 진동)은 완만하게. 몸이 발에 매달리는 감각 = 하강 우선.
const LIFT_TRACK_DOWN_HZ: f32 = 30.0;
const LIFT_TRACK_UP_HZ: f32 = 6.0;
/// 바인드 FootLift 대비 허용 편차(하한=앉기·착지, 상한=부유 방지).
const LIFT_MIN_DELTA: f32 = -0.10;
const LIFT_MAX_DELTA: f32 = 0.015;

/// 액터 루트 y 단일 소유 — `y = max(surface, stand_min) − FootLift`.
/// FootAnchors 보유 액터는 FootLift를 현재 최저 발 앵커로 추적 갱신하고,
/// Fall 보유 액터는 절벽 단차에서 물리낙하로 전환한다(fall.rs 참조).
pub fn ground_snap<F: Resource + SurfaceField>(
    world: Res<F>,
    gravity: Res<SpaceGravity>,
    time: Res<Time>,
    mut q: Query<
        (
            &mut Transform,
            &mut FootLift,
            Option<&FootAnchors>,
            Option<&mut Fall>,
        ),
        With<GroundActor>,
    >,
    gts: Query<&GlobalTransform>,
) {
    let dt = time.delta_secs();
    for (mut tr, mut lift, anchors, fall) in &mut q {
        if let Some(anchors) = anchors {
            // 현재 포즈 최저 앵커의 루트 상대높이 — 루트 y 평행이동과 무원.
            let mut rel_min = f32::INFINITY;
            for e in &anchors.entities {
                if let Ok(gt) = gts.get(*e) {
                    rel_min = rel_min.min(gt.translation().y - tr.translation.y);
                }
            }
            if rel_min.is_finite() {
                let target = (rel_min + SOLE_CLEAR)
                    .clamp(anchors.bind_lift + LIFT_MIN_DELTA, anchors.bind_lift + LIFT_MAX_DELTA);
                let hz = if target < lift.0 {
                    LIFT_TRACK_DOWN_HZ
                } else {
                    LIFT_TRACK_UP_HZ
                };
                let k = 1.0 - (-time.delta_secs() * hz).exp();
                lift.0 += (target - lift.0) * k;
            }
        }
        let ground = world
            .surface_height(tr.translation.x, tr.translation.z)
            .max(world.stand_min_y());
        let support = ground - lift.0;
        if let Some(mut f) = fall {
            let ny = integrate(tr.translation.y, &mut f, support, gravity.0, dt);
            if let Some(v) = f.landed.filter(|v| *v > 1.0) {
                info!("[grav] 낙하 착지 v={v:.1}m/s (h≈{:.2}m)", v * v / (2.0 * gravity.0));
            }
            tr.translation.y = ny;
        } else {
            tr.translation.y = support;
        }
    }
}
