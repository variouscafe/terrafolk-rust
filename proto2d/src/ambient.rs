//! 앰비언트 연출 — 바람 나무 흔들림·밤 반딧불·구름 그림자.
//! 전부 런타임(신규 에셋 없음) — 관전 세계의 '살아있음'을 위한 저비용 레이어.

use bevy::prelude::*;

use crate::mapdata::{self, project};

/// 바람 흔들림 대상(나무류) — 위상은 위치 해시로 결정(집단 파도 방지)
#[derive(Component)]
pub struct Sway {
    pub phase: f32,
}

pub fn insert_sway(x: i32, y: i32) -> Sway {
    let h = (x as u32).wrapping_mul(374761393) ^ (y as u32).wrapping_mul(668265263);
    Sway {
        phase: ((h >> 8) & 0xFF) as f32 / 255.0 * std::f32::consts::TAU,
    }
}

/// 나무 흔들림 — 미세 회전(±0.022rad)·주기 ~3.4s. Anchor가 발밑이라
/// 아랫단 고정 상단만 흔들린다.
pub fn sway_trees(time: Res<Time>, mut q: Query<(&Sway, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (s, mut tr) in &mut q {
        let w = (t * 1.85 + s.phase).sin();
        tr.rotation = Quat::from_rotation_z(w * 0.022);
    }
}

// ── 반딧불 ───────────────────────────────────────────────

#[derive(Component)]
pub(crate) struct Firefly {
    seed: f32,
    home: (f32, f32),
}

/// 반딧불 스폰 — 마을 주변 24마리(밤에만 표시, 색상 알파로 제어)
pub fn spawn_fireflies(mut commands: Commands) {
    let mut rng = mapdata::XorShift::new(0xF1EF1E);
    let (cx, cy) = mapdata::village_center();
    for i in 0..24 {
        let gx = cx as f32 + rng.range_i32(-30, 30) as f32 + rng.next_f32();
        let gy = cy as f32 + rng.range_i32(-30, 30) as f32 + rng.next_f32();
        let (px, py) = project(gx, gy);
        commands.spawn((
            Firefly {
                seed: (i * 97) as f32,
                home: (px, py),
            },
            Transform::from_xyz(px, -py, 30.0),
            Sprite {
                color: Color::srgba(1.0, 0.93, 0.45, 0.0), // 밤에만 뜸
                custom_size: Some(Vec2::new(7.0, 7.0)),
                ..default()
            },
        ));
    }
}

/// 반딧불 갱신 — 밤 인자(0=낮, 1=밤)로 페이드·느린 부유·점멸.
pub fn update_fireflies(
    time: Res<Time>,
    gt: Res<crate::daynight::GameTime>,
    mut q: Query<(&Firefly, &mut Transform, &mut Sprite)>,
) {
    // 밤 인자 — t 0.75→1→0→0.25 구간
    let night = if gt.t > 0.75 || gt.t < 0.25 {
        1.0
    } else if gt.t > 0.70 {
        (gt.t - 0.70) / 0.05 // 해질 들며 서서히
    } else if gt.t < 0.30 {
        1.0 - (gt.t - 0.25) / 0.05 // 새벽 밝아지며 소등
    } else {
        0.0
    };
    let t = time.elapsed_secs();
    for (f, mut tr, mut s) in &mut q {
        let drift = (t * 0.35 + f.seed).sin();
        let drift2 = (t * 0.23 + f.seed * 1.7).cos();
        tr.translation.x = f.home.0 + drift * 90.0;
        tr.translation.y = -f.home.1 + drift2 * 55.0 + (t * 0.9 + f.seed).sin() * 8.0;
        // 점멸 — 1.1s 주기 개별 위상
        let blink = ((t * 5.7 + f.seed * 3.1).sin() * 0.5 + 0.5).powi(2);
        let a = night * (0.25 + 0.75 * blink);
        s.color = Color::srgba(1.0, 0.93, 0.45, a * 0.9);
        // 크기도 숨 쉬듯
        let sz = 6.0 + blink * 3.5 * night;
        s.custom_size = Some(Vec2::new(sz, sz));
    }
}

// ── 구름 그림자 ──────────────────────────────────────────

#[derive(Component)]
pub(crate) struct CloudShadow {
    pub speed: Vec2,
}

/// 런타임 부드러운 원형 그라디언트 텍스처 — 구름 그림자용(반경 256)
fn soft_blob_texture(images: &mut Assets<Image>) -> Handle<Image> {
    let size = 256u32;
    let mut data = Vec::with_capacity((size * size * 4) as usize);
    for y in 0..size {
        for x in 0..size {
            let dx = (x as f32 + 0.5 - size as f32 / 2.0) / (size as f32 / 2.0);
            let dy = (y as f32 + 0.5 - size as f32 / 2.0) / (size as f32 / 2.0);
            let d = (dx * dx + dy * dy).sqrt().min(1.0);
            // 부드러운 falloff — 타원 뭉게 3겹 합성
            let a = (1.0 - d).powi(2) * 0.55 + (1.0 - d.min(0.7) / 0.7).powi(3) * 0.45;
            data.extend_from_slice(&[10, 16, 8, (a * 255.0) as u8]); // 짙은 녹색-검정
        }
    }
    images.add(Image::new(
        bevy::render::render_resource::Extent3d {
            width: size,
            height: size,
            depth_or_array_layers: 1,
        },
        bevy::render::render_resource::TextureDimension::D2,
        data,
        bevy::render::render_resource::TextureFormat::Rgba8UnormSrgb,
        bevy::asset::RenderAssetUsages::MAIN_WORLD | bevy::asset::RenderAssetUsages::RENDER_WORLD,
    ))
}

/// 구름 그림자 3장 — 맵을 가로질러 느리게 표류(화면 밖 랩)
pub fn spawn_clouds(mut commands: Commands, mut images: ResMut<Assets<Image>>) {
    let tex = soft_blob_texture(&mut images);
    let (mw, _mh) = mapdata::dims();
    let span = mw as f32 * mapdata::TW * 0.5; // 대각 폭 근사
    let mut rng = mapdata::XorShift::new(0xC10D);
    for i in 0..3 {
        let gx = rng.next_f32() * span * 2.0 - span;
        let gy = rng.next_f32() * span;
        let (px, py) = project(gx, gy);
        commands.spawn((
            CloudShadow {
                speed: Vec2::new(14.0 + i as f32 * 5.0, 5.0),
            },
            // 그림자는 지면 위·소품 아래(z 음수 — 지면 z≈0보다 아래 두면 안 가려짐,
            // 대신 지면보다 위·소품보다 아래 미세 z로 얹는다: 4.0 + py 보정)
            Transform::from_xyz(px, -py, 4.0 + py * 0.001),
            Sprite {
                image: tex.clone(),
                color: Color::srgba(1.0, 1.0, 1.0, 0.5), // 텍스처 알파와 곱
                custom_size: Some(Vec2::new(1400.0, 900.0)),
                ..default()
            },
        ));
    }
}

/// 구름 표류 — 경계 넘으면 반대편 랩. y-sort z는 유지(그림자라 소품 아래).
pub fn drift_clouds(time: Res<Time>, mut q: Query<(&CloudShadow, &mut Transform)>) {
    let (mw, mh) = mapdata::dims();
    let ex = mw as f32 * mapdata::TW * 0.5 + 1600.0;
    let ey = mh as f32 * mapdata::TH + 1600.0;
    for (c, mut tr) in &mut q {
        tr.translation.x += c.speed.x * time.delta_secs();
        tr.translation.y -= c.speed.y * time.delta_secs();
        if tr.translation.x > ex {
            tr.translation.x = -ex;
        }
        if tr.translation.y < -ey {
            tr.translation.y = ey;
        }
    }
}
