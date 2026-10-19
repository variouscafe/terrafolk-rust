//! 주민 렌더 진단 — 유령(안 보임)·투명 사태 디버깅 스위치 모음.
//! 진단 경로가 본체(villager.rs)와 성격이 달라 파일 분리(500줄 규칙).
//! 스위치: TF_HIDE_VILLAGERS(은닉 A/B) · TF_NOSKIN(스킨 제거) ·
//! TF_NOANIM(정지) · TF_PROBE_SPAWN=cube|rawglb(대조군) ·
//! TF_FOCUS_VILLAGER(카메라 고정 줌인).

use bevy::prelude::*;

use super::villager::{MODELS_M, Villager, VillagerAnims};
use crate::net::apply::LiveAgentTag;

/// 진단 — 주민 렌더 상태 덤프(유령 디버깅): 스폰 8초 후 1회.
/// 애님 연결·스킨·재질 알파 모드를 콘솔에 기록한다.
#[allow(clippy::too_many_arguments, clippy::type_complexity)] // Bevy 시스템 파라미터·쿼리 — finish_villager_setup과 동일 계열
pub fn villager_render_probe(
    mut commands: Commands,
    mut rig: ResMut<crate::world::camera_rig::CameraRig>,
    assets: Res<AssetServer>,
    world: Res<crate::world::data::WorldData>,
    mut meshes: ResMut<Assets<Mesh>>,
    time: Res<Time>,
    day: Res<crate::world::daynight::DayTime>,
    q_root: Query<(Entity, Option<&LiveAgentTag>, Has<VillagerAnims>), With<Villager>>,
    q_tr: Query<&Transform, With<Villager>>,
    children: Query<&Children>,
    mut players: Query<(Entity, &mut AnimationPlayer)>,
    skinned: Query<(Entity, &bevy::mesh::skinning::SkinnedMesh)>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    mat_handles: Query<&MeshMaterial3d<StandardMaterial>>,
) {
    #[allow(clippy::collapsible_if)]
    if time.elapsed_secs() < 8.0 || time.elapsed_secs() > 8.2 {
        return;
    }
    // 진단 스위치 — TF_HIDE_VILLAGERS=1이면 주민 루트를 숨긴다(A/B 차분 촬영).
    if std::env::var("TF_HIDE_VILLAGERS").is_ok() {
        for (e, _, _) in &q_root {
            commands.entity(e).insert(Visibility::Hidden);
        }
        info!(
            "[probe] TF_HIDE_VILLAGERS — 주민 {}명 은닉",
            q_root.iter().count()
        );
        return;
    }
    // 진단 스위치 — TF_NOSKIN=1이면 자손 SkinnedMesh를 제거한다(스킨 렌더
    // 경로 고립 실험: 보이면 스키닝 경로가 범인, 안 보이면 다른 원인).
    if std::env::var("TF_NOSKIN").is_ok() {
        let mut n = 0;
        for (e, _, _) in &q_root {
            for child in children.iter_descendants(e) {
                if skinned.get(child).is_ok() {
                    commands
                        .entity(child)
                        .remove::<bevy::mesh::skinning::SkinnedMesh>();
                    n += 1;
                }
            }
        }
        info!("[probe] TF_NOSKIN — 스킨 제거 {n}개");
        return;
    }
    // 진단 스위치 — TF_NOANIM=1이면 모든 주민 애님 일시정지(바인드포즈 정지).
    if std::env::var("TF_NOANIM").is_ok() {
        let mut n = 0;
        for (e, _, _) in &q_root {
            for child in children.iter_descendants(e) {
                if let Ok((_, mut player)) = players.get_mut(child) {
                    player.pause_all();
                    n += 1;
                }
            }
        }
        info!("[probe] TF_NOANIM — 애님 정지 {n}개");
        return;
    }
    // 진단 대조군 — TF_PROBE_SPAWN=cube: 맵 중앙 빨강 큐브(일반 메시 렌더 확인).
    // TF_PROBE_SPAWN=rawglb: 후처리 없는 주민 GLB 씬 원본 스폰(애님·틴트·파기 판별).
    if let Ok(mode) = std::env::var("TF_PROBE_SPAWN")
        && !mode.is_empty()
    {
        use bevy::gltf::GltfAssetLabel;
        let center = world.size as f32 / 2.0;
        if mode == "cube" {
            commands.spawn((
                Mesh3d(meshes.add(Mesh::from(bevy::shape::Cuboid::new(
                    1.5, 1.5, 1.5,
                )))),
                MeshMaterial3d(mats.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 0.05, 0.05),
                    ..default()
                })),
                Transform::from_translation(Vec3::new(center, 1.5, center)),
            ));
            info!("[probe] TF_PROBE_SPAWN=cube — 중앙 큐브 스폰 ({center},{center})");
        } else if mode == "rawglb" {
            let scene = assets.load(GltfAssetLabel::Scene(0).from_asset(MODELS_M[0]));
            commands.spawn((
                bevy::world_serialization::WorldAssetRoot(scene),
                Transform::from_translation(Vec3::new(center, 0.5, center)),
            ));
            info!("[probe] TF_PROBE_SPAWN=rawglb — 원본 씬 스폰 ({center},{center})");
        }
    }
    // 진단 스위치 — TF_FOCUS_VILLAGER=1이면 카메라를 첫 주민에 고정·줌인.
    if std::env::var("TF_FOCUS_VILLAGER").is_ok()
        && let Some(tr) = q_tr.iter().next()
    {
        rig.target = tr.translation.truncate();
        rig.zoom_goal = 2.4;
        info!(
            "[probe] TF_FOCUS_VILLAGER — 카메라 고정 ({:.1},{:.1}) 줌 2.4",
            tr.translation.x, tr.translation.z
        );
    }
    for (e, tag, anims) in &q_root {
        let tag_name = tag.map(|t| t.0.as_str()).unwrap_or("demo").to_owned();
        let mut n_mesh = 0;
        let mut n_skinned = 0;
        let mut alpha_modes = std::collections::BTreeMap::new();
        for child in children.iter_descendants(e) {
            if let Ok((se, _)) = skinned.get(child) {
                n_skinned += 1;
                let _ = se;
            }
            if let Ok(h) = mat_handles.get(child) {
                n_mesh += 1;
                if let Some(m) = mats.get(h) {
                    let key = format!("{:?}", m.alpha_mode);
                    *alpha_modes.entry(key).or_insert(0) += 1;
                }
            }
        }
        let playing = children
            .iter_descendants(e)
            .any(|c| players.get(c).is_ok_and(|(_, p)| !p.all_paused()));
        let fit = q_tr.get(e).map(|t| {
            format!(
                "sc={:.2} pos=({:.1},{:.1},{:.1})",
                t.scale.x, t.translation.x, t.translation.y, t.translation.z
            )
        });
        info!(
            "[probe] {} anims={} playing={} meshes={} skinned={} alpha={:?} {} t={:.0}h",
            tag_name,
            anims,
            playing,
            n_mesh,
            n_skinned,
            alpha_modes,
            fit.unwrap_or_else(|_| "no-tr".into()),
            day.hour
        );
    }
}
