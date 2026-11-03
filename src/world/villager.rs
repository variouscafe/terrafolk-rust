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
/// 주민 fit 목표(미터) — **치비화 이전 raw 기준치**(오토폿이 chibiify 전
/// AABB를 재므로). 가시 신장 1.72m 유지값 — 세계 물체 대비 가시 신장은
/// REAL 시대(1.72)와 동일, 카메라·오버레이 보정 불변.
/// 이력: CHIBI_REF(head 2.20) 1.94 → 2026-11-03 CHIBI_REF2(head 2.60·
/// leg 0.44)로 가시 신장 ×1.085(qchar 5종 LBS 실측 평균, 망토 모델
/// m-adventurer 제외) → 1.94/1.085 = 1.79.
pub const VILLAGER_H: f32 = 1.79;

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

/// 시간 기반 자유 난수(배회 휴식 길이) — 결정론 대상 아님(연출).
fn rand_free(seed: u64, scale: f32) -> f32 {
    super::data::Rng::new(seed.wrapping_mul(2654435761)).f32() * scale
}

/// 데모 휴식 활동 풀 — act.* 어휘에 있는 코드만(UI 라벨·애님 매핑 보장).
const DEMO_ACTS: [&str; 7] = ["gather", "chop", "forage", "craft", "drink", "give", "eat"];

pub fn wander(
    time: Res<Time>,
    world: Res<WorldData>,
    mut q: Query<
        (&mut Villager, &mut Transform),
        (
            Without<crate::net::apply::LiveAgentTag>,
            // 행동 중(캐치볼 등) 정지 — 서버 배회와 행동 연출의 경합 방지.
            Without<crate::character::ActionLock>,
        ),
    >,
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
                // 이동 방향으로 몸 돌리기 — 지수 감쇠 스무딩(이동 개시의
                // 하드 스냅을 매 프레임 수렴으로 — 이징 디테일 2026-10-26).
                let d = v.to - v.from;
                if d.length_squared() > 1e-6 {
                    crate::character::action::turn_yaw(
                        &mut tr,
                        d.x.atan2(d.z) + MODEL_YAW,
                        time.delta_secs(),
                    );
                }
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
                // 회전은 이동계(mover)가 방향으로 지수 수렴시킨다(하드 스냅 폐지).
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

/// 유휴 몸동작(피젯) — 한자리에 가만히 서 있는 주민은 주기적으로
/// 짧은 몸동작(두리번·기지개)을 섞는다. 순수 Idle 클립의 무한 루프는
/// 주민을 조형품처럼 보이게 한다(생명감 직결 — 2026-10-26 디테일 강화).
/// 길이는 클립별 자연 길이차(기지개>두리번)+개체 랜덤, 교차 선택으로
/// 같은 동작 반복을 막는다.
#[derive(Component)]
pub struct IdleFidget {
    /// 다음 피젯까지 남은 시간(s) — 순수 Idle일 때만 감소.
    next_in: f32,
    /// 피젯 애님 유지 종료 시각(s — elapsed 기준).
    until: f32,
    /// 교차 선택(Wave↔Interact 번갈아 — 같은 동작 반복 방지).
    wave: bool,
}

impl IdleFidget {
    pub fn new(seed: u64) -> Self {
        Self {
            next_in: 3.0 + rand_free(seed, 4.0),
            until: 0.0,
            wave: rand_free(seed.wrapping_add(3), 1.0) < 0.5,
        }
    }
}

/// 애님 상태 전환(플레이어는 하위 엔티티에 있음) — 이동 우선, 정지 시
/// 활동 코드(act 우선·status 폴백)로 Walk/Idle/Interact/Wave를 고른다.
/// 지시 행동(ActiveAction) 수행 중엔 프레임워크 요청 애님(던지기 모션 등)을,
/// PoseHold(캐치볼 수신 자세 등 연출 홀드)를 활동보다 우선한다. 만료된
/// 홀드는 여기서 정리(부착은 게임 — anim.rs PoseHold 계약). 0.25초
/// 크로스페이드 — 하드 컷은 주민을 기계적으로 보이게 한다(생명감 직결).
pub fn animate_states(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(
        Entity,
        &Villager,
        &mut VillagerAnims,
        &mut crate::character::AnimState,
        Option<&mut IdleFidget>,
        Option<&crate::character::ActiveAction>,
        Option<&crate::character::PoseHold>,
    )>,
    mut players: Query<(&mut AnimationPlayer, &mut AnimationTransitions)>,
) {
    let now = time.elapsed_secs();
    for (e, v, mut anims, mut state, fidget, action, pose) in &mut q {
        // 만료 포즈 홀드 정리 — 소비처 책임(anim.rs 계약).
        if pose.is_some_and(|p| now >= p.until) {
            if let Ok(mut ec) = commands.get_entity(e) {
                ec.remove::<crate::character::PoseHold>();
            }
        }
        let want = if v.moving {
            AnimKind::Walk
        } else {
            act_kind(if v.act.is_empty() { &v.status } else { &v.act })
        };
        // 우선순위 — 행동 상태머신 > 포즈 홀드 > 활동 코드.
        let held = pose.filter(|p| now < p.until).map(|p| p.anim);
        let want = match action {
            Some(a) => a.anim(), // 행동 중 오버라이드(준비 모션·복원)
            None => held.unwrap_or(want),
        };
        // 유휴 피젯 — 행동·이동·활동 중엔 타이머만 리셋(끼어들지 않는다).
        let want = if action.is_none() && want == AnimKind::Idle {
            match fidget {
                Some(mut fg) => {
                    if now >= fg.until {
                        fg.next_in -= time.delta_secs();
                        if fg.next_in <= 0.0 {
                            // 몸동작 길이 다양성 — 클립별 자연 길이차(기지개는
                            // 길게 두리번은 짧게)+개체 랜덤(이징 디테일 2026-10-26).
                            fg.wave = !fg.wave;
                            let len = if fg.wave {
                                0.85 + rand_free((now * 991.0) as u64, 0.45)
                            } else {
                                0.55 + rand_free((now * 991.0) as u64, 0.35)
                            };
                            fg.until = now + len;
                            fg.next_in = 4.0 + rand_free((now * 997.0) as u64, 5.0);
                        }
                    }
                    if now < fg.until {
                        if fg.wave { AnimKind::Wave } else { AnimKind::Interact }
                    } else {
                        want
                    }
                }
                None => want,
            }
        } else {
            want
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
