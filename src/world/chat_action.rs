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

/// 수면 판정 — 서버 활동/상태 코드(act 우선·status 폴백)에 sleep 계열이면
/// 잠든 것으로 본다(애님·zZz 배지와 동일 어휘).
fn is_sleeping(v: &crate::world::villager::Villager) -> bool {
    let codes = [v.act.as_str(), v.status.as_str()];
    codes
        .iter()
        .any(|c| matches!(*c, "sleep" | "sleeping" | "sleepwalking"))
}

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
/// 수식어(받아/멀리/버려/같이)를 먼저 검사한다. 행동 추가 시 여기에 추가.
pub fn parse_directive(text: &str) -> Option<ActionKind> {
    // 캐치볼 — 같이·서로 놀기 계열은 1인 토스보다 먼저 판정.
    if text.contains("캐치볼")
        || ((text.contains("같이") || text.contains("서로") || text.contains("놀아"))
            && (text.contains("던지") || text.contains("던져") || text.contains("던졌")))
    {
        return Some(ActionKind::PlayCatch);
    }
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

/// 캐치볼 총 패스 수(왕복 포함 캐치 성공 횟수).
const PLAY_PASSES: u8 = 3;
/// 행동 이동 정지 만료(s) — 최장 행동(캐치볼 ~7s)+여유.
const LOCK_SECS: f32 = 9.0;

/// 브리지 큐 반영 — 지시 파싱 → 대상 주민(LiveAgentTag)에 ActiveAction 부착.
/// 수행 중(ActiveAction 보유)이면 무시 — 중복 지시 겹침 방지. 캐치볼은
/// 수행 중이 아닌 **최근접 다른 주민**을 상대로 결정하고 양쪽 이동을 잠근다
/// (ActionLock — 서버 배회와의 경합 방지, 시간 만료 자정리).
#[allow(clippy::type_complexity)]
pub fn drain_bridge(
    time: Res<Time>,
    mut commands: Commands,
    bridge: ResMut<ActionBridge>,
    q: Query<
        (
            Entity,
            &Transform,
            &LiveAgentTag,
            &crate::world::villager::Villager,
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
        // 수신 로그는 소유자 탐색보다 먼저 — 대상 미발견이 무음이 되면
        // 진단이 불가능하다(2026-10-24 라이브 무음 사건 교훈).
        info!("[action] 지시 수신 {agent_id}: {text} → {kind:?}");
        // 1) 대상 주민 — 수면·점프-Fall 결번은 명시 거부(관전자 침묵 혼란 방지).
        let Some((owner, anim, parts, sleeping, fallless)) = q
            .iter()
            .find(|(_, _, tag, _, _, _, active, _)| tag.0 == agent_id && active.is_none())
            .map(|(e, _, _, v, anim, tool, _, fall)| {
                (
                    e,
                    anim.0,
                    tool.parts.clone(),
                    is_sleeping(v),
                    fall.is_none(),
                )
            })
        else {
            warn!("[action] 대상 없음/수행 중 — {agent_id} 스킵");
            continue;
        };
        if sleeping {
            warn!("[action] {agent_id} 수면 중 — 지시 거부(자는 주민은 움직이지 않는다)");
            continue;
        }
        if kind == ActionKind::Jump && fallless {
            warn!("[action] {agent_id} Fall 없음 — 점프 거부");
            continue;
        }
        // 2) 캐치볼 — 최근젝 다른 깨어 있는 주민(수행 중 아님). 없으면 혼자 토스로 강등.
        let partner = if kind == ActionKind::PlayCatch {
            let origin = q.get(owner).ok().map(|(_, tr, ..)| tr.translation);
            let pick = origin.and_then(|op| {
                q.iter()
                    .filter(|(e, _, tag, v, _, _, active, _)| {
                        *e != owner
                            && tag.0 != agent_id
                            && active.is_none()
                            && !is_sleeping(v)
                    })
                    .min_by_key(|(_, tr, ..)| tr.translation.distance_squared(op).to_bits())
                    .map(|(e, ..)| e)
            });
            if pick.is_none() {
                info!("[action] 캐치볼 상대 없음 — 토스로 대체");
            }
            pick
        } else {
            None
        };
        let lock = crate::character::ActionLock {
            expire_at: time.elapsed_secs() + LOCK_SECS,
        };
        if let Some(p) = partner {
            // 상대는 지시 주인을 바라보며 받을 준비(요 회전 스무딩 —
            // face_targets가 만료까지 적용, 프레임워크 제공) + 받을 자세
            // 유지(PoseHold — ActiveAction은 지시 주인만 갖으므로 비행 중
            // 팔 내밀기가 활동 애님에 묻히지 않게 연출 우선 홀드).
            let until = time.elapsed_secs() + LOCK_SECS;
            commands.entity(p).insert((
                lock,
                crate::character::action::FaceTarget {
                    target: owner,
                    expire_at: until,
                },
                crate::character::PoseHold {
                    anim: crate::character::AnimKind::Punch,
                    until,
                },
            ));
        }
        commands.entity(owner).insert(lock);
        let act = match partner {
            Some(p) => crate::character::action::ActiveAction::new(
                kind,
                anim,
                parts,
                deodeok(),
            )
            .with_partner(p, PLAY_PASSES),
            None => crate::character::action::ActiveAction::new(
                if kind == ActionKind::PlayCatch {
                    ActionKind::TossCatch
                } else {
                    kind
                },
                anim,
                parts,
                deodeok(),
            ),
        };
        commands.entity(owner).insert(act);
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
        assert_eq!(parse_directive("캐치볼 해봐"), Some(ActionKind::PlayCatch));
        assert_eq!(
            parse_directive("빛나래랑 같이 던져서 놀아"),
            Some(ActionKind::PlayCatch)
        );
        assert_eq!(parse_directive("안녕! 오늘 뭐해?"), None);
    }
}
