//! 채팅 지시 행동 — 물리 기반 액션 상태머신(게임 무관, 2026-10-24).
//!
//! 게임이 지시(채팅 파싱 등)를 `ActiveAction` 부착으로 전달하면 본 모듈이
//! 수행한다. **모든 동작은 공간의 물리 법칙을 따른다** — 투사체와 액터
//! 모두 `SpaceGravity`(이 게임=지구 9.81)로 적분하고, 지면 충돌은
//! `SurfaceField` 표면장으로 판정한다(반발계수 감쇠).
//!
//! | 종류 | 모션 | 물리 |
//! |---|---|---|
//! | TossCatch | 든 물건 수직 토스→낙하→손높이 캐치 | vy₀=4.6 포물선, 하강 중 손높이 통과 시 캐치 |
//! | ThrowGround | 전방 던지기→지형 충돌 반발→정지 | v₀=6.5·50°, 반발 0.35·수평감쇠 0.6 |
//! | Jump | 액터 자체 도약 | `Fall{vy: 양수}` 주입 — fall.rs 점프 확장점 |
//!
//! 행동 추가 = `ActionKind` 변체 + 파싱표(게임) + 본 파일 구현 하나.
//! 애님은 게임 애니메이터가 `anim()`에 물어 반영한다(플레이어 소유 게임 측).

use bevy::prelude::*;

use super::anim::AnimKind;
use super::fall::{Fall, SpaceGravity};
use super::ground::SurfaceField;
use super::tool::{self, HeldTool, ToolPart};

/// 행동 종류 — 채팅 지시 파서(게임 측)가 고른다.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum ActionKind {
    /// 수직 토스 후 손에서 캐치(던졌다가 받아).
    TossCatch,
    /// 전방 포물선 → 지형 반발 → 정지(멀리 던져).
    ThrowGround,
    /// 제자리 도약(점프).
    Jump,
    /// 둘이서 캐치볼 — 근처 주민과 물건 주고받기(놓치면 지면 반발).
    PlayCatch,
}

/// 행동 진행 단계.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    /// 준비 동작(던지기 팔 흔들).
    Windup,
    /// 물리 진행 중(투사체 비행·점프 체공).
    Flight,
    /// 지면 정지(ThrowGround 착지 후).
    Rest,
    /// 캐치 후 짧은 유지.
    Hold,
}

/// 수행 중인 행동 — 부착 즉시 시작, 완료 시 프레임워크가 제거한다.
#[derive(Component)]
pub struct ActiveAction {
    pub kind: ActionKind,
    pub phase: Phase,
    pub t: f32,
    /// 시작 시 애님(종료 시 복원 원본).
    pub saved_anim: AnimKind,
    /// 시작 시 도구 부속(비행 중 은닉, 종료 시 복원 — 빈손이면 빈 벡터).
    pub saved_parts: Vec<ToolPart>,
    /// 빈손일 때 던질 기본 물건(게임 제공 — 이 게임: 더덕).
    pub fallback: ToolPart,
    /// 투사체 루트 엔티티(점프 등 투사체 없음 = None).
    pub proj: Option<Entity>,
    /// 투사체 속도(m/s).
    pub vel: Vec3,
    /// 투사체 자전속(비행 연출, rad/s).
    spin: Vec3,
    /// 반발 횟수(ThrowGround).
    bounces: u8,
    /// 시작 후 누적 시간(s) — 단계별 t와 무관한 총시한 근거.
    total: f32,
    /// 캐치볼 상대(PlayCatch).
    pub partner: Option<Entity>,
    /// 현재 물건 잡은 손 주인(PlayCatch Hold — None=시작 전).
    holder: Option<Entity>,
    /// 남은 캐치 횟수(PlayCatch — 0 되면 종료).
    pub passes_left: u8,
}

impl ActiveAction {
    /// 지시 수행 시작 — 저장 원본(애님·도구)은 이때 스냅샷.
    pub fn new(
        kind: ActionKind,
        saved_anim: AnimKind,
        saved_parts: Vec<ToolPart>,
        fallback: ToolPart,
    ) -> Self {
        Self {
            kind,
            phase: Phase::Windup,
            t: 0.0,
            saved_anim,
            saved_parts,
            fallback,
            proj: None,
            vel: Vec3::ZERO,
            spin: Vec3::ZERO,
            bounces: 0,
            total: 0.0,
            partner: None,
            holder: None,
            passes_left: 0,
        }
    }

    /// 애님 요청 — 던지기 준비 동작만 오버라이드, 나머진 원본 복귀.
    pub fn anim(&self) -> AnimKind {
        match (self.kind, self.phase) {
            (
                ActionKind::TossCatch | ActionKind::ThrowGround | ActionKind::PlayCatch,
                Phase::Windup,
            ) => AnimKind::Punch,
            _ => self.saved_anim,
        }
    }

    /// 캐치볼 상대·총 패스 수 지정(PlayCatch 전용 빌더).
    pub fn with_partner(mut self, partner: Entity, passes: u8) -> Self {
        self.partner = Some(partner);
        self.passes_left = passes;
        self
    }
}

/// 준비 동작 길이(s).
const WINDUP_T: f32 = 0.30;
/// 수직 토스 초기속도(m/s) — 정점 ≈ vy²/2g = 1.08m.
const TOSS_VY: f32 = 4.6;
/// 토스 수평 흔들림 최대(m/s) — 손 중심 밖 자연스러운 타원 궤적.
const TOSS_JITTER: f32 = 0.12;
/// 전방 던지기 초기속도(m/s)·발사각(°) — 사거리 ≈ v²sin2θ/g ≈ 4.2m.
const THROW_V0: f32 = 6.5;
const THROW_ELEV_DEG: f32 = 50.0;
/// 지형 반발계수·수평 감쇠(ThrowGround) — 2회 접촉 후 정지.
const RESTITUTION: f32 = 0.35;
const HORIZ_DAMP: f32 = 0.6;
/// 점프 초기속도(m/s) — 높이 ≈ 0.52m·체공 ≈ 0.65s.
const JUMP_VY: f32 = 3.2;
/// 캐치볼 체공·캐치 판정 — 체공은 거리 비례(기준 구속 10m/s, 범위 0.6~1.6s):
/// 짧은 패스는 부드럽게, 먼 패스는 ½gT² 만큼 높은 포물선(로브). 고정 T는
/// 28m 패스를 33m/s 레이저로 만들었다(실측 2026-10-24).
const CATCH_V: f32 = 10.0;
const CATCH_T_MIN: f32 = 0.6;
const CATCH_T_MAX: f32 = 1.6;
const CATCH_R: f32 = 0.55;

/// 거리별 체공 시간(s).
fn catch_t(from: Vec3, to: Vec3) -> f32 {
    (from.distance(to) / CATCH_V).clamp(CATCH_T_MIN, CATCH_T_MAX)
}
/// 캐치 유지·지면 정지·비행 이상 시한(s).
const HOLD_T: f32 = 0.25;
const REST_T: f32 = 1.2;
const FLIGHT_MAX: f32 = 3.0;
/// 행동 총시한(s) — 단계 루프 이상 시 강제 종료.
const ACTION_TOTAL_MAX: f32 = 12.0;
/// 투사체 반지름 근사(m) — 지면·손 접촉 판정 여유.
const PROJ_R: f32 = 0.08;

/// 투사체 마커 — 루트(부속 메시는 자식). 소유 액터 철거 시 정리.
#[derive(Component)]
pub struct ActionProj {
    pub owner: Entity,
}

/// 행동 진행 — 준비→발사(투사체 스폰·도구 은닉)→물리→캐치/정지→복원.
#[allow(clippy::too_many_arguments)]
pub fn advance_actions<F: Resource + SurfaceField>(
    mut commands: Commands,
    gravity: Res<SpaceGravity>,
    world: Res<F>,
    time: Res<Time>,
    mut q: Query<
        (
            Entity,
            &Transform,
            &mut ActiveAction,
            &mut HeldTool,
            Option<&mut Fall>,
        ),
        // B0001 회피 — 투사체(Transform 쓰기)와 주민(Transform 읽기) 접근
        // 동립성 증명(투사체 루트엔 ActiveAction이 없다).
        Without<ActionProj>,
    >,
    mut projs: Query<&mut Transform, With<ActionProj>>,
    gts: Query<&GlobalTransform>,
    children_q: Query<&Children>,
    names: Query<&Name>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    let dt = time.delta_secs();
    let g = gravity.0;
    for (root_e, root_tr, mut act, mut tool, fall) in &mut q {
        act.t += dt;
        // 총시한 — 단계 전환이 어떤 경로로든 막혀도 영구 스택 방지(진단
        // 로그와 함께). 캐치볼 3패스(~5s)·LOCK 9s보다 여유.
        act.total += dt;
        if act.total > ACTION_TOTAL_MAX {
            warn!("[action] 총시한 초과 종료 {:?} ({:.1}s)", act.kind, act.total);
            let p = act.proj;
            finish(&mut commands, root_e, &mut act, &mut tool, p);
            continue;
        }
        match act.phase {
            Phase::Windup => {
                if act.t < WINDUP_T {
                    continue;
                }
                // 캐치볼 최초 소유 확정(시작 주민이 물건을 쥔다).
                if act.kind == ActionKind::PlayCatch && act.holder.is_none() {
                    act.holder = Some(root_e);
                    info!("[action] 캐치볼 시작 — 패스 {}회", act.passes_left);
                }
                match act.kind {
                    ActionKind::Jump => {
                        let Some(mut f) = fall else {
                            finish(&mut commands, root_e, &mut act, &mut tool, None);
                            continue;
                        };
                        f.vy = JUMP_VY;
                        f.airborne = true;
                        act.phase = Phase::Flight;
                        act.t = 0.0;
                        info!("[action] 점프 vy={JUMP_VY}m/s");
                    }
                    ActionKind::TossCatch
                    | ActionKind::ThrowGround
                    | ActionKind::PlayCatch => {
                        // 발사 — 손 위치·주민 정면 방향에서 투사체 스폰, 도구 은닉.
                        let Some(at) = tool::find_hand(root_e, &children_q, &names)
                            .and_then(|h| gts.get(h).ok())
                            .map(|gt| gt.translation())
                        else {
                            finish(&mut commands, root_e, &mut act, &mut tool, None);
                            continue;
                        };
                        // 캐치볼: 상대 손을 향한 탄도 해(체공 T 고정).
                        let play_target = if act.kind == ActionKind::PlayCatch {
                            act.partner
                                .and_then(|p| tool::find_hand(p, &children_q, &names))
                                .and_then(|h| gts.get(h).ok())
                                .map(|gt| gt.translation())
                        } else {
                            None
                        };
                        let vel = match (act.kind, play_target) {
                            (ActionKind::ThrowGround, _) => {
                                let mut fwd = root_tr.forward().with_y(0.0);
                                if fwd.length_squared() < 1e-4 {
                                    fwd = Vec3::Z;
                                }
                                fwd = fwd.normalize();
                                let e = THROW_ELEV_DEG.to_radians();
                                fwd * (THROW_V0 * e.cos()) + Vec3::Y * (THROW_V0 * e.sin())
                            }
                            (ActionKind::PlayCatch, Some(to)) => {
                                ballistic(at, to, g, catch_t(at, to))
                            }
                            _ => {
                                Vec3::Y * TOSS_VY
                                    + Vec3::new(
                                        rand_jitter(root_e.to_bits() as u64),
                                        0.0,
                                        rand_jitter(root_e.to_bits() as u64 ^ 0x9E37),
                                    ) * TOSS_JITTER
                            }
                        };
                        let parts: Vec<ToolPart> = if tool.parts.is_empty() {
                            vec![act.fallback.clone()]
                        } else {
                            std::mem::take(&mut tool.parts)
                        };
                        let proj = spawn_proj(
                            root_e, &parts, at, &mut commands, &mut meshes, &mut mats,
                        );
                        act.proj = Some(proj);
                        act.vel = vel;
                        act.spin = Vec3::new(
                            rand_jitter(root_e.to_bits() as u64 | 1),
                            2.0,
                            rand_jitter(root_e.to_bits() as u64 ^ 0x55),
                        );
                        act.phase = Phase::Flight;
                        act.t = 0.0;
                        info!(
                            "[action] 발사 {:?} v=({:.1},{:.1},{:.1})m/s",
                            act.kind, vel.x, vel.y, vel.z
                        );
                    }
                }
            }
            Phase::Flight => {
                let Some(proj_e) = act.proj else {
                    // 점프 — 체공 종료(Fall 착지) 대기.
                    if !fall.map(|f| f.airborne).unwrap_or(false) {
                        act.phase = Phase::Hold;
                        act.t = 0.0;
                    }
                    continue;
                };
                if act.t > FLIGHT_MAX || commands.get_entity(proj_e).is_err() {
                    finish(&mut commands, root_e, &mut act, &mut tool, Some(proj_e));
                    continue;
                }
                let Ok(mut ptr) = projs.get_mut(proj_e) else {
                    continue;
                };
                act.vel.y -= g * dt;
                ptr.translation += act.vel * dt;
                if let Ok(axis) = Dir3::new(act.spin) {
                    ptr.rotate_axis(axis, 3.2 * dt);
                }
                let hand_pos = || {
                    tool::find_hand(root_e, &children_q, &names)
                        .and_then(|h| gts.get(h).ok())
                        .map(|gt| gt.translation())
                };
                match act.kind {
                    ActionKind::TossCatch => {
                        // 하강 중 현재 손높이 통과 = 캐치(이동 중 손 추적).
                        if act.vel.y < 0.0
                            && let Some(hy) = hand_pos()
                            && ptr.translation.y <= hy.y + PROJ_R
                        {
                            info!("[action] 캐치 t={:.2}s", act.t);
                            ptr.translation = hy;
                            act.phase = Phase::Hold;
                            act.t = 0.0;
                        }
                    }
                    ActionKind::ThrowGround => {
                        // 지형 충돌 — 표면장 접촉·반발 감쇠, 약한 접촉은 정지.
                        let ground = world
                            .surface_height(ptr.translation.x, ptr.translation.z)
                            .max(world.stand_min_y())
                            + PROJ_R;
                        if ptr.translation.y <= ground && act.vel.y < 0.0 {
                            ptr.translation.y = ground;
                            if act.bounces >= 1 || act.vel.y > -1.2 {
                                act.vel = Vec3::ZERO;
                                act.phase = Phase::Rest;
                                act.t = 0.0;
                                info!("[action] 지면 정지");
                            } else {
                                act.vel.y = -act.vel.y * RESTITUTION;
                                act.vel.x *= HORIZ_DAMP;
                                act.vel.z *= HORIZ_DAMP;
                                act.bounces += 1;
                                info!("[action] 반발 v_y={:.1}m/s", act.vel.y);
                            }
                        }
                    }
                    ActionKind::Jump => {}
                    ActionKind::PlayCatch => {
                        // 수신자(마지막 소유자의 상대) 손 근처 도달 시 캐치.
                        // holder=주인 → 상대가 받고, holder=상대 → 주인이 받는다.
                        let catcher = if act.holder == Some(root_e) {
                            act.partner
                        } else {
                            Some(root_e)
                        };
                        let caught = catcher
                            .and_then(|c| tool::find_hand(c, &children_q, &names))
                            .and_then(|h| gts.get(h).ok())
                            .map(|gt| gt.translation())
                            .filter(|hp| {
                                act.vel.y < 0.0 && ptr.translation.distance(*hp) <= CATCH_R
                            });
                        if let Some(hp) = caught {
                            act.passes_left = act.passes_left.saturating_sub(1);
                            act.holder = catcher;
                            ptr.translation = hp;
                            act.phase = Phase::Hold;
                            act.t = 0.0;
                            info!(
                                "[action] 캐치볼 캐치 — 남은 패스 {}",
                                act.passes_left
                            );
                        } else if ptr.translation.y
                            <= world
                                .surface_height(ptr.translation.x, ptr.translation.z)
                                .max(world.stand_min_y())
                                + PROJ_R
                            && act.vel.y < 0.0
                        {
                            // 놓침 — 1회 반발 후 실패(같은 프레임 정지는 어색).
                            ptr.translation.y = world
                                .surface_height(ptr.translation.x, ptr.translation.z)
                                .max(world.stand_min_y())
                                + PROJ_R;
                            if act.bounces >= 1 {
                                info!("[action] 캐치볼 실패 — 놓침");
                                act.vel = Vec3::ZERO;
                                act.phase = Phase::Rest;
                                act.t = 0.0;
                            } else {
                                act.vel.y = -act.vel.y * RESTITUTION;
                                act.vel.x *= HORIZ_DAMP;
                                act.vel.z *= HORIZ_DAMP;
                                act.bounces += 1;
                            }
                        }
                    }
                }
            }
            Phase::Rest => {
                if act.t >= REST_T {
                    let p = act.proj;
                    finish(&mut commands, root_e, &mut act, &mut tool, p);
                }
            }
            Phase::Hold => {
                // 캐치 유지 — 물건을 쥔 손(holder)에 붙들기.
                let holder = act.holder.unwrap_or(root_e);
                if let Some(p) = act.proj {
                    if let (Ok(mut ptr), Some(hp)) =
                        (projs.get_mut(p), hand_hold(holder, &children_q, &names, &gts))
                    {
                        ptr.translation = hp;
                    }
                }
                if act.t >= HOLD_T {
                    // 캐치볼 — 남은 패스면 holder가 상대 손으로 되던지기(탄도 해).
                    if act.kind == ActionKind::PlayCatch && act.passes_left > 0 {
                        let other = if holder == root_e {
                            act.partner
                        } else {
                            Some(root_e)
                        };
                        let from = hand_hold(holder, &children_q, &names, &gts);
                        let to = other.and_then(|o| hand_hold(o, &children_q, &names, &gts));
                        if let (Some(p), Some(from), Some(to)) = (act.proj, from, to)
                            && let Ok(mut ptr) = projs.get_mut(p)
                        {
                            ptr.translation = from;
                            act.vel = ballistic(from, to, g, catch_t(from, to));
                            act.phase = Phase::Flight;
                            act.t = 0.0;
                            info!(
                                "[action] 캐치볼 되던지기 v=({:.1},{:.1},{:.1})m/s",
                                act.vel.x, act.vel.y, act.vel.z
                            );
                            continue;
                        }
                    }
                    let p = act.proj;
                    finish(&mut commands, root_e, &mut act, &mut tool, p);
                }
            }
        }
    }
}

/// 손 현재 위치(Hold 붙들기용).
fn hand_hold(
    root: Entity,
    children_q: &Query<&Children>,
    names: &Query<&Name>,
    gts: &Query<&GlobalTransform>,
) -> Option<Vec3> {
    tool::find_hand(root, children_q, names)
        .and_then(|h| gts.get(h).ok())
        .map(|gt| gt.translation())
}

/// 종료 공통 — 투사체 정리·도구 복원·컴포넌트 제거(빈손 원본은 복원 없음).
fn finish(
    commands: &mut Commands,
    root: Entity,
    act: &mut ActiveAction,
    tool: &mut HeldTool,
    proj: Option<Entity>,
) {
    if let Some(p) = proj
        && let Ok(mut ec) = commands.get_entity(p)
    {
        ec.despawn();
    }
    act.proj = None;
    if !act.saved_parts.is_empty() {
        tool.parts = std::mem::take(&mut act.saved_parts);
    }
    commands.entity(root).remove::<ActiveAction>();
}

/// 투사체 스폰 — 부속(또는 폴백 물건) 프리미티브를 자식으로 구성.
fn spawn_proj(
    owner: Entity,
    parts: &[ToolPart],
    at: Vec3,
    commands: &mut Commands,
    meshes: &mut ResMut<Assets<Mesh>>,
    mats: &mut ResMut<Assets<StandardMaterial>>,
) -> Entity {
    let root = commands
        .spawn((
            Transform::from_translation(at),
            ActionProj { owner },
        ))
        .id();
    for part in parts {
        let mesh = match part.prim {
            super::tool::ToolPrim::Cyl { r, h } => meshes.add(Cylinder::new(r, h)),
            super::tool::ToolPrim::Box { x, y, z } => meshes.add(Cuboid::new(x, y, z)),
        };
        let e = commands
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(mats.add(StandardMaterial {
                    base_color: part.color,
                    ..default()
                })),
                Transform::from_translation(part.offset),
            ))
            .id();
        commands.entity(root).add_child(e);
    }
    root
}

/// 행동 중 이동 정지 마커 — 게임 이동 시스템(live_move·wander)이 존중.
/// 행동·상대 이탈에도 잔류하지 않게 시간 만료 자정리(reap_action_locks).
#[derive(Component, Clone, Copy)]
pub struct ActionLock {
    /// 만료 시각(초 — Time::elapsed_secs 기준).
    pub expire_at: f32,
}

/// 만료된 이동 정지 정리.
pub fn reap_action_locks(time: Res<Time>, q: Query<(Entity, &ActionLock)>, mut commands: Commands) {
    let now = time.elapsed_secs();
    for (e, l) in &q {
        if now >= l.expire_at
            && let Ok(mut ec) = commands.get_entity(e)
        {
            ec.remove::<ActionLock>();
        }
    }
}

/// 손→손 탄도 해 — 체공 T 고정: v₀ = Δ/T + ½gT ŷ(수직은 중력 보정).
/// 지구 중력에서 정확히 T초 뒤 목적지에 도달하는 초기속도.
pub fn ballistic(from: Vec3, to: Vec3, g: f32, t: f32) -> Vec3 {
    let d = to - from;
    Vec3::new(d.x / t, d.y / t + 0.5 * g * t, d.z / t)
}

/// 고아 투사체 정리 — 소유 액터 철거(룩 재스폰 등) 시 잔류 방지.
pub fn reap_orphan_projs(mut commands: Commands, q: Query<(Entity, &ActionProj)>) {
    for (e, p) in &q {
        if commands.get_entity(p.owner).is_err()
            && let Ok(mut ec) = commands.get_entity(e)
        {
            ec.despawn();
        }
    }
}

/// 엔티티 파생 결정론 지터(-1..1) — 재현 가능한 토스 흔들림.
fn rand_jitter(seed: u64) -> f32 {
    let mut s = seed.wrapping_mul(0x9E3779B97F4A7C15);
    s ^= s >> 31;
    (s % 2000) as f32 / 1000.0 - 1.0
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 탄도 해 — T초 뒤 정확히 목적지(중력 적분과 상쇄). 지구 기준.
    #[test]
    fn ballistic_lands_on_target() {
        let g = 9.81;
        let (from, to) = (Vec3::ZERO, Vec3::new(2.0, 0.0, 1.0));
        let t = 0.85_f32;
        let mut v = ballistic(from, to, g, t);
        // 세미암시적 오일러(속도 갱신 후 위치) — 게임 루프와 동일 계열 적분.
        let (mut p, mut dt_sum) = (from, 0.0_f32);
        let dt = 1.0 / 480.0;
        while dt_sum < t - 1e-6 {
            let step = (t - dt_sum).min(dt);
            v.y -= g * step;
            p += v * step;
            dt_sum += step;
        }
        assert!(p.distance(to) < 0.02, "p={p:?} to={to:?}");
    }
}
