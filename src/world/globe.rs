//! 줌아웃 글로브 전환 — 착시 곡률(docs/globe-map-review.md 안 A, 2026-10-25 구현).
//! 시뮬레이션(AI·이동·접지·네트 동기)은 평면 좌표 그대로, 렌더만 구면으로 휸:
//! 지형·물 = 정점 셰이더(terrain_globe_vertex.wgsl·water.wgsl — 같은 수식),
//! 월드 엔티티 = 이 모듈의 CPU 트랜스폼 워프(아래 수식·셰이더와 계약 동일).
//!
//! 수식(cx,cz=맵 중심, 구 중심=(cx,−R_eff,cz)):
//!   d=p.xz−c, L=|d|, u=d/L, θ=L/R_eff
//!   n=(u.x·sinθ, cosθ, u.y·sinθ)                — 국소 법선(=엔티티 up)
//!   warp(p)=c+(u.x·R·sinθ, R·(cosθ−1), u.y·R·sinθ)+n·p.y
//!   Q=from_axis_angle((u.y,0,−u.x), θ)          — +Y→n (엔티티: rot′=Q·rot)
//! R_eff=R_planet/factor — factor<FACTOR_EPS(0.003)면 워프 스킵(줌인 픽셀 동일).

use bevy::prelude::*;

use super::camera_rig::CameraRig;
use super::data::WorldData;
use super::splat::TerrainMaterial;
use super::terrain::TerrainMesh;
use super::water::{WaterMaterial, WaterMesh};

/// 곡률 램프 시작 줌(이 값부터 휘기 시작 — 구 ZOOM_MAX·기존 최대 4.2 위)
pub const ZOOM_FLAT: f32 = 4.55;
/// 곡률 완성 줌(행성) — camera_rig::ZOOM_MAX와 정합.
pub const ZOOM_FULL: f32 = 6.0;
/// 이 값 미만 factor는 항등 경로(평면) — 줌인 화면 픽셀 동일 보장.
pub const FACTOR_EPS: f32 = 0.003;
/// 완전 곡률 때 카메라 궤도 거리(행성 반경 배수) — 지구 관전 구도(화면 높이 ~85%).
pub const ORBIT_DIST: f32 = 3.4;

/// 기본(평면) 유니폼값 — R_eff 크게(θ≈0)·factor 0.
pub fn flat_vec(half: f32) -> Vec4 {
    Vec4::new(half, half, 1e9, 0.0)
}

/// 줌 → 곡률 factor(smoothstep 램프).
pub fn factor_for(zoom: f32) -> f32 {
    let t = ((zoom - ZOOM_FLAT) / (ZOOM_FULL - ZOOM_FLAT)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

/// 행성 반경 — 물 평면(3×size)이 구를 정확히 1회 피복(θ_max=π).
/// 맵 대륙은 전반구, 나머지 반구는 원양.
pub fn planet_radius(world: &WorldData) -> f32 {
    1.5 * world.size as f32 / std::f32::consts::PI
}

/// 한 프레임의 워프 상태(불변 값) — CPU 엔티티 워프·역변환·광선 판정 공용.
#[derive(Clone, Copy, Debug)]
pub struct WarpFrame {
    pub center: Vec2,
    pub r_eff: f32,
    pub r_planet: f32,
    pub factor: f32,
}

impl WarpFrame {
    pub fn new(world: &WorldData, factor: f32) -> Self {
        let r_planet = planet_radius(world);
        WarpFrame {
            center: Vec2::splat(world.size as f32 * 0.5),
            r_eff: r_planet / factor.max(FACTOR_EPS),
            r_planet,
            factor,
        }
    }

    /// 활성 프레임 — factor<FACTOR_EPS면 None(호출부는 평면 경로 유지).
    pub fn active(world: &WorldData, zoom: f32) -> Option<Self> {
        let f = factor_for(zoom);
        (f >= FACTOR_EPS).then(|| Self::new(world, f))
    }

    /// 셰이더 유니폼 인코딩 (x=cx, y=cz, z=R_eff, w=factor).
    pub fn as_vec4(&self) -> Vec4 {
        Vec4::new(self.center.x, self.center.y, self.r_eff, self.factor)
    }

    /// 평면 θ 클램프(π−ε) — 셰이더와 동일 계약(해양 확장 코너 극 관통 방지).
    const THETA_MAX: f32 = 3.1405;

    /// 평면 위치 → 구면 위치. θ→0 극한에서 항등(중심 직상공 포함, 왕복 정확).
    pub fn warp_point(&self, p: Vec3) -> Vec3 {
        let d = p.xz() - self.center;
        let l = d.length();
        let u = if l > 1e-6 { d / l } else { Vec2::X };
        let (s, c) = Self::THETA_MAX.min(l / self.r_eff).sin_cos();
        let n = Vec3::new(u.x * s, c, u.y * s);
        let base = Vec3::new(
            self.center.x + u.x * self.r_eff * s,
            self.r_eff * (c - 1.0),
            self.center.y + u.y * self.r_eff * s,
        );
        base + n * p.y
    }

    /// 평면 xz에서의 국소 up 회전(+Y→법선) — 엔티티 자세 구성용.
    pub fn warp_quat(&self, xz: Vec2) -> Quat {
        let d = xz - self.center;
        let l = d.length();
        let u = if l > 1e-6 { d / l } else { Vec2::X };
        Quat::from_axis_angle(
            Vec3::new(u.y, 0.0, -u.x),
            Self::THETA_MAX.min(l / self.r_eff),
        )
    }

    /// 구면 위치 → 평면 위치(역변환, 폐형 — warp와 같은 프레임에서만 유효).
    pub fn unwarp_point(&self, q: Vec3) -> Vec3 {
        let m = q - Vec3::new(self.center.x, -self.r_eff, self.center.y);
        let r = m.length();
        if r < 1e-4 {
            return q;
        }
        let n = m / r;
        let h = r - self.r_eff;
        let theta = n.y.clamp(-1.0, 1.0).acos();
        if theta < 1e-4 {
            return Vec3::new(self.center.x, h, self.center.y);
        }
        let s = theta.sin();
        let u = Vec2::new(n.x / s, n.z / s);
        let l = theta * self.r_eff;
        Vec3::new(self.center.x + u.x * l, h, self.center.y + u.y * l)
    }
}

/// 직전 프레임 적용 워프 — restore가 정확한 역변환을 쓰도록 보존
/// (전환 중 factor가 매 프레임 변하므로 현재값이 아닌 적용값의 역 필요).
#[derive(Resource, Default)]
pub struct LastWarp(pub Option<WarpFrame>);

/// 이번 프레임에 워프된 루트 표시 — 다음 프레임 PreUpdate 복원 대상.
/// (워프는 멱등성이 없어 평면 원본 보존이 핵심 — 미표록 엔티티는 평면 그대로)
#[derive(Component)]
pub struct Warped;

/// 하늘 전용 루트(별 루트 등) — 곡률 제외(원점 중심 반구 배치라 휘면 왜곡).
#[derive(Component)]
pub struct GlobeExempt;

/// 프레임 시작 복원 — 워프됐던 루트를 평면 자세로 되돌린다.
/// 시뮬레이션(Update 전체)이 읽는 Transform은 항상 평면 좌표가 된다.
pub fn restore_flat(
    mut commands: Commands,
    last: Res<LastWarp>,
    mut q: Query<(Entity, &mut Transform), With<Warped>>,
) {
    let Some(frame) = last.0 else {
        return;
    };
    for (e, mut tr) in &mut q {
        let flat = frame.unwarp_point(tr.translation);
        tr.rotation = frame.warp_quat(flat.xz()).inverse() * tr.rotation;
        tr.translation = flat;
        commands.entity(e).remove::<Warped>();
    }
}

/// 프레임 끝 워프 — 루트 엔티티 전체(주민·소품·발명품·자원·건물·새·달·섬광)를
/// 구면에 올린다. 자식(메시·토on 재질·블롭 그림자·등)은 전파로 자동 추종.
/// 제외: 카메라·UI 노드·지형/물 메시(셰이더 담당)·태양 평행광·하늘 면제.
pub fn apply_warp(
    mut commands: Commands,
    mut last: ResMut<LastWarp>,
    world: Res<WorldData>,
    rig: Res<CameraRig>,
    mut q: Query<
        (Entity, &mut Transform),
        (
            Without<ChildOf>,
            Without<Camera>,
            Without<Node>,
            Without<TerrainMesh>,
            Without<WaterMesh>,
            Without<DirectionalLight>,
            Without<GlobeExempt>,
        ),
    >,
) {
    let Some(frame) = WarpFrame::active(&world, rig.zoom) else {
        last.0 = None;
        return;
    };
    last.0 = Some(frame);
    for (e, mut tr) in &mut q {
        let flat = tr.translation; // restore(PreUpdate) 이후라 항상 평면
        tr.translation = frame.warp_point(flat);
        tr.rotation = frame.warp_quat(flat.xz()) * tr.rotation;
        commands.entity(e).try_insert(Warped);
    }
}

/// 셰이더 유니폼 반영 — 지형·물 재질의 globe Vec4 기입(값 변화 시에만 써서
/// 바인드그룹 재생성 최소화. 패턴: water.rs water_night_dim).
pub fn sync_globe_uniforms(
    world: Res<WorldData>,
    rig: Res<CameraRig>,
    mut terrain: ResMut<Assets<TerrainMaterial>>,
    mut water: ResMut<Assets<WaterMaterial>>,
) {
    let v = WarpFrame::active(&world, rig.zoom)
        .map(|f| f.as_vec4())
        .unwrap_or_else(|| flat_vec(world.size as f32 * 0.5));
    for (_, m) in terrain.iter_mut() {
        if m.extension.globe != v {
            m.extension.globe = v;
        }
    }
    for (_, m) in water.iter_mut() {
        if m.globe != v {
            m.globe = v;
        }
    }
}
