//! 유물 카드(§17.38) — 맵의 발명품(relic)을 탭/클릭하면 뜨는 설명 카드.
//! 레거시 FE select-relic → res 팝업 상단 카드의 직역: 이름·개선형·만든 이·
//! 목적(purpose — 발명가가 쓴 설명 그대로). agents 픽킹이 에이전트 미스 시
//! 근접 relic을 찾아 이 리소스를 채운다.

use bevy::prelude::*;
use bevy_egui::egui;

use super::i18n::{Tr, tr};
use super::theme;
use super::widgets::card;
use super::{UiState, ctx};

/// 열린 유물 카드 — None이면 닫힘. agents::pick_relic이 기록.
#[derive(Resource, Default)]
pub struct RelicCard(pub Option<RelicInfo>);

#[derive(Clone, Debug)]
pub struct RelicInfo {
    /// 발명품 이름(relic 자원의 species)
    pub name: String,
    pub by: String,
    pub purpose: String,
    pub based_on: String,
}

/// 유물 카드 — 하단 중앙 앵커(대화창 카드와 같은 자리 정책). ✕·빈 월드
/// 탭으로 닫는다. 데스크톱·모바일 동일.
pub fn show(
    mut contexts: bevy_egui::EguiContexts,
    ui_state: Res<UiState>,
    mut mt: ResMut<super::mt::MtCache>,
    mut open: ResMut<RelicCard>,
) {
    let Some(info) = open.0.clone() else {
        return;
    };
    let Some(c) = ctx(&mut contexts) else { return };
    let lang = ui_state.lang;
    let narrow = c.screen_rect().width() < 700.0;
    let w = if narrow {
        c.screen_rect().width() - 20.0
    } else {
        (c.screen_rect().width() * 0.4).clamp(280.0, 380.0)
    };
    // 채팅 독 위에 뜬다(대화창 카드와 같은 자리 정책) — 독이 켜 있으면
    // 그 높이만큼 위로. 상수는 ui/mod.rs 단일 진실원.
    let dock = if ui_state.show_chat {
        if narrow {
            super::CHAT_H_NARROW
        } else {
            super::CHAT_H_DESKTOP
        }
    } else {
        0.0
    };
    egui::Window::new("relic_card")
        .anchor(egui::Align2::CENTER_BOTTOM, egui::vec2(0.0, -(8.0 + dock)))
        .fixed_size([w, 0.0])
        .frame(card())
        .title_bar(false)
        .show(&c, |ui| {
            ui.set_min_width(w - 12.0);
            ui.horizontal(|ui| {
                ui.label(
                    egui::RichText::new(format!("⚒ {}", info.name))
                        .size(14.0)
                        .strong()
                        .color(theme::TEXT_WARM),
                );
                if !info.based_on.is_empty() {
                    super::widgets::chip(
                        ui,
                        &tr(lang, Tr::CivImproved).replacen("{}", &info.based_on, 1),
                        theme::WARN,
                        theme::glass(),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .button(egui::RichText::new("✕").size(11.0).color(theme::TEXT_DIM))
                        .clicked()
                    {
                        open.0 = None;
                    }
                });
            });
            if !info.by.is_empty() {
                ui.label(
                    egui::RichText::new(format!("{} · {}", tr(lang, Tr::RelicBy), info.by))
                        .size(10.0)
                        .color(theme::TEXT_DIM),
                );
            }
            if !info.purpose.is_empty() {
                ui.add_space(2.0);
                // 발명가가 쓴 영문 설명 — 표시 시점 MT(§17.51)
                ui.label(
                    egui::RichText::new(mt.get(&info.purpose, lang))
                        .size(11.0)
                        .color(theme::TEXT_WARM),
                );
            } else {
                ui.label(
                    egui::RichText::new(tr(lang, Tr::CivEmpty))
                        .size(10.0)
                        .color(theme::TEXT_DIM),
                );
            }
        });
}
