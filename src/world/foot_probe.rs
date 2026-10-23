//! 발 앵커 계측 프로브 — 뒤꿈치 부유 원인 좁히기(2026-10-23 재보고, TF_FOOTDBG=1).
//!
//! chibi_settle이 잰 FootLift와 실제 앵커(발목·발가락·_end)의 현재 월드 y가
//! 발밑 표면고도와 얼마나 어긋나는지 주기 로그한다. 갭 > 0 = 부유(발이 뜸),
//! < 0 = 침몰(지형에 박힘). ground_snap 불변식(root.y == surf−lift) 잔차로
//! 스냅 미적용(FootLift 누락 등)도 잡는다. 앵커는 뼈라 메시 밑창 두께(δ)는
//! 별도 오프라인 GLB 분석으로 보정 판단한다.

use std::sync::OnceLock;

use bevy::prelude::*;

use crate::character::FootLift;
use super::data::WorldData;
use super::ground::WATER_STAND_Y;
use super::villager::{Villager, VillagerAnims};

/// 측정 앵커 — chibi_settle과 동일 목록(양 리그).
const ANCHORS: [&str; 12] = [
    "Foot.L",
    "Foot.R",
    "foot_l",
    "foot_r",
    "PT.L",
    "PT.R",
    "toe_l",
    "toe_r",
    "Foot.L_end",
    "Foot.R_end",
    "PT.L_end",
    "PT.R_end",
];

/// 앵커 표시명 — 리그 접미사 제거(L/R·_end 보존).
fn tag(n: &str) -> String {
    n.trim_start_matches("foot_")
        .trim_start_matches("toe_")
        .trim_start_matches("Foot.")
        .trim_start_matches("PT.")
        .to_owned()
}

fn enabled() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| std::env::var("TF_FOOTDBG").is_ok())
}

/// 진단 전용 프레이밍 — 첫 주민을 최대 줌(ZOOM_MIN)으로 추적. 발 포즈를
/// 인앱 스크린샷(TF_SHOT)에 잡기 위한 것으로, 운영 입력이 없는 진동 실행에서만.
pub fn footdbg_camera(
    mut rig: ResMut<super::camera_rig::CameraRig>,
    q: Query<&Transform, With<Villager>>,
) {
    if !enabled() {
        return;
    }
    if let Some(tr) = q.iter().next() {
        rig.target = Vec2::new(tr.translation.x, tr.translation.z);
        rig.zoom_goal = super::camera_rig::ZOOM_MIN;
    }
}

/// 1초마다 1프레임 샘플, 60회(≈60초) 후 자동 종료 — 로그 홍수 방지.
pub fn foot_probe(
    time: Res<Time>,
    world: Res<WorldData>,
    q: Query<(Entity, &Transform, &FootLift, &Villager, &VillagerAnims)>,
    children_q: Query<&Children>,
    names: Query<&Name>,
    gts: Query<&GlobalTransform>,
    mut next: Local<f32>,
    mut count: Local<u32>,
) {
    if !enabled() || *count >= 60 {
        return;
    }
    let now = time.elapsed_secs();
    if now < *next {
        return;
    }
    *next = now + 1.0;
    *count += 1;

    for (e, tr, lift, v, anims) in &q {
        let surf_c = world
            .surface_height(tr.translation.x, tr.translation.z)
            .max(WATER_STAND_Y);
        // ground_snap 불변식 잔차 — 0이어야 정상(스냅 살아있음)
        let resid = tr.translation.y - (surf_c - lift.0);
        // 앵커별 갭(앵커 월드 y − 앵커 발밑 표면고도) — 부유/침몰 방향 성립
        let mut line = String::new();
        let mut min_gap = f32::INFINITY;
        for d in children_q.iter_descendants(e) {
            let Ok(name) = names.get(d) else { continue };
            let n = name.as_str();
            if !ANCHORS.contains(&n) {
                continue;
            }
            let Ok(gt) = gts.get(d) else {
                continue;
            };
            let p = gt.translation();
            let surf = world.surface_height(p.x, p.z).max(WATER_STAND_Y);
            let gap = p.y - surf;
            min_gap = min_gap.min(gap);
            line.push_str(&format!(" {}={:+.3}", tag(n), gap));
        }
        info!(
            "[footdbg#{:02}] {} {:?} mov={} lift={:.4} surfC={:.4} resid={:+.4} minGap={:+.3} |{}",
            *count,
            v.name,
            anims.current,
            v.moving,
            lift.0,
            surf_c,
            resid,
            min_gap,
            line
        );
    }
}
