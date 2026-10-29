//! 미니맵 — 단일 구현(웹 wasm + 네이티브 공용, 2026-10-25 웹 DOM→bevy_ui
//! 통일 첫 이식 — 나머지 웹 UI는 DOM 유지, CLAUDE.md UI 원칙의 명시 예외).
//! 지형은 WorldData(GID 그리드)를 1px/타일로 베이크하고, 500ms 주기로
//! 문명 레이어(건축물·시설)·자원 점·주민 점·뷰포트 박스를 덧그린다.
//! 탭 → PanBridge 큐 → apply_pan_to(추적 해제·관성 정리는 그 규약).
//! 하단 칩은 링크 상태(LIVE/데모)·발명 수 — 문명 팝업은 후속 이슈.

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use bevy::ui::RelativeCursorPosition;
use std::collections::HashMap;

use super::minimap_draw::{
    ResState, SELECTED, TERRAIN, VILLAGER, VIEW_DARK, VIEW_GOLD, draw_demo_props, draw_live,
    rect_fill, rect_stroke, terrain_sig,
};
use super::theme::UiTheme;
use crate::net::apply::LiveAgentTag;
use crate::net::{NetState, NetStatus};
use crate::world::camera_rig::CameraRig;
use crate::world::data::WorldData;
use crate::world::overlay::{PanBridge, view_box};
use crate::world::pick::Selection;
use crate::world::agent::Villager;

/// 미니맵 플러그인 — 웹(wasm)·네이티브 양측에서 등록(lib.rs). UiPlugin과
/// 별개로 자체 테마 확보(ensure_theme)하므로 플러그인 추가 순서와 무관하게
/// 스폰이 안전하다(Res<UiTheme> 부재 패닉 회피).
pub struct MinimapPlugin;

impl Plugin for MinimapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MinimapState>()
            .add_systems(Startup, (super::ensure_theme, spawn_minimap).chain())
            .add_systems(Update, (update_minimap, apply_safe_top).chain());
        // 웹은 DOM UI라 UiPlugin의 전체 Interaction 스캔이 없다 — 미니맵 위
        // 포인터 → 월드 팬 양보(UiHover)를 여기서 직접 갱신. 탭 픽킹은
        // pick.rs가 실시간 Interaction 스캔으로 자동 제외.
        #[cfg(target_arch = "wasm32")]
        app.add_systems(Update, hover_gate);
    }
}

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
/// HUD 높이 30px + 여백 — MinimapRoot.top 오프셋(네이티브).
pub const SAFE_TOP_OFFSET: f32 = 34.0;
/// 웹 DOM 상단 바 아래 슬롯 — 구 tf-minimap.js top:46과 동일 자리.
const WEB_TOP_OFFSET: f32 = 46.0;
const UPDATE_SECS: f32 = 0.5;
/// 라이브 칩 색 — 접속=초록, 끊김(연결·재접속)=빨강(2026-10-25 사용자 지시.
/// 하단 DOM 배지는 폐지하고 이 칩이 접속 상태의 단일 표시지점).
const LIVE_GREEN: Color = Color::srgb(0.33, 0.85, 0.47);
const LIVE_RED: Color = Color::srgb(1.0, 0.42, 0.42);

/// 루트 top — 네이티브=인셋+HUD, 웹=DOM 상단 바 아래 고정 슬롯.
fn top_offset(safe: &crate::platform::safe_area::SafeArea) -> f32 {
    if cfg!(target_arch = "wasm32") {
        WEB_TOP_OFFSET
    } else {
        safe.top + SAFE_TOP_OFFSET
    }
}

#[derive(Resource)]
pub struct MinimapState {
    side: f32,
    bake: Option<Bake>,
    image: Option<Handle<Image>>,
    acc: f32,
    last_chip: String,
    /// 자원 stateJson 파싱 캐시(id → (원문, 파싱)) — 500ms마다 전수 재파싱 방지.
    res_cache: HashMap<u32, (String, ResState)>,
}

#[derive(Clone)]
struct Bake {
    sig: u64,
    size: usize,
    pixels: Vec<u8>,
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
                top: Val::Px(top_offset(&safe)),
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
            // 칩 색은 update_minimap이 첫 500ms 주기에 기록(label이
            // (TextFont, TextColor) 튜플이라 스폰에서 TextColor 중복 금지).
            p.spawn((MinimapChip, Text::new("…연결 중"), theme.label(9.0)));
        });
}

/// 지도 탭 → 월드 점프(PanBridge 큐 — apply_pan_to가 다음 프레임에 반영:
/// 추적 해제·관성 정리까지 동일 규약). Interaction 폴링이 아닌 확정 이벤트
/// 옵저버 — 빠른 탭 유실 방지(native-ui 레시피).
fn on_map_tap(
    trigger: On<Pointer<Click>>,
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
    mut q_chip: Query<(&mut Text, &mut TextColor), With<MinimapChip>>,
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
        if let Some(h) = st.image.clone() {
            img.image = h;
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
    // 뷰박스 — 곡률(행성 뷰) 구간에선 평면 AABB가 무의미하므로 숨긴다.
    if crate::world::globe::factor_for(rig.zoom) < 0.05 {
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
    }

    if let Some(h) = &st.image
        && let Some(mut img) = images.get_mut(h)
    {
        img.data = Some(buf);
    }

    // 상태 칩 — 라이브=초록 '라이브', 끊김·재접속=빨강(라벨과 색은 상태 1:1).
    let (chip, color) = chip_label(&net);
    if chip != st.last_chip {
        st.last_chip = chip.clone();
        if let Ok((mut t, mut c)) = q_chip.single_mut() {
            **t = chip;
            *c = TextColor(color);
        }
    }
}

/// 루트 top 패치 — 안드로이드 인셋 늦은 확정(0.5s 리프레시) 대응.
/// UiPlugin::apply_safe_area에서 이관(미니맵이 플러그인 분리와 함께 소유).
fn apply_safe_top(
    safe: Res<crate::platform::safe_area::SafeArea>,
    mut q: Query<&mut Node, With<MinimapRoot>>,
    mut applied: Local<f32>,
) {
    let top = top_offset(&safe);
    if (top - *applied).abs() < 0.5 {
        return;
    }
    *applied = top;
    if let Ok(mut n) = q.single_mut() {
        n.top = Val::Px(top);
    }
}

/// 웹 전용 — 미니맵 위 포인터를 월드 팬 입력 양보로 알림. 네이티브은
/// UiPlugin의 전체 Interaction 스캔이 같은 역할을 한다(이 시스템 없음).
#[cfg(target_arch = "wasm32")]
fn hover_gate(
    q: Query<&Interaction, With<MinimapMap>>,
    mut hover: ResMut<crate::platform::UiHover>,
) {
    hover.0 = q
        .single()
        .is_ok_and(|i| matches!(i, Interaction::Hovered | Interaction::Pressed));
}

/// 상태 칩 라벨+색 — 접속=초록 '라이브', 그 외 링크 상태는 빨강(Idle=중립).
fn chip_label(net: &NetState) -> (String, Color) {
    match net.status {
        NetStatus::Connected => ("● 라이브".to_owned(), LIVE_GREEN),
        NetStatus::Connecting => ("…연결 중".to_owned(), LIVE_RED),
        NetStatus::Reconnecting => ("↻ 재접속 중".to_owned(), LIVE_RED),
        NetStatus::Idle => ("데모 마을".to_owned(), super::theme::INK400),
    }
}
