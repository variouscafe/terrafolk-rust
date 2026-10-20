//! 월드 활동 표현(§17.39 시각화) — 행동 중 주민 머리 위 진행 링+아이콘,
//! 수면 zzz, 욕구 위험 경고. 레거시 scene/agents.ts updateActivity 직역.
//!
//! egui 페인터 오버레이로 그린다(월드 스페이스 텍스처·별도 폰트 로딩 대신
//! 패널과 같은 Phosphor/텍스트 계층 재사용 — 2026-10-21). 아이콘은 이모지가
//! 아니라 이미 설치된 Phosphor 글리프(폰트 없는 이모지는 두부가 된다).

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};
use egui_phosphor::regular;

use super::ctx;
use crate::agents::Agent;
use crate::net::NetState;
use crate::net::apply::LiveAgentTag;

/// actType → Phosphor 글리프(레거시 ACT_ICON 대응 — 이모지 폰트 미탑재).
/// fetch·store·rest는 레거시 맵에 없지만 최빈 행동이라 표시한다(개선).
fn act_icon(act: &str) -> Option<&'static str> {
    Some(match act {
        "chop" => regular::AXE,
        "mine" => regular::SHOVEL,
        "forage" => regular::LEAF,
        "hunt" => regular::SWORD,
        "eat" => regular::FORK_KNIFE,
        "drink" => regular::DROP,
        "fetch" => regular::DROP_SIMPLE,
        "store" => regular::BASKET,
        "campfire" => regular::CAMPFIRE,
        "warm" => regular::THERMOMETER_HOT,
        "wear" => regular::SHIRT_FOLDED,
        "give" => regular::GIFT,
        "trade" => regular::HANDSHAKE,
        "gather" => regular::PLANT,
        "build" => regular::HOUSE,
        "invent" => regular::LIGHTBULB,
        "craft" => regular::HAMMER,
        "perform" => regular::MASK_HAPPY,
        _ => return None,
    })
}

/// 머리 위 오버레이 — 매 프레임 주민 투영 위치에 그린다. 패널(BACKGROUND
/// 위 Order::Middle) 아래 깔리므로 입력도 막지 않는다(interactable(false)).
pub fn overlay(
    mut contexts: EguiContexts,
    time: Res<Time>,
    net: Option<Res<NetState>>,
    camera_q: Query<(&Camera, &GlobalTransform), With<Camera2d>>,
    agents_q: Query<(&Transform, Option<&LiveAgentTag>), With<Agent>>,
) {
    let Some(frame) = net.as_ref().and_then(|n| n.frame.as_ref()) else {
        return;
    };
    let Some(c) = ctx(&mut contexts) else { return };
    let Ok((camera, cam_tf)) = camera_q.single() else { return };
    let screen = c.screen_rect();
    let p = c.layer_painter(egui::LayerId::background());
    let t = time.elapsed_secs();

    for (tr, tag) in &agents_q {
        let Some(la) = tag.and_then(|t| frame.agents.iter().find(|a| a.id == t.0)) else {
            continue;
        };
        // 몸통 중심 → 머리 위(스프라이트 ~150px급 — 발밑 y에서 위로)
        let head = tr.translation.truncate() + Vec2::new(0.0, 118.0);
        let Ok(sp) = camera.world_to_viewport(cam_tf, head.extend(tr.translation.z)) else {
            continue;
        };
        let sp = egui::pos2(sp.x, sp.y);
        if !screen.expand(40.0).contains(sp) {
            continue;
        }
        // ── 활동 링 — 진행률 두께(레거시 2+frac*3) + 아이콘 ──
        let acting = la
            .act_type
            .as_deref()
            .and_then(act_icon)
            .filter(|_| la.act_progress.unwrap_or(0.0) > 0.0);
        if let Some(icon) = acting {
            let frac = (la.act_progress.unwrap_or(0.0)).clamp(0.02, 1.0);
            p.circle(
                sp,
                9.0,
                egui::Color32::from_rgba_unmultiplied(20, 26, 20, 150),
                egui::Stroke::new(1.5 + frac * 2.5, egui::Color32::from_rgb(143, 224, 138)),
            );
            p.text(
                sp,
                egui::Align2::CENTER_CENTER,
                icon,
                egui::FontId::proportional(11.0),
                egui::Color32::WHITE,
            );
            continue; // 링이 있으면 zzz·경고와 겹치지 않게(레거시도 동시 표시 안 함)
        }
        // ── 수면 zzz — 1.6s 주기 위로 떠오르며 페이드(레거시 트윈 직역) ──
        if la.status == "sleeping" {
            let k = (t % 1.6) / 1.6;
            let alpha = ((1.0 - k) * 210.0) as u8;
            p.text(
                egui::pos2(sp.x + 12.0, sp.y - 8.0 - k * 18.0),
                egui::Align2::CENTER_CENTER,
                "z",
                egui::FontId::proportional(13.0),
                egui::Color32::from_rgba_unmultiplied(207, 227, 255, alpha),
            );
        }
        // ── 위험 경고 — 최대 욕구 ≥90(레거시 critical) 점프 바운스 ──
        let critical = [la.hunger, la.thirst, la.fatigue, la.warmth]
            .iter()
            .flatten()
            .any(|v| *v >= 90.0);
        if critical {
            let bob = (t * 9.0).sin() * 3.0;
            p.text(
                egui::pos2(sp.x - 16.0, sp.y - 2.0 + bob),
                egui::Align2::CENTER_CENTER,
                regular::WARNING,
                egui::FontId::proportional(13.0),
                egui::Color32::from_rgb(255, 176, 76),
            );
        }
    }
}
