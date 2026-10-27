//! 레퍼런스 파머 룩 — 주민 1명에게 레퍼런스 캐릭터(오픈소스 레포
//! docs/reference-analysis.md)를 완전 적용한다(2026-10-27 사용자 목표
//! "레퍼런스 캐릭터를 캐릭터 하나에 완전 적용" → 라이브 검증).
//!
//! 구성: CHIBI_REF 비율(전 주민) + **1명 한정** 밀짚모자·장갑·큰 눈·입
//! (crate wear 절차적 파트 — 좌표는 리그 로컬이라 프로파일 자동 전파) +
//! 데님/크림 틴트·원본 눈/입 프림 숨김(toonify 훅).

use bevy::prelude::*;
use chibi_character::{WearPart, WearPrim, WearSlot, WornItems};

/// 레퍼런스 룩 대상 마커 — 스폰 순 첫 주민 1명(apply.rs).
#[derive(Component)]
pub struct ReferenceVillager;

/// 착용 + 얼굴 — 오픈소스 뷰어 reference.rs와 동일 상수(단일 룩 계약).
pub fn items() -> WornItems {
    let hat = vec![
        // 브림 — 넓은 납작 원판(머리 폭 ~1.6배)
        WearPart::new(WearPrim::Cyl { r: 0.26, h: 0.018 })
            .at(Vec3::new(0.0, 0.255, 0.0))
            .tint(Color::srgb_u8(217, 164, 65)),
        // 왕관 밴드 — 어두운 짚 링
        WearPart::new(WearPrim::Cyl { r: 0.172, h: 0.05 })
            .at(Vec3::new(0.0, 0.272, 0.0))
            .tint(Color::srgb_u8(178, 133, 48)),
        // 왕관 — 납작 돔
        WearPart::new(WearPrim::Sph { r: 0.165 })
            .at(Vec3::new(0.0, 0.315, 0.0))
            .scaled(Vec3::new(1.0, 0.72, 1.0))
            .tint(Color::srgb_u8(232, 197, 111)),
    ];
    // 얼굴 — 큰 타원 눈 + 웃는 입(원본 Eye/Red 프림은 toonify가 숨김)
    let face = vec![
        WearPart::new(WearPrim::Sph { r: 0.045 })
            .at(Vec3::new(-0.058, 0.145, 0.132))
            .scaled(Vec3::new(0.72, 1.25, 0.45))
            .tint(Color::srgb_u8(43, 26, 18)),
        WearPart::new(WearPrim::Sph { r: 0.045 })
            .at(Vec3::new(0.058, 0.145, 0.132))
            .scaled(Vec3::new(0.72, 1.25, 0.45))
            .tint(Color::srgb_u8(43, 26, 18)),
        WearPart::new(WearPrim::Sph { r: 0.02 })
            .at(Vec3::new(0.0, 0.052, 0.148))
            .scaled(Vec3::new(1.5, 0.55, 0.35))
            .tint(Color::srgb_u8(196, 74, 62)),
    ];
    let glove_l = vec![
        WearPart::new(WearPrim::Sph { r: 0.055 })
            .at(Vec3::new(0.0, -0.035, 0.01))
            .scaled(Vec3::new(1.0, 0.85, 1.3))
            .tint(Color::srgb_u8(200, 164, 110)),
    ];
    let glove_r = glove_l.clone();
    WornItems(vec![
        (WearSlot::Hat, hat),
        (WearSlot::Face, face),
        (WearSlot::HandLeft, glove_l),
        (WearSlot::HandRight, glove_r),
    ])
}

/// 숨길 원본 헤드 프림(선형색 매칭) — Eye(점 눈)·Red(입/코).
pub fn hide_face_prim(src: LinearRgba) -> bool {
    let l = [src.red, src.green, src.blue];
    let close = |a: [f32; 3]| {
        (l[0] - a[0]).abs() < 0.004 && (l[1] - a[1]).abs() < 0.004 && (l[2] - a[2]).abs() < 0.004
    };
    close([0.024850, 0.014529, 0.009046]) || close([0.161519, 0.003910, 0.011038])
}

/// 레퍼런스 팔레트 틴트 — m-farmer 재질(선형색) → 데님 #6B84C4·크림
/// #F5F0DC(레퍼런스 분석 팔레트 실측값).
pub fn tint_for(src: LinearRgba) -> Option<Color> {
    let l = [src.red, src.green, src.blue];
    let close = |a: [f32; 3]| {
        (l[0] - a[0]).abs() < 0.004 && (l[1] - a[1]).abs() < 0.004 && (l[2] - a[2]).abs() < 0.004
    };
    // GLB 원본(linear): LightBlue → 데님 · Beige → 크림
    if close([0.026484, 0.039765, 0.050408]) {
        return Some(Color::srgb_u8(107, 132, 196));
    }
    if close([0.265961, 0.247575, 0.161245]) {
        return Some(Color::srgb_u8(245, 240, 220));
    }
    None
}
