//! egui 폰트·아이콘 — Pretendard(한글·라틴 본체) + NotoSansKR(한자 폴백) +
//! Phosphor(아이콘). include_bytes!는 wasm +6MB라 불가(CF Pages 25MiB 한도) —
//! 에셋 경로 유지. Pretendard에 한자가 없어 표의 한자(伐·話)는 Noto가 맡고,
//! Phosphor 아이콘(PUA 영역)은 Proportional 폴백 사슬 끝에서 해석된다.

use bevy::prelude::*;
use bevy_egui::{EguiContexts, egui};

/// 설치 대상 폰트 에셋 — (핸들 이름, 에셋 경로)
const FONT_ASSETS: [(&str, &str); 3] = [
    ("pretendard", "fonts/Pretendard-Regular.otf"),
    ("pretendard_semi", "fonts/Pretendard-SemiBold.otf"),
    ("noto_kr", "fonts/NotoSansKR-Regular.otf"),
];

pub fn install_korean_font(
    mut commands: Commands,
    assets: Res<AssetServer>,
    fonts: Res<Assets<bevy::text::Font>>,
    mut contexts: EguiContexts,
    mut handles: Local<Vec<Handle<bevy::text::Font>>>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    if handles.is_empty() {
        for (_, path) in FONT_ASSETS {
            let h: Handle<bevy::text::Font> = assets.load(path);
            handles.push(h);
        }
        commands.insert_resource(KoreanFont(handles.clone()));
    }
    // 전부 로드됐을 때만 설치 — 일부만으로 set_fonts하면 나머지가 영구 누락
    if handles.iter().any(|h| fonts.get(h).is_none()) {
        return; // 다음 프레임 재시도
    }
    let Some(ctx) = contexts.ctx_mut().ok().cloned() else {
        return;
    };
    let mut defs = egui::FontDefinitions::default();
    for ((name, _), h) in FONT_ASSETS.iter().zip(handles.iter()) {
        let bytes = fonts.get(h).unwrap().data.data().to_vec();
        defs.font_data
            .insert((*name).into(), egui::FontData::from_owned(bytes).into());
    }
    // Phosphor 아이콘 폰트 — Proportional 1번지에 끼워 넣는다(크레이트 기본).
    // 아래에서 사슬을 통째로 다시 만들므로 여기선 등록만.
    egui_phosphor::add_to_fonts(&mut defs, egui_phosphor::Variant::Regular);
    // 본체 사슬 재구성 — 라틴·한글은 Pretendard, 한자는 Noto, 아이콘(PUA)은
    // Phosphor. egui 기본 폰트(Ubuntu 등)는 빼 톤을 Pretendard로 통일.
    defs.families.insert(
        egui::FontFamily::Proportional,
        vec!["pretendard".into(), "noto_kr".into(), "phosphor".into()],
    );
    defs.families.insert(
        egui::FontFamily::Monospace,
        vec!["Hack".into(), "noto_kr".into()],
    );
    // 헤더·타이틀용 세미볼드 패밀리(theme::font_semi가 쓴다)
    defs.families.insert(
        egui::FontFamily::Name("semi".into()),
        vec!["pretendard_semi".into(), "noto_kr".into()],
    );
    ctx.set_fonts(defs);
    // 이미지 로더 — UI 어디서든 Image::from_uri로 아이콘·스프라이트 표시 가능
    // (에셋 CDN의 발명품·아이템 아이콘 붙일 준비. http는 ehttp→wasm fetch)
    egui_extras::install_image_loaders(&ctx);
    *done = true;
    info!("[ui] egui 폰트 설치 완료 — Pretendard+Noto+Phosphor·이미지 로더");
}

/// 설치된 폰트 핸들(핸들 유지 — 에셋 언로드 방지)
#[derive(Resource)]
pub struct KoreanFont(#[allow(dead_code)] pub Vec<Handle<bevy::text::Font>>);
