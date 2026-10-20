//! 공용 egui 위젯 — 코지 크림 크롬(레퍼런스 UI 추출) 빌딩 블록.
//! 카드 프레임·섹션 헤더·욕구/진행 필 바·칩 배지. 전부 painter 기반 커스텀.

use bevy_egui::egui;

use super::theme;

/// 플로팅 카드 프레임 — 크림 표면 + 탄색 스트로크 + 부드러운 그림자
/// (레퍼런스: 크림 토스트·핫바, 12~14px 라운드)
pub fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(245, 242, 234, 248))
        .stroke(egui::Stroke::new(1.6_f32, theme::STROKE_TAN))
        .corner_radius(12.0)
        .inner_margin(egui::Margin::same(10))
        .shadow(egui::Shadow {
            offset: [0, 3],
            blur: 10,
            spread: 0,
            color: egui::Color32::from_black_alpha(90),
        })
}

/// 섹션 헤더 — 한자 아이콘(골드) + 잉크 타이틀 + 탄색 언더라인
pub fn section(ui: &mut egui::Ui, icon: &str, title: &str) {
    ui.add_space(3.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(icon)
                .size(13.0)
                .color(theme::GOLD_DEEP)
                .strong(),
        );
        ui.label(
            egui::RichText::new(title)
                .size(13.0)
                .color(theme::INK)
                .strong(),
        );
    });
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 2.0), egui::Sense::hover());
    let p = ui.painter();
    p.line_segment(
        [rect.left_center(), rect.right_center()],
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(185, 168, 140, 90)),
    );
    ui.add_space(3.0);
}

/// 작은 배지 칩 — 크림 필 + 탄색 스트로크(레퍼런스 아이콘 칩 직역)
pub fn chip(ui: &mut egui::Ui, text: &str, fg: egui::Color32, bg: egui::Color32) {
    egui::Frame::new()
        .fill(bg)
        .stroke(egui::Stroke::new(1.0_f32, theme::STROKE_TAN))
        .corner_radius(999.0)
        .inner_margin(egui::Margin::symmetric(8, 2))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(10.5).color(fg).strong());
        });
}

/// 둥근 게이지 바(필) — 라벨 왼쪽, 바+값 오른쪽. 위험(≥70%)이면 빨강.
/// 레거시 FE 욕구 바(70 임계 빨강) 직역 — 크림 트랙에 포화 필.
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
        let (rect, _) = ui.allocate_exact_size(egui::vec2(bw, 9.0), egui::Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, 4.5, theme::CREAM_3);
        let mut fr = rect;
        fr.set_width(rect.width() * v / 100.0);
        p.rect_filled(fr, 4.5, fill);
        p.rect_stroke(rect, 4.5, egui::Stroke::new(1.0_f32, theme::STROKE_TAN), egui::StrokeKind::Middle);
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
    p.rect_filled(rect, h / 2.0, theme::CREAM_3);
    let mut fr = rect;
    fr.set_width(rect.width() * ratio);
    p.rect_filled(fr, h / 2.0, color);
}

/// 상태 점 — 주민 행동 상태 색 점
pub fn status_dot(ui: &mut egui::Ui, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(10.0, 10.0), egui::Sense::hover());
    let p = ui.painter();
    p.circle_filled(rect.center(), 3.5, color);
}
