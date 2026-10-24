//! 주민 — Quaternius CC0 리깅 캐릭터(GLB) + 상태 애니메이션 + 배회 FSM.
//! Phase 2에서 서버 agents 스키마 좌표로 구동된다(구조 동일).
//! 스폰 → WorldInstanceReady 옵저버에서 오토펏(족장 지면 착지)·애니메이션
//! 그래프(Walk/Idle/Interact/Wave 4클립) 연결. 이동 시 캐릭터가 이동 방향으로
//! 회전. 활동(채집·건설…)은 Interact, 주고받기(give·trade)는 Wave로 연출한다.

use bevy::gltf::Gltf;
use bevy::prelude::*;
use bevy::world_serialization::WorldInstanceReady;
use std::time::Duration;

use super::data::WorldData;

const WALK_TIME: f32 = 0.85;
/// 주민 키 목표(미터) — 2026-10-19 사용자 요청 축소(1.62→1.38)
pub const VILLAGER_H: f32 = 1.38;

/// 모델 전방 보정 — Quaternius 리깅는 +Z 전방(실측 후 조정)
const MODEL_YAW: f32 = 0.0;

#[derive(Component)]
pub struct Villager {
    pub from: Vec3,
    pub to: Vec3,
    pub t: f32,
    pub moving: bool,
    pub idle: f32,
    pub target_h: f32,
    /// 표시명(데모: 이름 풀 · 라이브: 서버 name) — UI 주민 목록·선택 패널
    pub name: String,
    /// 상태 코드(walk·idle·chop… 레거시 status.* 규칙) — UI가 한글로 번역
    pub status: String,
    /// 활동 코드(라이브: 서버 actType[gather·build…] · 데모: 배회 휴식 연출) —
    /// 애님 상태(Interact/Wave)와 UI 라벨의 원천. 비면 status로 폴백.
    pub act: String,
}

/// 애님 상태 — character 프레임워크 소유(재사용 계층), 게임은 재수출.
pub use crate::character::AnimKind;

/// 활동 코드 → 애님 상태. 타격성 작업(사냥·건설·채굴·벌목·어로)은 Punch
/// (망치·찌르기 — 실제 동작 유사성, 2026-10-24 P3), 주고받기는 Wave,
/// 휴식은 Idle. 알 수 없는 코드는 Interact(활동형 기본 — 시각적으로
/// 하는 일이 있는 게 생명감).
fn act_kind(code: &str) -> AnimKind {
    match code {
        "moving" => AnimKind::Walk,
        "hunt" | "build" | "craft" | "chop" | "mine" | "smelt" | "fish" => AnimKind::Punch,
        "give" | "trade" => AnimKind::Wave,
        "idle" | "sleep" | "sleeping" | "thinking" | "" => AnimKind::Idle,
        _ => AnimKind::Interact,
    }
}

#[derive(Component)]
pub struct VillagerAnims {
    pub walk: AnimationNodeIndex,
    pub idle_anim: AnimationNodeIndex,
    pub interact: AnimationNodeIndex,
    pub wave: AnimationNodeIndex,
    /// 망치질·내지르기 계열(사냥·건설·채굴) — qchar Punch_Left 폴백 Interact.
    pub punch: AnimationNodeIndex,
    pub player_e: Entity,
    pub current: AnimKind,
}

#[derive(Component)]
pub struct VillagerModel(pub Handle<Gltf>, pub Option<String>);

/// Quaternius Ultimate Modular Men/Women(CC0, 표준 휴머노이드 리그·애님 24종,
/// Body/Legs/Feet 파츠 분리) — 원시 마을 테마 6종. 치비화(본 스케일)는
/// chibiify가 담당. 착의 다양성은 변형(의상·헤어)이, 부착 아이템은 wear가 담당.
pub(crate) const MODELS_M: [&str; 4] = [
    "models/qchar/m-farmer.glb",
    "models/qchar/m-worker.glb",
    "models/qchar/m-adventurer.glb",
    "models/qchar/m-casual-character.glb",
];
const MODELS_F: [&str; 2] = [
    "models/qchar/f-adventurer.glb",
    "models/qchar/f-worker.glb",
];

/// 라이브 에이전트용 — lookJson.sex(있으면) 또는 id 해시로 남/녀 그룹을 고르고,
/// 해시로 변형(의상·헤어)을 1택한다.
pub fn model_path_for(id: &str, look: Option<&str>) -> &'static str {
    let female = if let Some(look) = look
        && look.contains("\"sex\":\"")
        && let Some(i) = look.find("\"sex\":\"")
    {
        let c = look[i + 7..].chars().next().unwrap_or('M');
        c == 'F' || c == 'f'
    } else {
        false
    };
    let h = id.bytes().fold(0u64, |a, b| a.wrapping_mul(31) + b as u64);
    let group: &[&str] = if female { &MODELS_F } else { &MODELS_M };
    group[(h % group.len() as u64) as usize]
}

/// 스폰 완료 보장 마커 — finish_villager_setup이 아직 못 끝낸 주민.
/// 무음 early-return이 애님+오토펏을 영구 누락시켜 원시 스케일 유령(2026-10-17
/// 신규 합류 주민 '테린' 사례 — 화면에 아예 안 보임)을 낳는다. 재시도로 보장한다.
#[derive(Component)]
pub struct VillagerPending(pub f32);

pub fn villager_ready(
    trigger: On<WorldInstanceReady>,
    mut commands: Commands,
    q_self: Query<(&Villager, &VillagerModel, &Transform)>,
    children: Query<&Children>,
    mut players: Query<(Entity, &mut AnimationPlayer)>,
    q_walk: super::prop_fit::FitQuery,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    meshes: Res<Assets<Mesh>>,
    mut aabb_cache: ResMut<super::prop_fit::MeshAabbCache>,
    world: Res<super::data::WorldData>,
) {
    let Ok((villager, model, tr)) = q_self.get(trigger.entity) else {
        return;
    };
    let done = finish_villager_setup(
        trigger.entity,
        villager,
        model,
        tr,
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
    for (entity, villager, model, tr, mut pending) in &mut q {
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
    ));
    true
}

/// 시간 기반 자유 난수(배회 휴식 길이) — 결정론 대상 아님(연출).
fn rand_free(seed: u64, scale: f32) -> f32 {
    super::data::Rng::new(seed.wrapping_mul(2654435761)).f32() * scale
}

/// 데모 휴식 활동 풀 — act.* 어휘에 있는 코드만(UI 라벨·애님 매핑 보장).
const DEMO_ACTS: [&str; 7] = ["gather", "chop", "forage", "craft", "drink", "give", "eat"];

pub fn wander(
    time: Res<Time>,
    world: Res<WorldData>,
    mut q: Query<(&mut Villager, &mut Transform), Without<crate::net::apply::LiveAgentTag>>,
) {
    for (mut v, mut tr) in &mut q {
        // 상태 코드 갱신(데모 전용 — 라이브는 sync_agents가 서버 값 반영).
        // 활동 중(act 비빔)엔 활동 코드가 곧 상태(라벨·애님 일관).
        let want = if v.moving {
            "walk".to_string()
        } else if v.act.is_empty() {
            "idle".to_string()
        } else {
            v.act.clone()
        };
        if v.status != want {
            v.status = want;
        }
        if v.moving {
            v.t += time.delta_secs() / WALK_TIME;
            if v.t >= 1.0 {
                v.t = 0.0;
                v.moving = false;
                // 도착 휴식 — 35%는 자리에서 활동(채집·손짓), 나머지는 그냥 대기.
                let seed = (time.elapsed_secs() * 1000.0) as u64
                    ^ (tr.translation.x as u64).wrapping_mul(31);
                if rand_free(seed, 1.0) < 0.35 {
                    let pool = DEMO_ACTS.len();
                    v.act =
                        DEMO_ACTS[(rand_free(seed.wrapping_add(7), pool as f32) as usize) % pool]
                            .into();
                    v.idle = 1.6 + rand_free(seed.wrapping_add(13), 2.0);
                } else {
                    v.act.clear();
                    v.idle = 0.4 + rand_free(seed, 2.0);
                }
                tr.translation = v.to;
            } else {
                let s = v.t * v.t * (3.0 - 2.0 * v.t); // smoothstep
                tr.translation = v.from.lerp(v.to, s);
                // 고도 보간 — 계단 절벽에서 직선 관통 방지(낮은 쪽 따라감)
                tr.translation.y = tr.translation.y.max(v.from.y.min(v.to.y));
            }
            continue;
        }
        v.idle -= time.delta_secs();
        if v.idle > 0.0 {
            continue;
        }
        let here = (
            tr.translation.x.floor() as i32,
            tr.translation.z.floor() as i32,
        );
        let mut rng = super::data::Rng::new(
            (here.0 as u64 * 31 + here.1 as u64).wrapping_mul(2654435761)
                ^ (time.elapsed_secs() as u64),
        );
        let dirs = [
            [0, 1],
            [0, -1],
            [1, 0],
            [-1, 0],
            [1, 1],
            [-1, 1],
            [1, -1],
            [-1, -1],
        ];
        let start = (rng.next_u64() % 8) as usize;
        for i in 0..8 {
            let d = dirs[(start + i) % 8];
            let (nx, ny) = (here.0 + d[0], here.1 + d[1]);
            if world.walkable(nx, ny) {
                let target = Vec3::new(nx as f32 + 0.5, world.height_at(nx, ny), ny as f32 + 0.5);
                // 이동 방향 회전(발밑 고정 — 스케일 보존)
                let yaw = (d[0] as f32).atan2(d[1] as f32) + MODEL_YAW;
                tr.rotation = Quat::from_rotation_y(yaw);
                v.from = tr.translation;
                v.to = target;
                v.moving = true;
                v.act.clear(); // 이동 개시 — 활동 종료
                v.t = 0.0;
                break;
            }
        }
        if !v.moving {
            v.idle = rand_free(
                (time.elapsed_secs() * 1000.0) as u64 ^ (here.0 as u64 * 31 + here.1 as u64),
                1.5,
            );
        }
    }
}

/// 애님 상태 전환(플레이어는 하위 엔티티에 있음) — 이동 우선, 정지 시
/// 활동 코드(act 우선·status 폴백)로 Walk/Idle/Interact/Wave를 고른다.
/// 지시 행동(ActiveAction) 수행 중엔 프레임워크 요청 애님(던지기 모션 등)을
/// 우선한다. 0.25초 크로스페이드 — 하드 컷은 주민을 기계적으로 보이게
/// 한다(생명감 직결).
pub fn animate_states(
    mut q: Query<(
        &Villager,
        &mut VillagerAnims,
        &mut crate::character::AnimState,
        Option<&crate::character::ActiveAction>,
    )>,
    mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    for (v, mut anims, mut state, action) in &mut q {
        let want = if v.moving {
            AnimKind::Walk
        } else {
            act_kind(if v.act.is_empty() { &v.status } else { &v.act })
        };
        let want = match action {
            Some(a) => a.anim(), // 행동 중 오버라이드(준비 모션·복원)
            None => want,
        };
        if want == anims.current {
            continue;
        }
        anims.current = want;
        state.0 = want; // 캐릭터 프레임워크 스냅샷 동기(foot_lock 등 소비)
        let idx = match want {
            AnimKind::Walk => anims.walk,
            AnimKind::Idle => anims.idle_anim,
            AnimKind::Interact => anims.interact,
            AnimKind::Punch => anims.punch,
            AnimKind::Wave => anims.wave,
        };
        if let Ok((mut player, mut transitions)) = players.get_mut(anims.player_e) {
            transitions
                .play(&mut player, idx, Duration::from_millis(250))
                .repeat();
        }
    }
}

/// 화면 밖 주민 애님 일시정지 — 카메라 시야(정사영) 밖 주민은
/// AnimationPlayer를 pause해 샘플링 비용을 끊는다(2026-10-19 아키텍처 검토
/// 반영 — 대규모 월드 CPU 절감). 이동 자체는 계속돼 화면 복귀 시 위치만
/// 갱신되고, 포즈는 재개 다음 프레임에 수렴한다. 렌더 컬링은 별도로
/// 이미 동작하므로 이 시스템은 CPU 애님 비용만 담당.
pub fn throttle_offscreen_anims(
    rig: Res<super::camera_rig::CameraRig>,
    q: Query<(&Transform, &VillagerAnims)>,
    mut players: Query<&mut AnimationPlayer>,
) {
    // 시야 반경 근사 — 세로 시야(VIEW_H×zoom)의 대각 여유 계수. 줌아웃
    // (zoom 최대 4.2 → ~74m)까지 커버한다.
    let view_r = super::camera_rig::VIEW_H * rig.zoom * 0.8;
    for (tr, anims) in &q {
        let off = tr.translation.xz().distance(rig.target) > view_r;
        if let Ok(mut player) = players.get_mut(anims.player_e) {
            // 0.19은 애님별 pause — 재생 중 전체에 일괄 적용(전환 중 복수 클립).
            let any_live = player.playing_animations().any(|(_, a)| !a.is_paused());
            if off && any_live {
                for (_, a) in player.playing_animations_mut() {
                    a.pause();
                }
            } else if !off && !any_live {
                for (_, a) in player.playing_animations_mut() {
                    a.resume();
                }
            }
        }
    }
}
