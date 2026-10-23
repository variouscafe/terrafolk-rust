//! 캐릭터 프레임워크 — 게임 무관 계층(2026-10-24 재설계, P1).
//!
//! 다른 Bevy 게임에 그대로 복사해 쓰는다(platform 계층 관례 동일).
//! 게임 측 제공물은 3가지뿐:
//! - 표면장 리소스 `F: SurfaceField`(지형 고도 연속 함수)
//! - 액터 마커 `GroundActor`·`ChibiTarget`·`AnimState`(액터 스폰 시 부착)
//! - `CharacterPlugin::<F>` 등록 + GroundSnapSet 순서 조율
//!
//! 하위 시스템: anim(상태 종류)·rig(본 이름표·바인드 상수)·
//! proportion(치비화·부양량 측정)·ground(루트 y 단일 소유)·
//! foot_lock(비보행 발바닥 수평). 근거·이행 계획:
//! docs/character-module-redesign.md.

use bevy::prelude::*;

pub mod anim;
pub mod foot_lock;
pub mod ground;
pub mod proportion;
pub mod rig;
pub mod tool;

pub use anim::{AnimKind, AnimState};
pub use ground::{GroundActor, SurfaceField};
pub use proportion::{ChibiDone, ChibiTarget, FootLift, SettleProbe};

/// 접지 스냅 시스템 셋 — 게임 이동 시스템(live_move·wander 등) **이후**에
/// 실행되도록 게임 측이 configure_sets로 순서를 건다(같은 프레임 갱신
/// 위치 즉시 반영 — 한 프레임 뒤처짐·충돌 경합 방지).
#[derive(SystemSet, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroundSnapSet;

/// 캐릭터 접지·비율·포즈 시스템 꾸러미 — 표면장 F로 매개화.
pub struct CharacterPlugin<F: Resource + SurfaceField> {
    _marker: std::marker::PhantomData<fn(F)>,
}

impl<F: Resource + SurfaceField> Default for CharacterPlugin<F> {
    fn default() -> Self {
        Self {
            _marker: std::marker::PhantomData,
        }
    }
}

impl<F: Resource + SurfaceField> Plugin for CharacterPlugin<F> {
    fn build(&self, app: &mut App) {
        app.add_systems(
            Update,
            (proportion::chibiify, proportion::chibi_settle).chain(),
        );
        app.add_systems(Update, ground::ground_snap::<F>.in_set(GroundSnapSet));
        app.add_systems(Update, foot_lock::foot_lock);
        app.add_systems(Update, tool::tool_sync);
    }
}
