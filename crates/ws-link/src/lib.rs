//! ws-link — WebSocket 재접속 링크(재사용 라이브러리).
//!
//! 게임·프로토콜 무지: URL로 접속해 이벤트 스트림(Connected/Text/Binary/
//! Disconnected)을 주고, 핸들로 송신한다. 끊기면 지수 백오프로 재접속한다
//! (`WsConfig::reconnect = false`면 1회 접속 후 종료 — 상위 프로토콜이
//! 재접속 주기를 직접 관리할 때).
//!
//! 설계 계약:
//! - `spawn_link`는 전용 스레드(+current-thread tokio 런타임)를 띄운다 —
//!   호출자에 비동기 런타임이 없어도 된다(게임 엔진 부팅 직접 호출 용이).
//! - 이벤트 수신자를 떨어뜨리면(수신 정체·drop) 링크는 스스로 종료한다.
//! - TLS는 rustls(webpki 루트) — 크로스컴파일 안전(native-tls/openssl 무의존).
//! - WS 프로토콜 수준 ping/pong은 자동 응답(tungstenite) — 앱 심장박동 불필요.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// 접속 설정 — 헤더·메시지 상한·백오프 곡선.
#[derive(Clone, Debug)]
pub struct WsConfig {
    pub url: String,
    /// 업그레이드 요청에 실을 헤더(예: Authorization).
    pub headers: Vec<(String, String)>,
    /// WS 메시지 상한(바이트) — 큰 상태 스냅숏을 받으려면 넉넉히.
    pub max_message_size: usize,
    /// 끊김 시 자동 재접속 여부.
    pub reconnect: bool,
    pub min_backoff: Duration,
    pub max_backoff: Duration,
}

impl WsConfig {
    pub fn new(url: impl Into<String>) -> Self {
        WsConfig {
            url: url.into(),
            headers: Vec::new(),
            max_message_size: 16 * 1024 * 1024,
            reconnect: true,
            min_backoff: Duration::from_millis(3000),
            max_backoff: Duration::from_millis(30000),
        }
    }
}

/// 링크 이벤트 — 수명 주기와 데이터 프레임.
#[derive(Debug)]
pub enum LinkEvent {
    Connected,
    Text(String),
    Binary(Vec<u8>),
    /// 접속 실패·종료(사유). 재접속 모드면 이후 재시도가 이어진다.
    Disconnected(String),
}

enum Cmd {
    Text(String),
    Binary(Vec<u8>),
    Close,
}

/// 링크 조작 핸들 — Clone 가능(송신자 공유).
pub struct LinkHandle {
    tx: mpsc::UnboundedSender<Cmd>,
    connected: Arc<AtomicBool>,
}

impl LinkHandle {
    pub fn send_text(&self, s: impl Into<String>) {
        let _ = self.tx.send(Cmd::Text(s.into()));
    }
    pub fn send_binary(&self, v: Vec<u8>) {
        let _ = self.tx.send(Cmd::Binary(v));
    }
    /// 정상 종료 — 재접속도 멈춘다.
    pub fn close(&self) {
        let _ = self.tx.send(Cmd::Close);
    }
    pub fn is_connected(&self) -> bool {
        self.connected.load(Ordering::Relaxed)
    }
}

impl Clone for LinkHandle {
    fn clone(&self) -> Self {
        LinkHandle {
            tx: self.tx.clone(),
            connected: Arc::clone(&self.connected),
        }
    }
}

/// 링크 기동 — 전용 스레드에서 접속 루프를 돌린다.
pub fn spawn_link(cfg: WsConfig) -> (LinkHandle, mpsc::Receiver<LinkEvent>) {
    let (cmd_tx, cmd_rx) = mpsc::unbounded_channel();
    let (ev_tx, ev_rx) = mpsc::channel::<LinkEvent>(256);
    let connected = Arc::new(AtomicBool::new(false));
    let handle = LinkHandle {
        tx: cmd_tx,
        connected: Arc::clone(&connected),
    };
    let _ = std::thread::Builder::new()
        .name("ws-link".into())
        .spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            let Ok(rt) = rt else {
                return;
            };
            rt.block_on(run(cfg, cmd_rx, ev_tx, connected));
        });
    (handle, ev_rx)
}

async fn run(
    cfg: WsConfig,
    mut cmd_rx: mpsc::UnboundedReceiver<Cmd>,
    ev_tx: mpsc::Sender<LinkEvent>,
    connected: Arc<AtomicBool>,
) {
    let mut attempt: u32 = 0;
    loop {
        connected.store(false, Ordering::Relaxed);
        attempt += 1;
        match session(&cfg, &mut cmd_rx, &ev_tx, &connected).await {
            SessionEnd::ClosedByCommand => return,
            SessionEnd::Dropped(reason) => {
                let _ = ev_tx.send(LinkEvent::Disconnected(reason)).await;
                if !cfg.reconnect {
                    return;
                }
            }
        }
        // 지수 백오프 대기 — Close 명령이 오면 즉시 중단.
        let wait = cfg
            .min_backoff
            .saturating_mul(1u32 << (attempt - 1).min(16))
            .min(cfg.max_backoff);
        match tokio::time::timeout(wait, cmd_rx.recv()).await {
            Ok(Some(Cmd::Close)) | Ok(None) => return,
            Ok(Some(_)) => {} // 대기 중 송신 명령 — 폐기(아직 미접속)
            Err(_) => {}      // 대기 완료 — 재접속
        }
    }
}

enum SessionEnd {
    ClosedByCommand,
    Dropped(String),
}

/// 1회 접속 세션 — 성립 → 이벤트 펌프 → 종료 사유 반환.
async fn session(
    cfg: &WsConfig,
    cmd_rx: &mut mpsc::UnboundedReceiver<Cmd>,
    ev_tx: &mpsc::Sender<LinkEvent>,
    connected: &Arc<AtomicBool>,
) -> SessionEnd {
    use futures_util::SinkExt;
    use futures_util::StreamExt;
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    use tokio_tungstenite::tungstenite::http::HeaderValue;
    use tokio_tungstenite::tungstenite::Message;
    use tokio_tungstenite::tungstenite::protocol::WebSocketConfig;

    let mut req = match cfg.url.as_str().into_client_request() {
        Ok(r) => r,
        Err(e) => return SessionEnd::Dropped(format!("URL 오류: {e}")),
    };
    for (k, v) in &cfg.headers {
        let Ok(name) = tokio_tungstenite::tungstenite::http::HeaderName::try_from(k.as_str())
        else {
            return SessionEnd::Dropped(format!("헤더 이름 오류: {k}"));
        };
        let Ok(val) = HeaderValue::from_str(v) else {
            return SessionEnd::Dropped(format!("헤더 값 오류: {k}"));
        };
        req.headers_mut().insert(name, val);
    }
    let mut ws_cfg = WebSocketConfig::default();
    ws_cfg.max_message_size = Some(cfg.max_message_size);
    ws_cfg.max_frame_size = Some(cfg.max_message_size);
    let (mut ws, _resp) = match tokio_tungstenite::connect_async_tls_with_config(
        req, Some(ws_cfg), false, None,
    )
    .await
    {
        Ok(c) => c,
        Err(e) => return SessionEnd::Dropped(format!("접속 실패: {e}")),
    };
    connected.store(true, Ordering::Relaxed);
    if ev_tx.send(LinkEvent::Connected).await.is_err() {
        return SessionEnd::ClosedByCommand;
    }
    loop {
        tokio::select! {
            biased;
            cmd = cmd_rx.recv() => match cmd {
                Some(Cmd::Text(s)) => {
                    if ws.send(Message::Text(s.into())).await.is_err() {
                        return SessionEnd::Dropped("송신 실패".into());
                    }
                }
                Some(Cmd::Binary(v)) => {
                    if ws.send(Message::Binary(v.into())).await.is_err() {
                        return SessionEnd::Dropped("송신 실패".into());
                    }
                }
                Some(Cmd::Close) | None => {
                    let _ = ws.close(None).await;
                    return SessionEnd::ClosedByCommand;
                }
            },
            msg = ws.next() => match msg {
                Some(Ok(Message::Text(t))) => {
                    if ev_tx.send(LinkEvent::Text(t.to_string())).await.is_err() {
                        return SessionEnd::ClosedByCommand;
                    }
                }
                Some(Ok(Message::Binary(b))) => {
                    if ev_tx.send(LinkEvent::Binary(b.to_vec())).await.is_err() {
                        return SessionEnd::ClosedByCommand;
                    }
                }
                Some(Ok(Message::Ping(_) | Message::Pong(_))) => {} // 자동 응답됨
                Some(Ok(Message::Close(c))) => {
                    return SessionEnd::Dropped(format!("종료: {c:?}"));
                }
                Some(Ok(Message::Frame(_))) => {}
                Some(Err(e)) => return SessionEnd::Dropped(format!("오류: {e}")),
                None => return SessionEnd::Dropped("스트림 종료".into()),
            },
        }
    }
}
