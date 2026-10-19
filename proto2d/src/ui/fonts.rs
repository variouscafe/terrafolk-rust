//! egui 한글 폰트 — NotoSansKR를 런타임 에셋으로 로드해 설치.
//! include_bytes!는 wasm +4.4MB라 불가(CF Pages 25MiB 한도) — 에셋 경로 유지.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

pub fn install_korean_font(
    mut commands: Commands,
    assets: Res<AssetServer>,
    fonts: Res<Assets<bevy::text::Font>>,
    mut contexts: EguiContexts,
    mut handle: Local<Option<Handle<bevy::text::Font>>>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    let h = handle.get_or_insert_with(|| {
        let h: Handle<bevy::text::Font> = assets.load("fonts/NotoSansKR-Regular.otf");
        commands.insert_resource(KoreanFont(h.clone()));
        h
    });
    let Some(font) = fonts.get(h) else {
        return; // 다음 프레임 재시도
    };
    let Some(ctx) = contexts.ctx_mut().ok().cloned() else {
        return;
    };
    let mut defs = egui::FontDefinitions::default();
    // proportional 첫 폰트의 폴백으로 추가 — 라틴은 egui 기본, 한글은 Noto
    let bytes: Vec<u8> = font.data.data().to_vec();
    defs.font_data.insert(
        "noto_kr".into(),
        egui::FontData::from_owned(bytes).into(),
    );
    if let Some(family) = defs.families.get_mut(&egui::FontFamily::Proportional) {
        family.push("noto_kr".into());
    }
    if let Some(family) = defs.families.get_mut(&egui::FontFamily::Monospace) {
        family.push("noto_kr".into());
    }
    ctx.set_fonts(defs);
    *done = true;
    info!("[ui] egui 한글 폰트 설치 완료");
}

/// 설치된 폰트 핸들(핸들 유지 — 에셋 언로드 방지)
#[derive(Resource)]
pub struct KoreanFont(#[allow(dead_code)] pub Handle<bevy::text::Font>);
