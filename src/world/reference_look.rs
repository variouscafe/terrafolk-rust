//! 레퍼런스 농부 소년 룩 — 주민 1명(스폰 순 첫 봇)에게 레퍼런스 캐릭터
//! (오픈소스 레포 docs/reference-analysis.md — 캐릭터 디자인 바이블
//! 2026-10-28 판)를 완전 적용한다.
//!
//! 구성: folk 파라메트릭 몸체 베이스(전 주민 — 팔레트는 파트 재질에 내장) + **1명
//! 한정** 밀짚모자(테라코타 밴드)·큰 초록 눈·앞머리·멜빵 끈+버클·앞포켓
//! 소품(crate wear 절차적 파트 — 좌표는 리그 로컬이라 프로파일 자동 전파).
//! hide_face_prim·tint_for는 qchar 시대 훅(toonify 색 매칭) — folk
//! 재질 색엔 무매치라 무동작(함수 유지: toon.rs 호출부 계약).

use bevy::prelude::*;
use chibi_character::{WearPart, WearPrim, WearSlot, WornItems};

/// 레퍼런스 룩 대상 마커 — 스폰 순 첫 주민 1명(apply.rs).
#[derive(Component)]
pub struct ReferenceVillager;

/// 착용 + 얼굴 + 가슴 — folk 몸체(2026-10-29 교체) 시대: 멜빵·비브·포켓은
/// build_folk.py 메시가 담당, 프림은 모자·얼굴·포켓 디테일만(뷰어 reference.rs 정본 동기)
/// 상수(단일 룩 계약 — 2026-10-29 정본 동기: 0.3.5 상수 + 모자·앞머리
/// 패싯). 좌표는 head/chest 본 로컬 bind 단위(실측
/// 2026-10-28: head 본 원점 y=1.241·헤드 y[−0.08,+0.95]·폭 ±0.55·전방
/// +Z. chest 본 원점 y=0.973·몸통 전면 z≈+0.37).
pub fn items() -> WornItems {
    // 팔레트(바이블 §6): 짚 #D9AD52 · 챙 #C79A45 · 밴드 테라코타 #C96B2D ·
    // 홍채 올리브그린 #5E8E37 · 눈썹/입 짙은 브라운 · 주근깨 코랄브라운.
    // 모자 전체 z −0.06 — 1차 측면 캡처에서 브림 앞:뒤 돌출 2:1 쏠림 상쇄.
    const HAT_Z: f32 = -0.06;
    let hat = vec![
        // 브림 — 넓은 납작 원판(0.3.2 원복값 — 사용자 지시 2026-10-29).
        WearPart::new(WearPrim::Cyl { r: 0.98, h: 0.05 })
            .at(Vec3::new(0.0, 0.86, HAT_Z))
            .tint(Color::srgb_u8(199, 154, 69))
            .faceted(),
        // 밴드 — 테라코타 링(위아래 경계 식별 가능해야 — 바이블 §4).
        WearPart::new(WearPrim::Cyl { r: 0.335, h: 0.15 })
            .at(Vec3::new(0.0, 0.90, HAT_Z))
            .tint(Color::srgb_u8(201, 107, 45))
            .faceted(),
        // 왕관 — 낮은 둥근 돔.
        WearPart::new(WearPrim::Sph { r: 0.30 })
            .at(Vec3::new(0.0, 0.97, HAT_Z))
            .scaled(Vec3::new(1.0, 0.78, 1.0))
            .tint(Color::srgb_u8(217, 173, 82))
            .faceted(),
        // 밴드 상·하 트림 — 어두운 테라코타 얇은 링(§4 '위아래 경계 식별
        // 가능해야' — 3차 디테일 2026-10-29).
        WearPart::new(WearPrim::Cyl { r: 0.34, h: 0.022 })
            .at(Vec3::new(0.0, 0.972, HAT_Z))
            .tint(Color::srgb_u8(154, 79, 32))
            .faceted(),
        WearPart::new(WearPrim::Cyl { r: 0.34, h: 0.022 })
            .at(Vec3::new(0.0, 0.828, HAT_Z))
            .tint(Color::srgb_u8(154, 79, 32))
            .faceted(),
        // 브림 이면 — 챙 밑 어두운 올리브 립(§4 '밑면은 어두운 올리브브라운'
        // — 정면에서 밑면 그림자 라인으로 읽힌다).
        WearPart::new(WearPrim::Cyl { r: 0.985, h: 0.025 })
            .at(Vec3::new(0.0, 0.832, HAT_Z))
            .tint(Color::srgb_u8(110, 96, 58))
            .faceted(),
    ];
    let face = vec![
        // 눈 — 크고 둥근 세로 타원 홍채(바이블 §3.2 최강 정체성). 크고 둥근 세로 타원(0.3.2 원복값).
        WearPart::new(WearPrim::Sph { r: 0.105 })
            .at(Vec3::new(-0.205, 0.44, 0.47))
            .scaled(Vec3::new(0.90, 1.35, 0.5))
            .tint(Color::srgb_u8(94, 142, 55)),
        WearPart::new(WearPrim::Sph { r: 0.105 })
            .at(Vec3::new(0.205, 0.44, 0.47))
            .scaled(Vec3::new(0.90, 1.35, 0.5))
            .tint(Color::srgb_u8(94, 142, 55)),
        // 하이라이트 — 작은 흰 점(양안 대칭 — 1차 비대칭 지적 반영).
        WearPart::new(WearPrim::Sph { r: 0.028 })
            .at(Vec3::new(-0.20, 0.50, 0.515))
            .tint(Color::srgb_u8(250, 250, 244)),
        WearPart::new(WearPrim::Sph { r: 0.028 })
            .at(Vec3::new(0.21, 0.50, 0.515))
            .tint(Color::srgb_u8(250, 250, 244)),
        // 눈썹 — 굵고 짧은 다크브라운 아치(확대 눈에 맞춰 폭 1.7).
        WearPart::new(WearPrim::Sph { r: 0.055 })
            .at(Vec3::new(-0.205, 0.62, 0.44))
            .scaled(Vec3::new(1.5, 0.55, 0.6))
            .tint(Color::srgb_u8(91, 46, 28)),
        WearPart::new(WearPrim::Sph { r: 0.055 })
            .at(Vec3::new(0.205, 0.62, 0.44))
            .scaled(Vec3::new(1.5, 0.55, 0.6))
            .tint(Color::srgb_u8(91, 46, 28)),
        // 주근깨 — 양 뺨 3점씩, 흐리고 sparse(바이블 §3.1: 짙거나 많으면
        // 안 됨 — negative 프롬프트 'dense freckles' 회피).
        WearPart::new(WearPrim::Sph { r: 0.014 })
            .at(Vec3::new(-0.30, 0.30, 0.46))
            .tint(Color::srgb_u8(201, 138, 94)),
        WearPart::new(WearPrim::Sph { r: 0.014 })
            .at(Vec3::new(-0.35, 0.26, 0.44))
            .tint(Color::srgb_u8(201, 138, 94)),
        WearPart::new(WearPrim::Sph { r: 0.014 })
            .at(Vec3::new(-0.26, 0.24, 0.45))
            .tint(Color::srgb_u8(201, 138, 94)),
        WearPart::new(WearPrim::Sph { r: 0.014 })
            .at(Vec3::new(0.30, 0.30, 0.46))
            .tint(Color::srgb_u8(201, 138, 94)),
        WearPart::new(WearPrim::Sph { r: 0.014 })
            .at(Vec3::new(0.35, 0.26, 0.44))
            .tint(Color::srgb_u8(201, 138, 94)),
        WearPart::new(WearPrim::Sph { r: 0.014 })
            .at(Vec3::new(0.26, 0.24, 0.45))
            .tint(Color::srgb_u8(201, 138, 94)),
        // 입 — 작고 얇은 갈색 미소 곡선(다문 입).
        WearPart::new(WearPrim::Sph { r: 0.05 })
            .at(Vec3::new(0.0, 0.17, 0.50))
            .scaled(Vec3::new(1.5, 0.42, 0.4))
            .tint(Color::srgb_u8(122, 74, 58)),
        // 코 — 작고 둥근 버튼(§3.1 '웜 오렌지살구 톤, 짧고 둥근 삼각형'
        // — 동글동글한 평면 얼굴에 입체 디테일).
        WearPart::new(WearPrim::Sph { r: 0.055 })
            .at(Vec3::new(0.0, 0.29, 0.505))
            .scaled(Vec3::new(0.85, 0.75, 0.7))
            .tint(Color::srgb_u8(215, 120, 69)),
        // 귀 2 — 머리 폭(±0.55)에서 살짝 돌출(§3.1 '양옆으로 약간 돌출,
        // 둥금'). 속귀는 더 깊은 오렌지 점.
        WearPart::new(WearPrim::Sph { r: 0.105 })
            .at(Vec3::new(-0.54, 0.36, 0.03))
            .scaled(Vec3::new(0.45, 1.0, 0.85))
            .tint(Color::srgb_u8(230, 169, 114)),
        WearPart::new(WearPrim::Sph { r: 0.105 })
            .at(Vec3::new(0.54, 0.36, 0.03))
            .scaled(Vec3::new(0.45, 1.0, 0.85))
            .tint(Color::srgb_u8(230, 169, 114)),
        WearPart::new(WearPrim::Sph { r: 0.038 })
            .at(Vec3::new(-0.565, 0.36, 0.055))
            .tint(Color::srgb_u8(215, 120, 69)),
        WearPart::new(WearPrim::Sph { r: 0.038 })
            .at(Vec3::new(0.565, 0.36, 0.055))
            .tint(Color::srgb_u8(215, 120, 69)),
        // 앞머리 — 브림 아래 굵고 각진 뭉치 3개(바이블 §3.3: 중앙 길게
        // 아래로, 양옆 크게 갈라짐. 사실적 가닥 금지 — 조각 뭉치).
        WearPart::new(WearPrim::Sph { r: 0.10 })
            .at(Vec3::new(0.0, 0.70, 0.42))
            .scaled(Vec3::new(0.72, 1.5, 0.5))
            .tint(Color::srgb_u8(107, 50, 30))
            .faceted(),
        WearPart::new(WearPrim::Sph { r: 0.09 })
            .at(Vec3::new(-0.245, 0.73, 0.38))
            .scaled(Vec3::new(0.95, 1.15, 0.5))
            .tint(Color::srgb_u8(107, 50, 30))
            .faceted(),
        WearPart::new(WearPrim::Sph { r: 0.09 })
            .at(Vec3::new(0.245, 0.73, 0.38))
            .scaled(Vec3::new(0.95, 1.15, 0.5))
            .tint(Color::srgb_u8(107, 50, 30))
            .faceted(),
    ];
    // 가슴 — 카라·셔츠 버튼·체커 패치·흙손·새싹(바이블 §5.1~5.2 — 포켓
    // 소품 3종은 정체성 필수). 멜빵·비브·버클은 folk 몸체 메시가 이미
    // 담당(build_folk bib 파트) — 프림 중복 z-fight 방지로 싣지 않는다
    // (2026-10-29 정리, 뷰어 folk_farmer_items와 동일 계약).
    // chest 본 로컬 bind 단위(chest 원점 y=0.973 · 몸통 전면 z≈+0.42).
    let chest = vec![        // 카라 — 목 밑 웜 아이보리 양단(안쪽으로 살짝 벌어진 V 인상).
        WearPart::new(WearPrim::Box {
            w: 0.10,
            h: 0.035,
            d: 0.014,
        })
        .at(Vec3::new(-0.05, 0.13, 0.41))
        .tint(Color::srgb_u8(233, 223, 192)),
        WearPart::new(WearPrim::Box {
            w: 0.10,
            h: 0.035,
            d: 0.014,
        })
        .at(Vec3::new(0.05, 0.13, 0.41))
        .tint(Color::srgb_u8(233, 223, 192)),        // 셔츠 버튼 — 가슴 중앙 세로 2개, 작고 얌전한 갈색(§5.1).
        WearPart::new(WearPrim::Box {
            w: 0.028,
            h: 0.028,
            d: 0.012,
        })
        .at(Vec3::new(0.0, 0.02, 0.42))
        .tint(Color::srgb_u8(122, 82, 48)),
        WearPart::new(WearPrim::Box {
            w: 0.028,
            h: 0.028,
            d: 0.012,
        })
        .at(Vec3::new(0.0, -0.07, 0.42))
        .tint(Color::srgb_u8(122, 82, 48)),        // 가슴 세그먼트만 오버레이).

        // 체커 패치 — 오렌지 바탕 + 노랑 스트라이프 2(체커 인상).
        WearPart::new(WearPrim::Box {
            w: 0.105,
            h: 0.085,
            d: 0.014,
        })
        .at(Vec3::new(-0.065, -0.335, 0.44))
        .tint(Color::srgb_u8(232, 163, 61)),
        WearPart::new(WearPrim::Box {
            w: 0.105,
            h: 0.016,
            d: 0.016,
        })
        .at(Vec3::new(-0.065, -0.325, 0.442))
        .tint(Color::srgb_u8(233, 181, 27)),
        WearPart::new(WearPrim::Box {
            w: 0.105,
            h: 0.016,
            d: 0.016,
        })
        .at(Vec3::new(-0.065, -0.352, 0.442))
        .tint(Color::srgb_u8(233, 181, 27)),        // 흙손(삽) — 회색 날 + 갈색 손잡이.
        WearPart::new(WearPrim::Box {
            w: 0.04,
            h: 0.095,
            d: 0.014,
        })
        .at(Vec3::new(0.105, -0.335, 0.44))
        .tint(Color::srgb_u8(168, 168, 164)),
        WearPart::new(WearPrim::Box {
            w: 0.018,
            h: 0.07,
            d: 0.018,
        })
        .at(Vec3::new(0.105, -0.255, 0.44))
        .tint(Color::srgb_u8(122, 82, 48)),        // 새싹 — 가느다란 줄기 + 좌우 잎 2장.
        WearPart::new(WearPrim::Sph { r: 0.011 })
            .at(Vec3::new(0.0, -0.295, 0.448))
            .scaled(Vec3::new(0.5, 2.4, 0.5))
            .tint(Color::srgb_u8(127, 179, 90)),
        WearPart::new(WearPrim::Sph { r: 0.030 })
            .at(Vec3::new(-0.032, -0.252, 0.448))
            .scaled(Vec3::new(1.5, 0.5, 0.6))
            .tint(Color::srgb_u8(156, 203, 92)),
        WearPart::new(WearPrim::Sph { r: 0.030 })
            .at(Vec3::new(0.032, -0.252, 0.448))
            .scaled(Vec3::new(1.5, 0.5, 0.6))
            .tint(Color::srgb_u8(156, 203, 92)),
    ];
    WornItems(vec![
        (WearSlot::Hat, hat),
        (WearSlot::Face, face),
        (WearSlot::Chest, chest),
    ])
}

/// 숨길 원본 헤드 프림(선형색 매칭) — qchar 시대 훅. folk 재질(플랫
/// baseColorFactor)엔 무매치라 무동작(toon.rs 호출부 계약 유지).
pub fn hide_face_prim(src: LinearRgba) -> bool {
    let l = [src.red, src.green, src.blue];
    let close = |a: [f32; 3]| {
        (l[0] - a[0]).abs() < 0.004 && (l[1] - a[1]).abs() < 0.004 && (l[2] - a[2]).abs() < 0.004
    };
    close([0.024850, 0.014529, 0.009046]) || close([0.161519, 0.003910, 0.011038])
}

/// 데님/크림 팔레트 틴트 — qchar m-farmer 재질 매칭 시대 훅(무동작 동일).
pub fn tint_for(src: LinearRgba) -> Option<Color> {
    let l = [src.red, src.green, src.blue];
    let close = |a: [f32; 3]| {
        (a[0] - l[0]).abs() < 0.004 && (a[1] - l[1]).abs() < 0.004 && (a[2] - l[2]).abs() < 0.004
    };
    if close([0.026484, 0.039765, 0.050408]) {
        return Some(Color::srgb_u8(107, 132, 196));
    }
    if close([0.265961, 0.247575, 0.161245]) {
        return Some(Color::srgb_u8(245, 240, 220));
    }
    None
}
