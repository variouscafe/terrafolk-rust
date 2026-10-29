//! 날씨 파티클(네이티브 전용) — 비·폭풍(3배)·눈 + 번개 섬광(FE weatherfx.ts
//! 직역). 웹은 DOM/CSS(tf-ambience.js)가 담당하므로 cfg로 제외해 이중 구현을
//! 피한다. 파티클은 엔티티 풀(공유 메시·unlit 블렌드 재질) — 시야 영역
//! (rig.target 중심 view_box×1.5) 안에서 떨어지고 지면 도달·이탈 시 재롤.
//! 안개·흐림 톤은 기존 sky_mix→apply_sky가 담당(구현 완료 상태).
//!
//! **중력 낙하(2026-10-24)**: 낙하 속도는 상수가 아니라 종단속도로 수렴하는
//! 적분(비 g=25·항력 큼, 눈 g=3.8·부유폭 넓음 — 재롤 직후 저속 가속이
/// 보인다). 지면 접촉은 y=0 근사가 아니라 **표면장(WorldData.surface_height)**
/// — 언덕에 파묻히거나 수면을 뚫지 않는다. 눈은 착지 후 잠시 체류(퇴적감) 후
/// 재롤.

use bevy::math::primitives::Rectangle;
use bevy::picking::Pickable;
use bevy::prelude::*;

use super::camera_rig::CameraRig;
use super::data::{Rng, WorldData};
use super::agent::WATER_STAND_Y;
use super::overlay::view_box;
use super::precip::WeatherState;
use super::propsfx::CAM_FACE;

/// 파티클 수 — FE weatherfx 밀도 근사(비 220·폭풍 3배·눈 160).
const COUNT_RAIN: usize = 220;
const COUNT_STORM: usize = 660;
const COUNT_SNOW: usize = 160;
/// 재롤 높이 범위(m).
const TOP_MIN: f32 = 10.0;
const TOP_MAX: f32 = 13.0;
/// 중력·항력 (m/s², 1/s) — vy' = g − k·vy → 종단속도 g/k.
const RAIN_G: f32 = 25.0;
const RAIN_DRAG: f32 = 2.6; // 종단 ≈ 9.6m/s
const SNOW_G: f32 = 3.8;
const SNOW_DRAG: f32 = 2.4; // 종단 ≈ 1.6m/s
/// 눈 착지 체류(s) — 퇴적감.
const SNOW_REST: (f32, f32) = (0.6, 1.8);

#[derive(Component)]
pub struct PrecipDrop {
    vy: f32,
    sway: f32,
    phase: f32,
    snow: bool,
    /// 착지 눈의 남은 체류(s) — None이면 낙하 중.
    rest: Option<f32>,
}

#[derive(Component)]
pub struct LightningFlash;

#[derive(Resource)]
pub struct PrecipState {
    kind: String,
    rain_mesh: Handle<Mesh>,
    snow_mesh: Handle<Mesh>,
    rain_mat: Handle<StandardMaterial>,
    snow_mat: Handle<StandardMaterial>,
    /// 다음 번개 시각(초 — storm에서만).
    next_strike: f32,
    flash_alpha: f32,
    /// 파티클 난수(비결정 — 시드는 부팅 시각).
    rng: Rng,
}

/// 착지 판정 지면 고도 — 표면장(불투명 수면 위 클램프).
fn ground_at(world: &WorldData, x: f32, z: f32) -> f32 {
    world.surface_height(x, z).max(WATER_STAND_Y)
}

/// 플래시 노드 스폰(부팅 1회) + 상태 리소스 — 전면 커버, 픽킹 제외.
pub fn spawn_lightning_flash(mut commands: Commands) {
    commands.spawn((
        LightningFlash,
        Pickable::IGNORE,
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.918, 0.949, 1.0, 0.0)),
    ));
    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9e3779b9);
    commands.insert_resource(PrecipState {
        kind: String::new(),
        rain_mesh: Handle::default(),
        snow_mesh: Handle::default(),
        rain_mat: Handle::default(),
        snow_mat: Handle::default(),
        next_strike: 0.0,
        flash_alpha: 0.0,
        rng: Rng::new(seed),
    });
}

/// 날씨 변화 → 풀 크기 조정. 메시·재질은 1회 생성해 공유(파티클 다수가
/// 같은 핸들을 가리킨다 — 배칭 유리). 카메라 이동 후 빈 하늘은 update_precip의
/// 재롤이 메운다.
pub fn adjust_pool(
    mut commands: Commands,
    weather: Res<WeatherState>,
    rig: Res<CameraRig>,
    window: Query<&Window>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut st: ResMut<PrecipState>,
    q_drops: Query<Entity, With<PrecipDrop>>,
) {
    if weather.kind == st.kind {
        return; // 날씨 무변화 — 풀 유지
    }
    let Ok(window) = window.single() else {
        return;
    };
    let aspect = window.width() / window.height().max(1.0);
    // 메시·재질 1회 생성 — 빗줄기(가늘고 긴 쿼드)·눈송이(작은 정사각).
    if st.rain_mesh.id() == bevy::asset::AssetId::invalid() {
        st.rain_mesh = meshes.add(Rectangle::new(0.02, 0.55));
        st.snow_mesh = meshes.add(Rectangle::new(0.07, 0.07));
        st.rain_mat = mats.add(StandardMaterial {
            base_color: Color::srgba(0.62, 0.72, 0.90, 0.45),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        st.snow_mat = mats.add(StandardMaterial {
            base_color: Color::srgba(0.95, 0.97, 1.0, 0.85),
            unlit: true,
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
    }
    let prev_kind = st.kind.clone();
    st.kind = weather.kind.clone();
    let (target, snow) = match weather.kind.as_str() {
        "rain" => (COUNT_RAIN, false),
        "storm" => (COUNT_STORM, false),
        "snow" => (COUNT_SNOW, true),
        _ => (0, false),
    };
    let existing: Vec<Entity> = q_drops.iter().collect();
    // 종 전환(비↔눈)은 성분이 달라 전량 재스폰 — 같은 종 증감만 부분 조정.
    let reuse = if prev_kind.is_empty()
        || matches!(prev_kind.as_str(), "rain" | "storm") == matches!(weather.kind.as_str(), "rain" | "storm")
    {
        existing.len()
    } else {
        for &e in &existing {
            commands.entity(e).despawn();
        }
        0
    };
    if reuse > target {
        for &e in &existing[target..] {
            commands.entity(e).despawn();
        }
    }
    let spawn_from = reuse.min(target);
    let (mesh, mat) = if snow {
        (st.snow_mesh.clone(), st.snow_mat.clone())
    } else {
        (st.rain_mesh.clone(), st.rain_mat.clone())
    };
    let (vw, vh) = view_box(rig.zoom, aspect);
    let (cx, cz) = (rig.target.x, rig.target.y);
    for _ in spawn_from..target {
        let (x, z, y) = roll_pos(&mut st.rng, cx, cz, vw, vh);
        let mut tr = Transform::from_translation(Vec3::new(x, y, z));
        tr.look_at(tr.translation - CAM_FACE, Vec3::Y); // 카메라를 향한 쿼드
        let drop = if snow {
            PrecipDrop {
                // 재롤 직후 저속 — 중력 가속으로 종단속도 수렴(가시 가속감).
                vy: st.rng.range(0.4, 0.9),
                sway: st.rng.range(0.4, 0.9),
                phase: st.rng.range(0.0, std::f32::consts::TAU),
                snow: true,
                rest: None,
            }
        } else {
            PrecipDrop {
                vy: st.rng.range(4.0, 7.0),
                sway: 0.0,
                phase: 0.0,
                snow: false,
                rest: None,
            }
        };
        commands.spawn((drop, Mesh3d(mesh.clone()), MeshMaterial3d(mat.clone()), tr));
    }
}

fn roll_pos(rng: &mut Rng, cx: f32, cz: f32, vw: f32, vh: f32) -> (f32, f32, f32) {
    (
        cx + rng.range(-0.75, 0.75) * vw,
        cz + rng.range(-0.75, 0.75) * vh,
        rng.range(TOP_MIN, TOP_MAX),
    )
}

/// 중력 낙하·바람·스웨이 — 표면장 지면 도달(눈은 체류) 또는 시야 이탈 시
/// 영역 안 재롤.
pub fn update_precip(
    time: Res<Time>,
    world: Res<WorldData>,
    rig: Res<CameraRig>,
    window: Query<&Window>,
    mut st: ResMut<PrecipState>,
    mut q: Query<(&mut PrecipDrop, &mut Transform)>,
) {
    let dt = time.delta_secs();
    if dt <= 0.0 {
        return;
    }
    let Ok(window) = window.single() else {
        return;
    };
    let aspect = window.width() / window.height().max(1.0);
    let (vw, vh) = view_box(rig.zoom, aspect);
    let (cx, cz) = (rig.target.x, rig.target.y);
    let t = time.elapsed_secs();
    for (mut drop, mut tr) in &mut q {
        // 착지 눈 — 체류 감쇠 후 재롤(스웨이만 미세 유지).
        if let Some(r) = drop.rest {
            drop.rest = Some(r - dt);
            if r > 0.0 {
                continue;
            }
            let (x, z, y) = roll_pos(&mut st.rng, cx, cz, vw, vh);
            tr.translation = Vec3::new(x, y, z);
            drop.vy = st.rng.range(0.4, 0.9);
            drop.rest = None;
            continue;
        }
        // 중력 적분 — vy' = g − k·vy (종단속도 수렴).
        let (g, k) = if drop.snow {
            (SNOW_G, SNOW_DRAG)
        } else {
            (RAIN_G, RAIN_DRAG)
        };
        drop.vy += (g - k * drop.vy) * dt;
        tr.translation.y -= drop.vy * dt;
        if drop.snow {
            // 눈 — 좌우 스웨이(부유)
            tr.translation.x += (t + drop.phase).sin() * drop.sway * dt;
            tr.translation.z += (t * 0.7 + drop.phase).cos() * drop.sway * 0.6 * dt;
        } else {
            // 비 — 바람 드리프트(고정 방향, 속도 비례)
            tr.translation.x -= 0.35 * drop.vy * dt;
            tr.translation.z -= 0.35 * drop.vy * dt;
        }
        let gy = ground_at(&world, tr.translation.x, tr.translation.z);
        let out = tr.translation.y < gy || (tr.translation.x - cx).abs() > vw || (tr.translation.z - cz).abs() > vh;
        if out {
            if drop.snow && tr.translation.y < gy {
                // 눈 — 표면에 얹혀 체류(퇴적감) 후 재롤.
                tr.translation.y = gy;
                drop.rest = Some(st.rng.range(SNOW_REST.0, SNOW_REST.1));
                continue;
            }
            let (x, z, y) = roll_pos(&mut st.rng, cx, cz, vw, vh);
            tr.translation = Vec3::new(x, y, z);
            drop.vy = if drop.snow {
                st.rng.range(0.4, 0.9)
            } else {
                st.rng.range(4.0, 7.0)
            };
        }
    }
}

/// 번개 — storm에서만 4~9s 무작위 섬광(30% 이중 타격). 풀스크린 플래시를
/// 0.3s 지수 감쇠(FE weatherfx 직역 근사).
pub fn update_lightning(
    time: Res<Time>,
    weather: Res<WeatherState>,
    mut st: ResMut<PrecipState>,
    mut q_flash: Query<&mut BackgroundColor, With<LightningFlash>>,
) {
    let t = time.elapsed_secs();
    if weather.kind != "storm" {
        st.flash_alpha = 0.0;
        st.next_strike = t + 4.0;
    } else if t >= st.next_strike {
        st.flash_alpha = 0.55;
        // 이중 타격 30% — 짧은 후속
        st.next_strike = t
            + if st.rng.f32() < 0.3 {
                0.12
            } else {
                st.rng.range(4.0, 9.0)
            };
    }
    if st.flash_alpha > 0.0 {
        st.flash_alpha *= (-7.0 * time.delta_secs()).exp();
        if st.flash_alpha < 0.01 {
            st.flash_alpha = 0.0;
        }
    }
    if let Ok(mut bg) = q_flash.single_mut() {
        bg.0 = Color::srgba(0.918, 0.949, 1.0, st.flash_alpha);
    }
}
