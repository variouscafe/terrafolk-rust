//! Terrafolk 2D proto — 원본 FE(웹) 2D 도트 룩의 Bevy 패리티 프로토타입.
//! 레거시 맵(shared-map.json)·LPC 주민·픽셀 프롭을 정투형 타일 월드로 렌더.
mod agents;
mod ambient;
mod cam;
mod daynight;
mod ground;
mod mapdata;
mod net;
mod nightglow;
mod props;
mod ui;
mod ui2;
mod weather;

use bevy::prelude::*;

#[cfg(target_arch = "wasm32")]
fn install_panic_hook() {
    console_error_panic_hook::set_once();
}

/// URL 실험 스위치(?exp=a|b|c|d) — 실기기 스프라이트 미렌더 판별용.
/// a=통제 · b=타일 축소(중심 49×49) · c=MSAA 끔 · d=b+c
#[derive(Resource)]
pub struct Exp(pub char);

/// r/s/t 실험 상태 노트 — 상태표시줄에 표시(스폰/재기입 완료 시점 판별)
#[derive(Resource, Default)]
pub struct ProbeNote(pub String);

/// 지면 스킨 초기값 — 웹 ?exp=w · 네이티브 TF_SKIN=snow(눈 프리뷰·E2E 게이트).
/// 라이브 접속 후엔 계절·날씨가 덮어쓴다(net/apply::sync_daynight).
fn initial_skin(_exp: char) -> ground::GroundSkin {
    #[cfg(target_arch = "wasm32")]
    {
        if _exp == 'w' {
            return ground::GroundSkin::Snow;
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        if std::env::var("TF_SKIN").as_deref() == Ok("snow") {
            return ground::GroundSkin::Snow;
        }
    }
    ground::GroundSkin::Normal
}

fn url_exp() -> char {
    #[cfg(target_arch = "wasm32")]
    {
        web_sys::window()
            .and_then(|w| w.location().search().ok())
            .and_then(|s| {
                s.trim_start_matches('?')
                    .split('&')
                    .find(|p| p.starts_with("exp="))
                    .and_then(|p| p.chars().nth(4))
            })
            .unwrap_or('a')
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        'a'
    }
}

fn main() {
    #[cfg(target_arch = "wasm32")]
    install_panic_hook();

    let exp = url_exp();
    let mut app = App::new();
    app.insert_resource(Exp(exp))
        .insert_resource(initial_skin(exp))
        .insert_resource(ClearColor(Color::srgb(0.13, 0.18, 0.11))) // 진단성 있는 깊은 초록
        .insert_resource(cam::TapPos::default())
        .insert_resource(ProbeNote::default())
        .insert_resource(THandle::default())
        .insert_resource(cam::DelayedCamera {
            spawned: false,
            last_count: 0,
            stable_since: 0.0,
        });
    app
        .add_plugins((ui2::Ui2Plugin,
            DefaultPlugins
                .set(bevy::asset::AssetPlugin {
                    // 웹 배포(CF Pages)가 없는 *.meta를 200+HTML로 응답해 파싱 에러 →
                    // 프로브 자체를 끈다(미처리 에셋 모드 — 로컬·프로덕션 동일 동작).
                    meta_check: bevy::asset::AssetMetaCheck::Never,
                    ..default()
                })
                .set(WindowPlugin {
                    primary_window: Some(Window {
                        title: "Terrafolk 2D proto".into(),
                        canvas: Some("#bevy".into()),
                        fit_canvas_to_parent: true,
                        ..default()
                    }),
                    ..default()
                })
                .set(ImagePlugin::default_linear()), // 레마스터 룩 — ×2~4 고밀도 아트 + 매끄러운 샘플링
        ))
        .insert_resource(cam::DragState { total: 0.0 })
        .insert_resource(cam::ManualUntil(0.0))
        .insert_resource(daynight::GameTime { day: 0, t: 0.35 })
        .init_resource::<weather::WeatherKind>() // 08:24 시작
        .add_systems(
            Startup,
            (
                cam::spawn_camera,
                ground::spawn.run_if(|e: Res<Exp>| !matches!(e.0, 'h' | 'p' | 'r' | 's' | 't' | 'u' | 'v' | 'x')),
                props::spawn.run_if(|e: Res<Exp>| !matches!(e.0, 'h' | 'p' | 'r' | 's' | 't' | 'u' | 'v' | 'x')),
                agents::spawn.run_if(|e: Res<Exp>| !matches!(e.0, 'h' | 'p' | 'r' | 's' | 't' | 'u' | 'v' | 'x')),
                ambient::spawn_fireflies,
                ambient::spawn_clouds,
                spawn_exp_probes,
            ),
        )
        .add_plugins(net::NetPlugin)
        .add_plugins(ui::UiPlugin)
        .add_systems(
            Update,
            (
                ambient::sway_trees,
                ambient::update_fireflies,
                nightglow::attach,
                nightglow::update,
                weather::attach,
                weather::sync,
                weather::fall,
                ambient::drift_clouds,
                daynight::attach_tint,
                daynight::update_tint,
                daynight::advance,
            ),
        )
        .add_systems(
            Update,
            (
                net::chat::fade,
                cam::pan_zoom,
                cam::touch_gestures,
                cam::delayed_camera,
                exp_p_probe,
                exp_s_probe,
                exp_t_rewrite,
                exp_v_probe,
                agents::wander,
                agents::animate,
                props::animate_campfire,
                ground::water_frames,
                agents::select_on_click,
                agents::select_on_tap,
                agents::marker_follow,
                cam::follow_selected,
                diag_title,
            ),
        )
        .run();
}

/// 실험 프로브 — 렌더 계층 판별(실기기 스프라이트 미렌더 추적).
/// e=텍스처 없는 색 쿼드(텍스처 업로드 vs 파이프라인) · g=월드 공간 Text2d(월드 패스 생존)
/// 런타임 단색 이미지 헬퍼
fn runtime_solid_image(size: u32, rgba: [u8; 4]) -> Image {
    Image::new(
        bevy::render::render_resource::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        rgba.iter().cycle().take((size * size * 4) as usize).copied().collect(),
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
    )
}

/// exp=s — 3초 후에 런타임 단색 생성(타이밍 격리). 생성되면 노트 갱신.
fn exp_s_probe(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    mut timer: Local<f32>,
    mut fired: Local<bool>,
    mut note: ResMut<ProbeNote>,
    exp: Res<Exp>,
) {
    if exp.0 != 's' || *fired {
        return;
    }
    *timer += 1.0 / 60.0; // 시스템 매 프레임 가동 가정(Δt 대신 고정 증가 — 단순화)
    if *timer < 3.0 {
        return;
    }
    *fired = true;
    let (cx, cy) = mapdata::village_center();
    let (px, py) = mapdata::tile_center(cx, cy);
    let img = images.add(runtime_solid_image(320, [255, 0, 255, 255]));
    commands.spawn((
        Transform::from_xyz(px, -py, 5.0),
        Sprite {
            image: img,
            custom_size: Some(Vec2::splat(400.0)),
            ..default()
        },
    ));
    note.0 = "s:spawned@3s".into();
}

/// exp=v — 3초 후에 스프라이트 생성(에셋 로드는 이미 완료된 상태).
/// u가 안 보이고 v가 보이면 "늦은 핸들 해석" 확정.
fn exp_v_probe(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut timer: Local<f32>,
    mut fired: Local<bool>,
    mut note: ResMut<ProbeNote>,
    exp: Res<Exp>,
) {
    if exp.0 != 'v' || *fired {
        return;
    }
    *timer += 1.0 / 60.0;
    if *timer < 3.0 {
        return;
    }
    *fired = true;
    let (cx, cy) = mapdata::village_center();
    let (px, py) = mapdata::tile_center(cx, cy);
    commands.spawn((
        Transform::from_xyz(px, -py, 5.0),
        Sprite {
            image: assets.load("tiles/iso_grass_0.png"),
            custom_size: Some(Vec2::splat(420.0)),
            ..default()
        },
    ));
    note.0 = "v:spawned@3s".into();
}

/// exp=t — Startup 이미지를 3초 후 재기입(마젠타→초록) — 재업로드 경로 격리.
fn exp_t_rewrite(
    mut images: ResMut<Assets<Image>>,
    mut timer: Local<f32>,
    mut fired: Local<bool>,
    mut note: ResMut<ProbeNote>,
    t_handle: Res<THandle>,
    exp: Res<Exp>,
) {
    if exp.0 != 't' || *fired {
        return;
    }
    *timer += 1.0 / 60.0;
    if *timer < 3.0 {
        return;
    }
    *fired = true;
    if let Some(h) = t_handle.0.clone() {
    if let Some(mut img) = images.get_mut(&h) {
        if let Some(d) = img.data.as_mut() {
            for px in d.chunks_exact_mut(4) {
                px[0] = 0;
                px[1] = 255;
                px[2] = 0;
            }
        }
    }
    }
    note.0 = "t:rewritten@3s".into();
}

/// t 실험 핸들 전달용(간이 전역)
#[derive(Resource, Default)]
pub struct THandle(pub Option<bevy::asset::Handle<Image>>);

fn spawn_exp_probes(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut t_handle: ResMut<THandle>,
    exp: Res<Exp>,
) {
    let (cx, cy) = mapdata::village_center();
    let (px, py) = mapdata::tile_center(cx, cy);
    match exp.0 {
        'e' => {
            commands.spawn((
                Transform::from_xyz(px, -py, 5.0),
                Sprite {
                    color: Color::srgb(1.0, 0.9, 0.1), // 노란 쿼드 — 이미지 핸들 없음
                    custom_size: Some(Vec2::new(420.0, 420.0)),
                    ..default()
                },
            ));
        }
        'h' => {
            // 단일 텍스처 타일 프로브 — 지면·소품·주민 없이 잔디 타일 1장만(거대화)
            commands.spawn((
                Transform::from_xyz(px, -py, 5.0),
                Sprite {
                    image: assets.load("tiles/iso_grass_0.png"),
                    custom_size: Some(Vec2::new(768.0, 384.0)),
                    ..default()
                },
            ));
        }
        'x' => {
            // 크기·POT 스윕 — 네 개 런타임 단색(마젠타)을 가로로:
            // 64²(POT) · 100²(비POT) · 128×64(POT 사각형) · 320²(비POT, 기지 양성 대조)
            let specs: [(f32, u32, u32); 4] = [
                (-560.0, 64, 64),
                (-190.0, 100, 100),
                (190.0, 128, 64),
                (560.0, 320, 320),
            ];
            for (dx, w, h) in specs {
                let img = images.add(Image::new(
                    bevy::render::render_resource::Extent3d {
                        width: w,
                        height: h,
                        depth_or_array_layers: 1,
                    },
                    bevy::render::render_resource::TextureDimension::D2,
                    [255, 0, 255, 255]
                        .iter()
                        .cycle()
                        .take((w * h * 4) as usize)
                        .copied()
                        .collect(),
                    bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                    bevy::asset::RenderAssetUsages::MAIN_WORLD
                        | bevy::asset::RenderAssetUsages::RENDER_WORLD,
                ));
                commands.spawn((
                    Transform::from_xyz(px + dx, -py, 5.0),
                    Sprite {
                        image: img,
                        custom_size: Some(Vec2::new(170.0, 170.0)),
                        ..default()
                    },
                ));
            }
        }
        'u' => {
            // 핵심 실험 — Startup에 스프라이트 생성 + assets.load 핸들(텍스처는 나중에 도착).
            // 월드 스프라이트와 동일한 패턴. 안 보이면 "늦은 핸들 해석"이 범인.
            commands.spawn((
                Transform::from_xyz(px, -py, 5.0),
                Sprite {
                    image: assets.load("tiles/iso_grass_0.png"),
                    custom_size: Some(Vec2::splat(420.0)),
                    ..default()
                },
            ));
        }
        'r' => {
            // 대조군 — Startup에 런타임 단색(320²) 즉시 생성. v2.7 exp=i 조건 재현.
            let img = images.add(runtime_solid_image(320, [255, 0, 255, 255]));
            commands.spawn((
                Transform::from_xyz(px, -py, 5.0),
                Sprite {
                    image: img,
                    custom_size: Some(Vec2::splat(400.0)),
                    ..default()
                },
            ));
        }
        't' => {
            // 업로드 시점 테스트용 — Startup 생성(색: 마젠타), 3초 후 데이터 재기입(초록)은 exp_t_rewrite가 수행
            let img = images.add(runtime_solid_image(320, [255, 0, 255, 255]));
            commands.spawn((
                Transform::from_xyz(px, -py, 5.0),
                Sprite {
                    image: img.clone(),
                    custom_size: Some(Vec2::splat(400.0)),
                    ..default()
                },
            ));
            *t_handle = THandle(Some(img));
        }
        'i' => {
            // 런타임 생성 텍스처 2종 — 왼쪽 마젠타(Rgba8Unorm) / 오른쪽 초록(Rgba8Srgb).
            // PNG 로드 경로를 완전히 배제해 포맷(sRGB) vs 업로드 경로를 가른다.
            let mk = |format, bytes: [u8; 4]| -> Image {
                let size = 320u32;
                let data: Vec<u8> = bytes
                    .iter()
                    .cycle()
                    .take((size * size * 4) as usize)
                    .copied()
                    .collect();
                Image::new(
                    bevy::render::render_resource::Extent3d {
                        width: size,
                        height: size,
                        depth_or_array_layers: 1,
                    },
                    bevy::render::render_resource::TextureDimension::D2,
                    data,
                    format,
                    bevy::asset::RenderAssetUsages::default(),
                )
            };
            let left = images.add(mk(
                bevy::render::render_resource::TextureFormat::Rgba8Unorm,
                [255, 0, 255, 255],
            ));
            let right = images.add(mk(
                bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
                [0, 255, 0, 255],
            ));
            commands.spawn((
                Transform::from_xyz(px - 280.0, -py, 5.0),
                Sprite {
                    image: left,
                    custom_size: Some(Vec2::splat(320.0)),
                    ..default()
                },
            ));
            commands.spawn((
                Transform::from_xyz(px + 280.0, -py, 5.0),
                Sprite {
                    image: right,
                    custom_size: Some(Vec2::splat(320.0)),
                    ..default()
                },
            ));
        }
        'g' => {
            commands.spawn((
                Transform::from_xyz(px, -py + 160.0, 50.0),
                Text2d::new("WORLD-PASS OK"),
                bevy::text::TextFont {
                    font: bevy::text::FontSource::Handle(
                        assets.load("fonts/NotoSansKR-Regular.otf"),
                    ),
                    font_size: bevy::text::FontSize::Px(44.0),
                    ..default()
                },
                TextColor(Color::srgb(1.0, 0.95, 0.2)),
            ));
        }
        _ => {}
    }
}

/// 실험 p — 3중 매트릭스(월드 없음, 어두운 배경 위):
/// 좌=에셋 PNG 그대로 · 우=같은 PNG 데이터의 런타임 재생성 · 하단=128×64 런타임 단색(치수 격리).
/// 로드 완료까지 매 프레임 재시도한다(1프레임 판정 버그 수정 — 2026-10-19).
fn exp_p_probe(
    mut commands: Commands,
    assets: Res<AssetServer>,
    mut images: ResMut<Assets<Image>>,
    mut done: Local<bool>,
    exp: Res<Exp>,
) {
    if exp.0 != 'p' || *done {
        return;
    }
    let handle: Handle<Image> = assets.load("tiles/iso_grass_0.png");
    let Some(img) = images.get(&handle) else {
        return; // 다음 프레임 재시도
    };
    let (cx, cy) = mapdata::village_center();
    let (px, py) = mapdata::tile_center(cx, cy);
    let size = img.texture_descriptor.size;
    let data = img.data.clone().unwrap_or_default();
    let clone = images.add(Image::new(
        size,
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
    ));
    let solid = images.add(Image::new(
        bevy::render::render_resource::Extent3d {
            width: 128,
            height: 64,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        [255, 220, 60, 255]
            .iter()
            .cycle()
            .take(128 * 64 * 4)
            .copied()
            .collect(),
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
    ));
    for (dx, dy, image, sz) in [
        (-360.0, 60.0, handle.clone(), Vec2::new(420.0, 210.0)), // 좌: 에셋
        (360.0, 60.0, clone, Vec2::new(420.0, 210.0)),           // 우: 런타임 복제
        (0.0, -220.0, solid, Vec2::new(420.0, 210.0)),           // 하단: 런타임 단색 128×64
    ] {
        commands.spawn((
            Transform::from_xyz(px + dx, -py + dy, 5.0),
            Sprite {
                image,
                custom_size: Some(sz),
                ..default()
            },
        ));
    }
    *done = true;
}

/// 원격 진단 — 페이지 하단 상태표시줄(DOM) + 탭 제목(1초 주기).
/// 렌더가 죽어도 DOM은 살아있어 실기기 회색 화면 판별용 채널이 된다.
#[cfg(target_arch = "wasm32")]
fn diag_title(
    time: Res<Time>,
    mut acc: Local<f32>,
    exp: Res<Exp>,
    images: Res<Assets<Image>>,
    agents: Query<&Transform, With<agents::Agent>>,
    camera: Query<(&Transform, &Projection), With<Camera2d>>,
    note: Res<ProbeNote>,
) {
    *acc += time.delta_secs();
    if *acc < 1.0 {
        return;
    }
    *acc = 0.0;
    let (cx, cy, scale) = camera
        .single()
        .map(|(t, p)| {
            let s = match p {
                Projection::Orthographic(o) => o.scale,
                _ => 1.0,
            };
            (t.translation.x, t.translation.y, s)
        })
        .unwrap_or((0.0, 0.0, -1.0));
    // 로드된 이미지 포맷 히스토그램 — S=Rgba8Srgb U=Rgba8Unorm R8/RG8=채널 축소(서버 변조 의심)
    let mut hist = std::collections::BTreeMap::new();
    for (_id, im) in images.iter() {
        let f = format!("{:?}", im.texture_descriptor.format);
        let k = if f.contains("Rgba8UnormSrgb") {
            "S"
        } else if f.contains("Rgba8Unorm") {
            "U"
        } else if f.contains("R8") {
            "R8"
        } else if f.contains("Rg8") {
            "RG8"
        } else {
            "?"
        };
        *hist.entry(k).or_insert(0) += 1;
    }
    let hs = hist
        .iter()
        .map(|(k, v)| format!("{k}{v}"))
        .collect::<Vec<_>>()
        .join(" ");
    let msg = format!(
        "v3.4-019 · exp {} · img {} · fmt {hs} · ag {} · cam {cx:.0},{cy:.0} s{scale:.2} · {}",
        exp.0,
        images.len(),
        agents.iter().count(),
        note.0
    );
    if let Some(win) = web_sys::window() {
        if let Some(doc) = win.document() {
            if let Some(el) = doc.get_element_by_id("status") {
                el.set_text_content(Some(&msg));
            }
            use wasm_bindgen::JsCast;
            if let Ok(hd) = doc.dyn_into::<web_sys::HtmlDocument>() {
                hd.set_title(&msg);
            }
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn diag_title() {}
