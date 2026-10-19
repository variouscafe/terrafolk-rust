//! 야간 주민 가시성 — 발밑 은은한 웜 미광(밤 인자로만 페이드).
//! (백로그: 야간 주민 실루엣 개선 — 2026-10-20 룩 3차에서 미광 경로 채택)

use bevy::prelude::*;
use bevy::sprite::Anchor;

use crate::agents::Agent;
use crate::daynight::GameTime;

#[derive(Component)]
pub struct NightGlow;

/// 주민마다 발밑 미광 자식 스폰(지연 스폰 주민 포함 — 부재→출현 감시)
pub fn attach(
    mut commands: Commands,
    mut images: ResMut<Assets<Image>>,
    agents: Query<(Entity, &Transform), (With<Agent>, Without<NightGlowParent>)>,
) {
    if agents.is_empty() {
        return;
    }
    let tex = images.add(radial_texture());
    for (e, _) in &agents {
        commands.entity(e).insert(NightGlowParent).with_child((
            NightGlow,
            Transform::from_xyz(0.0, 6.0, -0.1), // 그림자 자매·발밑
            Anchor::BOTTOM_CENTER,
            Sprite {
                image: tex.clone(),
                color: Color::srgba(1.0, 0.82, 0.5, 0.0), // 밤에만
                custom_size: Some(Vec2::new(150.0, 80.0)),
                ..default()
            },
        ));
    }
}

/// 밤 인자(반딧불과 동일 곡선) — 미광 강도
fn night_factor(t: f32) -> f32 {
    if t > 0.75 || t < 0.25 {
        1.0
    } else if t > 0.70 {
        (t - 0.70) / 0.05
    } else if t < 0.30 {
        1.0 - (t - 0.25) / 0.05
    } else {
        0.0
    }
}

/// 미광 갱신 — 밤에만 표시(은은한 웜, 그림자 위)
pub fn update(gt: Res<GameTime>, mut q: Query<&mut Sprite, With<NightGlow>>) {
    let n = night_factor(gt.t);
    for mut s in &mut q {
        s.color = Color::srgba(1.0, 0.82, 0.5, n * 0.30);
    }
}

fn radial_texture() -> Image {
    let size = 64u32;
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let dx = (x as f32 + 0.5 - size as f32 / 2.0) / (size as f32 / 2.0);
            let dy = (y as f32 + 0.5 - size as f32 / 2.0) / (size as f32 / 2.0);
            let d = (dx * dx + dy * dy).sqrt().min(1.0);
            let a = (1.0 - d).powi(2);
            data.extend_from_slice(&[255, 220, 150, (a * 255.0) as u8]);
        }
    }
    Image::new(
        bevy::render::render_resource::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
    )
}

/// 부착 마커(중복 부착 방지)
#[derive(Component)]
pub struct NightGlowParent;
