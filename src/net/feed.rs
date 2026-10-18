//! 관전 피드 버퍼 — 채팅·이벤트(룸 WS 실시간 + HTTP 백로그 시딩)·관계·생활기록.
//! 스레드원(Arc<Mutex> 큐)이 채우고 drain_feeds가 ECS 리소스(ChatLog·EventLog)로
//! 정리한다. 토스트·말풍선은 각자의 watermark(EventLog.seen·ChatLog.seen)로
//! 새 행만 소비한다(§17.53 순차 — 큐 도착순 곧 방송순).

use bevy::prelude::*;
use serde_json::Value;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

const CHAT_CAP: usize = 120;
const EVENT_CAP: usize = 400;

/// 채팅 행 — 룸 방송 {agent_id,speaker,text,gmin,lang,text_en}과
/// HTTP 백로그 {...,ts,game_min} 양쪽 형상을 흡수한다.
#[derive(Clone, Debug)]
pub struct ChatRow {
    pub agent_id: String,
    pub speaker: String,
    pub text: String,
    pub text_en: Option<String>,
    pub lang: String,
    pub gmin: Option<i64>,
    pub ts: Option<String>,
}

impl ChatRow {
    /// 중복 판정 키(백로그·실시간 병합 시) — ts·gmin은 어느 한쪽에만 있다.
    fn same_as(&self, o: &ChatRow) -> bool {
        self.speaker == o.speaker && self.text == o.text && self.gmin == o.gmin
    }
}

/// 이벤트 행 — 룸 방송(e 객체)과 HTTP /events 행({id,ts,...}) 양쪽.
#[derive(Clone, Debug)]
pub struct LiveEvent {
    pub id: Option<i64>,
    pub ts: Option<String>,
    pub tick: u32,
    pub kind: String,
    pub agent_id: String,
    pub payload: Value,
}

impl LiveEvent {
    fn same_as(&self, o: &LiveEvent) -> bool {
        self.tick == o.tick
            && self.kind == o.kind
            && self.agent_id == o.agent_id
            && self.payload == o.payload
    }
}

/// 관계 행(HTTP /relations — warmth DESC 정렬로 온다).
#[derive(Clone, Debug)]
pub struct RelRow {
    pub a_id: String,
    pub b_id: String,
    pub warmth: f32,
    pub chats: u32,
    pub trades: u32,
}

/// 생활 기록 행(HTTP /agents/{id}/history — type: gain|trade|craft|consume).
#[derive(Clone, Debug)]
pub struct HistRow {
    pub ts: String,
    pub type_: String,
    pub text: String,
}

/// 하이라이트 토스트 1건(우하단) — toasts.rs가 소비.
#[derive(Clone, Debug)]
pub struct Toast {
    pub icon: &'static str,
    pub who: String,
    pub text: String,
}

/// 스레드→ECS 큐 — 룸 WS 실시간(chat·event) + HTTP 백로그 시딩(*_seed).
#[derive(Resource, Clone, Default)]
pub struct FeedQueues {
    pub chat: Arc<Mutex<Vec<ChatRow>>>,
    pub event: Arc<Mutex<Vec<LiveEvent>>>,
    /// 백로그는 오래된 순으로 로그 선두에 병합된다(접속 에지 1회).
    pub chat_seed: Arc<Mutex<Vec<ChatRow>>>,
    pub event_seed: Arc<Mutex<Vec<LiveEvent>>>,
}

/// HTTP 폴 결과 저장소 — 관계(20s)·생활 기록(선택 주민, 15s).
#[derive(Resource, Clone, Default)]
pub struct PollStore {
    pub relations: Arc<Mutex<Vec<RelRow>>>,
    pub history: Arc<Mutex<Vec<HistRow>>>,
    /// UI가 요청한 생활기록 대상 id(선택 주민) — None이면 폴 중단.
    pub want_history: Arc<Mutex<Option<String>>>,
}

/// 채팅 표시 버퍼 — seen은 말풍선이 마지막으로 소비한 행 수(watermark).
#[derive(Resource, Default)]
pub struct ChatLog {
    pub rows: VecDeque<ChatRow>,
    pub seen: usize,
}

/// 이벤트 표시 버퍼 — seen은 토스트가 마지막으로 소비한 행 수(watermark).
#[derive(Resource, Default)]
pub struct EventLog {
    pub rows: Vec<LiveEvent>,
    pub seen: usize,
}

/// 토스트 적재 큐(하이라이트 판정 통과분) — toasts.rs가 프레임마다 소비.
#[derive(Resource, Default)]
pub struct ToastQ {
    pub pending: Vec<Toast>,
}

fn s(v: &Value, k: &str) -> Option<String> {
    v.get(k).and_then(Value::as_str).map(str::to_owned)
}

/// 룸 'chat' 방송 → ChatRow.
pub fn chat_from_room(v: &Value) -> Option<ChatRow> {
    Some(ChatRow {
        agent_id: s(v, "agent_id")?,
        speaker: s(v, "speaker").unwrap_or_default(),
        text: s(v, "text")?,
        text_en: s(v, "text_en").filter(|t| !t.is_empty()),
        lang: s(v, "lang").unwrap_or_else(|| "en".into()),
        gmin: v.get("gmin").and_then(Value::as_i64),
        ts: None,
    })
}

/// 룸 'event' 방송(이벤트 객체 e) → LiveEvent.
pub fn event_from_room(v: &Value) -> Option<LiveEvent> {
    Some(LiveEvent {
        id: v.get("id").and_then(Value::as_i64),
        ts: s(v, "ts"),
        tick: v.get("tick").and_then(Value::as_u64).unwrap_or(0) as u32,
        kind: s(v, "kind")?,
        agent_id: s(v, "agent_id").unwrap_or_default(),
        payload: v.get("payload").cloned().unwrap_or(Value::Null),
    })
}

/// HTTP /api/worldsim/chat 행 → ChatRow.
pub fn chat_from_http(v: &Value) -> Option<ChatRow> {
    Some(ChatRow {
        agent_id: s(v, "agent_id")?,
        speaker: s(v, "speaker").unwrap_or_default(),
        text: s(v, "text")?,
        text_en: s(v, "text_en").filter(|t| !t.is_empty()),
        lang: s(v, "lang").unwrap_or_else(|| "en".into()),
        gmin: v.get("game_min").and_then(Value::as_i64),
        ts: s(v, "ts"),
    })
}

/// HTTP /api/worldsim/events 행 → LiveEvent.
pub fn event_from_http(v: &Value) -> Option<LiveEvent> {
    event_from_room(v)
}

/// 룸 WS 실시간 push — native_conn Data 디코딩에서 호출.
pub fn push_chat(q: &FeedQueues, v: &Value) {
    let Some(row) = chat_from_room(v) else {
        return;
    };
    if let Ok(mut q) = q.chat.lock() {
        q.push(row);
    }
}

pub fn push_event(q: &FeedQueues, v: &Value) {
    let Some(row) = event_from_room(v) else {
        return;
    };
    if let Ok(mut q) = q.event.lock() {
        q.push(row);
    }
}

fn take<T>(slot: &Mutex<Vec<T>>) -> Vec<T> {
    std::mem::take(&mut *slot.lock().unwrap_or_else(|e| e.into_inner()))
}

/// 큐 → 표시 버퍼. 백로그 시드는 선두 병합(오래된 순) + 내용 중복 제거,
/// 실시간 행은 꼬리 적재 후 cap 잘라낸다.
pub fn drain_feeds(
    queues: ResMut<FeedQueues>,
    mut chat: ResMut<ChatLog>,
    mut evlog: ResMut<EventLog>,
) {
    for row in take(&queues.chat_seed).into_iter().rev() {
        let dup = chat.rows.iter().take(48).any(|r| r.same_as(&row));
        if !dup {
            chat.rows.push_front(row);
        }
    }
    for row in take(&queues.event_seed).into_iter().rev() {
        let dup = evlog.rows.iter().take(48).any(|r| r.same_as(&row));
        if !dup {
            evlog.rows.insert(0, row);
        }
    }
    for row in take(&queues.chat) {
        chat.rows.push_back(row);
    }
    for row in take(&queues.event) {
        evlog.rows.push(row);
    }
    while chat.rows.len() > CHAT_CAP {
        chat.rows.pop_front();
    }
    let len = evlog.rows.len();
    if len > EVENT_CAP {
        evlog.rows.drain(0..len - EVENT_CAP);
    }
    // watermark가 cap 잘림을 앞지르지 않게 클램프(소비자는 '새 행'만 본다).
    chat.seen = chat.seen.min(chat.rows.len());
    evlog.seen = evlog.seen.min(evlog.rows.len());
}
