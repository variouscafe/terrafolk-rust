//! 주민 패널 — 우측 목록(상태 점·활동 진행) + 선택 상세 카드.
//! 레거시 FE CharsPopup 직역: 욕구 바(>70 빨강)·소지품 칩+짐 무게 바·
//! 생활기록(유형색)·인연 상위 6·나이/성별. 클릭 선택 → 월드 마커·추적 연동.

use bevy::prelude::*;
use bevy_egui::egui;

use super::i18n::{Lang, Tr, tr, trf, act_icon, status_color, status_label, hist_color};
use super::theme;
use egui_phosphor::regular;
use super::vocab::Vocab;
use super::widgets::{card, need_bar, section, status_dot, thin_bar};
use super::{UiState, ctx};
use crate::agents::{Agent, Selected};
use crate::cam::ManualUntil;
use crate::net::apply::LiveAgentTag;
use crate::net::{NetState, PollQueuesRes};
use crate::net::LiveAgent;

/// lookJson 최소 파싱(나이·성별 표시)
#[derive(serde::Deserialize)]
struct LookBits {
    #[serde(default)]
    sex: Option<String>,
    #[serde(default)]
    age: Option<u32>,
}

/// invJson 파싱 {itemId: count}
fn inv_items(inv: Option<&str>) -> Vec<(String, u32)> {
    inv.and_then(|s| serde_json::from_str::<std::collections::BTreeMap<String, u32>>(s).ok())
        .map(|m| m.into_iter().filter(|(_, n)| *n > 0).collect())
        .unwrap_or_default()
}

/// 짐 무게(레거시 CarryBar — wt 미기재 폴백 1, 상한 5000, 85% 경고).
/// 착용품은 §17.81 짐 합산 제외(레거시 WORN_ITEMS 직역).
const CARRY_CAP: f32 = 5000.0;
const WORN_ITEMS: [&str; 5] = [
    "grass_skirt",
    "leather_wrap",
    "woven_leather",
    "stitched_leather",
    "leather_shoes",
];
fn carry_weight(v: &Vocab, items: &[(String, u32)]) -> f32 {
    items
        .iter()
        .filter(|(id, _)| !WORN_ITEMS.contains(&id.as_str()))
        .map(|(id, n)| v.item_wt(id) * *n as f32)
        .sum()
}

/// 목록 행 사본 — 패널(데스크톱)·모달 창(모바일) 양쪽 렌더가 같은 데이터로
/// 그린다. LiveAgent 복제는 주민 수(≈5)라 감수.
struct RowData {
    entity: Entity,
    name: String,
    tag_id: Option<String>,
    is_sel: bool,
    status: String,
    act: String,
    progress: Option<f32>,
    la: Option<LiveAgent>,
}

/// 본문 렌더 결과 — 선택 변경(행 클릭)·창 닫기(✕)를 패널 바깥 Commands로 적용
struct BodyOut {
    select: Option<Entity>,
    close: bool,
}

#[allow(clippy::type_complexity)] // Bevy 파라미터 — 선택·라이브 프레임·폴 동시 참조
pub fn panel(
    mut contexts: bevy_egui::EguiContexts,
    time: Res<Time>,
    net: Option<Res<NetState>>,
    vocab: Res<Vocab>,
    mut commands: Commands,
    agents_q: Query<(Entity, &Agent, Option<&LiveAgentTag>, Option<&Selected>)>,
    mut manual: ResMut<ManualUntil>,
    polls: Res<PollQueuesRes>,
    mut ui_state: ResMut<UiState>,
) {
    if !ui_state.show_villagers {
        return;
    }
    let Some(c) = ctx(&mut contexts) else { return };
    let lang = ui_state.lang;
    let frame = net.as_ref().and_then(|n| n.frame.as_ref());
    let live_of = |id: &str| -> Option<LiveAgent> {
        frame
            .and_then(|f| f.agents.iter().find(|la| la.id == id))
            .cloned()
    };
    // 행 수집(1회 순회 — 클론)
    let mut rows: Vec<RowData> = Vec::new();
    for (e, a, tag, sel) in &agents_q {
        let id = tag.map(|t| t.0.clone());
        let la = id.as_deref().and_then(live_of);
        let status = la
            .as_ref()
            .map(|l| l.status.clone())
            .unwrap_or_else(|| if a.rest > 0.0 { "idle".into() } else { "moving".into() });
        let act = la.as_ref().and_then(|l| l.act_type.clone()).unwrap_or_default();
        let progress = la.as_ref().and_then(|l| l.act_progress);
        rows.push(RowData {
            entity: e,
            name: a.name.clone(),
            tag_id: id,
            is_sel: sel.is_some(),
            status,
            act,
            progress,
            la,
        });
    }
    let mut out = BodyOut { select: None, close: false };
    let narrow = c.screen_rect().width() < 760.0;
    if narrow {
        // 모바일 — 레거시 CharsPopup처럼 중앙 모달(맵 최대 확보·단일 창 정책 대상)
        egui::Window::new(tr(lang, Tr::Villagers))
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, -10.0))
            .default_width((c.screen_rect().width() - 24.0).min(330.0))
            .frame(card())
            .title_bar(false)
            .show(&c, |ui| {
                ui.set_min_width((c.screen_rect().width() - 24.0).min(320.0).max(260.0));
                egui::ScrollArea::vertical()
                    .id_salt("vmodal")
                    .max_height(c.screen_rect().height() * 0.66)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        out = body(ui, lang, &vocab, frame, &polls, &rows, true);
                    });
            });
    } else {
        // 데스크톱 — 우측 도킹 패널(기존 레이아웃)
        egui::Panel::right("villagers")
            .exact_width(super::VILLAGERS_W)
            .frame(super::widgets::dock())
            .show(&c, |ui| {
                out = body(ui, lang, &vocab, frame, &polls, &rows, false);
            });
    }
    // 액션 적용 — 창 닫기·선택 토글(월드 마커·추적 재사용)
    if out.close {
        ui_state.show_villagers = false;
    }
    if let Some(e) = out.select {
        for (e2, _, _, _) in &agents_q {
            commands.entity(e2).remove::<Selected>();
        }
        let was_sel = rows.iter().find(|r| r.entity == e).map(|r| r.is_sel).unwrap_or(false);
        if !was_sel {
            commands.entity(e).insert(Selected);
            manual.0 = time.elapsed_secs(); // 추적 즉시 재개
        }
    }
}

/// 본문 — 헤더(+모바일 ✕)·목록·선택 상세. 공통(패널·모달 양쪽).
fn body(
    ui: &mut egui::Ui,
    lang: Lang,
    vocab: &Vocab,
    frame: Option<&crate::net::LiveFrame>,
    polls: &PollQueuesRes,
    rows: &[RowData],
    modal: bool,
) -> BodyOut {
    let mut out = BodyOut { select: None, close: false };
    ui.add_space(2.0);
    // 헤더 — 골드 아이콘 칩 + 타이틀(+모바일 ✕)
    ui.horizontal(|ui| {
        super::widgets::title_bar(
            ui,
            regular::USERS,
            &tr(lang, Tr::VillagersN).replace("{}", &rows.len().to_string()),
        );
        if modal {
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui
                    .button(egui::RichText::new("×").size(11.0).color(theme::TEXT_DIM))
                    .clicked()
                {
                    out.close = true;
                }
            });
        }
    });
    ui.add_space(2.0);
    let selected = rows.iter().find(|r| r.is_sel);
    // ── 목록 ──
    egui::ScrollArea::vertical()
        .id_salt("vlist")
        .max_height(if selected.is_some() { 150.0 } else { f32::INFINITY })
        .auto_shrink([false, false])
        .show(ui, |ui| {
            for r in rows {
                let raw_status = r.status.as_str();
                let name_color = r
                    .tag_id
                    .as_deref()
                    .map(theme::agent_ink)
                    .unwrap_or(theme::INK);
                let row = egui::Frame::new()
                    .fill(if r.is_sel {
                        theme::sel_fill()
                    } else {
                        egui::Color32::TRANSPARENT
                    })
                    .corner_radius(7.0)
                    .inner_margin(egui::Margin::symmetric(7, 4))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            status_dot(ui, status_color(raw_status));
                            ui.label(
                                egui::RichText::new(&r.name)
                                    .size(12.0)
                                    .strong()
                                    .color(if r.is_sel { theme::TEXT_WARM } else { name_color }),
                            );
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if !r.act.is_empty() {
                                        ui.label(
                                            egui::RichText::new(act_icon(&r.act))
                                                .size(11.0)
                                                .color(theme::ACCENT_SAND),
                                        );
                                    }
                                    let lbl = status_label(lang, raw_status);
                                    ui.label(
                                        egui::RichText::new(if lbl.is_empty() { raw_status } else { lbl })
                                            .size(10.0)
                                            .color(status_color(raw_status)),
                                    );
                                },
                            );
                        });
                        // 행동 진행 바(actProgress 0..1)
                        if let Some(p) = r.progress.filter(|p| *p > 0.01) {
                            thin_bar(ui, p, theme::GOLD, 3.0);
                        }
                    })
                    .response
                    .interact(egui::Sense::click());
                // 호버 피드백 — 살짝 밝은 오버레이 + 손 커서
                if !r.is_sel && row.hovered() {
                    ui.painter().rect_filled(row.rect, 7.0, egui::Color32::from_white_alpha(9));
                }
                let row = row.on_hover_cursor(egui::CursorIcon::PointingHand);
                if row.clicked() {
                    out.select = Some(r.entity);
                }
            }
        });
    // ── 선택 상세 ──
    let Some(sel) = selected else {
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(tr(lang, Tr::SelectHint))
                .size(10.5)
                .color(theme::TEXT_DIM),
        );
        if frame.is_none() {
            ui.label(
                egui::RichText::new(tr(lang, Tr::LiveDetailHint))
                    .size(10.0)
                    .color(theme::TEXT_DIM),
            );
        }
        return out;
    };
    ui.add_space(4.0);
    card().show(ui, |ui| {
        let Some(la) = &sel.la else {
            // 데모 선택 — 이름·안내만(정직 표시)
            ui.label(
                egui::RichText::new(&sel.name)
                    .size(14.0)
                    .strong()
                    .color(theme::TEXT_WARM),
            );
            ui.label(
                egui::RichText::new(tr(lang, Tr::LiveDetailHint))
                    .size(10.0)
                    .color(theme::TEXT_DIM),
            );
            return;
        };
        let id = sel.tag_id.clone().unwrap_or_default();
        detail(ui, lang, vocab, la, &id, polls, frame);
    });
    out
}

/// 선택 상세 카드 본문 — 라이브 데이터만 호출
fn detail(
    ui: &mut egui::Ui,
    lang: Lang,
    vocab: &Vocab,
    la: &LiveAgent,
    tag_id: &str,
    polls: &PollQueuesRes,
    frame: Option<&crate::net::LiveFrame>,
) {
    let color = theme::agent_color(tag_id);
    ui.horizontal(|ui| {
        // 아바타 칩 — 크림 링 + 주민색 + 이니셜(레퍼런스 프로필 직역)
        let initial = la.name.chars().next().map(String::from).unwrap_or_default();
        super::widgets::avatar_chip(ui, &initial, color, 32.0);
        ui.label(egui::RichText::new(&la.name).size(14.0).strong().color(theme::TEXT_WARM));
        // kind 배지 — 참여 캐릭터(user)/씨앗 주민(bot)
        if la.kind == "user" {
            super::widgets::chip(ui, tr(lang, Tr::Participating), theme::ACCENT_SAND, egui::Color32::from_rgb(52, 44, 20));
        }
        // 상태
        let lbl = status_label(lang, &la.status);
        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                egui::RichText::new(if lbl.is_empty() { la.status.as_str() } else { lbl })
                    .size(10.5)
                    .color(status_color(&la.status)),
            );
        });
    });
    // 활동 + 진행률
    if let Some(act) = la.act_type.as_deref().filter(|s| !s.is_empty()) {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(act_icon(act)).size(12.0).color(theme::ACCENT_SAND));
            ui.label(egui::RichText::new(act).size(11.0).color(theme::TEXT_DIM));
            if let Some(p) = la.act_progress {
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(egui::RichText::new(format!("{}%", (p * 100.0) as u32)).size(10.0).color(theme::TEXT_DIM));
                });
            }
        });
        if let Some(p) = la.act_progress {
            thin_bar(ui, p, theme::ACCENT_SAND, 4.0);
        }
    }
    // 나이·성별(lookJson) — 외형 시드와 별개 표기
    if let Some(bits) = la.look.as_deref().and_then(|l| serde_json::from_str::<LookBits>(l).ok()) {
        ui.horizontal(|ui| {
            let sex = match bits.sex.as_deref() {
                Some("female") => Some(tr(lang, Tr::Female)),
                Some("male") => Some(tr(lang, Tr::Male)),
                _ => None,
            };
            if let Some(s) = sex {
                ui.label(egui::RichText::new(s).size(10.0).color(theme::TEXT_DIM));
            }
            if let Some(a) = bits.age {
                ui.label(egui::RichText::new(trf(lang, Tr::Age, &a.to_string())).size(10.0).color(theme::TEXT_DIM));
            }
        });
    }
    // 목표 — goalKey 템플릿 우선(events.json goal.*), 폴백 goalText 원문.
    // 파라미터 치환값이 없어(계약에 goal-params 미탑재) 토큰이 남으면
    // goalText 원문으로 물러난다 — "{species을}" 리터럴 노출 방지(리뷰).
    let goal = la
        .goal_key
        .as_deref()
        .filter(|k| vocab.has_event(k))
        .map(|k| vocab.event_text(lang, &serde_json::json!({"key": k})))
        .map(|t| {
            if t.contains('{') {
                la.goal_text
                    .clone()
                    .filter(|s| !s.is_empty())
                    .unwrap_or(t)
            } else {
                t
            }
        })
        .or_else(|| la.goal_text.as_deref().filter(|s| !s.is_empty()).map(str::to_owned));
    if let Some(g) = goal {
        ui.label(egui::RichText::new(format!("{} {g}", tr(lang, Tr::Goal))).size(10.5).color(theme::TEXT_DIM));
    }
    ui.add_space(3.0);
    // 욕구 바 4종 — >70 빨강(레거시 규칙)
    section(ui, regular::HEART, tr(lang, Tr::Villagers));
    need_bar(ui, tr(lang, Tr::Hunger), "食", la.hunger, theme::NEED_HUNGER);
    need_bar(ui, tr(lang, Tr::Thirst), "水", la.thirst, theme::NEED_THIRST);
    need_bar(ui, tr(lang, Tr::Fatigue), "眠", la.fatigue, theme::NEED_FATIGUE);
    need_bar(ui, tr(lang, Tr::Warmth), "溫", la.warmth, theme::NEED_WARMTH);
    // 소지품 — 칩 + 짐 무게 바
    let items = inv_items(la.inv_json.as_deref());
    section(ui, regular::HANDBAG, tr(lang, Tr::Inventory));
    if items.is_empty() {
        ui.label(egui::RichText::new(tr(lang, Tr::InventoryEmpty)).size(10.0).color(theme::TEXT_DIM));
    } else {
        let wt = carry_weight(vocab, &items);
        let ratio = (wt / CARRY_CAP).clamp(0.0, 1.0);
        let heavy = ratio >= 0.85;
        ui.horizontal_wrapped(|ui| {
            for (id, n) in &items {
                let label = format!("{} ×{}", vocab.item(lang, id), n);
                super::widgets::item_chip(ui, &label)
                    .on_hover_text(vocab.item(Lang::En, id));
            }
        });
        thin_bar(ui, ratio, if heavy { theme::WARN } else { theme::LIVE_GREEN }, 5.0);
        ui.label(
            egui::RichText::new(format!("{:.0} / {:.0}", wt, CARRY_CAP))
                .size(9.5)
                .color(if heavy { theme::WARN } else { theme::TEXT_DIM }),
        );
    }
    // 인연 — 이 주민 관계 상위 6(레거시 패리티)
    let rels: Vec<crate::net::RelRow> = polls
        .relations
        .lock()
        .map(|q| {
            q.iter()
                .filter(|r| r.a_id == tag_id || r.b_id == tag_id)
                .cloned()
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if !rels.is_empty() {
        section(ui, regular::LINK, tr(lang, Tr::Relations));
        let mut sorted = rels;
        sorted.sort_by(|a, b| b.warmth.partial_cmp(&a.warmth).unwrap_or(std::cmp::Ordering::Equal));
        for r in sorted.iter().take(6) {
            rel_row(ui, lang, frame, tag_id, r);
        }
    }
    // 생활기록 — 유형색(최근 8행 역순)
    let hist = polls.history.lock().map(|q| q.clone()).unwrap_or_default();
    if !hist.is_empty() {
        section(ui, regular::CLOCK_COUNTER_CLOCKWISE, tr(lang, Tr::History));
        egui::ScrollArea::vertical()
            .id_salt("vhist")
            .max_height(150.0)
            .auto_shrink([false, false])
            .show(ui, |ui| {
                for h in hist.iter().rev().take(12) {
                    ui.horizontal(|ui| {
                        ui.label(
                            egui::RichText::new(super::fmt_hhmm(Some(h.ts.as_str())))
                                .size(9.5)
                                .color(theme::TEXT_DIM),
                        );
                        ui.label(
                            egui::RichText::new(vocab.hist(lang, &h.type_))
                                .size(9.5)
                                .strong()
                                .color(hist_color(&h.type_)),
                        );
                        ui.label(egui::RichText::new(&h.text).size(9.5).color(theme::TEXT_WARM));
                    });
                }
            });
    }
}

/// 관계 행 — 상대 이름(주민색) + 온기 바(hsl) + 등급 라벨 + 대화/거래 수
fn rel_row(
    ui: &mut egui::Ui,
    lang: Lang,
    frame: Option<&crate::net::LiveFrame>,
    self_id: &str,
    r: &crate::net::RelRow,
) {
    let other_id = if r.a_id == self_id { &r.b_id } else { &r.a_id };
    let name = frame
        .and_then(|f| f.agents.iter().find(|a| &a.id == other_id))
        .map(|a| a.name.clone())
        .unwrap_or_else(|| other_id.chars().take(8).collect());
    let warmth = r.warmth.clamp(0.0, 100.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(name)
                .size(11.0)
                .color(theme::agent_ink(other_id)),
        );
        let (rect, _) = ui.allocate_exact_size(egui::vec2((ui.available_width() - 132.0).max(40.0), 6.0), egui::Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, 3.0, theme::track());
        let mut fr = rect;
        fr.set_width(rect.width() * warmth / 100.0);
        p.rect_filled(fr, 3.0, super::vocab::warmth_hsl(warmth));
        ui.label(
            egui::RichText::new(format!("{} {:.0}", theme::warmth_tier(lang, warmth), warmth))
                .size(9.5)
                .color(super::vocab::warmth_hsl(warmth)),
        );
    })
    .response
    .on_hover_text(tr(lang, Tr::RelChatsTrades)
        .replacen("{}", &r.chats.to_string(), 1)
        .replacen("{}", &r.trades.to_string(), 1));
}

/// fmt_gmin — 절대 게임 분 → "D{n} HH:MM"(FE 직역, 연도 표기 생략형)
pub fn fmt_gmin(gmin: i64) -> String {
    let day = gmin.div_euclid(1440) + 1; // 1기준 — 상단바 태양력 날짜와 동일 관례
    let min = gmin.rem_euclid(1440);
    format!("D{day} {:02}:{:02}", min / 60, min % 60)
}
