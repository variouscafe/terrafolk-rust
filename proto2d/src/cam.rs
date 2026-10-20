//! 카메라 — 드래그 팬·휠 줌·WASD/화살표·선택 주민 추적(수동 조작 시 해제).
use bevy::ecs::message::MessageReader;
use bevy::input::mouse::{AccumulatedMouseMotion, MouseWheel};
use bevy::prelude::*;
use crate::mapdata::*;

#[derive(Resource)]
pub struct DragState {
    pub total: f32,
}

/// 줌 한도(직교 scale — 작을수록 확대). 상한 4.5는 1024px 뷰포트에서
/// 지도 x폭의 ~27%가 보이는 조망(2026-10-20 상향 2.8→4.5).
const ZOOM_MIN: f32 = 0.6;
const ZOOM_MAX: f32 = 4.5;

#[derive(Resource)]
pub struct ManualUntil(pub f32);

/// 터치 탭(짧고 정지한 터치) 화면좌표 — 주민 선택 트리거
#[derive(Resource, Default)]
pub struct TapPos(pub Option<Vec2>);

/// 지연 카메라(exp=n) — 실기기 가설 "첫 렌더 프레임 이후 텍스처 업로드는 실패" 검증.
/// 모든 이미지 로드 + 1초 안정 후 카메라 스폰 → 첫 프레임에 전부 업로드.
#[derive(Resource)]
pub struct DelayedCamera {
    pub spawned: bool,
    pub last_count: usize,
    pub stable_since: f32,
}

/// 추적 중 터치 — 위치·누적 이동·이동했는가(핀치 자격)·UI 위에서 시작했는가
#[derive(Clone, Copy)]
pub(crate) struct TouchTrack {
    pos: Vec2,
    drift: f32,
    moved: bool,
    ui: bool,
}

/// 터치 제스처 — 한 손가락 드래그 팬 · 두 손가락 핀치 줌 · 탭 선택.
/// egui가 터치를 무조건 점유로 보고(터치 중 egui_wants_pointer_input)
/// 옛 코드가 점유 중 이벤트를 안 읽어 GC로 Ended가 유실 → 유령 손가락이
/// 남고 이후 1손 드래그가 유령과의 핀치로 계산돼 "드래그하면 줌되는"
/// 사고가 났다(2026-10-20 실사용기). 3원칙으로 재작성:
/// ① 이벤트는 항상 읽는다(점유·프레임 드랍과 무관 — 유령 원천 차단).
/// ② UI 여부는 egui_owns_input(포인터 배타 주장)이 아니라 시작 위치의
///    히트 테스트로 판정(패널·창 위=UI, 중앙 월드=제스처).
/// ③ 핀치는 두 손가락이 모두 실제로 움직였을 때만, 프레임 단위 거리 비.
pub fn touch_gestures(
    mut egui_ctx: bevy_egui::EguiContexts,
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
    mut manual: ResMut<ManualUntil>,
    mut tap: ResMut<TapPos>,
    mut touch_in: MessageReader<bevy::input::touch::TouchInput>,
    mut active: Local<std::collections::HashMap<u64, TouchTrack>>,
    mut pinch_prev: Local<Option<f32>>,
) {
    let mut over_ui = |pos: bevy_egui::egui::Pos2| -> bool {
        let Some(c) = egui_ctx.ctx_mut().ok().map(|c| c.clone()) else {
            return false; // 컨텍스트 없으면 월드로 취급(게임이 우선)
        };
        use bevy_egui::egui::Order;
        match c.layer_id_at(pos) {
            // 창·메뉴·토스트 계층 위 — UI
            Some(l) if l.order != Order::Background => true,
            // 배경 계층 — 도킹 패널 영역이면 UI, 중앙(패널 제외)이면 월드.
            // available_rect가 정확히 '패널 뺀 중앙'(deprecated 표기는 이동
            // 권고일 뿐 — 대안 content_rect는 안전영역이라 의미가 다르다)
            Some(_) => {
                #[expect(deprecated)]
                !c.available_rect().contains(pos)
            }
            None => false,
        }
    };
    let mut pan = Vec2::ZERO;
    for t in touch_in.read() {
        use bevy::input::touch::TouchPhase::*;
        match t.phase {
            Started => {
                let ui = over_ui(bevy_egui::egui::pos2(t.position.x, t.position.y));
                active.insert(
                    t.id,
                    TouchTrack { pos: t.position, drift: 0.0, moved: false, ui },
                );
            }
            Moved => {
                let Some(tr) = active.get_mut(&t.id) else { continue };
                let d = t.position - tr.pos;
                tr.pos = t.position;
                tr.drift += d.length();
                tr.moved = tr.moved || d.length() > 0.25;
                // 월드 1손 = 라이브 팬(즉시 적용)
                let world_solo = !tr.ui && active.values().filter(|x| !x.ui).count() == 1;
                if world_solo {
                    pan += Vec2::new(-d.x, d.y); // 그랩 팬(월드 y 반전)
                }
            }
            Ended | Canceled => {
                if let Some(tr) = active.remove(&t.id)
                    && !tr.ui
                    && tr.drift < 10.0
                    && active.values().all(|x| x.ui)
                {
                    tap.0 = Some(tr.pos);
                }
            }
        }
    }
    let Ok((mut tr_, mut proj)) = camera.single_mut() else { return };
    // 핀치 — 월드 손가락 2개·둘 다 움직임. 프레임 거리 비(이중 계산 없음)
    let mut zoom: f32 = 1.0;
    let world: Vec<&TouchTrack> = active.values().filter(|x| !x.ui).collect();
    if world.len() == 2 && world.iter().all(|t| t.moved) {
        let d = (world[0].pos - world[1].pos).length().max(1.0);
        if let Some(prev) = *pinch_prev {
            // prev/d — 벌리면 d>prev → 비<1. 곱해야 벌리기=확대
            zoom = (prev / d).clamp(0.8, 1.25);
        }
        *pinch_prev = Some(d);
    } else {
        *pinch_prev = None;
    }

    if pan != Vec2::ZERO || zoom != 1.0 {
        manual.0 = 1_000_000.0; // 터치 조작 중 추적 유예 — 다음 탭 선택 시 즉시 복귀
        let scale = match &mut *proj {
            Projection::Orthographic(o) => {
                if zoom != 1.0 {
                    o.scale = (o.scale * zoom).clamp(ZOOM_MIN, ZOOM_MAX);
                }
                o.scale
            }
            _ => 1.0,
        };
        // 그랩 팬 — 화면 px당 월드 단위는 scale에 비례(축소=scale↑=넓게
        // 보임)하므로 곱셈 보상. 나눗셈이면 축소할수록 손가락보다 느려진다.
        tr_.translation.x += pan.x * scale;
        tr_.translation.y += pan.y * scale;
        let (ex, ey) = world_extent();
        tr_.translation.x = tr_.translation.x.clamp(-ex, ex);
        tr_.translation.y = tr_.translation.y.clamp(-ey, 320.0);
    }
}

pub fn spawn_camera(mut commands: Commands, exp: Res<crate::Exp>) {
    if exp.0 == 'n' {
        return; // 지연 모드 — delayed_camera가 스폰
    }
    let (cx, cy) = village_center();
    let (px, py) = tile_center(cx, cy);
    let _ = exp; // 실험 c/d MSAA는 0.19에서 리소스 방식 — 다운그레이드 빌드에선 무효(원인 아님 판명됨)
    // 레마스터 룩 — 기본 줌 인(스프라이트를 크게)
    commands.spawn((Camera2d, Transform::from_xyz(px, -py, 1000.0)));
}

/// exp=n — 이미지 40장 이상 + 1초 안정 후 카메라 스폰(첫 프레임 일괄 업로드)
pub fn delayed_camera(
    mut commands: Commands,
    time: Res<Time>,
    exp: Res<crate::Exp>,
    images: Res<Assets<Image>>,
    mut st: ResMut<DelayedCamera>,
) {
    if st.spawned || exp.0 != 'n' {
        return;
    }
    let n = images.len();
    let now = time.elapsed_secs();
    if n < 40 {
        return;
    }
    if n != st.last_count {
        st.last_count = n;
        st.stable_since = now;
        return;
    }
    if now - st.stable_since < 1.0 {
        return;
    }
    st.spawned = true;
    let (cx, cy) = village_center();
    let (px, py) = tile_center(cx, cy);
    commands.spawn((Camera2d, Transform::from_xyz(px, -py, 1000.0)));
}

pub fn pan_zoom(
    time: Res<Time>,
    mut egui_ctx: bevy_egui::EguiContexts,
    mut zoom_inited: Local<bool>,
    keys: Res<ButtonInput<KeyCode>>,
    buttons: Res<ButtonInput<MouseButton>>,
    mouse: Res<AccumulatedMouseMotion>,
    mut wheel: MessageReader<MouseWheel>,
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
    mut drag: ResMut<DragState>,
    mut manual: ResMut<ManualUntil>,
) {
    let Ok((mut tr, mut proj)) = camera.single_mut() else { return };
    let dt = time.delta_secs();

    // 레마스터 룩 — 최초 1회 기본 줌 인(스프라이트 크게)
    if !*zoom_inited {
        *zoom_inited = true;
        if let Projection::Orthographic(ref mut o) = *proj {
            o.scale = 0.8;
        }
    }

    // 줌 — 휠 (scale 작을수록 확대). winit 웹은 DOM deltaY를 반전해 네이티브
    // 관례(휠 업=+y)로 정규화하므로 부호를 minus로 — 휠 업=확대가 표준 UX.
    // (2026-10-20 실측: plus였을 때 휠 업=축소로 반전 동작)
    for e in wheel.read() {
        if let Projection::Orthographic(ref mut o) = *proj {
            o.scale = (o.scale * (1.0 - e.y * 0.0015)).clamp(ZOOM_MIN, ZOOM_MAX);
        }
    }

    // egui 키보드 점유(채팅 입력 등) 중엔 WASD 판 금지
    if crate::ui::egui_owns_input(&mut egui_ctx) {
        return;
    }
    let mut v = Vec2::ZERO;
    if keys.any_pressed([KeyCode::KeyW, KeyCode::ArrowUp]) {
        v.y += 1.0;
    }
    if keys.any_pressed([KeyCode::KeyS, KeyCode::ArrowDown]) {
        v.y -= 1.0;
    }
    if keys.any_pressed([KeyCode::KeyA, KeyCode::ArrowLeft]) {
        v.x -= 1.0;
    }
    if keys.any_pressed([KeyCode::KeyD, KeyCode::ArrowRight]) {
        v.x += 1.0;
    }

    // 드래그 팬 — 그랩 방향(화면 dx→월드 −x, dy→월드 +y). 줌 보상은
    // 곱셈(축소=scale↑=px당 월드↑ — 1:1 그랩)이고 dt를 곱하지 않는다:
    // mouse.delta는 이미 이번 프레임 누적분이라 dt를 곱하면 프레임레이트
    // 배만 느려진다(구현 결함 — 축소 시 특히 체감, 2026-10-20 수정).
    let scale = match &*proj {
        Projection::Orthographic(o) => o.scale,
        _ => 1.0,
    };
    let mut drag_v = Vec2::ZERO;
    if buttons.pressed(MouseButton::Left) {
        drag.total += mouse.delta.length();
        if drag.total > 5.0 {
            drag_v = Vec2::new(-mouse.delta.x, mouse.delta.y) * scale;
        }
    } else {
        drag.total = 0.0;
    }

    if v != Vec2::ZERO || drag_v != Vec2::ZERO {
        manual.0 = time.elapsed_secs() + 1.5;
        // 키보드는 속도×dt, 드래그는 화면 이동량 그대로(서로 별개 경로)
        tr.translation.x += v.x * 620.0 * dt + drag_v.x;
        tr.translation.y += v.y * 620.0 * dt + drag_v.y;
    }

    // 아이소 월드 경계 클램프(y는 위쪽이 −py → [−여유, 최대+여유])
    let (ex, ey) = world_extent();
    tr.translation.x = tr.translation.x.clamp(-ex, ex);
    tr.translation.y = tr.translation.y.clamp(-ey, 320.0);
}

/// 선택 주민 추적 — 수동 조작 직후 1.5초 유예 후 재개
pub fn follow_selected(
    time: Res<Time>,
    manual: Res<ManualUntil>,
    selected: Query<&Transform, With<crate::agents::Selected>>,
    mut camera: Query<&mut Transform, (With<Camera2d>, Without<crate::agents::Selected>)>,
) {
    if time.elapsed_secs() < manual.0 {
        return;
    }
    let Some(st) = selected.iter().next() else { return };
    let Ok(mut tr) = camera.single_mut() else { return };
    let target = st.translation.truncate() + Vec2::new(0.0, 60.0);
    tr.translation.x += (target.x - tr.translation.x) * 0.06;
    tr.translation.y += (target.y - tr.translation.y) * 0.06;
}
