//! 표시 계층 번역(§17.51) — 영문 정본(발명 목적·생활 기록·목표 폴백)을
//! 관전 언어로. 레거시 FE translate.ts 직역: 세션 캐시 + 디바운스 배치
//! (40건/0.4s) + POST /api/worldsim/translate(관전 공개, docs §17.57).
//! 백엔드가 DB 공유 캐시(worldsim_translations)를 우선 조회 — 한 번 번역된
//! 문장은 모든 사용자가 즉시 히트한다.
//!
//! 실패·미도착 시 원문 그대로(표시가 막히지 않는다). 채팅은 예외(§17.55 —
//! 발화 원문이 정본). 네이티브(모바일 앱)는 라이브 관전이 아니므로 미지원 —
//! 요청 없이 원문.

use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use super::i18n::Lang;

/// 번역 캐시·요청 상태. 표시 시점 get() — 히트면 번역문, 미스면 원문 반환 후
/// 요청 등록(다음 프레임부터 번역문으로 바뀐다).
#[derive(Resource, Default)]
pub struct MtCache {
    map: HashMap<String, String>,
    pending: HashSet<String>,
    queue: Vec<String>,
    last_flush: f32,
}

impl MtCache {
    /// 표시 문자열 — ko면 캐시 조회(미스 시 요청 등록), en·빈문자열은 원문.
    pub fn get(&mut self, orig: &str, lang: Lang) -> String {
        if lang == Lang::En || orig.is_empty() {
            return orig.to_owned();
        }
        if let Some(t) = self.map.get(orig) {
            return t.clone();
        }
        if !self.pending.contains(orig) && self.queue.len() + self.pending.len() < 600 {
            self.queue.push(orig.to_owned());
            self.pending.insert(orig.to_owned());
        }
        orig.to_owned()
    }

    /// 응답 수신(인벨로프 'mt') — 캐시 적재·pending 해제
    pub fn receive(&mut self, texts: &serde_json::Value) {
        let Some(obj) = texts.as_object() else { return };
        for (k, v) in obj {
            if let Some(t) = v.as_str() {
                self.pending.remove(k);
                self.map.insert(k.clone(), t.to_owned());
            }
        }
    }

    /// 이벤트 문장 렌더 + 자유문 폴백 번역 — vocab.event_text가 템플릿을
    /// 못 찾아 payload.text/goal 원문(영문)으로 물러난 경우만 MT를 건다
    /// (템플릿 렌더는 이미 로케일 네이티브 — 번역하면 이중 번역, §17.51).
    pub fn event_text(
        &mut self,
        vocab: &super::vocab::Vocab,
        lang: Lang,
        payload: &serde_json::Value,
    ) -> String {
        let t = vocab.event_text(lang, payload);
        let raw = payload
            .get("text")
            .or_else(|| payload.get("goal"))
            .and_then(serde_json::Value::as_str)
            .unwrap_or("");
        if lang != Lang::En && !raw.is_empty() && t == raw {
            return self.get(&t, lang);
        }
        t
    }
}

/// 배치 플러시 — 0.4s 디바운스로 최대 40건을 JS 브리지(worldsim 포크의
/// window.terrafolk.translate)로. 포크가 배치·CORS를 맡는다.
pub fn flush(
    time: Res<Time>,
    mut cache: ResMut<MtCache>,
    mut announced: Local<bool>,
) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if !*announced {
            *announced = true;
            info!("[mt] 네이티브 미지원 — 표시 원문 유지");
        }
        cache.queue.clear();
        cache.pending.clear();
        return;
    }
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        let now = time.elapsed_secs();
        if cache.queue.is_empty() || now - cache.last_flush < 0.4 {
            return;
        }
        cache.last_flush = now;
        let n = cache.queue.len().min(40);
        let batch: Vec<String> = cache.queue.drain(..n).collect();
        let Ok(arr) = serde_json::to_string(&batch) else { return };
        if let Some(win) = web_sys::window()
            && let Ok(tf) = js_sys::Reflect::get(&win, &"terrafolk".into())
            && let Ok(ui) = js_sys::Reflect::get(&tf, &"ui".into())
            && let Ok(f) = js_sys::Reflect::get(&ui, &"translate".into())
            && let Ok(f) = f.dyn_into::<js_sys::Function>()
        {
            let js = wasm_bindgen::JsValue::from_str(&arr);
            // target 고정 'ko' — proto2d 로케일이 ko/en 2종이라(en은 MT 자체를
            // 건너뛴다) 요청 시점 언어와 무관하게 ko 번역만 존재한다.
            let _ = f.call2(&ui, &js, &"ko".into());
        }
    }
}
