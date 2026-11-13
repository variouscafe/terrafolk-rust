//! 네이티브 자유 텍스트 기계번역(§17.118) — 웹 tf-i18n.js translateText 포트.
//! 서버는 영문 정본을 저장하고, 표시 계층만 화면 로케일로 번역한다.
//! 캐시 미스면 원문을 즉시 돌려주고 큐에 넣는다(UI는 번역을 기다리지 않음).
//! 번역이 도착하면 generation이 오르고 패널이 다시 그린다.
//! 순수 로직(Cache·Queue)은 I/O와 분리해 단위 테스트한다. 주민 이름은 번역 금지.

use bevy::prelude::*;
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, OnceLock};
use std::time::{Duration, Instant};

use super::NetState;
use crate::ui::i18n::ui_locale;

pub const CACHE_CAP: usize = 4000;
pub const BATCH_MAX: usize = 40;
pub const TEXT_MAX: usize = 300;
const DEBOUNCE: Duration = Duration::from_millis(250);
const BACKOFF_MIN: Duration = Duration::from_secs(1);
const BACKOFF_MAX: Duration = Duration::from_secs(30);

type Key = (String, String);

// ---- 순수 로직 ----

/// 번역 대상 정규화 — 앞뒤 공백 제거. 빈 문자열·300자 초과·글자 없는 텍스트는 None.
/// (300자 초과는 API 한도라 잘라 번역하면 내용이 사라지므로 원문 그대로 둔다.)
pub fn normalize(text: &str) -> Option<&str> {
    let t = text.trim();
    if t.is_empty() || t.chars().count() > TEXT_MAX || !t.chars().any(char::is_alphabetic) {
        return None;
    }
    Some(t)
}

fn has_hangul(s: &str) -> bool {
    s.chars().any(|c| ('\u{ac00}'..='\u{d7a3}').contains(&c))
}

/// 번역이 필요한가 — en 로케일(원문이 영문)·이미 한글인 ko 텍스트는 불필요.
pub fn needs_translation(locale: &str, text: &str) -> bool {
    if locale == "en" || locale.is_empty() {
        return false;
    }
    !(locale == "ko" && has_hangul(text))
}

/// FIFO 상한 캐시.
pub struct Cache {
    map: HashMap<Key, String>,
    order: VecDeque<Key>,
    cap: usize,
}

impl Cache {
    pub fn new(cap: usize) -> Self {
        Self { map: HashMap::new(), order: VecDeque::new(), cap: cap.max(1) }
    }
    pub fn get(&self, locale: &str, text: &str) -> Option<&str> {
        self.map.get(&(locale.to_owned(), text.to_owned())).map(String::as_str)
    }
    pub fn insert(&mut self, locale: &str, text: &str, out: &str) {
        let key = (locale.to_owned(), text.to_owned());
        if self.map.insert(key.clone(), out.to_owned()).is_none() {
            self.order.push_back(key);
        }
        while self.map.len() > self.cap {
            if let Some(old) = self.order.pop_front() {
                self.map.remove(&old);
            }
        }
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}

/// 요청 큐 — 중복 제거·40개 분할·지수 백오프.
#[derive(Default)]
pub struct Queue {
    pending: Vec<Key>,
    /// 큐·전송·재시도 중인 키(중복 요청 방지)
    known: HashSet<Key>,
    retry: Vec<(Key, Instant)>,
    fails: u32,
}

impl Queue {
    /// 새로 큐에 넣었으면 true(이미 대기·전송·백오프 중이면 false).
    pub fn request(&mut self, locale: &str, text: &str) -> bool {
        let key = (locale.to_owned(), text.to_owned());
        if !self.known.insert(key.clone()) {
            return false;
        }
        self.pending.push(key);
        true
    }
    pub fn has_work(&self) -> bool {
        !self.pending.is_empty() || !self.retry.is_empty()
    }
    /// 재시도 시각이 지난 것까지 모아 로케일별 ≤40개 배치로 분할.
    pub fn take_batches(&mut self, now: Instant) -> Vec<(String, Vec<String>)> {
        let (ready, wait): (Vec<_>, Vec<_>) = self.retry.drain(..).partition(|(_, at)| *at <= now);
        self.retry = wait;
        self.pending.extend(ready.into_iter().map(|(k, _)| k));
        let mut by_loc: Vec<(String, Vec<String>)> = Vec::new();
        for (loc, text) in self.pending.drain(..) {
            match by_loc.iter_mut().find(|(l, _)| *l == loc) {
                Some((_, v)) => v.push(text),
                None => by_loc.push((loc, vec![text])),
            }
        }
        let mut out = Vec::new();
        for (loc, texts) in by_loc {
            for chunk in texts.chunks(BATCH_MAX) {
                out.push((loc.clone(), chunk.to_vec()));
            }
        }
        out
    }
    /// 전송 성공 — 키 해제, 백오프 초기화.
    pub fn done(&mut self, locale: &str, texts: &[String]) {
        for t in texts {
            self.known.remove(&(locale.to_owned(), t.clone()));
        }
        self.fails = 0;
    }
    /// 전송 실패 — 지수 백오프(1s→2s→…30s) 후 재시도, 그동안 재요청 금지(known 유지).
    pub fn fail(&mut self, locale: &str, texts: &[String], now: Instant) -> Duration {
        let wait = (BACKOFF_MIN * (1u32 << self.fails.min(5))).min(BACKOFF_MAX);
        self.fails = self.fails.saturating_add(1);
        for t in texts {
            self.retry.push(((locale.to_owned(), t.clone()), now + wait));
        }
        wait
    }
}

// ---- 공유 상태 ----

#[derive(Default)]
struct Inner {
    cache: Mutex<Option<Cache>>,
    queue: Mutex<Queue>,
    /// 워커 → 메인 결과(로케일, 원문, 번역)
    results: Mutex<Vec<(String, String, String)>>,
    names: Mutex<HashSet<String>>,
    generation: AtomicU64,
}

fn lock<T>(m: &Mutex<T>) -> MutexGuard<'_, T> {
    m.lock().unwrap_or_else(|e| e.into_inner())
}

static GLOBAL: OnceLock<Arc<Inner>> = OnceLock::new();

/// 번역 캐시 리소스 — 패널은 `generation()` 변화로 재빌드를 판단한다.
#[derive(Resource, Clone)]
pub struct MtCache(Arc<Inner>);

impl Default for MtCache {
    fn default() -> Self {
        let inner = Arc::new(Inner::default());
        *lock(&inner.cache) = Some(Cache::new(CACHE_CAP));
        Self(inner)
    }
}

impl MtCache {
    pub fn generation(&self) -> u64 {
        self.0.generation.load(Ordering::Relaxed)
    }
    /// 캐시 히트만 조회(큐잉 없음).
    pub fn get(&self, text: &str) -> Option<String> {
        let t = normalize(text)?;
        lock(&self.0.cache).as_ref()?.get(ui_locale(), t).map(str::to_owned)
    }
    /// 표시용 — 히트면 번역, 아니면 원문 + 요청 큐잉.
    pub fn mt(&self, text: &str) -> String {
        let loc = ui_locale();
        let Some(t) = normalize(text) else { return text.to_owned() };
        if !needs_translation(loc, t) || lock(&self.0.names).contains(t) {
            return text.to_owned();
        }
        if let Some(hit) = lock(&self.0.cache).as_ref().and_then(|c| c.get(loc, t)) {
            return hit.to_owned();
        }
        lock(&self.0.queue).request(loc, t);
        text.to_owned()
    }
    fn set_names(&self, names: impl Iterator<Item = String>) {
        *lock(&self.0.names) = names.collect();
    }
}

/// 전역 접근 — i18n 렌더 함수(시그니처 변경 없이)가 쓴다. 플러그인 미등록이면 원문.
pub fn global_mt(text: &str) -> String {
    match GLOBAL.get() {
        Some(inner) => MtCache(inner.clone()).mt(text),
        None => text.to_owned(),
    }
}

// ---- 워커 ----

fn spawn_worker(inner: Arc<Inner>) {
    let _ = std::thread::Builder::new().name("tf-mt".into()).spawn(move || {
        let Ok(rt) = tokio::runtime::Builder::new_current_thread().enable_all().build() else {
            return;
        };
        rt.block_on(worker(inner));
    });
}

async fn worker(inner: Arc<Inner>) {
    let (api, _) = super::native_conn::pick_bases();
    let Ok(http) = reqwest::Client::builder().timeout(Duration::from_secs(15)).build() else {
        return;
    };
    loop {
        tokio::time::sleep(DEBOUNCE).await;
        let batches = {
            let mut q = lock(&inner.queue);
            if !q.has_work() {
                continue;
            }
            q.take_batches(Instant::now())
        };
        for (loc, texts) in batches {
            match post(&http, api, &loc, &texts).await {
                Some(map) => {
                    let mut res = lock(&inner.results);
                    for t in &texts {
                        // 응답에 없는 키는 원문 유지(재요청 방지로 캐시에 원문 저장)
                        let dst = map.get(t).cloned().unwrap_or_else(|| t.clone());
                        res.push((loc.clone(), t.clone(), dst));
                    }
                    drop(res);
                    lock(&inner.queue).done(&loc, &texts);
                }
                None => {
                    let wait = lock(&inner.queue).fail(&loc, &texts, Instant::now());
                    log::warn!("[mt] 번역 실패 — {wait:?} 후 재시도");
                }
            }
        }
    }
}

async fn post(http: &reqwest::Client, api: &str, locale: &str, texts: &[String]) -> Option<HashMap<String, String>> {
    let body = json!({ "texts": texts, "target": locale });
    let v: Value = http
        .post(format!("{api}/api/worldsim/translate"))
        .json(&body)
        .send()
        .await
        .ok()?
        .error_for_status()
        .ok()?
        .json()
        .await
        .ok()?;
    let obj = v.get("texts")?.as_object()?;
    Some(
        obj.iter()
            .filter_map(|(k, v)| Some((k.clone(), v.as_str()?.to_owned())))
            .collect(),
    )
}

// ---- 플러그인 ----

/// 워커 결과를 캐시에 반영하고 generation을 올린다. 주민 이름(번역 제외)도 1Hz 동기.
fn apply_mt_results(mt: Res<MtCache>, net: Res<NetState>, mut tick: Local<u32>) {
    let drained: Vec<_> = std::mem::take(&mut *lock(&mt.0.results));
    if !drained.is_empty() {
        if let Some(c) = lock(&mt.0.cache).as_mut() {
            for (loc, src, dst) in &drained {
                c.insert(loc, src, dst);
            }
        }
        mt.0.generation.fetch_add(1, Ordering::Relaxed);
    }
    *tick = tick.wrapping_add(1);
    if (*tick).is_multiple_of(30)
        && let Some(f) = net.frame.as_ref()
    {
        mt.set_names(f.agents.iter().map(|a| a.name.clone()));
    }
}

pub struct MtPlugin;

impl Plugin for MtPlugin {
    fn build(&self, app: &mut App) {
        let mt = MtCache::default();
        let _ = GLOBAL.set(mt.0.clone());
        spawn_worker(mt.0.clone());
        app.insert_resource(mt).add_systems(Update, apply_mt_results);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_trims_and_rejects() {
        assert_eq!(normalize("  hello  "), Some("hello"));
        assert_eq!(normalize("   "), None);
        assert_eq!(normalize("123 45"), None);
        assert_eq!(normalize(&"a".repeat(301)), None);
        assert!(normalize(&"a".repeat(300)).is_some());
    }

    #[test]
    fn en_and_hangul_skip() {
        assert!(!needs_translation("en", "hello"));
        assert!(!needs_translation("ko", "안녕"));
        assert!(needs_translation("ko", "hello"));
        assert!(needs_translation("ja", "안녕"));
    }

    #[test]
    fn queue_dedups() {
        let mut q = Queue::default();
        assert!(q.request("ko", "a"));
        assert!(!q.request("ko", "a"));
        assert!(q.request("ja", "a"));
        let b = q.take_batches(Instant::now());
        assert_eq!(b.iter().map(|(_, t)| t.len()).sum::<usize>(), 2);
    }

    #[test]
    fn queue_splits_at_40() {
        let mut q = Queue::default();
        for i in 0..95 {
            q.request("ko", &format!("t{i}"));
        }
        let b = q.take_batches(Instant::now());
        let sizes: Vec<_> = b.iter().map(|(_, t)| t.len()).collect();
        assert_eq!(sizes, vec![40, 40, 15]);
    }

    #[test]
    fn queue_backoff_doubles_and_caps() {
        let mut q = Queue::default();
        let now = Instant::now();
        let txt = vec!["x".to_owned()];
        assert_eq!(q.fail("ko", &txt, now), Duration::from_secs(1));
        assert_eq!(q.fail("ko", &txt, now), Duration::from_secs(2));
        for _ in 0..8 {
            q.fail("ko", &txt, now);
        }
        assert_eq!(q.fail("ko", &txt, now), Duration::from_secs(30));
    }

    #[test]
    fn retry_waits_and_blocks_rerequest() {
        let mut q = Queue::default();
        q.request("ko", "x");
        let now = Instant::now();
        let b = q.take_batches(now);
        q.fail("ko", &b[0].1, now);
        assert!(!q.request("ko", "x"), "백오프 중 재요청 금지");
        assert!(q.take_batches(now).is_empty());
        assert_eq!(q.take_batches(now + Duration::from_secs(2)).len(), 1);
    }

    #[test]
    fn done_resets_and_allows_again() {
        let mut q = Queue::default();
        q.request("ko", "x");
        let b = q.take_batches(Instant::now());
        q.fail("ko", &b[0].1, Instant::now());
        q.done("ko", &b[0].1);
        assert!(q.request("ko", "x"));
        assert_eq!(q.fails, 0);
    }

    #[test]
    fn cache_fifo_evicts_oldest() {
        let mut c = Cache::new(3);
        for i in 0..5 {
            c.insert("ko", &format!("k{i}"), "v");
        }
        assert_eq!(c.len(), 3);
        assert!(c.get("ko", "k0").is_none());
        assert!(c.get("ko", "k4").is_some());
    }

    #[test]
    fn cache_keyed_by_locale() {
        let mut c = Cache::new(10);
        c.insert("ko", "a", "가");
        assert_eq!(c.get("ko", "a"), Some("가"));
        assert!(c.get("ja", "a").is_none());
    }

    #[test]
    fn mt_miss_returns_original_and_queues_hit_returns_translation() {
        let m = MtCache::default();
        assert_eq!(m.mt("Stone axe"), "Stone axe");
        assert!(lock(&m.0.queue).has_work());
        lock(&m.0.cache).as_mut().unwrap().insert("ko", "Stone axe", "돌도끼");
        assert_eq!(m.mt(" Stone axe "), "돌도끼");
        assert_eq!(m.mt(""), "");
    }

    #[test]
    fn mt_skips_villager_names() {
        let m = MtCache::default();
        m.set_names(["Mina".to_owned()].into_iter());
        assert_eq!(m.mt("Mina"), "Mina");
        assert!(!lock(&m.0.queue).has_work());
    }
}
