//! 공용 egui 위젯 — 코지 글래스 크롬(레퍼런스 UI 실측) 빌딩 블록.
//! 글래스 카드·도킹 바·타이틀 바(골드 아이콘 칩)·섹션 헤더·베벨 필 바·칩.

use bevy_egui::egui;

use super::theme;

/// 플로팅 카드 프레임 — 다크 올리브 글래스 + 라이트 올리브 스트로크 +
/// 부드러운 그림자(레퍼런스 글래스 패널 #222F28@85%·#586A50 실측)
pub fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(theme::glass())
        .stroke(egui::Stroke::new(1.5_f32, theme::STROKE_OLIVE))
        .corner_radius(12.0)
        .inner_margin(egui::Margin::same(10))
        .shadow(egui::Shadow {
            offset: [0, 4],
            blur: 14,
            spread: 0,
            color: egui::Color32::from_black_alpha(110),
        })
}

/// 도킹 패널 프레임 — 상단 바·우측 패널·채팅 바(불투명 가독)
pub fn dock() -> egui::Frame {
    egui::Frame::new()
        .fill(theme::glass_dock())
        .stroke(egui::Stroke::new(1.0_f32, theme::STROKE_OLIVE))
}

// ── 모던 플랫 변형(2026-10-20) — 스트로크 없는 라운드, 채움·섀도로만 분리 ──

/// 플로팅 카드(플랫) — 스트로크 빠진 라운드 글래스 + 소프트 섀도.
/// '요즘 웹 UI' 톤 — 채팅 대화창 등에서 사용.
pub fn card_soft() -> egui::Frame {
    egui::Frame::new()
        .fill(theme::glass())
        .corner_radius(18.0)
        .inner_margin(egui::Margin::same(12))
        .shadow(egui::Shadow {
            offset: [0, 8],
            blur: 24,
            spread: 0,
            color: egui::Color32::from_black_alpha(90),
        })
}

/// 도킹 바(플랫) — 스트로크 대신 위로 뻗는 섀도로 배경과 분리
pub fn dock_flat() -> egui::Frame {
    egui::Frame::new().fill(theme::glass_dock()).shadow(egui::Shadow {
        offset: [0, -3],
        blur: 12,
        spread: 0,
        color: egui::Color32::from_black_alpha(55),
    })
}

/// 골드 아이콘 칩(플랫) — 링 없는 골드 틴트 원
pub fn icon_chip_flat(ui: &mut egui::Ui, icon: &str, size: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let p = ui.painter();
    let c = rect.center();
    p.circle_filled(c, size * 0.5, egui::Color32::from_rgba_unmultiplied(232, 184, 64, 30));
    p.text(
        c,
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(size * 0.62),
        theme::GOLD_SOFT,
    );
}

/// 타이틀 바(플랫) — 플랫 아이콘 칩 + 크림 타이틀
pub fn title_bar_flat(ui: &mut egui::Ui, icon: &str, title: &str) {
    ui.horizontal(|ui| {
        icon_chip_flat(ui, icon, 20.0);
        ui.label(
            egui::RichText::new(title)
                .size(13.5)
                .strong()
                .color(theme::INK),
        );
    });
}

/// 배지 칩(플랫) — 스트로크 없는 필만
pub fn chip_flat(ui: &mut egui::Ui, text: &str, fg: egui::Color32, bg: egui::Color32) {
    egui::Frame::new()
        .fill(bg)
        .corner_radius(999.0)
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(10.5).color(fg).strong());
        });
}

/// 원형 아바타(플랫) — 링 없는 색 원 + 이니셜(모던 톤)
pub fn avatar_dot(ui: &mut egui::Ui, initial: &str, color: egui::Color32, size: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let p = ui.painter();
    let c = rect.center();
    p.circle_filled(c, size * 0.5, color);
    p.text(
        c,
        egui::Align2::CENTER_CENTER,
        initial,
        egui::FontId::proportional(size * 0.45),
        egui::Color32::from_rgb(40, 34, 24),
    );
}

/// 골드 아이콘 칩 — 한자 아이콘을 골드 반투명 원 위에(레퍼런스
/// 섹션 헤더 아이콘 칩 직역)
pub fn icon_chip(ui: &mut egui::Ui, icon: &str, size: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let p = ui.painter();
    let c = rect.center();
    let r = size * 0.5;
    p.circle_filled(c, r, egui::Color32::from_rgba_unmultiplied(232, 184, 64, 26));
    p.circle_stroke(c, r - 0.5, egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(232, 184, 64, 90)));
    p.text(
        c,
        egui::Align2::CENTER_CENTER,
        icon,
        egui::FontId::proportional(size * 0.62),
        theme::GOLD_SOFT,
    );
}

/// 패널 타이틀 바 — 골드 아이콘 칩 + 크림 타이틀(윈도우 헤더 공용)
pub fn title_bar(ui: &mut egui::Ui, icon: &str, title: &str) {
    ui.horizontal(|ui| {
        icon_chip(ui, icon, 20.0);
        ui.label(
            egui::RichText::new(title)
                .size(13.5)
                .strong()
                .color(theme::INK),
        );
    });
}

/// 섹션 헤더 — 골드 아이콘 칩(소형) + 잉크 타이틀 + 어두운 헤어라인
pub fn section(ui: &mut egui::Ui, icon: &str, title: &str) {
    ui.add_space(4.0);
    ui.horizontal(|ui| {
        icon_chip(ui, icon, 16.0);
        ui.label(
            egui::RichText::new(title)
                .size(12.5)
                .strong()
                .color(theme::INK),
        );
    });
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 2.0), egui::Sense::hover());
    let p = ui.painter();
    p.line_segment(
        [rect.left_center(), rect.right_center()],
        egui::Stroke::new(1.0_f32, egui::Color32::from_black_alpha(70)),
    );
    ui.add_space(3.0);
}

/// 작은 배지 칩 — 글래스 필 + 올리브 스트로크(레퍼런스 아이콘 칩 직역)
pub fn chip(ui: &mut egui::Ui, text: &str, fg: egui::Color32, bg: egui::Color32) {
    egui::Frame::new()
        .fill(bg)
        .stroke(egui::Stroke::new(1.0_f32, theme::STROKE_OLIVE))
        .corner_radius(999.0)
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(10.5).color(fg).strong());
        });
}

/// 소지품 칩 — 레퍼런스 핫바 슬롯 직역(퍼치먼트 테두리 + 어두운 내부)
pub fn item_chip(ui: &mut egui::Ui, text: &str) -> egui::Response {
    egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(18, 24, 20, 200))
        .stroke(egui::Stroke::new(1.2_f32, theme::parchment()))
        .corner_radius(7.0)
        .inner_margin(egui::Margin::symmetric(7, 2))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(10.0).color(theme::INK));
        })
        .response
}

/// 베벨 필 채움 — 밝은 상단 하이라이트 + 어두운 하단 셰이드로
/// 핸드페인트 약병 바 질감(레퍼런스 HP/스태미나 바 실측).
fn pill_fill(p: &egui::Painter, rect: egui::Rect, color: egui::Color32) {
    let r = rect.height() / 2.0;
    p.rect_filled(rect, r, color);
    let mut hi = rect;
    hi.set_height((rect.height() * 0.5).max(1.0));
    p.rect_filled(hi, (r * 0.9).max(1.0), egui::Color32::from_white_alpha(48));
    let mut lo = rect;
    lo.set_top(rect.bottom() - 1.5);
    p.rect_filled(lo, 1.0, egui::Color32::from_black_alpha(40));
}

/// 둥근 게이지 바(필) — 라벨 왼쪽, 바+값 오른쪽. 위험(≥70%)이면 빨강.
/// 레거시 FE 욕구 바(70 임계 빨강) 직역 — 어두운 트랙에 베벨 필.
pub fn need_bar(
    ui: &mut egui::Ui,
    label: &str,
    icon: &str,
    value: Option<f32>,
    color: egui::Color32,
) {
    let Some(v) = value else { return };
    let v = v.clamp(0.0, 100.0);
    let danger = v >= 70.0;
    let fill = if danger { theme::DANGER } else { color };
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(icon).size(11.0).color(fill));
        ui.label(egui::RichText::new(label).size(11.5).color(theme::INK_DIM));
        let bw = (ui.available_width() - 36.0).max(30.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(bw, 10.0), egui::Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, 5.0, theme::track());
        let mut fr = rect;
        fr.set_width(rect.width() * v / 100.0);
        pill_fill(p, fr, fill);
        p.rect_stroke(rect, 5.0, egui::Stroke::new(1.0_f32, theme::STROKE_OLIVE), egui::StrokeKind::Middle);
        ui.label(
            egui::RichText::new(format!("{}%", v.round() as u32))
                .size(10.5)
                .color(if danger { theme::DANGER } else { theme::INK_DIM }),
        );
    });
}

/// 얇은 진행 필 바(행동 진행률·건설 progress) — 라벨 없이 바만
pub fn thin_bar(ui: &mut egui::Ui, ratio: f32, color: egui::Color32, h: f32) {
    let ratio = ratio.clamp(0.0, 1.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, h / 2.0, theme::track());
    let mut fr = rect;
    fr.set_width(rect.width() * ratio);
    pill_fill(p, fr, color);
}

/// 상태 점 — 주민 행동 상태 색 점(어두운 링 — 월드 점과 동일 관례)
pub fn status_dot(ui: &mut egui::Ui, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
    let p = ui.painter();
    p.circle_stroke(rect.center(), 4.2, egui::Stroke::new(1.2_f32, egui::Color32::from_rgba_unmultiplied(16, 22, 15, 180)));
    p.circle_filled(rect.center(), 3.5, color);
}

/// 원형 아바타 칩 — 크림 링 + 주민색 필 + 이니셜(레퍼런스 프로필
/// 아바타 직역 — 선택 상세 카드 헤더용)
pub fn avatar_chip(ui: &mut egui::Ui, initial: &str, color: egui::Color32, size: f32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    let p = ui.painter();
    let c = rect.center();
    let r = size * 0.5;
    p.circle_filled(c, r, color);
    p.circle_stroke(c, r - 1.0, egui::Stroke::new(2.0_f32, theme::INK));
    p.text(
        c,
        egui::Align2::CENTER_CENTER,
        initial,
        egui::FontId::proportional(size * 0.45),
        egui::Color32::from_rgb(40, 34, 24),
    );
}
