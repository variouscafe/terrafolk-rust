// 캐릭터 툰 셰이딩 — 일본 2.5D 애니 셀(2026-11-03 개편, 이전 2026-10-22
// 핸드페인트 소프트 밴드). 하드 2톤(명·암 경계 1px AA) + 웜 라이트/쿨 섀도
// 이중 틴트 + 명부 림라이트. 외곽선은 잉크 헐(chibi-character ink). PBR 스펙큘러·환경
// 반사를 아예 계산하지 않아(커스텀 프래그먼트) 매트 카툰 촉감이 된다.
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

    // 태양(단일 DirectionalLight — daynight) · 밤엔 색·세기가 낮아진다.
    // ⚠️ 패킹 계약(light.rs:1733): rgb=색×illuminance · a=illuminance.
    // 강도를 분리해 순수 색(노을 틴트)만 쓰고, 강도는 낮 태양(15000 lux,
    // daynight.rs illuminance) 기준 정규화 — 곱셈 직행 시 15000배 노출로
    // 캐릭터가 새하얘진다(2026-10-22 하얀 캐릭터 사고).
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

    // 반구 램프 — 위(하늘빛)·아래(땅빛) 미세 보정으로 정면보다 입체감
    let hemi = n.y * 0.5 + 0.5;
    let nd = dot(n, sun_dir) * 0.5 + 0.5;
    // 셀 2톤 — 명암 경계(CEL_EDGE)에서 하드 전환. 폭은 화면 미분(fwidth)
    // 기반 ~1px라 줌과 무관하게 계단 앨리어싱 없이 선명한 셀 경계가 된다.
    // 경계를 반사광 중앙(0.5)보다 약간 위로 — 애니 작화처럼 그림자 면적 확보.
    let CEL_EDGE = 0.54;
    let aa = max(fwidth(nd) * 1.2, 0.002);
    let t = smoothstep(CEL_EDGE - aa, CEL_EDGE + aa, nd);

    // 웜 라이트 / 쿨 섀도 — 레퍼런스 좌상단 웜키 광원 감각. 강도 정규화 게인
    // (낮 1.0 · 밤 ≈0.37)으로 daynight 감광 전파. ⚠️ 섀도 스케일러는 lit와
    // 같은 lit_f에서 파생 — 별도 상수식이면 밤(gain 0.1)에 섀도(0.60)가
    // 리트(0.37)보다 밝아져 밴드가 뒤집힌다(2026-10-23 '카툰→3D처럼' 보고).
    let lit_f = 0.30 + 0.70 * sun_gain;
    let lit_c = albedo * vec3<f32>(1.06, 1.00, 0.90) * sun_col * lit_f * 0.95;
    let sha_c = albedo * vec3<f32>(0.66, 0.70, 0.86) * lit_f * 0.92;
    var col = mix(sha_c, lit_c, t);
    // 반구 미세 보정(하늘빛 위·땅빛 아래) — 셀 평면감 안의 최소 입체.
    col += albedo * 0.04 * (hemi - 0.5);

    // 림라이트 — 명부(t) 실루엣 가장자리에 얇은 웜 하이라이트(애니 역광
    // 테두리). 시선 수직 성분만 하드 스텝으로 → 띠가 아닌 선 느낌.
    let v = normalize(view.world_position.xyz - in.world_position.xyz);
    let fres = 1.0 - max(dot(n, v), 0.0);
    let rim = smoothstep(0.72, 0.80, fres) * t;
    col += (albedo * 0.5 + vec3<f32>(0.10, 0.09, 0.07)) * sun_col * lit_f * rim * 0.45;

    // 앰비언트(야간 최소 가시성) — 패킹 동일(rgb=색×brightness · a=brightness,
    // 낮 280) 정규화 후 가산.
    let ab = max(lights.ambient_color.a, 1.0);
    let amb_c = lights.ambient_color.rgb / ab;
    col += albedo * amb_c * clamp(ab / 280.0, 0.0, 1.0) * 0.38;

    // 수동 원근 안개 — terrain_splat.wgsl과 동일 축·상수, 계수는 낮게(0.62):
    // 툰 재질(베이크 명암·카드 소품 포함)은 안개에 잠기면 청록 실루엣으로
    // 가라앉는다(구 라인 폰 세로 프레임 실측 2026-10-30) — 원거리 정합은
    // 유지하되 색 식별을 지형(0.75)보다 보존. 근거리(캐릭터 통상 프레임)는
    // fog_f=0이라 셀 룩에 무영향.
    let view_dir = vec3<f32>(-0.5792, -0.5736, -0.5792);
    let axis = dot(in.world_position.xyz - view.world_position, view_dir) - 140.0;
    let fog_f = smoothstep(42.0, 125.0, axis);
    let fog_col = vec3<f32>(0.72, 0.81, 0.92);
    col = mix(col, fog_col, fog_f * 0.62);

    return vec4<f32>(col, alpha);
}
