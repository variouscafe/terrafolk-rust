//! 룸 — ws-link 위 Colyseus 룸 라이프사이클(조인 ACK·상태·메시지 라우팅).
//!
//! 사용 계약:
//! 1. 앱이 matchmake(HTTP POST `/matchmake/joinOrCreate/:room`, Bearer 티켓)로
//!    `{sessionId, roomId, processId}`를 받는다 — 인증은 앱 영역(라이브러리
//!    무지). 헬퍼: [`ws_url`](crate::ws_url).
//! 2. `Room::connect(url, token)`으로 접속 — WS URL은
//!    `ws(s)://host/{processId}/{roomId}?sessionId={sid}[&_authToken=…]`.
//! 3. 서버 JOIN_ROOM(10) 수신 즉시 라이브러리가 1바이트 ACK를 보낸다
//!    (**ACK 없으면 서버는 상태를 보내지 않는다** — 0.18 규약).
//! 4. 이벤트: Joined(Reflection → `SchemaState::apply_reflection`) →
//!    State/Patch(스키마 바이트 → `apply_full`/`apply_patch`) → Data(룸 메시지).
//! 5. 끊기면 `Closed` 1회 — Colyseus 좌석은 재발급이 필요하므로 자동 재접속
//!    대신 앱이 1~5단계를 다시 밟는다(백오프 주기 관리도 앱).

use crate::msgpack;
use crate::protocol::{self, code, frame_code};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use tokio::sync::mpsc;
use ws_link::{LinkEvent, LinkHandle, WsConfig};

/// 룸 이벤트 — 상태는 원시 바이트(디코딩은 `SchemaState` 담당).
#[derive(Debug)]
pub enum RoomEvent {
    /// 조인 완립(ACK 송신 직후) — reflection을 `SchemaState::apply_reflection`으로.
    Joined {
        reconnection_token: String,
        serializer_id: String,
        reflection: Vec<u8>,
    },
    /// ROOM_STATE — 전체 상태 바이트.
    State(Vec<u8>),
    /// ROOM_STATE_PATCH — 증분 바이트.
    Patch(Vec<u8>),
    /// ROOM_DATA — 타입 문자열 + msgpack 페이로드(JSON 값).
    Data {
        kind: String,
        payload: serde_json::Value,
    },
    /// ROOM_DATA_BYTES — 이진 페이로드(본 프로토콜에선 미사용, 통과).
    DataBytes {
        kind: String,
        payload: Vec<u8>,
    },
    Error {
        code: u16,
        message: String,
    },
    /// 연결 종료(사유) — 재접속은 앱이 결정한다.
    Closed(String),
}

/// 룸 조작 핸들.
pub struct Room {
    link: LinkHandle,
    connected: Arc<AtomicBool>,
}

impl Room {
    /// WS 접속 + 이벤트 펌프 기동. `auth_token`은 업그레이드 헤더에 싣는다
    /// (서버 onAuth는 matchmake 단계에서 이미 통과 — 예비).
    /// 호출자의 비동기 런타임(또는 스레드) 위에서 폴링된다.
    pub async fn connect(
        url: String,
        auth_token: Option<&str>,
    ) -> Result<(Room, mpsc::Receiver<RoomEvent>), String> {
        let mut cfg = WsConfig::new(url);
        cfg.reconnect = false; // 좌석 재발급 필요 — 재접속은 앱이 관리
        cfg.max_message_size = 32 * 1024 * 1024;
        if let Some(t) = auth_token {
            cfg.headers
                .push(("Authorization".into(), format!("Bearer {t}")));
        }
        let (link, link_rx) = ws_link::spawn_link(cfg);
        let (ev_tx, ev_rx) = mpsc::channel::<RoomEvent>(128);
        let connected = Arc::new(AtomicBool::new(false));
        let room = Room {
            link: link.clone(),
            connected: Arc::clone(&connected),
        };
        tokio::spawn(pump(link, link_rx, ev_tx, connected));
        Ok((room, ev_rx))
    }

    /// 룸 메시지 송신 — `[13][msgpack 타입][msgpack 페이로드]`.
    pub fn send(&self, kind: &str, payload: &serde_json::Value) {
        self.link.send_binary(protocol::data_frame(kind, payload));
    }

    /// 퇴장 — 서버가 WS를 닫는다(코드 4000).
    pub fn leave(&self) {
        self.link.send_binary(protocol::leave_frame());
    }

    /// 왕복 지연 측정용 ping(서버가 `[18]` 에코).
    pub fn ping(&self) {
        self.link.send_binary(protocol::ping_frame());
    }

    pub fn is_connected(&self) -> bool {
        self.connected.load(std::sync::atomic::Ordering::Relaxed)
    }
}

async fn pump(
    link: LinkHandle,
    mut link_rx: mpsc::Receiver<LinkEvent>,
    ev_tx: mpsc::Sender<RoomEvent>,
    connected: Arc<AtomicBool>,
) {
    let mut joined = false;
    while let Some(ev) = link_rx.recv().await {
        match ev {
            LinkEvent::Connected => {
                connected.store(true, std::sync::atomic::Ordering::Relaxed);
            }
            LinkEvent::Binary(frame) => {
                route(&link, &frame, &mut joined, &ev_tx).await;
            }
            LinkEvent::Text(_) => {} // Colyseus는 바이너리 프레임만
            LinkEvent::Disconnected(reason) => {
                connected.store(false, std::sync::atomic::Ordering::Relaxed);
                let _ = ev_tx.send(RoomEvent::Closed(reason)).await;
                return;
            }
        }
    }
    connected.store(false, std::sync::atomic::Ordering::Relaxed);
    let _ = ev_tx.send(RoomEvent::Closed("링크 종료".into())).await;
}

async fn route(
    link: &LinkHandle,
    frame: &[u8],
    joined: &mut bool,
    ev_tx: &mpsc::Sender<RoomEvent>,
) {
    if frame.is_empty() {
        return;
    }
    let body = &frame[1..];
    match frame_code(frame[0]) {
        code::JOIN_ROOM if !*joined => {
            match protocol::parse_join_room(body) {
                Ok(info) => {
                    // 필수 ACK — 이후 서버가 ROOM_STATE를 보내기 시작한다.
                    link.send_binary(protocol::ack_join());
                    *joined = true;
                    let _ = ev_tx
                        .send(RoomEvent::Joined {
                            reconnection_token: info.reconnection_token,
                            serializer_id: info.serializer_id,
                            reflection: info.reflection,
                        })
                        .await;
                }
                Err(e) => {
                    let _ = ev_tx
                        .send(RoomEvent::Closed(format!("조인 파싱 실패: {e}")))
                        .await;
                }
            }
        }
        code::ROOM_STATE => {
            let _ = ev_tx.send(RoomEvent::State(body.to_vec())).await;
        }
        code::ROOM_STATE_PATCH => {
            let _ = ev_tx.send(RoomEvent::Patch(body.to_vec())).await;
        }
        code::ROOM_DATA => {
            let mut off = 0usize;
            let parsed = (|| -> Result<RoomEvent, String> {
                let kind = msgpack::read_string(body, &mut off)?;
                let payload = msgpack::read_value(body, &mut off)?;
                Ok(RoomEvent::Data { kind, payload })
            })();
            match parsed {
                Ok(ev) => {
                    let _ = ev_tx.send(ev).await;
                }
                Err(e) => {
                    let _ = ev_tx
                        .send(RoomEvent::Closed(format!("룸 메시지 파싱 실패: {e}")))
                        .await;
                }
            }
        }
        code::ROOM_DATA_BYTES => {
            let mut off = 0usize;
            if let Ok(kind) = msgpack::read_string(body, &mut off) {
                let _ = ev_tx
                    .send(RoomEvent::DataBytes {
                        kind,
                        payload: body[off..].to_vec(),
                    })
                    .await;
            }
        }
        code::ERROR => {
            let mut off = 0usize;
            let c = msgpack::read_value(body, &mut off)
                .ok()
                .and_then(|v| v.as_f64())
                .unwrap_or(0.0) as u16;
            let m = msgpack::read_value(body, &mut off)
                .ok()
                .and_then(|v| v.as_str().map(|s| s.to_owned()))
                .unwrap_or_default();
            let _ = ev_tx
                .send(RoomEvent::Error {
                    code: c,
                    message: m,
                })
                .await;
        }
        code::LEAVE_ROOM => {
            let _ = ev_tx.send(RoomEvent::Closed("서버 퇴장 요청".into())).await;
        }
        code::PING => {} // 에코 — 무시
        _ => {}
    }
}

/// matchmake 응답 → WS 젌속 URL 조립.
/// `ws_base`: `http(s)://host[:port]` 형태(스킴은 ws(s)로 치환).
pub fn ws_url(ws_base: &str, process_id: &str, room_id: &str, session_id: &str) -> String {
    let scheme = if ws_base.starts_with("https") {
        "wss"
    } else {
        "ws"
    };
    let host = ws_base
        .trim_start_matches("https://")
        .trim_start_matches("http://")
        .trim_end_matches('/');
    format!("{scheme}://{host}/{process_id}/{room_id}?sessionId={session_id}")
}
