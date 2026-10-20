//! 문명 패널 — 레거시 FE CivPopup 직역: 시대 요약(civScore=N+2G·ERA_STEPS)·
//! 6대분류 진척(도트 8칸)·만든 이들·살아있는 전통·마을 연대기·발명 목록/상세.
//! inventionsJson은 원문 변화 시만 재파싱(캐시).

use bevy::prelude::*;
use bevy_egui::egui;

use super::i18n::{Lang, Tr, tr, trf};
use super::theme;
use super::vocab::{Vocab, civ_score};
use super::widgets::{chip, section};
use super::{EventLog, UiState, ctx};
use crate::net::NetState;

/// inventionsJson 행 — docs §2.4(서버 serializeInventions)
#[derive(serde::Deserialize, Clone)]
pub(crate) struct InventionRow {
    name: String,
    #[serde(default)]
    purpose: Option<String>,
    #[serde(default)]
    inventor: Option<String>,
    #[serde(default)]
    based_on: Option<String>,
    #[serde(default)]
    category: Option<String>,
    #[serde(default)]
    day: Option<u32>,
    #[serde(default)]
    materials: Option<std::collections::BTreeMap<String, u32>>,
    #[serde(default)]
    performs: Option<u32>,
    #[serde(default, rename = "last_perform_day")]
    last_perform_day: Option<u32>,
}

const GROUPS: [&str; 6] = ["living", "production", "infrastructure", "culture", "knowledge", "institutions"];

/// 발명 상세 — 열린 발명 이름(None이면 목록 뷰). Local로 패널 수명 유지.
#[derive(Default)]
pub(crate) struct CivView {
    detail: Option<String>,
}

pub fn panel(
    mut contexts: bevy_egui::EguiContexts,
    ui_state: Res<UiState>,
    net: Option<Res<NetState>>,
    vocab: Res<Vocab>,
    events: Res<EventLog>,
    mut cache: Local<Option<(String, Vec<InventionRow>)>>,
    mut view: Local<CivView>,
) {
    if !ui_state.show_civ {
        *view = CivView::default();
        return;
    }
    let Some(c) = ctx(&mut contexts) else { return };
    let lang = ui_state.lang;
    let raw = net
        .as_ref()
        .and_then(|n| n.frame.as_ref())
        .and_then(|f| f.inventions.as_deref())
        .map(str::to_owned);
    let changed = cache.as_ref().map(|(s, _)| s != raw.as_deref().unwrap_or(""));
    if changed != Some(false) {
        *cache = raw.map(|s| {
            let rows = serde_json::from_str::<Vec<InventionRow>>(&s).unwrap_or_default();
            (s, rows)
        });
        // 원문이 바뀌면 상세 뷰 무효화(해당 발명이 사라졌을 수 있다)
        *view = CivView::default();
    }
    let rows: &[InventionRow] = cache.as_ref().map(|(_, r)| r.as_slice()).unwrap_or(&[]);

    egui::Window::new(tr(lang, Tr::Civ))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(-20.0, 0.0))
        .default_width(340.0)
        .default_open(true)
        .frame(crate::ui::widgets::card())
        .title_bar(false)
        .show(&c, |ui| {
            if let Some(name) = view.detail.clone()
                && let Some(r) = rows.iter().find(|r| r.name == name)
            {
                inv_detail(ui, lang, &vocab, r, &mut |v| *view = CivView { detail: v });
                return;
            }
            summary(ui, lang, &vocab, rows);
            groups_grid(ui, lang, &vocab, rows);
            makers(ui, lang, rows);
            traditions(ui, lang, rows);
            chronicle(ui, lang, &vocab, &events);
            inv_list(ui, lang, &vocab, rows, &mut |v| *view = CivView { detail: v });
        });
}

/// 요약 카드 — 시대 라벨 + 발명 수·문명 점수
fn summary(ui: &mut egui::Ui, lang: Lang, vocab: &Vocab, rows: &[InventionRow]) {
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(format!("文 {}", tr(lang, Tr::Civ)))
                .size(13.5)
                .strong()
                .color(theme::TEXT_WARM),
        );
        let groups = rows.iter().map(|r| vocab.inv_group_of(r.category.as_deref().unwrap_or("")).to_owned());
        let (score, era) = civ_score(groups);
        chip(
            ui,
            vocab.era(lang, era),
            theme::ACCENT_SAND,
            egui::Color32::from_rgb(54, 46, 22),
        );
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(trf(lang, Tr::CivCount, &rows.len().to_string()))
                    .size(10.5)
                    .color(theme::TEXT_DIM),
            );
            ui.label(
                egui::RichText::new(trf(lang, Tr::CivScore, &score.to_string()))
                    .size(10.5)
                    .color(theme::TEXT_DIM),
            );
        });
    });
    ui.add_space(2.0);
}

/// 6대분류 진척 — 그룹명 + 수 + 도트 8칸(레거시 GroupDots)
fn groups_grid(ui: &mut egui::Ui, lang: Lang, vocab: &Vocab, rows: &[InventionRow]) {
    section(ui, "階", &tr(lang, Tr::Civ));
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = 10.0;
        for g in GROUPS {
            let n = rows
                .iter()
                .filter(|r| vocab.inv_group_of(r.category.as_deref().unwrap_or("")) == g)
                .count();
            let col = ui.vertical(|ui| {
                ui.label(
                    egui::RichText::new(vocab.inv_group(lang, g))
                        .size(10.0)
                        .color(if n > 0 { theme::ACCENT_SAND } else { theme::TEXT_DIM }),
                );
                let (rect, _) = ui.allocate_exact_size(egui::vec2(44.0, 8.0), egui::Sense::hover());
                let p = ui.painter();
                let filled = (n as f32 / 4.0).ceil().min(8.0) as usize;
                for i in 0..8 {
                    let x = rect.left() + 2.0 + i as f32 * 5.5;
                    p.circle_filled(
                        egui::pos2(x, rect.center().y),
                        1.8,
                        if i < filled { theme::ACCENT_SAND } else { egui::Color32::from_black_alpha(120) },
                    );
                }
                ui.label(egui::RichText::new(n.to_string()).size(9.0).color(theme::TEXT_DIM));
            })
            .response;
            let _ = col;
        }
    });
}

/// 만든 이들 — 발명가별 수 상위 4(빈 이름·조상 제외)
fn makers(ui: &mut egui::Ui, lang: Lang, rows: &[InventionRow]) {
    let mut by: std::collections::BTreeMap<&str, u32> = Default::default();
    for r in rows {
        if let Some(i) = r.inventor.as_deref().filter(|s| !s.is_empty() && *s != "the ancestors") {
            *by.entry(i).or_insert(0) += 1;
        }
    }
    if by.is_empty() {
        return;
    }
    section(ui, "匠", tr(lang, Tr::CivMakers));
    let mut top: Vec<(&str, u32)> = by.into_iter().collect();
    top.sort_by(|a, b| b.1.cmp(&a.1));
    ui.horizontal_wrapped(|ui| {
        for (name, n) in top.iter().take(4) {
            chip(
                ui,
                &format!("{name} · {n}"),
                theme::TEXT_WARM,
                egui::Color32::from_black_alpha(90),
            );
        }
    });
}

/// 살아있는 전통 — 문화 계열 & performs>0. 깊이=min(3, performs/4)
fn traditions(ui: &mut egui::Ui, lang: Lang, rows: &[InventionRow]) {
    let t: Vec<&InventionRow> = rows
        .iter()
        .filter(|r| r.performs.unwrap_or(0) > 0 && r.based_on.is_some())
        .collect();
    let t: Vec<&InventionRow> = t
        .into_iter()
        .filter(|r| r.category.as_deref() == Some("traditions") || r.category.as_deref() == Some("arts"))
        .collect();
    if t.is_empty() {
        return;
    }
    section(ui, "戲", tr(lang, Tr::CivTraditions));
    for r in t.iter().take(4) {
        let depth = (r.performs.unwrap_or(0) / 4).min(3);
        let line = tr(lang, Tr::CivTradDepth)
            .replacen("{}", &r.performs.unwrap_or(0).to_string(), 1)
            .replacen("{}", &depth.to_string(), 1);
        let resp = ui.label(
            egui::RichText::new(format!("{} — {line}", r.name))
                .size(10.5)
                .color(theme::TEXT_WARM),
        );
        if let Some(d) = r.last_perform_day {
            resp.on_hover_text(trf(lang, Tr::CivLastPerform, &d.to_string()));
        }
    }
}

/// 마을 연대기 — chronicle 종 이벤트 최근 12(문장은 vocab 렌더)
fn chronicle(ui: &mut egui::Ui, lang: Lang, vocab: &Vocab, events: &EventLog) {
    let rows: Vec<&super::EventRow> = events
        .rows
        .iter()
        .filter(|e| e.kind == "chronicle")
        .collect();
    if rows.is_empty() {
        return;
    }
    section(ui, "史", tr(lang, Tr::CivChronicle));
    for e in rows.iter().rev().take(12) {
        ui.label(
            egui::RichText::new(vocab.event_text(lang, &e.payload))
                .size(10.0)
                .color(egui::Color32::from_rgba_unmultiplied(233, 203, 125, 230)),
        );
    }
}

/// 발명 목록 — 그룹별 섹션·최신 우선. 행 클릭 → 상세 뷰
fn inv_list(
    ui: &mut egui::Ui,
    lang: Lang,
    vocab: &Vocab,
    rows: &[InventionRow],
    open: &mut dyn FnMut(Option<String>),
) {
    if rows.is_empty() {
        ui.label(
            egui::RichText::new(tr(lang, Tr::CivEmpty))
                .size(10.5)
                .color(theme::TEXT_DIM),
        );
        return;
    }
    egui::ScrollArea::vertical()
        .id_salt("civlist")
        .max_height(300.0)
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for g in GROUPS {
                let mut in_group: Vec<&InventionRow> = rows
                    .iter()
                    .filter(|r| vocab.inv_group_of(r.category.as_deref().unwrap_or("")) == g)
                    .collect();
                if in_group.is_empty() {
                    continue;
                }
                in_group.sort_by(|a, b| b.day.unwrap_or(0).cmp(&a.day.unwrap_or(0)));
                section(ui, "·", vocab.inv_group(lang, g));
                for r in in_group {
                    let resp = egui::Frame::new()
                        .fill(egui::Color32::from_black_alpha(60))
                        .corner_radius(7.0)
                        .inner_margin(egui::Margin::symmetric(7, 5))
                        .show(ui, |ui| {
                            ui.set_width(ui.available_width());
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new(&r.name)
                                        .size(11.5)
                                        .strong()
                                        .color(theme::TEXT_WARM),
                                );
                                if let Some(d) = r.day {
                                    ui.with_layout(
                                        egui::Layout::right_to_left(egui::Align::Center),
                                        |ui| {
                                            chip(
                                                ui,
                                                &trf(lang, Tr::CivDayN, &d.to_string()),
                                                theme::TEXT_DIM,
                                                egui::Color32::from_black_alpha(90),
                                            );
                                        },
                                    );
                                }
                            });
                            let mut meta = Vec::new();
                            if let Some(inv) = r.inventor.as_deref().filter(|s| !s.is_empty()) {
                                meta.push(format!("{} · {inv}", tr(lang, Tr::CivInventor)));
                            }
                            if let Some(cat) = r.category.as_deref() {
                                meta.push(vocab.invcat(lang, cat).to_owned());
                            }
                            if !meta.is_empty() {
                                ui.label(
                                    egui::RichText::new(meta.join(" · "))
                                        .size(9.5)
                                        .color(theme::TEXT_DIM),
                                );
                            }
                            if let Some(p) = r.purpose.as_deref().filter(|s| !s.is_empty()) {
                                ui.label(
                                    egui::RichText::new(p)
                                        .size(9.5)
                                        .color(theme::TEXT_DIM)
                                        .weak(),
                                );
                            }
                        })
                        .response
                        .interact(egui::Sense::click());
                    if resp.clicked() {
                        open(Some(r.name.clone()));
                    }
                    resp.on_hover_cursor(egui::CursorIcon::PointingHand);
                }
            }
        });
}

/// 발명 상세 — 큰 제목·개선형·날짜·재료·purpose
fn inv_detail(
    ui: &mut egui::Ui,
    lang: Lang,
    vocab: &Vocab,
    r: &InventionRow,
    open: &mut dyn FnMut(Option<String>),
) {
    if ui
        .button(egui::RichText::new("← 목록").size(10.5).color(theme::ACCENT_SAND))
        .clicked()
    {
        open(None);
        return;
    }
    ui.add_space(2.0);
    ui.label(
        egui::RichText::new(format!("⚒ {}", r.name))
            .size(15.0)
            .strong()
            .color(theme::TEXT_WARM),
    );
    if let Some(base) = r.based_on.as_deref().filter(|s| !s.is_empty()) {
        let label = tr(lang, Tr::CivImproved).replacen("{}", base, 1);
        chip(ui, &label, theme::WARN, egui::Color32::from_rgb(52, 42, 16));
    }
    if let Some(inv) = r.inventor.as_deref().filter(|s| !s.is_empty()) {
        ui.label(
            egui::RichText::new(format!("{} · {inv}", tr(lang, Tr::CivInventor)))
                .size(10.5)
                .color(theme::TEXT_DIM),
        );
    }
    let mut badges = Vec::new();
    if let Some(d) = r.day {
        let (y, m, dd) = theme::date_of(d);
        badges.push(tr(lang, Tr::CivDate)
            .replacen("{}", &y.to_string(), 1)
            .replacen("{}", &m.to_string(), 1)
            .replacen("{}", &dd.to_string(), 1));
    }
    if let Some(cat) = r.category.as_deref() {
        badges.push(vocab.invcat(lang, cat).to_owned());
    }
    if !badges.is_empty() {
        ui.label(egui::RichText::new(badges.join(" · ")).size(9.5).color(theme::TEXT_DIM));
    }
    if let Some(mats) = r.materials.as_ref().filter(|m| !m.is_empty()) {
        let joined = mats
            .iter()
            .map(|(id, n)| format!("{} ×{}", vocab.item(lang, id), n))
            .collect::<Vec<_>>()
            .join(" · ");
        ui.label(
            egui::RichText::new(format!("{} · {joined}", tr(lang, Tr::CivMaterials)))
                .size(10.0)
                .color(theme::TEXT_DIM),
        );
    }
    if let Some(p) = r.purpose.as_deref().filter(|s| !s.is_empty()) {
        ui.add_space(3.0);
        ui.label(egui::RichText::new(p).size(11.0).color(theme::TEXT_WARM));
    }
}
