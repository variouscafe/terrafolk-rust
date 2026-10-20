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

/// 온디맨드 CJK 폰트 — 언어를 zh/ja로 바꾸면 그 폰트를 추가 적재해 사슬을
/// 다시 설치한다(한국어 사용자는 8~16MB 다운로드를 물지 않는다). 커버리지:
/// NotoSansKR은 가나·간체가 없다 — zh=간체(SC), ja=가나+상용한자(JP).
/// 로케일별 자산 — 에셋은 wasm 밖(에셋 파일)이라 wasm 크기 불변.
pub fn install_cjk_font(
    assets: Res<AssetServer>,
    fonts: Res<Assets<bevy::text::Font>>,
    base: Option<Res<KoreanFont>>,
    ui_state: Res<crate::ui::UiState>,
    mut contexts: EguiContexts,
    mut handle: Local<Option<Handle<bevy::text::Font>>>,
    mut installed: Local<Option<crate::ui::i18n::Lang>>,
) {
    use crate::ui::i18n::Lang;
    let want = match ui_state.lang {
        Lang::Zh => Some(("fonts/NotoSansSC-Regular.otf", "noto_sc", Lang::Zh)),
        Lang::Ja => Some(("fonts/NotoSansJP-Regular.otf", "noto_jp", Lang::Ja)),
        _ => None,
    };
    let Some((path, name, lang)) = want else {
        return; // ko/en — 기본 폰트(한자는 NotoSansKR 폴백)
    };
    if *installed == Some(lang) {
        return;
    }
    let Some(base) = base else {
        return; // 기본 폰트 아직 — install_korean_font가 먼저
    };
    let h = handle.get_or_insert_with(|| assets.load(path));
    let Some(f) = fonts.get(h) else {
        return; // 다운로드 중 — 다음 프레임 재시도
    };
    let Some(ctx) = contexts.ctx_mut().ok().cloned() else {
        return;
    };
    let bytes = f.data.data().to_vec();
    // 사슬 재구성 — 기본 3종 + CJK. CJK 폰트를 noto_kr 앞에(우선 매치).
    let mut defs = egui::FontDefinitions::default();
    let base_names = [
        ("pretendard", "fonts/Pretendard-Regular.otf"),
        ("pretendard_semi", "fonts/Pretendard-SemiBold.otf"),
        ("noto_kr", "fonts/NotoSansKR-Regular.otf"),
    ];
    for (n, p) in base_names {
        if let Some(h) = base.0.iter().find(|_| true) {
            let _ = h; // 아래에서 fonts.get 재조회
        }
        let bh: Handle<bevy::text::Font> = assets.load(p);
        let Some(bf) = fonts.get(&bh) else {
            return; // 기본 폰트 로드 확인(캐시히트라 즉시)
        };
        defs.font_data
            .insert(n.into(), egui::FontData::from_owned(bf.data.data().to_vec()).into());
    }
    defs.font_data
        .insert(name.into(), egui::FontData::from_owned(bytes).into());
    egui_phosphor::add_to_fonts(&mut defs, egui_phosphor::Variant::Regular);
    defs.families.insert(
        egui::FontFamily::Proportional,
        vec![
            "pretendard".into(),
            name.into(),
            "noto_kr".into(),
            "phosphor".into(),
        ],
    );
    defs.families.insert(
        egui::FontFamily::Monospace,
        vec!["Hack".into(), name.into(), "noto_kr".into()],
    );
    defs.families.insert(
        egui::FontFamily::Name("semi".into()),
        vec!["pretendard_semi".into(), name.into(), "noto_kr".into()],
    );
    ctx.set_fonts(defs);
    *installed = Some(lang);
    info!("[ui] CJK 폰트 설치 — {name}({lang:?})");
}
