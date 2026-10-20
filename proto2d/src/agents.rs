//! 주민 — LPC walk 시트(1152×512 = 4방향×9프레임, 프레임 128) 8인
//! 배회·애님·선택. 행 순서는 LPC 표준(0=상 1=좌 2=하 3=우).
use bevy::prelude::*;
use bevy::sprite::Anchor;
use bevy::text::{FontSize, FontSource};
use crate::mapdata::*;
use crate::props::z_of;

#[derive(Component)]
pub struct Agent {
    pub name: String,
    pub gx: f32,
    pub gy: f32,
    pub tx: f32,
    pub ty: f32,
    pub rest: f32,
    pub dir: usize,
    pub anim: f32,
    rng: XorShift,
}

/// 데모 배회 AI 전용 태그 — 라이브 주민은 wander를 돌리지 않는다
#[derive(Component)]
pub struct DemoAgent;

#[derive(Component)]
pub struct Selected;

#[derive(Component)]
pub struct SelectionMarker;

#[derive(Component)]
pub struct SelectionName;

#[derive(serde::Deserialize)]
struct VillagerDef {
    name: String,
    file: String,
    cols: usize,
    frame: usize,
    #[allow(dead_code)]
    fps: f32,
}

/// LPC 시트 라이브러리 — 데모/라이브 스폰 공용(Startup 1회 빌드)
#[derive(Resource)]
pub struct SheetLibrary {
    pub sheets: Vec<SheetDef>,
    pub shadow: Handle<Image>,
}

pub struct SheetDef {
    pub name: String,
    pub image: Handle<Image>,
    pub layout: Handle<TextureAtlasLayout>,
    pub cols: usize,
}

/// lookJson.uuid(실패 시 id) → 시트 인덱스 — 결정론(같은 uuid = 같은 외형).
/// len은 SheetLibrary 길이(하드코딩 금지 — villagers.json 시트 수와 정합).
pub fn sheet_for(key: &str, len: usize) -> usize {
    let mut h: u32 = 2166136261;
    for b in key.bytes() {
        h ^= b as u32;
        h = h.wrapping_mul(16777619);
    }
    (h as usize) % len.max(1)
}

/// villagers.json에서 시트 라이브러리 빌드
pub fn build_library(
    assets: &AssetServer,
    layouts: &mut Assets<TextureAtlasLayout>,
) -> SheetLibrary {
    let defs: Vec<VillagerDef> =
        serde_json::from_str(include_str!("../assets/lpc/villagers.json")).unwrap();
    let sheets = defs
        .into_iter()
        .map(|def| SheetDef {
            name: def.name,
            image: assets.load(def.file),
            layout: layouts.add(TextureAtlasLayout::from_grid(
                UVec2::splat(def.frame as u32),
                def.cols as u32,
                4,
                None,
                None,
            )),
            cols: def.cols,
        })
        .collect();
    SheetLibrary {
        sheets,
        shadow: assets.load("shadow.png"),
    }
}

/// 라이브 주민용 Agent 생성자 — 배회 rng는 미사용(더미)
pub fn live_agent(name: &str, gx: f32, gy: f32, moving: bool, dir: usize) -> Agent {
    Agent {
        name: name.to_owned(),
        gx,
        gy,
        tx: gx,
        ty: gy,
        rest: if moving { 0.0 } else { 1.0 },
        dir,
        anim: 0.0,
        rng: XorShift::new(1),
    }
}

/// 시트 스폰 공용 헬퍼 — 스프라이트+그림자(Agent는 호출자 값으로 삽입)
pub fn spawn_sheet(
    commands: &mut Commands,
    lib: &SheetLibrary,
    sheet: usize,
    agent: Agent,
) -> Entity {
    let def = &lib.sheets[sheet % lib.sheets.len()];
    let (px, py) = project(agent.gx, agent.gy);
    commands
        .spawn((
            agent,
            Transform::from_xyz(px, -py - TH * 0.25, z_of(py)),
            Anchor::BOTTOM_CENTER,
            Sprite {
                image: def.image.clone(),
                texture_atlas: Some(TextureAtlas {
                    layout: def.layout.clone(),
                    index: 2 * def.cols, // 하 방향 프레임0
                }),
                ..default()
            },
        ))
        .with_child((
            Transform::from_xyz(0.0, 4.0, -0.2),
            Sprite {
                image: lib.shadow.clone(),
                custom_size: Some(Vec2::new(66.0, 25.0)),
                color: Color::srgba(0.02, 0.015, 0.0, 0.38),
                ..default()
            },
        ))
        .id()
}

pub fn spawn(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut layouts: ResMut<Assets<TextureAtlasLayout>>,
) {
    let lib = build_library(&assets, &mut layouts);
    let font = assets.load("fonts/NotoSansKR-Regular.otf");
    let (cx, cy) = village_center();
    let mut rng = XorShift::new(777);
    for i in 0..lib.sheets.len() {
        // 마을 중심 주변 walkable 스폰
        let (sx, sy) = loop {
            let x = cx + rng.range_i32(-6, 6);
            let y = cy + rng.range_i32(-6, 6);
            if walkable(x, y) {
                break (x, y);
            }
        };
        let mut a = Agent {
            name: lib.sheets[i].name.clone(),
            gx: sx as f32,
            gy: sy as f32,
            tx: sx as f32,
            ty: sy as f32,
            rest: rng.next_f32() * 2.0,
            dir: 2,
            anim: 0.0,
            rng: XorShift::new(1000 + i as u32),
        };
        pick_target(&mut a);
        let e = spawn_sheet(&mut commands, &lib, i, a);
        commands.entity(e).insert(DemoAgent);
    }

    // 라이브 스폰용 리소스로도 등록(핸들 재사용 — 추가 로드 없음)
    commands.insert_resource(lib);

    // 선택 마커 — 링 + 이름 라벨(평시 숨김)
    commands
        .spawn((
            SelectionMarker,
            Visibility::Hidden,
            Transform::default(),
            Sprite {
                image: assets.load("ring.png"),
                custom_size: Some(Vec2::new(88.0, 44.0)),
                ..default()
            },
        ))
        .with_children(|p| {
            p.spawn((
                SelectionName,
                Transform::from_xyz(0.0, 152.0, 0.0),
                Text2d::new(""),
                TextFont {
                    font: FontSource::Handle(font),
                    font_size: FontSize::Px(26.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.97, 0.86)),
            ));
        });
}

fn pick_target(a: &mut Agent) {
    for _ in 0..12 {
        let x = a.gx as i32 + a.rng.range_i32(-8, 8);
        let y = a.gy as i32 + a.rng.range_i32(-8, 8);
        if walkable(x, y) {
            a.tx = x as f32;
            a.ty = y as f32;
            return;
        }
    }
    a.rest += 0.5; // 못 찾으면 좀 더 쉰다
}

/// 배회(데모 전용) — 그리드 좌표 이동 → 아이소 투영 반영(y-sort z 포함)
pub fn wander(time: Res<Time>, mut q: Query<(&mut Agent, &mut Transform), With<DemoAgent>>) {
    let dt = time.delta_secs();
    for (mut a, mut tr) in &mut q {
        if a.rest > 0.0 {
            a.rest -= dt;
            a.anim = 0.0;
        } else {
            let (dx, dy) = (a.tx - a.gx, a.ty - a.gy);
            let d = (dx * dx + dy * dy).sqrt();
            if d < 0.05 {
                a.rest = a.rng.next_f32() * 3.0 + 1.0;
                pick_target(&mut a);
            } else {
                let step = (1.6 * dt).min(d);
                a.gx += dx / d * step;
                a.gy += dy / d * step;
                // 화면(아이소 투영) 방향 우세축: sx=dx−dy, sy=dx+dy
                let (sx, sy) = (dx - dy, dx + dy);
                a.dir = if sx.abs() > sy.abs() {
                    if sx > 0.0 { 3 } else { 1 }
                } else if sy > 0.0 { 2 } else { 0 };
                a.anim += dt;
            }
        }
        let (px, py) = project(a.gx, a.gy);
        tr.translation = Vec3::new(px, -py - TH * 0.25, z_of(py));
    }
}

/// 애님 — 걷는 동안 9fps 순환, 휴식 중 정지 프레임
pub fn animate(mut q: Query<(&Agent, &mut Sprite)>) {
    for (a, mut s) in &mut q {
        let Some(ta) = &mut s.texture_atlas else { continue };
        let cols = 9;
        let f = if a.rest > 0.0 { 0 } else { (a.anim * 9.0) as usize % cols };
        ta.index = a.dir * cols + f;
    }
}

/// 공통 픽킹 — 반경 내 최근접 주민 토글(재클릭 해제) + 선택 시 추적 재개.
/// 주민이 아니면 유물(relic) 픽(§17.38) — 근접 relic 탭/클릭으로 발명품
/// 카드를 연다. 빈 땅 탭은 선택·카드 모두 끈다(레거시 단일 창 정책).
#[allow(clippy::too_many_arguments)]
fn pick_and_apply(
    commands: &mut Commands,
    world: Vec2,
    agents_q: &Query<(Entity, &Transform, Option<&Selected>), With<Agent>>,
    res_q: &Query<(&crate::net::live_props::LiveResourceTag, &Transform)>,
    net: Option<&crate::net::NetState>,
    relic: &mut crate::ui::relic::RelicCard,
    marker: &mut Query<&mut Visibility, With<SelectionMarker>>,
    manual: &mut crate::cam::ManualUntil,
    now: f32,
) {
    let mut best: Option<(Entity, f32, bool)> = None;
    for (e, tr, sel) in agents_q {
        // 몸통 중심(발밑 위 56px) 기준 거리
        let c = tr.translation.truncate() + Vec2::new(0.0, 56.0);
        let d = c.distance(world);
        if d < 72.0 && best.map(|(_, bd, _)| d < bd).unwrap_or(true) {
            best = Some((e, d, sel.is_some()));
        }
    }
    for (e, _, _) in agents_q {
        commands.entity(e).remove::<Selected>();
    }
    let mut show = false;
    if let Some((e, _, already)) = best {
        if !already {
            commands.entity(e).insert(Selected);
            show = true;
        }
    }
    if show {
        manual.0 = now; // 터치/드래그 유예 해제 — 즉시 추적 재개
    }
    if let Ok(mut vis) = marker.single_mut() {
        *vis = if show { Visibility::Visible } else { Visibility::Hidden };
    }
    // 유물 픽 — 주민이 잡히지 않았을 때만(주민이 우선).
    if best.is_none() {
        if let Some(info) = pick_relic(world, res_q, net) {
            relic.0 = Some(info);
            return;
        }
    }
    relic.0 = None;
}

/// 근접 relic 탐색 — 자원 엔티티 중 kind=relic의 최근접(반경 64). stateJson의
/// by·purpose·based_on을 카드로 싣는다(레거시 select-relic 페이로드 동일).
fn pick_relic(
    world: Vec2,
    res_q: &Query<(&crate::net::live_props::LiveResourceTag, &Transform)>,
    net: Option<&crate::net::NetState>,
) -> Option<crate::ui::relic::RelicInfo> {
    let frame = net?.frame.as_ref()?;
    let mut best: Option<(u32, f32)> = None;
    for (tag, tr) in res_q {
        let d = tr.translation.truncate().distance(world);
        if d < 64.0 && best.map(|(_, bd)| d < bd).unwrap_or(true) {
            best = Some((tag.0, d));
        }
    }
    let id = best?.0;
    let r = frame.resources.iter().find(|r| r.id == id)?;
    if r.kind != "relic" {
        return None;
    }
    let st = r
        .state
        .as_deref()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        .unwrap_or(serde_json::Value::Null);
    let s = |k: &str| {
        st.get(k)
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_owned()
    };
    Some(crate::ui::relic::RelicInfo {
        name: r.species.clone(),
        by: s("by"),
        purpose: s("purpose"),
        based_on: s("based_on"),
        icon: st
            .get("icon")
            .cloned()
            .and_then(|v| serde_json::from_value(v).ok()),
    })
}

/// 클릭 선택 — 드래그가 아니었을 때만.
pub fn select_on_click(
    mut commands: Commands,
    time: Res<Time>,
    mut egui_ctx: bevy_egui::EguiContexts,
    buttons: Res<ButtonInput<MouseButton>>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    drag: Res<crate::cam::DragState>,
    agents_q: Query<(Entity, &Transform, Option<&Selected>), With<Agent>>,
    res_q: Query<(&crate::net::live_props::LiveResourceTag, &Transform)>,
    net: Option<Res<crate::net::NetState>>,
    mut relic: ResMut<crate::ui::relic::RelicCard>,
    mut marker: Query<&mut Visibility, With<SelectionMarker>>,
    mut manual: ResMut<crate::cam::ManualUntil>,
) {
    if !buttons.just_released(MouseButton::Left) || drag.total > 5.0 {
        return;
    }
    // egui가 포인터를 쓰는 중(패널 클릭)이면 월드 선택 아님 — 세지 않게 가드
    if crate::ui::egui_owns_input(&mut egui_ctx) {
        return;
    }
    let Ok(window) = windows.single() else { return };
    let Some(cursor) = window.cursor_position() else { return };
    let Ok((camera, cam_tf)) = camera_q.single() else { return };
    let Ok(world) = camera.viewport_to_world_2d(cam_tf, cursor) else { return };
    pick_and_apply(
        &mut commands,
        world,
        &agents_q,
        &res_q,
        net.as_deref(),
        &mut relic,
        &mut marker,
        &mut manual,
        time.elapsed_secs(),
    );
}

/// 터치 탭 선택 — touch_gestures가 기록한 탭 좌표 소비
pub fn select_on_tap(
    mut commands: Commands,
    time: Res<Time>,
    mut egui_ctx: bevy_egui::EguiContexts,
    mut tap: ResMut<crate::cam::TapPos>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    agents_q: Query<(Entity, &Transform, Option<&Selected>), With<Agent>>,
    res_q: Query<(&crate::net::live_props::LiveResourceTag, &Transform)>,
    net: Option<Res<crate::net::NetState>>,
    mut relic: ResMut<crate::ui::relic::RelicCard>,
    mut marker: Query<&mut Visibility, With<SelectionMarker>>,
    mut manual: ResMut<crate::cam::ManualUntil>,
) {
    // tap.0은 touch_gestures가 월드 탭으로만 채운다(패널 위 탭은 애초에
    // 기록되지 않음) — egui 포인터 배타 주장과 무관하게 소비한다.
    let _ = &mut egui_ctx;
    let Some(screen) = tap.0.take() else { return };
    let Ok((camera, cam_tf)) = camera_q.single() else { return };
    let Ok(world) = camera.viewport_to_world_2d(cam_tf, screen) else { return };
    pick_and_apply(
        &mut commands,
        world,
        &agents_q,
        &res_q,
        net.as_deref(),
        &mut relic,
        &mut marker,
        &mut manual,
        time.elapsed_secs(),
    );
}

/// 선택 마커 추적 — 링은 발밑, 이름은 머리 위
pub fn marker_follow(
    time: Res<Time>,
    selected: Query<(Entity, &Transform), (With<Selected>, Without<SelectionMarker>)>,
    agents: Query<&Agent>,
    mut marker: Query<&mut Transform, (With<SelectionMarker>, Without<Selected>)>,
    mut name: Query<&mut Text2d, With<SelectionName>>,
) {
    let Some((e, st)) = selected.iter().next() else { return };
    if let Ok(mut m) = marker.single_mut() {
        m.translation = st.translation.truncate().extend(st.translation.z + 0.5);
        // 선택 링 펄스 — 호흡듯 6% 스케일 (호버 대상 인지성)
        let pulse = 1.0 + (time.elapsed_secs() * 3.2).sin() * 0.06;
        m.scale = Vec3::splat(pulse);
    }
    if let (Ok(mut t), Ok(a)) = (name.single_mut(), agents.get(e)) {
        **t = a.name.clone().into();
    }
}
