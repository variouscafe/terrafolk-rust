//! 주민 지면 스냅 — 주민 루트 y의 **단일 소유자**(2026-10-23 재설계).
//!
//! 재설계 계기('뒷꿈치 부유 + 고지에서 발이 지형에 박힘' 재보고):
//! ① `live_move`가 y를 **목적지 타일 중심 고도**(`height_at` 양자화값)로
//!    걷는 내내 고정 — 발밑 연속 표면고도와의 단차만큼 이동 중 발이 뜨고,
//!    고지 도착 시엔 타일 중심↔연속 표면 오차로 발이 지형에 박혔다.
//! ② y 작성이 fit(스폰)·settle(DropMark 즉시 하강)·live_move 세 곳에
//!    흩어져 서로를 무효화하는 패치가 반복됐다(2차 d9de5bf·4차 e9b3044).
//!
//! 설계: FootLift(치비 부양량, chibi.rs 측정)가 붙은 주민의 루트 y는
//! **매 프레임 이 시스템만** 쓴다 — 발밑 위치의 `surface_height`(지형
//! 메시와 공유하는 연속 표면장, data.rs)에서 물 상면 클램프 후 FootLift
//! 만큼 내린 값. 이동 시스템(live_move·wander)은 수평(x/z)·회전만
//! 담당하고, 본 시스템은 양쪽 **이후에 실행**돼 같은 프레임 갱신 위치를
//! 즉시 반영한다(최종 작성자). 측정 전(스폰 2프레임) 주민은 스폰
//! 클램프값을 유지하되, 스폰이 `Visibility::Hidden`(툰 스왑 게이트)이라
//! 눈에 띄지 않는다.

use bevy::prelude::*;

use super::chibi::FootLift;
use super::data::WorldData;
use super::villager::Villager;

/// 물 타일 위 주민 상면 클램프(§17.80 취수) — 서버가 물 타일에 세워도
/// 불투명 수면(-0.18) 아래로 몸이 잠기지 않게 발을 상면 위로 못 박는다
/// (apply.rs 스폰 클램프와 동일 기준 — 상수 단일화).
pub const WATER_STAND_Y: f32 = -0.14;

pub fn ground_snap(
    world: Res<WorldData>,
    mut q: Query<(&mut Transform, &FootLift), With<Villager>>,
) {
    for (mut tr, lift) in &mut q {
        let ground = world
            .surface_height(tr.translation.x, tr.translation.z)
            .max(WATER_STAND_Y);
        tr.translation.y = ground - lift.0;
    }
}

pub fn plugin(app: &mut App) {
    // live_move(라이브)·wander(데모) 이후 — 이동이 같은 프레임에 옮긴
    // 위치의 표면고도로 스냅한다(실행 전이면 한 프레임 뒤처짐·충돌 경합).
    app.add_systems(
        Update,
        ground_snap
            .after(crate::net::apply::live_move)
            .after(super::villager::wander),
    );
}
