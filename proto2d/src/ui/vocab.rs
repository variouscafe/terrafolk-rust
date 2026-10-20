//! 어휘·이벤트 사전 — shared/vocab.json·events.json을 런타임 에셋으로 로드.
//! 레거시 FE dict.ts+vocab.gen.ts+events.gen.ts의 데이터 정본 그대로
//! (임베드 대신 에셋 — wasm 크기 예산 보존, 폰트와 같은 비동기 로드 패턴).
//! 이벤트 문장 렌더(조사 판정·라벨 치환)도 여기가 정본.

use bevy::asset::io::Reader;
use bevy::asset::{AssetLoader, LoadContext};
use bevy::prelude::*;
use bevy_egui::egui::Color32;
use serde_json::Value;
use std::collections::HashMap;

use super::i18n::Lang;

/// 원시 JSON 에셋 — 어휘·이벤트 사전 로드용(커스텀 로더)
#[derive(Asset, TypePath)]
pub struct JsonAsset(pub Value);

#[derive(Default, TypePath)]
pub struct JsonLoader;

impl AssetLoader for JsonLoader {
    type Asset = JsonAsset;
    type Settings = ();
    type Error = std::io::Error;

    fn load(
        &self,
        reader: &mut dyn Reader,
        _settings: &Self::Settings,
        _load_context: &mut LoadContext<'_>,
    ) -> impl bevy::tasks::ConditionalSendFuture<Output = Result<Self::Asset, Self::Error>> {
        Box::pin(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await?;
            let v = serde_json::from_slice::<Value>(&bytes).map_err(std::io::Error::other)?;
            Ok(JsonAsset(v))
        })
    }

    fn extensions(&self) -> &[&str] {
        &["json"]
    }
}

/// 라벨 사중목(ko, en, zh, ja) — vocab labels[4] 전체
#[derive(Clone)]
pub struct Lbl(pub String, pub String, pub String, pub String);

impl Lbl {
    pub fn pick(&self, lang: Lang) -> &str {
        match lang {
            Lang::Ko => &self.0,
            Lang::En => &self.1,
            Lang::Zh => &self.2,
            Lang::Ja => &self.3,
        }
    }
}

#[derive(Default, Resource)]
pub struct Vocab {
    pub loaded: bool,
    items: HashMap<String, Lbl>,
    item_wt: HashMap<String, f32>,
    species: HashMap<String, Lbl>,
    kinds: HashMap<String, Lbl>,
    invcats: HashMap<String, Lbl>,
    invcat_group: HashMap<String, String>,
    invgroups: HashMap<String, Lbl>,
    eras: HashMap<String, Lbl>,
    seasons: HashMap<String, Lbl>,
    hists: HashMap<String, Lbl>,
    buildings: HashMap<String, Lbl>,
    bldstages: HashMap<(String, String), Lbl>,
    events: HashMap<String, Lbl>,
}

fn lbl_of(v: &Value) -> Option<Lbl> {
    let a = v.get("labels")?.as_array()?;
    if a.len() < 2 {
        return None;
    }
    let l = |i: usize| a.get(i).and_then(Value::as_str).unwrap_or_default().to_owned();
    Some(Lbl(l(0), l(1), l(2), l(3)))
}

/// 비동기 에셋 로드 → 파싱 1회(폰트 설치와 같은 Update 재시도 패턴)
pub fn load(
    mut commands: Commands,
    assets: Res<AssetServer>,
    store: Res<Assets<JsonAsset>>,
    mut vocab: ResMut<Vocab>,
    mut handles: Local<Option<[Handle<JsonAsset>; 2]>>,
) {
    if vocab.loaded {
        return;
    }
    let hs = handles.get_or_insert_with(|| {
        [
            assets.load("data/vocab.json"),
            assets.load("data/events.json"),
        ]
    });
    // 두 에셋 모두 로드 확인
    if !store.contains(&hs[0]) || !store.contains(&hs[1]) {
        return; // 다음 프레임 재시도
    }
    let Some(JsonAsset(v)) = store.get(&hs[0]) else { return };
    let Some(JsonAsset(ev)) = store.get(&hs[1]) else { return };
    let v = v.clone();
    let ev = ev.clone();
    for (id, e) in v.get("items").and_then(Value::as_object).into_iter().flatten() {
        if let Some(l) = lbl_of(e) {
            if let Some(wt) = e.get("wt").and_then(Value::as_f64) {
                vocab.item_wt.insert(id.clone(), wt as f32);
            }
            vocab.items.insert(id.clone(), l);
        }
    }
    let table = |key: &str, map: &mut HashMap<String, Lbl>| {
        if let Some(obj) = v.get(key).and_then(Value::as_object) {
            for (id, e) in obj {
                if let Some(l) = lbl_of(e) {
                    map.insert(id.clone(), l);
                }
            }
        }
    };
    table("species", &mut vocab.species);
    table("kinds", &mut vocab.kinds);
    table("inventionCategories", &mut vocab.invcats);
    table("inventionGroups", &mut vocab.invgroups);
    table("eras", &mut vocab.eras);
    table("seasons", &mut vocab.seasons);
    table("historyTypes", &mut vocab.hists);
    table("buildings", &mut vocab.buildings);
    // 발명 범주 → 6대분류(inventionCategories.group)
    if let Some(obj) = v.get("inventionCategories").and_then(Value::as_object) {
        for (id, e) in obj {
            if let Some(g) = e.get("group").and_then(Value::as_str) {
                vocab.invcat_group.insert(id.clone(), g.to_owned());
            }
        }
    }
    // 건축 단계(bldstage.{bld}.{stage} 2단 키)
    if let Some(obj) = v.get("buildings").and_then(Value::as_object) {
        for (bld, e) in obj {
            if let Some(stages) = e.get("stages").and_then(Value::as_array) {
                for st in stages {
                    if let (Some(k), Some(l)) = (st.get("key").and_then(Value::as_str), lbl_of(st)) {
                        vocab.bldstages.insert((bld.clone(), k.to_owned()), l);
                    }
                }
            }
        }
    }
    // 별칭(tool→tools·building→housing — 레거시 INVCAT_ALIAS)
    if let Some(obj) = v.get("inventionCategoryAliases").and_then(Value::as_object) {
        for (from, to) in obj {
            let (f, t) = (from.as_str(), to.as_str().unwrap_or_default());
            if let Some(l) = vocab.invcats.get(t).cloned() {
                vocab.invcats.insert(f.to_owned(), l);
            }
            if let Some(g) = vocab.invcat_group.get(t).cloned() {
                vocab.invcat_group.insert(f.to_owned(), g);
            }
        }
    }
    // events.json — ev.*·goal.* 문장 템플릿(모양이 다르다: labels 배열이
    // 아니라 ko/en 필드 직접 — vocab.json과 통일되지 않은 정본 그대로)
    if let Some(obj) = ev.get("events").and_then(Value::as_object) {
        for (k, e) in obj {
            let g = |key: &str| {
                e.get(key)
                    .and_then(Value::as_str)
                    .map(str::to_owned)
                    .unwrap_or_default()
            };
            let l = Lbl(g("ko"), g("en"), g("zh"), g("ja"));
            if !l.0.is_empty() || !l.1.is_empty() {
                vocab.events.insert(k.clone(), l);
            }
        }
    }
    vocab.loaded = true;
    info!(
        "[vocab] 사전 로드 완료 — items {} · events {}",
        vocab.items.len(),
        vocab.events.len()
    );
    commands.insert_resource(VocabReady);
}

/// 사전 준비 플래그(패널들이 즉시 폴백 라벨로 그려지지 않게 할 필요는 없다 —
/// 라벨 폴백=ID 원문이라 로드 전후 라벨만 바뀐다)
#[derive(Resource)]
pub struct VocabReady;

impl Vocab {
    fn pick<'a>(m: &'a HashMap<String, Lbl>, lang: Lang, id: &'a str) -> &'a str {
        m.get(id).map(|l| l.pick(lang)).unwrap_or(id)
    }
    pub fn item<'a>(&'a self, lang: Lang, id: &'a str) -> &'a str {
        Self::pick(&self.items, lang, id)
    }
    pub fn item_wt(&self, id: &str) -> f32 {
        // 미기재 폴백 wt=1(레거시 CarryBar 관례)
        self.item_wt.get(id).copied().unwrap_or(1.0)
    }
    pub fn sp<'a>(&'a self, lang: Lang, id: &'a str) -> &'a str {
        Self::pick(&self.species, lang, id)
    }
    pub fn kind<'a>(&'a self, lang: Lang, id: &'a str) -> &'a str {
        Self::pick(&self.kinds, lang, id)
    }
    pub fn invcat<'a>(&'a self, lang: Lang, id: &'a str) -> &'a str {
        Self::pick(&self.invcats, lang, id)
    }
    /// 범주 → 6대분류 키. 미지 범주는 None — 레거시 invGroupOf가 ''을
    /// 돌려주고 civScore가 빈 그룹을 걸러내는 것과 동일(잘못된 생활 편입·
    /// 다양성 보너스 오염 방지)
    pub fn inv_group_of(&self, cat: &str) -> Option<&str> {
        self.invcat_group.get(cat).map(String::as_str)
    }
    pub fn inv_group<'a>(&'a self, lang: Lang, group: &'a str) -> &'a str {
        Self::pick(&self.invgroups, lang, group)
    }
    pub fn era<'a>(&'a self, lang: Lang, id: &'a str) -> &'a str {
        Self::pick(&self.eras, lang, id)
    }
    pub fn hist<'a>(&'a self, lang: Lang, id: &'a str) -> &'a str {
        Self::pick(&self.hists, lang, id)
    }
    pub fn building<'a>(&'a self, lang: Lang, id: &'a str) -> &'a str {
        Self::pick(&self.buildings, lang, id)
    }
    pub fn bldstage<'a>(&'a self, lang: Lang, bld: &'a str, stage: &'a str) -> &'a str {
        self.bldstages
            .get(&(bld.to_owned(), stage.to_owned()))
            .map(|l| l.pick(lang))
            .unwrap_or(stage)
    }
    /// 이벤트/goal 키가 사전에 있는가(목표 문장 폴백 판정)
    pub fn has_event(&self, key: &str) -> bool {
        self.events.contains_key(key)
    }

    /// 이벤트 payload → 문장(레거시 eventRender 직역: key→ev.* 템플릿 우선,
    /// goalKey→goal.*, 없으면 text 원문). 한국어는 조사 토큰 판정.
    pub fn event_text(&self, lang: Lang, payload: &Value) -> String {
        if let Some(key) = payload.get("key").and_then(Value::as_str) {
            if let Some(t) = self.render_event(lang, key, payload.get("params")) {
                return t;
            }
        }
        if let Some(goal) = payload.get("goalKey").and_then(Value::as_str) {
            if let Some(t) = self.render_event(lang, goal, payload.get("params")) {
                return t;
            }
        }
        payload
            .get("text")
            .or_else(|| payload.get("goal"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_owned()
    }

    fn render_event(&self, lang: Lang, key: &str, params: Option<&Value>) -> Option<String> {
        let l = self.events.get(key)?;
        let tmpl = match lang {
            Lang::Ko => l.0.as_str(),
            Lang::En => l.1.as_str(),
            Lang::Zh => l.2.as_str(),
            Lang::Ja => l.3.as_str(),
        };
        let tmpl = if tmpl.is_empty() { l.0.as_str() } else { tmpl };
        Some(match params.and_then(Value::as_object) {
            Some(p) => self.resolve_params(lang, tmpl, p),
            None => tmpl.to_owned(),
        })
    }

    /// 템플릿 param 치환(레거시 resolveParams 직역 — 라벨 분기 + 조사 토큰)
    fn resolve_params(
        &self,
        lang: Lang,
        tmpl: &str,
        params: &serde_json::Map<String, Value>,
    ) -> String {
        let mut out = tmpl.to_string();
        for (name, raw) in params {
            let s = val_str(raw);
            let val = match name.as_str() {
                "item" | "item2" => self.item(lang, &s).to_owned(),
                "species" => self.sp(lang, &s).to_owned(),
                "cat" => self.invcat(lang, &s).to_owned(),
                "era" => self.era(lang, &s).to_owned(),
                "stage" => {
                    let (bld, stage) = s.split_once(':').unwrap_or((s.as_str(), ""));
                    if stage.is_empty() {
                        s.clone()
                    } else {
                        self.bldstage(lang, bld, stage).to_owned()
                    }
                }
                "label" => {
                    let (kind, id) = s.split_once(':').unwrap_or((s.as_str(), ""));
                    if kind == "kind" {
                        self.kind(lang, id).to_owned()
                    } else {
                        self.sp(lang, id).to_owned()
                    }
                }
                "why" => {
                    let t = s.trim();
                    if t.is_empty() {
                        String::new()
                    } else {
                        format!(" — {t}")
                    }
                }
                "bases" => s
                    .split(',')
                    .filter(|i| !i.is_empty())
                    .map(|i| self.item(lang, i))
                    .collect::<Vec<_>>()
                    .join("/"),
                "itemqtys" => s
                    .split(',')
                    .filter(|kv| kv.contains(':'))
                    .map(|kv| {
                        let (i, n) = kv.split_once(':').unwrap_or((kv, ""));
                        format!("{} {}개", self.item(lang, i), n)
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
}

/// 받침 판정(FE hasBatchim 직역)
fn has_batchim(s: &str) -> bool {
    let code = s.chars().next_back().map_or(0, |c| c as u32);
    (0xac00..=0xd7a3).contains(&code) && (code - 0xac00) % 28 > 0
}

fn josa(tag: &str, s: &str) -> &'static str {
    let b = has_batchim(s);
    match tag {
        "이가" => if b { "이가" } else { "가" },
        "을" => if b { "을" } else { "를" },
        "은" => if b { "은" } else { "는" },
        "와" => if b { "과" } else { "와" },
        "으로" => if b { "으로" } else { "로" },
        "이" => if b { "이" } else { "가" },
        _ => "",
    }
}

fn val_str(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Number(n) => n
            .as_i64()
            .map(|i| i.to_string())
            .or_else(|| n.as_f64().map(|f| f.to_string()))
            .unwrap_or_default(),
        Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

/// 문명 시대 사다리(레거시 ERA_STEPS·civScore 직역)
const ERA_STEPS: [(&str, u32); 9] = [
    ("band", 0),
    ("camp", 6),
    ("village", 12),
    ("large_village", 20),
    ("town", 32),
    ("city", 48),
    ("metropolis", 70),
    ("megacity", 100),
    ("beyond", 140),
];

/// 문명 점수 = 발명 수 + 서로 다른 그룹 수 × 2(미지 범주는 그룹 수제외)
pub fn civ_score(cats: impl Iterator<Item = Option<String>>) -> (u32, &'static str) {
    let mut groups: Vec<String> = Vec::new();
    let mut count = 0u32;
    for c in cats {
        count += 1;
        if let Some(g) = c
            && !groups.contains(&g)
        {
            groups.push(g);
        }
    }
    let score = count + groups.len() as u32 * 2;
    let mut era = "band";
    for (key, at) in ERA_STEPS {
        if score >= at {
            era = key;
        }
    }
    (score, era)
}

/// 관계 온기 등급색 — 레거시 hsl(warmth×1.2, 65%, 55%) 직역(빨강→초록)
pub fn warmth_hsl(w: f32) -> Color32 {
    let (r, g, b) = hsl_to_rgb(w.clamp(0.0, 100.0) * 1.2 / 360.0, 0.65, 0.55);
    Color32::from_rgb(r, g, b)
}

/// 표준 HSL→RGB(h∈0..1)
fn hsl_to_rgb(h: f32, s: f32, l: f32) -> (u8, u8, u8) {
    let c = (1.0 - (2.0 * l - 1.0).abs()) * s;
    let hp = (h * 6.0) % 6.0;
    let x = c * (1.0 - (hp % 2.0 - 1.0).abs());
    let (r1, g1, b1) = match hp as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = l - c / 2.0;
    (
        ((r1 + m) * 255.0).round() as u8,
        ((g1 + m) * 255.0).round() as u8,
        ((b1 + m) * 255.0).round() as u8,
    )
}
