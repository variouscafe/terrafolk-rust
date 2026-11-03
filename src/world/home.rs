//! '내 집' 표시(문명 리디자인 §5 갭2, 2026-11-03) — 로그인 주민이 소유한
//! 완공 건물 지붕에 올리브 깃발을 세우고 미니맵에 핑을 띤다.
//! 내 주민 id의 공급 경로: 웹은 tf-ui loadMine이 `tf.setMine([id,…])`
//! 브리지(chat_action 패턴), 네이티브는 look_sync(/me 폴 스레드)의
//! LookStore를 mine_sync_native가 매핑한다.

use std::sync::{Arc, Mutex};

use bevy::prelude::*;

use crate::net::resources::LiveResourceTag;

/// 내 주민 id 목록(미로그인·관전 = 빈). 마커·미니맵 핑의 판정 원천.
#[derive(Resource, Default, PartialEq)]
pub struct MineAgent(pub Vec<String>);

/// wasm 브리지 버퍼 — JS `tf.setMine([id,…])`가 덮어쓰고 drain_mine이 소비.
#[derive(Resource)]
pub struct MineBridge(pub Arc<Mutex<Vec<String>>>);

impl Default for MineBridge {
    fn default() -> Self {
        Self(Arc::new(Mutex::new(Vec::new())))
    }
}

/// 완공 건물 루트 메타 — 소유자 id(빈집 null, §2.1 owner)·지면→지붕 높이.
/// resources.rs가 스폰 시 부착한다(res_sig 재스폰마다 최신값).
#[derive(Component)]
pub struct BldOwner {
    pub owner: Option<String>,
    pub top_h: f32,
}

/// 내 집 깃발 부착 상태(건물 루트 마커) — FlagPart 자식이 존재함을 뜻한다.
#[derive(Component)]
pub struct HomeFlag;

/// 깃발 구성 파트(깃대·깃발천) — 철거 대상 태그. 루트 despawn에 동반 소멸.
#[derive(Component)]
pub struct FlagPart;

/// 올리브 — 레퍼런스 팔레트 웜그린 계열. 깃대는 밝은 목재.
const OLIVE: Color = Color::srgb(0.42, 0.5, 0.22);
const POLE_COLOR: Color = Color::srgb(0.62, 0.5, 0.34);
/// 깃대 높이(지붕 위)·깃발천 크기 — 원경 가시성 우선의 다소 큰 사이즈.
const POLE_H: f32 = 1.15;
const FLAG_W: f32 = 0.62;
const FLAG_H: f32 = 0.4;

/// 브리지 등록 — JS가 `tf.setMine([agentId,…])` 호출(wasm 전용).
/// 로그인 세션 확정·로그아웃 시 tf-ui가 다시 호출한다(빈 배열 = 해제).
pub fn register_mine_bridge(bridge: Res<MineBridge>) {
    #[cfg(target_arch = "wasm32")]
    {
        use wasm_bindgen::closure::Closure;
        let q = Arc::clone(&bridge.0);
        let set_mine = Closure::<dyn Fn(js_sys::Array)>::new(move |ids: js_sys::Array| {
            let v: Vec<String> = ids.iter().filter_map(|j| j.as_string()).collect();
            if let Ok(mut q) = q.lock() {
                *q = v;
            }
        });
        let window = web_sys::window().expect("window");
        let tf = js_sys::Reflect::get(&window, &"terrafolk".into()).expect("terrafolk bridge");
        let _ = js_sys::Reflect::set(&tf, &"setMine".into(), set_mine.as_js_value());
        set_mine.forget(); // 앱 수명 유지
    }
    #[cfg(not(target_arch = "wasm32"))]
    let _ = bridge;
}

/// 브리지 소비 — 변경 시에만 MineAgent 재설정(home_flags is_changed 트리거).
pub fn drain_mine(bridge: Res<MineBridge>, mut mine: ResMut<MineAgent>) {
    let Ok(q) = bridge.0.lock() else {
        return;
    };
    if mine.0 != *q {
        mine.0 = q.clone();
    }
}

/// 네이티브 /me → MineAgent 매핑(look_sync 폴 결과, wasm은 drain_mine 경유).
#[cfg(not(target_arch = "wasm32"))]
pub fn mine_sync_native(
    look: Res<crate::net::look_sync::LookStore>,
    mut mine: ResMut<MineAgent>,
) {
    let m = look.inner.mine.lock().unwrap_or_else(|e| e.into_inner());
    let ids: Vec<String> = m.iter().map(|a| a.id.clone()).collect();
    if mine.0 != ids {
        mine.0 = ids;
    }
}

/// 내 집 깃발 유지 — BldOwner.owner가 MineAgent 목록과 일치하는 완공 건물
/// 지붕에 올리브 깃발(깃대+천). 로그아웃·소유 변경은 즉시 철거.
/// 건물 수가 적어(마을 규모 ~수십) 매 프레임 전수 판정이 저렴하다 —
/// res_sig 재스폰한 건물도 다음 프레임에 자동으로 다시 꽂힌다.
pub fn home_flags(
    mine: Res<MineAgent>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut mats: ResMut<Assets<StandardMaterial>>,
    q_root: Query<
        (
            Entity,
            &BldOwner,
            &Transform,
            Option<&HomeFlag>,
            Option<&Children>,
        ),
        With<LiveResourceTag>,
    >,
    q_part: Query<&FlagPart>,
) {
    let is_mine = |owner: &Option<String>| {
        owner
            .as_deref()
            .is_some_and(|o| mine.0.iter().any(|m| m == o))
    };
    for (root, bld, tr, flagged, children) in &q_root {
        let mine_now = is_mine(&bld.owner);
        if mine_now && flagged.is_none() {
            let base = tr.translation;
            let top = base.y + bld.top_h;
            let pole = meshes.add(Cylinder::new(0.03, POLE_H));
            let pole_mat = mats.add(StandardMaterial {
                base_color: POLE_COLOR,
                ..default()
            });
            let cloth = meshes.add(Cuboid::new(FLAG_W, FLAG_H, 0.02));
            let cloth_mat = mats.add(StandardMaterial {
                base_color: OLIVE,
                unlit: true,
                ..default()
            });
            let pole_e = commands
                .spawn((
                    FlagPart,
                    Mesh3d(pole),
                    MeshMaterial3d(pole_mat),
                    // 지붕면 5cm 침투 — 골조 끝에서 뜨지 않게
                    Transform::from_translation(Vec3::new(
                        base.x,
                        top + POLE_H / 2.0 - 0.05,
                        base.z,
                    )),
                ))
                .id();
            let cloth_e = commands
                .spawn((
                    FlagPart,
                    Mesh3d(cloth),
                    MeshMaterial3d(cloth_mat),
                    // 깃대 상단에서 바람 방향(남동)으로 펄럭 — 시선각 고정이라
                    // 정지 천도 실루엣이 산다. 원경 가시성에 얇은 옆면은 빼고
                    // 정면판만(양면 렌더는 큐브 박스판으로 해결).
                    Transform::from_translation(Vec3::new(
                        base.x + FLAG_W / 2.0,
                        top + POLE_H - 0.05 - FLAG_H / 2.0,
                        base.z,
                    ))
                    .with_rotation(Quat::from_rotation_y(0.5)),
                ))
                .id();
            commands.entity(root).add_child(pole_e);
            commands.entity(root).add_child(cloth_e);
            commands.entity(root).insert(HomeFlag);
        } else if !mine_now
            && flagged.is_some()
            && let Some(children) = children
        {
            for child in children {
                if q_part.get(*child).is_ok() {
                    commands.entity(*child).despawn();
                }
            }
            commands.entity(root).remove::<HomeFlag>();
        }
    }
}
