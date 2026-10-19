//! 네이티브 라이브 접속 — root src/net/native_conn.rs의 관전(무인증) 축소판.
//! 흐름(§17.82): 관전 티켓 GET → matchmake POST(Bearer) → WS 조인(라이브러리가
//! ACK) → 스키마 디코딩 → LiveFrame JSON을 FrameQueue에, 룸 채팅은 ChatQueue에.
//! 끊기면 3s→30s 지수 백오프. 스레드 패닉은 삼켜 데모 관전 유지.

use super::{ChatQueue, ChatRow, FrameQueue, LiveAgent, LiveFrame, LiveResource, PHASE_CONNECTING, PHASE_RECONNECTING};
use colyseus_lite::{MatchmakeResp, Room, RoomEvent, SchemaState, Value as ColyValue};
use log::info;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

const PROD_API: &str = "https://tf-api.suseona.com";
const PROD_WS: &str = "https://tf-world.suseona.com";
const LOCAL_API: &str = "http://127.0.0.1:8892";
const LOCAL_WS: &str = "http://127.0.0.1:8893";

/// 접속 스레드 기동 — NetPlugin(native) connect_bridge에서 1회 호출.
pub fn spawn(queue: FrameQueue, phase: Arc<AtomicU8>, chats: ChatQueue) {
    let _ = std::thread::Builder::new()
        .name("tf-net".into())
        .spawn(move || {
            if catch_unwind(AssertUnwindSafe(|| run_blocking(queue, phase, chats))).is_err() {
                // 패닉 전파는 앱 전체 중단(release abort) — 삼키고 로그만.
                eprintln!("[net] 네트워크 스레드 패닉 — 중단(데모 관전 유지)");
            }
        });
}

fn run_blocking(queue: FrameQueue, phase: Arc<AtomicU8>, chats: ChatQueue) {
    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return;
    };
    rt.block_on(run(queue, phase, chats));
}

/// 로컬 백엔드 우선 프로브 — 데스크톱 개발 무설정. 127.0.0.1 거부는 즉시
/// 실패라 실기기 체감 지연 없음.
fn pick_bases() -> (&'static str, &'static str) {
    use std::net::TcpStream;
    let addr: std::net::SocketAddr = "127.0.0.1:8892".parse().expect("주소");
    let local = TcpStream::connect_timeout(&addr, Duration::from_millis(600)).is_ok();
    if local {
        (LOCAL_API, LOCAL_WS)
    } else {
        (PROD_API, PROD_WS)
    }
}

async fn run(queue: FrameQueue, phase: Arc<AtomicU8>, chats: ChatQueue) {
    let (api, ws) = pick_bases();
    info!("[net] 라이브 베이스 — api {api} · ws {ws}");
    let Ok(http) = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    else {
        return;
    };
    let mut retry: u32 = 0;
    loop {
        phase.store(PHASE_CONNECTING, Ordering::Relaxed);
        if let Err(e) = once(&http, api, ws, &queue, &chats).await {
            info!("[net] 접속 종료: {e}");
        }
        retry += 1;
        let wait = Duration::from_millis((3000u64 * (1 << (retry - 1).min(4))).min(30000));
        phase.store(PHASE_RECONNECTING, Ordering::Relaxed);
        info!("[net] {wait:?} 후 재접속(시도 {retry})");
        tokio::time::sleep(wait).await;
    }
}

/// 관전 티켓(무인증) — 게이트 면제 경로, 항상 동작.
async fn spectate_ticket(http: &reqwest::Client, api: &str) -> Result<String, String> {
    http.get(format!("{api}/api/worldsim/spectate-ticket"))
        .send()
        .await
        .map_err(|e| format!("티켓 요청 실패: {e}"))?
        .error_for_status()
        .map_err(|e| format!("티켓 HTTP {e}"))?
        .json::<serde_json::Value>()
        .await
        .map_err(|e| e.to_string())?
        .get("ticket")
        .and_then(|t| t.as_str())
        .map(str::to_owned)
        .ok_or_else(|| "티켓 응답에 ticket 없음".into())
}

/// 1회 접속 주기 — 티켓부터 스트림 종료까지.
async fn once(
    http: &reqwest::Client,
    api: &str,
    ws: &str,
    queue: &FrameQueue,
    chats: &ChatQueue,
) -> Result<(), String> {
    let ticket = spectate_ticket(http, api).await?;
    // matchmake — 좌석 확정(onAuth도 이 HTTP 단계에서).
    let mm: MatchmakeResp = http
        .post(format!("{ws}/matchmake/joinOrCreate/world"))
        .bearer_auth(&ticket)
        .json(&serde_json::json!({}))
        .send()
        .await
        .map_err(|e| format!("matchmake 실패: {e}"))?
        .error_for_status()
        .map_err(|e| format!("matchmake HTTP {e}"))?
        .json()
        .await
        .map_err(|e| e.to_string())?;
    let url = colyseus_lite::ws_url(ws, &mm.processId, &mm.roomId, &mm.sessionId);
    let (_room, mut rx) = Room::connect(url, Some(&ticket)).await?;
    info!("[net] 룸 접속 완료 — room {}", mm.roomId);
    let mut state = SchemaState::new();
    // mapJson은 fullStateOnly — 첫 전체 상태에만 실어 보낸다(이후 None).
    let mut first_state = true;
    while let Some(ev) = rx.recv().await {
        match ev {
            RoomEvent::Joined {
                reflection,
                serializer_id,
                ..
            } => {
                if serializer_id != "schema" {
                    return Err(format!("미지원 시리얼라이저 {serializer_id}"));
                }
                state.apply_reflection(&reflection)?;
            }
            RoomEvent::State(b) => {
                state.apply_full(&b)?;
                push_frame(&state, &mut first_state, queue);
            }
            RoomEvent::Patch(b) => {
                state.apply_patch(&b)?;
                push_frame(&state, &mut first_state, queue);
            }
            RoomEvent::Data { kind, payload } => match kind.as_str() {
                "chat" => {
                    let row = ChatRow {
                        agent_id: payload
                            .get("agent_id")
                            .or_else(|| payload.get("agentId"))
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_owned(),
                        speaker: payload
                            .get("speaker")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_owned(),
                        text: payload
                            .get("text")
                            .and_then(|v| v.as_str())
                            .unwrap_or_default()
                            .to_owned(),
                    };
                    if !row.text.is_empty() {
                        if let Ok(mut q) = chats.inner.lock() {
                            // 말풍선 폭주 방지 — 최근 16행
                            q.push(row);
                            let len = q.len();
                            if len > 16 {
                                q.drain(0..len - 16);
                            }
                        }
                    }
                }
                "event" | "hello" => {} // 이벤트 로그·인사 — MVP 범위 밖
                other => info!("[net] 룸 메시지 수신: {other}"),
            },
            RoomEvent::Error { code, message } => {
                return Err(format!("룸 오류 {code}: {message}"));
            }
            RoomEvent::Closed(r) => return Err(format!("연결 종료: {r}")),
            RoomEvent::DataBytes { .. } => {}
        }
    }
    Err("이벤트 스트림 종료".into())
}

/// 스키마 값 트리 → LiveFrame(JSON 문자열) → 큐. 큐는 최근 5프레임.
fn push_frame(state: &SchemaState, first_state: &mut bool, queue: &FrameQueue) {
    let Some(frame) = build_frame(state, *first_state) else {
        return;
    };
    *first_state = false;
    let Ok(json) = serde_json::to_string(&frame) else {
        return;
    };
    if let Ok(mut q) = queue.inner.lock() {
        q.push(json);
        let len = q.len();
        if len > 5 {
            q.drain(0..len - 5);
        }
    }
}

fn build_frame(state: &SchemaState, with_map: bool) -> Option<LiveFrame> {
    let r = state.root();
    let f = |name: &str| r.field(name);
    let strv = |name: &str| f(name).and_then(ColyValue::as_str);
    let numv = |name: &str| f(name).and_then(ColyValue::as_f64);
    let agents = f("agents")?
        .entries()?
        .iter()
        .filter_map(|(id, v)| {
            let av = |name: &str| v.field(name);
            Some(LiveAgent {
                id: id.clone(),
                name: av("name")?.as_str()?.to_owned(),
                kind: av("kind")
                    .and_then(ColyValue::as_str)
                    .unwrap_or("bot")
                    .to_owned(),
                x: av("x")?.as_f64()? as f32,
                y: av("y")?.as_f64()? as f32,
                facing: av("facing")
                    .and_then(ColyValue::as_str)
                    .unwrap_or("S")
                    .to_owned(),
                status: av("status")
                    .and_then(ColyValue::as_str)
                    .unwrap_or("idle")
                    .to_owned(),
                act_type: av("actType").and_then(ColyValue::as_str).map(str::to_owned),
                _act_type_r: None,
                look: av("lookJson")
                    .and_then(ColyValue::as_str)
                    .map(str::to_owned),
                act_progress: av("actProgress")
                    .and_then(ColyValue::as_f64)
                    .map(|v| v as f32),
                goal_text: av("goalText").and_then(ColyValue::as_str).map(str::to_owned),
                goal_key: av("goalKey").and_then(ColyValue::as_str).map(str::to_owned),
                hunger: av("hunger").and_then(ColyValue::as_f64).map(|v| v as f32),
                thirst: av("thirst").and_then(ColyValue::as_f64).map(|v| v as f32),
                fatigue: av("fatigue").and_then(ColyValue::as_f64).map(|v| v as f32),
                warmth: av("warmth").and_then(ColyValue::as_f64).map(|v| v as f32),
                inv_json: av("invJson").and_then(ColyValue::as_str).map(str::to_owned),
            })
        })
        .collect();
    let resources = f("resources")?
        .entries()?
        .iter()
        .filter_map(|(_, v)| {
            let rv = |name: &str| v.field(name);
            Some(LiveResource {
                id: rv("id")?.as_f64()? as u32,
                kind: rv("kind")?.as_str()?.to_owned(),
                species: rv("species")
                    .and_then(ColyValue::as_str)
                    .unwrap_or("")
                    .to_owned(),
                x: rv("x")?.as_f64()? as u16,
                y: rv("y")?.as_f64()? as u16,
                state: rv("stateJson")
                    .and_then(ColyValue::as_str)
                    .map(str::to_owned),
            })
        })
        .collect();
    Some(LiveFrame {
        tick: numv("tick")? as u32,
        day: numv("day").unwrap_or(0.0) as u16,
        hour: numv("hour").unwrap_or(0.0) as u16,
        minute: numv("minute").unwrap_or(0.0) as u16,
        season: strv("season").unwrap_or("spring").to_owned(),
        weather: strv("weather").map(str::to_owned),
        sunrise: numv("sunrise").map(|v| v as u16),
        sunset: numv("sunset").map(|v| v as u16),
        map: if with_map {
            strv("mapJson").map(str::to_owned)
        } else {
            None
        },
        agents,
        resources,
        inventions: strv("inventionsJson").map(str::to_owned),
    })
}
