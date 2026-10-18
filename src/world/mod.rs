//! 월드 — 데모 결정론 데이터 + 2.5D 아이소 렌더(스플랫 지형·3D 소품·하루순환·수면).
//! 계약: 서버는 정사각 그리드를 주고 클라이언트가 투영(docs/data-contract.md).

pub mod animal_anim;
pub mod bld;
pub mod cam_follow;
pub mod camera_input;
pub mod camera_rig;
pub mod data;
pub mod daynight;
pub mod legacy_world;
pub mod materials_guard;
pub mod models;
pub mod occlusion;
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
pub mod villager_probe;
pub mod water;
pub mod wear;
pub mod wind;

use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(bevy::pbr::MaterialPlugin::<water::WaterMaterial>::default());
        app.init_asset::<water::WaterMaterial>();
        app.add_plugins(MaterialPlugin::<splat::TerrainMaterial>::default());
        app.add_plugins(MaterialPlugin::<wind::WindMaterial>::default());
        app.insert_resource(ClearColor(Color::srgb(0.30, 0.48, 0.68)))
            .init_resource::<models::MeshAabbCache>()
            .insert_resource(data::WorldData::generate(20261015))
            .insert_resource(daynight::DayTime::default())
            .insert_resource(precip::WeatherState::default())
            .insert_resource(camera_rig::CameraRig::default())
            .insert_resource(camera_input::PointerInput::default())
            .insert_resource(pick::Selection::default())
            .insert_resource(pick::GroundTap::default())
            .insert_resource(pick::SelectBridge::default())
            .insert_resource(cam_follow::CamFollow::default())
            .insert_resource(overlay::PanBridge::default())
            .add_systems(
                Startup,
                (
                    camera_rig::spawn_camera,
                    daynight::spawn_sun,
                    sky::spawn_birds,
                    pick::spawn_ring,
                    pick::register_bridge,
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
        // 데모 결정론 월드는 네이티브 개발 빌드 전용 — 웹(wasm)은 라이브 접속→
        // 프라임으로 곧장 간다(스폰-철거 낭비·플래시 제거. 로딩 오버레이는
        // tf.uiPrime 통보로 해제).
        // 네이티브 데모 — 레거시 FE와 동일한 공용 맵(시드 20261011)으로 구동.
        // 웹은 prime_live_world가 라이브 맵으로 교체한다.
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Update, legacy_world::prime_legacy_world);
        app.add_systems(Update, animal_anim::play_animal_idles);
        app.add_systems(Update, materials_guard::guard_blend_materials);
        app.add_systems(
            Update,
            (
                overlay::apply_pan_to,
                camera_rig::camera_control,
                overlay::overlay_feed,
                daynight::advance_time,
                daynight::apply_sky,
                models::fit_props,
                models::apply_fit,
                wind::apply_wind,
                models::flicker_campfires,
                (
                    bld::site_dressing,
                    bld::hut_lights,
                    overlay::hover_res, // §17.78 자원 호버 툴팁 — 미등록으로 죽어있었다(리뷰 #3)
                    propsfx::bob_relics,
                    propsfx::spark_veins,
                    sky::sky_update,
                ),
                villager::wander,
                villager::animate_walk_idle,
                villager::retry_villager_setup,
                wear::apply_wears,
                // 렌더 진단 프로브 — dev 빌드 전용(운영 콘솔 지저분함 방지, 리뷰 #9)
                #[cfg(debug_assertions)]
                villager_probe::villager_render_probe,
                (pick::drain_select, pick::pick_agent, pick::follow_selection).chain(),
                occlusion::conceal_occluders,
                cam_follow::follow_camera,
            ),
        );
    }
}
