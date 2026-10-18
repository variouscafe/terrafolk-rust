//! 동물 애니메이션 — Quaternius Animated Animal Pack(스킨드)의 Idle 클립
//! 반복 재생. 정적 소품(애님 없는 GLB)은 스킵. 이동 중 Gallop 전환
//! (RoamTo 연동)은 2차 과제 — 지금은 Idle로 "살아있는" 자세만 확보.

use bevy::asset::Assets;
use bevy::gltf::Gltf;
use bevy::prelude::*;


use super::data::PropKind;
use super::models::PropRoot;

/// 애님 부착 완료 마커(재처리 방지).
#[derive(Component)]
pub struct AnimalAnimDone;

/// 클립 선택 우선순위 — Idle 계열 우선, 없으면 첫 클립.
fn pick_clip(gltf: &Gltf) -> Option<Handle<AnimationClip>> {
    gltf.named_animations
        .iter()
        .find(|(name, _)| name.contains("Idle"))
        .map(|(_, h)| h.clone())
        .or_else(|| gltf.animations.first().cloned())
}

/// 스킨드 동물(사슴 계열)에 Idle 반복 재생 부착. GLB 로드가 비동기라
/// 로드 전 스킵 후 다음 프레임 재시도 — 마커로 완료 분기.
#[allow(clippy::type_complexity)]
pub fn play_animal_idles(
    mut commands: Commands,
    assets: Res<AssetServer>,
    gltfs: Res<Assets<Gltf>>,
    mut graphs: ResMut<Assets<AnimationGraph>>,
    q_animals: Query<(Entity, &PropRoot), Without<AnimalAnimDone>>,
    players: Query<(Entity, &AnimationPlayer), Without<AnimalAnimDone>>,
    children_q: Query<&Children>,
) {
    for (entity, prop) in q_animals.iter() {
        if !matches!(prop.kind, PropKind::AnimalDeer | PropKind::AnimalWaterDeer) {
            commands.entity(entity).insert(AnimalAnimDone);
            continue;
        }
        let handle: Handle<Gltf> = assets.load(prop.kind.model().path);
        let Some(gltf) = gltfs.get(&handle) else {
            continue; // 아직 로드 전 — 다음 프레임 재시도
        };
        let Some(clip) = pick_clip(gltf) else {
            commands.entity(entity).insert(AnimalAnimDone);
            continue;
        };
        // 자손 AnimationPlayer(bevy_gltf 자동 스폰) — 없으면 씬 스폰 대기
        let mut player_e = None;
        for child in children_q.iter_descendants(entity) {
            if players.get(child).is_ok() {
                player_e = Some(child);
                break;
            }
        }
        let Some(player_e) = player_e else {
            continue;
        };
        let mut graph = AnimationGraph::new();
        graph.add_clip(clip, 1.0, graph.root);
        let graph_h = graphs.add(graph);
        if let Ok((_, player)) = players.get(player_e) {
            let mut player = player.clone();
            player.play(0.into()).repeat();
            commands
                .entity(player_e)
                .insert((AnimationGraphHandle(graph_h), player));
        }
        commands.entity(entity).insert(AnimalAnimDone);
    }
}
