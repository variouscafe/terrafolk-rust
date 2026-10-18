//! 라이브 주민 표시 상태(AgentHud) — 룸 스키마의 진행률·목표·욕구를
//! 오버레이·선택 패널이 읽는 단일 사본. sync_agents와 분리해 apply.rs
//! 500줄 규칙을 지킨다. 데모 주민은 미부착 → 표시 계층이 자동 라이브 전용.

use bevy::prelude::*;

use super::{NetState, NetStatus};
use super::apply::LiveAgentTag;

/// 주민별 관전 데이터(1s 프레임 주기 갱신).
#[derive(Component, Default, Clone)]
pub struct AgentHud {
    /// 진행 중 행동 종류(chop|mine|... — 비활동 시 빈 문자열)
    pub act_type: String,
    /// 행동 진행률 0..1(머리 위 링)
    pub act_progress: f32,
    /// 목표 — 영문 정본(표시는 key 렌더 우선, 이건 폴백)
    pub goal_text: String,
    /// 목표 폴백 키(goal.*) — 네이티브 렌더 우선 경로
    pub goal_key: String,
    /// 욕구 — hunger, thirst, fatigue, warmth(0..100)
    pub needs: [f32; 4],
}

/// 프레임 → AgentHud 동기(스폰은 sync_agents가 기본값으로 부착).
pub fn sync_agent_hud(net: Res<NetState>, mut q: Query<(&LiveAgentTag, &mut AgentHud)>) {
    if net.status != NetStatus::Connected {
        return;
    }
    let Some(frame) = net.frame.as_ref() else {
        return;
    };
    for (tag, mut hud) in &mut q {
        let Some(a) = frame.agents.iter().find(|a| a.id == tag.0) else {
            continue;
        };
        hud.act_progress = a.act_progress.unwrap_or(0.0);
        hud.act_type = a.act_type.clone().unwrap_or_default();
        hud.goal_text = a.goal_text.clone().unwrap_or_default();
        hud.goal_key = a.goal_key.clone().unwrap_or_default();
        hud.needs = [
            a.hunger.unwrap_or(0.0),
            a.thirst.unwrap_or(0.0),
            a.fatigue.unwrap_or(0.0),
            a.warmth.unwrap_or(0.0),
        ];
    }
}
