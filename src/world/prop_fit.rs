//! 프롭 자동 핏 시스템 — GLB 원천 스케일이 제각각이라 계층 AABB를 측정해
//! 목표 높이로 정규화한다(models.rs 스폰이 PropRoot/DynProp로 넘기면
//! 이 모듈이 측정·변환 적용). 파일당 500줄·기능 분리 규칙으로 models.rs에서
//! 이동(2026-10-22).

use bevy::prelude::*;
use bevy::render::mesh::VertexAttributeValues;


use super::models::{PropRoot, pos_hash};
/// 동적 원격 프롭(발명품 3D 모델 §17.86 확장) — PropKind 없이 씬 핸들+목표높이.
/// 리소스 동기(resources.rs)가 발명명 매칭 원격 GLB를 스폰할 때 쓴다.
/// 핏은 fit_props와 동일 규칙(계층 AABB→목표높이, FitTransform→apply_fit).
#[derive(Component)]
pub struct DynProp {
    pub target_h: f32,
    pub ground_y: f32,
    pub rot_y: f32,
}


#[derive(Component)]
pub struct Fitted;

pub fn mesh_aabb(mesh: &Mesh) -> Option<(Vec3, Vec3)> {
    let vals = mesh.attribute(bevy::render::mesh::Mesh::ATTRIBUTE_POSITION)?;
    let VertexAttributeValues::Float32x3(pts) = vals else {
        return None;
    };
    let mut mn = Vec3::splat(f32::MAX);
    let mut mx = Vec3::splat(f32::MIN);
    for p in pts {
        let v = Vec3::new(p[0], p[1], p[2]);
        mn = mn.min(v);
        mx = mx.max(v);
    }
    Some((mn, mx))
}

/// 핏 측정용 쿼리 타입
pub type FitQuery<'w, 's> = Query<
    'w,
    's,
    (
        Option<&'static Name>,
        Option<&'static Children>,
        Option<&'static Mesh3d>,
        &'static Transform,
        Option<&'static ChildOf>,
    ),
>;

/// 메시 엔티티의 루트 미포함 조상 체인 로컬 행렬(자신 포함).
/// GLB 번들은 RootNode에 전역 스케일이 있는 경우가 있어 체인 전체 곱이 필수.
fn chain_matrix(entity: Entity, root_e: Entity, q: &FitQuery<'_, '_>) -> Mat4 {
    let mut m = Mat4::IDENTITY;
    let mut cur = entity;
    for _ in 0..16 {
        if cur == root_e {
            break;
        }
        if let Ok((_, _, _, tf, parent)) = q.get(cur) {
            m = tf.to_matrix() * m;
            match parent {
                Some(p) => cur = p.parent(),
                None => break,
            }
        } else {
            break;
        }
    }
    m
}

/// 메시 AABB 캐시 — 버텍스 전수 순회를 에셋별 1회로.
/// 라이브 자원 수천 개(2026-10-17 4,695개)가 동시 로드 완료되며 같은 GLB
/// 메시를 반복 측정해 메인 스레드가 수십 초 블록되던 프리즈의 근본 대책.
/// 유니크 GLB 메시는 ~26개뿐 — 캐시 적중률 사실상 100%.
#[derive(Resource, Default)]
pub struct MeshAabbCache(
    pub(crate) std::collections::HashMap<bevy::asset::AssetId<Mesh>, (Vec3, Vec3)>,
);

/// 유지 메시들의 결합 AABB — 각 메시를 체인 행렬로 루트 로컬 공간으로.
pub fn meshes_aabb(
    ents: &[Entity],
    root_e: Entity,
    q: &FitQuery<'_, '_>,
    meshes: &Assets<Mesh>,
    cache: &mut MeshAabbCache,
) -> Option<(Vec3, Vec3)> {
    let mut mn = Vec3::splat(f32::MAX);
    let mut mx = Vec3::splat(f32::MIN);
    for &e in ents {
        let Ok((_, _, mesh, _, _)) = q.get(e) else {
            continue;
        };
        let Some(mh) = mesh else {
            return None; // 미로딩 — 재시도
        };
        let Some(mesh) = meshes.get(&mh.0) else {
            return None; // 미로딩 — 재시도
        };
        let (amn, amx) = match cache.0.get(&mh.id()) {
            Some(cached) => *cached,
            None => {
                let aabb = mesh_aabb(mesh)?; // 측정 실패 메시는 캐시하지 않는다
                cache.0.insert(mh.id(), aabb);
                aabb
            }
        };
        let mat = chain_matrix(e, root_e, q);
        for cx in [amn.x, amx.x] {
            for cy in [amn.y, amx.y] {
                for cz in [amn.z, amx.z] {
                    let v = mat.transform_point3(Vec3::new(cx, cy, cz));
                    mn = mn.min(v);
                    mx = mx.max(v);
                }
            }
        }
    }
    (mn.x <= mx.x).then_some((mn, mx))
}

/// 서브트리의 메시 엔티티 전수 수집(이름 병기).
fn collect_meshes(entity: Entity, q: &FitQuery<'_, '_>, out: &mut Vec<(Entity, String)>) {
    let Ok((name, children, mesh, _, _)) = q.get(entity) else {
        return;
    };
    if mesh.is_some() {
        out.push((
            entity,
            name.map(|n| n.as_str().to_string()).unwrap_or_default(),
        ));
    }
    if let Some(children) = children {
        for c in children.iter() {
            collect_meshes(c, q, out);
        }
    }
}

/// 자동 핏(1단계) — 씬 자식이 생기면: 변형 선택 → AABB 측정 → 목표 계산.
/// GLB 번들은 RootNode 아래 변형 메시들이 나란히 붙는 구조 — 서브트리 전체에서
/// 이름 매칭 메시 1개만 남기고 타 메시는 재귀 파기(묶음 통째 렌더 방지).
#[allow(clippy::type_complexity)] // Bevy 쿼리 타입 — 배타 컴포넌트 필터 조합
pub fn fit_props(
    mut commands: Commands,
    q_root: Query<
        (Entity, &PropRoot, &Children),
        (With<PropRoot>, Without<Fitted>, Without<FitTransform>),
    >,
    q_all: FitQuery,
    meshes: Res<Assets<Mesh>>,
    mut aabb_cache: ResMut<MeshAabbCache>,
) {
    for (root_e, prop, children) in &q_root {
        // 변형 선택(2026-10-26 개정) — GLB 번들의 변형(House_1·PalmTree_3)은
        // 벽·지붕·잎처럼 여러 primitive로 구성된다. 예전 '이름 매칭 메시 첫 1개만
        // 유지'는 같은 변형의 나머지 부품까지 파기해 완공 집이 벽 파편 하나로
        // 렌더되었다('집이 안 보임' 실측 2026-10-26, CottageHouses 9-primitive).
        // pick 접두사에 매칭되는 메시는 전부 유지하고 비매칭 메시만 파기한다.
        // 비메시 노드(RootNode 등 빈 변환)는 파기하지 않는다 — despawn이 재귀라
        // 유지 메시의 조상까지 죽일 수 있고, 메시가 없으니 렌더·측정에도 무해.
        let mut measure: Vec<Entity> = Vec::new();
        if !prop.variants.is_empty() {
            let pick_idx = (pos_hash(
                (prop.ground_y.to_bits() as usize) % 997,
                (prop.rot_y.to_bits() as usize) % 991,
                7,
            ) as usize)
                % prop.variants.len();
            let pick = prop.variants[pick_idx];
            let mut all_meshes = Vec::new();
            for c in children.iter() {
                collect_meshes(c, &q_all, &mut all_meshes);
            }
            for (e, name) in &all_meshes {
                if name.starts_with(pick) {
                    measure.push(*e); // 변형 구성 메시 전체 유지
                } else {
                    commands.entity(*e).despawn();
                }
            }
        }
        // 측정 대상 없음(변형 미지정=통째 사용, 또는 매칭 실패) — 전체 메시
        if measure.is_empty() {
            let mut all = Vec::new();
            for c in children.iter() {
                collect_meshes(c, &q_all, &mut all);
            }
            measure = all.into_iter().map(|(e, _)| e).collect();
        }
        // 메시 미로딩이면 다음 프레임 재시도(핏될 때까지 매 프레임 대상 유지)
        let Some((mn, mx)) = meshes_aabb(&measure, root_e, &q_all, &meshes, &mut aabb_cache) else {
            continue;
        };
        let size = (mx - mn).max(Vec3::splat(1e-4));
        let mut s = (prop.target_h / size.y) * prop.scale_jitter;
        if !s.is_finite() {
            continue;
        }
        // 방어 클램프 — 병리 데이터로 거대화 차단(원천 0.05~5배)
        let clamped = s.clamp(0.02, 6.0);
        #[cfg(target_arch = "wasm32")]
        if (s - clamped).abs() > 1e-6 {
            web_sys::console::log_1(
                &format!(
                    "FIT 클램프 {:.2}→{:.2} size=({:.2},{:.2},{:.2}) variants={:?}",
                    s, clamped, size.x, size.y, size.z, prop.variants
                )
                .into(),
            );
        }
        s = clamped;
        let root_y = prop.ground_y - mn.y * s;
        // 카드 나무(절출 스프라이트 GLB — 2026-10-30 2단계 리페인트)는
        // 법선 +Z가 곧 카메라 정면(카메라 요 0° 고정). 결정론 회전을 그대로
        // 쓰면 종잇장 옆면·빗면이 보인다 — 요 0 고정.
        let card_tree = matches!(
            prop.kind,
            super::data::PropKind::TreeBroadleaf
                | super::data::PropKind::TreePine
                | super::data::PropKind::TreeSapling
                | super::data::PropKind::BerryBush
        );
        commands.entity(root_e).insert(FitTransform {
            root_y,
            scale: s,
            rot_y: if card_tree { 0.0 } else { prop.rot_y },
        });
    }
}

#[derive(Component)]
pub struct FitTransform {
    root_y: f32,
    scale: f32,
    rot_y: f32,
}

/// 자동 핏(2단계) — 기존 x,z 유지, y·스케일·회전 적용.
pub fn apply_fit(
    mut commands: Commands,
    q: Query<(Entity, &FitTransform, &Transform), Without<Fitted>>,
) {
    for (e, fit, tf) in &q {
        let mut new_tf = *tf;
        new_tf.translation.y = fit.root_y;
        new_tf.scale = Vec3::splat(fit.scale);
        new_tf.rotation = Quat::from_rotation_y(fit.rot_y);
        commands
            .entity(e)
            .insert(new_tf)
            .insert(Fitted)
            .remove::<FitTransform>();
    }
}


/// 모닥불 빛·불꽃 플리커 — 따뜻한 요동(결정론 아님, 연출).
pub fn fit_dyn_props(
    mut commands: Commands,
    q_root: Query<
        (Entity, &DynProp, &Children),
        (With<DynProp>, Without<Fitted>, Without<FitTransform>),
    >,
    q_all: FitQuery,
    meshes: Res<Assets<Mesh>>,
    mut aabb_cache: ResMut<MeshAabbCache>,
) {
    for (root_e, prop, children) in &q_root {
        let mut all_meshes = Vec::new();
        for c in children.iter() {
            collect_meshes(c, &q_all, &mut all_meshes);
        }
        let measure: Vec<Entity> = all_meshes.into_iter().map(|(e, _)| e).collect();
        let Some((mn, mx)) = meshes_aabb(&measure, root_e, &q_all, &meshes, &mut aabb_cache)
        else {
            continue; // 메시 미로딩 — 다음 프레임 재시도
        };
        let size = (mx - mn).max(Vec3::splat(1e-4));
        let s = (prop.target_h / size.y).clamp(0.02, 6.0);
        if !s.is_finite() {
            continue;
        }
        commands.entity(root_e).insert(FitTransform {
            root_y: prop.ground_y - mn.y * s,
            scale: s,
            rot_y: prop.rot_y,
        });
    }
}
