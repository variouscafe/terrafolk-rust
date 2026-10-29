//! WebGL2 재질 가드 — StandardMaterial의 AlphaMode::Blend 금지.
//! (범용 렌더 정책·에러 처리는 platform/gpu — 여기는 이 게임 재질의
//! WebGL2 특수 대응만. 아래 상세는 webgl2-fragment-uniform-limit 메모와 정합.)
//!
//! 이 스택(bevy 0.19 PBR)에서 alpha_blend 메시 파이프라인은 프래그먼트
//! 유니폼 버퍼 13개를 요구하는데 WebGL2 명세 한도가 12개라 생성 즉시
//! 검증 실패 → 렌더 에러 정책(기본 종료)이 관전 화면을 죽인다. 잎·꽃
//! 텍스처는 0/1 알파라 컷아웃(Mask)과 위상이 같으므로 전 표준 재질을 강제
//! 전환해 alpha_blend 파이프라인이 아예 생성되지 않게 한다. 커스텀 재질
//! (물·바람)은 가벼운 자체 레이아웃이라 한도 안 — Blend 유지.

use bevy::prelude::*;

/// 매 프레임 전 재질 스캔(멱등·저비용) — GLB 재질이 비동기 삽입되는 어떤
/// 시점에도 추출(파이프라인 특수화) 전에 한 프레임 안에 가드된다.
pub fn guard_blend_materials(mut mats: ResMut<Assets<StandardMaterial>>) {
    for (_, m) in mats.iter_mut() {
        if m.alpha_mode == AlphaMode::Blend {
            m.alpha_mode = AlphaMode::Mask(0.5);
        }
    }
}
