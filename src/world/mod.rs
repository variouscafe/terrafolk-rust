//! 월드 — 데모 결정론 데이터 + 2.5D 아이소 렌더(스플랫 지형·3D 소품·하루순환·수면).
//! 계약: 서버는 정사각 그리드를 주고 클라이언트가 투영(docs/data-contract.md).

pub mod animal_anim;
pub mod bld;
pub mod chat_action;
pub mod cam_follow;
pub mod camera_input;
pub mod camera_rig;
pub mod data;
pub mod daynight;
pub mod foot_probe;
pub mod globe;
pub mod grav_probe;
pub mod ground;
pub mod night_glow;
pub mod materials_guard;
pub mod models;
pub mod prop_fit;
pub mod occlusion;
pub mod outfit;
pub mod outline;
pub mod toon;
pub mod look_tint;
pub mod look_edit;
pub mod overlay;
pub mod pick;
pub mod precip;
/// 날씨 파티클(비·눈·번개) — 네이티브 전용(웹은 DOM 오버레이 담당)
#[cfg(not(target_arch = "wasm32"))]
pub mod precip_fx;
pub mod propsfx;
pub mod sky;
pub mod splat;
pub mod terrain;
pub mod villager;
pub mod villager_tools;
pub mod water;
pub mod wear;
#[allow(dead_code)] // 바람 흔들림 효과 제거(2026-10-22 사용자 지시 — 성능) — 모듈은 보존
pub mod wind;

use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy::pbr::MaterialPlugin::<water::WaterMaterial>::default());
        app.init_asset::<water::WaterMaterial>();
        app.add_plugins(MaterialPlugin::<splat::TerrainMaterial>::default());
        // 바람 재질 플러그인 제거(2026-10-22) — 효과 비활성, 소품은 StandardMaterial
        app.add_plugins(night_glow::plugin);
        app.add_plugins(toon::plugin);
        app.add_systems(Update, water::water_night_dim);
        app.insert_resource(ClearColor(Color::srgb(0.30, 0.48, 0.68)))
            .init_resource::<prop_fit::MeshAabbCache>()
            .insert_resource(chat_action::ActionBridge::new())
            .insert_resource(data::WorldData::generate(20261015))
            // 공간 중력 — 맵은 지구(주민 물리낙하, character/fall.rs).
            .insert_resource(crate::character::SpaceGravity::EARTH)
            .insert_resource(daynight::daytime_override_or_default())
            .insert_resource(precip::WeatherState::default())
            .insert_resource(camera_rig::CameraRig::default())
            .insert_resource(camera_input::PointerInput::default())
            .insert_resource(pick::Selection::default())
            .insert_resource(pick::GroundTap::default())
            .insert_resource(pick::SelectBridge::default())
            .insert_resource(cam_follow::CamFollow::default())
            .insert_resource(cam_follow::ChatFocus::default())
            .insert_resource(look_edit::LookEditBridge::default())
            .insert_resource(look_edit::LocalLooks::default())
            .insert_resource(look_edit::LookFocus::default())
            .insert_resource(overlay::PanBridge::default())
            .init_resource::<globe::LastWarp>()
            // 글로브 곡률 — 프레임 시작 복원(시뮬레이션은 항상 평면 좌표)·
            // 프레임 끝 워프(렌더는 구면 — 전파 직전에 적용해 자식·투영 정합).
            .add_systems(PreUpdate, globe::restore_flat)
            .add_systems(
                PostUpdate,
                globe::apply_warp.before(bevy::transform::TransformSystems::Propagate),
            )
            .add_systems(
                Startup,
                (
                    camera_rig::spawn_camera,
                    daynight::spawn_sun,
                    sky::spawn_birds,
                    pick::spawn_ring,
                    pick::register_bridge,
                    look_edit::register_bridge,
                    chat_action::register_bridge,
                    overlay::register_pan_bridge,
                    #[cfg(not(target_arch = "wasm32"))]
                    precip_fx::spawn_lightning_flash,
                ),
            );
        #[cfg(not(target_arch = "wasm32"))]
        {
            app.add_systems(
                Update,
                (precip_fx::adjust_pool, precip_fx::update_precip, precip_fx::update_lightning).chain(),
            );
        }
        // 데모 월드 제거(2026-10-23 사용자 결정) — 네이티브도 라이브 전제.
        // 이전: 레거시 맵 임베드 데모가 네이티브에서 구동(바이너리 용량 부담).
        app.add_systems(Update, animal_anim::play_animal_idles);
        // 캐릭터 프레임워크(비율·접지·발바닥 수평) — 게임 무관 계층
        // (2026-10-24 P1, docs/character-module-redesign.md). 표면장=WorldData.
        // 비율 룩 = 원본 리그 REAL(2026-10-26 사용자 결정 — 나무·건물 대비
        // 사람 비율 정합. 치비 v1은 rig::CHIBI_V1로 언제든 복귀 가능).
        app.add_plugins(crate::character::CharacterPlugin::<data::WorldData> {
            profile: crate::character::rig::REAL,
            ..Default::default()
        });
        app.configure_sets(
            Update,
            crate::character::GroundSnapSet
                .after(crate::net::apply::live_move)
                .after(villager::wander),
        );
        // 중력 낙하 프로브(?tfdbg=drop) — 리프트가 스냅 전에 반영돼야 한다.
        app.add_systems(
            Update,
            grav_probe::drop_probe.before(crate::character::GroundSnapSet),
        );
        // 채팅 지시 브리지 소비 — 파싱·부착(character/action.rs가 수행).
        app.add_systems(Update, chat_action::drain_bridge);
        // 발 앵커 계측(진단) — TF_FOOTDBG=1일 때만 로그, 평소 무음·무비용.
        app.add_systems(Update, (foot_probe::foot_probe, foot_probe::footdbg_camera));
        app.add_systems(
            Update,
            (
                outfit::vary_outfits,
                outfit::spawn_blob_shadows,
                outfit::blob_shadow_ground,
                outline::attach_outlines,
            ),
        );
        app.add_systems(Update, materials_guard::guard_blend_materials);
        app.add_systems(
            Update,
            (
                overlay::apply_pan_to,
                camera_rig::camera_control,
                overlay::overlay_feed,
                daynight::advance_time,
                daynight::apply_sky,
                prop_fit::fit_props,
                prop_fit::fit_dyn_props,
                prop_fit::apply_fit,
                models::flicker_campfires,
                (
                    bld::site_dressing,
                    bld::hut_lights,
                    overlay::hover_res, // §17.78 자원 호버 툴팁 — 미등록으로 죽어있었다(리뷰 #3)
                    propsfx::bob_relics,
                    propsfx::spark_veins,
                    sky::sky_update,
                    globe::sync_globe_uniforms, // 카메라 이후 병렬 그룹에서 유니폼 반영
                ),
                villager::wander,
                villager_tools::apply_villager_tools,
                villager::animate_states,
                villager::throttle_offscreen_anims,
                villager::retry_villager_setup,
                wear::apply_wears,
                (pick::drain_select, pick::pick_agent, pick::follow_selection).chain(),
                look_edit::drain_bridge,
                occlusion::conceal_occluders,
                cam_follow::follow_camera,
            ),
        );
    }
}
