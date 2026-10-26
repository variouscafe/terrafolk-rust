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
    /// 양팔 내지르기·망치질 계열 원샷성 작업 모션(사냥·건설·채굴).
    Punch,
    Wave,
}

impl AnimKind {
    /// 이 상태의 클립이 Root 직속 발 본(Foot/PT)의 translation을 직접
    /// 구동하는가 — qchar 실측: Walk·Punch 계열은 저작 채널이 있고
    /// Idle/Interact/Wave는 없다(발은 bind 위치 고정). foot_lock이 저작값
    /// 사상 vs bind 복원 중 어느 쪽을 쓸지 정한다.
    pub fn drives_foot_bones(&self) -> bool {
        matches!(self, AnimKind::Walk | AnimKind::Punch)
    }
}

/// 캐릭터 시스템이 읽는 애님 상태 스냅샷 — 게임 애님 컴포넌트가 갱신 시
/// 함께 쓴다(클립 핸들은 게임 소유라 여기 없음).
#[derive(Component)]
pub struct AnimState(pub AnimKind);

/// 우선 포즈 홀드 — 만료 시각까지 지정 애님을 유지(게임이 연출로 부착·
/// 애니메이터가 활동 코드보다 우선해 소비). 예: 캐치볼 수신 측의 받을
/// 자세 — `ActiveAction` 없이 짝 행동에 참여하는 액터의 연출(행동
/// 상태머신은 지시 주인만 갖는다). 만료 잔류 컴포넌트는 소비처가 정리.
#[derive(Component)]
pub struct PoseHold {
    pub anim: AnimKind,
    /// 만료 시각(s — `Time::elapsed_secs` 기준).
    pub until: f32,
}
