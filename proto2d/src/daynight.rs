//! 주야 — 하루 150초. 풀스크린 UI 노드 틴트(야간 짙푸른·새벽/해질 warm).
use bevy::prelude::*;

#[derive(Resource)]
pub struct GameTime {
    pub day: u32,
    pub t: f32, // 0.0=자정 … 0.5=정오
}

pub const DAY_SECS: f32 = 150.0;

#[derive(Component)]
pub struct NightTint;

/// 진단 스위치(2026-10-19): 실기기에서 "UI는 그려지는데 월드 스프라이트만 안 보임"
/// 증상의 두 용의자를 가르기 위해 틴트 노드를 끈다. 월드가 보이면 틴트 노드가
/// 범인(불투명 렌더) → 카메라 자식 스프라이트로 재구현할 것.
const TINT_ENABLED: bool = false;

pub fn spawn(mut commands: Commands) {
    if !TINT_ENABLED {
        return;
    }
    commands.spawn((
        NightTint,
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            ..default()
        },
        BackgroundColor(Color::NONE),
    ));
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

pub fn advance(
    time: Res<Time>,
    net: Option<Res<crate::net::NetState>>,
    mut gt: ResMut<GameTime>,
    mut q: Query<&mut BackgroundColor, With<NightTint>>,
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
    let (a, [r, g, b]) = tint(gt.t);
    if let Ok(mut c) = q.single_mut() {
        c.0 = if a <= 0.001 {
            Color::NONE
        } else {
            Color::srgba(r, g, b, a)
        };
    }
}
