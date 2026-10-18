//! i18n 표시 계층 — 하이라이트 판정·색·아이콘표·시각 포맷(FE parts.tsx·
//! tf-i18n.js 직역). 렌더 코어(i18n.rs)와 분리해 500줄 규칙을 지킨다.

use bevy::prelude::*;
use serde_json::Value;

use super::i18n::{event_text, t_or};
use crate::net::feed::{LiveEvent, Toast};

// ---- 하이라이트 판정(FE parts.tsx highlightOf·tf-i18n.js 직역) ----

/// 우하단 토스트·기록 하이라이트 탭 공유 판정. None이면 대상 아님(잡음).
/// ok/action/carry 조건·분기 순서까지 FE 그대로 — name_of는 주민 이름 조회 주입.
/// 아이콘은 폰트 세이프 한자·기호(NotoSansKR 글리프 보증).
pub fn highlight_of(ev: &LiveEvent, name_of: &dyn Fn(&str) -> String) -> Option<Toast> {
    let p = ev.payload.get("payload").cloned().unwrap_or(Value::Null);
    let p = p.as_object()?;
    let ok = p.get("ok").and_then(Value::as_bool).unwrap_or(false);
    let action = p.get("action").and_then(Value::as_str).unwrap_or("");
    let has_carry = p.get("carry").is_some();
    let mut who = if ev.agent_id.is_empty() {
        String::new()
    } else {
        name_of(&ev.agent_id)
    };
    let mut text = String::new();
    let icon: &'static str = match ev.kind.as_str() {
        "relation" => {
            who.clear();
            "♥"
        }
        "trade" if ok => "換",
        "give" if ok => "贈",
        "skill" => "技",
        "teach" if ok => "敎",
        "inherit" => {
            who.clear();
            "傳"
        }
        "birth" if ok => {
            who.clear();
            "生"
        }
        "routine" => {
            who.clear();
            "常"
        }
        "perform" if ok => "戲",
        "chronicle" => {
            who.clear();
            "史"
        }
        "home" => {
            who.clear();
            "家"
        }
        "spawn" => {
            text = format!(
                "{} — {}",
                p.get("name").and_then(Value::as_str).unwrap_or(""),
                who
            )
            .trim()
            .to_string();
            "迎"
        }
        "result" if action == "campfire" && ok => "火",
        "result" if (action == "drink" || action == "fetch") && ok => "水",
        "result" if action == "thirst" && !ok => "渴",
        "result" if (action == "store" || action == "withdraw") && ok => "藏",
        "result" if (action == "drop" || action == "pickup") && ok => "置",
        _ if has_carry => "重",
        "result" if action == "wear" && ok => "衣",
        "result" if action == "build" && ok => "建",
        "invention" => {
            who.clear();
            "器"
        }
        "talk" => {
            who.clear();
            "話"
        }
        "death" => {
            who.clear();
            let fallback = format!(
                "{} (aged {})",
                p.get("name").and_then(Value::as_str).unwrap_or(""),
                p.get("age").map(|a| a.to_string()).unwrap_or_else(|| "?".into())
            );
            text = event_text(&Value::Object(p.clone()));
            if text.is_empty() {
                text = fallback;
            }
            "逝"
        }
        "system" if p.contains_key("weather") => {
            who.clear();
            "世"
        }
        _ => return None,
    };
    if text.is_empty() {
        text = event_text(&Value::Object(p.clone()));
    }
    if who.is_empty() && text.is_empty() {
        return None;
    }
    Some(Toast { icon, who, text })
}

// ---- 색·아이콘표(FE parts.tsx 상수 직역) ----

/// 주민별 안정 색 — id 해시×31 팔레트 8색.
pub fn agent_color(id: &str) -> Color {
    const PALETTE: [Color; 8] = [
        Color::srgb(1.0, 0.824, 0.541),
        Color::srgb(0.604, 0.816, 1.0),
        Color::srgb(0.647, 0.910, 0.690),
        Color::srgb(0.941, 0.659, 0.784),
        Color::srgb(0.847, 0.761, 1.0),
        Color::srgb(1.0, 0.690, 0.627),
        Color::srgb(0.627, 0.910, 0.878),
        Color::srgb(0.910, 0.878, 0.604),
    ];
    let mut h: u32 = 0;
    for c in id.chars() {
        h = h.wrapping_mul(31).wrapping_add(c as u32);
    }
    PALETTE[(h % 8) as usize]
}

/// 이벤트 분류 — 기록 팝업 7탭(전체·하이라이트 + 5분류) 매핑.
pub fn ev_cat_of(kind: &str) -> &'static str {
    match kind {
        "relation" | "give" | "talk" | "chat" | "perform" => "social",
        "act" | "result" | "trade" | "think" => "living",
        "skill" | "invention" | "teach" | "inherit" | "routine" => "knowledge",
        "spawn" | "birth" | "death" => "life",
        "system" | "resource" | "chronicle" => "world",
        _ => "living",
    }
}

pub const EV_CATS: [&str; 5] = ["social", "living", "knowledge", "life", "world"];

/// 종류 아이콘(한자 치환) — 기록 팝업 행.
pub fn ev_kind_icon(kind: &str) -> &'static str {
    match kind {
        "act" => "動",
        "think" => "思",
        "result" => "成",
        "resource" => "採",
        "trade" => "換",
        "give" => "贈",
        "relation" => "♥",
        "skill" => "技",
        "spawn" => "迎",
        "death" => "逝",
        "invention" => "器",
        "talk" => "話",
        "teach" => "敎",
        "inherit" => "傳",
        "birth" => "生",
        "routine" => "常",
        "system" => "世",
        "chat" => "言",
        "perform" => "戲",
        "chronicle" => "史",
        _ => "·",
    }
}

/// 종류색(FE EVENT_COLOR 직역).
pub fn event_color(kind: &str) -> Color {
    match kind {
        "act" => Color::srgb(0.490, 0.827, 0.988),
        "result" => Color::srgb(0.682, 0.718, 0.776),
        "think" => Color::srgb(0.769, 0.710, 0.992),
        "trade" => Color::srgb(0.988, 0.827, 0.302),
        "skill" => Color::srgb(0.431, 0.906, 0.718),
        "chat" => Color::srgb(0.800, 0.827, 0.871),
        "resource" => Color::srgb(0.745, 0.949, 0.392),
        "spawn" => Color::srgb(0.420, 0.463, 0.537),
        "system" => Color::srgb(0.290, 0.329, 0.408),
        "relation" | "give" => Color::srgb(0.976, 0.659, 0.831),
        "death" => Color::srgb(0.831, 0.831, 0.847),
        "invention" => Color::srgb(0.992, 0.902, 0.541),
        "teach" => Color::srgb(0.655, 0.953, 0.816),
        "inherit" => Color::srgb(0.729, 0.902, 0.992),
        "birth" => Color::srgb(0.984, 0.812, 0.910),
        "routine" => Color::srgb(0.867, 0.839, 0.996),
        "talk" => Color::srgb(0.996, 0.953, 0.780),
        "perform" => Color::srgb(0.961, 0.816, 0.996),
        "chronicle" => Color::srgb(0.996, 0.953, 0.780),
        _ => Color::srgb(0.682, 0.718, 0.776),
    }
}

/// 생활 기록 유형색(FE HIST_COLOR 직역).
pub fn hist_color(type_: &str) -> Color {
    match type_ {
        "gain" | "획득" => Color::srgb(0.431, 0.906, 0.718),
        "trade" | "거래" => Color::srgb(0.988, 0.827, 0.302),
        "craft" | "제작" => Color::srgb(0.490, 0.827, 0.988),
        "consume" | "소모" => Color::srgb(0.992, 0.643, 0.686),
        _ => Color::srgb(0.682, 0.718, 0.776),
    }
}

/// 생활 기록 유형 라벨.
pub fn hist_label(type_: &str) -> String {
    t_or(&format!("hist.{type_}"), type_)
}

/// 활동 배지 아이콘(FE ACT_ICON 한자 치환) — 머리 위 링 안.
pub fn act_icon(act: &str) -> Option<&'static str> {
    Some(match act {
        "chop" => "伐",
        "mine" => "採",
        "forage" => "集",
        "hunt" => "狩",
        "eat" => "食",
        "drink" => "水",
        "campfire" => "火",
        "warm" => "溫",
        "wear" => "衣",
        "give" => "贈",
        "trade" => "換",
        "gather" => "拾",
        "build" => "建",
        "invent" => "創",
        "craft" => "作",
        "perform" => "戲",
        _ => return None,
    })
}

// ---- 시각 포맷 ----

/// 게임 분 → "D{day} {hh}:{mm} · Y{year}"(FE fmtGmin 직역).
pub fn fmt_gmin(gmin: i64) -> String {
    let day = gmin.div_euclid(1440);
    let min = gmin.rem_euclid(1440);
    let (year, _, _, _) = super::hud::date_of(day as u32);
    format!("D{day} {:02}:{:02} · Y{year}", min / 60, min % 60)
}

/// ISO ts → "HH:MM"(UTC 직독 — 로컬 시각 보정은 언어 설정 추가 시).
pub fn fmt_time(ts: &str) -> String {
    // "2026-10-18T09:05:40.572Z" → "09:05"
    if let Some(t) = ts.split('T').nth(1) {
        let hhmm: String = t.chars().take(5).collect();
        if hhmm.len() == 5 && hhmm.as_bytes()[2] == b':' {
            return hhmm;
        }
    }
    String::new()
}
