//! 라이브 월드 브리지 — root 클라이언트(src/net) 계약의 2D 축소 미러.
//! 채널: 접속 스레드(네이티브 colyseus-lite) 또는 JS sink(worldsim.js 포크,
//! wasm) → FrameQueue(평면 JSON) → drain_frames → NetState.frame → 적용 시스템들.
//! 관전 티켓(무인증)만 사용 — 채팅 송신 등 계정 경로는 스트레치.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

pub mod apply;
pub mod chat;
pub mod live_props;

/// HTTP 관전 폴러(백로그·관계·생활기록) — 네이티브 전용
#[cfg(not(target_arch = "wasm32"))]
pub mod http_poll;

/// 네이티브 라이브 접속(colyseus-lite) — wasm은 JS sink 경로
#[cfg(not(target_arch = "wasm32"))]
pub mod native_conn;

/// JS·스레드 → ECS 프레임 큐(전역 — Closure·네트워크 스레드 수명이 시스템보다 길다)
#[derive(Resource, Clone, Default)]
pub struct FrameQueue {
    pub inner: Arc<Mutex<Vec<String>>>,
}

/// 룸 채팅 행(스레드/JS → ECS) — 말풍선 소비
#[derive(Debug, Clone)]
pub struct ChatRow {
    pub agent_id: String,
    /// 화자명 — 채팅 로그 UI(egui 단계)에서 소비
    #[allow(dead_code)]
    pub speaker: String,
    pub text: String,
}

#[derive(Resource, Clone, Default)]
pub struct ChatQueue {
    pub inner: Arc<Mutex<Vec<ChatRow>>>,
}

/// 채팅 큐 푸시 헬퍼 — 상한 16행 유지(스레드·폴러 공용)
#[cfg_attr(target_arch = "wasm32", allow(dead_code))] // wasm은 폴러 없음
pub(crate) fn chat_row_push(q: &ChatQueue, row: ChatRow) {
    if row.text.is_empty() {
        return;
    }
    if let Ok(mut q) = q.inner.lock() {
        q.push(row);
        let len = q.len();
        if len > 16 {
            q.drain(0..len - 16);
        }
    }
}

/// 라이브 이벤트 행(스레드 → ECS) — 이벤트 로그 패널 소스
#[derive(Resource, Clone, Default)]
pub struct EventQueue {
    pub inner: Arc<Mutex<Vec<String>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetStatus {
    Idle,
    Connecting,
    Connected,
    Reconnecting,
}

/// 네이티브 접속 스레드가 알리는 링크 단계(원자값 — 시스템이 폴링).
pub const PHASE_CONNECTING: u8 = 1;
pub const PHASE_CONNECTED: u8 = 2;
pub const PHASE_RECONNECTING: u8 = 3;

#[derive(Resource, Clone, Default)]
pub struct NetLinkPhase(pub Arc<AtomicU8>);

#[derive(Resource)]
pub struct NetState {
    pub status: NetStatus,
    /// 최신 프레임(파싱 완료본)
    pub frame: Option<LiveFrame>,
}

impl Default for NetState {
    fn default() -> Self {
        NetState {
            status: NetStatus::Idle,
            frame: None,
        }
    }
}

// ── 프레임 타입 — root src/net/mod.rs와 동일 직렬화 모양(build_frame 호환) ──

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
    #[serde(rename = "aId")]
    pub a_id: String,
    #[serde(rename = "bId")]
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

/// 폴러 큐(스레드 → ECS) — wasm에선 항상 빈 상태(패널이 안내 문구 표시)
#[derive(Resource, Clone, Default)]
pub struct PollQueuesRes {
    pub relations: Arc<Mutex<Vec<RelRow>>>,
    pub history: Arc<Mutex<Vec<HistRow>>>,
}

/// 선택 주민 id 공유(UI → 폴러)
#[derive(Resource, Clone, Default)]
pub struct SelectedAgentRes(pub Arc<Mutex<Option<String>>>);

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameQueue>()
            .init_resource::<ChatQueue>()
            .init_resource::<EventQueue>()
            .init_resource::<PollQueuesRes>()
            .init_resource::<SelectedAgentRes>()
            .init_resource::<NetState>()
            .init_resource::<NetLinkPhase>()
            .add_systems(Startup, connect_bridge)
            .add_systems(
                Update,
                (
                    update_link_phase,
                    drain_frames,
                    apply::prime,
                    apply::sync_daynight,
                    crate::ground::apply_skin,
                    live_props::sync,
                    apply::sync_agents,
                    apply::live_move,
                    live_props::roam_move,
                    chat::drain_and_bubble,
                    drain_events,
                    mirror_selected_agent,
                )
                    .chain(),
            );
    }
}

/// 브리지 기동 — wasm은 JS에 sink 등록 + 접속 요청, 네이티브는 접속 스레드.
#[cfg(target_arch = "wasm32")]
fn connect_bridge(queue: Res<FrameQueue>) {
    if url_live_off() {
        info!("[net] ?live=0 — 라이브 접속 스킵(데모 유지)");
        return;
    }
    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::prelude::*;
    let q = std::sync::Arc::clone(&queue.inner);
    let sink = Closure::<dyn Fn(String)>::new(move |json: String| {
        if let Ok(mut q) = q.lock() {
            q.push(json);
            // 큐 폭주 방지 — 최근 5프레임만
            let len = q.len();
            if len > 5 {
                q.drain(0..len - 5);
            }
        }
    });
    let window = web_sys::window().expect("window");
    let tf = js_sys::Reflect::get(&window, &"terrafolk".into()).expect("terrafolk bridge");
    let set = js_sys::Reflect::get(&tf, &"setSink".into()).expect("setSink");
    let set: js_sys::Function = set.dyn_into().expect("setSink fn");
    let sink_js = sink.as_js_value().clone();
    let _ = set.call1(&tf, &sink_js);
    sink.forget(); // 앱 수명 유지

    let conn = js_sys::Reflect::get(&tf, &"connect".into()).expect("connect");
    let conn: js_sys::Function = conn.dyn_into().expect("connect fn");
    let _ = conn.call0(&tf); // 베이스 URL은 JS가 오리진으로 선택
}

/// ?live=0 — 웹 데모 강제(부팅 스킵은 worldsim.js 포크의 boot()도 같이 검사)
#[cfg(target_arch = "wasm32")]
fn url_live_off() -> bool {
    web_sys::window()
        .and_then(|w| w.location().search().ok())
        .map(|s| s.contains("live=0"))
        .unwrap_or(false)
}

#[cfg(not(target_arch = "wasm32"))]
fn connect_bridge(
    queue: Res<FrameQueue>,
    phase: Res<NetLinkPhase>,
    chats: Res<ChatQueue>,
    events: Res<EventQueue>,
    polls: Res<PollQueuesRes>,
    selected: Res<SelectedAgentRes>,
) {
    if std::env::var("TF_NO_NET").is_ok() {
        info!("[net] TF_NO_NET — 접속 스킵(데모 진단)");
        return;
    }
    let (api, ws) = native_conn::pick_bases();
    native_conn::spawn(queue.clone(), Arc::clone(&phase.0), chats.clone(), events.clone());
    http_poll::spawn(
        Arc::clone(&phase.0),
        chats.clone(),
        api,
        polls.clone(),
        selected.clone(),
    );
    let _ = ws;
}

/// 링크 단계(원자값) → NetState.status. drain_frames가 성공 시 CONNECTED를
/// 원자값에도 기록해 두 방향이 한 값을 바라본다.
fn update_link_phase(phase: Res<NetLinkPhase>, mut net: ResMut<NetState>) {
    let status = match phase.0.load(Ordering::Relaxed) {
        PHASE_CONNECTING => NetStatus::Connecting,
        PHASE_CONNECTED => NetStatus::Connected,
        PHASE_RECONNECTING => NetStatus::Reconnecting,
        _ => NetStatus::Idle,
    };
    if net.status != status {
        info!("[net] 링크 → {status:?}");
        net.status = status;
    }
}

/// 큐 → 채팅 인젤로프는 ChatQueue로, 프레임은 최신 1개만 채택(1s 주기라 충분).
/// 인젤로프 `{tf:"chat", row}` — wasm sink 경로 전용(네이티브는 스레드가
/// ChatQueue에 직접 밀어 넣는다).
fn drain_frames(
    queue: Res<FrameQueue>,
    chats: Res<ChatQueue>,
    events: Res<EventQueue>,
    phase: Res<NetLinkPhase>,
    mut net: ResMut<NetState>,
) {
    let drained: Vec<String> = {
        let Ok(mut q) = queue.inner.lock() else {
            return;
        };
        q.drain(..).collect()
    };
    let mut last_frame: Option<String> = None;
    for s in drained {
        // 인젤로프 판별 — 접두 "{\"tf\""로 시작해야 한다. 예전 s[2..8] 검사는
        // 여는 따옴표가 인덱스 1이라 절대 매치되지 않아 채팅·이벤트가 전부
        // LiveFrame 파서로 새고 스냅숏을 덮어썼다(2026-10-20 실측).
        if s.len() > 12 && s.starts_with("{\"tf\"") {
            // 채팅 인젤로프 — ChatQueue로
            if s.contains("\"chat\"")
                && let Ok(env) = serde_json::from_str::<ChatEnvelope>(&s)
            {
                if let Ok(mut q) = chats.inner.lock() {
                    q.push(env.row.into());
                }
                continue;
            }
            // 이벤트 인젤로프 — EventQueue로(drain_events가 UI 로그로)
            if s.contains("\"event\"")
                && let Ok(env) = serde_json::from_str::<EventEnvelope>(&s)
            {
                if let Ok(mut q) = events.inner.lock() {
                    q.push(format_event(&env.row));
                }
                continue;
            }
        }
        last_frame = Some(s);
    }
    let Some(last) = last_frame else {
        return;
    };
    match serde_json::from_str::<LiveFrame>(&last) {
        Ok(frame) => {
            if net.status != NetStatus::Connected {
                net.status = NetStatus::Connected;
                info!("[net] 라이브 프레임 수신 개시");
            }
            phase.0.store(PHASE_CONNECTED, Ordering::Relaxed);
            net.frame = Some(frame);
        }
        Err(e) => warn!("[net] 프레임 파싱 실패: {e}"),
    }
}

/// wasm sink 채팅 인젤로프(worldsim.js 포크가 송출)
#[derive(Deserialize)]
struct ChatEnvelope {
    row: ChatRowSer,
}

#[derive(Deserialize)]
struct ChatRowSer {
    #[serde(rename = "agentId")]
    agent_id: String,
    speaker: Option<String>,
    text: Option<String>,
}

impl From<ChatRowSer> for ChatRow {
    fn from(r: ChatRowSer) -> Self {
        ChatRow {
            agent_id: r.agent_id,
            speaker: r.speaker.unwrap_or_default(),
            text: r.text.unwrap_or_default(),
        }
    }
}

/// 선택 주민 id를 폴러 공유 변수로 미러(http_poll이 history 대상 결정)
fn mirror_selected_agent(
    selected_q: Query<&apply::LiveAgentTag, With<crate::agents::Selected>>,
    sel: Res<SelectedAgentRes>,
) {
    let id = selected_q.iter().next().map(|t| t.0.clone());
    if let Ok(mut g) = sel.0.lock() {
        *g = id;
    }
}

/// 이벤트 큐 drain — EventLog(UI 패널)로. wasm은 worldsim 포크의
/// {tf:'event'} 인젤로프가 FrameQueue로, 여기 drain_frames와 합류한다.
fn drain_events(queue: Res<EventQueue>, mut log: ResMut<crate::ui::EventLog>) {
    let rows: Vec<String> = {
        let Ok(mut q) = queue.inner.lock() else {
            return;
        };
        q.drain(..).collect()
    };
    for r in rows {
        log.rows.push(r);
    }
    let len = log.rows.len();
    if len > 100 {
        log.rows.drain(0..len - 100);
    }
}

/// wasm 이벤트 인젤로프(worldsim 포크가 송출)
#[derive(Deserialize)]
struct EventEnvelope {
    row: serde_json::Value,
}

/// 이벤트 JSON → 관전 타임라인 문자열(네이티브 native_conn과 동일 서식)
pub(crate) fn format_event(row: &serde_json::Value) -> String {
    let kind = row.get("kind").and_then(|v| v.as_str()).unwrap_or("?");
    let text = row
        .get("payload")
        .and_then(|p| p.get("text").or_else(|| p.get("key")))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let agent = row.get("agent_id").and_then(|v| v.as_str()).unwrap_or("");
    if text.is_empty() {
        return format!("[{kind}]");
    }
    if agent.is_empty() {
        format!("[{kind}] {text}")
    } else {
        format!("[{kind}] {agent}: {text}")
    }
}
