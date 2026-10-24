//! 외형 편집 미리보기(2026-10-23 사용자 요청) — 편집기의 선택 변경을 서버
//! 왕복(PATCH look → 룸 프레임) 없이 즉시 3D 캐릭터에 반영한다.
//! 두 가지 브리지를 JS에 노출한다(pick.rs selectAgent 패턴):
//!   - tf.setLocalLook(agentId, lookJson) — 낙관적 look 오버라이드. 빈 문자열이면 해제.
//!     apply.rs가 에이전트 look을 정할 때 오버라이드가 서버 정본보다 우선하고,
//!     서버 프레임이 오버라이드값과 같아지면(저장 수렴) 스스로 제거된다.
//!   - tf.setLookFocus(bool) — 편집 카메라 포커스. 켜면 선택 주민 줌 인 고정
//!     (cam_follow가 추적, 팬 해제 억제), 끄면 진입 전 줌으로 복원.

use bevy::prelude::*;

use super::camera_rig::CameraRig;

/// 편집 줌 — 카메라 시야폭 = VIEW_H×zoom: 값이 작을수록 가깝다(ZOOM_MIN 0.3).
/// 치비 신장(1.38m)이 모바일 세로 화면에서도 크게 읽히는 근접 프레이밍.
pub const EDIT_ZOOM: f32 = 0.34;

/// JS→Rust 편집 브리지 큐 — (agentId, partsJson)과 포커스 토글.
#[derive(Resource, Default)]
pub struct LookEditBridge {
    look_q: std::sync::Arc<std::sync::Mutex<Vec<(String, String)>>>,
    focus_q: std::sync::Arc<std::sync::Mutex<Vec<bool>>>,
}

impl LookEditBridge {
    /// 낙관적 look 오버라이드 적재 — wasm JS 브리지와 네이티브 편집기
    /// (ui/look.rs) 양쪽이 같은 경로로 넣는다(drain_bridge가 단일 소비).
    pub fn push_look(&self, agent_id: &str, parts_json: &str) {
        if let Ok(mut q) = self.look_q.lock() {
            q.push((agent_id.to_owned(), parts_json.to_owned()));
        }
    }
    /// 편집 카메라 포커스 토글 적재.
    pub fn push_focus(&self, on: bool) {
        if let Ok(mut q) = self.focus_q.lock() {
            q.push(on);
        }
    }
}

/// 낙관적 look 오버라이드 — agent_id → parts JSON(문서 전체가 아니라
/// parts만: 서버 정본 look의 나머지 필드(uuid·sex·age·art)는 보존).
#[derive(Resource, Default)]
pub struct LocalLooks(pub std::collections::HashMap<String, String>);

/// 서버 정본 look 문서의 parts만 오버라이드로 교체 — 성별·아트 세대 등
/// 정본 필드를 그대로 물려받아 모델 선택(model_path_for)이 흔들리지 않게.
pub fn merged_look(base: &str, parts_json: &str) -> Option<String> {
    let mut doc = serde_json::from_str::<serde_json::Value>(base).ok()?;
    let parts = serde_json::from_str::<serde_json::Value>(parts_json).ok()?;
    doc.as_object_mut()?["parts"] = parts;
    Some(doc.to_string())
}

/// 서버 정본 look의 parts가 오버라이드와 의미상 같은지(저장 수렴 판정).
pub fn parts_equal(base: &str, parts_json: &str) -> bool {
    let parse = |s: &str| {
        serde_json::from_str::<serde_json::Value>(s)
            .ok()
            .and_then(|v| v.get("parts").cloned())
    };
    match (parse(base), parse(parts_json)) {
        (Some(a), Some(b)) => a == b,
        _ => false,
    }
}

/// 편집 카메라 포커스 — 활성 중 줌 고정·팬 해제 억제(cam_follow가 소비).
#[derive(Resource, Default)]
pub struct LookFocus {
    pub active: bool,
    /// 진입 전 줌(해제 시 복원)
    pub prev_zoom: Option<f32>,
}

/// 브리지 등록 — JS가 tf.setLocalLook(id, json|'')·tf.setLookFocus(bool) 호출(wasm).
pub fn register_bridge(bridge: Res<LookEditBridge>) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::closure::Closure;
        let lq = std::sync::Arc::clone(&bridge.look_q);
        let set_look = Closure::<dyn Fn(String, String)>::new(move |id: String, look: String| {
            if let Ok(mut q) = lq.lock() {
                q.push((id, look));
            }
        });
        let fq = std::sync::Arc::clone(&bridge.focus_q);
        let set_focus = Closure::<dyn Fn(bool)>::new(move |on: bool| {
            if let Ok(mut q) = fq.lock() {
                q.push(on);
            }
        });
        let window = web_sys::window().expect("window");
        let tf = js_sys::Reflect::get(&window, &"terrafolk".into()).expect("terrafolk bridge");
        let _ = js_sys::Reflect::set(&tf, &"setLocalLook".into(), set_look.as_js_value());
        let _ = js_sys::Reflect::set(&tf, &"setLookFocus".into(), set_focus.as_js_value());
        set_look.forget(); // 앱 수명 유지
        set_focus.forget();
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = bridge;
}

/// 브리지 큐 반영 — 오버라이드 삽입/해제, 포커스 토글(줌 저장·복원).
pub fn drain_bridge(
    bridge: ResMut<LookEditBridge>,
    mut looks: ResMut<LocalLooks>,
    mut focus: ResMut<LookFocus>,
    mut rig: ResMut<CameraRig>,
) {
    let look_drained: Vec<(String, String)> = {
        let Ok(mut q) = bridge.look_q.lock() else {
            return;
        };
        q.drain(..).collect()
    };
    for (id, look) in look_drained {
        if look.is_empty() {
            looks.0.remove(&id);
        } else {
            looks.0.insert(id, look);
        }
    }
    let focus_drained: Vec<bool> = {
        let Ok(mut q) = bridge.focus_q.lock() else {
            return;
        };
        q.drain(..).collect()
    };
    for on in focus_drained {
        if on && !focus.active {
            focus.prev_zoom = Some(rig.zoom_goal);
        } else if !on && focus.active {
            if let Some(z) = focus.prev_zoom.take() {
                rig.zoom_goal = z;
            }
        }
        focus.active = on;
    }
}
