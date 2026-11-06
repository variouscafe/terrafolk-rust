//! 글로브 원거리 비용 절감(2026-11-06, B2-3) — 줌 ≥ FAR_ZOOM에서 주민은 몇 픽셀이다.
//! 룩 불변 계약: 기본·근접 줌에서는 아무것도 바뀌지 않는다(far=false면 전부 원상).
//!  · 잉크 헐 숨김(스킨 메시 드로 2배 제거) — 킬스위치 `?tfdbg=no_gl_ink`
//!  · 주민 애님 정지(throttle_offscreen_anims가 far 참조) — `no_gl_anim`
//!  · 크레이트 LOD 티어 2 배정(foot_lock·leg_ik 생략) — `no_gl_lod`
//! 킬스위치는 같은 빌드에서 전후 A/B 측정용이다.

use bevy::prelude::*;

use super::camera_rig::CameraRig;
use crate::character::proportion::FootRig;
use crate::character::ink::InkHull;
use crate::character::leg_ik::LegIkBind;
use crate::character::{Lod, LodFocus, LodRanges};

/// 이 줌 이상이면 원거리 모드(글로브 ZOOM_FLAT 4.55~ZOOM_MAX 6.0 중 상단).
pub const FAR_ZOOM: f32 = 5.5;

#[derive(Resource, Default)]
pub struct FarState(pub bool);

pub fn update_far(rig: Res<CameraRig>, mut st: ResMut<FarState>) {
    let far = rig.zoom >= FAR_ZOOM;
    if st.0 != far {
        st.0 = far;
    }
}

/// 헐 숨김/복원 — far 동안 매 프레임 새 헐도 숨기고, 해제 시 한 번 복원.
pub fn far_ink(
    st: Res<FarState>,
    mut was: Local<bool>,
    mut q: Query<&mut Visibility, With<InkHull>>,
) {
    if crate::net::dbg_gate::off("no_gl_ink") {
        return;
    }
    if st.0 {
        for mut v in &mut q {
            if *v != Visibility::Hidden {
                *v = Visibility::Hidden;
            }
        }
        *was = true;
    } else if *was {
        for mut v in &mut q {
            if *v == Visibility::Hidden {
                *v = Visibility::Inherited;
            }
        }
        *was = false;
    }
}

/// 크레이트 Lod 컴포넌트 부여(미부여=Tier 0 호환) — 발 리그 보유 액터에.
pub fn attach_lod(
    mut commands: Commands,
    q: Query<Entity, (Or<(Added<FootRig>, Added<LegIkBind>)>, Without<Lod>)>,
) {
    for e in &q {
        commands.entity(e).try_insert(Lod(0));
    }
}

/// far면 전원 Tier 2(범위 0), 아니면 전원 Tier 0(범위 최대) — 평시 동작 동일.
pub fn far_lod(
    st: Res<FarState>,
    rig: Res<CameraRig>,
    mut ranges: ResMut<LodRanges>,
    mut focus: ResMut<LodFocus>,
) {
    if crate::net::dbg_gate::off("no_gl_lod") {
        return;
    }
    let (n, m) = if st.0 { (0.0, 0.0) } else { (f32::MAX, f32::MAX) };
    if ranges.near2 != n || ranges.mid2 != m {
        ranges.near2 = n;
        ranges.mid2 = m;
    }
    let f = Vec3::new(rig.target.x, 0.0, rig.target.y);
    if focus.0 != f {
        focus.0 = f;
    }
}
