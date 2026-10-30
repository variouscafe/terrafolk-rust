// 맵 소품 셰이딩(2026-10-30 7차 재튜닝) — 지형 루마 밴딩과 짝을 이루는
// 3밴드 셀 램프(카툰을 보는 듯한). 구 캐릭터용 S-커브·새틴 스펙큘러(§7)는
// 폐기 — 맵 소품은 매트. 웜 라이트/쿨 섀도 이중 틴트·daynight 패킹 계약은
// 유지. 안개는 terrain_splat.wgsl과 동일 수식(원거리 소품-지형 정합).
// 버텍스는 bevy 기본(mesh.wgsl) — SKINNED 분기가 스키닝(동물)을 처리한다.
#import bevy_pbr::{
    mesh_view_bindings::{lights, view},
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

    // 태양(단일 DirectionalLight) · 밤엔 색·세기가 낮아진다(게임 daynight
    // 패킹 계약 호환 — 뷰어는 정적 광원이라 사실상 상수).
    // ⚠️ 패킹 계약: rgb=색×illuminance · a=illuminance. 강도를 분리해 순수
    // 색만 쓰고, 강도는 낮 태양(15000 lux) 기준 정규화.
    var sun_dir = vec3<f32>(0.35, 0.8, 0.45);
    var sun_col = vec3<f32>(1.0);
    var sun_gain = 1.0;
    if (lights.n_directional_lights > 0u) {
        let l = lights.directional_lights[0];
        sun_dir = l.direction_to_light;
        let e = max(l.color.a, 1.0);
        sun_col = l.color.rgb / e;
        sun_gain = clamp(e / 15000.0, 0.0, 1.25);
    }

    // 반구 램프 — 위(하늘빛)·아래(땅빛) 미세 보정
    let hemi = n.y * 0.5 + 0.5;
    let nd = dot(n, sun_dir) * 0.5 + 0.5;
    // 셀 램프 — 3단 계단(카툰 감광). 하드 스텝에 10% 스무딩을 남겨
    // 첩벽 포스터가 아니라 붓터치 띠로 읽히게.
    var t = smoothstep(0.26, 0.76, nd);
    let t3 = clamp(floor(t * 2.99) / 2.0, 0.0, 1.0);
    t = mix(t, t3, 0.9);

    // 웜 라이트 / 쿨 섀도 — 좌상단 웜키 감각 유지, 섀도는 들어올려 대비
    // 완화(§7 'no harsh hard-edged shadows'). lit_f는 daynight 감광 전파
    // (낮 1.0 · 밤 ≈0.37 — 섀도 스케일러도 같은 lit_f에서 파생).
    // 2026-10-30 맵 소품 재튜닝 — 구 캐릭터 노출보다 밝게(PBR 앰비언트와
    // 균형 맞춤: 나무가 검은 실루엣으로 가라앉는 것 방지).
    let lit_f = 0.34 + 0.70 * sun_gain;
    let lit_c = albedo * vec3<f32>(1.10, 1.03, 0.92) * sun_col * lit_f * 1.02;
    let sha_c = albedo * vec3<f32>(0.82, 0.85, 0.95) * lit_f * 1.02;
    var col = mix(sha_c, lit_c, t);
    // 중간 톤 하이라이트 — 붓질감 보정(소량 유지)
    col += albedo * 0.05 * (hemi - 0.5);

    // 앰비언트(야간 최소 가시성) — 패킹 동일(rgb=색×brightness · a=brightness,
    // 낮 280) 정규화 후 가산.
    let ab = max(lights.ambient_color.a, 1.0);
    let amb_c = lights.ambient_color.rgb / ab;
    col += albedo * amb_c * clamp(ab / 280.0, 0.0, 1.0) * 0.55;

    // 수동 원근 안개 — terrain_splat.wgsl과 동일 수식·상수(화면 정합).
    let view_dir = vec3<f32>(-0.5792, -0.5736, -0.5792);
    let axis = dot(in.world_position.xyz - view.world_position, view_dir) - 140.0;
    let fog_f = smoothstep(42.0, 125.0, axis);
    let fog_col = vec3<f32>(0.72, 0.81, 0.92);
    col = mix(col, fog_col, fog_f * 0.75);

    return vec4<f32>(col, alpha);
}
