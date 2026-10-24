//! 채팅 지시 → 행동 브리지(2026-10-24).
//!
//! 웹 채팅 송신 시 ui.js가 `tf.chatAction(agentId, text)`로 원문을 넘긴다
//! (모든 채팅이 방을 통해 브레인으로 가지만, **즉발 연출 지시**는 클라이언트
//! 물리로 수행한다). 본 모듈은 한국어 어간표로 파싱해 character 프레임워크의
//! `ActiveAction`을 대상 주민에 부착한다 — 행동 물리(중력·충돌)는 프레임워크
//! (character/action.rs)가 담당하고, **시나리오 추가는 parse_directive 표에
//! 한 줄 + ActionKind 구현**이면 끝난다.

use std::sync::{Arc, Mutex};

use bevy::prelude::*;

use crate::character::action::ActionKind;
use crate::character::tool::{ToolPart, ToolPrim};
use crate::net::apply::LiveAgentTag;

/// JS 브리지 큐 — (agentId, 채팅 원문).
#[derive(Resource, Default)]
pub struct ActionBridge {
    pub q: Arc<Mutex<Vec<(String, String)>>>,
}

impl ActionBridge {
    pub fn new() -> Self {
        Self::default()
    }
}

/// 지시 파싱표 — 어근 매칭(한국어 활용 포괄). 한도: 오탐 방지를 위해
/// 수식어(받아/멀리/버려)를 먼저 검사한다. 행동 추가 시 여기에 추가.
pub fn parse_directive(text: &str) -> Option<ActionKind> {
    // 어간 3형태: 던지(다)·던져(봐)·던졌(다) — 활용형 포괄.
    let throws = text.contains("던지") || text.contains("던져") || text.contains("던졌");
    if throws {
        if text.contains('받') {
            return Some(ActionKind::TossCatch); // 던졌다가 받아
        }
        if text.contains("멀리") || text.contains("버려") || text.contains("버리") {
            return Some(ActionKind::ThrowGround); // 멀리 던져
        }
        return Some(ActionKind::TossCatch); // 던져봐(기본=토스)
    }
    if text.contains("토스") {
        return Some(ActionKind::TossCatch);
    }
    if text.contains("점프") || text.contains("뛰어") {
        return Some(ActionKind::Jump);
    }
    None
}

/// 빈손일 때 던질 기본 물건 — 더덕(긴 뿌리, 샌드톤).
fn deodeok() -> ToolPart {
    ToolPart {
        prim: ToolPrim::Cyl {
            r: 0.05,
            h: 0.30,
        },
        offset: Vec3::ZERO,
        yaw_deg: 18.0,
        color: Color::srgb(0.78, 0.64, 0.38),
    }
}

/// 브리지 등록 — JS가 tf.chatAction(agentId, text) 호출(wasm 전용).
pub fn register_bridge(bridge: Res<ActionBridge>) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::closure::Closure;
        let q = std::sync::Arc::clone(&bridge.q);
        let chat_action = Closure::<dyn Fn(String, String)>::new(move |id: String, text: String| {
            if let Ok(mut q) = q.lock() {
                q.push((id, text));
            }
        });
        let window = web_sys::window().expect("window");
        let tf = js_sys::Reflect::get(&window, &"terrafolk".into()).expect("terrafolk bridge");
        let _ = js_sys::Reflect::set(&tf, &"chatAction".into(), chat_action.as_js_value());
        chat_action.forget(); // 앱 수명 유지
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = bridge;
}

/// 브리지 큐 반영 — 지시 파싱 → 대상 주민(LiveAgentTag)에 ActiveAction 부착.
/// 수행 중(ActiveAction 보유)이면 무시 — 중복 지시 겹침 방지.
pub fn drain_bridge(
    mut commands: Commands,
    bridge: ResMut<ActionBridge>,
    mut q: Query<
        (
            Entity,
            &LiveAgentTag,
            &crate::character::AnimState,
            &crate::character::tool::HeldTool,
            Option<&crate::character::action::ActiveAction>,
            Option<&crate::character::Fall>,
        ),
        With<crate::character::GroundActor>,
    >,
) {
    let drained: Vec<(String, String)> = {
        let Ok(mut q) = bridge.q.lock() else {
            return;
        };
        q.drain(..).collect()
    };
    if drained.is_empty() {
        return;
    }
    for (agent_id, text) in drained {
        let Some(kind) = parse_directive(&text) else {
            continue;
        };
        info!("[action] 지시 수신 {agent_id}: {text} → {kind:?}");
        for (e, tag, anim, tool, active, fall) in &mut q {
            if tag.0 != agent_id || active.is_some() {
                continue;
            }
            // 점프는 Fall(물리낙하 참여)이 있는 액터만.
            if kind == ActionKind::Jump && fall.is_none() {
                continue;
            }
            commands.entity(e).insert(crate::character::action::ActiveAction::new(
                kind,
                anim.0,
                tool.parts.clone(),
                deodeok(),
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 어간 활용형 포괄 — 던져(봐)가 던지/던졌와 다른 어근임에 주의(실측 누락).
    #[test]
    fn parses_throw_conjugations() {
        assert_eq!(
            parse_directive("더덕을 던졌다가 받아봐"),
            Some(ActionKind::TossCatch)
        );
        assert_eq!(parse_directive("멀리 던져봐"), Some(ActionKind::ThrowGround));
        assert_eq!(parse_directive("던져서 버려"), Some(ActionKind::ThrowGround));
        assert_eq!(parse_directive("토스해봐"), Some(ActionKind::TossCatch));
        assert_eq!(parse_directive("점프해봐"), Some(ActionKind::Jump));
        assert_eq!(parse_directive("던져봐"), Some(ActionKind::TossCatch));
        assert_eq!(parse_directive("안녕! 오늘 뭐해?"), None);
    }
}
