//! 수중 동물(§17.43) — 어류·가재·민물조개의 절차 물고기 렌더. 라이브
//! 물 타일 자원은 지형 합성에서 스킵되던 것(어류 서식)을 수면 유영으로
//! 되살린다. GLB가 없어 몸·꼬리를 스케일 구(Sphere)로 합성 — 핸드페인트
//! 팔레트(레퍼런스 웜톤 계열의 어종색 7종).
//!
//! 이동은 두 경로: 정착 좌표(프레임 행 x/y)와 이벤트 좌표 갱신(§17.43 —
//! worldsim 'event' 페이로드 {id,x,y}가 sink 프레임에 편승) → sync가
//! RoamTo로 등속 보간(live_res_move 재사용 — 이동 방향 바라보기 포함).
//! 물은 불투명(AlphaMode::Opaque)이라 수면 위 등 부분만 보인다 —
//! 코이 연못 하향 시점의 '수면 유영' 실루엣.

use bevy::math::primitives::Sphere;
use bevy::prelude::*;

use crate::net::resources::{res_sig, LiveResMeta, LiveResourceTag};

/// 수면 y — water.rs 글로벌 평면(-0.18)과 동일 계약.
pub const SURFACE_Y: f32 = -0.18;
/// 몸 중심 오프셋 — 등이 수면 위로 살짝 나오는 높이.
const FLOAT_Y: f32 = SURFACE_Y + 0.05;

/// 어종 스펙 — 종별 몸길이·기저색·유영 여부. 색은 sRGB.
struct FishSpec {
    len: f32,
    color: Color,
    swim: bool,
}

fn spec(species: &str) -> FishSpec {
    let (len, rgb, swim) = match species {
        // 피라미 — 은청
        "pale_chub" => (0.50, [0.56, 0.71, 0.77], true),
        // 쉬리 — 올리브
        "swiri" => (0.55, [0.49, 0.60, 0.43], true),
        // 메기 — 진회갈(대형)
        "catfish" => (1.00, [0.35, 0.33, 0.29], true),
        // 잉어 — 금등
        "carp" => (0.85, [0.79, 0.54, 0.29], true),
        // 붕어 — 은녹
        "crucian_carp" => (0.70, [0.58, 0.66, 0.56], true),
        // 가재 — 적갈 소형
        "crayfish" => (0.35, [0.54, 0.29, 0.23], true),
        // 민물조개 — 모래빛(부유 않음)
        _ => (0.25, [0.76, 0.70, 0.58], false),
    };
    FishSpec {
        len,
        color: Color::srgb(rgb[0], rgb[1], rgb[2]),
        swim,
    }
}

/// 물고기 루트 마커 — 상하 보브·위상. LiveResourceTag와 함께 붙는다.
#[derive(Component)]
pub struct FishRoot {
    phase: f32,
    swim: bool,
}

/// 꼬리 피벗 — 좌우 스위시 회전 대상.
#[derive(Component)]
pub struct FishTail {
    phase: f32,
}

/// 라이브 수중 동물 스폰 — 루트(위치·태그) + 몸(타원 구) + 꼬리(피벗·편구).
/// 죽은 것(잡힌 개체 alive=false)은 호출처(resources.rs)가 스킵한다.
pub fn spawn_live_fish(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    std_mats: &mut Assets<bevy::pbr::StandardMaterial>,
    r: &crate::net::LiveResource,
) {
    let sp = spec(&r.species);
    let phase = (r.id % 360) as f32 / 360.0 * std::f32::consts::TAU;
    let sig = res_sig(r, true);
    let root = commands
        .spawn((
            FishRoot {
                phase,
                swim: sp.swim,
            },
            LiveResourceTag(r.id),
            LiveResMeta(sig),
            Transform::from_translation(Vec3::new(
                r.x as f32 + 0.5,
                FLOAT_Y,
                r.y as f32 + 0.5,
            ))
            .with_rotation(Quat::from_rotation_y(phase)),
        ))
        .id();
    // 몸 — 장축 Z(진행 방향), 납작한 등선. 몸길이=지름.
    let body = meshes.add(Sphere::new(0.5));
    let mat = std_mats.add(bevy::pbr::StandardMaterial {
        base_color: sp.color,
        perceptual_roughness: 0.85,
        ..default()
    });
    commands.entity(root).with_children(|p| {
        p.spawn((
            Mesh3d(body),
            MeshMaterial3d(mat),
            Transform::from_scale(Vec3::new(sp.len * 0.30, sp.len * 0.45, sp.len)),
        ));
        if sp.swim {
            let tail = meshes.add(Sphere::new(0.5));
            let fin_mat = std_mats.add(bevy::pbr::StandardMaterial {
                base_color: sp.color,
                perceptual_roughness: 0.85,
                ..default()
            });
            p.spawn((
                FishTail { phase },
                Transform::from_translation(Vec3::new(0.0, 0.0, -sp.len * 0.55)),
            ))
            .with_children(|tp| {
                tp.spawn((
                    Mesh3d(tail),
                    MeshMaterial3d(fin_mat),
                    // 꼬리 지느러미 — 얇고 세로로 낸 편구(피벗 뒤로 오프셋)
                    Transform::from_translation(Vec3::new(0.0, 0.0, -sp.len * 0.18))
                        .with_scale(Vec3::new(sp.len * 0.06, sp.len * 0.42, sp.len * 0.40)),
                ));
            });
        }
    });
}

/// 유영 갱신 — 루트 보브·꼬리 스위시. 이동 자체는 live_res_move(RoamTo 등속
/// + 진행 방향 바라보기)가 담당 — 여기선 자세만. 루트와 꼬리가 같은
/// Transform을 쓰므로 한 시스템에 두 쿼리를 두면 B0001 파라미터 충돌로
/// 스케줄 초기화가 죽는다(2026-11-04 실측 — 시스템 분리로 해결).
pub fn bob_fish(time: Res<Time>, mut roots: Query<(&FishRoot, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (fr, mut tr) in &mut roots {
        if fr.swim {
            tr.translation.y = FLOAT_Y + (t * 1.7 + fr.phase).sin() * 0.02;
        }
    }
}

/// 꼬리 스위시 — 피벗 좌우 회전(진행축 유지).
pub fn swish_fish_tails(time: Res<Time>, mut tails: Query<(&FishTail, &mut Transform)>) {
    let t = time.elapsed_secs();
    for (ft, mut tr) in &mut tails {
        tr.rotation = Quat::from_rotation_y((t * 8.0 + ft.phase).sin() * 0.45);
    }
}
