// 지형 스플랫 — 5레이어 페인티드 블렌딩(ExtendedMaterial 확장).
// 버텍스 COLOR.rgb = (grass, forest, sand) 가중치 · UV_B.xy = (rock, dirt).
// uv = 세계좌표×UV_SCALE(코너 정합 — 타일 시임 없음).
#import bevy_pbr::{
    pbr_fragment::pbr_input_from_standard_material,
    pbr_functions::alpha_discard,
}
#ifdef PREPASS_PIPELINE
#import bevy_pbr::{
    prepass_io::{VertexOutput, FragmentOutput},
    pbr_deferred_functions::deferred_output,
}
#else
#import bevy_pbr::{
    forward_io::{VertexOutput, FragmentOutput},
    pbr_functions::{apply_pbr_lighting, main_pass_post_lighting_processing},
    mesh_view_bindings::view,
}
#endif

@group(#{MATERIAL_BIND_GROUP}) @binding(100) var t_grass: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(101) var t_forest: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(102) var t_sand: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(103) var t_rock: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(104) var t_dirt: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(105) var s_grass: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(106) var s_forest: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(107) var s_sand: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(108) var s_rock: sampler;
@group(#{MATERIAL_BIND_GROUP}) @binding(109) var s_dirt: sampler;
// 글로브 곡률 — terrain_globe_vertex.wgsl과 동일 바인딩(프래그먼트에선
// 안개 소등용 w=factor만 사용). 곡률 구간의 수평면 근사 안개는 끈다.
@group(#{MATERIAL_BIND_GROUP}) @binding(110) var<uniform> globe: vec4<f32>;

fn hash21(p: vec2<f32>) -> f32 {
    var q = fract(p * vec2<f32>(127.1, 311.7));
    q *= (17.0 + dot(q, q * 43.0));
    return fract(q.x * q.y * 95.4337);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(
        mix(hash21(i), hash21(i + vec2<f32>(1.0, 0.0)), u.x),
        mix(hash21(i + vec2<f32>(0.0, 1.0)), hash21(i + vec2<f32>(1.0, 1.0)), u.x),
        u.y);
}

@fragment
fn fragment(
    in: VertexOutput,
    @builtin(front_facing) is_front: bool,
) -> FragmentOutput {
    var pbr_input = pbr_input_from_standard_material(in, is_front);

    var w = vec3<f32>(max(pbr_input.material.base_color.rgb, vec3<f32>(0.0)));
#ifdef VERTEX_UVS_B
    var w2 = vec2<f32>(max(in.uv_b, vec2<f32>(0.0)));
#else
    var w2 = vec2<f32>(0.0, 1.0);
#endif
    let total = w.x + w.y + w.z + w2.x + w2.y + 1e-5;
    w = w / total;
    w2 = w2 / total;

    let uv = in.uv;
    // ── 지브리톤 레이어 틴트(2026-10-30 해안 마을 정본 실측) — 원본 타일
    //    (assetgen) 불변, 렌더 계층에서 재조준. 핵심 = 루마 보존 색조 시프트:
    //    텍스처의 명암 구조는 살리고 채도만 올리브/골드 팔레트로 눌러
    //    페인티드 패치 느낌(무아웃라인·가치대비 조형). 순수 채널 게인은
    //    전역 그레이딩(채도 +0.12)과 프레이밍별 조도 편차에 휘둘려 목표에서
    //    이탈했다(실측 2회 교차 검증).
    let g_l = dot(textureSample(t_grass, s_grass, uv).rgb, vec3<f32>(0.3, 0.59, 0.11));
    let grass_c = mix(
        textureSample(t_grass, s_grass, uv).rgb * vec3<f32>(0.88, 0.94, 1.15),
        vec3<f32>(g_l) * vec3<f32>(0.95, 1.02, 0.80), 0.55);
    let f_l = dot(textureSample(t_forest, s_forest, uv).rgb, vec3<f32>(0.3, 0.59, 0.11));
    let forest_c = mix(
        textureSample(t_forest, s_forest, uv).rgb * vec3<f32>(0.82, 0.86, 1.05),
        vec3<f32>(f_l) * vec3<f32>(0.80, 0.92, 0.72), 0.65);
    let s_l = dot(textureSample(t_sand, s_sand, uv).rgb, vec3<f32>(0.3, 0.59, 0.11));
    let sand_c = mix(
        textureSample(t_sand, s_sand, uv).rgb * vec3<f32>(1.05, 0.96, 0.86),
        vec3<f32>(s_l) * vec3<f32>(1.12, 0.99, 0.80), 0.50);
    var col = w.x * grass_c
            + w.y * forest_c
            + w.z * sand_c
            + w2.x * textureSample(t_rock, s_rock, uv).rgb
            + w2.y * textureSample(t_dirt, s_dirt, uv).rgb;

    // ── 경사 절벽 — 급경사(노멀.y 낮음)에 암석+흙 줄무늬 혼합·살짝 어둡게(AO감).
    //    유니폼 없이 상수 임계(WebGL2 12유니폼 한도 준수).
    let nrm = normalize(in.world_normal);
    let slope = smoothstep(0.90, 0.62, nrm.y); // 0=평지, 1=가파름
    if slope > 0.001 {
        let rock_c = textureSample(t_rock, s_rock, uv).rgb;
        let dirt_c = textureSample(t_dirt, s_dirt, uv).rgb;
        col = mix(col, rock_c, slope * 0.88);
        col = mix(col, dirt_c, slope * slope * 0.30);
        col *= 1.0 - slope * 0.10;
    }

    // 거시 변조 — 세계좌표 노이즈로 온도/명도 미세 왕복(붓 화법 패치 느낌)
    let macro1 = vnoise(in.world_position.xz * 0.16);
    let macro2 = vnoise(in.world_position.xz * 0.045 + vec2<f32>(37.2, 11.8));
    let warmth = (macro1 - 0.5) * 0.07;
    col *= vec3<f32>(1.0 + warmth, 1.0 + (macro2 - 0.5) * 0.05, 1.0 - warmth * 0.8);
    col *= 0.96 + macro2 * 0.08;

    pbr_input.material.base_color = vec4<f32>(col, 1.0);
    pbr_input.material.base_color = alpha_discard(pbr_input.material, pbr_input.material.base_color);

#ifdef PREPASS_PIPELINE
    let out = deferred_output(in, pbr_input);
#else
    var out: FragmentOutput;
    out.color = apply_pbr_lighting(pbr_input);
    out.color = main_pass_post_lighting_processing(pbr_input, out.color);
    // 7차 지형 루마 밴딩은 철회(2026-10-30 사용자 보고 '테두리 선 이상') —
    // 연한 경사에서 등고선형 밴드 윤곽이 둘렌다. 정본 '가치대비 조형'은
    // 형태 간 명암 대비지 형태 내부 그라데이션 포스터화가 아니다. 지형은
    // 부드러운 PBR(6차) + 페인티드 블롭 섀도가 정답.
    // 수동 원근 안개 — 카메라 DistanceFog는 WebGL2 유니폼 한도로 불가.
    // 정사영 리그는 거리가 고정(camera_rig CAM_DIST=140)이라 절대거리로 쓰면 전면이
    // 안개에 묻힌다. 뷰축거리−카메라거리(=타깃에서의 이격)로 공기 원근을 건다.
    // VIEW_DIR/140은 camera_rig.rs의 CAM_PITCH 35°·CAM_YAW 45°·CAM_DIST와 정합.
    let view_dir = vec3<f32>(-0.5792, -0.5736, -0.5792);
    let axis = dot(in.world_position.xyz - view.world_position, view_dir) - 140.0;
    // 곡률(factor) 구간에서는 수평면 근사 안개가 성립하지 않는다 — 소등.
    let fog_f = smoothstep(42.0, 125.0, axis) * (1.0 - clamp(globe.w, 0.0, 1.0));
    let fog_col = vec3<f32>(0.72, 0.81, 0.92);
    out.color = vec4<f32>(mix(out.color.rgb, fog_col, fog_f * 0.75), out.color.a);
#endif

    return out;
}
