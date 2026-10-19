//! 라이브 프레임 → 렌더 적용 — 지형 프라임(최초 1회)·시간·스킨·주민.
//! 데모(정적 맵·배회 AI)를 라이브 데이터로 교체한다(root src/net/apply.rs의
//! 2D 투영). 주민 이동은 서버 목적지로 등속 트윈(1.25타일/s — 서버 속도).

use bevy::prelude::*;

use super::{LiveFrame, NetState, NetStatus, TiledMap};
use crate::agents::{self, Agent, SheetLibrary};
use crate::cam::ManualUntil;
use crate::daynight::GameTime;
use crate::ground::{self, GroundSkin, GroundTile};
use crate::mapdata;
use crate::props::DemoProp;

#[derive(Component)]
pub struct LiveAgentTag(pub String);

/// 프레임 공백 유예(초) — 재접속·드랍 프레임 사이 주민 엔티티를 즉시 철거하지
/// 않아 선택 링·카메라 추적이 튕기지 않게 한다.
#[derive(Component, Default)]
pub struct AgentGrace(pub f32);

/// 서버 이동 속도(타일/초) — 룸 walk 0.625타일/500ms틱 = 1.25
const SPEED: f32 = 1.25;
/// 순간이동 임계(타일) — 서버·클라 위치차가 이 이상이면 트윈 없이 스냅
const SNAP_DIST: f32 = 3.0;

/// 라이브 전환 — 데모 지면·소품·주민 제거 후 라이브 지형 스폰(1회).
#[allow(clippy::type_complexity)] // Bevy 시스템 파라미터 — 철거·스폰 자원 일괄
pub fn prime(
    mut commands: Commands,
    mut net: ResMut<NetState>,
    mut primed: Local<bool>,
    q_ground: Query<Entity, With<GroundTile>>,
    q_props: Query<Entity, With<DemoProp>>,
    q_demo_agents: Query<Entity, (With<Agent>, Without<LiveAgentTag>)>,
    assets: Res<AssetServer>,
    skin: Res<GroundSkin>,
    mut camera: Query<&mut Transform, With<Camera2d>>,
    mut manual: ResMut<ManualUntil>,
    time: Res<Time>,
) {
    if *primed || net.status != NetStatus::Connected {
        return;
    }
    let Some(frame) = net.frame.as_ref() else {
        return;
    };
    let Some(map_json) = frame.map.as_ref() else {
        return;
    };
    let Ok(map) = serde_json::from_str::<TiledMap>(map_json) else {
        warn!("[net] mapJson 파싱 실패");
        return;
    };
    *primed = true;
    info!("[net] 라이브 지형 프라임 {}×{}", map.width, map.height);

    // 데모 세계 철거 — 지면 전체·데모 소품·데모 주민(라이브 주민은 유지)
    for e in &q_ground {
        commands.entity(e).despawn();
    }
    for e in &q_props {
        commands.entity(e).despawn();
    }
    for e in &q_demo_agents {
        commands.entity(e).despawn();
    }

    // 라이브 맵 설치(이후 불변) → 지형 전개
    mapdata::set_live_map(map.width, map.height, map.layers[0].data.clone());
    ground::spawn_live(&mut commands, &assets, &map, *skin);
    if std::env::var("TF_DUMP_GIDS").is_ok() {
        let mut hist = std::collections::BTreeMap::new();
        for &g in &map.layers[0].data {
            *hist.entry(g).or_insert(0) += 1;
        }
        let s: Vec<String> = hist.iter().map(|(g, n)| format!("{g}:{n}")).collect();
        info!("[net] GID 히스토그램 — {}", s.join(" "));
    }

    // 카메라 재중심 — '맵 중심에 가장 가까운 주민'에게(root 관전 부팅 패턴).
    // 주민들은 멀리 활동 중이라 광장을 비추면 첫 화면에 사람이 안 보인다.
    let (mw, mh) = mapdata::dims();
    let center = ((mw / 2) as f32, (mh / 2) as f32);
    let target = frame
        .agents
        .iter()
        .filter(|a| a.kind == "bot" || a.kind == "user")
        .map(|a| (a.x, a.y))
        .min_by(|a, b| {
            let da = (a.0 - center.0).hypot(a.1 - center.1);
            let db = (b.0 - center.0).hypot(b.1 - center.1);
            da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
        })
        .unwrap_or(center);
    let (px, py) = mapdata::project(target.0, target.1);
    if let Ok(mut tr) = camera.single_mut() {
        tr.translation.x = px;
        tr.translation.y = -py;
    }
    // 수동 조작 유예 풀기 — 선택 추적 즉시 재개 가능 상태로
    manual.0 = time.elapsed_secs();
    let _ = (&q_ground, &q_props);

    // map은 1회 소비(이후 프레임에서 중복 파싱 방지)
    net.frame = Some(LiveFrame {
        map: None,
        ..frame.clone()
    });
}

/// 서버 시간 → GameTime(로컬 진행 정지 — 서버가 정본) + 계절·날씨 → 스킨.
pub fn sync_daynight(
    net: Res<NetState>,
    mut day: ResMut<GameTime>,
    mut skin: ResMut<GroundSkin>,
    override_skin: Res<crate::ui::menu::SkinOverride>,
) {
    if net.status != NetStatus::Connected {
        return;
    }
    let Some(f) = net.frame.as_ref() else {
        return;
    };
    day.day = f.day as u32;
    day.t = (f.hour as f32 * 60.0 + f.minute as f32) / 1440.0;
    // 겨울 또는 폭설 → 설원(레퍼런스의 백색 대지). 메뉴 강제(SkinOverride) 우선.
    let snow = f.season == "winter" || f.weather.as_deref() == Some("snow");
    let want = if snow { GroundSkin::Snow } else { GroundSkin::Normal };
    let want = override_skin.0.unwrap_or(want);
    if *skin != want {
        info!("[net] 계절 {} · 날씨 {:?} → 스킨 {want:?}", f.season, f.weather);
        *skin = want;
    }
}

/// 주민 동기화 — 신규 스폰 / 목적지·방향·상태 갱신 / 사라진 것 제거(유예 후).
#[allow(clippy::type_complexity)] // Bevy 동기 루프 — 유예 필드까지 읽는 5튜플
pub fn sync_agents(
    mut commands: Commands,
    net: Res<NetState>,
    time: Res<Time>,
    lib: Option<Res<SheetLibrary>>,
    mut q_live: Query<(Entity, &LiveAgentTag, &mut Agent, Option<&mut AgentGrace>)>,
    mut first_log: Local<u32>,
) {
    if net.status != NetStatus::Connected {
        return;
    }
    let Some(frame) = net.frame.as_ref() else {
        return;
    };
    const GRACE_SECS: f32 = 5.0;
    if *first_log == 0 {
        *first_log = 1;
        let eligible = frame
            .agents
            .iter()
            .filter(|a| a.kind == "bot" || a.kind == "user")
            .count();
        info!("[net] 주민 동기 시작 — 프레임 {}명(대상 {}명)", frame.agents.len(), eligible);
    }

    // 갱신·제거
    for (e, tag, mut a, grace) in &mut q_live {
        let Some(la) = frame.agents.iter().find(|la| la.id == tag.0) else {
            match grace {
                Some(mut g) => {
                    g.0 += time.delta_secs();
                    if g.0 >= GRACE_SECS {
                        commands.entity(e).despawn();
                    }
                }
                None => {
                    commands.entity(e).insert(AgentGrace(time.delta_secs()));
                }
            }
            continue;
        };
        if let Some(mut g) = grace {
            g.0 = 0.0;
        }
        // 목적지 = 최신 서버 위치. 크게 벌어지면(리스폰·재접속) 스냅.
        let d = (la.x - a.gx).hypot(la.y - a.gy);
        if d > SNAP_DIST {
            a.gx = la.x;
            a.gy = la.y;
        }
        a.tx = la.x;
        a.ty = la.y;
        let moving = la.status == "moving";
        a.rest = if moving { 0.0 } else { 1.0 };
        a.dir = facing_dir(&la.facing, Some(&a));
        if a.name != la.name {
            a.name = la.name.clone();
        }
    }

    // 신규 스폰 — lookJson.uuid(실패 시 id) 해시로 기존 8시트 결정론 선택
    let Some(lib) = lib else {
        return; // 진단 exp로 agents::spawn이 스킵됐으면 라이브 주민도 없음
    };
    let live_ids: std::collections::HashSet<&str> =
        q_live.iter().map(|(_, t, _, _)| t.0.as_str()).collect();
    let mut spawned = 0usize;
    for la in &frame.agents {
        if live_ids.contains(la.id.as_str()) {
            continue;
        }
        if la.kind != "bot" && la.kind != "user" {
            continue;
        }
        let key = la
            .look
            .as_deref()
            .and_then(|l| serde_json::from_str::<LookUuid>(l).ok())
            .and_then(|lk| lk.uuid)
            .unwrap_or_else(|| la.id.clone());
        let sheet = agents::sheet_for(&key, lib.sheets.len());
        let agent = agents::live_agent(
            &la.name,
            la.x,
            la.y,
            la.status == "moving",
            facing_dir(&la.facing, None),
        );
        let e = agents::spawn_sheet(&mut commands, &lib, sheet, agent);
        commands
            .entity(e)
            .insert((LiveAgentTag(la.id.clone()), AgentGrace::default()));
        spawned += 1;
    }
    if spawned > 0 {
        info!("[net] 라이브 주민 스폰 {spawned}개 완료");
    }
}

#[derive(serde::Deserialize)]
struct LookUuid {
    uuid: Option<String>,
}

/// 서버 facing → LPC 시트 행(0=상 1=좌 2=하 3=우). 미지값은 이동 델타의
/// 화면축 우세축, 그것도 없으면 2(하).
fn facing_dir(facing: &str, a: Option<&Agent>) -> usize {
    match facing {
        "down" | "S" => 2,
        "up" | "N" => 0,
        "left" | "W" => 1,
        "right" | "E" => 3,
        _ => {
            if let Some(a) = a {
                let (dx, dy) = (a.tx - a.gx, a.ty - a.gy);
                let (sx, sy) = (dx - dy, dx + dy); // 화면축 우세
                if sx.abs() > sy.abs() {
                    if sx > 0.0 { 3 } else { 1 }
                } else if sy > 0.0 { 2 } else { 0 }
            } else {
                2
            }
        }
    }
}

/// 라이브 주민 이동 — 서버 목적지로 등속 트윈 + 투영·y-sort 갱신 + 걷기 애님.
pub fn live_move(time: Res<Time>, mut q: Query<(&LiveAgentTag, &mut Agent, &mut Transform)>) {
    let dt = time.delta_secs();
    for (_, mut a, mut tr) in &mut q {
        if a.rest <= 0.0 {
            let (dx, dy) = (a.tx - a.gx, a.ty - a.gy);
            let d = (dx * dx + dy * dy).sqrt();
            let step = SPEED * dt;
            if d < 0.05 || d <= step {
                a.gx = a.tx;
                a.gy = a.ty;
                a.rest = 1.0; // 도달 — 다음 패치까지 대기
            } else {
                a.gx += dx / d * step;
                a.gy += dy / d * step;
                a.anim += dt;
            }
        }
        let (px, py) = mapdata::project(a.gx, a.gy);
        tr.translation = Vec3::new(px, -py - mapdata::TH * 0.25, crate::props::z_of(py));
    }
}
