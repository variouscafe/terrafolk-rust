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

/// 지브리톤 지붕 잠금 — 파란 지붕 재질을 테라코타로 재지정(2026-10-30).
/// 정본 레퍼런스 지붕 팔레트는 테라코타 단일(#C46C2B 계열 실측)인데
/// CottageHouses GLB에 파란 RoofTiles 변형(#4F7EA8 선형)이 섞여 있다.
/// GLB 재질은 비동기 삽입이라 매 프레임 스캔(멱등 — 재지정 뒤엔 시그니처가
/// 불일치해 자연 종료). 창문 짙은 파랑(루마 0.27)은 루마 하한(0.38)으로
/// 제외 — 지붕 파랑(루마 0.455)만 매치.
pub fn lock_roof_palette(mut mats: ResMut<Assets<StandardMaterial>>) {
    for (_, m) in mats.iter_mut() {
        m.base_color = lock_roof_color(m.base_color.to_linear()).into();
    }
}

/// 지붕 잠금 규칙(단색) — PBR 스캔 경로와 툰 스왑 경로(toon_props)가 같은
/// 규칙을 쓴다: 스왑이 먼저 재질을 소비해도 파란 지붕은 테라코타로.
pub fn lock_roof_color(c: LinearRgba) -> LinearRgba {
    let (r, g, b) = (c.red, c.green, c.blue);
    let luma = r * 0.3 + g * 0.59 + b * 0.11;
    if b > r + 0.06 && b > g + 0.03 && (0.38..0.70).contains(&luma) {
        // CottageHouses 테라코타 변형(선형 0.72, 0.36, 0.26)과 동일값 —
        // 마을 지붕 전체가 한 팔레트로 읽히게.
        LinearRgba {
            red: 0.72,
            green: 0.36,
            blue: 0.26,
            alpha: c.alpha,
        }
    } else {
        c
    }
}
