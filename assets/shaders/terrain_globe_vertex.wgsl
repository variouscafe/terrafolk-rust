// 지형 글로브 버텍스 — bevy_pbr mesh.wgsl의 최소 미러 + 구면 워프(월드공간).
// MaterialExtension::vertex_shader가 기본 버텍스 스테이지를 완전 대체하므로
// 표준 경로(position/normal/uv/uv_b/color 전달)를 그대로 재현한다.
// SKINNED·MORPH_TARGETS 없음(정적 지형). factor<0.003이면 warp 블록 스킵
// — mesh.wgsl과 동일 경로(줌인 픽셀 동일).
// ⚠ 워프 수식은 src/world/globe.rs WarpFrame·water.wgsl과 동일 계약
//   (수정 시 세 곳 함께).
#import bevy_pbr::{
    forward_io::{Vertex, VertexOutput},
    mesh_functions,
    view_transformations::position_world_to_clip,
}

// x=cx, y=cz, z=R_eff, w=curvature factor (0 = 평면 패스스루)
@group(#{MATERIAL_BIND_GROUP}) @binding(110) var<uniform> globe: vec4<f32>;

// 쿼터니언 회전(Rodrigues) — q=(axis·sin(θ/2), cos(θ/2))
fn quat_rot(q: vec4<f32>, v: vec3<f32>) -> vec3<f32> {
    let t = 2.0 * cross(q.xyz, v);
    return v + q.w * t + cross(q.xyz, t);
}

// 평면 위치 → 구면 위치 (θ→0에서 항등 — 중심 직상공 포함)
fn globe_warp_pos(g: vec4<f32>, p: vec3<f32>) -> vec3<f32> {
    let d = p.xz - g.xy;
    let l = length(d);
    let u = select(vec2<f32>(1.0, 0.0), d / l, l > 1e-6);
    // θ 클램프(π−ε) — 확장 평면(해양·밑판 3×) 코너가 극을 지나 감겨
    // 같은 반경에 이중 피복→z-fight 동심원(2026-10-25 실측). 남극 모자에 흡수.
    let theta = min(l / g.z, 3.1405);
    let s = sin(theta);
    let c = cos(theta);
    let n = vec3<f32>(u.x * s, c, u.y * s);
    let base = vec3<f32>(g.x, 0.0, g.y)
        + vec3<f32>(u.x * g.z * s, g.z * (c - 1.0), u.y * g.z * s);
    return base + n * p.y;
}

// 평면 xz에서의 국소 up 회전(+Y→법선) — 노멀 회전용
fn globe_warp_quat(g: vec4<f32>, p: vec3<f32>) -> vec4<f32> {
    let d = p.xz - g.xy;
    let l = length(d);
    let u = select(vec2<f32>(1.0, 0.0), d / l, l > 1e-6);
    let theta = min(l / g.z, 3.1405);
    let axis = vec3<f32>(u.y, 0.0, -u.x);
    return vec4<f32>(axis * sin(0.5 * theta), cos(0.5 * theta));
}

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;

    // 지형 엔티티는 Transform 없이 원점 스폰(terrain.rs) — local==world.
    let world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);

#ifdef VERTEX_POSITIONS
    out.world_position = mesh_functions::mesh_position_local_to_world(
        world_from_local, vec4<f32>(vertex_no_morph.position, 1.0));
#endif
#ifdef VERTEX_NORMALS
    var n_world = mesh_functions::mesh_normal_local_to_world(
        vertex_no_morph.normal, vertex_no_morph.instance_index);
#endif

    // ── 글로브 워프(월드공간) — globe.w < 0.003이면 스킵(평면 = 기존 경로)
    if (globe.w >= 0.003) {
#ifdef VERTEX_POSITIONS
        let p = out.world_position.xyz;
        out.world_position = vec4<f32>(globe_warp_pos(globe, p), 1.0);
#endif
#ifdef VERTEX_NORMALS
        n_world = quat_rot(globe_warp_quat(globe, p), n_world);
#endif
    }

#ifdef VERTEX_NORMALS
    out.world_normal = n_world;
#endif
#ifdef VERTEX_POSITIONS
    out.position = position_world_to_clip(out.world_position.xyz);
#endif
#ifdef VERTEX_UVS_A
    out.uv = vertex_no_morph.uv;
#endif
#ifdef VERTEX_UVS_B
    out.uv_b = vertex_no_morph.uv_b;
#endif
#ifdef VERTEX_TANGENTS
    out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
        world_from_local, vertex_no_morph.tangent, vertex_no_morph.instance_index);
#endif
#ifdef VERTEX_COLORS
    out.color = vertex_no_morph.color;
#endif
#ifdef VERTEX_OUTPUT_INSTANCE_INDEX
    // 필수 패스스루 — pbr_fragment가 재질 슬롯 조회에 사용(누락 시 무음 파손)
    out.instance_index = vertex_no_morph.instance_index;
#endif
#ifdef VISIBILITY_RANGE_DITHER
    out.visibility_range_dither = mesh_functions::get_visibility_range_dither_level(
        vertex_no_morph.instance_index, world_from_local[3]);
#endif
    return out;
}
