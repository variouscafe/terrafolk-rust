//! 월드 — 데모 결정론 데이터 + 2.5D 아이소 렌더(스플랫 지형·3D 소품·하루순환·수면).
//! 계약: 서버는 정사각 그리드를 주고 클라이언트가 투영(docs/data-contract.md).
//! 캐릭터 시스템은 2026-10-29 전면 철퇴(사용자 지시) — 주민은 데이터
//! 엔티티(world/agent.rs)로만 존재하고 비주얼은 새 설계 후 재부착된다.

pub mod agent;
pub mod animal_anim;
pub mod bld;
pub mod cam_follow;
pub mod camera_input;
pub mod camera_rig;
pub mod data;
pub mod daynight;
pub mod globe;
pub mod materials_guard;
pub mod models;
pub mod prop_fit;
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
pub mod water;

use bevy::pbr::MaterialPlugin;
use bevy::prelude::*;

pub struct WorldPlugin;

impl Plugin for WorldPlugin {
    fn build(&self, app: &mut App) {
        // 섀도 맵 1024 — 정본 '경계 뭉개진 페인티드 블롭' 그림자(2026-10-30
        // 6차). 기본 Gaussian 9탭 필터에 맵을 거칠게 두면 가장자리가
        // 스무디해져 붓으로 찍은 듯한 덩어리가 된다(고해상도는 사진적 경계).
        app.insert_resource(bevy::light::DirectionalLightShadowMap { size: 1024 });
        app.add_plugins(bevy::pbr::MaterialPlugin::<water::WaterMaterial>::default());
        app.init_asset::<water::WaterMaterial>();
        app.add_plugins(MaterialPlugin::<splat::TerrainMaterial>::default());
        // 바람 재질 플러그인 제거(2026-10-22) — 효과 비활성, 소품은 StandardMaterial
        app.add_systems(Update, water::water_night_dim);
        app.insert_resource(ClearColor(Color::srgb(0.30, 0.48, 0.68)))
            .init_resource::<prop_fit::MeshAabbCache>()
            .insert_resource(data::WorldData::generate(20261015))
            .insert_resource(daynight::daytime_override_or_default())
            .insert_resource(precip::WeatherState::default())
            .insert_resource(camera_rig::CameraRig::default())
            .insert_resource(camera_input::PointerInput::default())
            .insert_resource(pick::Selection::default())
            .insert_resource(pick::GroundTap::default())
            .insert_resource(pick::SelectBridge::default())
            .insert_resource(cam_follow::CamFollow::default())
            .insert_resource(cam_follow::ChatFocus::default())
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
        app.add_systems(Update, materials_guard::guard_blend_materials);
        app.add_systems(Update, materials_guard::lock_roof_palette);
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
                (
                    bld::site_dressing,
                    bld::hut_lights,
                    overlay::hover_res, // §17.78 자원 호버 툴팁 — 미등록으로 죽어있었다(리뷰 #3)
                    propsfx::bob_relics,
                    propsfx::spark_veins,
                    sky::sky_update,
                    globe::sync_globe_uniforms, // 카메라 이후 병렬 그룹에서 유니폼 반영
                ),
                (pick::drain_select, pick::pick_agent, pick::follow_selection).chain(),
                occlusion::conceal_occluders,
                cam_follow::follow_camera,
            ),
        );
    }
}
