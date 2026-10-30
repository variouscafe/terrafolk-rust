//! 텍스처 밉 체인 생성(2026-10-30 10차, 10.1 수축) — bevy의 PNG/GLB 이미지
//! 로드는 mip_level_count=1로 올라온다(압축 포맷만 체인 내장). 10차 디테일
//! 억제(textureSampleLevel 밉 바이어스)가 걸리려면 체인이 필요하다.
//!
//! **범위는 최소한으로(10.1)** — 최초 버전의 전역 스캔은 나무 잎 알파
//! 컷아웃 가장자리를 뭉개 '가지 끝 붓터치' 아티팩트를 만들었다(사용자
//! 보고). 체인이 실제로 필요한 곳만:
//! 1. 지형 타일 5종(terrain_splat LOD 5) — 아래 정적 경로로 등록.
//! 2. 툰 스왑된 소품 재질(toon LOD 2) — toonify_props가 스왑 시점에 호출.
//! 그 외(나무 PBR·물·UI)는 레벨 0 클램프 종전 동작을 유지한다.
//! sRGB 공간 그대로의 박스 필터 — 미세 채도 상승은 페인티드 룩에 유리.

use bevy::asset::AssetId;
use bevy::image::Image;
use bevy::platform::collections::HashSet;
use bevy::prelude::*;
use bevy::render::render_resource::TextureFormat;

/// 지형 스플랫 타일 — terrain.rs가 로드하는 것과 동일 경로(단일 진실원은
/// terrain.rs의 load 호출; 여기 재등록은 밉 생성 의존 주입용 상수).
pub const TERRAIN_TILE_PATHS: [&str; 5] = [
    "textures/ground/grass.png",
    "textures/ground/forest.png",
    "textures/ground/sand.png",
    "textures/ground/rock.png",
    "textures/ground/dirt.png",
];

/// 조건에 맞는 이미지에 전체 밉 체인을 만든다(멱등).
/// 렌더 타깃(data 없음)·작은 텍스처·비지원 포맷은 건드리지 않는다.
/// 반환값 = 처리 완료(재시도 불필요).
pub fn ensure_mips(img: &mut Image) -> bool {
    let d = &img.texture_descriptor;
    if d.mip_level_count > 1 {
        return true; // 이미 체인 있음
    }
    let Some(data) = img.data.as_ref() else {
        return true; // 렌더 타깃류 — 대상 아님
    };
    let (w, h) = (d.size.width, d.size.height);
    if w < 4 || h < 4 {
        return true;
    }
    if !matches!(
        d.format,
        TextureFormat::Rgba8UnormSrgb | TextureFormat::Rgba8Unorm
    ) {
        return true; // 압축 포맷 등 — CPU 다운필터 미지원
    }
    if data.len() != (w * h * 4) as usize {
        return true; // 배열/아틀라스 등 특수 레이아웃 — 패스
    }

    let mut all = data.clone();
    let mut cur = data.clone();
    let (mut lw, mut lh) = (w, h);
    let mut levels = 1u32;
    while lw > 1 || lh > 1 {
        let nw = (lw / 2).max(1);
        let nh = (lh / 2).max(1);
        let next = mip_down(&cur, lw, lh, nw, nh);
        all.extend_from_slice(&next);
        cur = next;
        lw = nw;
        lh = nh;
        levels += 1;
    }
    img.data = Some(all);
    img.texture_descriptor.mip_level_count = levels;
    debug!("[mips] 체인 생성 {}x{} → {}레벨", w, h, levels);
    true
}

/// 2×2 박스 필터 다운샘플(에지 클램프) — RGBA8 타이트 패킹 가정.
fn mip_down(src: &[u8], w: u32, h: u32, nw: u32, nh: u32) -> Vec<u8> {
    let (w, h, nw, nh) = (w as usize, h as usize, nw as usize, nh as usize);
    let mut out = vec![0u8; nw * nh * 4];
    for y in 0..nh {
        for x in 0..nw {
            let x0 = (x * 2).min(w - 1);
            let x1 = (x0 + 1).min(w - 1);
            let y0 = (y * 2).min(h - 1);
            let y1 = (y0 + 1).min(h - 1);
            let mut acc = [0u32; 4];
            for (px, py) in [(x0, y0), (x1, y0), (x0, y1), (x1, y1)] {
                let o = (py * w + px) * 4;
                for c in 0..4 {
                    acc[c] += src[o + c] as u32;
                }
            }
            let o = (y * nw + x) * 4;
            for c in 0..4 {
                out[o + c] = (acc[c] / 4) as u8;
            }
        }
    }
    out
}

/// 지형 타일 밉 체인 — 등록 경로를 로드(멱등)해 체인이 확정될 때까지 재시도.
/// toon 소품은 toonify_props가 ensure_mips를 직접 호출(별도 경로).
fn chain_terrain_tiles(
    assets: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut done: Local<HashSet<AssetId<Image>>>,
) {
    for path in TERRAIN_TILE_PATHS {
        let h: Handle<Image> = assets.load(path);
        if done.contains(&h.id()) {
            continue;
        }
        if let Some(mut img) = images.get_mut(&h) {
            if ensure_mips(&mut img) {
                done.insert(h.id());
            }
        }
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, chain_terrain_tiles);
}
