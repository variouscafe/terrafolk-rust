//! 야간 주민 가시성 — 발밑 미광(2026-10-21 백로그). 밤에도 실루엣 유지:
//! 전역 앰비언트를 올려 밤 무드를 씻는 대신, 주민 발밑에만 은은한 난로빛을
//! 둔다(관전 몰입 — 북극성). 그림자 없음·짧은 범위·night_factor 비례 제곱
//(낮잠광 억제). 데모·라이브 스폰 경로와 무관하게 Villager 마커로 멱등 부착.

use bevy::prelude::*;

/// 루트(주민)측 마커 — 미광 자식 부착 완료 표시.
#[derive(Component)]
struct GlowMark;

/// 미광 라이트 자식측 마커(쿼리 대상).
#[derive(Component)]
struct NightFootGlow;

fn ensure_glow(
    mut commands: Commands,
    q: Query<Entity, (With<super::villager::Villager>, Without<GlowMark>)>,
) {
    for e in &q {
        let light = commands
            .spawn((
                NightFootGlow,
                PointLight {
                    color: Color::srgb(1.0, 0.78, 0.52),
                    intensity: 0.0,
                    range: 3.2,
                    radius: 0.5,
                    shadow_maps_enabled: false,
                    ..default()
                },
                Transform::from_translation(Vec3::new(0.0, 0.45, 0.0)),
            ))
            .id();
        commands.entity(e).insert(GlowMark).add_child(light);
    }
}

/// 밤 요인만큼 미광 램프 — 낮(0)엔 완전 소등.
fn update_glow(
    day: Res<super::daynight::DayTime>,
    mut q: Query<&mut PointLight, With<NightFootGlow>>,
) {
    let n = day.night_factor.clamp(0.0, 1.0);
    for mut l in &mut q {
        l.intensity = n * n * 14_000.0;
    }
}

pub fn plugin(app: &mut App) {
    app.add_systems(Update, (ensure_glow, update_glow).chain());
}
