//! colyseus-lite — Colyseus 0.18 클라이언트 최소 구현(재사용 라이브러리).
//!
//! 다른 Rust 앱에서 그대로 쓰는 방법(게임 무관 — Bevy 의존 없음):
//!
//! ```ignore
//! // 1. 인증·matchmake는 앱 영역(예: reqwest)
//! let ticket = app_fetch_ticket().await;                    // GET /spectate-ticket
//! let mm: MatchmakeResp = app_post(
//!     &format!("{ws_base}/matchmake/joinOrCreate/world"),   // Bearer ticket, body {}
//! ).await;
//! // 2. 룸 접속 — ACK·프레임 라우팅은 라이브러리가
//! let (room, mut rx) = Room::connect(
//!     colyseus_lite::ws_url(&ws_base, &mm.process_id, &mm.room_id, &mm.session_id),
//!     Some(&ticket),
//! ).await?;
//! // 3. 상태 디코딩 — reflection에서 임의 스키마 자동 발견
//! let mut state = SchemaState::new();
//! while let Some(ev) = rx.recv().await {
//!     match ev {
//!         RoomEvent::Joined { reflection, .. } => state.apply_reflection(&reflection)?,
//!         RoomEvent::State(b) => state.apply_full(&b)?,
//!         RoomEvent::Patch(b) => state.apply_patch(&b)?,
//!         RoomEvent::Data { kind, payload } => { /* 채팅 등 */ }
//!         RoomEvent::Closed(_) => break, // 재접속 정책은 앱
//!         _ => {}
//!     }
//! }
//! let tick = state.root().field("tick").and_then(Value::as_f64);
//! room.send("chat", &serde_json::json!({ "agentId": id, "text": t }));
//! ```
//!
//! 구현 범위(프래그머틱): schema 시리얼라이저 1종(동적 디코딩)·joinOrCreate·
//! 룸 메시지·ping/leave. presence·룸 목록·reconnect 토큰 재사용·입력 채널
//! (19/20/21/22)은 이 프로토콜의 사용 경로에 없어 미구현.

pub mod msgpack;
pub mod protocol;
pub mod room;
pub mod schema;

pub use room::{ws_url, Room, RoomEvent};
pub use schema::{SchemaState, Value};

/// 룸 접속 확정 응답(matchmake POST `/matchmake/joinOrCreate/:room`의 본문).
/// 필드명은 와이어(camelCase) 그대로.
#[derive(Debug, Clone, serde::Deserialize)]
#[allow(non_snake_case)]
pub struct MatchmakeResp {
    pub name: String,
    pub sessionId: String,
    pub roomId: String,
    pub processId: String,
}
