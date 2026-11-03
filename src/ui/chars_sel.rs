//! 주민 하단 선택 패널 — chars.rs에서 분리(500줄 규칙·기능별 분리, 2026-11-03).
//! 선택 카드(이름·행동·목표·욕구 바·추적 칩·외형 편집 진입·채팅 버튼)의
//! 컴포넌트·해제 옵저버·갱신 시스템을 소유한다. 팝업 목록·행 스폰은 chars.rs.

use bevy::prelude::*;

use super::chars::CharsPopupRoot;
use super::i18n;
use super::theme::{self, UiTheme};
use crate::net::agent_hud::AgentHud;
use crate::net::apply::LiveAgentTag;
use crate::world::cam_follow::CamFollow;
use crate::world::pick::Selection;
use crate::world::villager::Villager;

/// 하단 선택 패널 마커들
#[derive(Component)]
pub struct SelPanel;
#[derive(Component)]
pub struct SelName;
#[derive(Component)]
pub struct SelStatus;
#[derive(Component)]
pub struct SelFollowChip;
/// 목표 문장 한 줄(FE CharsPopup 목표 표시 패리티)
#[derive(Component)]
pub struct SelGoal;
/// 욕구 바 컨테이너(라이브 전용 — 데모는 숨기고 안내 문구)
#[derive(Component)]
pub struct SelNeeds;
/// 욕구 바 채움 노드(인덱스 = 욕구 4종 순서)
#[derive(Component)]
pub struct SelNeedFill(pub usize);
/// 욕구 바 트랙(배경)
#[derive(Component)]
pub struct SelNeedTrack;
/// 욕구 수치 텍스트
#[derive(Component)]
pub struct SelNeedVal(pub usize);
/// 데모(비라이브) 안내 문구
#[derive(Component)]
pub struct SelLiveHint;
/// 선택 해제 버튼
#[derive(Component)]
pub struct SelClearButton;

/// 선택 패널 말걸기 버튼(F1 채팅 입구).
#[derive(Component)]
pub struct SelChatButton;

/// 하단 패널 해제 칩 — 선택 해제. 행 선택과 동일하게 확정 클릭 이벤트.
pub(super) fn on_clear_click(_: On<Pointer<Click>>, mut sel: ResMut<Selection>) {
    sel.id = None;
    sel.entity = None;
}

/// 하단 선택 패널 갱신 — 이름·행동·목표·욕구 바(라이브)·추적 칩 표시.
#[allow(clippy::type_complexity)]
pub fn refresh_sel_panel(
    sel: Res<Selection>,
    follow: Res<CamFollow>,
    mine: Res<crate::net::look_sync::LookStore>,
    villagers: Query<(&Villager, Option<&AgentHud>)>,
    mut q_panel: Query<
        &mut Visibility,
        (
            With<SelPanel>,
            Without<CharsPopupRoot>,
            Without<SelFollowChip>,
            Without<SelNeeds>,
            Without<SelLiveHint>,
        ),
    >,
    mut q_name: Query<
        &mut Text,
        (
            With<SelName>,
            Without<SelStatus>,
            Without<SelGoal>,
            Without<SelNeedVal>,
        ),
    >,
    mut q_status: Query<
        &mut Text,
        (
            With<SelStatus>,
            Without<SelName>,
            Without<SelGoal>,
            Without<SelNeedVal>,
        ),
    >,
    mut q_goal: Query<
        &mut Text,
        (
            With<SelGoal>,
            Without<SelName>,
            Without<SelStatus>,
            Without<SelNeedVal>,
        ),
    >,
    mut q_chip: Query<
        &mut Visibility,
        (
            With<SelFollowChip>,
            Without<SelPanel>,
            Without<SelNeeds>,
            Without<SelLiveHint>,
        ),
    >,
    mut q_needs: Query<
        &mut Visibility,
        (
            With<SelNeeds>,
            Without<SelPanel>,
            Without<SelFollowChip>,
            Without<SelLiveHint>,
        ),
    >,
    mut q_hint: Query<
        &mut Visibility,
        (
            With<SelLiveHint>,
            Without<SelPanel>,
            Without<SelFollowChip>,
            Without<SelNeeds>,
        ),
    >,
    mut q_look: Query<
        &mut Visibility,
        (
            With<super::look::SelLookButton>,
            Without<SelPanel>,
            Without<SelFollowChip>,
            Without<SelNeeds>,
            Without<SelLiveHint>,
        ),
    >,
    mut q_fills: Query<(&SelNeedFill, &mut Node, &mut BackgroundColor), Without<SelNeedVal>>,
    mut q_vals: Query<(&SelNeedVal, &mut Text), Without<SelNeedFill>>,
) {
    let Some(entity) = sel.entity.filter(|e| villagers.get(*e).is_ok()) else {
        if let Ok(mut v) = q_panel.single_mut() {
            *v = Visibility::Hidden;
        }
        return;
    };
    let Ok((v, hud)) = villagers.get(entity) else {
        return; // 직전 is_some 확인 경합(같은 프레임 철거) — 벨트 재구성
    };
    if let Ok(mut p) = q_panel.single_mut() {
        *p = Visibility::Visible;
    }
    if let Ok(mut t) = q_name.single_mut()
        && **t != v.name
    {
        **t = v.name.clone();
    }
    if let Ok(mut t) = q_status.single_mut() {
        let label = i18n::act_label(
            &hud.map(|h| h.act_type.as_str())
                .filter(|a| !a.is_empty())
                .unwrap_or(&v.status),
        );
        if **t != label {
            **t = label;
        }
    }
    // 욕구 바·목표 — 라이브(AgentHud)만. 데모는 안내 문구(정직 표시).
    let live = hud.is_some();
    if let Ok(mut n) = q_needs.single_mut() {
        *n = if live {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if let Ok(mut h) = q_hint.single_mut() {
        *h = if live {
            Visibility::Hidden
        } else {
            Visibility::Visible
        };
    }
    if let Some(hud) = hud {
        for (mark, mut node, mut bg) in &mut q_fills {
            let value = hud.needs[mark.0].clamp(0.0, 100.0);
            node.width = Val::Percent(value);
            // 70 초과 빨강(FE red-400 근사)
            *bg = BackgroundColor(if value > 70.0 {
                Color::srgb(0.96, 0.42, 0.42)
            } else {
                theme::ACCENT
            });
        }
        for (mark, mut t) in &mut q_vals {
            let s = format!("{}", hud.needs[mark.0].round() as u32);
            if **t != s {
                **t = s;
            }
        }
        if let Ok(mut t) = q_goal.single_mut() {
            let goal = i18n::goal_text(&hud.goal_key, &hud.goal_text);
            if **t != goal {
                **t = goal;
            }
        }
    } else if let Ok(mut t) = q_goal.single_mut()
        && !t.is_empty()
    {
        **t = String::new();
    }
    if let Ok(mut c) = q_chip.single_mut() {
        *c = if follow.active {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    // 외형 편집 진입 — 선택 주민이 소유(/me)일 때만.
    if let Ok(mut b) = q_look.single_mut() {
        let mine_now = sel.id.as_deref().is_some_and(|id| mine.contains(id));
        *b = if mine_now {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
}
