//! 라이브 자원 → 2D 소품 — root src/net/{resources,visual_map}.rs의 투영.
/// 서버 kind/species/stateJson을 기존 프롭 스프라이트로 매핑하고 id 단위로
/// 생성·갱신·제거한다. 동물 배위(§17.64)는 서명 재스폰 없이 RoamTo 트윈.

use bevy::prelude::*;

use super::{LiveResource, NetState, NetStatus};
use crate::agents::SheetLibrary;
use crate::mapdata::{self, project};
use crate::props::{self, Campfire};

/// 라이브 자원 태그(서버 id)
#[derive(Component)]
pub struct LiveResourceTag(pub u32);

/// 자원 상태 서명 — 변화(사망·건축 단계) 감지용. hut은 정체성 필드만
/// (stage·type·w·h — work/progress는 스냅숏마다 변해 깜빡임 유발).
#[derive(Component)]
pub struct LiveResMeta(pub u64);

/// 동물 배위(§17.64) — 현재·목적지 그리드 연속 좌표 쌍. 서버 x/y diff를
/// 재스폰 없이 부드러운 이동으로 (역투영 없이 좌표를 직접 들고 간다).
#[derive(Component)]
pub struct RoamTo {
    pub cur: (f32, f32),
    pub to: (f32, f32),
}

/// 자원 stateJson 최소 파싱(렌더 관련 필드만)
#[derive(Default, serde::Deserialize)]
struct ResState {
    #[serde(default)]
    alive: Option<bool>,
    #[serde(default)]
    aquatic: Option<bool>,
    #[serde(default)]
    stage: Option<String>,
    #[serde(default, rename = "type")]
    bld_type: Option<String>,
    #[serde(default)]
    w: Option<u16>,
    #[serde(default)]
    h: Option<u16>,
    #[serde(default)]
    maturity: Option<u8>,
}

/// 발판 크기 — state.w/h 우선, 없으면 타입 기본값
fn bld_size(st: &ResState, bld_type: &str) -> (u16, u16) {
    if let (Some(w), Some(h)) = (st.w, st.h)
        && w >= 1
        && h >= 1
    {
        return (w, h);
    }
    match bld_type {
        "WINDBREAK" => (1, 1),
        "LONGHOUSE" => (3, 2),
        "WAREHOUSE" => (3, 3),
        _ => (2, 2),
    }
}

fn parse_state(r: &LiveResource) -> ResState {
    r.state
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or_default()
}

fn res_sig(r: &LiveResource, vis: bool) -> u64 {
    let mut h = 1469598103934665603u64;
    let mut feed = String::new();
    if r.kind == "hut" {
        let st = parse_state(r);
        feed.push_str(st.stage.as_deref().unwrap_or(""));
        feed.push('|');
        feed.push_str(st.bld_type.as_deref().unwrap_or(""));
        feed.push_str(&format!("|{}|{}", st.w.unwrap_or(0), st.h.unwrap_or(0)));
    } else {
        feed.push_str(r.state.as_deref().unwrap_or(""));
    }
    for b in r
        .kind
        .bytes()
        .chain(r.species.bytes())
        .chain(feed.bytes())
        .chain(std::iter::once(if vis { 1 } else { 0 }))
    {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

/// 비주얼 결정 — 스프라이트 경로·월드 크기·스케일·틴트. None = 비표시.
struct Visual {
    path: &'static str,
    shadow_w: f32,
    custom: Option<Vec2>,
    scale: f32,
    color: Color,
    animal: bool,
    campfire: bool,
}

fn visual(kind: &str, species: &str, st: &ResState) -> Option<Visual> {
    let v = |path: &'static str, shadow_w: f32| Visual {
        path,
        shadow_w,
        custom: None,
        scale: 1.0,
        color: Color::WHITE,
        animal: false,
        campfire: false,
    };
    match kind {
        // 동물 — 사냥 사망(alive=false)·어류(aquatic)는 비표시. 배위는 RoamTo로.
        "animal" => {
            if st.alive == Some(false) || st.aquatic == Some(true) {
                return None;
            }
            Some(Visual {
                animal: true,
                ..v("props/deer.png", 70.0)
            })
        }
        "hut" => {
            let stage = st.stage.as_deref().unwrap_or("COMPLETED");
            let bld_type = st.bld_type.as_deref().unwrap_or("HUT");
            if stage != "COMPLETED" {
                // 공사 중 — 회색조 실루엣(비계 스프라이트는 스트레치)
                let mut x = v("props/hut.png", 150.0);
                x.color = Color::srgb(0.55, 0.55, 0.55);
                return Some(x);
            }
            match bld_type {
                "WINDBREAK" => Some(v("props/windbreak.png", 90.0)),
                "LONGHOUSE" => Some(Visual {
                    // 생성 스프라이트 종횡비(398×405) 보존 — 발판 3×2 위 비율
                    custom: Some(Vec2::new(288.0, 293.0)),
                    ..v("props/longhouse.png", 170.0)
                }),
                "WAREHOUSE" => Some(Visual {
                    // 398×438 종횡비 보존 — 발판 3×3
                    custom: Some(Vec2::new(344.0, 379.0)),
                    ..v("props/warehouse.png", 200.0)
                }),
                _ => Some(v("props/hut.png", 150.0)),
            }
        }
        "tree" => {
            // 성장 단계 — 0:묘목 1:어린(0.5×) 2:반성장(0.78×) 3:성목
            let m = st.maturity.unwrap_or(3).min(3);
            let path = match m {
                0 => "props/tree_sapling.png",
                _ if species.contains("pine") => "props/tree_pine.png",
                _ => "props/tree_broadleaf.png",
            };
            let scale = match m {
                1 => 0.5,
                2 => 0.78,
                _ => 1.0,
            };
            Some(Visual {
                scale,
                ..v(path, 120.0)
            })
        }
        "plant" => Some(v("props/berry_bush.png", 50.0)),
        "gold_vein" => Some(v("props/gold_vein.png", 80.0)),
        "grave" => Some(v("props/grave.png", 56.0)),
        "stash" => Some(v("props/stash.png", 56.0)),
        "relic" => Some(v("props/relic.png", 70.0)),
        "campfire" => Some(Visual {
            campfire: true,
            ..v("props/campfire_0.png", 58.0)
        }),
        _ => None,
    }
}

/// 자원 동기화 — 신규 스폰 / 사라진 것·상태 변화(사망·건축 단계) 재스폰.
/// 스폰 예산 256/프레임 — 프라임 직후 대량 자원도 프레임 드랍 없이 분산.
pub fn sync(
    mut commands: Commands,
    net: Res<NetState>,
    assets: Res<AssetServer>,
    lib: Option<Res<SheetLibrary>>,
    mut q_live: Query<(
        Entity,
        &LiveResourceTag,
        &LiveResMeta,
        Option<&mut RoamTo>,
    )>,
    mut first_log: Local<u32>,
) {
    let _ = lib; // (주민 시트와 무관 — 시그니처 일관성만)
    if net.status != NetStatus::Connected {
        return;
    }
    let Some(frame) = net.frame.as_ref() else {
        return;
    };
    if *first_log == 0 {
        *first_log = 1;
        info!("[net] 자원 동기 시작 — 프레임 {}건", frame.resources.len());
    }

    // 갱신·제거 — 서명 변화 시 despawn(다음 루프에서 새 시각으로 재스폰).
    // 동물 좌표 이동(§17.64 배위)은 서명과 무관 — RoamTo 목적지만 갱신.
    for (e, tag, meta, roam) in &mut q_live {
        let Some(r) = frame.resources.iter().find(|r| r.id == tag.0) else {
            commands.entity(e).despawn();
            continue;
        };
        let vis = visual(&r.kind, &r.species, &parse_state(r));
        let sig = res_sig(r, vis.is_some());
        if sig != meta.0 {
            commands.entity(e).despawn();
            continue;
        }
        if let (Some(mut roam), true) = (roam, vis.map(|v| v.animal).unwrap_or(false)) {
            roam.to = (r.x as f32, r.y as f32);
        }
    }

    // 신규 — 서버 id 사전 조회(O(1))
    let live_ids: std::collections::HashSet<u32> =
        q_live.iter().map(|(_, t, _, _)| t.0).collect();
    let shadow = assets.load("shadow.png");
    let mut budget = 256usize;
    for r in &frame.resources {
        if budget == 0 {
            break;
        }
        if live_ids.contains(&r.id) {
            continue;
        }
        let Some(v) = visual(&r.kind, &r.species, &parse_state(r)) else {
            continue;
        };
        let sig = res_sig(r, true);
        // 건물 앵커(좌상단) → 렌더 중심(발판 중앙)
        let (gx, gy) = if r.kind == "hut" {
            let st = parse_state(r);
            let (w, h) = bld_size(&st, st.bld_type.as_deref().unwrap_or("HUT"));
            (
                r.x as f32 + (w as f32 - 1.0) / 2.0,
                r.y as f32 + (h as f32 - 1.0) / 2.0,
            )
        } else {
            (r.x as f32, r.y as f32)
        };
        let e = props::spawn_prop_ex(
            &mut commands,
            &shadow,
            assets.load(v.path),
            gx,
            gy,
            v.shadow_w,
            v.custom,
            v.scale,
            v.color,
        );
        commands.entity(e).insert((LiveResourceTag(r.id), LiveResMeta(sig)));
        if v.path.contains("tree_") {
            commands.entity(e).insert(crate::ambient::insert_sway(r.x as i32, r.y as i32));
        }
        if v.animal {
            commands.entity(e).insert(RoamTo {
                cur: (gx, gy),
                to: (r.x as f32, r.y as f32),
            });
        }
        if v.campfire {
            let frames = [
                assets.load("props/campfire_0.png"),
                assets.load("props/campfire_1.png"),
                assets.load("props/campfire_2.png"),
            ];
            commands.entity(e).insert(Campfire { frames });
        }
        budget -= 1;
    }
}

/// 동물 배위 이동(§17.64) — RoamTo.to로 등속 보간 + 진행 방향 좌우 반전.
pub fn roam_move(time: Res<Time>, mut q: Query<(&mut RoamTo, &mut Transform, &mut Sprite)>) {
    const SPEED: f32 = 0.6; // 타일/초 — 배위 동물은 주민보다 느리게
    let dt = time.delta_secs();
    for (mut roam, mut tr, mut s) in &mut q {
        let (dx, dy) = (roam.to.0 - roam.cur.0, roam.to.1 - roam.cur.1);
        let d = (dx * dx + dy * dy).sqrt();
        if d < 1e-4 {
            continue;
        }
        let step = SPEED * dt;
        roam.cur = if d <= step {
            roam.to
        } else {
            (roam.cur.0 + dx / d * step, roam.cur.1 + dy / d * step)
        };
        s.flip_x = dx < 0.0; // 걷는 방향 좌우 반전(사슴 정면 시트)
        let (px, py) = project(roam.cur.0, roam.cur.1);
        tr.translation = Vec3::new(px, -py - mapdata::TH * 0.25, props::z_of(py));
    }
}
