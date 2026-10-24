//! 도구 시스템 — 손 본 앵커 부착(P3 2단계, 2026-10-24).
//! 게임 무관: 액터의 `HeldTool.parts`를 갱신(빈 벡터=해제)하면
//! `tool_sync`가 **변경 프레임에만** 이전 부속을 정리하고 재부착한다.
//! 부속은 프리미티브 데이터(자루·날·통)로 구성 — 에셋 의존 0, 메시
//! 팩토리 대신 게임이 부품표를 제공한다. 착용(wear) 손 앵커 관례 재사용.

use bevy::prelude::*;

/// 도구 부품 프리미티브 — 범용 3종(자루·날·통 조합으로 충분).
#[derive(Clone, PartialEq)]
pub enum ToolPrim {
    /// 실린더(반경·높이) — 자루·창대·물통.
    Cyl { r: f32, h: f32 },
    /// 직육면체 — 망머리·괭이날·도끼날.
    Box { x: f32, y: f32, z: f32 },
}

/// 도구 부속 — 프리미티브 + 손 앵커 기준 위치·요·색.
#[derive(Clone, PartialEq)]
pub struct ToolPart {
    pub prim: ToolPrim,
    /// 손 앵커 기준 추가 위치(미터).
    pub offset: Vec3,
    pub yaw_deg: f32,
    pub color: Color,
}

/// 손에 든 도구 — parts 갱신으로 장착·해제(빈 벡터=맨손). 부속 엔티티는
/// `spawned`가 추적, 변경 시 정리 후 재구성.
#[derive(Component, Default)]
pub struct HeldTool {
    pub parts: Vec<ToolPart>,
    /// 부착된 부속 엔티티(내부 관리).
    spawned: Vec<Entity>,
}

/// 오른손 본 후보 — wear HandRight 관례 동일(리그 3종 폴백).
const HAND_BONES: [&str; 3] = ["LowerArm.R", "hand_r", "arm-right"];
/// 손 앵커 오프셋 — 팔끝 아래 소폭 전방(wear 손 슬롯 캘리브레이션 근사).
const HAND_OFFSET: Vec3 = Vec3::new(0.0, -0.07, 0.05);

/// 손 본 엔티티 탐색 — 액터 하위 트리에서 HAND_BONES 최초 매치
/// (tool_sync 부착·action 토스 발사/캐치 손 위치 공용).
pub fn find_hand(
    root: Entity,
    children_q: &Query<&Children>,
    names: &Query<&Name>,
) -> Option<Entity> {
    children_q.iter_descendants(root).find(|d| {
        names
            .get(*d)
            .map(|n| HAND_BONES.contains(&n.as_str()))
            .unwrap_or(false)
    })
}

/// HeldTool 변경 시 부속 재구성 — 손 본 자식으로 부착(팔 애님 따라감).
#[allow(clippy::type_complexity)]
pub fn tool_sync(
    mut commands: Commands,
    mut q: Query<(Entity, &mut HeldTool), Changed<HeldTool>>,
    children_q: Query<&Children>,
    names: Query<&Name>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
) {
    for (root, mut tool) in &mut q {
        for e in tool.spawned.drain(..) {
            // get_entity — 주민 철거(룩 재스폰 등)로 부속이 이미 계층째
            // 죽었을 수 있다(2026-10-24 0.8.12 APK 패닉 실측: 스폰 직후
            // 명령 충돌). 죽은 엔티티 명령은 건너뛴다.
            if let Ok(mut ec) = commands.get_entity(e) {
                ec.despawn();
            }
        }
        if tool.parts.is_empty() {
            continue;
        }
        let Some(hand) = find_hand(root, &children_q, &names) else {
            continue;
        };
        let parts = tool.parts.clone();
        for part in &parts {
            let mesh = match part.prim {
                ToolPrim::Cyl { r, h } => meshes.add(Cylinder::new(r, h)),
                ToolPrim::Box { x, y, z } => meshes.add(Cuboid::new(x, y, z)),
            };
            let mut tr = Transform::from_translation(HAND_OFFSET + part.offset);
            tr.rotation = Quat::from_rotation_y(part.yaw_deg.to_radians());
            let e = commands
                .spawn((
                    Mesh3d(mesh),
                    MeshMaterial3d(mats.add(StandardMaterial {
                        base_color: part.color,
                        ..default()
                    })),
                    tr,
                ))
                .id();
            commands.entity(hand).add_child(e);
            tool.spawned.push(e);
        }
    }
}
