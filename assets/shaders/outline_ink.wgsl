// 치비 잉크 아웃라인 — 화면공간 일정 두께 헐(2026-11-03, 일본 애니 방향).
// 버텍스에서 법선 방향 팽창량을 뷰 깊이에 비례시켜(=화면 세로 대비 일정
// 비율) 줌·거리와 무관하게 일정 픽셀 두께의 라인을 유지한다. 상수 두께
// 헐(구 CPU 부풀림)은 줌 구간마다 굵기가 20배 변해 어딘가에선 소멸했다.
// 컬링(전면)=Material::specialize, 스키닝·모프는 bevy mesh.wgsl 패턴
// 그대로(SkinnedMesh 컴포넌트 복제로 본 추종).
#import bevy_pbr::{
    mesh_bindings::mesh,
    mesh_functions,
    skinning,
    morph::{morph_position, morph_normal, morph_tangent},
    forward_io::{Vertex, VertexOutput},
    view_transformations::position_world_to_clip,
}
#import bevy_pbr::mesh_view_bindings::view

// rgb=잉크색(#291F17 ≈ srgb 0.16/0.12/0.09) · a=목표 두께(화면 세로 비율)
@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> params: vec4<f32>;

// 깊이 1m당 가시 세로폭 = 2·tan(FOV/2). camera_rig FOV 40°와 계약 —
// FOV 바꾸면 이 값도 함께.
const VIEW_H_PER_DEPTH: f32 = 0.72794;

#ifdef MORPH_TARGETS
fn morph_vertex(vertex_in: Vertex, instance_index: u32) -> Vertex {
    var vertex = vertex_in;
    let first_vertex = mesh[instance_index].first_vertex_index;
    let vertex_index = vertex.index - first_vertex;
    let weight_count = bevy_pbr::morph::layer_count(instance_index);
    for (var i: u32 = 0u; i < weight_count; i ++) {
        let weight = bevy_pbr::morph::weight_at(i, instance_index);
        if weight == 0.0 {
            continue;
        }
        vertex.position += weight * morph_position(vertex_index, i, instance_index);
        #ifdef VERTEX_NORMALS
            vertex.normal += weight * morph_normal(vertex_index, i, instance_index);
        #endif
        #ifdef VERTEX_TANGENTS
            vertex.tangent += vec4(weight * morph_tangent(vertex_index, i, instance_index), 0.0);
        #endif
    }
    return vertex;
}
#endif

@vertex
fn vertex(vertex_no_morph: Vertex) -> VertexOutput {
    var out: VertexOutput;

    #ifdef MORPH_TARGETS
        var vertex = morph_vertex(vertex_no_morph, vertex_no_morph.instance_index);
    #else
        var vertex = vertex_no_morph;
    #endif

    let mesh_world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);

    #ifdef SKINNED
        var world_from_local = skinning::skin_model(
            vertex.joint_indices,
            vertex.joint_weights,
            vertex_no_morph.instance_index
        );
    #else
        var world_from_local = mesh_world_from_local;
    #endif

    #ifdef VERTEX_NORMALS
        #ifdef SKINNED
            out.world_normal = skinning::skin_normals(world_from_local, vertex.normal);
        #else
            out.world_normal = mesh_functions::mesh_normal_local_to_world(
                vertex.normal,
                vertex_no_morph.instance_index
            );
        #endif
    #endif

    #ifdef VERTEX_POSITIONS
        out.world_position = mesh_functions::mesh_position_local_to_world(world_from_local, vec4<f32>(vertex.position, 1.0));

        // ── 잉크 팽창: 화면공간 일정 두께 ──
        // 이 정점 깊이의 가시 세로폭 = VIEW_H_PER_DEPTH·w → 팽창량 =
        // 비율(params.a)×그 폭. 상하한(미터): 하한 8mm(근접 모발선),
        // 상한 120mm(신장 1.94m의 ~6% — 원거리에서 비율 폭주해 라인이
        // 캐릭터를 잠식하는 것 방지).
        let clip0 = view.view_proj * out.world_position;
        let d = max(clip0.w, 0.05);
        let t = clamp(params.a * VIEW_H_PER_DEPTH * d, 0.008, 0.120);
        let n = normalize(out.world_normal);
        out.world_position = vec4<f32>(out.world_position.xyz + n * t, 1.0);
        out.position = position_world_to_clip(out.world_position.xyz);
    #endif

    #ifdef VERTEX_UVS_A
        out.uv = vertex.uv;
    #endif
    #ifdef VERTEX_UVS_B
        out.uv_b = vertex.uv_b;
    #endif
    #ifdef VERTEX_TANGENTS
        out.world_tangent = mesh_functions::mesh_tangent_local_to_world(
            world_from_local,
            vertex.tangent,
            vertex_no_morph.instance_index
        );
    #endif
    #ifdef VERTEX_COLORS
        out.color = vertex.color;
    #endif
    #ifdef VERTEX_OUTPUT_INSTANCE_INDEX
        out.instance_index = vertex_no_morph.instance_index;
    #endif

    return out;
}

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    // 언릿 단색 — 조명·안개 없음: 야간·원거리에서도 잉크는 진하게 유지
    // (PBR 안개가 라인을 희석해 기본 줌에서 소멸했던 구 결함의 교정).
    return vec4<f32>(params.rgb, 1.0);
}
