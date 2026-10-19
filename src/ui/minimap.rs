//! 미니맵 — 웹 tf-minimap.js의 bevy_ui 포트 + 문명 레이어.
//! 지형은 WorldData(GID 그리드)를 1px/타일로 베이크하고, 500ms 주기로
//! 문명 레이어(건축물·시설)·자원 점·주민 점·뷰포트 박스를 덧그린다.
//! 탭 → PanBridge 큐 → apply_pan_to(추적 해제·관성 정리는 그 규약).
//! 하단 칩은 링크 상태(LIVE/데모)·발명 수 — 문명 팝업은 후속 이슈.

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::ui_widgets::Button; // 0.20: prelude의 Button은 deprecated alias
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::RelativeCursorPosition;
use std::collections::HashMap;

use super::theme::UiTheme;
use crate::net::apply::LiveAgentTag;
use crate::net::{NetState, NetStatus};
use crate::world::camera_rig::CameraRig;
use crate::world::data::{PropKind, WorldData};
use crate::world::overlay::{PanBridge, view_box};
use crate::world::pick::Selection;
use crate::world::villager::Villager;

/// 루트 마커 — 세이프에어리어 패치 대상(top = 인셋 + HUD 아래).
#[derive(Component)]
pub struct MinimapRoot;
/// 탭 대상 지도 노드(버튼 + 이미지).
#[derive(Component)]
pub struct MinimapMap;
/// 상태 칩 텍스트.
#[derive(Component)]
pub struct MinimapChip;

const SIDE_DESK: f32 = 148.0;
const SIDE_MOB: f32 = 112.0;
/// HUD 높이 30px + 여백 — MinimapRoot.top 오프셋(apply_safe_area와 커플).
pub const SAFE_TOP_OFFSET: f32 = 34.0;
const UPDATE_SECS: f32 = 0.5;

type Rgba = [u8; 4];
const fn rgb(hex: [u8; 3]) -> Rgba {
    [hex[0], hex[1], hex[2], 255]
}
/// GID 1..6 → 지형색(웹 TERRAIN_COLORS와 동일 팔레트).
const TERRAIN: [Rgba; 6] = [
    rgb([0x63, 0xc5, 0xcf]), // water
    rgb([0xea, 0xa5, 0x6c]), // sand
    rgb([0x8d, 0xc4, 0x35]), // grass
    rgb([0x4f, 0x94, 0x48]), // forest
    rgb([0x9a, 0x94, 0x8a]), // rock
    rgb([0xc8, 0xa0, 0x6a]), // dirt
];
const TREE: Rgba = rgb([0x2f, 0x5d, 0x2a]);
const TREE_DEAD: Rgba = rgb([0x6b, 0x5a, 0x48]);
const GOLD: Rgba = rgb([0xf0, 0xc0, 0x2e]);
const PLANT: Rgba = rgb([0xb8, 0xe9, 0x86]);
const HUT_DONE: Rgba = rgb([0xb5, 0x71, 0x3f]);
const HUT_WIP: Rgba = rgb([0xff, 0xd2, 0x8a]);
const CAMP_LIT: Rgba = rgb([0xff, 0x9f, 0x43]);
const CAMP_OUT: Rgba = rgb([0x6b, 0x5a, 0x48]);
const GRAVE: Rgba = rgb([0xe8, 0xe4, 0xda]);
const RELIC: Rgba = rgb([0xb4, 0x8e, 0xe0]);
const STASH: Rgba = rgb([0xc9, 0xa3, 0x6a]);
const VILLAGER: Rgba = rgb([0xff, 0x6b, 0x6b]);
const SELECTED: Rgba = rgb([0xff, 0xff, 0xff]);
const VIEW_DARK: Rgba = [20, 24, 16, 165];
const VIEW_GOLD: Rgba = [255, 224, 138, 250];

#[derive(Resource)]
pub struct MinimapState {
    side: f32,
    bake: Option<Bake>,
    image: Option<Handle<Image>>,
    acc: f32,
    last_chip: String,
    /// 자원 stateJson 파싱 캐시(id → (원문, 파싱)) — 500ms마다 전수 재파싱 방지.
    res_cache: HashMap<u32, (String, ResState)>,
    inv_count: Option<(String, usize)>,
}

#[derive(Clone)]
struct Bake {
    sig: u64,
    size: usize,
    pixels: Vec<u8>,
}

#[derive(Default, Clone, Copy)]
struct ResState {
    w: u16,
    h: u16,
    done: bool,
    lit: bool,
}

impl Default for MinimapState {
    fn default() -> Self {
        MinimapState {
            side: SIDE_DESK,
            bake: None,
            image: None,
            acc: UPDATE_SECS,
            last_chip: String::new(),
            res_cache: HashMap::new(),
            inv_count: None,
        }
    }
}

pub fn spawn_minimap(
    mut commands: Commands,
    theme: Res<UiTheme>,
    safe: Res<crate::platform::safe_area::SafeArea>,
) {
    commands
        .spawn((
            MinimapRoot,
            Node {
                position_type: PositionType::Absolute,
                top: Val::Px(safe.top + SAFE_TOP_OFFSET),
                left: Val::Px(12.0),
                flex_direction: FlexDirection::Column,
                row_gap: Val::Px(4.0),
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                MinimapMap,
                Button,
                ImageNode::default(),
                RelativeCursorPosition::default(),
                Node {
                    width: Val::Px(SIDE_DESK),
                    height: Val::Px(SIDE_DESK),
                    border: UiRect::all(Val::Px(2.0)),
                    ..default()
                },
                BorderColor::from(Color::srgba(0.02, 0.03, 0.04, 0.9)),
            ))
            .observe(on_map_tap);
            p.spawn((MinimapChip, Text::new("데모 마을"), theme.label(9.0)));
        });
}

/// 지도 탭 → 월드 점프(PanBridge 큐 — apply_pan_to가 다음 프레임에 반영:
/// 추적 해제·관성 정리까지 동일 규약). Interaction 폴링이 아닌 확정 이벤트
/// 옵저버 — 빠른 탭 유실 방지(native-ui 레시피).
fn on_map_tap(
    trigger: On<PointerClick>,
    q_map: Query<&RelativeCursorPosition, With<MinimapMap>>,
    world: Res<WorldData>,
    pan: ResMut<PanBridge>,
) {
    let _ = trigger;
    let (Ok(rel), Ok(mut q)) = (q_map.single(), pan.queue.lock()) else {
        return;
    };
    let Some(norm) = rel.normalized else { return };
    let size = world.size as f32;
    // 상대 좌표(0..1, y위는 아래) → 타일. 노드 여백·테두리 포함이라 근사면 충분.
    let x = (norm.x.clamp(0.0, 1.0) * size).min(size - 1.0);
    let y = ((1.0 - norm.y.clamp(0.0, 1.0)) * size).min(size - 1.0);
    q.push((x, y));
}

#[allow(clippy::type_complexity)]
#[allow(clippy::too_many_arguments)] // Bevy 시스템 파라미터(월드·넷·카메라·UI 자원 동시 참조)
pub fn update_minimap(
    time: Res<Time>,
    world: Res<WorldData>,
    net: Res<NetState>,
    rig: Res<CameraRig>,
    selection: Res<Selection>,
    villagers: Query<(Entity, &Transform, Option<&LiveAgentTag>), With<Villager>>,
    window: Single<&Window>,
    mut q_map: Query<(&mut Node, &mut ImageNode), With<MinimapMap>>,
    mut q_chip: Query<&mut Text, With<MinimapChip>>,
    mut st: ResMut<MinimapState>,
    mut images: ResMut<Assets<Image>>,
) {
    // 500ms 주기 — 베이크·오버레이·칩 갱신(텍스처 쓰기는 여기서만).
    st.acc += time.delta_secs();
    let due = st.acc >= UPDATE_SECS;
    if !due {
        return;
    }
    st.acc = 0.0;

    // 창 폭 전환 — 데스크톱 148/모바일 112 재적용(웹과 동일 기준 640px).
    let side = if window.width() < 640.0 {
        SIDE_MOB
    } else {
        SIDE_DESK
    };
    if let Ok((mut node, mut img)) = q_map.single_mut() {
        if st.side != side {
            st.side = side;
            node.width = Val::Px(side);
            node.height = Val::Px(side);
        }
        if st.image.is_some() {
            img.image = st.image.clone().unwrap();
        }
    }

    // 지형 베이크 — 크기·내용 시그니처 변화 시 1회 재구성.
    let size = world.size;
    let sig = terrain_sig(&world.terrain);
    let need_bake = st
        .bake
        .as_ref()
        .map(|b| b.sig != sig || b.size != size)
        .unwrap_or(true);
    if need_bake {
        if size == 0 {
            return;
        }
        let mut pixels = Vec::with_capacity(size * size * 4);
        for &gid in &world.terrain {
            pixels.extend_from_slice(&TERRAIN[(gid.clamp(1, 6) - 1) as usize]);
        }
        let image = Image::new(
            Extent3d {
                width: size as u32,
                height: size as u32,
                depth_or_array_layers: 1,
            },
            TextureDimension::D2,
            pixels.clone(),
            TextureFormat::Rgba8UnormSrgb,
            RenderAssetUsages::default(),
        );
        match &st.image {
            Some(h) => {
                if let Some(mut img) = images.get_mut(h) {
                    img.data = Some(pixels.clone());
                    // 크기 변화(데모→라이브 맵 교체)까지 반영
                    img.texture_descriptor.size = Extent3d {
                        width: size as u32,
                        height: size as u32,
                        depth_or_array_layers: 1,
                    };
                }
            }
            None => {
                st.image = Some(images.add(image));
            }
        }
        st.bake = Some(Bake { sig, size, pixels });
    }
    let Some(bake) = st.bake.clone() else {
        return;
    };

    // 오버레이 — 베이크 사본에 문명 레이어·자원·주민·뷰포트 박스.
    let mut buf = bake.pixels;
    if let Some(frame) = net.frame.as_ref() {
        draw_live(&mut buf, size, frame, &mut st.res_cache);
    } else {
        draw_demo_props(&mut buf, size, &world);
    }
    for (entity, tr, _tag) in villagers.iter() {
        let x = tr.translation.x.round().clamp(0.0, (size - 1) as f32) as usize;
        let y = tr.translation.z.round().clamp(0.0, (size - 1) as f32) as usize;
        let sel = selection.entity == Some(entity);
        let c = if sel { SELECTED } else { VILLAGER };
        if sel {
            rect_fill(
                &mut buf,
                size,
                x.saturating_sub(1),
                y.saturating_sub(1),
                3,
                3,
                c,
            );
        } else {
            rect_fill(&mut buf, size, x, y, 2, 2, c);
        }
    }
    let (vw, vh) = view_box(rig.zoom, window.width() / window.height().max(1.0));
    let x0 = (rig.target.x - vw * 0.5).floor().clamp(0.0, size as f32) as usize;
    let y0 = (rig.target.y - vh * 0.5).floor().clamp(0.0, size as f32) as usize;
    let x1 = ((rig.target.x + vw * 0.5)
        .ceil()
        .clamp(0.0, (size - 1) as f32)) as usize;
    let y1 = ((rig.target.y + vh * 0.5)
        .ceil()
        .clamp(0.0, (size - 1) as f32)) as usize;
    rect_stroke(&mut buf, size, x0, y0, x1, y1, VIEW_DARK);
    if x1 > x0 + 1 && y1 > y0 + 1 {
        rect_stroke(&mut buf, size, x0 + 1, y0 + 1, x1 - 1, y1 - 1, VIEW_GOLD);
    }

    if let Some(h) = &st.image
        && let Some(mut img) = images.get_mut(h)
    {
        img.data = Some(buf);
    }

    // 상태 칩 — LIVE면 발명 수(문명 진척), 그 외 링크 단계.
    let chip = chip_label(&net, &mut st.inv_count);
    if chip != st.last_chip {
        st.last_chip = chip.clone();
        if let Ok(mut t) = q_chip.single_mut() {
            **t = chip;
        }
    }
}

fn chip_label(net: &NetState, inv: &mut Option<(String, usize)>) -> String {
    match net.status {
        NetStatus::Connected => {
            let count = net
                .frame
                .as_ref()
                .and_then(|f| f.inventions.as_deref())
                .map(|raw| {
                    if inv.as_ref().map(|(r, _)| r == raw).unwrap_or(false) {
                        inv.as_ref().map(|(_, n)| *n).unwrap_or(0)
                    } else {
                        let n = serde_json::from_str::<serde_json::Value>(raw)
                            .ok()
                            .and_then(|v| v.as_array().map(|a| a.len()))
                            .unwrap_or(0);
                        *inv = Some((raw.to_owned(), n));
                        n
                    }
                });
            match count {
                Some(n) => format!("● LIVE · 발명 {n}"),
                None => "● LIVE".to_owned(),
            }
        }
        NetStatus::Connecting => "…연결 중".to_owned(),
        NetStatus::Reconnecting => "↻ 재접속 중".to_owned(),
        NetStatus::Idle => "데모 마을".to_owned(),
    }
}

/// 라이브 자원·구조물 — resMap과 동일 팔레트 + 문명 레이어(건축 발판은
/// 서버 앵커=좌상단 타일, state.w/h — §17.75).
fn draw_live(
    buf: &mut [u8],
    size: usize,
    frame: &crate::net::LiveFrame,
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
            _ => {} // animal 등 — 주민 점과 혼동돼 스킵(웹과 동일)
        }
    }
}

/// 데모 프롭 — WorldData.props(결정론 배치)를 같은 문명 팔레트로.
fn draw_demo_props(buf: &mut [u8], size: usize, world: &WorldData) {
    for p in &world.props {
        let (x, y) = (p.x, p.y);
        match p.kind {
            PropKind::TreeBroadleaf
            | PropKind::TreePine
            | PropKind::TreePalm
            | PropKind::TreeSapling => px(buf, size, x, y, TREE),
            PropKind::TreeDead => px(buf, size, x, y, TREE_DEAD),
            PropKind::GoldVein => px(buf, size, x, y, GOLD),
            PropKind::BerryBush => px(buf, size, x, y, PLANT),
            PropKind::Boulder => px(buf, size, x, y, TERRAIN[4]),
            PropKind::Hut | PropKind::Tent => rect_fill(buf, size, x, y, 2, 2, HUT_DONE),
            PropKind::StorageHut => rect_fill(buf, size, x, y, 3, 3, HUT_DONE),
            PropKind::Windbreak => px(buf, size, x, y, HUT_WIP),
            PropKind::Campfire => rect_fill(buf, size, x, y, 2, 2, CAMP_LIT),
            PropKind::Grave => px(buf, size, x, y, GRAVE),
            PropKind::Stash => px(buf, size, x, y, STASH),
            PropKind::Relic => px(buf, size, x, y, RELIC),
            PropKind::FlowerClump
            | PropKind::GrassClump
            | PropKind::WoodLog
            | PropKind::AnimalBoar
            | PropKind::AnimalDeer
            | PropKind::AnimalWaterDeer
            | PropKind::AnimalSquirrel
            | PropKind::AnimalPheasant => {}
        }
    }
}

/// 지형 시그니처(FNV-1a) — 맵 교체(데모→라이브 프라임) 감지.
fn terrain_sig(terrain: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf29ce484222325;
    for &b in terrain {
        hash ^= b as u64;
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn px(buf: &mut [u8], size: usize, x: usize, y: usize, c: Rgba) {
    if x < size && y < size {
        let o = (y * size + x) * 4;
        buf[o..o + 4].copy_from_slice(&c);
    }
}

fn rect_fill(buf: &mut [u8], size: usize, x: usize, y: usize, w: usize, h: usize, c: Rgba) {
    for dy in 0..h {
        for dx in 0..w {
            px(buf, size, x + dx, y + dy, c);
        }
    }
}

/// 픽셀 외곽선(양 끝 포함) — 뷰포트 박스.
fn rect_stroke(buf: &mut [u8], size: usize, x0: usize, y0: usize, x1: usize, y1: usize, c: Rgba) {
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
