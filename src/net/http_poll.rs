//! HTTP 관전 폴러(네이티브 전용) — 백로그 시딩(events·chat) + 관계(20s)·
//! 생활 기록(선택 주민, 15s). 룸 재접속 루프(native_conn once)와 생명주기를
//! 분리한 "tf-http" 스레드 — NetLinkPhase 원자값만 관찰해 Connected일 때만
//! 동작한다(데모·TF_NO_NET에서 서버를 때리지 않는다). 폴 주기는 웹 포트
//! 선례(tf-ui.js: relations 20s·history 15s·백로그 1회+2.5s×3 재시도).

use super::feed::{FeedQueues, PollStore};
use super::{PHASE_CONNECTED, native_conn};
use log::{info, warn};
use serde_json::Value;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::time::{Duration, Instant};

const TICK_MS: u64 = 500;
const REL_EVERY: Duration = Duration::from_secs(20);
const HIST_EVERY: Duration = Duration::from_secs(15);
const SEED_RETRY: Duration = Duration::from_millis(2500);

pub fn spawn(feeds: FeedQueues, store: PollStore, phase: Arc<AtomicU8>) {
    let _ = std::thread::Builder::new()
        .name("tf-http".into())
        .spawn(move || {
            if catch_unwind(AssertUnwindSafe(|| run_blocking(feeds, store, phase))).is_err() {
                eprintln!("[net] HTTP 폴러 패닉 — 중단(관전 나머지는 유지)");
            }
        });
}

fn run_blocking(feeds: FeedQueues, store: PollStore, phase: Arc<AtomicU8>) {
    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return;
    };
    rt.block_on(run(feeds, store, phase));
}

async fn run(feeds: FeedQueues, store: PollStore, phase: Arc<AtomicU8>) {
    let (api, _) = native_conn::pick_bases();
    info!("[net] HTTP 폴러 기동 — api {api}");
    let Ok(http) = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    else {
        return;
    };
    let seeded = AtomicBool::new(false);
    let mut was_connected = false;
    let mut next_rel = Instant::now();
    let mut hist_id: Option<String> = None;
    let mut next_hist = Instant::now();
    loop {
        tokio::time::sleep(Duration::from_millis(TICK_MS)).await;
        if phase.load(Ordering::Relaxed) != PHASE_CONNECTED {
            was_connected = false; // 재접속 에지에서 재시딩 준비
            continue;
        }
        if !was_connected {
            was_connected = true;
            seeded.store(false, Ordering::Relaxed);
        }
        if !seeded.load(Ordering::Relaxed) {
            if seed_backlog(&http, api, &feeds).await {
                seeded.store(true, Ordering::Relaxed);
            } else {
                tokio::time::sleep(SEED_RETRY).await; // 2.5s×N 재시도(웹 선례)
            }
            continue;
        }
        if Instant::now() >= next_rel {
            poll_relations(&http, api, &store).await;
            next_rel = Instant::now() + REL_EVERY;
        }
        let want = store
            .want_history
            .lock()
            .unwrap_or_else(|e| e.into_inner())
            .clone();
        match want {
            Some(id) if hist_id.as_deref() != Some(id.as_str()) || Instant::now() >= next_hist => {
                poll_history(&http, api, &id, &store).await;
                hist_id = Some(id);
                next_hist = Instant::now() + HIST_EVERY;
            }
            None => hist_id = None,
            Some(_) => {}
        }
    }
}

async fn get_json(http: &reqwest::Client, url: &str) -> Result<Value, String> {
    http.get(url)
        .send()
        .await
        .map_err(|e| e.to_string())?
        .error_for_status()
        .map_err(|e| format!("HTTP {e}"))?
        .json()
        .await
        .map_err(|e| e.to_string())
}

/// 접속 에지 백로그 — events(200행)·chat(60행)을 시드 큐에 오래된 순으로.
async fn seed_backlog(http: &reqwest::Client, api: &str, feeds: &FeedQueues) -> bool {
    let Ok(ev) = get_json(http, &format!("{api}/api/worldsim/events")).await else {
        warn!("[net] 백로그 events 폴 실패 — 재시도");
        return false;
    };
    let Ok(ch) = get_json(http, &format!("{api}/api/worldsim/chat")).await else {
        warn!("[net] 백로그 chat 폴 실패 — 재시도");
        return false;
    };
    let mut n_ev = 0usize;
    if let Some(rows) = ev.as_array() {
        let mut q = feeds.event_seed.lock().unwrap_or_else(|e| e.into_inner());
        for v in rows {
            if let Some(row) = super::feed::event_from_http(v) {
                q.push(row);
                n_ev += 1;
            }
        }
    }
    let mut n_ch = 0usize;
    if let Some(rows) = ch.as_array() {
        let mut q = feeds.chat_seed.lock().unwrap_or_else(|e| e.into_inner());
        for v in rows {
            if let Some(row) = super::feed::chat_from_http(v) {
                q.push(row);
                n_ch += 1;
            }
        }
    }
    info!("[net] 백로그 시딩 — 이벤트 {n_ev}행 · 채팅 {n_ch}행");
    true
}

async fn poll_relations(http: &reqwest::Client, api: &str, store: &PollStore) {
    let Ok(v) = get_json(http, &format!("{api}/api/worldsim/relations")).await else {
        return; // 조용한 실패 — 다음 폴에서 회복
    };
    let Some(rows) = v.as_array() else {
        return;
    };
    let parsed: Vec<_> = rows
        .iter()
        .filter_map(|r| {
            Some(super::feed::RelRow {
                a_id: r.get("a_id")?.as_str()?.to_owned(),
                b_id: r.get("b_id")?.as_str()?.to_owned(),
                warmth: r.get("warmth").and_then(Value::as_f64).unwrap_or(0.0) as f32,
                chats: r.get("chats").and_then(Value::as_u64).unwrap_or(0) as u32,
                trades: r.get("trades").and_then(Value::as_u64).unwrap_or(0) as u32,
            })
        })
        .collect();
    *store.relations.lock().unwrap_or_else(|e| e.into_inner()) = parsed;
}

async fn poll_history(http: &reqwest::Client, api: &str, id: &str, store: &PollStore) {
    let Ok(v) = get_json(http, &format!("{api}/api/worldsim/agents/{id}/history")).await else {
        return;
    };
    let Some(rows) = v.as_array() else {
        return;
    };
    let parsed: Vec<_> = rows
        .iter()
        .filter_map(|r| {
            Some(super::feed::HistRow {
                ts: r.get("ts")?.as_str()?.to_owned(),
                type_: r.get("type").and_then(Value::as_str).unwrap_or("").to_owned(),
                text: r.get("text").and_then(Value::as_str).unwrap_or("").to_owned(),
            })
        })
        .collect();
    *store.history.lock().unwrap_or_else(|e| e.into_inner()) = parsed;
}
