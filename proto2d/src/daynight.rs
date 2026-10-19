//! 주야 — 게임 시각 자원. 데모는 하루 150초 로컬 진행, 라이브는 서버 정본.
//! 풀스크린 틴트 노드는 제거(2026-10-20) — egui 전환으로 bevy_ui 피처를 뗐고,
//! 야간 그레이딩은 향후 월드 스프라이트 팔레트/카메라 틴트로 이관 예정(스트레치).
use bevy::prelude::*;

#[derive(Resource)]
pub struct GameTime {
    pub day: u32,
    pub t: f32, // 0.0=자정 … 0.5=정오
}

pub const DAY_SECS: f32 = 150.0;

pub fn spawn(_commands: Commands) {}

pub fn advance(
    time: Res<Time>,
    net: Option<Res<crate::net::NetState>>,
    mut gt: ResMut<GameTime>,
) {
    // 라이브 접속 중은 서버가 시간 정본 — 로컬 진행 정지(net/apply::sync_daynight)
    if let Some(net) = net
        && net.status == crate::net::NetStatus::Connected
    {
        return;
    }
    gt.t += time.delta_secs() / DAY_SECS;
    if gt.t >= 1.0 {
        gt.t -= 1.0;
        gt.day += 1;
    }
}
