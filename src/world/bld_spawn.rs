//! 라이브 hut 스폰 — bld.rs에서 분리(500줄 규칙, 2026-11-03). 서버 hut 행
//! 하나를 발판·시공 스테이지·완공 GLB·창불·'내 집' 메타로 스폰하는
//! resources.rs 진입 본체. 발판 어휘(말뚝·밧줄·링·창불)는 bld.rs.

use bevy::prelude::*;

use super::bld::{spawn_site, spawn_window_glow, SiteRect};

/// 라이브 hut 스폰 — resources.rs에서 이관(500줄 규칙, 2026-11-03).
/// 공사 중=절차 발판(말뚝·밧줄·링·진행 바·재료 더미), 완공=타입별 GLB +
/// 창불 + '내 집' 마커 메타(BldOwner, §5 갭2). res_sig 기반 재스폰 계약은
/// 호출처(sync_resources)가 소유 — 여기선 서명을 받아 태그로만 붙인다.
#[allow(clippy::too_many_arguments)] // 스폰 파라미터 — 자산·프레임 주입
pub fn spawn_live_hut(
    commands: &mut Commands,
    meshes: &mut Assets<Mesh>,
    std_mats: &mut Assets<StandardMaterial>,
    assets: &AssetServer,
    r: &crate::net::LiveResource,
    st: &crate::net::visual_map::ResState,
    world: &super::data::WorldData,
    ready: Option<&crate::net::inventions::ReadyModels>,
) {
    use bevy::world_serialization::WorldAssetRoot;

    use super::home::BldOwner;
    use super::data::PropKind;
    use super::models::{prop_model, prop_scene, PropRoot};
    use super::prop_fit::DynProp;
    use crate::net::resources::{res_sig, LiveResMeta, LiveResourceTag};
    use crate::net::visual_map::{bld_size, live_visual};

    let bld_type = st.bld_type.clone().unwrap_or_else(|| "HUT".into());
    let (w, h) = bld_size(st, &bld_type);
    let stage = st.stage.clone().unwrap_or_else(|| "COMPLETED".into());
    let cx = r.x as f32 + w as f32 / 2.0;
    let cz = r.y as f32 + h as f32 / 2.0;
    let ground = world.height_at(r.x as i32 + w as i32 / 2, r.y as i32 + h as i32 / 2);
    let sig = res_sig(r, true);
    let rect = SiteRect {
        x: r.x as i32,
        y: r.y as i32,
        w,
        h,
    };
    if stage != "COMPLETED" {
        let site = if crate::net::dbg_gate::off("no_scaffold") {
            commands.spawn(Transform::default()).id() // 진단 게이트 — 빈 발판
        } else {
            spawn_site(
                commands,
                meshes,
                std_mats,
                r.x as i32,
                r.y as i32,
                w,
                h,
                ground,
                crate::net::visual_map::hut_progress(st),
                &stage,
            )
        };
        commands
            .entity(site)
            .insert((LiveResourceTag(r.id), LiveResMeta(sig)));
        // 재료 더미 — 부지 준비 단계의 장작(FE SITE_PREPARED 재료 표현)
        let log_h = prop_model(PropKind::WoodLog).0 * 0.9;
        let pile = commands
            .spawn((
                PropRoot {
                    kind: PropKind::WoodLog,
                    target_h: log_h,
                    ground_y: ground,
                    rot_y: (r.id % 360) as f32 / 360.0 * std::f32::consts::TAU,
                    scale_jitter: 1.0,
                    variants: prop_model(PropKind::WoodLog).1,
                },
                WorldAssetRoot(prop_scene(PropKind::WoodLog, assets)),
                Transform::from_translation(Vec3::new(
                    r.x as f32 + 0.35,
                    ground,
                    r.y as f32 + 0.35,
                )),
            ))
            .id();
        commands
            .entity(pile)
            .insert((LiveResourceTag(r.id), LiveResMeta(sig)));
    } else if let Some((kind, target_h)) = live_visual(&r.kind, &r.species, st) {
        // 발명품 3D 모델 우선(§17.86 확장) — relic 부지의 species가
        // 모델 보유 발명명과 일치하면 원격 GLB(DynProp)로 스폰.
        // 미매칭·미로딩은 기존 정적 종(folder relic) 폴백 그대로.
        let invention_model = if kind == PropKind::Relic {
            ready.and_then(|rd| {
                // species는 발명명(한글) 또는 안정 key(영문 — 시드 발명) 둘 다.
                rd.by_name
                    .get(&r.species)
                    .or_else(|| rd.key_to_name.get(&r.species).and_then(|n| rd.by_name.get(n)))
                    .cloned()
            })
        } else {
            None
        };
        if let Some(scene) = invention_model {
            commands
                .spawn((
                    DynProp {
                        target_h: 1.4,
                        ground_y: ground,
                        // meshgen 절출 카드 — 정면(+Z) 고정(요 회전 시 옆면 노출,
                        // 2026-11-07 celprop 카드 계약과 동일. 구 (id%4)*90°는
                        // 발명 카드 50%가 옆면이던 잠복 결함)
                        rot_y: 0.0,
                    },
                    WorldAssetRoot(scene),
                    LiveResourceTag(r.id),
                    LiveResMeta(sig),
                    Transform::from_translation(Vec3::new(cx, ground, cz)),
                ))
                .id();
        } else {
            let card = kind.model().card;
            commands
                .spawn((
                    PropRoot {
                        kind,
                        target_h,
                        ground_y: ground,
                        // 카드 프롭은 요 0 고정(정면 +Z) — 그 외 종결정론 회전
                        rot_y: if card {
                            0.0
                        } else {
                            (r.id % 4) as f32 * std::f32::consts::FRAC_PI_2
                        },
                        scale_jitter: 1.0,
                        variants: prop_model(kind).1,
                    },
                    WorldAssetRoot(prop_scene(kind, assets)),
                    LiveResourceTag(r.id),
                    LiveResMeta(sig),
                    // '내 집' 마커·미니맵 핑 판정용(§5 갭2) — 소유자 id·지붕 높이
                    BldOwner {
                        owner: st.owner.clone(),
                        top_h: target_h,
                    },
                    Transform::from_translation(Vec3::new(cx, ground, cz)),
                ))
                .id();
        }
        let glow = spawn_window_glow(
            commands,
            meshes,
            std_mats,
            rect,
            cx,
            cz,
            ground,
            target_h,
        );
        commands
            .entity(glow)
            .insert((LiveResourceTag(r.id), LiveResMeta(sig)));
    }
}
