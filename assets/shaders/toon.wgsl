// 캐릭터 셰이딩 — 레퍼런스 §7 재질감(2026-10-28 질감 재현 4차).
// 이전: 3밴드 양자화 툰(핸드페인트 카툰). 레퍼런스는 'soft faceted
// low-poly · matte+satin · soft studio light'라 밴드 경계가 없는 넓은
// S-커브 그라디언트로 바꾸고, 낮은 강도의 넓은 스펙큘러(새틴 머리·러버
// 장화 감촉)를 더했다. 웜 라이트/쿨 섀도 이중 틴트는 유지하되 섀도 대비를
// 낮춰 스튜디오 감광(harsh shadow 금지 §7)에 맞춘다.
// 버텍스는 bevy 기본(mesh.wgsl) — SKINNED 분기가 스키닝을 처리한다.
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
    // 소프트 스튜디오 램프 — 밴드 없는 넓은 S-커브(§7 gentle shading).
    // 레퍼런스 렌더는 계단 없이 부드럽게 감소하는 휘도 gradient다.
    let t = smoothstep(0.16, 0.84, nd);

    // 웜 라이트 / 쿨 섀도 — 좌상단 웜키 감각 유지, 섀도는 들어올려 대비
    // 완화(§7 'no harsh hard-edged shadows'). lit_f는 daynight 감광 전파
    // (낮 1.0 · 밤 ≈0.37 — 섀도 스케일러도 같은 lit_f에서 파생).
    let lit_f = 0.30 + 0.70 * sun_gain;
    let lit_c = albedo * vec3<f32>(1.06, 1.00, 0.90) * sun_col * lit_f * 0.97;
    let sha_c = albedo * vec3<f32>(0.78, 0.80, 0.90) * lit_f * 0.94;
    var col = mix(sha_c, lit_c, t);
    // 중간 톤 하이라이트 — 붓질감 보정(소량 유지)
    col += albedo * 0.05 * (hemi - 0.5);

    // 부드러운 광택(§7 satin hair·rubbery boots) — 넓은 로브의 저강도
    // Blinn 스펙큘러. PBR 정합이 아니라 장난감 같은 매끈함의 인상용.
    let v = normalize(view.world_position - in.world_position.xyz);
    let h = normalize(sun_dir + v);
    let nh = max(dot(n, h), 0.0);
    col += albedo * sun_col * pow(nh, 26.0) * 0.11 * lit_f;
    // 위에서 살짝 비치는 브로드 스펙(라이트가 크고 부드러운 인상) — 저주파 성분
    col += albedo * sun_col * pow(nh, 6.0) * 0.05 * lit_f;

    // 앰비언트(야간 최소 가시성) — 패킹 동일(rgb=색×brightness · a=brightness,
    // 낮 280) 정규화 후 가산.
    let ab = max(lights.ambient_color.a, 1.0);
    let amb_c = lights.ambient_color.rgb / ab;
    col += albedo * amb_c * clamp(ab / 280.0, 0.0, 1.0) * 0.38;

    return vec4<f32>(col, alpha);
}
