//! 네이티브 i18n 렌더 — FE eventRender.ts·assets/web/tf-i18n.js 직역 포트.
//! 이벤트 키→한국어 문장(조사 판정·라벨 치환), 하이라이트 판정, 색·아이콘표.
//! 이모지는 NotoSansKR에 글리프가 없어 폰트 세이프 한자·기호로 치환한다
//! (theme::weather_icon 선례 — 에뮬레이터 스크린샷으로 글리프 확인).

use bevy::prelude::*;
use serde_json::Value;

use super::i18n_data::{EVENT_DICT, UI_D, VOCAB};
use super::theme;
/// 표시 계층(i18n_view) 재수출 — 호출부는 i18n:: 단일 경로 유지.
pub use super::i18n_view::{
    EV_CATS, act_icon, agent_color, event_color, ev_cat_of, ev_kind_icon, fmt_gmin, fmt_time,
    highlight_of, hist_color, hist_label,
};

/// 정렬된 (key, ko, en) 표 조회 — ko 우선, 빈값이면 en.
fn find3(tbl: &'static [(&'static str, &'static str, &'static str)], key: &str) -> Option<(&'static str, &'static str)> {
    let (ko, en) = tbl
        .binary_search_by(|probe| probe.0.cmp(key))
        .ok()
        .map(|i| (tbl[i].1, tbl[i].2))?;
    if ko.is_empty() && en.is_empty() {
        None
    } else {
        Some((ko, en))
    }
}

/// 로케일 확정값 — 네이티브는 한국어 고정(설정 언어 기능 추가 시 확장).
fn pick((ko, en): (&'static str, &'static str)) -> String {
    if !ko.is_empty() {
        fx(ko)
    } else {
        fx(en)
    }
}

/// 템플릿·표시문 이모지 → 폰트 세이프 한자 치환. NotoSansKR 단일 폰트라
/// 색 이모지 글리프가 없다(bevy 0.19 TextFont은 폴백 스택 부재) — events.json
/// ko 템플릿에 내장된 10종+무덤을 아이콘 어휘와 같은 한자로 바꾼다.
pub fn fx(s: &str) -> String {
    s.replace('🎭', "戲")
        .replace('👶', "生")
        .replace('💝', "贈")
        .replace('📜', "史")
        .replace('🕯', "逝")
        .replace('🤝', "換")
        .replace('🤲', "傳")
        .replace('🧭', "常")
        .replace('🪔', "燈")
        .replace('🪙', "錢")
        .replace('🪦', "墓")
}

/// UI 문구 — D 사전(t 패밀리). 없으면 None.
pub fn t(key: &str) -> Option<String> {
    find3(UI_D, key).map(pick)
}

pub fn t_or(key: &str, fallback: &str) -> String {
    t(key).unwrap_or_else(|| fallback.into())
}

/// UI 문구 + param 치환({k} 토큰) — tip.* 조립용.
pub fn t_fmt(key: &str, params: &[(&str, &str)]) -> String {
    let mut s = t(key).unwrap_or_else(|| key.into());
    for (k, v) in params {
        s = s.replace(&format!("{{{k}}}"), v);
    }
    s
}

/// 어휘 공개 라벨(bld.{type} 등 — 툴팁·표시 계층용).
pub fn vocab_label(prefix: &str, id: &str) -> String {
    label4(prefix, id)
}

/// 어휘 라벨(item.{id}·sp.{id} 등) — 없으면 id 원문.
fn label4(prefix: &str, id: &str) -> String {
    find3(VOCAB, &format!("{prefix}.{id}")).map(pick).unwrap_or_else(|| id.into())
}

/// 행동 라벨 — act.{code} 우선, 서버 status 구값은 기존 status_label 폴백.
pub fn act_label(code: &str) -> String {
    if let Some(s) = t(&format!("act.{code}")) {
        return s;
    }
    theme::status_label(code).into()
}

pub fn need_label(i: usize) -> String {
    t_or(&["needs.hunger", "needs.thirst", "needs.fatigue", "needs.warmth"][i], "?")
}

/// 관계 등급 — FE REL_BUCKET(<30 bad·<45 awkward·<65 normal·<80 good·else best).
pub fn rel_bucket(w: f32) -> &'static str {
    match w as i32 {
        ..30 => "bad",
        ..45 => "awkward",
        ..65 => "normal",
        ..80 => "good",
        _ => "best",
    }
}

pub fn rel_label(w: f32) -> String {
    t_or(&format!("rel.{}", rel_bucket(w)), rel_bucket(w))
}

// ---- 조사 판정(FE eventRender.ts hasBatchim·JOSA 직역) ----

fn has_batchim(s: &str) -> bool {
    let code = s.chars().next_back().map_or(0, |c| c as u32);
    (0xac00..=0xd7a3).contains(&code) && (code - 0xac00) % 28 > 0
}

fn josa(tag: &str, s: &str) -> &'static str {
    let batchim = has_batchim(s);
    match tag {
        "이가" => if batchim { "이가" } else { "가" },
        "을" => if batchim { "을" } else { "를" },
        "은" => if batchim { "은" } else { "는" },
        "와" => if batchim { "과" } else { "와" },
        "으로" => if batchim { "으로" } else { "로" },
        "이" => if batchim { "이" } else { "가" },
        _ => "",
    }
}

/// JSON 값 → 치환 문자열(정수 표기 오염 방지).
fn val_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                i.to_string()
            } else {
                n.as_f64().map(|f| f.to_string()).unwrap_or_default()
            }
        }
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

/// 템플릿 param 치환 — FE resolveParams 직역(item/species/dir/skill/cat/era/
/// stage/label/why/acts/bases/seasons/itemqtys 분기 + 조사 토큰).
fn resolve_params(tmpl: &str, params: &serde_json::Map<String, Value>) -> String {
    let mut out = tmpl.to_string();
    for (name, raw) in params {
        let s = val_str(raw);
        let val = match name.as_str() {
            "item" | "item2" => label4("item", &s),
            "species" => label4("sp", &s),
            "dir" => label4("dir", &s),
            "skill" => t(&format!("act.{s}")).unwrap_or_else(|| s.clone()),
            "cat" => label4("invcat", &s),
            "era" => label4("era", &s),
            "stage" => {
                // "bld:stage" 2단 키
                let (bld, stage) = s.split_once(':').unwrap_or((s.as_str(), ""));
                if stage.is_empty() {
                    s.clone()
                } else {
                    find3(VOCAB, &format!("bldstage.{bld}.{stage}"))
                        .map(pick)
                        .unwrap_or_else(|| s.clone())
                }
            }
            "label" => {
                // "kind:id" | "species:id"
                let (kind, id) = s.split_once(':').unwrap_or((s.as_str(), ""));
                if kind == "kind" {
                    label4("kind", id)
                } else {
                    label4("sp", id)
                }
            }
            "why" => {
                let trimmed = s.trim();
                if trimmed.is_empty() {
                    String::new()
                } else {
                    format!(" — {trimmed}")
                }
            }
            "acts" => s
                .split(',')
                .filter(|a| !a.is_empty())
                .map(|a| t(&format!("act.{a}")).unwrap_or_else(|| a.into()))
                .collect::<Vec<_>>()
                .join("·"),
            "bases" => s
                .split(',')
                .filter(|i| !i.is_empty())
                .map(|i| label4("item", i))
                .collect::<Vec<_>>()
                .join("/"),
            "seasons" => s
                .split(',')
                .filter(|x| !x.is_empty())
                .map(|x| t(&format!("season.{x}")).unwrap_or_else(|| x.into()))
                .collect::<Vec<_>>()
                .join("·"),
            "itemqtys" => s
                .split(',')
                .filter(|kv| kv.contains(':'))
                .map(|kv| {
                    let (i, n) = kv.split_once(':').unwrap_or((kv, ""));
                    format!("{} {}개", label4("item", i), n)
                })
                .collect::<Vec<_>>()
                .join(", "),
            _ => s.clone(),
        };
        for tag in ["이가", "을", "은", "와", "으로", "이"] {
            let token = format!("{{{name}{tag}}}");
            if out.contains(&token) {
                let rep = format!("{val}{}", josa(tag, &val));
                out = out.replace(&token, &rep);
            }
        }
        out = out.replace(&format!("{{{name}}}"), &val);
    }
    out
}

/// 이벤트 키 렌더 — 사전에 없는 키는 None(호출부 폴백).
pub fn render_event_key(key: &str, params: Option<&serde_json::Map<String, Value>>) -> Option<String> {
    let (ko, en) = find3(EVENT_DICT, key)?;
    let tmpl = if !ko.is_empty() { ko } else { en };
    Some(match params {
        Some(p) => fx(&resolve_params(tmpl, p)),
        None => fx(tmpl),
    })
}

fn params_of(v: &Value, k: &str) -> Option<serde_json::Map<String, Value>> {
    v.get(k)
        .and_then(Value::as_object)
        .filter(|m| !m.is_empty())
        .cloned()
}

/// 이벤트 payload → 문장 — key 우선, think는 goalKey, 없으면 text 원문.
pub fn event_text(p: &Value) -> String {
    if let Some(key) = p.get("key").and_then(Value::as_str) {
        if let Some(out) = render_event_key(key, params_of(p, "params").as_ref()) {
            return out;
        }
    }
    if let Some(key) = p.get("goalKey").and_then(Value::as_str) {
        if let Some(out) = render_event_key(key, params_of(p, "goalParams").as_ref()) {
            return out;
        }
    }
    p.get("text")
        .or_else(|| p.get("goal"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .into()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tables_sorted_for_binary_search() {
        assert!(EVENT_DICT.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(VOCAB.windows(2).all(|w| w[0].0 < w[1].0));
        assert!(UI_D.windows(2).all(|w| w[0].0 < w[1].0));
    }

    #[test]
    fn renders_josa_item_and_emoji() {
        let mut p = serde_json::Map::new();
        p.insert("name".into(), "아라".into());
        p.insert("age".into(), 42.into());
        assert_eq!(
            render_event_key("ev.death", Some(&p)).unwrap(),
            "逝 아라가 눈을 감고 다시 뜨지 않았다 (향년 42세)"
        );
        // 받침 없는 이름 — '가' 조사
        p.insert("name".into(), "두리".into());
        let out = render_event_key("ev.death", Some(&p)).unwrap();
        assert!(out.contains("두리가 눈을 감고"));
        // item 라벨 치환 + 수량
        let mut q = serde_json::Map::new();
        q.insert("item".into(), "wood".into());
        q.insert("n".into(), 3.into());
        let key = EVENT_DICT
            .iter()
            .find(|(k, ko, _)| ko.contains("{item") && ko.contains("{n}"))
            .map(|(k, _, _)| *k)
            .expect("수량 템플릿");
        let out = render_event_key(key, Some(&q)).unwrap();
        assert!(out.contains("목재"), "item 라벨 치환: {out}");
    }

    #[test]
    fn goal_prefers_key_render() {
        assert!(!goal_text("ev.sleep.wake", "woke up").is_empty());
        assert_eq!(goal_text("", "raw goal"), "raw goal");
    }
}

/// 목표 문장 — AgentHud의 goal_key 렌더 우선, 없으면 goal_text 원문.
pub fn goal_text(goal_key: &str, goal_text: &str) -> String {
    if !goal_key.is_empty() {
        if let Some(out) = render_event_key(goal_key, None) {
            return out;
        }
    }
    if goal_text.is_empty() {
        String::new()
    } else {
        goal_text.into()
    }
}
