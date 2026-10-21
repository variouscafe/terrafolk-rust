//! Universal Base Characters 전시 — 애님 라이브러리(Universal Animation
//! Library, 이 리그 전용 클립) 확보 전까지 베이스 캐릭터를 마을 한켠에
//! 세워 렌더링·치비 비율·헤어 부착을 실증한다. UE5 마네킹 리그
//! (pelvis·spine_0x·thigh_*)라 chibi·wear의 이중 리그 지원을 함께 검증.
//! 애님 팩 투입 시 주민 베이스(MODELS_*)로 전환하고 이 전시는 철거한다.

use bevy::asset::Assets;
use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;
use bevy::world_serialization::{WorldAssetRoot, WorldInstanceReady};

use super::chibi;
use super::data::WorldData;
use super::prop_fit::{FitQuery, MeshAabbCache, meshes_aabb};

/// 전시 캐릭터 루트.
#[derive(Component)]
pub struct CharDisplay;

/// 전시 완결 마커(오토폿·치비·헤어 적용 후).
#[derive(Component)]
pub struct DisplayDone;

/// 스폰 완료 재시도 마커(자산 비동기 지연 흡수 — villager Pending 패턴).
#[derive(Component)]
pub struct DisplayPending(pub f32);

/// 전시 개체 자산 묶음 — 완결 처리에서 헤어 씬에 쓴다.
#[derive(Component)]
pub struct DisplayModel(pub Handle<bevy::world_serialization::WorldAsset>);

/// 전시 개체 정의 — 모델·헤어.
const DISPLAYS: [(&str, &str); 2] = [
    ("models/qubase/ubase-male.glb", "models/qubase/hair-long.glb"),
    ("models/qubase/ubase-female.glb", "models/qubase/hair-buns.glb"),
];

/// 전시 키(미터) — 주민(1.38)보다 살짝 작게(2026-10-19 동반 축소).
const DISPLAY_H: f32 = 1.25;

/// 콩눈 캘리브레이션(Head 본 로컬 단위 — UE5 리그 원본 스케일 기준,
/// 2026-10-19 흰 점 픽셀·비전 실측으로 위치 확정).
const EYE_R: f32 = 0.016;
const EYE_GAP: f32 = 0.036;
const EYE_Y: f32 = 0.028;
const EYE_Z: f32 = 0.058;
/// 콩눈 색 — 진갈색(안드로이드 눈동자 톤, 언릿이라 밤에도 유지).
const EYE_COLOR: Color = Color::srgb(0.13, 0.09, 0.07);

/// 마을 중심 인근에 전시 2인 스폰(월드 준비 후 1회).
pub fn spawn_char_display(
    world: Res<WorldData>,
    mut commands: Commands,
    assets: Res<AssetServer>,
    q: Query<(), With<CharDisplay>>,
) {
    if !q.is_empty() {
        return;
    }
    let c = world.size as i32 / 2;
    for (dx, dz) in [(4, 0), (3, 0), (5, 0), (3, 1), (5, 1), (4, 1), (2, 0), (6, 0)] {
        let (x, y) = (c + dx, c + dz);
        if !world.walkable(x, y) {
            continue;
        }
        let ground = world.height_at(x, y);
        for (i, (model, hair)) in DISPLAYS.iter().enumerate() {
            let scene = assets.load(GltfAssetLabel::Scene(0).from_asset(*model));
            let hair_scene = assets.load(GltfAssetLabel::Scene(0).from_asset(*hair));
            // 나란히 카메라(남동 45°)쪽을 살짝 비스듬히(좌 38°·우 52°)
            let side = i as f32 - 0.5;
            let yaw = std::f32::consts::FRAC_PI_4 + side * 0.24;
            commands
                .spawn((
                    CharDisplay,
                    DisplayModel(hair_scene),
                    WorldAssetRoot(scene),
                    Transform::from_translation(Vec3::new(
                        x as f32 + 0.5 + side * 1.4,
                        ground,
                        y as f32 + 0.5,
                    ))
                    .with_rotation(Quat::from_rotation_y(yaw)),
                ))
                .observe(display_ready);
        }
        info!("[display] Universal Base Characters 전시 스폰 @({x},{y})");
        return;
    }
}

/// 씬 스폰 완료 — 완결 본체 호출(불완결이면 재시도 마커).
#[allow(clippy::type_complexity)] // Bevy 시스템 파라미터 — finish_display와 동일 세트
pub fn display_ready(
    trigger: On<WorldInstanceReady>,
    mut commands: Commands,
    q: Query<(&DisplayModel, &Transform)>,
    children: Query<&Children>,
    names: Query<&Name>,
    q_mesh: FitQuery,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut aabb_cache: ResMut<MeshAabbCache>,
    world: Res<WorldData>,
) {
    let Ok((model, tr)) = q.get(trigger.entity) else {
        return;
    };
    if !finish_display(
        trigger.entity,
        model,
        tr,
        &mut commands,
        &children,
        &names,
        &q_mesh,
        &mut meshes,
        &mut mats,
        &mut aabb_cache,
        &world,
    ) {
        commands.entity(trigger.entity).insert(DisplayPending(0.0));
    }
}

/// 전시 완결 재시도(초 단위 누적, 30s 포기).
#[allow(clippy::type_complexity)] // Bevy 시스템 파라미터 — finish_display와 동일 세트
pub fn retry_display(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &DisplayModel, &Transform, &mut DisplayPending)>,
    children: Query<&Children>,
    names: Query<&Name>,
    q_mesh: FitQuery,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mut aabb_cache: ResMut<MeshAabbCache>,
    world: Res<WorldData>,
) {
    for (entity, model, tr, mut pending) in &mut q {
        pending.0 += time.delta_secs();
        if pending.0 > 30.0 {
            warn!("[display] 전시 완결 실패(30s) — 포기");
            commands.entity(entity).remove::<DisplayPending>();
            continue;
        }
        if finish_display(
            entity,
            model,
            tr,
            &mut commands,
            &children,
            &names,
            &q_mesh,
            &mut meshes,
            &mut mats,
            &mut aabb_cache,
            &world,
        ) {
            commands.entity(entity).remove::<DisplayPending>();
        }
    }
}

/// 본 스케일 지연 적용 마커 — 완결 중 Commands로 넣고 실제 곱은 이 시스템이
/// 수행(서브트리 순회용 children 쿼리와 &mut Transform의 접근 충돌 회피).
#[derive(Component)]
pub struct ScaleMark(f32);

pub fn apply_scale_marks(mut commands: Commands, mut q: Query<(Entity, &ScaleMark, &mut Transform)>) {
    for (e, mark, mut tr) in &mut q {
        tr.scale *= mark.0;
        commands.entity(e).remove::<ScaleMark>();
    }
}

/// 전시 완결 공용 본체 — 오토폿(AABB) → 치비 본 스케일 → Head 본 헤어 부착.
/// 메시·본이 아직 없으면 false(호출처가 재시도 처리).
#[allow(clippy::too_many_arguments)] // 옵저버·재시도 양쪽이 같은 자산 집합을 주입
fn finish_display(
    entity: Entity,
    model: &DisplayModel,
    tr: &Transform,
    commands: &mut Commands,
    children: &Query<&Children>,
    names: &Query<&Name>,
    q_mesh: &FitQuery,
    meshes: &mut Assets<Mesh>,
    mats: &mut Assets<StandardMaterial>,
    aabb_cache: &mut MeshAabbCache,
    world: &WorldData,
) -> bool {
    // 1) 오토폿 — 서브트리 결합 AABB로 목표 키·착지(발바닥 기준)
    let mut measure: Vec<Entity> = Vec::new();
    for child in children.iter_descendants(entity) {
        if q_mesh
            .get(child)
            .map(|(_, _, m, _, _)| m.is_some())
            .unwrap_or(false)
        {
            measure.push(child);
        }
    }
    let Some((mn, _mx)) = meshes_aabb(&measure, entity, q_mesh, meshes, aabb_cache) else {
        return false; // 메시 아직 로딩 — 재시도로 흡수
    };
    let span = _mx.y - mn.y;
    if span < 0.1 {
        return false; // AABB 미완(파편만 잡힘)
    }
    let s = (DISPLAY_H / span.max(1e-4)).clamp(0.05, 50.0);
    let ground = world.height_at(
        tr.translation.x.floor() as i32,
        tr.translation.z.floor() as i32,
    );
    let mut fitted = *tr;
    fitted.translation.y = ground - mn.y * s;
    fitted.scale = Vec3::splat(s);
    commands.entity(entity).insert(fitted);
    info!(
        "[display] 오토폿 s={s:.2} span={span:.2}m → {DISPLAY_H}m · 착지 y={:.2}",
        fitted.translation.y
    );
    // 2) 치비 본 스케일(마커 지연 적용) + Head 본 탐색
    let mut head_bone = None;
    for descendant in children.iter_descendants(entity) {
        let Ok(name) = names.get(descendant) else {
            continue;
        };
        if name.as_str() == "Head" {
            head_bone = Some(descendant);
        }
        if let Some(scale) = chibi::bone_scale(name.as_str()) {
            commands.entity(descendant).insert(ScaleMark(scale));
        }
    }
    // 3) 헤어 — Head 본에 부착(스킨드 씬이라 Head와 함께 이동)
    // 4) 콩눈 — 타겟 얼굴(Memoji풍) 근사: Head 본 정면에 어두운 원 2개.
    //    언릿 머티리얼(조명 무시) — 항상 또렷한 점 눈.
    if let Some(head) = head_bone {
        let hair = commands
            .spawn((
                Name::new("display-hair"),
                WorldAssetRoot(model.0.clone()),
                Transform::IDENTITY,
            ))
            .id();
        commands.entity(head).add_child(hair);
        // 디버그 — 4방향 탐침(밝은 흰 점): 얼굴 정면 축 실측 후 제거
        for side in [-1.0f32, 1.0] {
            let eye = commands
                .spawn((
                    Name::new("display-eye"),
                    Mesh3d(meshes.add(Circle::new(EYE_R).mesh().build())),
                    MeshMaterial3d(mats.add(StandardMaterial {
                        base_color: EYE_COLOR,
                        unlit: true,
                        ..default()
                    })),
                    Transform::from_translation(Vec3::new(side * EYE_GAP, EYE_Y, EYE_Z)),
                ))
                .id();
            commands.entity(head).add_child(eye);
        }
    }
    commands.entity(entity).insert(DisplayDone);
    true
}
