//! 라이브 에이전트 데이터 — 주민의 게임 데이터 성분(명·상태·활동·이동 보간).
//! 2026-10-29 캐릭터 시스템 전면 철퇴(사용자 지시 — 코드·Kit·레퍼런스 전부):
//! 비주얼(모델·리그·애님·착용·루크)은 제거되고 UI(주민 목록·채팅·미니맵·
//! 선택)와 net 동기가 소비하는 데이터 컴포넌트만 남는다. 새 캐릭터 시스템
//! (해안 마을 레퍼런스 — CLAUDE.md 그래픽 지향점) 설계 후 이 모듈에 다시 붙는다.

use bevy::prelude::*;

/// 물 타일 위 주민 상면 클램프(§17.80 취수) — 서버가 물 타일에 세워도
/// 엔티티가 불투명 수면 아래로 잠기지 않게 하는 스폰 상한.
pub const WATER_STAND_Y: f32 = -0.14;

/// 라이브 주민 — 서버 프레임이 정본인 표시 데이터 + 이동 보간 상태.
/// 시각적 몸체는 없다(철퇴). `to`/`moving`은 net::apply::live_move가
/// 서버 목적지로 등속 보간하는 데 쓴다.
#[derive(Component)]
pub struct Villager {
    /// 이동 목적지(월드 좌표 — y는 스폰 시 표면 고정).
    pub to: Vec3,
    /// 이동 중 여부(서버 좌표와 로컬 보간 차로 갱신).
    pub moving: bool,
    /// 표시명 — UI 주민 목록·선택 패널.
    pub name: String,
    /// 상태 코드(walk·idle·chop… 레거시 status.* 규칙) — UI가 한글로 번역.
    pub status: String,
    /// 활동 코드(라이브: 서버 actType[gather·build…]) — UI 라벨의 원천.
    /// 비면 status로 폴백.
    pub act: String,
}
