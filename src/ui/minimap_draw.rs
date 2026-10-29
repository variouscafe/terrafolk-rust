//! 미니맵 픽셀 드로잉 — 팔레트·지형 베이크 시그니처·라이브/데모 레이어.
//! minimap.rs(플러그인·노드·업데이트)에서 픽셀 조작 계열만 분리(500줄 규칙).

use std::collections::HashMap;

use crate::net::LiveFrame;
use crate::world::data::{PropKind, WorldData};

pub type Rgba = [u8; 4];
pub const fn rgb(hex: [u8; 3]) -> Rgba {
    [hex[0], hex[1], hex[2], 255]
}
/// GID 1..6 → 지형색(구 웹 TERRAIN_COLORS와 동일 팔레트).
pub const TERRAIN: [Rgba; 6] = [
    rgb([0x63, 0xc5, 0xcf]), // water
    rgb([0xea, 0xa5, 0x6c]), // sand
    rgb([0x8d, 0xc4, 0x35]), // grass
    rgb([0x4f, 0x94, 0x48]), // forest
    rgb([0x9a, 0x94, 0x8a]), // rock
    rgb([0xc8, 0xa0, 0x6a]), // dirt
];
pub const TREE: Rgba = rgb([0x2f, 0x5d, 0x2a]);
pub const TREE_DEAD: Rgba = rgb([0x6b, 0x5a, 0x48]);
pub const GOLD: Rgba = rgb([0xf0, 0xc0, 0x2e]);
pub const PLANT: Rgba = rgb([0xb8, 0xe9, 0x86]);
pub const HUT_DONE: Rgba = rgb([0xb5, 0x71, 0x3f]);
pub const HUT_WIP: Rgba = rgb([0xff, 0xd2, 0x8a]);
pub const CAMP_LIT: Rgba = rgb([0xff, 0x9f, 0x43]);
pub const CAMP_OUT: Rgba = rgb([0x6b, 0x5a, 0x48]);
pub const GRAVE: Rgba = rgb([0xe8, 0xe4, 0xda]);
pub const RELIC: Rgba = rgb([0xb4, 0x8e, 0xe0]);
pub const STASH: Rgba = rgb([0xc9, 0xa3, 0x6a]);
pub const VILLAGER: Rgba = rgb([0xff, 0x6b, 0x6b]);
pub const SELECTED: Rgba = rgb([0xff, 0xff, 0xff]);
pub const VIEW_DARK: Rgba = [20, 24, 16, 165];
pub const VIEW_GOLD: Rgba = [255, 224, 138, 250];

/// 라이브 자원 stateJson 파싱 결과(캐시 값).
#[derive(Default, Clone, Copy)]
pub struct ResState {
    pub w: u16,
    pub h: u16,
    pub done: bool,
    pub lit: bool,
}

/// 지형 시그니처(FNV-1a) — 맵 교체(데모→라이브 프라임) 감지.
pub fn terrain_sig(terrain: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in terrain {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

pub fn px(buf: &mut [u8], size: usize, x: usize, y: usize, c: Rgba) {
    if x < size && y < size {
        let o = (y * size + x) * 4;
        buf[o..o + 4].copy_from_slice(&c);
    }
}

pub fn rect_fill(buf: &mut [u8], size: usize, x: usize, y: usize, w: usize, h: usize, c: Rgba) {
    for dy in 0..h {
        for dx in 0..w {
            px(buf, size, x + dx, y + dy, c);
        }
    }
}

/// 픽셀 외곽선(양 끝 포함) — 뷰포트 박스.
pub fn rect_stroke(
    buf: &mut [u8],
    size: usize,
    x0: usize,
    y0: usize,
    x1: usize,
    y1: usize,
    c: Rgba,
) {
    if x1 < x0 || y1 < y0 || x1 >= size || y1 >= size {
        return;
    }
    for x in x0..=x1 {
        px(buf, size, x, y0, c);
        px(buf, size, x, y1, c);
    }
    for y in y0..=y1 {
        px(buf, size, x0, y, c);
        px(buf, size, x1, y, c);
    }
}

/// 라이브 자원·구조물 — resMap과 동일 팔레트 + 문명 레이어(건축 발판은
/// 서버 앵커=좌상단 타일, state.w/h — §17.75).
pub fn draw_live(
    buf: &mut [u8],
    size: usize,
    frame: &LiveFrame,
    cache: &mut HashMap<u32, (String, ResState)>,
) {
    use crate::net::LiveResource;
    let parse = |r: &LiveResource, cache: &mut HashMap<u32, (String, ResState)>| {
        let raw = r.state.as_deref().unwrap_or("");
        if let Some((prev, parsed)) = cache.get(&r.id)
            && prev == raw
        {
            return *parsed;
        }
        let v: serde_json::Value = serde_json::from_str(raw).unwrap_or_default();
        let g = |k: &str| v.get(k);
        let ty = g("type").and_then(|t| t.as_str()).unwrap_or("");
        let w = g("w")
            .and_then(|w| w.as_u64())
            .filter(|w| *w >= 1)
            .map(|w| w as u16)
            .unwrap_or(match ty {
                "LONGHOUSE" | "WAREHOUSE" => 3,
                "WINDBREAK" => 1,
                _ => 2,
            });
        let h = g("h")
            .and_then(|h| h.as_u64())
            .filter(|h| *h >= 1)
            .map(|h| h as u16)
            .unwrap_or(match ty {
                "WAREHOUSE" => 3,
                "WINDBREAK" => 1,
                _ => 2,
            });
        let parsed = ResState {
            w,
            h,
            done: g("stage")
                .and_then(|s| s.as_str())
                .map(|s| s == "COMPLETED")
                .unwrap_or(true),
            lit: g("lit").and_then(|l| l.as_bool()).unwrap_or(false),
        };
        cache.insert(r.id, (raw.to_owned(), parsed));
        parsed
    };
    for r in &frame.resources {
        match r.kind.as_str() {
            "tree" => px(buf, size, r.x as usize, r.y as usize, TREE),
            "plant" => px(buf, size, r.x as usize, r.y as usize, PLANT),
            "gold_vein" => px(buf, size, r.x as usize, r.y as usize, GOLD),
            "hut" => {
                let st = parse(r, cache);
                let c = if st.done { HUT_DONE } else { HUT_WIP };
                rect_fill(
                    buf,
                    size,
                    r.x as usize,
                    r.y as usize,
                    st.w as usize,
                    st.h as usize,
                    c,
                );
            }
            "campfire" => {
                let st = parse(r, cache);
                rect_fill(
                    buf,
                    size,
                    r.x as usize,
                    r.y as usize,
                    2,
                    2,
                    if st.lit { CAMP_LIT } else { CAMP_OUT },
                );
            }
            "grave" => px(buf, size, r.x as usize, r.y as usize, GRAVE),
            "relic" => px(buf, size, r.x as usize, r.y as usize, RELIC),
            "stash" => px(buf, size, r.x as usize, r.y as usize, STASH),
            _ => {} // animal 등 — 주민 점과 혼동돼 스킵(구 웹과 동일)
        }
    }
}

/// 데모 프롭 — WorldData.props(결정론 배치)를 같은 문명 팔레트로.
pub fn draw_demo_props(buf: &mut [u8], size: usize, world: &WorldData) {
    for p in &world.props {
        let (x, y) = (p.x, p.y);
        match p.kind {
            PropKind::TreeBroadleaf
            | PropKind::TreePine
            | PropKind::TreeSapling => px(buf, size, x, y, TREE),
            PropKind::TreeDead => px(buf, size, x, y, TREE_DEAD),
            PropKind::GoldVein => px(buf, size, x, y, GOLD),
            PropKind::BerryBush => px(buf, size, x, y, PLANT),
            PropKind::Boulder => px(buf, size, x, y, TERRAIN[4]),
            PropKind::Hut => rect_fill(buf, size, x, y, 2, 2, HUT_DONE),
            PropKind::StorageHut => rect_fill(buf, size, x, y, 3, 3, HUT_DONE),
            PropKind::Grave => px(buf, size, x, y, GRAVE),
            PropKind::Relic => px(buf, size, x, y, RELIC),
            PropKind::FlowerClump
            | PropKind::GrassClump
            | PropKind::ClayPot
            | PropKind::AnimalBoar
            | PropKind::AnimalDeer
            | PropKind::AnimalWaterDeer
            | PropKind::AnimalSquirrel
            | PropKind::AnimalPheasant => {}
        }
    }
}
