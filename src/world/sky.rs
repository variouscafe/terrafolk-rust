//! 페인티드 하늘 돔(2026-10-30 10차) — 지브리풍 카툰 하늘. 천정→지평선
//! 그라디언트 + fbm 클럼프 구름 뭉치(물 포말 뭉치 기법 계승 — 큰 덩어리
//! 실루엣 + 내부 2톤). 카메라를 따라다니는 반지름 390 돔(내부면 렌더 —
//! 컬링 끔·깊기 기록 없음), ClearColor는 돔 하단 밖 폴백으로 잔존.
//! 밤엔 구름 소멸(별·달 메시가 돔 위에 그려짐 — 돔은 깊기를 안 쓰므로
//! 가까운 메시가 항상 이긴다).

use bevy::pbr::{MaterialPipeline, MaterialPipelineKey};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;

/// 하늘 유니폼 원천 — daynight::apply_sky가 시각·날씨로 매 프레임 갱신.
#[derive(Resource, Debug, Clone)]
pub struct SkyParams {
    pub top: Vec3,
    pub horizon: Vec3,
    pub sun_dir: Vec3,
    /// 구름 양(0=맑음 · 1=만개)
    pub cloud: f32,
}

impl Default for SkyParams {
    fn default() -> Self {
        Self {
            top: Vec3::new(0.72, 0.82, 0.88),
            horizon: Vec3::new(0.86, 0.88, 0.84),
            sun_dir: Vec3::new(0.35, 0.8, 0.45).normalize_or_zero(),
            cloud: 0.8,
        }
    }
}

/// 돔 재질 — 유니폼 3개(top/horizon/sun, WebGL2 프래그먼트 UBO 한도 내).
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct SkyMaterial {
    /// rgb=천정색 · a=구름 양
    #[uniform(0)]
    pub top: Vec4,
    /// rgb=지평선색 · a=예비
    #[uniform(1)]
    pub horizon: Vec4,
    /// xyz=태양 방향(단위벡터) · w=예비
    #[uniform(2)]
    pub sun: Vec4,
}

impl Material for SkyMaterial {
    fn fragment_shader() -> bevy::shader::ShaderRef {
        "shaders/sky.wgsl".into()
    }
    fn specialize(
        _pipeline: &MaterialPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: MaterialPipelineKey<Self>,
    ) -> Result<(), bevy::render::render_resource::SpecializedMeshPipelineError> {
        // 내부에서 보는 돔 — 백페이스 컬링 끔. 깊이 기록 없음(항상 배경).
        descriptor.primitive.cull_mode = None;
        if let Some(ds) = descriptor.depth_stencil.as_mut() {
            ds.depth_write_enabled = Some(false);
        }
        Ok(())
    }
}

#[derive(Component)]
struct SkyDome(Handle<SkyMaterial>);

const DOME_R: f32 = 390.0; // 카메라 far 420 안 — 항상 전경 너머

fn sky_setup(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<SkyMaterial>>,
    params: Res<SkyParams>,
) {
    if crate::net::dbg_gate::off("no_sky") {
        return;
    }
    let mat = mats.add(SkyMaterial {
        top: params.top.extend(params.cloud),
        horizon: params.horizon.extend(0.0),
        sun: params.sun_dir.extend(0.0),
    });
    commands.spawn((
        Mesh3d(meshes.add(Sphere::new(DOME_R))),
        MeshMaterial3d(mat.clone()),
        SkyDome(mat),
        Transform::default(),
    ));
}

/// 유니폼 전파 + 카메라 추적(스카이박스 계약 — 돔 중심 = 카메라 위치).
fn sky_update(
    params: Res<SkyParams>,
    cams: Query<&GlobalTransform, With<Camera3d>>,
    mut domes: Query<(&mut Transform, &SkyDome)>,
    mut mats: ResMut<Assets<SkyMaterial>>,
) {
    if let Ok((_, dome)) = domes.single() {
        if let Some(mut m) = mats.get_mut(&dome.0) {
            m.top = params.top.extend(params.cloud);
            m.horizon = params.horizon.extend(0.0);
            m.sun = params.sun_dir.extend(0.0);
        }
    }
    if let (Ok(cam), Ok((mut t, _))) = (cams.single(), domes.single_mut()) {
        t.translation = cam.translation();
    }
}

pub fn plugin(app: &mut App) {
    app.init_resource::<SkyParams>()
        .init_asset::<SkyMaterial>()
        .add_plugins(bevy::pbr::MaterialPlugin::<SkyMaterial>::default())
        .add_systems(Startup, sky_setup)
        .add_systems(Update, sky_update);
}
