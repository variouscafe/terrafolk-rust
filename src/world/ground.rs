//! 주민 접지 — 구현은 character 프레임워크로 이동(2026-10-24 P1).
//! `crate::character::{CharacterPlugin, ground}`가 루트 y 단일 소유자
//! (surface_height − FootLift, 재설계 사연은 character/ground.rs 모듈 문서).
//! 이 파일은 게임 상수(물 상면 클램프 — §17.80 취수)의 기존 import 경로를
//! 보존한다(net/apply.rs 등).

/// 물 타일 위 주민 상면 클램프 — 서버가 물 타일에 세워도 불투명 수면
/// (-0.18) 아래로 몸이 잠기지 않게 발을 상면 위로 못 박는다.
pub const WATER_STAND_Y: f32 = -0.14;
