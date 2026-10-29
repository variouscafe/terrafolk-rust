//! 레퍼런스 농부 소년 룩 — 주민 1명(스폰 순 첫 봇)에게 레퍼런스 캐릭터
//! (오픈소스 레포 docs/reference-analysis.md — 캐릭터 디자인 바이블
//! 2026-10-28 판)를 완전 적용한다.
//!
//! 구성: KayKit Farmer 베이스(전 주민 — 팔레트는 텍스처에 내장) + **1명
//! 한정** 밀짚모자(테라코타 밴드)·큰 초록 눈·앞머리·멜빵 끈+버클·앞포켓
//! 소품(crate wear 절차적 파트 — 좌표는 리그 로컬이라 프로파일 자동 전파).
//! hide_face_prim·tint_for는 qchar 시대 훅(toonify 색 매칭) — KayKit
//! 재질 색엔 무매치라 무동작(함수 유지: toon.rs 호출부 계약).

use bevy::prelude::*;
use chibi_character::{WearPart, WearPrim, WearSlot, WornItems};

/// 레퍼런스 룩 대상 마커 — 스폰 순 첫 주민 1명(apply.rs).
#[derive(Component)]
pub struct ReferenceVillager;

/// 착용 + 얼굴 + 가슴 — 오픈소스 뷰어 reference.rs kaykit_items와 동일
/// 상수(단일 룩 계약 — 2026-10-29 정본 동기: 0.3.5 상수 + 모자·앞머리
/// 패싯). 좌표는 KayKit head/chest 본 로컬 bind 단위(실측
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
    // 가슴 — 멜빵 끈 2(사각 버클 포함)·앞포켓 패널·체커 패치·흙손·새싹·
    // 셔츠 버튼 2·카라(바이블 §5.1~5.2 — 포켓 소품 3종은 정체성 필수).
    // chest 본 로컬 bind 단위(chest 원점 y=0.973 · 몸통 전면 z≈+0.42).
    let chest = vec![
        // 카라 — 목 밑 웜 아이보리 양단(안쪽으로 살짝 벌어진 V 인상).
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
        .tint(Color::srgb_u8(233, 223, 192)),
        // 셔츠 버튼 — 가슴 중앙 세로 2개, 작고 얌전한 갈색(§5.1).
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
        .tint(Color::srgb_u8(122, 82, 48)),
        // 멜빵 끈 — 가슴을 세로로 내려가는 넓은 끈(어깨 위는 클립이므로
        // 가슴 세그먼트만 오버레이).
        WearPart::new(WearPrim::Box {
            w: 0.05,
            h: 0.30,
            d: 0.016,
        })
        .at(Vec3::new(-0.155, -0.09, 0.415))
        .tint(Color::srgb_u8(102, 135, 70)),
        WearPart::new(WearPrim::Box {
            w: 0.05,
            h: 0.30,
            d: 0.016,
        })
        .at(Vec3::new(0.155, -0.09, 0.415))
        .tint(Color::srgb_u8(102, 135, 70)),
        // 사각 버클 — 오치어, 중앙 인셋 느낌은 어두운 내부 박스.
        WearPart::new(WearPrim::Box {
            w: 0.085,
            h: 0.065,
            d: 0.022,
        })
        .at(Vec3::new(-0.155, -0.185, 0.425))
        .tint(Color::srgb_u8(181, 138, 67)),
        WearPart::new(WearPrim::Box {
            w: 0.085,
            h: 0.065,
            d: 0.022,
        })
        .at(Vec3::new(0.155, -0.185, 0.425))
        .tint(Color::srgb_u8(181, 138, 67)),
        // 포켓 상단 시임 — 어두운 그린 얇은 라인(포켓 입구 — 3차 디테일).
        WearPart::new(WearPrim::Box {
            w: 0.36,
            h: 0.016,
            d: 0.012,
        })
        .at(Vec3::new(0.0, -0.244, 0.428))
        .tint(Color::srgb_u8(85, 115, 55)),
        // 앞포켓 — 본체보다 살짝 어두운 그린 패널.
        WearPart::new(WearPrim::Box {
            w: 0.36,
            h: 0.20,
            d: 0.018,
        })
        .at(Vec3::new(0.0, -0.345, 0.425))
        .tint(Color::srgb_u8(90, 122, 60)),
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
        .tint(Color::srgb_u8(233, 181, 27)),
        // 흙손(삽) — 회색 날 + 갈색 손잡이.
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
        .tint(Color::srgb_u8(122, 82, 48)),
        // 새싹 — 가느다란 줄기 + 좌우 잎 2장.
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
    // 소매 커프 — 팔꿈치 아래 접힌 소매 단(§5.1 'wide rolled cuffs'). wrist
    // 본 로컬(계약: +Y=원위·팔 축)에서 −Y(몸쪽) 0.07 — 원기둥 축이 팔에 정렬.
    let cuff = vec![
        WearPart::new(WearPrim::Cyl { r: 0.105, h: 0.075 })
            .at(Vec3::new(0.0, -0.07, 0.0))
            .tint(Color::srgb_u8(220, 201, 152))
            .faceted(),
    ];
    // 다리 — 바지 롤드 커프(§5.2) + 장화 밑창(§5.4 '두꺼운 오치어오렌지
    // 밑창 밴드, 명확히 분리'). lowerleg 본 로컬(계약: +Y=원위=하향, X/Z
    // 수평 — 발목 +Y 0.149 bind 실측).
    let leg = vec![
        // 바지 커프 — 장화 상단 바로 위, 본체와 다른 톤(§5.2).
        WearPart::new(WearPrim::Cyl { r: 0.155, h: 0.09 })
            .at(Vec3::new(0.0, 0.10, 0.0))
            .tint(Color::srgb_u8(86, 116, 58))
            .faceted(),
        // 밑창 — 발밑 평평한 오치어 밴드(발가락 +Z 전방).
        WearPart::new(WearPrim::Box {
            w: 0.17,
            h: 0.05,
            d: 0.30,
        })
        .at(Vec3::new(0.0, 0.205, 0.045))
        .tint(Color::srgb_u8(185, 109, 37))
        .faceted(),
    ];
    WornItems(vec![
        (WearSlot::Hat, hat),
        (WearSlot::Face, face),
        (WearSlot::Chest, chest),
        (WearSlot::HandLeft, cuff.clone()),
        (WearSlot::HandRight, cuff),
        (WearSlot::LegLeft, leg.clone()),
        (WearSlot::LegRight, leg),
    ])
}

/// 숨길 원본 헤드 프림(선형색 매칭) — qchar 시대 훅. KayKit 재질(아틀라스
/// 텍스처, baseColorFactor 흰색)엔 무매치라 무동작(toon.rs 호출부 계약 유지).
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
