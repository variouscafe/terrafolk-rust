//! 중력 낙하 프로브 — `?tfdbg=drop`(URL 게이트, 웹 전용 플래그 소스).
//!
//! 프라임 후 주민 1명을 공중 3m로 띄워 지구 중력 적분을 실렌더로 판정한다.
//! 0.1s 단위 궤적 로그([grav] t=… y=… vy= …)로 포물선 수치(2m→0.64s·
//! √(2gh)) 확인, 화면 캡처로 낙하 실루엣 확인. 지면에 닿는 순간의 착지는
//! ground_snap의 [grav] 낙하 착지 로그가 담당.

use bevy::prelude::*;

use crate::character::{Fall, GroundActor};

/// 리프트 높이(m) — 충격 v=√(2·9.81·3)≈7.7m/s, 낙하 t≈0.78s.
const LIFT: f32 = 3.0;
/// 리프트 시점 — 첫 주민 스폰 관측 후 모델 안착 대기(s).
const ARM_DELAY: f32 = 1.5;
/// 궤적 로그 주기(s).
const LOG_DT: f32 = 0.1;

/// 상태 — None=대기(첫 주민 관측), Some(t)=리프트 후 경과.
#[derive(Default)]
pub struct Arm {
    seen_at: Option<f32>,
    lifted: bool,
    lifted_at: f32,
    next_log: f32,
}

pub fn drop_probe(
    time: Res<Time>,
    mut arm: Local<Arm>,
    mut q: Query<(&mut Transform, &mut Fall), With<GroundActor>>,
) {
    if !crate::net::dbg_gate::on("drop") {
        return;
    }
    let now = time.elapsed_secs();
    if arm.seen_at.is_none() {
        if q.iter().next().is_some() {
            arm.seen_at = Some(now);
        }
        return;
    }
    if !arm.lifted {
        // seen_at은 위 is_none 조기반환이 Some을 보장 — 린트 벨트 경유 해체
        if let Some(seen) = arm.seen_at
            && now - seen < ARM_DELAY
        {
            return;
        }
        if let Some((mut tr, mut fall)) = q.iter_mut().next() {
            tr.translation.y += LIFT;
            fall.airborne = true;
            fall.vy = 0.0;
            arm.lifted = true;
            arm.lifted_at = now;
            arm.next_log = now;
            info!("[grav] 프로브 리프트 +{LIFT}m — 자유낙하 시작");
        }
        return;
    }
    // 궤적 샘플 — 적분은 ground_snap이 수행(프로브는 읽기만).
    if now >= arm.next_log && now - arm.lifted_at < 4.0 {
        arm.next_log = now + LOG_DT;
        if let Some((tr, fall)) = q.iter().next() {
            info!(
                "[grav] t={:.2} y={:.2} vy={:.2} 공중={}",
                now - arm.lifted_at,
                tr.translation.y,
                fall.vy,
                fall.airborne
            );
        }
    }
}
