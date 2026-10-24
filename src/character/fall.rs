//! 물리낙하 — 공간 중력과 낙하 적분(게임 무관, 2026-10-24).
//!
//! **공간 중력 설계**: 중력은 액터 각자의 상수가 아니라 **공간의 속성**이다.
//! `SpaceGravity` 리소스가 세계 단위(m/s²)로 소유하고 접지 시스템(ground)이
//! 읽어 적분한다 — 게임은 맵의 물리를 리소스 하나로 선언한다(이 게임의
//! 맵은 지구 → `SpaceGravity::EARTH` 9.81). 강수(precip_fx)는 기상 연출용
//! 종단속도 파라미터(항력 지배)를 별도로 쓴다 — 물방울 질량 문제가 아니다.
//!
//! **스냅과의 경합**(절벽 판정 이력현상): 접지 액터는 매 프레임 지지면에
//! 스냅된다(내리막 걷기 = 지지면이 프레임당 조금씩 내려감 → 즉시 추격).
//! 프레임 단차가 임계(절벽·순간이동)를 넘는 순간에만 공중으로 전환해
//! 자유낙하한다. 저프레임에서 걷기 내리맹이 오탐되지 않게 임계는
//! `max(고정 단차, 최대 스냅하강속도×dt)`로 dt 보정된다.
//!
//! **확장 포인트**: 점프는 `Fall { vy: 양수, airborne: true }` 주입으로
//! 같은 적분기 그대로 재사용(상승 → 정점 → 낙하). 착지 충격속도는
//! `landed`로 1프레임 노출되므로 연출(먼지·사운드) 훅이 붙는다.

use bevy::prelude::*;

/// 공간 중력(m/s², −Y 방향 크기) — 세계가 소유하는 중력. 게임이
/// `insert_resource`로 선언하고, 프레임워크는 미선언 시 지구로 기본값.
#[derive(Resource, Clone, Copy, Debug)]
pub struct SpaceGravity(pub f32);

impl SpaceGravity {
    /// 지구 표면 중력(9.81 m/s²) — 이 게임의 맵은 지구.
    pub const EARTH: Self = Self(9.81);
}

/// 액터 낙하 상태 — `GroundActor`에 붙이면 물리낙하에 참여한다
/// (미부착 = 종래의 하드 스냅. 점프 확장은 위 모듈 문서 참조).
#[derive(Component, Default, Debug)]
pub struct Fall {
    /// 수직 속도(m/s, +상승). 착지 시 0.
    pub vy: f32,
    /// 공중 여부 — true면 지지면을 향해 중력 적분 중.
    pub airborne: bool,
    /// 이번 프레임 착지 충격속도(|m/s|) — 다음 적분 초반에 청소.
    pub landed: Option<f32>,
}

/// 절벽 판정 최소 단차(m) — 이보다 큰 순간 하강만 낙하로 전환.
const CLIFF_MIN: f32 = 0.35;
/// 스냅 추격 허용 하강속도(m/s) — 걷기 내리막(≤ 수 m/s)은 프레임률과
/// 무관하게 스냅 유지. 임계는 `max(CLIFF_MIN, 이 값×dt)`.
const SNAP_DOWN_VMAX: f32 = 20.0;
/// 수직속도 하한(m/s) — 수치 안전클램프(지면 관통은 착지 클램프가 방지).
const VY_MIN: f32 = -40.0;

/// 한 프레임 낙하 적분 — ground_snap이 루트 y 소유권을 유지한 채 호출.
/// `y`: 현재 루트 y, `support`: 지지면 y(표면−FootLift). 반환: 새 루트 y.
pub fn integrate(y: f32, fall: &mut Fall, support: f32, g: f32, dt: f32) -> f32 {
    fall.landed = None;
    if fall.airborne {
        fall.vy = (fall.vy - g * dt).max(VY_MIN);
        let ny = y + fall.vy * dt;
        if ny <= support {
            fall.landed = Some(-fall.vy);
            fall.vy = 0.0;
            fall.airborne = false;
            return support;
        }
        return ny;
    }
    let drop = y - support;
    if drop > CLIFF_MIN.max(SNAP_DOWN_VMAX * dt) {
        fall.airborne = true;
        fall.vy = 0.0;
        return y; // 감지 프레임은 현 위치 유지 — 다음 프레임부터 적분.
    }
    support // 접지 스냅(오르막 즉시·내리막 추격)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// 지구 중력 자유낙하 — h=½gt²·충격 v=√(2gh). 2m 낙하 t≈0.639s v≈6.26m/s.
    #[test]
    fn earth_parabola() {
        let g = SpaceGravity::EARTH.0;
        let mut f = Fall {
            airborne: true,
            ..default()
        };
        let (mut y, mut t) = (2.0_f32, 0.0_f32);
        let dt = 1.0 / 240.0;
        while f.airborne && t < 3.0 {
            y = integrate(y, &mut f, 0.0, g, dt);
            t += dt;
        }
        assert_eq!(y, 0.0);
        assert!((t - (4.0 / g).sqrt()).abs() < 0.02, "t={t}");
        let impact = f.landed.expect("착지 기록");
        assert!((impact - (2.0 * g * 2.0).sqrt()).abs() < 0.3, "v={impact}");
    }

    /// 걷기 내리막(작은 프레임 단차)·오르막은 스냅 유지 — 공중 전이 없음.
    #[test]
    fn slope_stays_snapped() {
        let g = SpaceGravity::EARTH.0;
        let mut f = Fall::default();
        assert_eq!(integrate(1.0, &mut f, 0.98, g, 1.0 / 60.0), 0.98);
        assert!(!f.airborne);
        assert_eq!(integrate(0.98, &mut f, 1.05, g, 1.0 / 60.0), 1.05);
        assert!(!f.airborne);
    }

    /// 절벽(대단차)은 낙하 전환 → 포물선 하강 → 지지면에 정착.
    #[test]
    fn cliff_falls_and_lands() {
        let mut f = Fall::default();
        integrate(1.0, &mut f, -1.0, 9.81, 1.0 / 60.0);
        assert!(f.airborne);
        let (mut y, dt) = (1.0_f32, 1.0_f32 / 60.0);
        for _ in 0..120 {
            y = integrate(y, &mut f, -1.0, 9.81, dt);
            if !f.airborne {
                break;
            }
        }
        assert!(!f.airborne);
        assert_eq!(y, -1.0);
        assert!(f.landed.is_some());
    }

    /// 공중에서 지지면이 급상승(경사면에 비스듬히 낙하)해도 즉시 착지.
    #[test]
    fn lands_on_rising_slope() {
        let mut f = Fall {
            airborne: true,
            vy: -3.0,
            ..default()
        };
        let y = integrate(0.5, &mut f, 0.6, 9.81, 1.0 / 60.0);
        assert_eq!(y, 0.6);
        assert!(!f.airborne);
    }
}
