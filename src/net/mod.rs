//! 라이브 월드 브리지(Phase 2a) — colyseus.js를 JS에서 재사용하고 상태를
//! 평면 JSON으로 받아 ECS에 반영한다(프로토콜·인증·재접속은 JS 담당).
//! 관전 모드: 지형(mapJson)·자원·주민 좌표를 실서버 따라감.
//!
//! 채널: JS sink(wasm-bindgen Closure) → Mutex 큐 → drain_frames 시스템 →
//! LiveFrame 리소스 → 렌더 적용 시스템들.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};

/// JS→Rust 프레임 큐(전역 — Closure·네트워크 스레드 수명이 시스템보다 길다)
#[derive(Resource, Clone, Default)]
pub struct FrameQueue {
    pub inner: Arc<Mutex<Vec<String>>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NetStatus {
    Idle,
    Connecting,
    Connected,
    Reconnecting,
}

/// 네이티브 접속 스레드가 알리는 링크 단계(원자값 — 시스템이 폴링).
pub const PHASE_CONNECTING: u8 = 1;
pub const PHASE_CONNECTED: u8 = 2;
pub const PHASE_RECONNECTING: u8 = 3;

#[derive(Resource, Clone, Default)]
pub struct NetLinkPhase(pub Arc<AtomicU8>);

#[derive(Resource)]
pub struct NetState {
    pub status: NetStatus,
    /// 최신 프레임(파싱 완료본)
    pub frame: Option<LiveFrame>,
}

impl Default for NetState {
    fn default() -> Self {
        NetState {
            status: NetStatus::Idle,
            frame: None,
        }
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LiveAgent {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub x: f32,
    pub y: f32,
    #[allow(dead_code)]
    pub facing: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub act_type: Option<String>,
    #[serde(rename = "actType", skip_serializing_if = "Option::is_none")]
    pub _act_type_r: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub look: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LiveResource {
    pub id: u32,
    pub kind: String,
    #[allow(dead_code)]
    pub species: String,
    pub x: u16,
    pub y: u16,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LiveFrame {
    #[allow(dead_code)]
    pub tick: u32,
    pub day: u16,
    pub hour: u16,
    pub minute: u16,
    #[allow(dead_code)]
    pub season: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub weather: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sunrise: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sunset: Option<u16>,
    /// Tiled JSON — 최초 1회(fullStateOnly)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub map: Option<String>,
    pub agents: Vec<LiveAgent>,
    pub resources: Vec<LiveResource>,
    /// 발명 레지스트리 원문(inventionsJson) — 문명 요약(시대·발명 수) 원천.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub inventions: Option<String>,
}

/// Tiled mapJson 최소 파싱 — layers[0].data(GID 배열)·width·height.
#[derive(Debug, Deserialize)]
pub struct TiledMap {
    pub width: usize,
    pub height: usize,
    pub layers: Vec<TiledLayer>,
}

#[derive(Debug, Deserialize)]
pub struct TiledLayer {
    pub data: Vec<u8>,
}

pub struct NetPlugin;

impl Plugin for NetPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<FrameQueue>()
            .init_resource::<NetState>()
            .init_resource::<NetLinkPhase>()
            .add_systems(Startup, (connect_bridge, apply::prime_live_world).chain())
            .add_systems(
                Update,
                (
                    update_link_phase,
                    drain_frames,
                    apply::prime_live_world,
                    apply::sync_daynight,
                    resources::sync_resources,
                    apply::sync_agents,
                    apply::live_move,
                    resources::live_res_move,
                    apply::debug_goto_animal,
                )
                    .chain(),
            );
    }
}

/// 브리지 기동 — JS에 sink 등록 + 접속 요청(wasm 전용, 네이티브는 데모 유지).
#[cfg(target_arch = "wasm32")]
fn connect_bridge(queue: Res<FrameQueue>) {    use wasm_bindgen::closure::Closure;
    use wasm_bindgen::prelude::*;
    let q = std::sync::Arc::clone(&queue.inner);
    let sink = Closure::<dyn Fn(String)>::new(move |json: String| {
        if let Ok(mut q) = q.lock() {
            q.push(json);
            // 큐 폭주 방지 — 최근 5프레임만
            let len = q.len();
            if len > 5 {
                q.drain(0..len - 5);
            }
        }
    });
    let window = web_sys::window().expect("window");
    let tf = js_sys::Reflect::get(&window, &"terrafolk".into()).expect("terrafolk bridge");
    let _ = js_sys::Reflect::set(&tf, &"dummy".into(), &JsValue::NULL);
    // setSink(fn)
    let set = js_sys::Reflect::get(&tf, &"setSink".into()).expect("setSink");
    let set: js_sys::Function = set.dyn_into().expect("setSink fn");
    let sink_js = sink.as_js_value().clone();
    let _ = set.call1(&tf, &sink_js);
    sink.forget(); // 앱 수명 유지

    let conn = js_sys::Reflect::get(&tf, &"connect".into()).expect("connect");
    let conn: js_sys::Function = conn.dyn_into().expect("connect fn");
    let _ = conn.call0(&tf); // 베이스 URL은 JS가 오리진으로 선택
}

/// 네이티브 기동 — colyseus-lite(ws-link) 접속 스레드(spawn 후 링크 단계·
/// 프레임은 자원으로 흘러온다). 실패·끊김은 스레드 안에서 백오프 재접속.
#[cfg(not(target_arch = "wasm32"))]
fn connect_bridge(queue: Res<FrameQueue>, phase: Res<NetLinkPhase>) {
    native_conn::spawn(queue.clone(), Arc::clone(&phase.0));
}

/// 링크 단계(원자값) → NetState.status. drain_frames가 성공 시 CONNECTED를
/// 원자값에도 기록해 두 방향이 한 값을 바라본다.
fn update_link_phase(phase: Res<NetLinkPhase>, mut net: ResMut<NetState>) {
    let status = match phase.0.load(Ordering::Relaxed) {
        PHASE_CONNECTING => NetStatus::Connecting,
        PHASE_CONNECTED => NetStatus::Connected,
        PHASE_RECONNECTING => NetStatus::Reconnecting,
        _ => NetStatus::Idle,
    };
    if net.status != status {
        info!("[net] 링크 → {status:?}");
        net.status = status;
    }
}

/// 큐 → 최신 프레임 1개만 채택(중간 프레임 스킵 — 1s 주기라 충분).
pub fn drain_frames(
    queue: Res<FrameQueue>,
    phase: Res<NetLinkPhase>,
    mut net: ResMut<NetState>,
) {
    let drained: Vec<String> = {
        let Ok(mut q) = queue.inner.lock() else {
            return;
        };
        q.drain(..).collect()
    };
    let Some(last) = drained.into_iter().last() else {
        return;
    };
    match serde_json::from_str::<LiveFrame>(&last) {
        Ok(frame) => {
            if net.status != NetStatus::Connected {
                net.status = NetStatus::Connected;
                info!("[net] 라이브 프레임 수신 개시");
            }
            // 원자값도 CONNECTED로 — update_link_phase 폴링과 단일 상태 유지.
            phase.0.store(PHASE_CONNECTED, Ordering::Relaxed);
            net.frame = Some(frame);
        }
        Err(e) => warn!("[net] 프레임 파싱 실패: {e}"),
    }
}

/// 라이브 적용 로직(지형 프라임·시간·주민)
pub mod apply;
pub mod dbg_gate;
/// 네이티브 라이브 접속(colyseus-lite) — wasm은 JS sink 경로
#[cfg(not(target_arch = "wasm32"))]
pub mod native_conn;
/// 라이브 자원 동기(자원·발판·동물 배위) — apply에서 기능별 분리
pub mod resources;
pub mod visual_map;
