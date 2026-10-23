//! 접지 — 액터 루트 y의 단일 소유자(2026-10-23 재설계 원칙 이식).
//! FootLift가 붙은 GroundActor의 루트 y는 매 프레임 이 시스템만 쓴다:
//! 발밑 표면고도(SurfaceField)에서 물 상면 클램프 후 FootLift만큼 내린 값.
//! 이동 시스템은 수평(x/z)·회전만 담당하고 본 시스템은 양쪽 이후에 실행
//! (순서 조율은 게임 측 world가 after로 지정).

use bevy::prelude::*;

use super::proportion::FootLift;

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

/// 액터 루트 y 단일 소유 — `y = max(surface, stand_min) − FootLift`.
pub fn ground_snap<F: Resource + SurfaceField>(
    world: Res<F>,
    mut q: Query<(&mut Transform, &FootLift), With<GroundActor>>,
) {
    for (mut tr, lift) in &mut q {
        let ground = world
            .surface_height(tr.translation.x, tr.translation.z)
            .max(world.stand_min_y());
        tr.translation.y = ground - lift.0;
    }
}
