//! 외형 편집 동기화(네이티브 전용) — 소유 주민 조회(/api/worldsim/me) 폴링과
//! look 저장(PATCH /agents/{id}/look) 스레드. 웹(tf-ui.js loadMine·lookSave)의
//! 계약 그대로: me는 Bearer 필수(무토큰=빈 목록 → 편집 버튼 미노출), 저장은
//! {parts, wears} 본문. 폴 주기 30s(loadMine 15s의 절반 — 편집기 상태 갱신
//!용도로 충분, 저장 즉시 loadMine 재폴은 저장 큐 처리 후 1회 즉시 폴로 대응).

use super::{PHASE_CONNECTED, native_conn};
use bevy::prelude::Resource;
use log::info;
use serde_json::Value;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::{Duration, Instant};

const TICK_MS: u64 = 500;
const ME_EVERY: Duration = Duration::from_secs(30);

/// 소유 주민 1인분 — /api/worldsim/me 항목(tf-ui loadMine 필드 그대로).
#[derive(Clone, Debug)]
pub struct MeAgent {
    pub id: String,
    pub name: String,
    pub sex: String,
    pub age: i64,
    /// lookJson parts(문자열 키→문자열 값이라 단순 맵으로)
    pub parts: std::collections::BTreeMap<String, String>,
    /// 착용 슬롯 선택(top/bottom/feet → item|'auto')
    pub wears: std::collections::BTreeMap<String, String>,
    /// 소지 {아이템: 수량}
    pub inventory: std::collections::BTreeMap<String, f64>,
    /// 발명 의상 {name, cf, color}
    pub clothing_inventions: Vec<InvClothes>,
}

#[derive(Clone, Debug)]
pub struct InvClothes {
    pub name: String,
    pub cf: f32,
    pub color: String,
}

/// 저장 요청 — body는 완성된 {parts, wears} JSON(계약 조립은 UI 측).
pub struct SaveReq {
    pub agent_id: String,
    pub body: String,
}

/// UI↔스레드 공유 상태(자원 — UI는 mine·status 읽고 save_q에 쓴다).
#[derive(Resource, Clone, Default)]
pub struct LookStore {
    pub inner: Arc<Shared>,
}

#[derive(Default)]
pub struct Shared {
    pub mine: Mutex<Vec<MeAgent>>,
    /// me 목록 갱신 회차 — UI가 재구성 타이밍을 잡는 키.
    pub mine_rev: Mutex<u32>,
    pub save_q: Mutex<Vec<SaveReq>>,
    /// 마지막 저장 결과 — "ok" | "err: …" | ""(미저장). UI 상태 라벨 원천.
    pub status: Mutex<String>,
}

impl LookStore {
    /// 편집기 세계관 — mine 목록에서 해당 id를 찾는 편의 접근.
    pub fn find(&self, id: &str) -> Option<MeAgent> {
        let m = self.inner.mine.lock().unwrap_or_else(|e| e.into_inner());
        m.iter().find(|a| a.id == id).cloned()
    }
    pub fn contains(&self, id: &str) -> bool {
        let m = self.inner.mine.lock().unwrap_or_else(|e| e.into_inner());
        m.iter().any(|a| a.id == id)
    }
    /// 저장 요청 적재 — 호출 즉시 스레드가 픽업(탭 변경 단위라 디바운스 불필요).
    pub fn enqueue_save(&self, agent_id: &str, body: String) {
        let mut q = self.inner.save_q.lock().unwrap_or_else(|e| e.into_inner());
        // 같은 주민의 이전 요청은 최신이 이긴다 — 직렬 송신이지만 유실은 없게.
        q.retain(|r| r.agent_id != agent_id);
        q.push(SaveReq {
            agent_id: agent_id.to_owned(),
            body,
        });
        *self.inner.status.lock().unwrap_or_else(|e| e.into_inner()) = "saving".into();
    }
}

pub fn spawn(store: LookStore, phase: Arc<AtomicU8>) {
    let _ = std::thread::Builder::new()
        .name("tf-look".into())
        .spawn(move || {
            if catch_unwind(AssertUnwindSafe(|| run_blocking(store, phase))).is_err() {
                eprintln!("[net] look 동기화 스레드 패닉 — 중단(편집 기능만 상실)");
            }
        });
}

fn run_blocking(store: LookStore, phase: Arc<AtomicU8>) {
    let Ok(rt) = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
    else {
        return;
    };
    rt.block_on(run(store, phase));
}

async fn run(store: LookStore, phase: Arc<AtomicU8>) {
    let (api, _) = native_conn::pick_bases();
    info!("[net] look 동기화 기동 — api {api}");
    let Ok(http) = reqwest::Client::builder()
        .timeout(Duration::from_secs(10))
        .build()
    else {
        return;
    };
    let mut next_me = Instant::now();
    loop {
        tokio::time::sleep(Duration::from_millis(TICK_MS)).await;
        if phase.load(Ordering::Relaxed) != PHASE_CONNECTED {
            continue; // 룸 접속 전엔 me도 저장도 하지 않는다(http_poll 관례)
        }
        drain_saves(&http, api, &store).await;
        if Instant::now() >= next_me {
            if poll_me(&http, api, &store).await {
                next_me = Instant::now() + ME_EVERY;
            } else {
                next_me = Instant::now() + Duration::from_secs(5); // 조용한 재시도
            }
        }
    }
}

/// look 저장 — 직렬 처리, 결과는 status에. 저장 성공은 me 재폴을 즉시 예약.
async fn drain_saves(http: &reqwest::Client, api: &str, store: &LookStore) {
    let reqs: Vec<SaveReq> = {
        let mut q = store.inner.save_q.lock().unwrap_or_else(|e| e.into_inner());
        q.drain(..).collect()
    };
    if reqs.is_empty() {
        return;
    }
    let Some(token) = native_conn::load_auth().access else {
        *store.inner.status.lock().unwrap_or_else(|e| e.into_inner()) = "err: 로그인 필요".into();
        return;
    };
    for r in reqs {
        let url = format!("{api}/api/worldsim/agents/{}/look", r.agent_id);
        let res = http
            .patch(&url)
            .bearer_auth(&token)
            .header("Content-Type", "application/json")
            .body(r.body)
            .send()
            .await;
        let verdict = match res {
            Ok(resp) if resp.status().is_success() => "ok".to_owned(),
            Ok(resp) => format!("err: HTTP {}", resp.status()),
            Err(e) => format!("err: {e}"),
        };
        info!("[net] look 저장 {} → {verdict}", r.agent_id);
        *store.inner.status.lock().unwrap_or_else(|e| e.into_inner()) = verdict;
    }
}

/// /api/worldsim/me — 무토큰·401은 정상 상황(관전) — 조용히 빈 목록.
async fn poll_me(http: &reqwest::Client, api: &str, store: &LookStore) -> bool {
    let Some(token) = native_conn::load_auth().access else {
        return true; // 로그인 안 함 — 재시도 불필요
    };
    let Ok(resp) = http
        .get(format!("{api}/api/worldsim/me"))
        .bearer_auth(&token)
        .send()
        .await
    else {
        return false;
    };
    if !resp.status().is_success() {
        return false;
    }
    let Ok(v) = resp.json::<Value>().await else {
        return false;
    };
    let Some(rows) = v.as_array() else {
        return false;
    };
    let mut parsed = Vec::new();
    for r in rows {
        if let Some(a) = parse_me(r) {
            parsed.push(a);
        }
    }
    info!("[net] 소유 주민 {}명 갱신", parsed.len());
    *store.inner.mine.lock().unwrap_or_else(|e| e.into_inner()) = parsed;
    *store
        .inner
        .mine_rev
        .lock()
        .unwrap_or_else(|e| e.into_inner()) += 1;
    true
}

fn parse_me(r: &Value) -> Option<MeAgent> {
    let str_map = |v: Option<&Value>| -> std::collections::BTreeMap<String, String> {
        v.and_then(Value::as_object)
            .map(|o| {
                o.iter()
                    .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_owned())))
                    .collect()
            })
            .unwrap_or_default()
    };
    let num_map = |v: Option<&Value>| -> std::collections::BTreeMap<String, f64> {
        v.and_then(Value::as_object)
            .map(|o| {
                o.iter()
                    .filter_map(|(k, v)| v.as_f64().map(|n| (k.clone(), n)))
                    .collect()
            })
            .unwrap_or_default()
    };
    Some(MeAgent {
        id: r.get("id")?.as_str()?.to_owned(),
        name: r
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned(),
        sex: r
            .get("sex")
            .and_then(Value::as_str)
            .unwrap_or("female")
            .to_owned(),
        age: r.get("age").and_then(Value::as_i64).unwrap_or(40),
        parts: str_map(r.get("parts")),
        wears: str_map(r.get("wears")),
        inventory: num_map(r.get("inventory")),
        clothing_inventions: r
            .get("clothingInventions")
            .and_then(Value::as_array)
            .map(|arr| {
                arr.iter()
                    .filter_map(|c| {
                        Some(InvClothes {
                            name: c.get("name")?.as_str()?.to_owned(),
                            cf: c.get("cf").and_then(Value::as_f64).unwrap_or(1.0) as f32,
                            color: c
                                .get("color")
                                .and_then(Value::as_str)
                                .unwrap_or("brown")
                                .to_owned(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default(),
    })
}
