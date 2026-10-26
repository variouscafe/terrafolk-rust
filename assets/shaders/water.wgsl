// 수면 — 이중 스크롤 해시노이즈·반짝임·프레넬 투명도 + 해안 포말(거리장 베이크).
// Bevy 0.19 wgsl — 유니폼 명시 선언(공식 custom_material.wgsl 패턴).
// 유니폼 3개 패킹(2026-10-17): 스칼라 파라미터를 params Vec4에 겹쳐 담아
// WebGL2 프래그먼트 유니폼 한도(12) 초과로 파이프라인이 죽는 회귀를 막는다.
// 글로브 버텍스(2026-10-25): 커스텀 io 구조체로 워프 전 평면좌표(flat_xz)를
// 버링 전달 — 해안 UV·포말 노이즈는 평면 좌표 그대로, 프레넬·거리감쇠는
// 워프 좌표(world_position)로 쓴다(둘 다 정합). 워프 수식은 src/world/globe.rs
// WarpFrame·terrain_globe_vertex.wgsl과 동일 계약(수정 시 세 곳 함께).
#import bevy_pbr::{
    mesh_view_bindings::{globals, view},
    forward_io::Vertex,
    mesh_functions,
    view_transformations::position_world_to_clip,
}

@group(#{MATERIAL_BIND_GROUP}) @binding(0) var<uniform> deep: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(1) var<uniform> shallow: vec4<f32>;
// x=스크롤 속도 · y=파형 스케일 · z=월드 크기 · w=밤요인(0 낮~1 밤)
@group(#{MATERIAL_BIND_GROUP}) @binding(2) var<uniform> params: vec4<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(3) var t_shore: texture_2d<f32>;
@group(#{MATERIAL_BIND_GROUP}) @binding(4) var s_shore: sampler;
// 글로브 곡률 — 버텍스 워프 전용(프래그먼트 미참조 — 유니폼 예산 불변)
@group(#{MATERIAL_BIND_GROUP}) @binding(5) var<uniform> globe: vec4<f32>;

// 평면 위치 → 구면 위치 (θ→0에서 항등 — 중심 직상공 포함)
fn globe_warp_pos(g: vec4<f32>, p: vec3<f32>) -> vec3<f32> {
    let d = p.xz - g.xy;
    let l = length(d);
    let u = select(vec2<f32>(1.0, 0.0), d / l, l > 1e-6);
    // θ 클램프(π−ε) — 극 관통 이중 피복 z-fight 방지(terrain 버텍스와 동일)
    let theta = min(l / g.z, 3.1405);
    let s = sin(theta);
    let c = cos(theta);
    let n = vec3<f32>(u.x * s, c, u.y * s);
    let base = vec3<f32>(g.x, 0.0, g.y)
        + vec3<f32>(u.x * g.z * s, g.z * (c - 1.0), u.y * g.z * s);
    return base + n * p.y;
}

struct WaterVertexOutput {
    @builtin(position) position: vec4<f32>,
    // 워프 후 위치 — 프레넬·거리 감쇠용
    @location(0) world_position: vec4<f32>,
    // 워프 전 평면좌표 — 해안 UV·포말 노이즈용
    @location(1) flat_xz: vec2<f32>,
    // 플레인 UV 0..1 — 파형 스크롤용(불변)
    @location(2) uv: vec2<f32>,
}

@vertex
fn vertex(vertex_no_morph: Vertex) -> WaterVertexOutput {
    var out: WaterVertexOutput;
    // 수면 엔티티는 원점 평행이동만(Translation) — local→world 후 워프.
    let world_from_local = mesh_functions::get_world_from_local(vertex_no_morph.instance_index);
    var wp = mesh_functions::mesh_position_local_to_world(
        world_from_local, vec4<f32>(vertex_no_morph.position, 1.0));
    out.flat_xz = wp.xz;
    if (globe.w >= 0.003) {
        wp = vec4<f32>(globe_warp_pos(globe, wp.xyz), 1.0);
    }
    out.world_position = wp;
    out.position = position_world_to_clip(out.world_position.xyz);
#ifdef VERTEX_UVS_A
    out.uv = vertex_no_morph.uv;
#endif
    return out;
}

const MAX_SHORE: f32 = 14.0;

fn hash(p: vec2<f32>) -> f32 {
    var q = fract(p * vec2<f32>(127.1, 311.7));
    q *= (17.0 + dot(q, q * 43.0));
    return fract(q.x * q.y * 95.4337);
}

fn vnoise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    let a = hash(i);
    let b = hash(i + vec2<f32>(1.0, 0.0));
    let c = hash(i + vec2<f32>(0.0, 1.0));
    let d = hash(i + vec2<f32>(1.0, 1.0));
    return mix(mix(a, b, u.x), mix(c, d, u.x), u.y);
}

@fragment
fn fragment(in: WaterVertexOutput) -> @location(0) vec4<f32> {
    let t = globals.time;
    let scroll = params.x;
    let wave = params.y;
    let world_size = params.z;
    let wp = in.flat_xz;
    let uv1 = in.uv * wave + vec2<f32>(t * scroll, t * scroll * 0.7);
    let uv2 = in.uv * wave * 1.9 - vec2<f32>(t * scroll * 1.3, t * scroll * 0.5);
    let n = vnoise(uv1) * 0.6 + vnoise(uv2) * 0.4;

    let view_dir = normalize(view.world_position - in.world_position.xyz);
    let fres = pow(1.0 - clamp(view_dir.y, 0.0, 1.0), 2.0);
    var col = mix(shallow.rgb, deep.rgb, clamp(n * 0.7 + fres * 0.4, 0.0, 1.0));

    // ── 해안 포말 — 베이크 거리장 + 노이즈 요동으로 유기적 물결선
    // 원거리 감쇠 — 직접 플레이 QA(2026-10-22): 고주파 포말·스파클 노이즈는
    // 광역 줌에서 픽셀 밀도 부족으로 모아레(격자 무늬)로 보였다. 24m까지 풀
    // 디테일, 80m 이상에선 사라져 2톤 수면만 남는다.
    let dist = distance(view.world_position, in.world_position.xyz);
    let hi = clamp(1.0 - (dist - 24.0) / 56.0, 0.0, 1.0);
    let uv_s = wp / world_size;
    let o1 = vec2<f32>(0.7, 0.4) / world_size;
    let o2 = vec2<f32>(-0.5, 0.6) / world_size;
    let o3 = vec2<f32>(0.3, -0.7) / world_size;
    let s0 = textureSample(t_shore, s_shore, uv_s).r;
    let s1 = textureSample(t_shore, s_shore, uv_s + o1).r;
    let s2 = textureSample(t_shore, s_shore, uv_s + o2).r;
    let s3 = textureSample(t_shore, s_shore, uv_s + o3).r;
    // 포말 에지는 min 침식(사각 덩어리 완화), 얕은수는 평균(1텍셀=1타일
    // 선형 램프의 밴딩이 희미한 사선 격자로 보이는 것을 완화 — 2026-10-22).
    let sd = min(min(s0, s1), min(s2, s3)) * MAX_SHORE;
    let sd_sm = (s0 + s1 + s2 + s3) * 0.25 * MAX_SHORE;
    let wob = (vnoise(wp * 1.1 + vec2<f32>(t * 0.22, -t * 0.17)) - 0.5) * 1.1 * hi;
    let band = 0.75 + wob;
    let foam_edge = 1.0 - smoothstep(band - 0.35, band + 0.55, sd);
    // 포말 텍스처 — 미세 거품 얼룩
    let foam_tex = (vnoise(wp * 3.0 + vec2<f32>(t * 0.35, t * 0.2)) * 0.7
                 + vnoise(wp * 6.5 - vec2<f32>(t * 0.5)) * 0.3) * hi;
    let foam = foam_edge * smoothstep(0.35, 0.75, foam_tex + foam_edge * 0.35);
    col = mix(col, vec3<f32>(0.94, 0.97, 0.98), clamp(foam, 0.0, 1.0) * 0.92);

    // 얕은수 — 해안 근처 밝게(수심 환상) — 평균장으로 밴딩 완화
    let shallow_w = 1.0 - smoothstep(0.0, 3.2, sd_sm);
    col = mix(col, shallow.rgb * 1.12, shallow_w * 0.35);

    let sparkle = smoothstep(0.82, 0.93, vnoise(uv2 * 2.7 + vec2<f32>(t * 0.02))) * hi;
    col += sparkle * 0.3 * (1.0 - foam_edge * 0.6);

    // 야간 감광 — 무광 재질이라 씬 조명을 안 받는다. 밤요인(params.w)으로
    // 달빛 감각의 어두운 청색으로 눌러준다(직접 플레이 QA 2026-10-22).
    let night = clamp(params.w, 0.0, 1.0);
    col = mix(col, col * vec3<f32>(0.30, 0.38, 0.52), night);

    // AlphaMode::Opaque(블렌딩 불가) — 알파는 프레임버퍼에 남아 캔버스가
    // 페이지 배경과 합성되는 오염원이므로 1.0으로 고정한다.
    return vec4<f32>(col, 1.0);
}
