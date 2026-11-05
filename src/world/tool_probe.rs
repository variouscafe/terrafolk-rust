//! 손 도구 월드 크기 진단 — `?tfdbg=toolprobe` (평시 비용 0).
//! 크레이트 계약 T1 검증: 부품 월드 스케일 ≈ ToolScale(주민 배율),
//! 부품-액터 루트 거리 ≲ 2m. 120프레임마다 1줄 로그.

use bevy::prelude::*;

use crate::character::tool::ToolPartOf;

pub fn tool_probe(
    mut frame: Local<u32>,
    parts: Query<(&GlobalTransform, &ToolPartOf)>,
    gts: Query<&GlobalTransform>,
) {
    if !crate::net::dbg_gate::on("toolprobe") {
        return;
    }
    *frame += 1;
    if !(*frame).is_multiple_of(120) {
        return;
    }
    let (mut n, mut max_scale, mut max_dist) = (0usize, 0.0f32, 0.0f32);
    for (gt, of) in &parts {
        n += 1;
        let (s, _, t) = gt.to_scale_rotation_translation();
        max_scale = max_scale.max(s.max_element());
        if let Ok(actor) = gts.get(of.0) {
            max_dist = max_dist.max(t.distance(actor.translation()));
        }
    }
    if n == 0 {
        info!("[tool] probe parts=0");
    } else {
        info!("[tool] probe parts={n} max_scale={max_scale:.2} max_dist={max_dist:.2}m");
    }
}
