//! 카툰 잉크 아웃라인 — 캐릭터 크레이트 `ink`(InkPlugin) 어댑터.
//!
//! 2026-11-03: 게임 자체 인버티드 헐 구현을 크레이트로 이관·교체했다. 구판은
//! 라이브에서 선이 **0개**였다 — ① 메시 공간 절대 문턱(0.25)이 qchar 리그
//! 단위(메시 0.0008~0.017, ×100 아마추어)에 전부 탈락, ② 셰이더가 bevy
//! 0.19에 없는 `view.view_proj`를 참조. 크레이트판은 액터 로컬 중간 치수
//! 문턱·평활 법선 헐(로우폴리 분할 법선 균열 방지)·부위색 선을 쓴다.
//!
//! 게이트 `?tfdbg=ink{px}` — 800px 기준 두께 A/B(예: ink3·ink6, ink0=끔).
//! 주민 마커는 villager_spawn이 `Ink`를 부착, 선색은 toon.rs가 `InkColor`.

use bevy::prelude::*;

use crate::character::{InkPlugin, InkSettings};

/// 기본 두께(px@800) — 2026-11-03 사용자 '선이 너무 두껍다'로 4→2.
/// 선색은 크레이트 기본(단일 웜 다크브라운 — 부위색 '펜 효과' 반려).
const INK_PX_DEFAULT: f32 = 2.0;

pub fn plugin(app: &mut App) {
    let mut s = InkSettings {
        px: INK_PX_DEFAULT,
        ..Default::default()
    };
    match crate::net::dbg_gate::value("ink").and_then(|v| v.parse::<f32>().ok()) {
        Some(px) if px > 0.0 => s.px = px,
        Some(_) => s.enabled = false,
        None => {}
    }
    app.insert_resource(s);
    app.add_plugins(InkPlugin);
}
