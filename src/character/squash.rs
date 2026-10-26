//! 착지 임팩트 스쿼시 — `Fall::landed`(충격속도, 1프레임 노출)를 소비해
//! 루트 스케일 눌림·회복(연출 물리 잔향). ground_snap이 같은 프레임에
//! 착지를 기록하고 다음 적분 초반에 청소하므로 본 시스템은
//! `GroundSnapSet` **이후**에 실행해 노출 프레임을 놓치지 않는다.
//!
//! 루트 스케일의 원 소유자는 게임 핏(스폰 시 1회) — 트리거 순간 기준을
//! 스냅샷해 정확히 복원한다(ground.rs가 루트 y만 쓰는 단일 소유 원칙과
//! 같은 계약). 눌림은 충격속도 비례(점프 약하게·절벽 낙하 크게), 회복은
//! ease-out cubic — 임팩트는 즉시, 되돌음은 무게감 있게.

use bevy::prelude::*;

use super::fall::Fall;

/// 스쿼시 트리거 최소 충격속도(m/s) — 가벼운 걸음착지는 무시. 점프 착지
/// (vy≈3.2)는 걸리고, CLIFF 낙하(≥4)는 더 깊게.
const TRIGGER_V: f32 = 2.6;
/// 기준 눌림 비율(충격 4m/s 기준)·최대 눌림.
const DIP_BASE: f32 = 0.12;
const DIP_MAX: f32 = 0.20;
/// 회복 길이(s) — 임팩트 즉시 눌림 → ease-out 복원.
const RECOVER_S: f32 = 0.24;
/// 수평 불룩 계수 — 부피 보존 근사(눌림의 60% 만큼 옆으로 퍼짐).
const BULGE: f32 = 0.6;

/// 착지 스쿼시 진행 상태 — 트리거 시 부착, 복원 완료 시 제거.
#[derive(Component)]
pub struct LandSquash {
    /// 기준 루트 스케일(트리거 순간 스냅샷 — 정확 복원).
    base: Vec3,
    /// 눌림 비율(0..=DIP_MAX).
    dip: f32,
    t: f32,
}

/// 눌림 깊이 — 충격속도 비례(4m/s 기준, 상한 클램프).
fn dip_for(v: f32) -> f32 {
    (DIP_BASE * v / 4.0).min(DIP_MAX)
}

/// 진행도 k(0..1)의 눌림 잔량 — ease-out cubic: k=0 최대 눌림, k=1 완전 복원.
fn dip_at(dip: f32, k: f32) -> f32 {
    let eased = 1.0 - (1.0 - k).powi(3);
    dip * (1.0 - eased)
}

/// 스쿼시 스케일 — 눌림 d에서 y 축 소축·x/z 축 불룩.
fn squashed(base: Vec3, d: f32) -> Vec3 {
    Vec3::new(
        base.x * (1.0 + d * BULGE),
        base.y * (1.0 - d),
        base.z * (1.0 + d * BULGE),
    )
}

/// 착지 스쿼시 — landed 소비(부착) + 진행 액터 복원. 두 쿼리는 LandSquash
/// 유무로 상호 배타.
pub fn land_squash(
    time: Res<Time>,
    mut commands: Commands,
    q_land: Query<(Entity, &Transform, &Fall), (With<Fall>, Without<LandSquash>)>,
    mut q_run: Query<(Entity, &mut Transform, &mut LandSquash)>,
) {
    for (e, tr, f) in &q_land {
        if let Some(v) = f.landed.filter(|v| *v >= TRIGGER_V) {
            commands.entity(e).insert(LandSquash {
                base: tr.scale,
                dip: dip_for(v),
                t: 0.0,
            });
        }
    }
    for (e, mut tr, mut sq) in &mut q_run {
        sq.t += time.delta_secs();
        let k = (sq.t / RECOVER_S).clamp(0.0, 1.0);
        if k >= 1.0 {
            tr.scale = sq.base;
            if let Ok(mut ec) = commands.get_entity(e) {
                ec.remove::<LandSquash>();
            }
            continue;
        }
        tr.scale = squashed(sq.base, dip_at(sq.dip, k));
    }
}

/// 시각 곡선 단위 테스트 — 임팩트 즉시 최대·선형 아닌 복원·완전 수렴.
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dip_curve() {
        let d = dip_for(4.0);
        assert!((d - DIP_BASE).abs() < 1e-6);
        assert_eq!(dip_for(99.0), DIP_MAX, "상한 클램프");
        assert!((dip_at(d, 0.0) - d).abs() < 1e-6, "임팩트 프레임 최대 눌림");
        assert!(dip_at(d, 0.25) < d * 0.7, "초반 급복원(ease-out)");
        assert!(dip_at(d, 0.9) < d * 0.02, "후반 미세 수렴");
        assert_eq!(dip_at(d, 1.0), 0.0, "완전 복원");
    }

    #[test]
    fn squashed_shape() {
        let base = Vec3::splat(2.0);
        let s = squashed(base, 0.1);
        assert!(s.y < base.y && s.x > base.x && s.z > base.z, "y소축·xz불룩");
        assert!((s.x - 2.0 * 1.06).abs() < 1e-6 && (s.y - 2.0 * 0.9).abs() < 1e-6);
    }
}
