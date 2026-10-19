//! 카메라 — 드래그 팬·휠 줌·WASD/화살표·선택 주민 추적(수동 조작 시 해제).
use bevy::ecs::message::MessageReader;
use bevy::input::mouse::{AccumulatedMouseMotion, MouseWheel};
use bevy::prelude::*;
use crate::mapdata::*;

#[derive(Resource)]
pub struct DragState {
    pub total: f32,
}

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

/// 터치 제스처 — 한 손가락 드래그 팬 · 두 손가락 핀치 줌 · 탭 선택.
/// TouchInput 메시지 스트림으로 상태 추적(활성 터치 맵).
pub fn touch_gestures(
    mut egui_ctx: bevy_egui::EguiContexts,
    mut camera: Query<(&mut Transform, &mut Projection), With<Camera2d>>,
    mut manual: ResMut<ManualUntil>,
    mut tap: ResMut<TapPos>,
    mut touch_in: MessageReader<bevy::input::touch::TouchInput>,
    mut active: Local<std::collections::HashMap<u64, (Vec2, Vec2, f32)>>, // id → (pos, start, drift)
) {
    let Ok((mut tr, mut proj)) = camera.single_mut() else { return };
    // egui가 포인터를 쓰는 중이면 제스처 스킵(UI 탭이 판으로 세지 않게)
    if crate::ui::egui_owns_input(&mut egui_ctx) {
        return;
    }
    let mut pan = Vec2::ZERO;
    let mut zoom: f32 = 1.0;

    for t in touch_in.read() {
        use bevy::input::touch::TouchPhase::*;
        match t.phase {
            Started => {
                active.insert(t.id, (t.position, t.position, 0.0));
            }
            Moved => {
                let Some((old_pos, start, drift)) = active.get(&t.id).copied() else { continue };
                let d = t.position - old_pos;
                let drift = drift + d.length();
                let n = active.len();
                if n == 1 {
                    pan += Vec2::new(-d.x, d.y); // 그랩 팬(월드 y 반전)
                } else if n == 2 {
                    // 핀치 — 다른 손가락과의 거리 비로 줌
                    let other = active
                        .iter()
                        .find(|(id, _)| **id != t.id)
                        .map(|(_, (p, _, _))| *p);
                    if let Some(op) = other {
                        let old_d = (old_pos - op).length().max(1.0);
                        let new_d = (t.position - op).length().max(1.0);
                        zoom *= old_d / new_d;
                    }
                }
                active.insert(t.id, (t.position, start, drift));
            }
            Ended | Canceled => {
                if let Some((pos, _start, drift)) = active.remove(&t.id) {
                    if drift < 10.0 && active.is_empty() {
                        tap.0 = Some(pos);
                    }
                }
            }
        }
    }

    if pan != Vec2::ZERO || zoom != 1.0 {
        manual.0 = 1_000_000.0; // 터치 조작 중 추적 유예 — 다음 탭 선택 시 즉시 복귀
        let scale = match &mut *proj {
            Projection::Orthographic(o) => {
                if zoom != 1.0 {
                    // zoom = old_d/new_d — 벌리면 <1. 곱해야 벌리기=확대가 된다.
                    // (나누면 벌리기=축소로 반전 — 휠 반전과 같은 결함, 같이 교정)
                    o.scale = (o.scale * zoom).clamp(0.6, 2.8);
                }
                o.scale
            }
            _ => 1.0,
        };
        tr.translation.x += pan.x / scale;
        tr.translation.y += pan.y / scale;
        let (ex, ey) = world_extent();
        tr.translation.x = tr.translation.x.clamp(-ex, ex);
        tr.translation.y = tr.translation.y.clamp(-ey, 320.0);
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
            o.scale = (o.scale * (1.0 - e.y * 0.0015)).clamp(0.6, 2.8);
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

    // 드래그 팬 — 그랩 방향(화면 dx→월드 −x, dy→월드 +y), 줌 보상
    let scale = match &*proj {
        Projection::Orthographic(o) => o.scale,
        _ => 1.0,
    };
    if buttons.pressed(MouseButton::Left) {
        drag.total += mouse.delta.length();
        if drag.total > 5.0 {
            v += Vec2::new(-mouse.delta.x, mouse.delta.y) / scale;
        }
    } else {
        drag.total = 0.0;
    }

    if v != Vec2::ZERO {
        manual.0 = time.elapsed_secs() + 1.5;
        let speed = if buttons.pressed(MouseButton::Left) { 1.0 } else { 620.0 };
        tr.translation.x += v.x * speed * dt;
        tr.translation.y += v.y * speed * dt;
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
