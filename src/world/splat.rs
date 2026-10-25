//! 지형 스플랫 재질 — ExtendedMaterial(StandardMaterial + 5레이어 텍스처).
//! 확장 바인딩은 100번부터(StandardMaterial 기본 0..~12와 충돌 회피).
//! 주의: uniform은 16바이트 배수만(WebGL2 규약) — globe(Vec4) 준수.

use bevy::pbr::{ExtendedMaterial, MaterialExtension, StandardMaterial};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::shader::ShaderRef;

#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct TerrainExtension {
    #[texture(100)]
    #[sampler(105)]
    pub grass: Handle<Image>,
    #[texture(101)]
    #[sampler(106)]
    pub forest: Handle<Image>,
    #[texture(102)]
    #[sampler(107)]
    pub sand: Handle<Image>,
    #[texture(103)]
    #[sampler(108)]
    pub rock: Handle<Image>,
    #[texture(104)]
    #[sampler(109)]
    pub dirt: Handle<Image>,
    /// 글로브 곡률 (x=cx, y=cz, z=R_eff, w=factor) — 버텍스 워프·프래그먼트
    /// 안개 소등 공용. 기본값은 globe::flat_vec(평면).
    #[uniform(110)]
    pub globe: Vec4,
}

impl MaterialExtension for TerrainExtension {
    fn vertex_shader() -> ShaderRef {
        // mesh.wgsl(기본 버텍스 스테이지) 완전 대체 — 곡률 워프 삽입판.
        "shaders/terrain_globe_vertex.wgsl".into()
    }
    fn fragment_shader() -> ShaderRef {
        "shaders/terrain_splat.wgsl".into()
    }
}

pub type TerrainMaterial = ExtendedMaterial<StandardMaterial, TerrainExtension>;
