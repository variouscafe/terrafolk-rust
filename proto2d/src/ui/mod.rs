//! egui UI — 레거시 FE 관전 기능 패리티(전 플랫폼 단일 구현).
#![allow(deprecated)] // egui 0.34 최상위 Panel::show 폐기 예정 — 동작 유지
//! 셸(상단바)은 shell.rs, 주민 패널은 villagers.rs, 기록·경제는 각 파일 —
//! 500줄 규칙 분리. 어휘·이벤트 사전(vocab.rs)은 런타임 에셋 로드.

pub mod chat;
pub mod civ;
pub mod economy;
pub mod events;
pub mod fonts;
pub mod i18n;
pub mod menu;
pub mod minimap;
pub mod mt;
pub mod relations;
pub mod relic;
pub mod resources;
pub mod shell;
pub mod theme;
pub mod toasts;
pub mod tooltip;
pub mod villagers;
pub mod vocab;
pub mod widgets;

use bevy::prelude::*;
use bevy_egui::{EguiContexts, EguiPrimaryContextPass, egui};

// ── 패널 치수 단일 진실원(리뷰 2026-10-20) ──
/// 주민 목록 우측 도킹 폭 — villagers.rs 패널과 conversation 카드의
/// 남은 맵 영역 계산(chat.rs)이 같은 값을 써야 한다.
pub const VILLAGERS_W: f32 = 272.0;
/// 채팅 하단 도킹 높이(데스크톱/협소) — chat.rs 패널과 카드 배치 공용.
pub const CHAT_H_DESKTOP: f32 = 148.0;
pub const CHAT_H_NARROW: f32 = 94.0;

/// 채팅 로그(말풍선과 별개 UI 패널 소스) — 최근 120행(FE 버퍼 패리티)
#[derive(Resource, Default)]
pub struct ChatLog {
    pub rows: Vec<ChatLogRow>,
}

pub struct ChatLogRow {
    /// 도착 시각(초) — 폴백 스탬프(예비)
    #[allow(dead_code)]
    pub time: f32,
    /// 화자 주민 id — 이름 색·근접 말풍선 정합
    pub agent_id: String,
    pub speaker: String,
    pub text: String,
    /// 발화 게임 분(절대) — 시각 스탬프(FE fmtGmin 패리티)
    pub gmin: Option<i64>,
    pub ts: Option<String>,
}

/// 관전 타임라인(구조화) — 최근 400행. 문장 렌더는 표시 시점(vocab 사전).
#[derive(Resource, Default)]
pub struct EventLog {
    pub rows: Vec<EventRow>,
}

/// 이벤트 행 — net::RawEvent의 UI 소비형(payload 보존)
#[derive(Debug, Clone)]
pub struct EventRow {
    pub kind: String,
    pub agent_id: String,
    pub payload: serde_json::Value,
    pub ts: Option<String>,
    #[allow(dead_code)] // 계약 정본 필드 — 정렬·필터 확장 예약
    pub tick: u32,
}

impl From<crate::net::RawEvent> for EventRow {
    fn from(r: crate::net::RawEvent) -> Self {
        EventRow {
            kind: r.kind,
            agent_id: r.agent_id,
            payload: r.payload,
            ts: r.ts,
            tick: r.tick,
        }
    }
}

/// UI 상태 — 패널 표시 토글
#[derive(Resource)]
pub struct UiState {
    pub show_chat: bool,
    pub show_villagers: bool,
    pub show_events: bool,
    pub show_minimap: bool,
    pub show_civ: bool,
    pub show_relations: bool,
    pub show_economy: bool,
    pub show_resources: bool,
    pub show_settings: bool,
    /// 사용자가 ☰ 메뉴로 패널을 조작했는가 — 반응형 와치독 정지 조건
    pub user_adjusted: bool,
    pub lang: i18n::Lang,
}

impl Default for UiState {
    fn default() -> Self {
        UiState {
            show_chat: true,
            show_villagers: true,
            show_events: false,
            show_minimap: true,
            show_civ: false,
            show_relations: false,
            show_economy: false,
            show_resources: false,
            show_settings: false,
            user_adjusted: false,
            lang: i18n::Lang::default(),
        }
    }
}

pub struct UiPlugin;

impl Plugin for UiPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ChatLog>()
            .init_resource::<EventLog>()
            .init_resource::<UiState>()
            .init_resource::<relic::RelicCard>()
            .init_resource::<mt::MtCache>()
            .init_resource::<vocab::Vocab>()
            .init_resource::<menu::SkinOverride>()
            .init_resource::<toasts::ToastQ>()
            .init_resource::<minimap::MinimapBake>()
            .init_asset::<vocab::JsonAsset>()
            .register_asset_loader(vocab::JsonLoader)
            .add_plugins(bevy_egui::EguiPlugin::default())
            // 폰트 설치는 Update — 에셋이 비동기라 Startup 1회로는 절대 로드 전
            // 완료 못 함(웹 전체 두부 원인, 2026-10-20 실측). done 플래그로
            // 설치 후엔 매 프레임 조기 반환.
            // theme::apply — Startup 금지: egui 컨텍스트가 첫 프레임에 없으면
            // 실패 후 재시도 불가(폰트 설치와 같은 교훈). Update+done 플래그로
            // 컨텍스트 생길 때까지 재시도한다.
            .add_systems(Update, theme::apply)
            .add_systems(
                Update,
                (
                    fonts::install_korean_font,
                    vocab::load,
                    toasts::collect,
                    responsive_defaults,
                    debug_panel_cmds,
                    mt::flush,
                ),
            )
            .add_systems(
                EguiPrimaryContextPass,
                (
                    shell::top_bar,
                    villagers::panel,
                    chat::panel,
                    chat::conversation,
                    events::panel,
                    minimap::panel,
                    menu::panel,
                    toasts::show,
                    civ::panel,
                    relations::panel,
                    resources::panel,
                    economy::panel,
                    relic::show,
                    tooltip::show,
                ),
            );
    }
}

/// ISO ts → "HH:MM"(UTC 직독 — FE fmtTime 직역). 없으면 빈칸
pub fn fmt_hhmm(ts: Option<&str>) -> String {
    ts.and_then(|t| t.split('T').nth(1))
        .map(|t| t.chars().take(5).collect::<String>())
        .filter(|s| s.len() == 5 && s.as_bytes()[2] == b':')
        .unwrap_or_default()
}

/// 이벤트 하이라이트 판정(레거시 highlightOf 축소 직역) — 토스트·기록
/// 하이라이트 탭이 공유. 세부 ok/캐리 조건은 payload를 본다.
pub fn is_highlight(kind: &str, payload: &serde_json::Value) -> bool {
    let ok = payload.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
    // 캐리 존재는 종 무관 하이라이트(레거시 highlightOf 동일)
    if payload.get("carry").is_some() {
        return true;
    }
    match kind {
        "relation" | "skill" | "inherit" | "routine" | "invention" | "talk" | "death"
        | "chronicle" | "home" => true,
        "birth" | "trade" | "give" | "teach" | "perform" => ok,
        "result" => {
            let action = payload.get("action").and_then(|v| v.as_str()).unwrap_or("");
            ok && matches!(action, "campfire" | "drink" | "fetch" | "store" | "withdraw" | "drop" | "pickup" | "wear" | "build")
        }
        "spawn" => true,
        "system" => payload.get("weather").is_some(),
        _ => false,
    }
}

/// egui가 포인터/키보드를 점유 중인가 — 월드 입력(클릭 선택·탭·WASD 판)은
/// 이때 스킵해야 UI 조작이 세지 않는다(패널 클릭 → 즉시 선택 해제 사고 방지).
pub fn egui_owns_input(contexts: &mut bevy_egui::EguiContexts) -> bool {
    contexts
        .ctx_mut()
        .ok()
        .map(|c| c.egui_wants_pointer_input() || c.wants_keyboard_input())
        .unwrap_or(false)
}

/// egui 컨텍스트 헬퍼 — 멀티패스 프라이머리 컨텍스트 획득
pub(crate) fn ctx(contexts: &mut EguiContexts) -> Option<egui::Context> {
    contexts.ctx_mut().ok().cloned()
}

/// 모바일 반응형 기본값 — 좁은 화면(폰)은 사이드 패널·미니맵 접기 시작.
/// 상단바 토글로 언제든 전개 가능(기능 손실 없음).
/// 모바일 반응형 — Update 와치독(Startup의 창 폭은 웹에서 1280 오측이라
/// 못 믿는다 — 실제 뷰포트가 확정되는 몇 프레임 뒤에 접힘을 확정).
/// 사용자가 ☰ 메뉴를 조작한 뒤엔 간섭하지 않는다.
fn responsive_defaults(
    mut ui_state: ResMut<UiState>,
    windows: Query<&Window, With<bevy::window::PrimaryWindow>>,
    mut announced: Local<bool>,
) {
    if ui_state.user_adjusted {
        return;
    }
    let Ok(w) = windows.single() else { return };
    if w.width() < 700.0 {
        ui_state.show_villagers = false;
        ui_state.show_minimap = false;
        ui_state.show_chat = false;
        if !*announced {
            *announced = true;
            info!("[ui] 모바일 레이아웃 — 패널 접힘 기본(폭 {:.0})", w.width());
        }
    }
}

/// E2E 패널 구동 훅(웹) — `window.terrafolk.ui.panelCmds` 문자열 배열을
/// 드레인해 패널 토글을 적용한다. 헤드리스 SwiftShader 저FPS 환경에선
/// ☰ 메뉴 팝업이 프레스에도 열리지 않아(2026-10-21 실측 — 버튼 프레스
/// 스타일은 확인되나 팝업 미출현) 좌표 클릭만으로 전 패널 검증이 불가능
/// 하다. 훅은 ☰ 메뉴와 동일한 단일 창 정책을 우회하지 않는다(플로팅
/// 패널은 하나만). 값: 패널명 토글·"name:1" 강제 켬·"name:0" 끔.
#[cfg(target_arch = "wasm32")]
fn debug_panel_cmds(mut ui_state: ResMut<UiState>) {
    let Some(win) = web_sys::window() else { return };
    let cmds: Vec<(String, bool)> = (|| {
        use wasm_bindgen::JsCast;
        let tf = js_sys::Reflect::get(&win, &"terrafolk".into()).ok()?;
        let ui = js_sys::Reflect::get(&tf, &"ui".into()).ok()?;
        let arr = js_sys::Reflect::get(&ui, &"panelCmds".into()).ok()?;
        let arr = arr.dyn_into::<js_sys::Array>().ok()?;
        let mut out = Vec::new();
        while arr.length() > 0 {
            let v = arr.shift();
            let s = v.as_string().unwrap_or_default();
            if s.is_empty() {
                continue;
            }
            let (name, val) = match s.split_once(':') {
                Some((n, v)) => (n.to_owned(), v != "0"),
                None => (s.clone(), !panel_flag(&ui_state, &s)),
            };
            out.push((name, val));
        }
        Some(out)
    })()
    .unwrap_or_default();
    for (name, val) in cmds {
        if apply_panel_cmd(&mut ui_state, &name, val) {
            ui_state.user_adjusted = true;
            info!("[ui] E2E 패널 훅 — {name} = {val}");
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn debug_panel_cmds(_ui_state: ResMut<UiState>) {}

/// 패널명 → 현재 표시 여부(토글 기준값)
#[cfg(target_arch = "wasm32")]
fn panel_flag(s: &UiState, name: &str) -> bool {
    match name {
        "villagers" => s.show_villagers,
        "chat" => s.show_chat,
        "events" => s.show_events,
        "minimap" => s.show_minimap,
        "civ" => s.show_civ,
        "relations" => s.show_relations,
        "economy" => s.show_economy,
        "resources" => s.show_resources,
        "settings" => s.show_settings,
        _ => false,
    }
}

/// 패널명 적용 — 알 수 없는 이름이면 false(무시)
#[cfg(target_arch = "wasm32")]
fn apply_panel_cmd(s: &mut UiState, name: &str, val: bool) -> bool {
    match name {
        "villagers" => s.show_villagers = val,
        "chat" => s.show_chat = val,
        "events" => s.show_events = val,
        "minimap" => s.show_minimap = val,
        "civ" => s.show_civ = val,
        "relations" => s.show_relations = val,
        "economy" => s.show_economy = val,
        "resources" => s.show_resources = val,
        "settings" => s.show_settings = val,
        _ => return false,
    }
    true
}
