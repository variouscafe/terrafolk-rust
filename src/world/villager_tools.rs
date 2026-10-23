//! 주민 활동 도구 — act 코드 → 손에 든 절차 도구(character::tool).
//! P3 2단계(2026-10-24): 사냥=창, 건설·제작=망치, 벌목·채굴=괭이·도끼,
//! 어로=작살, 물 긷기·마시기=물통. 발명품 model_url 보유 시 그 GLB로
//! 대체하는 연동(§17.86)은 후속 — 지금은 프리미티브 부품표(에셋 의존 0).

use bevy::prelude::*;

use super::villager::Villager;
use crate::character::tool::{HeldTool, ToolPart, ToolPrim};

/// 목재 자루·금속 머리 팔레트 — 레퍼런스 핸드페인트 톤.
const WOOD: Color = Color::srgb(0.55, 0.42, 0.26);
const METAL: Color = Color::srgb(0.62, 0.64, 0.68);
const CLAY: Color = Color::srgb(0.78, 0.62, 0.45);

fn cyl(r: f32, h: f32, offset: Vec3, yaw: f32, color: Color) -> ToolPart {
    ToolPart {
        prim: ToolPrim::Cyl { r, h },
        offset,
        yaw_deg: yaw,
        color,
    }
}

fn cub(x: f32, y: f32, z: f32, offset: Vec3, yaw: f32, color: Color) -> ToolPart {
    ToolPart {
        prim: ToolPrim::Box { x, y, z },
        offset,
        yaw_deg: yaw,
        color,
    }
}

/// 창 — 자루 + 촉(사냥·어로).
fn spear() -> Vec<ToolPart> {
    vec![
        cyl(0.016, 0.95, Vec3::new(0.0, 0.0, 0.28), 0.0, WOOD),
        cub(0.035, 0.05, 0.16, Vec3::new(0.0, 0.0, 0.80), 0.0, METAL),
    ]
}

/// 망치 — 짧은 자루 + 머리(건설·제작·제련).
fn hammer() -> Vec<ToolPart> {
    vec![
        cyl(0.018, 0.42, Vec3::new(0.0, 0.0, 0.10), 0.0, WOOD),
        cub(0.09, 0.07, 0.13, Vec3::new(0.0, 0.05, 0.32), 0.0, METAL),
    ]
}

/// 괭이 — 자루 + 굽은 날(벌목·채굴 근사).
fn pick() -> Vec<ToolPart> {
    vec![
        cyl(0.017, 0.55, Vec3::new(0.0, 0.0, 0.14), 0.0, WOOD),
        cub(0.26, 0.04, 0.05, Vec3::new(0.0, 0.16, 0.40), 0.0, METAL),
    ]
}

/// 물통 — 통 몸통(취수·마시기).
fn bucket() -> Vec<ToolPart> {
    vec![cyl(0.07, 0.14, Vec3::new(0.0, -0.02, 0.16), 0.0, CLAY)]
}

/// 활동 코드 → 도구 부품표. Walk·Idle·대화 등은 맨손(빈 벡터).
fn tool_for_act(act: &str) -> Vec<ToolPart> {
    match act {
        "hunt" => spear(),
        "fish" => spear(),
        "build" | "craft" | "smelt" => hammer(),
        "chop" | "mine" => pick(),
        "gather" | "forage" | "fetch_water" | "drink" | "eat" => bucket(),
        _ => Vec::new(),
    }
}

/// Villager의 act 변화를 HeldTool에 반영 — animate_states와 같은 프레임.
/// 도구 부착 자체는 character::tool::tool_sync가 담당(변경 프레임 1회).
pub fn apply_villager_tools(
    mut q: Query<(&Villager, &mut HeldTool), Changed<Villager>>,
) {
    for (v, mut tool) in &mut q {
        let want = tool_for_act(&v.act);
        // Villager는 이동으로도 매 프레임 변경될 수 있다 — 부품이 다를 때만
        // 써서(tool_sync 재구성) 매 프레임 재부착 플리커를 막는다.
        if want != tool.parts {
            tool.parts = want;
        }
    }
}
