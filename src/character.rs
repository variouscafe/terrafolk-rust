//! 캐릭터 프레임워크 셤(shim) — 실체는 오픈소스 레포
//! `../3d-chibi-character`(crates/chibi-character)로 이관(2026-10-27).
//!
//! 기존 `crate::character::*` 임포트 경로를 그대로 보존한다(glob 재수출은
//! 하위 pub 모듈 rig·action·tool 등을 포함). 프레임워크의 게임 무관
//! 순수성 게이트는 이제 그 레포의 tools/check-purity.sh가 담당하고,
//! 본 레포의 check-layers.sh는 이 셤이 상위 계층을 참조하지 않는지만 검사.
pub use chibi_character::*;
