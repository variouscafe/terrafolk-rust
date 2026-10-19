//! HTTP 관전 폴러(네이티브 전용) — 관계 목록·선택 주민 생활기록·채팅 백로그.
//! root src/net/http_poll.rs의 축소 포트: 룸 접속(PHASE_CONNECTED) 후에만 동작.
//! relations 20s 주기, history는 선택 주민 있을 때 15s.

use crate::net::{
    ChatQueue, HistRow, PHASE_CONNECTED, PollQueuesRes, RelRow,
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
            rt.block_on(run(phase, chats, api_base, queues, selected));
        });
}

async fn run(
    phase: Arc<AtomicU8>,
    chats: ChatQueue,
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
    // 채팅 백로그 1회 시드(최근 60행)
    if let Ok(rows) = fetch_chat_backlog(&http, api).await {
        info!("[http] 채팅 백로그 {}행", rows.len());
        for r in rows.into_iter().rev() {
            chat_row_push(&chats, r);
        }
    }
    // 주기 폴 — relations 20s(시작 직후 1회), history 15s(선택 주민 있을 때)
    let mut last_rel = std::time::Instant::now() - Duration::from_secs(100);
    let mut last_hist = std::time::Instant::now() - Duration::from_secs(100);
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
            if let Ok(rows) = fetch_json::<Vec<HistRow>>(
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
    }
}

async fn fetch_json<T: serde::de::DeserializeOwned>(
    http: &reqwest::Client,
    url: &str,
) -> Result<T, String> {
    http.get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| e.to_string())?
        .json::<T>()
        .await
        .map_err(|e| e.to_string())
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
}

async fn fetch_chat_backlog(
    http: &reqwest::Client,
    api: &str,
) -> Result<Vec<crate::net::ChatRow>, String> {
    let rows: Vec<BacklogRow> = fetch_json(http, &format!("{api}/api/worldsim/chat")).await?;
    Ok(rows
        .into_iter()
        .map(|r| crate::net::ChatRow {
            agent_id: r.agent_id.or(r.agent_id2).unwrap_or_default(),
            speaker: r.speaker.unwrap_or_default(),
            text: r.text.unwrap_or_default(),
        })
        .collect())
}
