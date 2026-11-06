//! 글로브 정적 소품 워프 캐시(2026-11-06, docs/globe-static-warp-workorder.md).
//! 움직이지 않는 `PropRoot`(나무·바위·건물 등)는 매 프레임 "평면→구면" 왕복 쓰기를 하지 않는다.
//! 평면 정본은 `GlobeStatic.flat`에 두고, 워프 프레임이 비트 단위로 같으면 Transform을 건드리지
//! 않아 bevy 전파가 이 트리들을 건너뛴다. 줌 애니메이션 중에는 프레임이 매번 달라 기존과 같은 비용.
//! W2 예외: 이 소품은 Update 구간에도 구면 Transform이다 — 평면 소비자는 `flat_pos`/`flat_tf`를 쓴다.
//! 킬스위치 `?tfdbg=no_globe_static`(삽입 안 함 → 기존 경로와 동일).

use bevy::prelude::*;

use super::camera_rig::CameraRig;
use super::data::{PropKind, WorldData};
use super::globe::WarpFrame;
use super::models::PropRoot;
use super::prop_fit::Fitted;
use super::propsfx::FloatBob;
use crate::net::resources::RoamTo;

pub fn enabled() -> bool {
    !crate::net::dbg_gate::off("no_globe_static")
}

/// 워프 프레임의 비트 키 — 같을 때만 재쓰기를 생략한다.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct WarpKey([u32; 4]);

impl WarpKey {
    pub fn of(f: &WarpFrame) -> Self {
        WarpKey([
            f.center.x.to_bits(),
            f.center.y.to_bits(),
            f.r_eff.to_bits(),
            f.factor.to_bits(),
        ])
    }
}

/// 정적 소품 표식 — `flat`이 평면 정본, `applied`는 현재 Transform에 반영된 워프(None=평면).
#[derive(Component)]
pub struct GlobeStatic {
    pub flat: Transform,
    pub applied: Option<WarpKey>,
}

/// 평면 위치 — 정적 소품이면 정본, 아니면 현재 Transform(이미 평면).
pub fn flat_pos(tr: &Transform, st: Option<&GlobeStatic>) -> Vec3 {
    st.map_or(tr.translation, |s| s.flat.translation)
}

/// 평면 Transform 전체(깃발 로컬 변환 등).
pub fn flat_tf(tr: &Transform, st: Option<&GlobeStatic>) -> Transform {
    st.map_or(*tr, |s| s.flat)
}

fn is_animal(k: PropKind) -> bool {
    matches!(
        k,
        PropKind::AnimalBoar
            | PropKind::AnimalDeer
            | PropKind::AnimalWaterDeer
            | PropKind::AnimalSquirrel
            | PropKind::AnimalPheasant
    )
}

/// 핏 완료된 정적 소품에 GlobeStatic 부착. PostUpdate, warp 계열보다 앞(자동 싱크 포인트) —
/// 이 시점 Transform은 평면 최종값(restore 이후·워프 이전)이다.
#[allow(clippy::type_complexity)] // Bevy 쿼리 필터 조합
pub fn tag_static(
    mut commands: Commands,
    q: Query<
        (Entity, &Transform, &PropRoot),
        (
            With<Fitted>,
            Without<GlobeStatic>,
            Without<ChildOf>,
            Without<RoamTo>,
            Without<FloatBob>,
        ),
    >,
) {
    for (e, tr, prop) in &q {
        if is_animal(prop.kind) {
            continue; // 라이브 동물은 RoamTo 이동 — 동적 경로 유지
        }
        commands.entity(e).try_insert(GlobeStatic {
            flat: *tr,
            applied: None,
        });
    }
}

/// 정적 소품 워프 — 프레임 키가 적용값과 같으면 아무것도 쓰지 않는다(변경 감지 미발화).
pub fn warp_static(
    world: Res<WorldData>,
    rig: Res<CameraRig>,
    mut q: Query<(&mut Transform, &mut GlobeStatic)>,
) {
    let frame = WarpFrame::active(&world, rig.zoom);
    let key = frame.as_ref().map(WarpKey::of);
    for (mut tr, mut st) in &mut q {
        if st.applied == key {
            continue; // Deref(읽기)만 — Mut 변경 표시 없음
        }
        let flat = st.flat;
        *tr = match &frame {
            Some(f) => Transform {
                translation: f.warp_point(flat.translation),
                rotation: f.warp_quat(flat.translation.xz()) * flat.rotation,
                scale: flat.scale,
            },
            None => flat,
        };
        st.applied = key;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flat_pos_prefers_flat_original() {
        let sph = Transform::from_translation(Vec3::new(9.0, 9.0, 9.0));
        let st = GlobeStatic {
            flat: Transform::from_translation(Vec3::new(1.0, 2.0, 3.0)),
            applied: None,
        };
        assert_eq!(flat_pos(&sph, Some(&st)), Vec3::new(1.0, 2.0, 3.0));
        assert_eq!(flat_pos(&sph, None), Vec3::new(9.0, 9.0, 9.0));
    }

    #[test]
    fn warp_key_bitwise_equal_only() {
        let f = |z: f32| WarpFrame {
            center: Vec2::splat(64.0),
            r_eff: 100.0,
            r_planet: 80.0,
            factor: z,
        };
        assert_eq!(WarpKey::of(&f(0.5)), WarpKey::of(&f(0.5)));
        assert_ne!(WarpKey::of(&f(0.5)), WarpKey::of(&f(0.5001)));
    }
}
