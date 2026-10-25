//! 카메라 리그 — 원근(Perspective) 고정 절대각 뷰(2026-10-23 전환).
//! 참조 스타일(코지 페인티드 3/4): 피치 35°·요 0°(맵 변이 화면축과 정렬 —
//! 마름모 아이소 잔상 제거, 사각 채움 구도). 줌=거리(D ∝ zoom, 타깃 지면
//! 프레이밍은 구 정사영 VIEW_H·zoom와 동일) — 줌투커서 축척비 공식도 타깃면
//! 기준으로 그대로 유효. 팬: 광선 그랩(원근에서도 1:1) + WASD + 관성.

use bevy::core_pipeline::tonemapping::DebandDither;
use bevy::input::gestures::PinchGesture;
use bevy::input::mouse::AccumulatedMouseScroll;
use bevy::prelude::*;
use bevy::render::view::ColorGrading;

use super::camera_input::{PointerInput, ground_at};
use super::data::WorldData;

/// 줌 범위 — scale = VIEW_H × zoom(최대 줌인 ≈9m 시야 ↔ 최대 줌아웃 ≈126m 시야).
/// 6.0 = 글로브 완성(globe.rs ZOOM_FULL) — 줌아웃 끝에서 행성 관전.
pub const ZOOM_MIN: f32 = 0.3;
pub const ZOOM_MAX: f32 = 6.0;
/// 스무딩 계수(초당 수렴 속도)
const ZOOM_SMOOTH: f32 = 9.0;
/// 정사영 기본 시야 높이(월드 단위) — 줌 1에서 세로로 보이는 폭.
/// 참조 구도(마을 스케일 — 코티지 4.2m가 화면 세로의 ~20%).
pub const VIEW_H: f32 = 22.0;
/// 초상 화면 초기 줌 — 마을 스케일 근접(바다 과다 완화)
const PORTRAIT_INIT_ZOOM: f32 = 0.62;
/// ⚠ far(420)은 terrain_splat.wgsl 안개 축 상수(140·뷰방향)보다 넉넉 — 정합 유지.
/// 부저각 — 요 0°(맵 변 정렬·사각 채움)·피치 35°(내려다봄, 참조 3/4 뷰).
const CAM_YAW_DEG: f32 = 0.0;
pub const CAM_PITCH_DEG: f32 = 35.0;
/// 원근 FOV(세로) — 40°는 코지 참조의 온화한 원근감(광각 왜곡 없음).
const FOV_DEG: f32 = 40.0;
/// 줌→카메라 거리 — 타깃 지면에서 세로 시야폭이 구 정사영(VIEW_H·zoom)과
/// 같아지는 거리. 이 매핑 덕에 줌 체감·미니맵 view_box·줌투커서 축척비
/// 공식이 전부 그대로 유효하다.
pub fn cam_dist(zoom: f32) -> f32 {
    (VIEW_H * zoom * 0.5) / (FOV_DEG.to_radians() * 0.5).tan()
}

/// 휠 줌 — 한 스텝(노치)당 배율 기수(1.65^Δ)·Δ 게인
const WHEEL_ZOOM_BASE: f32 = 1.65;
const WHEEL_ZOOM_GAIN: f32 = 1.0;
/// 트랙패드 핀치 제스처 감도(macOS·iOS — 프레임당 스케일에 곱함)
const PINCH_GESTURE_GAIN: f32 = 5.0;
/// 모바일 두 손가락 핀치 — 프레임당 줌 비 클램프(상용 체감 폭)
pub const PINCH_FRAME_MIN: f32 = 0.55;
pub const PINCH_FRAME_MAX: f32 = 1.8;
/// 키보드 +/- 줌 속도(초당 배율 변화율)
const KEY_ZOOM_RATE: f32 = 1.4;
/// 키보드 팬 속도(초·줌 1 기준 월드 단위)
const KEY_PAN_SPEED: f32 = 16.0;

#[derive(Resource)]
pub struct CameraRig {
    pub target: Vec2,
    pub zoom: f32,      // 현재(렌더에 적용)
    pub zoom_goal: f32, // 목표(입력 누적)
    /// 초상(세로) 화면 초기 프레이밍 적용 여부 — 회전 전 창 크기가 가로로
    /// 먼저 보고되므로 첫 세로 해상도 확정 시 1회만 조정한다.
    portrait_framed: bool,
}

impl Default for CameraRig {
    fn default() -> Self {
        CameraRig {
            target: Vec2::splat(super::data::SIZE as f32 / 2.0),
            zoom: 1.0,
            zoom_goal: 1.0,
            portrait_framed: false,
        }
    }
}

/// 원근 투영 — 세로 FOV 고정, 줌은 거리(cam_dist)로.
fn perspective_projection() -> Projection {
    Projection::Perspective(PerspectiveProjection {
        fov: FOV_DEG.to_radians(),
        near: 0.5,
        far: 420.0,
        ..PerspectiveProjection::default()
    })
}

/// 뷰 방향 반대편 오프셋 — 요·피치 구면, 거리는 줌 비례.
fn view_offset(zoom: f32) -> Vec3 {
    let (p, y) = (CAM_PITCH_DEG.to_radians(), CAM_YAW_DEG.to_radians());
    let d = cam_dist(zoom);
    let horiz = d * p.cos();
    Vec3::new(horiz * y.cos(), d * p.sin(), horiz * y.sin())
}

/// 카메라 포즈(위치·시선점·업) — 곡률 반영. factor<ε(평면)에서는 기존
/// pan_offset+look_at 수식과 동일(줌인 픽셀 동일 게이트). 곡률 구간은 평면
/// 앵커를 워프한 국소 접면에 카메라를 띄운다(거리는 cam_dist→행성 궤도로
/// 보간 — factor→0에서 연속).
fn camera_pose(
    rig: &CameraRig,
    focus: &super::look_edit::LookFocus,
    world: &super::data::WorldData,
) -> (Vec3, Vec3, Vec3) {
    let target = Vec3::new(rig.target.x, 0.0, rig.target.y);
    // 편집 미리보기(look_edit) — 카메라 앵커를 -z(먼 쪽)로 옮겨 타깃을 화면
    // 위쪽에 고정(하단 시트·우측 패널과 공존). 0.24 = 화면 상단 ~30% 지점.
    let mut anchor = target;
    if focus.active {
        anchor.z -= VIEW_H * rig.zoom * 0.24;
    }
    match super::globe::WarpFrame::active(world, rig.zoom) {
        None => (anchor + view_offset(rig.zoom), target, Vec3::Y),
        Some(frame) => {
            let anchor_w = frame.warp_point(anchor);
            let q = frame.warp_quat(anchor.xz());
            let dir = q * view_offset(rig.zoom).normalize();
            let d_flat = cam_dist(rig.zoom);
            let d = d_flat
                + (frame.r_planet * super::globe::ORBIT_DIST - d_flat) * frame.factor;
            (anchor_w + dir * d, anchor_w, q * Vec3::Y)
        }
    }
}

pub fn spawn_camera(mut commands: Commands, world: Res<super::data::WorldData>, mut rig: ResMut<CameraRig>) {
    // 디버그 부팅 게이트 — 곡률 최대(행성 뷰)로 시작(E2E·실물 검증).
    #[cfg(target_arch = "wasm32")]
    let globe_boot = crate::net::dbg_gate::on("globe");
    #[cfg(not(target_arch = "wasm32"))]
    let globe_boot = std::env::var("TF_GLOBE").is_ok();
    if globe_boot {
        rig.zoom = ZOOM_MAX;
        rig.zoom_goal = ZOOM_MAX;
    }
    let (pos, look, up) = camera_pose(&rig, &super::look_edit::LookFocus::default(), &world);
    let mut tr = Transform::from_translation(pos);
    tr.look_at(look, up);
    commands.spawn((
        Camera3d::default(),
        perspective_projection(),
        tr,
        // 후처리 — DebandDither(하늘 밴딩 제거)만. Bloom은 2026-10-19 아트
        // 방향(플랫·클린 렌더)에서 제거 — 그랩패스 절감 겸용.
        DebandDither::Enabled,
        ColorGrading::default(),
    ));
}

/// 회전은 항상 동일(부저각 고정) — 위치만 옮긴 가상 카메라 변환.
/// 줌 스무딩 직후·팬 적용 전 좌표계로 그랩/앵커 광선을 계산한다.
fn camera_at(gt: &GlobalTransform, pos: Vec3) -> GlobalTransform {
    GlobalTransform::from_translation(pos - gt.translation()) * *gt
}

#[allow(clippy::too_many_arguments)] // Bevy 시스템 파라미터 — 입력·카메라·월드를 한 프레임 안에서 순서 보정
pub fn camera_control(
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mouse: Res<ButtonInput<bevy::input::mouse::MouseButton>>,
    wheel: Res<AccumulatedMouseScroll>,
    mut pinch: MessageReader<PinchGesture>,
    touches: Res<Touches>,
    window: Single<&Window>,
    mut rig: ResMut<CameraRig>,
    focus: Res<super::look_edit::LookFocus>,
    mut input: ResMut<PointerInput>,
    world: Res<WorldData>,
    ui_hover: Res<crate::platform::UiHover>,
    mut q: Query<(&Camera, &GlobalTransform, &mut Transform, &mut Projection), With<Camera3d>>,
) {
    let Ok((camera, gt, mut tr, mut proj)) = q.single_mut() else {
        return;
    };
    let dt = time.delta_secs().max(1e-4);
    let cursor = window.cursor_position();

    // UI 점유 중이면 신규 입력 양보 — 진행 중 제스처(월드에서 시작)는 유지.
    // 스무딩·클램프·변환 적용은 항상 돈다(멈추면 줌 수렴·경계가 얼어붙는다).
    let ui_block = ui_hover.0 && !input.gesture_in_progress();

    // ── 1) 터치 제스처 전이 + 두 손가락 핀치 비(줌 목표 반영 전에 확정)
    let pinch_zoom = if ui_block {
        1.0
    } else {
        input.begin_frame(&mouse, &touches)
    };

    // ── 2) 줌 입력 누적 — UI 점유 시 신규 입력 양보
    let typing = super::pick::ui_typing();
    if !ui_block {
        // 휠 — 상용 체감 배율(노치당 ~1.65배). 커서가 있으면 줌투커서 앵커 지정
        if wheel.delta.y.abs() > 1e-4 {
            let d = (wheel.delta.y * WHEEL_ZOOM_GAIN).clamp(-4.0, 4.0);
            rig.zoom_goal *= WHEEL_ZOOM_BASE.powf(-d);
            input.zoom_anchor = cursor;
        }
        // 트랙패드 핀치 제스처(macOS/iOS) — 감도 ×5.0, 앵커는 커서
        let mut gestured = false;
        for g in pinch.read() {
            rig.zoom_goal *= 1.0 / (1.0 + (g.0 * PINCH_GESTURE_GAIN).clamp(-0.9, 0.9));
            gestured = true;
        }
        if gestured {
            input.zoom_anchor = cursor;
        }
        // 모바일 두 손가락 핀치 비(begin_frame에서 전이 판정 완료)
        rig.zoom_goal *= pinch_zoom;
        // 키보드 +/- — 뷰 중심 줌(앵커 없음). 채팅 입력 타이핑 중에는 무시
        // (winit keydown은 window 스코프 — DOM 입력 키도 여기 온다)
        if !typing && (keys.pressed(KeyCode::Equal) || keys.pressed(KeyCode::NumpadAdd)) {
            rig.zoom_goal *= 1.0 - KEY_ZOOM_RATE * dt;
        }
        if !typing && (keys.pressed(KeyCode::Minus) || keys.pressed(KeyCode::NumpadSubtract)) {
            rig.zoom_goal *= 1.0 + KEY_ZOOM_RATE * dt;
        }
    }

    // ── 3) 목표 클램프 → 지수 스무딩
    rig.zoom_goal = rig.zoom_goal.clamp(ZOOM_MIN, ZOOM_MAX);
    let k = 1.0 - (-(ZOOM_SMOOTH * dt)).exp();
    let zoom_prev = rig.zoom;
    rig.zoom += (rig.zoom_goal - rig.zoom) * k;
    let zoom_moved = (rig.zoom - zoom_prev).abs() > 1e-5;

    // ── 4) 이번 프레임 적용될 카메라(줌 반영·팬 적용 전) — 그랩 광선 기준.
    //    그랩 팬 델타는 카메라 평행이동 불변이므로 팬 전 좌표로 계산해도 정확하다.
    let gt_next = camera_at(gt, camera_pose(&rig, &focus, &world).0);
    // 곡률 프레임 — 지면 광선 판정(그랩·핀치·줌 앵커)이 구면 교차로 전환.
    let gframe = super::globe::WarpFrame::active(&world, rig.zoom);

    // ── 5) 그랩 팬(마우스·한 손가락·핀치 중심) + 관성 — UI 점유 시 양보
    let frame = if ui_block {
        super::camera_input::PanFrame {
            pan: Vec2::ZERO,
            gesture_active: false,
        }
    } else {
        input.apply_pans(camera, &gt_next, &world, &mouse, &touches, cursor, dt, gframe.as_ref())
    };
    rig.target += frame.pan;

    // ── 6) 키보드 팬(WASD·화살표) — 줌 배율 반영 속도. 타이핑 중 무시(위 참조)
    let mut dir = Vec2::ZERO;
    if !typing && (keys.pressed(KeyCode::KeyW) || keys.pressed(KeyCode::ArrowUp)) {
        dir.y -= 1.0;
    }
    if !typing && (keys.pressed(KeyCode::KeyS) || keys.pressed(KeyCode::ArrowDown)) {
        dir.y += 1.0;
    }
    if !typing && (keys.pressed(KeyCode::KeyA) || keys.pressed(KeyCode::ArrowLeft)) {
        dir.x -= 1.0;
    }
    if !typing && (keys.pressed(KeyCode::KeyD) || keys.pressed(KeyCode::ArrowRight)) {
        dir.x += 1.0;
    }
    if dir != Vec2::ZERO {
        let sp = KEY_PAN_SPEED * rig.zoom * dt;
        rig.target += dir.normalize() * sp;
    }

    // ── 7) 줌투커서(정사영) — 능동 제스처가 없을 때만.
    //    정사영은 화면 오프셋 ∝ (세계점−뷰중심)/scale 이므로, 앵커 a₀를 커서 아래
    //    유지하려면 target += (a₀−target)·(1−zoom/zoom_prev). 앵커 아래 지형
    //    고도(2단 계산)로 잡아 언덕 위에서도 정확하다.
    if zoom_moved && !frame.gesture_active {
        if let Some(px) = input.zoom_anchor {
            let anchor = ground_at(camera, gt, px, 0.0, gframe.as_ref()).and_then(|g| {
                let h = world.height_at(g.x.round() as i32, g.z.round() as i32);
                ground_at(camera, gt, px, h, gframe.as_ref()).or(Some(g))
            });
            if let Some(a0) = anchor {
                let ratio = 1.0 - rig.zoom / zoom_prev.max(1e-4);
                let target = rig.target;
                rig.target += (Vec2::new(a0.x, a0.z) - target) * ratio;
            }
        }
    } else if !zoom_moved {
        input.zoom_anchor = None; // 수렴 완료 — 앵커 해제
    }

    // ── 8) 월드 경계 클램프 — 벽에 박으면 관성 즉시 정지(미끄러짐 없음)
    let bound = (world.size as f32 - 2.0).max(2.0);
    let unclamped = rig.target;
    rig.target = rig.target.clamp(Vec2::splat(2.0), Vec2::splat(bound));
    if rig.target != unclamped {
        input.stop_inertia();
    }

    // ── 9) 적용 — 위치·방향·투영
    // 초상(세로) 화면 초기 프레이밍 — 35° 정사영은 화면 22m가 지면 ~38m가
    // 되어 섬 전체+원해가 담겨 '바다가 과다'하게 보인다(2026-10-17 실기 보고).
    // 세로 화면은 마을 스케일 근접(0.62)로 시작 — 첫 세로 해상도에 1회.
    if !rig.portrait_framed && window.height() > window.width() {
        rig.portrait_framed = true;
        rig.zoom = PORTRAIT_INIT_ZOOM;
        rig.zoom_goal = PORTRAIT_INIT_ZOOM;
    }
    let (pos, look, up) = camera_pose(&rig, &focus, &world);
    tr.translation = pos;
    tr.look_at(look, up);
    // far — 곡률 구간에서 라이브 256맵 궤도 거리 대응해 넓힌다. 평면=420
    // 불변(terrain_splat.wgsl 안개 축 상수와 정합 유지).
    let want_far = 420.0 + 280.0 * super::globe::factor_for(rig.zoom);
    if let Projection::Perspective(ref mut p) = *proj {
        if p.far != want_far {
            p.far = want_far;
        }
    }
}
