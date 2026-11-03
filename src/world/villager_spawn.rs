//! 주민 스폰 파이프라인 — villager.rs에서 분리(500줄 규칙·기능별 분리,
//! 2026-11-03). GLB 로딩 경합을 흡수하는 완결 보장 3종(WorldInstanceReady
//! 옵저버 → 재시도 폴 → finish 공용 본체: 애님 그래프 5클립·크로스페이드·
//! 오토펏·캐릭터 프레임워크 마커). 배회 FSM·애님 상태는 villager.rs.

use bevy::gltf::Gltf;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use std::time::Duration;

use super::villager::{AnimKind, IdleFidget, Villager, VillagerAnims, VillagerModel};

/// 스폰 완료 보장 마커 — finish_villager_setup이 아직 못 끝낸 주민.
/// 무음 early-return이 애님+오토펏을 영구 누락시켜 원시 스케일 유령(2026-10-17
/// 신규 합류 주민 '테린' 사례 — 화면에 아예 안 보임)을 낳는다. 재시도로 보장한다.
#[derive(Component)]
pub struct VillagerPending(pub f32);

pub fn villager_ready(
    trigger: On<WorldInstanceReady>,
    mut commands: Commands,
    q_self: Query<(&Villager, &VillagerModel, &Transform, Option<&super::reference_look::ReferenceVillager>)>,
    children: Query<&Children>,
    mut players: Query<(Entity, &mut AnimationPlayer)>,
    q_walk: super::prop_fit::FitQuery,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    meshes: Res<Assets<Mesh>>,
    mut aabb_cache: ResMut<super::prop_fit::MeshAabbCache>,
    world: Res<super::data::WorldData>,
) {
    let Ok((villager, model, tr, reference)) = q_self.get(trigger.entity) else {
        return;
    };
    let done = finish_villager_setup(
        trigger.entity,
        villager,
        model,
        tr,
        reference.is_some(),
        &mut commands,
        &children,
        &mut players,
        &q_walk,
        &gltfs,
        &mut graphs,
        &meshes,
        &mut aabb_cache,
        &world,
    );
    if !done {
        commands.entity(trigger.entity).insert(VillagerPending(0.0));
    }
}

/// 스폰 완료 재시도 — Gltf·클립·AnimationPlayer가 비동기로 늦게 도착하는
/// 경합을 매 프레임 재시도로 흡수(성공 시 마커 제거, 60s 포기).
#[allow(clippy::too_many_arguments)] // Bevy 시스템 파라미터 — finish_villager_setup과 동일 세트
pub fn retry_villager_setup(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(
        Entity,
        &Villager,
        &VillagerModel,
        &Transform,
        Option<&super::reference_look::ReferenceVillager>,
        &mut VillagerPending,
    )>,
    children: Query<&Children>,
    mut players: Query<(Entity, &mut AnimationPlayer)>,
    q_walk: super::prop_fit::FitQuery,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    meshes: Res<Assets<Mesh>>,
    mut aabb_cache: ResMut<super::prop_fit::MeshAabbCache>,
    world: Res<super::data::WorldData>,
) {
    for (entity, villager, model, tr, reference, mut pending) in &mut q {
        pending.0 += time.delta_secs();
        if pending.0 > 60.0 {
            warn!("[villager] 스폰 완료 실패(60s) — 포기 {:?}", model.0.path());
            commands.entity(entity).remove::<VillagerPending>();
            continue;
        }
        let done = finish_villager_setup(
            entity,
            villager,
            model,
            tr,
            reference.is_some(),
            &mut commands,
            &children,
            &mut players,
            &q_walk,
            &gltfs,
            &mut graphs,
            &meshes,
            &mut aabb_cache,
            &world,
        );
        if done {
            info!("[villager] 스폰 완료 재시도 성공 (+{:.1}s)", pending.0);
            commands.entity(entity).remove::<VillagerPending>();
        }
    }
}

/// 주민 씬 완결 공용 본체 — 애님 그래프·크로스페이드 시작·오토펏.
/// Gltf/클립/플레이어 미준비면 false(호출처가 재시도 마커 처리).
#[allow(clippy::too_many_arguments)] // 옵저버·재시도 양쪽이 같은 자산 집합을 주입
fn finish_villager_setup(
    entity: Entity,
    villager: &Villager,
    model: &VillagerModel,
    tr: &Transform,
    reference: bool,
    commands: &mut Commands,
    children: &Query<&Children>,
    players: &mut Query<(Entity, &mut AnimationPlayer)>,
    q_walk: &super::prop_fit::FitQuery,
    gltfs: &Assets<Gltf>,
    graphs: &mut Assets<AnimationGraph>,
    meshes: &Assets<Mesh>,
    aabb_cache: &mut super::prop_fit::MeshAabbCache,
    world: &super::data::WorldData,
) -> bool {
    let Some(gltf) = gltfs.get(&model.0) else {
        return false; // Gltf 아직 로딩 — 재시도로 흡수
    };
    // Walk/Idle 클립 검색 — 접미 매칭(프리픽스는 모델별 상이)
    let find = |suffix: &str| {
        gltf.named_animations
            .iter()
            .find(|(name, _)| {
                name.to_ascii_lowercase()
                    .ends_with(&suffix.to_ascii_lowercase())
            })
            .map(|(_, h)| h.clone())
    };
    let (Some(walk_h), Some(idle_h)) = (find("Walk"), find("Idle")) else {
        return false;
    };
    let mut graph = AnimationGraph::new();
    let walk_i = graph.add_clip(walk_h, 1.0, graph.root);
    let idle_i = graph.add_clip(idle_h, 1.0, graph.root);
    // Interact·Wave — 이 팩엔 있으나 이후 합류 모델을 위해 폴백(Idle 인덱스)
    let interact_i = find("Interact")
        .map(|h| graph.add_clip(h, 1.0, graph.root))
        .unwrap_or(idle_i);
    let wave_i = find("Wave")
        .map(|h| graph.add_clip(h, 1.0, graph.root))
        .unwrap_or(idle_i);
    let punch_i = find("Punch_Left")
        .map(|h| graph.add_clip(h, 1.0, graph.root))
        .unwrap_or(interact_i);
    let graph_h = graphs.add(graph);

    // 하위 AnimationPlayer 탐색(bevy_gltf가 자동 스폰)
    let mut player_e = None;
    for child in children.iter_descendants(entity) {
        if players.get(child).is_ok() {
            player_e = Some(child);
            break;
        }
    }
    let Some(player_e) = player_e else {
        return false; // 플레이어 아직 스폰 전 — 재시도로 흡수
    };
    if let Ok((_, player)) = players.get_mut(player_e) {
        let mut player = player.clone();
        // AnimationTransitions — 크로스페이드 주체(재생은 이 컴포넌트 경유로만).
        let mut transitions = AnimationTransitions::default();
        transitions
            .play(&mut player, idle_i, Duration::ZERO)
            .repeat();
        commands.entity(player_e).insert((player, transitions));
    }
    commands
        .entity(player_e)
        .insert(AnimationGraphHandle(graph_h));

    // 오토펏 — 서브트리 전체 메시의 결합 AABB(조상 체인 행렬)로 스케일·착지.
    // GLB 루트 직속 자식엔 메시가 없고 Armature 아래 깊이 있으므로 재귀 수집.
    let mut measure: Vec<Entity> = Vec::new();
    for child in children.iter_descendants(entity) {
        if q_walk
            .get(child)
            .map(|(_, _, m, _, _)| m.is_some())
            .unwrap_or(false)
        {
            measure.push(child);
        }
    }
    if let Some((mn, mx)) = super::prop_fit::meshes_aabb(&measure, entity, q_walk, meshes, aabb_cache)
    {
        let size = (mx - mn).max(Vec3::splat(1e-4));
        let s = (villager.target_h / size.y).clamp(0.05, 50.0);
        // 착지 기준은 현재 위치의 지면 — 씬 로딩 사이 배회로 스폰점에서
        // 벗어났어도 되돌아가지 않는다(발밑 고정, 회전·수평 위치 보존).
        let ground = world.height_at(
            tr.translation.x.floor() as i32,
            tr.translation.z.floor() as i32,
        );
        let mut fitted = *tr;
        fitted.translation.y = ground - mn.y * s;
        fitted.scale = Vec3::splat(s);
        commands.entity(entity).insert(fitted);
    }
    commands.entity(entity).insert(VillagerAnims {
        walk: walk_i,
        idle_anim: idle_i,
        interact: interact_i,
        wave: wave_i,
        punch: punch_i,
        player_e,
        current: AnimKind::Idle,
    });
    // 캐릭터 프레임워크 마커 — 치비화 대상·접지 액터·애님 상태 스냅샷
    // (P1 모듈화: 게임 무관 시스템이 이 컴포넌트만 본다).
    // Fall: 절벽 단차에서 지구 중력 물리낙하 참여(fall.rs).
    commands.entity(entity).insert((
        crate::character::ChibiTarget,
        crate::character::GroundActor,
        crate::character::AnimState(AnimKind::Idle),
        crate::character::tool::HeldTool::default(),
        crate::character::Fall::default(),
        IdleFidget::new(entity.to_bits() as u64),
    ));
    true
}
