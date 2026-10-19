//! 소품 — 업스케일 프롭을 GID에 맞춰 산포 + 마을 클러스터(오두막·모닥불·무덤).
//! 모든 프롭은 발밑 그림자 블롭 자식으로 정착감 부여(레퍼런스 룩).
use bevy::prelude::*;
use bevy::sprite::Anchor;
use crate::mapdata::*;

/// y-sort 기준 z(화면 아래쪽 = 큰 z). 지면은 z=0.
pub fn z_of(py: f32) -> f32 {
    10.0 + py * 0.001
}

#[derive(Component)]
pub struct Campfire {
    pub frames: [Handle<Image>; 3],
}

/// 데모 산포 소품 태그 — 라이브 프라임 시 일괄 철거용
#[derive(Component)]
pub struct DemoProp;

pub fn spawn(mut commands: Commands, assets: Res<AssetServer>) {
    let shadow: Handle<Image> = assets.load("shadow.png");
    let mut rng = XorShift::new(2026_1019 ^ 0xBEEF);
    let (mw, mh) = dims();
    for y in 0..mh {
        for x in 0..mw {
            let (path, prob, sw): (&str, f32, f32) = match gid(x, y) {
                3 => ("props/tree_broadleaf.png", 0.085, 120.0), // forest
                5 => ("props/boulder.png", 0.06, 80.0),           // rock
                2 => ("props/berry_bush.png", 0.012, 56.0),       // grass
                _ => continue,
            };
            if rng.next_f32() >= prob {
                continue;
            }
            let e = spawn_prop(&mut commands, &shadow, assets.load(path), x, y, sw);
            commands.entity(e).insert(DemoProp);
        }
    }

    // ── 마을: 중심에 오두막 3·모닥불·무덤 2·저장고 ──
    let (cx, cy) = village_center();
    for (dx, dy, path, sw) in [
        (-3i32, -2i32, "props/hut.png", 190.0),
        (2, -2, "props/hut.png", 190.0),
        (-1, 3, "props/hut.png", 190.0),
        (4, 1, "props/grave.png", 56.0),
        (-4, 2, "props/grave.png", 56.0),
        (1, -4, "props/stash.png", 56.0),
    ] {
        let e = spawn_prop(&mut commands, &shadow, assets.load(path), cx + dx, cy + dy, sw);
        commands.entity(e).insert(DemoProp);
    }

    // 모닥불(3프레임 애님)
    let frames = [
        assets.load("props/campfire_0.png"),
        assets.load("props/campfire_1.png"),
        assets.load("props/campfire_2.png"),
    ];
    let (px, py) = tile_center(cx, cy);
    commands
        .spawn((
            DemoProp,
            Campfire {
                frames: frames.clone(),
            },
            Transform::from_xyz(px, -py - TH * 0.25, z_of(py)),
            Anchor::BOTTOM_CENTER,
            Sprite {
                image: frames[0].clone(),
                ..default()
            },
        ))
        .with_child((
            Transform::from_xyz(0.0, 6.0, -0.2),
            Sprite {
                image: shadow,
                custom_size: Some(Vec2::new(58.0, 22.0)),
                color: Color::srgba(0.02, 0.015, 0.0, 0.38),
                ..default()
            },
        ));
}

pub fn spawn_prop(
    commands: &mut Commands,
    shadow: &Handle<Image>,
    image: Handle<Image>,
    x: i32,
    y: i32,
    shadow_w: f32,
) -> Entity {
    spawn_prop_ex(commands, shadow, image, x as f32, y as f32, shadow_w, None, 1.0, Color::WHITE)
}

/// 소품 스폰 확장 — 그리드 연속 좌표·월드 크기 오버라이드·스케일·틴트.
/// 라이브 자원(건축물 custom_size·미완공 회색조·성장 나무 축소)이 쓴다.
pub fn spawn_prop_ex(
    commands: &mut Commands,
    shadow: &Handle<Image>,
    image: Handle<Image>,
    gx: f32,
    gy: f32,
    shadow_w: f32,
    custom: Option<Vec2>,
    scale: f32,
    color: Color,
) -> Entity {
    let (px, py) = project(gx, gy);
    commands
        .spawn((
            Transform {
                translation: Vec3::new(px, -py - TH * 0.25, z_of(py)),
                scale: Vec3::splat(scale),
                ..default()
            },
            Anchor::BOTTOM_CENTER,
            Sprite {
                image,
                color,
                custom_size: custom,
                ..default()
            },
        ))
        .with_child((
            Transform::from_xyz(0.0, 4.0, -0.2),
            Sprite {
                image: shadow.clone(),
                custom_size: Some(Vec2::new(shadow_w, shadow_w * 0.38)),
                color: Color::srgba(0.02, 0.015, 0.0, 0.38),
                ..default()
            },
        ))
        .id()
}

/// 모닥불 애님 — 0.15초/프레임 순환
pub fn animate_campfire(time: Res<Time>, mut q: Query<(&Campfire, &mut Sprite)>) {
    let t = time.elapsed_secs();
    for (c, mut s) in &mut q {
        let f = (t / 0.15) as usize % 3;
        s.image = c.frames[f].clone();
    }
}
