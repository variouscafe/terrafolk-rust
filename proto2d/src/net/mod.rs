//! 라이브 월드 브리지 — root 클라이언트(src/net) 계약의 2D 축소 미러.
//! 채널: 접속 스레드(네이티브 colyseus-lite) 또는 JS sink(worldsim.js 포크,
//! wasm) → FrameQueue(평면 JSON) → drain_frames → NetState.frame → 적용 시스템들.
//! 관전 티켓(무인증)만 사용 — 채팅 송신 등 계정 경로는 스트레치.

use bevy::prelude::*;
use serde::Deserialize;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

pub mod apply;
pub mod chat;
pub mod eco;
pub mod live_props;
pub mod types;

pub use eco::{EcoData, EcoQueue, eco_from_json};
pub use types::{HistRow, LiveAgent, LiveFrame, LiveResource, PollQueuesRes, RawEvent, RelRow, TiledMap};

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
    pub speaker: String,
    pub text: String,
    /// 발화 게임 분(절대) — 시각 스탬프(FE fmtGmin 패리티). 백로그는 ts로 폴백
    pub gmin: Option<i64>,
    pub ts: Option<String>,
}

#[derive(Resource, Clone, Default)]
pub struct ChatQueue {
    pub inner: Arc<Mutex<Vec<ChatRow>>>,
}

/// 채팅 큐 푸시 헬퍼 — 상한 120행 유지(스레드·폴러 공용 — 백로그 시딩 여유)
#[cfg_attr(target_arch = "wasm32", allow(dead_code))] // wasm은 폴러 없음
pub(crate) fn chat_row_push(q: &ChatQueue, row: ChatRow) {
    if row.text.is_empty() {
        return;
    }
    if let Ok(mut q) = q.inner.lock() {
        q.push(row);
        let len = q.len();
        if len > 120 {
            q.drain(0..len - 120);
        }
    }
}

/// 라이브 이벤트 원시 행(JSON 문자열 — 룸 방송·REST 백로그 동일 모양).
/// drain_events가 EventRow로 파싱해 UI로 넘긴다.
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
            .init_resource::<EcoQueue>()
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
    eco: Res<EcoQueue>,
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
        events.clone(),
        eco.clone(),
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
    polls: Res<PollQueuesRes>,
    eco: Res<EcoQueue>,
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
        // 인벤로프 — {"tf": kind, ...} 통일 모양. kind로 정밀 분기한다
        // (옛 부분문자열 contains("chat") 판별은 kind:'chat' 이벤트를 채팅으로
        // 오라우팅해 삼켰다 — 이벤트 종 'chat'은 계약이 예약한 어휘다).
        if s.starts_with("{\"tf\"") {
            let kind = serde_json::from_str::<TfKind>(&s).ok().map(|k| k.tf);
            match kind.as_deref() {
                Some("chat") => {
                    if let Ok(env) = serde_json::from_str::<ChatEnvelope>(&s)
                        && let Ok(mut q) = chats.inner.lock()
                    {
                        q.push(env.row.into());
                    }
                    continue;
                }
                Some("event") => {
                    if let Ok(env) = serde_json::from_str::<EventEnvelope>(&s)
                        && let Ok(mut q) = events.inner.lock()
                    {
                        q.push(env.row.to_string());
                        let len = q.len();
                        if len > 400 {
                            q.drain(0..len - 400);
                        }
                    }
                    continue;
                }
                Some(_) => {
                    // REST 폴 인젤로프(웹 JS 폴러) — 관계·생활기록·경제·채팅 백로그
                    if poll_envelope(&s, &chats, &polls, &eco) == Some(true) {
                        continue;
                    }
                }
                None => {}
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

/// 인벤로프 종류 판별 전용 최소 파싱 — {"tf":"chat"|...}
#[derive(Deserialize)]
struct TfKind {
    tf: String,
}

/// wasm sink 채팅 인젤로프(worldsim.js 포크가 송출)
#[derive(Deserialize)]
struct ChatEnvelope {
    row: ChatRowSer,
}

#[derive(Deserialize)]
struct ChatRowSer {
    #[serde(rename = "agentId", alias = "agent_id")]
    agent_id: String,
    #[serde(default)]
    speaker: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default)]
    gmin: Option<i64>,
    #[serde(default, rename = "game_min", alias = "gameMin")]
    game_min: Option<i64>,
    #[serde(default)]
    ts: Option<String>,
}

impl From<ChatRowSer> for ChatRow {
    fn from(r: ChatRowSer) -> Self {
        ChatRow {
            agent_id: r.agent_id,
            speaker: r.speaker.unwrap_or_default(),
            text: r.text.unwrap_or_default(),
            gmin: r.gmin.or(r.game_min),
            ts: r.ts,
        }
    }
}

/// REST 폴 인젤로프 — 웹 JS 폴러(worldsim 포크)가 {tf:'relations'|'history'|
/// 'economy'|'chatbacklog', rows|data} 로 송출. 네이티브 폴러는 같은 큐를
/// 직접 채운다. Some(true)=처리함, Some(false)=인젤로프 맞지만 미처리, None=비인젤로프
fn poll_envelope(s: &str, chats: &ChatQueue, polls: &PollQueuesRes, eco: &EcoQueue) -> Option<bool> {
    let v: serde_json::Value = serde_json::from_str(s).ok()?;
    let tf = v.get("tf")?.as_str()?;
    match tf {
        "relations" => {
            let rows: Vec<RelRow> = serde_json::from_value(v.get("rows")?.clone()).ok()?;
            if let Ok(mut q) = polls.relations.lock() {
                *q = rows;
            }
            Some(true)
        }
        "history" => {
            let rows: Vec<HistRow> = serde_json::from_value(v.get("rows")?.clone()).ok()?;
            if let Ok(mut q) = polls.history.lock() {
                *q = rows;
            }
            Some(true)
        }
        "economy" => {
            let d: EcoData = eco_from_json(v.get("data")?.clone())?;
            if let Ok(mut q) = eco.0.lock() {
                *q = Some(d);
            }
            Some(true)
        }
        "chatbacklog" => {
            // rows는 최신순(서버) — 시계순으로 뒤집어 푸시
            let rows: Vec<ChatRowSer> = serde_json::from_value(v.get("rows")?.clone()).ok()?;
            for r in rows.into_iter().take(30).rev() {
                chat_row_push(chats, r.into());
            }
            Some(true)
        }
        _ => Some(false),
    }
}

/// 선택 주민 id를 폴러 공유 변수로 미러(http_poll이 history 대상 결정).
/// wasm은 worldsim 포크의 ui.setWatchAgent도 호출(JS REST 폴러 대상).
fn mirror_selected_agent(
    selected_q: Query<&apply::LiveAgentTag, With<crate::agents::Selected>>,
    sel: Res<SelectedAgentRes>,
    mut last: Local<Option<String>>,
) {
    let id = selected_q.iter().next().map(|t| t.0.clone());
    if let Ok(mut g) = sel.0.lock() {
        *g = id.clone();
    }
    if *last == id {
        return;
    }
    *last = id.clone();
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        if let Some(window) = web_sys::window()
            && let Ok(tf) = js_sys::Reflect::get(&window, &"terrafolk".into())
            && let Ok(ui) = js_sys::Reflect::get(&tf, &"ui".into())
            && let Ok(f) = js_sys::Reflect::get(&ui, &"setWatchAgent".into())
            && let Ok(f) = f.dyn_into::<js_sys::Function>()
        {
            let arg = id
                .as_deref()
                .map(wasm_bindgen::JsValue::from_str)
                .unwrap_or(wasm_bindgen::JsValue::NULL);
            let _ = f.call1(&ui, &arg);
        }
    }
}

/// 이벤트 큐 drain — 원시 JSON 행을 파싱해 EventLog(UI)로. 룸 방송·REST
/// 백로그가 같은 모양({kind, agent_id, payload, tick, ts?})으로 온다.
/// 상한 400(FE 버퍼 패리티).
fn drain_events(queue: Res<EventQueue>, mut log: ResMut<crate::ui::EventLog>) {
    let rows: Vec<String> = {
        let Ok(mut q) = queue.inner.lock() else {
            return;
        };
        q.drain(..).collect()
    };
    for r in rows {
        match parse_raw_event(&r) {
            Some(ev) => log.rows.push(ev),
            None => warn!("[net] 이벤트 행 파싱 실패: {}", r.chars().take(80).collect::<String>()),
        }
    }
    let len = log.rows.len();
    if len > 400 {
        log.rows.drain(0..len - 400);
    }
}

fn parse_raw_event(s: &str) -> Option<crate::ui::EventRow> {
    let v: serde_json::Value = serde_json::from_str(s).ok()?;
    let get_str = |k: &str| v.get(k).and_then(|x| x.as_str()).unwrap_or_default().to_owned();
    Some(crate::ui::EventRow {
        kind: get_str("kind"),
        agent_id: get_str("agent_id"),
        payload: v.get("payload").cloned().unwrap_or(serde_json::Value::Null),
        ts: v.get("ts").and_then(|x| x.as_str()).map(str::to_owned),
        tick: v.get("tick").and_then(|x| x.as_u64()).unwrap_or(0) as u32,
    })
}

/// wasm 이벤트 인젤로프(worldsim 포크가 송출)
#[derive(Deserialize)]
struct EventEnvelope {
    row: serde_json::Value,
}
