//! 글로브 줌아웃 소형 소품 숨김(성능, 2026-11-06 작업 지시서 globe-small-prop-cull).
//! 최대 줌아웃에서 가시 메시 수가 렌더 비용을 지배한다 → `PropKind`가 아니라
//! 모델 `target_h` 임계 2개로 소형 소품 `PropRoot` 루트의 Visibility를 끈다.
//! 단계가 바뀐 프레임과 새로 스폰된 소품에만 쓴다(매 프레임 전수 쓰기 금지).
//! 킬스위치 `?tfdbg=no_globe_cull`, S단계만 `?tfdbg=globe_cull_s_only`.

use bevy::prelude::*;

use super::camera_rig::CameraRig;
use super::data::PropKind;
use super::models::PropRoot;

/// S단계(소) 대상 상한 target_h — 진입/복귀 줌(히스테리시스).
pub const CULL_S_MAX_H: f32 = 0.7;
pub const S_ENTER: f32 = 5.0;
pub const S_EXIT: f32 = 4.9;
/// M단계(중) 대상 상한 target_h.
pub const CULL_M_MAX_H: f32 = 1.2;
pub const M_ENTER: f32 = 5.5;
pub const M_EXIT: f32 = 5.4;

/// 현재 단계 0(없음)/1(S)/2(S+M).
#[derive(Resource, Default)]
pub struct CullStage(pub u8);

pub fn enabled() -> bool {
    !crate::net::dbg_gate::off("no_globe_cull")
}

fn max_stage() -> u8 {
    if crate::net::dbg_gate::on("globe_cull_s_only") {
        1
    } else {
        2
    }
}

/// 이 종류가 `stage`에서 숨김 대상인가 — 모닥불·유물 제외, target_h 임계로 판정.
pub fn hides(kind: PropKind, target_h: f32, stage: u8) -> bool {
    if stage == 0 || matches!(kind, PropKind::Campfire | PropKind::Relic) {
        return false;
    }
    target_h < CULL_S_MAX_H || (stage >= 2 && target_h < CULL_M_MAX_H)
}

fn next_stage(cur: u8, zoom: f32, max: u8) -> u8 {
    let mut s = cur;
    if s >= 1 && zoom < S_EXIT {
        s = 0;
    } else if s == 0 && zoom >= S_ENTER {
        s = 1;
    }
    if max >= 2 {
        if s >= 2 && zoom < M_EXIT {
            s = 1;
        } else if s == 1 && zoom >= M_ENTER {
            s = 2;
        }
    } else {
        s = s.min(1);
    }
    s
}

fn vis(hidden: bool) -> Visibility {
    if hidden {
        Visibility::Hidden
    } else {
        Visibility::Inherited
    }
}

/// 단계 갱신 + 전환 프레임 전수 적용 + 신규 소품 적용.
pub fn globe_cull(
    rig: Res<CameraRig>,
    mut stage: ResMut<CullStage>,
    q_all: Query<(Entity, &PropRoot)>,
    q_new: Query<(Entity, &PropRoot), Added<PropRoot>>,
    mut commands: Commands,
) {
    let next = next_stage(stage.0, rig.zoom, max_stage());
    if next != stage.0 {
        let (mut n_s, mut n_m) = (0u32, 0u32);
        for (e, p) in &q_all {
            let was = hides(p.kind, p.target_h, stage.0);
            let now = hides(p.kind, p.target_h, next);
            if was != now {
                commands.entity(e).insert(vis(now));
            }
            if hides(p.kind, p.target_h, 1) {
                n_s += 1;
            } else if hides(p.kind, p.target_h, 2) {
                n_m += 1;
            }
        }
        info!(
            "[globe_cull] stage {} -> {} (zoom {:.2}) S={} M={}",
            stage.0, next, rig.zoom, n_s, n_m
        );
        stage.0 = next;
        return; // 신규 소품도 위 순회에 포함(Added는 같은 프레임 q_all에 있다)
    }
    if stage.0 > 0 {
        for (e, p) in &q_new {
            if hides(p.kind, p.target_h, stage.0) {
                commands.entity(e).insert(Visibility::Hidden);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hysteresis() {
        assert_eq!(next_stage(0, 4.95, 2), 0);
        assert_eq!(next_stage(0, 5.0, 2), 1);
        assert_eq!(next_stage(1, 4.95, 2), 1);
        assert_eq!(next_stage(1, 4.89, 2), 0);
        assert_eq!(next_stage(1, 5.5, 2), 2);
        assert_eq!(next_stage(2, 5.45, 2), 2);
        assert_eq!(next_stage(2, 5.39, 2), 1);
        assert_eq!(next_stage(0, 6.0, 1), 1);
    }

    #[test]
    fn exclusions() {
        assert!(!hides(PropKind::Campfire, 0.3, 2));
        assert!(!hides(PropKind::Relic, 0.3, 2));
        assert!(hides(PropKind::GrassClump, 0.55, 1));
        assert!(!hides(PropKind::WoodLog, 0.9, 1));
        assert!(hides(PropKind::WoodLog, 0.9, 2));
        assert!(!hides(PropKind::TreePine, 7.2, 2));
    }
}
