// 캐릭터 툰 셰이딩 — 맵(핸드페인트 아소)과 재질감 통일(2026-10-22).
// 3밴드 양자화 조명 + 웜 라이트/쿨 섀도 이중 틴트. PBR 스펙큘러·환경
// 반사를 아예 계산하지 않아(커스텀 프래그먼트) 매트 카툰 촉감이 된다.
// 버텍스는 bevy 기본(mesh.wgsl) — SKINNED 분기가 스키닝을 처리한다.
#import bevy_pbr::{
    mesh_view_bindings::lights,
    forward_io::VertexOutput,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var t_base: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var s_base: sampler;
// 원본 GLB 재질의 baseColorFactor — 텍스처 없는 단색 부품 폴백 겸용
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> tint: vec4<f32>;

@fragment
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let base = textureSample(t_base, s_base, in.uv);
    var albedo = base.rgb * tint.rgb;
    let alpha = base.a * tint.a;

    let n = normalize(in.world_normal);

    // 태양(단일 DirectionalLight — daynight) · 밤엔 색·세기가 낮아진다
    var sun_dir = vec3<f32>(0.35, 0.8, 0.45);
    var sun_col = vec3<f32>(1.0);
    if (lights.n_directional_lights > 0u) {
        let l = lights.directional_lights[0];
        sun_dir = l.direction_to_light;
        // DirectionalLight.color.rgb×intensity — 밤 감광이 전파된다
        sun_col = l.color.rgb * l.color.a;
    }

    // 반구 램프 — 위(하늘빛)·아래(땅빛) 미세 보정으로 정면보다 입체감
    let hemi = n.y * 0.5 + 0.5;
    let nd = dot(n, sun_dir) * 0.5 + 0.5;
    // 2단 밴드(부드러운 계단 — 핸드페인트 느낌): 어두움→중간→밝음
    let t = smoothstep(0.28, 0.47, nd) * 0.55 + smoothstep(0.52, 0.72, nd) * 0.45;

    // 웜 라이트 / 쿨 섀도 — 레퍼런스 좌상단 웜키 광원 감각
    let lit_c = albedo * vec3<f32>(1.06, 1.00, 0.90) * sun_col;
    let sha_c = albedo * vec3<f32>(0.66, 0.70, 0.86);
    var col = mix(sha_c, lit_c, t);
    // 중간 밴드 하이라이트 — 테두리진 카툰보다 붓질감
    col += albedo * 0.06 * (hemi - 0.5);

    // 앰비언트(야간 최소 가시성 — daynight 앰비언트 색상 반영)
    col += albedo * lights.ambient_color.rgb * 0.30;

    return vec4<f32>(col, alpha);
}
