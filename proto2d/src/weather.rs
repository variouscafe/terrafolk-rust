//! 날씨 파티클 — 비(빗줄기)·폭설(눈송이). 서버 weather 필드 구동.
//! 파티클은 카메라 자식(뷰포트 국소 좌표) — 팬해도 화면을 벗어나지 않는다.

use bevy::prelude::*;

/// 현재 날씨(서버 정본 — sync_daynight가 기록)
#[derive(Resource, Default, PartialEq, Eq, Clone, Copy)]
pub enum WeatherKind {
    #[default]
    Clear,
    Rain,
    Storm,
    Snow,
}

impl WeatherKind {
    pub fn from_server_pub(w: &str) -> Self {
        match w {
            "rain" => WeatherKind::Rain,
            "storm" => WeatherKind::Storm,
            "snow" => WeatherKind::Snow,
            _ => WeatherKind::Clear,
        }
    }
}

#[derive(Component)]
pub(crate) struct WeatherParticle {
    /// 종류별 속도 계수(0..1 랜덤)
    pub k: f32,
    /// 기준 x(눈 흔들림 오프셋의 원점 — 누적 표류 방지)
    pub base_x: f32,
}

#[derive(Component)]
pub(crate) struct WeatherLayer;

const RAIN_N: usize = 110;
const SNOW_N: usize = 90;

/// 카메라에 날씨 레이어 부착(부재→출현 감시, 지연 카메라 포함)
pub fn attach(
    mut commands: Commands,
    cameras: Query<(Entity, &Projection), (With<Camera2d>, Without<WeatherLayerAttached>)>,
) {
    for (e, _) in &cameras {
        commands
            .entity(e)
            .insert(WeatherLayerAttached)
            .with_child((WeatherLayer, Transform::default()));
    }
}

#[derive(Component)]
pub(crate) struct WeatherLayerAttached;

/// 날씨 전환 — 파티클 재구성(맑으면 철거). 카메라 자식 레이어에 스폰.
pub fn sync(
    weather: Res<WeatherKind>,
    mut commands: Commands,
    layers: Query<(Entity, &Children), With<WeatherLayer>>,
    particles: Query<(Entity, &WeatherParticle)>,
    mut current: Local<WeatherKind>,
    mut inited: Local<bool>,
) {
    if !weather.is_changed() && *inited && *current == *weather {
        return;
    }
    *inited = true;
    if *current == *weather {
        return;
    }
    // 기존 파티클 철거
    for (e, _) in &particles {
        commands.entity(e).despawn();
    }
    // 레이어(카메라)가 아직 없으면 *current 커밋하지 않는다 — 다음 프레임 재시행
    // (지연 카메라 스폰 레이스 방지)
    let Ok((layer, _)) = layers.single() else {
        return;
    };
    *current = *weather;
    let mut rng = mapdata_rng();
    match *weather {
        WeatherKind::Clear => {}
        WeatherKind::Rain | WeatherKind::Storm => {
            let n = if *weather == WeatherKind::Storm { RAIN_N + 60 } else { RAIN_N };
            for _ in 0..n {
                let x = rng.next_f32() * 1700.0 - 850.0;
                let y = rng.next_f32() * 1200.0 - 600.0;
                let k = 0.7 + rng.next_f32() * 0.6;
                commands.entity(layer).with_child((
                    WeatherParticle { k, base_x: x },
                    // 빗줄기 — 가늘고 긴 사각형, 바람 기울임
                    Transform::from_xyz(x, y, 600.0)
                        .with_rotation(Quat::from_rotation_z(0.18)),
                    Sprite {
                        color: Color::srgba(0.62, 0.72, 0.85, 0.34),
                        custom_size: Some(Vec2::new(2.2, 16.0)),
                        ..default()
                    },
                ));
            }
        }
        WeatherKind::Snow => {
            for _ in 0..SNOW_N {
                let x = rng.next_f32() * 1700.0 - 850.0;
                let y = rng.next_f32() * 1200.0 - 600.0;
                let k = 0.5 + rng.next_f32() * 0.8;
                commands.entity(layer).with_child((
                    WeatherParticle { k, base_x: x },
                    Transform::from_xyz(x, y, 600.0),
                    Sprite {
                        color: Color::srgba(1.0, 1.0, 1.0, 0.85),
                        custom_size: Some(Vec2::new(5.0, 5.0)),
                        ..default()
                    },
                ));
            }
        }
    }
}

/// 파티클 낙하 — 비는 빠르게·눈은 흔들리며. 카메라 국소 y -600 아래로 가면 위 랩.
pub fn fall(time: Res<Time>, weather: Res<WeatherKind>, mut q: Query<(&WeatherParticle, &mut Transform)>) {
    if *weather == WeatherKind::Clear {
        return;
    }
    let t = time.elapsed_secs();
    let (speed, sway) = match *weather {
        WeatherKind::Snow => (110.0, 46.0),
        _ => (950.0, 0.0), // 비·폭설(비)
    };
    for (p, mut tr) in &mut q {
        tr.translation.y -= speed * p.k * time.delta_secs();
        if sway > 0.0 {
            // 기준 x + 사인 오프셋 — 흔들려도 밴드 안 유지(누적 표류 없음)
            tr.translation.x = p.base_x + (t * 1.3 + p.k * 9.0).sin() * 40.0;
        }
        if tr.translation.y < -650.0 {
            tr.translation.y = 650.0;
            tr.translation.x = t_x(t, p.k);
        }
    }
}

fn t_x(t: f32, k: f32) -> f32 {
    // 재배치 x — 결정론 난수 대신 위상 해시 근사(간단)
    (t * 7.13 + k * 211.7).sin() * 850.0
}

/// 간단 xorshift — mapdata 의존 없이 자체 보유
fn mapdata_rng() -> crate::mapdata::XorShift {
    crate::mapdata::XorShift::new(0x5EA7E2)
}
