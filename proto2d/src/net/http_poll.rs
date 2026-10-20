//! HTTP 관전 폴러(네이이티브 전용) — 관계 목록·선택 주민 생활기록·경제 관측·
//! 채팅·이벤트 백로그. root src/net/http_poll.rs의 축소 포트:
//! 룸 접속(PHASE_CONNECTED) 후에만 동작. relations 20s · history 15s(선택 시)
//! · economy 30s · 백로그(chat·events)는 1회 시드.

use crate::net::{
    ChatQueue, EcoQueue, EventQueue, PHASE_CONNECTED, PollQueuesRes, RelRow,
    SelectedAgentRes, chat_row_push,
};
use log::info;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

/// 접속 스레드 기동 — NetPlugin(native) connect_bridge에서 1회 호출.
/// 큐·선택 공유는 리소스가 들고 있는 Arc를 그대로 넘긴다.
pub fn spawn(
    phase: Arc<AtomicU8>,
    chats: ChatQueue,
    events: EventQueue,
    eco: EcoQueue,
    api_base: &'static str,
    queues: PollQueuesRes,
    selected: SelectedAgentRes,
) {
    let _ = std::thread::Builder::new()
        .name("tf-http".into())
        .spawn(move || {
            let rt = match tokio::runtime::Builder::new_current_thread().enable_all().build() {
                Ok(rt) => rt,
                Err(_) => return,
            };
            rt.block_on(run(phase, chats, events, eco, api_base, queues, selected));
        });
}

async fn run(
    phase: Arc<AtomicU8>,
    chats: ChatQueue,
    events: EventQueue,
    eco: EcoQueue,
    api: &'static str,
    queues: PollQueuesRes,
    selected: SelectedAgentRes,
) {
    // 접속 확정 대기(관전 티켓 스코프로 REST도 열림)
    loop {
        if phase.load(Ordering::Relaxed) == PHASE_CONNECTED {
            break;
        }
        tokio::time::sleep(Duration::from_secs(1)).await;
    }
    info!("[http] 관전 폴러 기동 — api {api}");
    let Ok(http) = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    else {
        return;
    };
    // 백로그 시드 1회 — 채팅(최신 30행)·이벤트(기록 팝업 즉시 채움).
    // 서버는 최신순이므로 앞 30행(=최신)을 시계순으로 뒤집어 푸시.
    if let Ok(rows) = fetch_chat_backlog(&http, api).await {
        info!("[http] 채팅 백로그 {}행 중 최신 30행 시드", rows.len());
        for r in rows.into_iter().take(30).rev() {
            chat_row_push(&chats, r);
        }
    }
    if let Ok(rows) = fetch_events_backlog(&http, api).await {
        info!("[http] 이벤트 백로그 {}행 시드", rows.len());
        // 최신순 → 시계순(드레인이 로그 뒤에 붙인다) — 최근 200행만
        if let Ok(mut q) = events.inner.lock() {
            for row in rows.into_iter().take(200).rev() {
                q.push(row);
            }
        }
    }
    // 주기 폴 — relations 20s(시작 직후 1회), history 15s(선택 주민 있을 때),
    // economy 30s(시작 직후 1회 — 401이면 조용히 재시도)
    let mut last_rel = std::time::Instant::now() - Duration::from_secs(100);
    let mut last_hist = std::time::Instant::now() - Duration::from_secs(100);
    let mut last_eco = std::time::Instant::now() - Duration::from_secs(100);
    loop {
        tokio::time::sleep(Duration::from_secs(5)).await;
        if phase.load(Ordering::Relaxed) != PHASE_CONNECTED {
            continue;
        }
        if last_rel.elapsed() >= Duration::from_secs(20) {
            last_rel = std::time::Instant::now();
            if let Ok(rows) =
                fetch_json::<Vec<RelRow>>(&http, &format!("{api}/api/worldsim/relations")).await
            {
                if let Ok(mut q) = queues.relations.lock() {
                    *q = rows;
                }
            }
        }
        if let Some(id) = selected.0.lock().ok().and_then(|g| g.clone())
            && last_hist.elapsed() >= Duration::from_secs(15)
        {
            last_hist = std::time::Instant::now();
            if let Ok(rows) = fetch_json::<Vec<crate::net::HistRow>>(
                &http,
                &format!("{api}/api/worldsim/agents/{id}/history"),
            )
            .await
            {
                if let Ok(mut q) = queues.history.lock() {
                    *q = rows;
                }
            }
        }
        if last_eco.elapsed() >= Duration::from_secs(30) {
            last_eco = std::time::Instant::now();
            if let Ok(text) = fetch_text(&http, &format!("{api}/api/worldsim/economy")).await
                && let Some(d) = parse_eco(&text)
            {
                if let Ok(mut q) = eco.0.lock() {
                    *q = Some(d);
                }
            }
        }
    }
}

async fn fetch_json<T: serde::de::DeserializeOwned>(
    http: &reqwest::Client,
    url: &str,
) -> Result<T, String> {
    let text = fetch_text(http, url).await?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

async fn fetch_text(http: &reqwest::Client, url: &str) -> Result<String, String> {
    http.get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .text()
        .await
        .map_err(|e| e.to_string())
}

/// /economy → EcoData(정본 파서 재사용 — 인젤로프 경로와 단일 구현)
fn parse_eco(text: &str) -> Option<crate::net::EcoData> {
    let v: serde_json::Value = serde_json::from_str(text).ok()?;
    crate::net::eco_from_json(v)
}

/// 백로그 채팅 행(REST /chat — 룸 chat과 같은 모양)
#[derive(serde::Deserialize)]
struct BacklogRow {
    #[serde(default, rename = "agentId")]
    agent_id: Option<String>,
    #[serde(default, rename = "agent_id")]
    agent_id2: Option<String>,
    #[serde(default)]
    speaker: Option<String>,
    #[serde(default)]
    text: Option<String>,
    #[serde(default, rename = "game_min")]
    game_min: Option<i64>,
    #[serde(default)]
    ts: Option<String>,
}

async fn fetch_chat_backlog(
    http: &reqwest::Client,
    api: &str,
) -> Result<Vec<crate::net::ChatRow>, String> {
    let rows: Vec<BacklogRow> =
        fetch_json(http, &format!("{api}/api/worldsim/chat?limit=30")).await?;
    Ok(rows
        .into_iter()
        .map(|r| crate::net::ChatRow {
            agent_id: r.agent_id.or(r.agent_id2).unwrap_or_default(),
            speaker: r.speaker.unwrap_or_default(),
            text: r.text.unwrap_or_default(),
            gmin: r.game_min,
            ts: r.ts,
        })
        .collect())
}

/// 이벤트 백로그(REST /events) — 원시 JSON 행 문자열(최신순).
/// {id, ts, tick, kind, agent_id, payload} 모양이 룸 방송과 동일하다.
async fn fetch_events_backlog(
    http: &reqwest::Client,
    api: &str,
) -> Result<Vec<String>, String> {
    let text = fetch_text(http, &format!("{api}/api/worldsim/events?limit=200")).await?;
    let v: serde_json::Value = serde_json::from_str(&text).map_err(|e| e.to_string())?;
    let Some(arr) = v.as_array() else {
        return Ok(Vec::new());
    };
    Ok(arr.iter().map(|r| r.to_string()).collect())
}
