//! 성능 프로브 — `?tfdbg=perf` (게이트 꺼짐=리소스 미삽입 → 모든 시스템 run_if로 비용 0).
//! 2초마다 `[perf] {json}` 한 줄: FPS·프레임 p50/p95(직전 2초)·엔티티/가시 메시/스킨/잉크 헐 수·
//! 구간별 평균 ms(pre=First+PreUpdate(W2 재전파 포함), update, post_pre=PostUpdate 시작~발IK·손 앵커 끝,
//! warp=apply_warp, post_rest=전파·가시성 등, rest=Main 이후 렌더 추출+브라우저 대기).
//! 구간은 스케줄 경계 스탬프 시스템으로 잰다(wasm 단일 스레드라 시스템 간 병렬 겹침 없음).

use bevy::mesh::skinning::SkinnedMesh;
use bevy::platform::time::Instant;
use bevy::prelude::*;

use crate::character::ink::InkMaterial;

/// 스탬프 수: 0 First · 1 PreUpdate 끝 · 2 SpawnScene(=Update 끝) · 3 발IK·앵커 끝 · 4 워프 전 · 5 워프 후 · 6 Last
const N: usize = 7;

#[derive(Resource)]
pub struct PerfClock {
    t: [Option<Instant>; N],
    /// 구간 누적(초) — pre, update, post_pre, warp, post_rest, main_total
    acc: [f64; 6],
    frames: u32,
    dts: Vec<f32>,
    since_log: f32,
}

impl Default for PerfClock {
    fn default() -> Self {
        Self { t: [None; N], acc: [0.0; 6], frames: 0, dts: Vec::new(), since_log: 0.0 }
    }
}

pub fn enabled() -> bool {
    crate::net::dbg_gate::on("perf")
}

pub fn perf_on(c: Option<Res<PerfClock>>) -> bool {
    c.is_some()
}

pub fn stamp<const I: usize>(mut c: ResMut<PerfClock>) {
    c.t[I] = Some(Instant::now());
}

fn span(c: &PerfClock, a: usize, b: usize) -> f64 {
    match (c.t[a], c.t[b]) {
        (Some(x), Some(y)) => y.saturating_duration_since(x).as_secs_f64(),
        _ => 0.0,
    }
}

#[allow(clippy::type_complexity, clippy::too_many_arguments)] // 프로브 카운터 쿼리 모음
pub fn report(
    mut c: ResMut<PerfClock>,
    time: Res<Time<Real>>,
    ents: Query<Entity>,
    vis: Query<&ViewVisibility, With<Mesh3d>>,
    skins: Query<(), With<SkinnedMesh>>,
    inks: Query<(), With<MeshMaterial3d<InkMaterial>>>,
    props: Query<(), With<super::models::PropRoot>>,
    statics: Query<(), With<super::globe_static::GlobeStatic>>,
    conc: Query<(), With<super::occlusion::Concealer>>,
) {
    c.t[6] = Some(Instant::now());
    let spans = [
        span(&c, 0, 1),
        span(&c, 1, 2),
        span(&c, 2, 3),
        span(&c, 4, 5),
        span(&c, 5, 6),
        span(&c, 0, 6),
    ];
    for (a, s) in c.acc.iter_mut().zip(spans) {
        *a += s;
    }
    c.frames += 1;
    let dt = time.delta_secs();
    c.dts.push(dt * 1000.0);
    c.since_log += dt;
    if c.since_log < 2.0 {
        return;
    }
    let mut d = std::mem::take(&mut c.dts);
    d.sort_by(|a, b| a.total_cmp(b));
    let pct = |p: f32| d.get(((d.len() as f32 * p) as usize).min(d.len().saturating_sub(1))).copied().unwrap_or(0.0);
    let n = c.frames.max(1) as f64;
    let ms = |i: usize| c.acc[i] / n * 1000.0;
    let main = ms(5);
    let fps = c.frames as f32 / c.since_log;
    let frame_avg = c.since_log as f64 / n * 1000.0;
    info!(
        "[perf] {{\"fps\":{fps:.1},\"p50\":{:.1},\"p95\":{:.1},\"ents\":{},\"vis_mesh\":{},\"skins\":{},\"ink\":{},\"props\":{},\"gstat\":{},\"conc\":{},\"pre\":{:.2},\"update\":{:.2},\"post_pre\":{:.2},\"warp\":{:.2},\"post_rest\":{:.2},\"main\":{main:.2},\"rest\":{:.2}}}",
        pct(0.5),
        pct(0.95),
        ents.iter().count(),
        vis.iter().filter(|v| v.get()).count(),
        skins.iter().count(),
        inks.iter().count(),
        props.iter().count(),
        statics.iter().count(),
        conc.iter().count(),
        ms(0),
        ms(1),
        ms(2),
        ms(3),
        ms(4),
        (frame_avg - main).max(0.0),
    );
    c.acc = [0.0; 6];
    c.frames = 0;
    c.since_log = 0.0;
}
