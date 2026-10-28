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
/// 상수(단일 룩 계약). 좌표는 KayKit head/chest 본 로컬 bind 단위(실측
/// 2026-10-28: head 본 원점 y=1.241·헤드 y[−0.08,+0.95]·폭 ±0.55·전방
/// +Z. chest 본 원점 y=0.973·몸통 전면 z≈+0.37).
pub fn items() -> WornItems {
    let hat = vec![
        // 브림 — 넓은 납작 원판(머리 반폭 0.55 대비 ~1.5배 = 바이블 1.35~1.55×).
        WearPart::new(WearPrim::Cyl { r: 0.85, h: 0.05 })
            .at(Vec3::new(0.0, 0.86, 0.0))
            .tint(Color::srgb_u8(199, 154, 69)),
        // 밴드 — 테라코타 링(위아래 경계 식별 가능 — 바이블 §4).
        WearPart::new(WearPrim::Cyl { r: 0.335, h: 0.15 })
            .at(Vec3::new(0.0, 0.90, 0.0))
            .tint(Color::srgb_u8(201, 107, 45)),
        // 왕관 — 낮은 둥근 돔.
        WearPart::new(WearPrim::Sph { r: 0.30 })
            .at(Vec3::new(0.0, 0.97, 0.0))
            .scaled(Vec3::new(1.0, 0.78, 1.0))
            .tint(Color::srgb_u8(217, 173, 82)),
    ];
    let face = vec![
        // 눈 — 크고 둥근 세로 타원 홍채(올리브그린 #5E8E37 — 바이블 §3.2
        // 최강 정체성). 원본 텍스처 눈 위 오버레이.
        WearPart::new(WearPrim::Sph { r: 0.105 })
            .at(Vec3::new(-0.205, 0.40, 0.47))
            .scaled(Vec3::new(0.82, 1.30, 0.5))
            .tint(Color::srgb_u8(94, 142, 55)),
        WearPart::new(WearPrim::Sph { r: 0.105 })
            .at(Vec3::new(0.205, 0.40, 0.47))
            .scaled(Vec3::new(0.82, 1.30, 0.5))
            .tint(Color::srgb_u8(94, 142, 55)),
        // 하이라이트 — 작은 흰 점(좌상단).
        WearPart::new(WearPrim::Sph { r: 0.028 })
            .at(Vec3::new(-0.185, 0.455, 0.515))
            .tint(Color::srgb_u8(250, 250, 244)),
        WearPart::new(WearPrim::Sph { r: 0.028 })
            .at(Vec3::new(0.225, 0.455, 0.515))
            .tint(Color::srgb_u8(250, 250, 244)),
        // 눈썹 — 굵고 짧은 다크브라운 아치.
        WearPart::new(WearPrim::Sph { r: 0.055 })
            .at(Vec3::new(-0.205, 0.585, 0.44))
            .scaled(Vec3::new(1.5, 0.55, 0.6))
            .tint(Color::srgb_u8(91, 46, 28)),
        WearPart::new(WearPrim::Sph { r: 0.055 })
            .at(Vec3::new(0.205, 0.585, 0.44))
            .scaled(Vec3::new(1.5, 0.55, 0.6))
            .tint(Color::srgb_u8(91, 46, 28)),
        // 입 — 작고 얇은 갈색 미소 곡선(다문 입).
        WearPart::new(WearPrim::Sph { r: 0.05 })
            .at(Vec3::new(0.0, 0.16, 0.50))
            .scaled(Vec3::new(1.5, 0.42, 0.4))
            .tint(Color::srgb_u8(122, 74, 58)),
        // 앞머리 — 브림 아래 굵고 각진 뭉치 3개(바이블 §3.3).
        WearPart::new(WearPrim::Sph { r: 0.10 })
            .at(Vec3::new(0.0, 0.70, 0.42))
            .scaled(Vec3::new(0.72, 1.5, 0.5))
            .tint(Color::srgb_u8(107, 50, 30)),
        WearPart::new(WearPrim::Sph { r: 0.09 })
            .at(Vec3::new(-0.245, 0.73, 0.38))
            .scaled(Vec3::new(0.95, 1.15, 0.5))
            .tint(Color::srgb_u8(107, 50, 30)),
        WearPart::new(WearPrim::Sph { r: 0.09 })
            .at(Vec3::new(0.245, 0.73, 0.38))
            .scaled(Vec3::new(0.95, 1.15, 0.5))
            .tint(Color::srgb_u8(107, 50, 30)),
    ];
    // 가슴 — 멜빵 끈+사각 버클·앞포켓·체커 패치·흙손·새싹(바이블 §5.2 —
    // 포켓 소품 3종은 정체성 필수: negative 프롬프트가 누락을 명시).
    let chest = vec![
        WearPart::new(WearPrim::Box { w: 0.05, h: 0.30, d: 0.016 })
            .at(Vec3::new(-0.155, -0.09, 0.415))
            .tint(Color::srgb_u8(102, 135, 70)),
        WearPart::new(WearPrim::Box { w: 0.05, h: 0.30, d: 0.016 })
            .at(Vec3::new(0.155, -0.09, 0.415))
            .tint(Color::srgb_u8(102, 135, 70)),
        WearPart::new(WearPrim::Box { w: 0.085, h: 0.065, d: 0.022 })
            .at(Vec3::new(-0.155, -0.185, 0.425))
            .tint(Color::srgb_u8(181, 138, 67)),
        WearPart::new(WearPrim::Box { w: 0.085, h: 0.065, d: 0.022 })
            .at(Vec3::new(0.155, -0.185, 0.425))
            .tint(Color::srgb_u8(181, 138, 67)),
        WearPart::new(WearPrim::Box { w: 0.36, h: 0.20, d: 0.018 })
            .at(Vec3::new(0.0, -0.345, 0.425))
            .tint(Color::srgb_u8(90, 122, 60)),
        WearPart::new(WearPrim::Box { w: 0.105, h: 0.085, d: 0.014 })
            .at(Vec3::new(-0.065, -0.335, 0.44))
            .tint(Color::srgb_u8(232, 163, 61)),
        WearPart::new(WearPrim::Box { w: 0.105, h: 0.016, d: 0.016 })
            .at(Vec3::new(-0.065, -0.325, 0.442))
            .tint(Color::srgb_u8(233, 181, 27)),
        WearPart::new(WearPrim::Box { w: 0.105, h: 0.016, d: 0.016 })
            .at(Vec3::new(-0.065, -0.352, 0.442))
            .tint(Color::srgb_u8(233, 181, 27)),
        WearPart::new(WearPrim::Box { w: 0.04, h: 0.095, d: 0.014 })
            .at(Vec3::new(0.105, -0.335, 0.44))
            .tint(Color::srgb_u8(168, 168, 164)),
        WearPart::new(WearPrim::Box { w: 0.018, h: 0.07, d: 0.018 })
            .at(Vec3::new(0.105, -0.255, 0.44))
            .tint(Color::srgb_u8(122, 82, 48)),
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
    WornItems(vec![(WearSlot::Hat, hat), (WearSlot::Face, face), (WearSlot::Chest, chest)])
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
