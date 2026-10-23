//! 애님 상태 종류 — 리그·게임 무관 최소 집합.
//! 활동 코드→상태 매핑(채집·건설 등 계약 문자열)은 게임 측이 갖는다.

use bevy::prelude::*;

/// 애님 상태 — 그래프 클립 인덱스와 1:1(게임이 그래프를 구성).
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum AnimKind {
    #[default]
    Idle,
    Walk,
    Interact,
    Wave,
}

/// 캐릭터 시스템이 읽는 애님 상태 스냅샷 — 게임 애님 컴포넌트가 갱신 시
/// 함께 쓴다(클립 핸들은 게임 소유라 여기 없음).
#[derive(Component)]
pub struct AnimState(pub AnimKind);
