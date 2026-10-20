//! 발명 아이콘 캐시(§17.36) — worldsim 포크의 캔버스 합성(재색상 SVG →
//! 16px 픽셀 스냅 → 2x nearest) 결과를 base64 PNG로 받아 egui Image로 띄운다.
//! 레거시 itemIcon.ts 직역 — 결정적(같은 스펙=같은 픽셀), 이모지 폴백 대신
//! 아이콘 없음 폴백(칩 텍스트만).

use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

/// 아이콘 스펙(서버 직렬화 그대로 — docs §17.30)
#[derive(serde::Deserialize, Clone, Debug)]
pub struct IconSpec {
    pub tpl: String,
    #[serde(default)]
    pub primary: String,
    #[serde(default)]
    pub accent: String,
}

#[derive(Resource, Default)]
pub struct IconCache {
    /// key "{tpl}|{primary}|{accent}" → PNG 바이트(실패는 None — 재요청 없음)
    map: HashMap<String, Option<Vec<u8>>>,
    pending: HashSet<String>,
    queue: Vec<(String, IconSpec)>,
    last_flush: f32,
}

impl IconCache {
    /// 표시용 아이콘 바이트 — 캐시 히트면 Some, 미스면 요청 등록 후 None.
    /// None이면 호출부는 아이콘 없이 텍스트만(로딩 스피너 없음 — 다음 프레임 치환).
    pub fn get(&mut self, spec: &IconSpec) -> Option<&[u8]> {
        let key = format!("{}|{}|{}", spec.tpl, spec.primary, spec.accent);
        if let Some(v) = self.map.get(&key) {
            return v.as_deref();
        }
        if !self.pending.contains(&key)
            && self.queue.len() + self.pending.len() < 120
            && let Some(spec) = valid(spec)
        {
            self.pending.insert(key.clone());
            self.queue.push((key, spec));
        }
        None
    }

    /// 응답 수신(인벨로프 'icon') — items [{key, png(base64)}]
    pub fn receive(&mut self, items: &serde_json::Value) {
        let Some(arr) = items.as_array() else { return };
        for it in arr {
            let Some(key) = it.get("key").and_then(|v| v.as_str()) else { continue };
            let png = it
                .get("png")
                .and_then(|v| v.as_str())
                .and_then(decode_base64_png);
            self.pending.remove(key);
            self.map.insert(key.to_owned(), png);
        }
    }
}

/// 벤더 아이콘 목록에 있는 템플릿만(존재하지 않으면 요청 낭비).
/// 벤더: proto2d/assets/web/icons(레거시 public/assets/icons와 동일 15종).
const TPLS: [&str; 15] = [
    "arrow", "arrows", "axe", "basket", "bow", "fishing", "hammer", "harpoon", "knife", "needle",
    "pot", "saw", "sling", "spear", "trap",
];

fn valid(s: &IconSpec) -> Option<IconSpec> {
    TPLS
        .contains(&s.tpl.as_str())
        .then(|| s.clone())
}

/// base64 PNG 디코드 — 표준 크레이트 없이 최소 구현(테이블 6bit).
/// wasm 크기 민감(29MB) — base64 크레이트 추가보다 30줄이 싸다.
fn decode_base64_png(s: &str) -> Option<Vec<u8>> {
    let mut out = Vec::with_capacity(s.len() * 3 / 4);
    let mut acc: u32 = 0;
    let mut bits = 0;
    for c in s.bytes() {
        let v = match c {
            b'A'..=b'Z' => (c - b'A') as u32,
            b'a'..=b'z' => (c - b'a' + 26) as u32,
            b'0'..=b'9' => (c - b'0' + 52) as u32,
            b'+' => 62,
            b'/' => 63,
            b'\r' | b'\n' | b'=' => continue,
            _ => return None,
        };
        acc = (acc << 6) | v;
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            out.push((acc >> bits) as u8);
        }
    }
    // PNG 매직 확인 — 깨진 응답은 None(원문 폴백)
    (out.starts_with(&[0x89, b'P', b'N', b'G'])).then_some(out)
}

/// 배치 플러시 — 0.5s·8건. worldsim 포크 ui.renderIcons(JSON)로 요청하면
/// 결과가 {tf:'icon'} 인벨로프로 돌아온다(net/mod.rs가 IconCache에 적재).
pub fn flush(time: Res<Time>, mut cache: ResMut<IconCache>, mut announced: Local<bool>) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if !*announced {
            *announced = true;
            info!("[icon] 네이티브 미지원 — 아이콘 없이 텍스트");
        }
        cache.queue.clear();
        cache.pending.clear();
        return;
    }
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        let now = time.elapsed_secs();
        if cache.queue.is_empty() || now - cache.last_flush < 0.5 {
            return;
        }
        cache.last_flush = now;
        let n = cache.queue.len().min(8);
        let batch: Vec<(String, IconSpec)> = cache.queue.drain(..n).collect();
        let reqs: Vec<serde_json::Value> = batch
            .iter()
            .map(|(k, s)| {
                serde_json::json!({
                    "key": k, "tpl": s.tpl, "primary": s.primary, "accent": s.accent,
                })
            })
            .collect();
        let Ok(json) = serde_json::to_string(&reqs) else { return };
        if let Some(win) = web_sys::window()
            && let Ok(tf) = js_sys::Reflect::get(&win, &"terrafolk".into())
            && let Ok(ui) = js_sys::Reflect::get(&tf, &"ui".into())
            && let Ok(f) = js_sys::Reflect::get(&ui, &"renderIcons".into())
            && let Ok(f) = f.dyn_into::<js_sys::Function>()
        {
            let _ = f.call1(&ui, &wasm_bindgen::JsValue::from_str(&json));
        }
    }
}
