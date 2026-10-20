//! ui2 — bevy_ui 웹 타당성 프로브(`?ui2=1`): egui → bevy_ui 전면 교체 검토용.
//! 대화창 카드의 대표 해부(패널·한글 Pretendard·칩 행·컴포저·버튼 클릭)를
//! bevy_ui로 그려 렌더·인터랙션·wasm 사이즈를 실측한다. egui와 동시 기동
//! 가능(별도 렌더 계층) — URL 스위치로만 등장한다.

use bevy::prelude::*;
use bevy::text::{FontSource, FontSize};

pub struct Ui2Plugin;

impl Plugin for Ui2Plugin {
    fn build(&self, app: &mut App) {
        if !enabled() {
            return;
        }
        app.add_systems(Update, spawn_probe)
            .add_observer(on_click)
            .add_observer(on_hover);
        info!("[ui2] bevy_ui 프로브 활성");
    }
}

fn enabled() -> bool {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::JsCast;
        web_sys::window()
            .and_then(|w| w.location().search().ok())
            .map(|s| s.contains("ui2=1"))
            .unwrap_or(false)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        std::env::var("TF_UI2").is_ok()
    }
}

/// 클릭 토글 마커 — 버튼 본문 텍스트를 갈아끼운다
#[derive(Component)]
struct ProbeToggle;

/// 클릭 로그 — 상태 줄에 클릭 횟수 표시(스크린샷 판정용)
#[derive(Component)]
struct ProbeCount(u32);

fn spawn_probe(
    mut commands: Commands,
    mut fonts: ResMut<Assets<Font>>,
    assets: Res<AssetServer>,
    cameras: Query<Entity, With<Camera2d>>,
    mut done: Local<bool>,
) {
    if *done {
        return;
    }
    // UI 노드는 TargetCamera 명시 바인딩 없으면 렌더 스킵(0.19) —
    // 카메라 엔티티를 찾아 루트에 물린다
    let Ok(cam) = cameras.single() else {
        return;
    };
    *done = true;
    info!("[ui2] 프로브 스폰 — 카메라 {:?} 바인딩", cam);
    // 런타임 에셋 폰트 — egui와 동일 파일 재사용(wasm 사이즈 영향 없음)
    let font: Handle<Font> = assets.load("fonts/Pretendard-Regular.otf");
    // 첫 프레임 로드 대기 없이 그리기 — 로드 전엔 빈 텍스트(무해)
    let _ = fonts.get_mut(&font);

    let ink = Color::srgb(0.96, 0.89, 0.75); // 크림 #F4E3BE
    let dim = Color::srgb(0.66, 0.72, 0.62); // 세이지
    let glass = Color::srgba(0.12, 0.16, 0.13, 0.94);
    let pill = Color::srgba(0.07, 0.09, 0.08, 0.78);
    let gold = Color::srgb(0.91, 0.72, 0.25);

    commands
        .spawn((
            Name::new("ui2_root"),
            ProbeCard,
            bevy::ui::UiTargetCamera(cam),
            Node {
                position_type: PositionType::Absolute,
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                flex_direction: FlexDirection::Column,
                justify_content: JustifyContent::FlexEnd,
                align_items: AlignItems::Center,
                padding: UiRect::bottom(Val::Px(160.0)),
                ..default()
            },
        ))
        .with_children(|root| {
            // ── 대화창 카드 — 플랫 글래스(스트로크 없음, 라운드 18) ──
            root.spawn((
                Node {
                    width: Val::Px(560.0),
                    flex_direction: FlexDirection::Column,
                    row_gap: Val::Px(6.0),
                    padding: UiRect::all(Val::Px(12.0)),
                    border_radius: BorderRadius::all(Val::Px(18.0)),
                    ..default()
                },
                BackgroundColor(glass),
                BoxShadow::new(
                    Color::srgba(0.0, 0.0, 0.0, 0.35),
                    Val::Px(0.0),
                    Val::Px(8.0),
                    Val::Px(0.0),
                    Val::Px(24.0),
                ),
            ))
            .with_children(|card| {
                // 헤더 — 아바타 점 + 이름 + 상태 칩 + ✕
                card.spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(8.0),
                        align_items: AlignItems::Center,
                        ..default()
                    },
                ))
                .with_children(|h| {
                    h.spawn((
                        Node {
                            width: Val::Px(26.0),
                            height: Val::Px(26.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border_radius: BorderRadius::all(Val::Px(13.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgb(0.93, 0.63, 0.75)),
                    ))
                    .with_children(|a| {
                        a.spawn((
                            Text::new("바"),
                            TextFont { font_size: FontSize::Px(12.0), font: FontSource::Handle(font.clone()), ..default() },
                            TextColor(Color::srgb(0.16, 0.13, 0.09)),
                        ));
                    });
                    h.spawn((
                        Text::new("바위"),
                        TextFont { font_size: FontSize::Px(13.5), font: FontSource::Handle(font.clone()), ..default() },
                        TextColor(ink),
                    ));
                    h.spawn((
                        Node {
                            padding: UiRect::axes(Val::Px(8.0), Val::Px(2.0)),
                            border_radius: BorderRadius::all(Val::Px(999.0)),
                            ..default()
                        },
                        BackgroundColor(Color::srgba(1.0, 1.0, 1.0, 0.055)),
                    ))
                    .with_children(|c| {
                        c.spawn((
                            Text::new("행동"),
                            TextFont { font_size: FontSize::Px(10.5), font: FontSource::Handle(font.clone()), ..default() },
                            TextColor(gold),
                        ));
                    });
                });
                // 로그 3행
                for line in [
                    ("D43 13:04", "spectator", "hi", gold),
                    ("D43 12:31", "바위", "물을 마셨다 — 모두 남음", ink),
                    ("D43 12:02", "바위", "아침을 챙겨 먹었다", ink),
                ] {
                    card.spawn((
                        Node {
                            flex_direction: FlexDirection::Row,
                            column_gap: Val::Px(6.0),
                            ..default()
                        },
                    ))
                    .with_children(|r| {
                        r.spawn((
                            Text::new(line.0),
                            TextFont { font_size: FontSize::Px(9.5), font: FontSource::Handle(font.clone()), ..default() },
                            TextColor(dim),
                        ));
                        r.spawn((
                            Text::new(format!("{}:", line.1)),
                            TextFont { font_size: FontSize::Px(11.0), font: FontSource::Handle(font.clone()), ..default() },
                            TextColor(line.3),
                        ));
                        r.spawn((
                            Text::new(line.2),
                            TextFont { font_size: FontSize::Px(11.0), font: FontSource::Handle(font.clone()), ..default() },
                            TextColor(ink),
                        ));
                    });
                }
                // ── 컴포저 — 보더리스 필 + 플랫 골드 버튼 ──
                card.spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        column_gap: Val::Px(6.0),
                        align_items: AlignItems::Center,
                        padding: UiRect::axes(Val::Px(6.0), Val::Px(3.0)),
                        border_radius: BorderRadius::all(Val::Px(19.0)),
                        ..default()
                    },
                    BackgroundColor(pill),
                ))
                .with_children(|p| {
                    p.spawn((
                        Text::new("바위에게 말걸기…"),
                        TextFont { font_size: FontSize::Px(12.0), font: FontSource::Handle(font.clone()), ..default() },
                        TextColor(dim),
                    ));
                    p.spawn((
                        Node {
                            flex_grow: 1.0,
                            ..default()
                        },
                    ));
                    p.spawn((
                        ProbeToggle,
                        Node {
                            width: Val::Px(30.0),
                            height: Val::Px(30.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            border_radius: BorderRadius::all(Val::Px(15.0)),
                            ..default()
                        },
                        BackgroundColor(gold),
                    ))
                    .with_children(|b| {
                        b.spawn((
                            ProbeCount(0),
                            Text::new("→"),
                            TextFont { font_size: FontSize::Px(15.0), font: FontSource::Handle(font.clone()), ..default() },
                            TextColor(Color::srgb(0.23, 0.19, 0.09)),
                        ));
                    });
                });
                // 상태 줄 — 클릭 판정용(스크린샷 판독)
                card.spawn((
                    Node::default(),
                ))
                .with_children(|s| {
                    s.spawn((
                        Text::new("ui2 ready — 클릭 전"),
                        TextFont { font_size: FontSize::Px(10.0), font: FontSource::Handle(font.clone()), ..default() },
                        TextColor(dim),
                    ));
                });
            });
        });
}

/// 카드 호버 판별 — 피킹 히트테스트 생존 확인(클릭 없어도 마우스 오버시 1회)
#[derive(Component)]
struct ProbeCard;

fn on_hover(
    _hover: On<Pointer<Over>>,
    mut q: Query<&mut Text, With<ProbeCount>>,
) {
    if let Ok(mut text) = q.single_mut() {
        **text = "hover!".into();
    }
    info!("[ui2] 카드 hover — 피킹 히트테스트 생존");
}

/// 송신 버튼 클릭 — 카운터 증가로 인터랙션 실측(웹 포인터 이벤트 경로)
fn on_click(
    click: On<Pointer<Click>>,
    mut q: Query<(&mut ProbeCount, &mut Text), With<ProbeToggle>>,
) {
    if let Ok((mut count, mut text)) = q.single_mut() {
        count.0 += 1;
        **text = format!("→ {}", count.0);
        info!("[ui2] 버튼 클릭 #{}", count.0);
    }
    let _ = click; // 카드 어디를 눌러도 버튼이 아닌 곳은 무시 — 버튼만 옵저버
}
