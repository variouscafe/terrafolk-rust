//! 라이브 계약 자료형 — root src/net 계약의 2D 미러(build_frame 호환
//! 직렬화 모양). net/mod.rs 500줄 규칙 분리(2026-10-20).

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};

/// 구조화 이벤트 행 — payload 그대로 보관(표시 언어 전환 시 재렌더·
/// 하이라이트 판정 모두 payload 기반이라 미리 문장을 구울 필요 없다)
#[derive(Debug, Clone)]
pub struct RawEvent {
    pub kind: String,
    pub agent_id: String,
    pub payload: serde_json::Value,
    pub ts: Option<String>,
    pub tick: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LiveAgent {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub x: f32,
    pub y: f32,
    pub facing: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act_type: Option<String>,
    #[serde(rename = "actType", skip_serializing_if = "Option::is_none")]
    pub _act_type_r: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub act_progress: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal_text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub goal_key: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hunger: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thirst: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub fatigue: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub warmth: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inv_json: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LiveResource {
    pub id: u32,
    pub kind: String,
    pub species: String,
    pub x: u16,
    pub y: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LiveFrame {
    pub tick: u32,
    pub day: u16,
    pub hour: u16,
    pub minute: u16,
    pub season: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weather: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sunrise: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sunset: Option<u16>,
    /// Tiled JSON — 최초 1회(fullStateOnly)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map: Option<String>,
    pub agents: Vec<LiveAgent>,
    pub resources: Vec<LiveResource>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventions: Option<String>,
}

/// Tiled mapJson 최소 파싱 — layers[0].data(GID 배열)·width·height.
#[derive(Debug, Deserialize)]
pub struct TiledMap {
    pub width: usize,
    pub height: usize,
    pub layers: Vec<TiledLayer>,
}

#[derive(Debug, Deserialize)]
pub struct TiledLayer {
    pub data: Vec<u8>,
}

/// 관계 행(REST /relations — 폴러가 채움)
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RelRow {
    /// REST /relations는 a_id(스네이크)로 내려온다 — 룸 방송은 aId(카멜) 폴백
    #[serde(alias = "aId")]
    pub a_id: String,
    #[serde(alias = "bId")]
    pub b_id: String,
    #[serde(default)]
    pub warmth: f32,
    #[serde(default)]
    pub chats: u32,
    #[serde(default)]
    pub trades: u32,
}

/// 생활기록 행(REST /agents/{id}/history)
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HistRow {
    /// 서버 타임스탬프 — 표시 형식화 예약
    #[serde(default)]
    #[allow(dead_code)]
    pub ts: String,
    #[serde(rename = "type", default)]
    pub type_: String,
    #[serde(default)]
    pub text: String,
}

/// 폴러 큐(스레드 → ECS) — wasm은 JS 폴러(worldsim 포크)가 같은 모양으로 채운다
#[derive(Resource, Clone, Default)]
pub struct PollQueuesRes {
    pub relations: Arc<Mutex<Vec<RelRow>>>,
    pub history: Arc<Mutex<Vec<HistRow>>>,
}
