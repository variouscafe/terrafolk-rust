//! 주야 — 게임 시각 자원 + 풀스크린 그레이딩(카메라 자식 틴트 쿼드).
//! 데모는 하루 150초 로컬 진행, 라이브는 서버 정본(sync_daynight).
//! egui 전환으로 bevy_ui 틴트 노드를 걷고(2026-10-20) 카메라 자식 스프라이트로
//! 이관 — 월드 위·egui 아래에 항상 뷰포트를 덮는다.

use bevy::prelude::*;

#[derive(Resource)]
pub struct GameTime {
    pub day: u32,
    pub t: f32, // 0.0=자정 … 0.5=정오
}

pub const DAY_SECS: f32 = 150.0;

/// 카메라에 틴트 쿼드가 붙었는지(부착 마커)
#[derive(Component)]
pub struct TintedCamera;

/// 풀스크린 틴트 쿼드 — 카메라 자식(스폰은 attach_tint가 지연 카메라 포함 처리)
#[derive(Component)]
pub struct NightTint;

/// 카메라 부재→출현을 감시해 틴트 쿼드 부착(자식이라 뷰포트 추종).
pub fn attach_tint(
    mut commands: Commands,
    cameras: Query<(Entity, &Projection), (With<Camera2d>, Without<TintedCamera>)>,
) {
    for (e, _) in &cameras {
        commands
            .entity(e)
            .insert(TintedCamera)
            .with_child((
                NightTint,
                // 카메라 로컬 z — 월드 최상단보다 위(소품 z_of ~10..20)
                Transform::from_xyz(0.0, 0.0, 500.0),
                Sprite {
                    color: Color::NONE,
                    custom_size: Some(Vec2::splat(20000.0)), // 뷰포트 전 덮기(줌아웃 2.8 대비)
                    ..default()
                },
            ));
    }
}

/// 시각 → 틴트 색 갱신 — 낮(0.3~0.7)=은은한 웜, 새벽/해질=웜, 밤=딥블루.
pub fn update_tint(gt: Res<GameTime>, mut q: Query<&mut Sprite, With<NightTint>>) {
    if !gt.is_changed() {
        return;
    }
    let (a, [r, g, b]) = tint(gt.t);
    for mut s in &mut q {
        s.color = Color::srgba(r, g, b, a);
    }
}

pub fn advance(
    time: Res<Time>,
    net: Option<Res<crate::net::NetState>>,
    mut gt: ResMut<GameTime>,
) {
    // 라이브 접속 중은 서버가 시간 정본 — 로컬 진행 정지(net/apply::sync_daynight)
    if let Some(net) = net
        && net.status == crate::net::NetStatus::Connected
    {
        return;
    }
    gt.t += time.delta_secs() / DAY_SECS;
    if gt.t >= 1.0 {
        gt.t -= 1.0;
        gt.day += 1;
    }
}

fn lerp(a: f32, b: f32, t: f32) -> f32 {
    a + (b - a) * t
}

/// 하루 시각 → (알파, RGB). 낮(0.3~0.7)=무색, 새벽/해질=웜, 밤=딥블루.
fn tint(t: f32) -> (f32, [f32; 3]) {
    let night = (0.52, [0.045, 0.075, 0.20]);
    let warm = (0.17, [0.85, 0.45, 0.16]);
    let (a, c) = if t < 0.20 {
        night
    } else if t < 0.30 {
        // 새벽: 밤 → 웜 → 소등
        let k = (t - 0.20) / 0.10;
        let k2 = (k - 0.5).max(0.0) * 2.0; // 후반부 페이드아웃
        (
            lerp(night.0, warm.0 * (1.0 - k2), k.min(1.0)),
            [
                lerp(night.1[0], warm.1[0], k.min(1.0)),
                lerp(night.1[1], warm.1[1], k.min(1.0)),
                lerp(night.1[2], warm.1[2], k.min(1.0)),
            ],
        )
    } else if t < 0.70 {
        // 낮 — 상시 은은한 웜 캐스트(레퍼런스 톤)
        (0.045, [1.0, 0.93, 0.80])
    } else if t < 0.80 {
        // 해질: 웜 → 밤
        let k = (t - 0.70) / 0.10;
        (
            lerp(warm.0, night.0, k),
            [
                lerp(warm.1[0], night.1[0], k),
                lerp(warm.1[1], night.1[1], k),
                lerp(warm.1[2], night.1[2], k),
            ],
        )
    } else {
        night
    };
    (a, c)
}
