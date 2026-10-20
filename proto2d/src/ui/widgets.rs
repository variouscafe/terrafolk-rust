//! 공용 egui 위젯 — 상용 게임 크롨(v2) 빌딩 블록.
//! 카드 프레임·섹션 헤더·욕구/진행 바·칩 배지. 전부 painter 기반 커스텀.

use bevy_egui::egui;

use super::theme;

/// 플로팅 카드 프레임 — 반투명 웜 그린 + 은은한 스트로크 + 그림자
pub fn card() -> egui::Frame {
    egui::Frame::new()
        .fill(egui::Color32::from_rgba_unmultiplied(24, 30, 19, 242))
        .stroke(egui::Stroke::new(1.0_f32, egui::Color32::from_white_alpha(22)))
        .corner_radius(10.0)
        .inner_margin(egui::Margin::same(10))
        .shadow(egui::Shadow {
            offset: [0, 3],
            blur: 10,
            spread: 0,
            color: egui::Color32::from_black_alpha(120),
        })
}

/// 섹션 헤더 — 한자 아이콘 + 타이틀 + 골드 언더라인
pub fn section(ui: &mut egui::Ui, icon: &str, title: &str) {
    ui.add_space(3.0);
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(icon)
                .size(13.0)
                .color(theme::ACCENT_SAND)
                .strong(),
        );
        ui.label(
            egui::RichText::new(title)
                .size(13.5)
                .color(theme::TEXT_WARM)
                .strong(),
        );
    });
    let w = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(w, 2.0), egui::Sense::hover());
    let p = ui.painter();
    p.line_segment(
        [rect.left_center(), rect.right_center()],
        egui::Stroke::new(1.0_f32, egui::Color32::from_rgba_unmultiplied(233, 203, 125, 60)),
    );
    ui.add_space(3.0);
}

/// 작은 배지 칩 — 배경색+글자색. 통계·라벨 표시용
pub fn chip(ui: &mut egui::Ui, text: &str, fg: egui::Color32, bg: egui::Color32) {
    egui::Frame::new()
        .fill(bg)
        .corner_radius(999.0)
        .inner_margin(egui::Margin::symmetric(7, 2))
        .show(ui, |ui| {
            ui.label(egui::RichText::new(text).size(10.5).color(fg).strong());
        });
}

/// 둥근 게이지 바 — label 왼쪽, 바+값 오른쪽. 위험(≥threshold%)이면 빨강.
/// 레거시 FE 욕구 바(70 임계 빨강) 직역.
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
        ui.label(egui::RichText::new(label).size(11.5).color(theme::TEXT_DIM));
        let bw = (ui.available_width() - 34.0).max(30.0);
        let (rect, _) = ui.allocate_exact_size(egui::vec2(bw, 8.0), egui::Sense::hover());
        let p = ui.painter();
        p.rect_filled(rect, 4.0, egui::Color32::from_black_alpha(90));
        let mut fr = rect;
        fr.set_width(rect.width() * v / 100.0);
        p.rect_filled(fr, 4.0, fill);
        p.rect_stroke(rect, 4.0, egui::Stroke::new(1.0_f32, egui::Color32::from_white_alpha(16)), egui::StrokeKind::Middle);
        ui.label(
            egui::RichText::new(format!("{}%", v.round() as u32))
                .size(10.5)
                .color(if danger { theme::DANGER } else { theme::TEXT_DIM }),
        );
    });
}

/// 얇은 진행 바(행동 진행률·건설 progress) — 라벨 없이 바만
pub fn thin_bar(ui: &mut egui::Ui, ratio: f32, color: egui::Color32, h: f32) {
    let ratio = ratio.clamp(0.0, 1.0);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), h), egui::Sense::hover());
    let p = ui.painter();
    p.rect_filled(rect, h / 2.0, egui::Color32::from_black_alpha(90));
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

