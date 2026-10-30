// 페인티드 하늘(2026-10-30 10차) — 지브리풍 카툰 하늘 돔 프래그먼트.
// 천정→지평선 그라디언트 + 지평선 헤이즈 띠(지형 안개색 fog_col 정합) +
// fbm 클럼프 구름: 큰 덩어리 실루엣(smoothstep) + 내부 2톤(상단 백·하단
// 밝은 회청) + 태양쪽 웜 틴트. 물 포말 뭉치 블롭 기법의 하늘 판.
// 버텍스는 bevy 기본(mesh.wgsl).
#import bevy_pbr::{
    mesh_view_bindings::view,
    forward_io::VertexOutput,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> u_top: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> u_hor: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> u_sun: vec4<f32>;

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
fn fragment(in: VertexOutput) -> @location(0) vec4<f32> {
    let dir = normalize(in.world_position.xyz - view.world_position);
    let up = clamp(dir.y, -1.0, 1.0);

    // 그라디언트 — 천정(짙음) → 지평선(웜·밝음). 부드러운 페인티드 전환.
    var col = mix(u_hor.rgb, u_top.rgb, smoothstep(-0.04, 0.42, up));
    // 지평선 헤이즈 띠 — 지형 안개색(terrain_splat fog_col)과 동일 색으로
    // 원거리 융합. 수평선 아래(바다 너머)는 완전 헤이즈.
    col = mix(col, vec3<f32>(0.72, 0.81, 0.92), smoothstep(0.18, -0.02, up) * 0.85);

    // ── 페인티드 구름 — 방향의 평면 투영 + 저주파 덩어리 클럼프.
    //    스케일 실측(10.1): 가시 반구의 |p| 범위 0~4 — 주파수를 올려
    //    하늘에 덩어리 6~8개가 걸리게(낮으면 셀 1개=구름 0개 사태).
    if (up > 0.05 && u_top.a > 0.01) {
        let p = dir.xz / (up + 0.30) * 4.0;
        let n1 = vnoise(p * 0.55 + vec2<f32>(11.3, 7.9));
        let n2 = vnoise(p * 1.35 + vec2<f32>(37.2, 11.8));
        let mass = smoothstep(0.46, 0.74, n1 * 0.72 + n2 * 0.28); // 큰 덩어리
        let dens = smoothstep(0.50, 0.92, n1 * 0.42 + n2 * 0.58); // 내부 밀도
        // 2톤 — 하단 밝은 회청 ↔ 상단 크림화이트(가치대비 조형: 형태 간)
        var cloud = mix(u_hor.rgb * 1.02 + vec3<f32>(0.10, 0.10, 0.12), vec3<f32>(0.98, 0.97, 0.95), dens);
        // 하늘 밝기 추종(밤·흐림엔 구름도 어둡게 — 유니폼 색에서 파생)
        let amb = (u_top.rgb + u_hor.rgb) * 0.5;
        cloud *= clamp(amb * 1.6, vec3<f32>(0.05), vec3<f32>(1.0));
        // 태양 쪽 웜 틴트
        let sun_f = clamp(dot(dir, normalize(u_sun.xyz)), 0.0, 1.0);
        cloud *= mix(vec3<f32>(1.0), vec3<f32>(1.07, 1.01, 0.92), sun_f * 0.8);
        let fade = smoothstep(0.05, 0.30, up); // 지평선 소멸(신축 왜곡 완화)
        col = mix(col, cloud, mass * u_top.a * fade);
    }

    return vec4<f32>(col, 1.0);
}
