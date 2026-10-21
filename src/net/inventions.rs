//! 발명품 3D 모델 소비(§17.86 확장) — inventionsJson의 `model` URL을
//! 파싱해 원격 GLB를 내려받아 캐시·로드, relic 부지(species=발명명)에
//! 동적 프롭으로 스폰한다. 폴백 체인 불변: model 실패·부재 → 기존 정적
//! relic GLB 경로 유지(게임은 큐를 기다리지 않는다).
//!
//! 단계: sync(프레임 → 이름↔URL 맵) → download(전용 스레드, http_poll 패턴)
//! → ready(asset 핸들 사전 등록) → resources.rs relic 분기가 스폰.
//! wasm은 비활성(데스크톱·안드로이드 우선 — 원격 소스가 파일 리더 기반).

use bevy::prelude::*;

#[derive(serde::Deserialize, Clone)]
pub(crate) struct InvRow {
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) model: Option<String>,
    /// §17.44 안정 key — 월드 부지 species(영문)와 매칭. 시드 발명(hut↔오두막).
    #[serde(default)]
    pub(crate) key: Option<String>,
}

/// 발명명 → 원격 GLB URL(모델 있는 행만) + 안정 key → 발명명(부지 species 매칭).
#[derive(Resource, Default)]
pub(crate) struct InventionModels {
    pub(crate) by_name: std::collections::HashMap<String, String>,
    pub(crate) key_to_name: std::collections::HashMap<String, String>,
}

/// 다운로드+로드 완료된 모델 — 발명명 → 씬 핸들. resources.rs가 소비.
#[derive(Resource, Default)]
pub(crate) struct ReadyModels {
    pub(crate) by_name: std::collections::HashMap<String, Handle<bevy::world_serialization::WorldAsset>>,
    /// 안정 key → 발명명(부지 species가 key로 오는 경로 — 시드 발명).
    pub(crate) key_to_name: std::collections::HashMap<String, String>,
}

/// 진행 상황(진단 로그용).
#[derive(Resource, Default)]
struct PendingDownloads {
    started: std::collections::HashSet<String>,
}

pub(crate) fn plugin(app: &mut App) {
    app.init_resource::<InventionModels>()
        .init_resource::<ReadyModels>()
        .init_resource::<PendingDownloads>();
    app.add_systems(Update, sync_models);
    #[cfg(target_arch = "wasm32")]
    app.add_systems(Update, (pump_downloads_wasm, dbg_glbload));
    #[cfg(not(target_arch = "wasm32"))]
    app.add_systems(Update, pump_downloads);
}

/// 프레임 inventions 원문 → 이름↔URL 맵(원문 변화 시만 재파싱).
fn sync_models(
    net: Res<crate::net::NetState>,
    mut inv: ResMut<InventionModels>,
    mut last_raw: Local<String>,
) {
    let Some(raw) = net.frame.as_ref().and_then(|f| f.inventions.as_deref()) else {
        return;
    };
    if raw == *last_raw {
        return;
    }
    *last_raw = raw.to_string();
    let Ok(rows) = serde_json::from_str::<Vec<InvRow>>(raw) else {
        return;
    };
    let n = rows.len();
    inv.by_name = rows
        .iter()
        .filter_map(|r| r.model.clone().map(|m| (r.name.clone(), m)))
        .collect();
    inv.key_to_name = rows
        .into_iter()
        .filter_map(|r| r.key.filter(|_| r.model.as_deref().is_some()).map(|k| (k, r.name)))
        .collect();
    info!("[inventions] 3D 모델 {}/{}행", inv.by_name.len(), n);
}

/// 완료된 다운로드를 파일에서 로드해 ReadyModels로. 다운로드 시작은
/// native 전용(파일 캐시 + remote 소스).
#[cfg(not(target_arch = "wasm32"))]
fn pump_downloads(
    inv: Res<InventionModels>,
    mut pending: ResMut<PendingDownloads>,
    mut ready: ResMut<ReadyModels>,
    assets: Res<AssetServer>,
) {
    for (name, url) in &inv.by_name {
        if ready.by_name.contains_key(name) || url.is_empty() {
            continue;
        }
        match super::remote_assets::cached_path(url) {
            Some(path) => {
                // 캐시 적중(이전 실행 또는 완료된 다운로드) — 로드.
                let handle = super::remote_assets::load_remote(&assets, &path);
                info!("[inventions] 모델 로드: {name}");
                ready.by_name.insert(name.clone(), handle);
            }
            None => {
                // 미캐시 — 1회만 시작(스레드가 파일로 완료).
                if pending.started.insert(url.clone()) {
                    super::remote_assets::spawn_download(url.clone());
                    info!("[inventions] 모델 다운로드 시작: {name}");
                }
            }
        }
    }
    // key 인덱스 동기화(부지 species가 key로 오는 경로 — 로드 완료분만)
    ready.key_to_name = inv
        .key_to_name
        .iter()
        .filter(|(_, n)| ready.by_name.contains_key(n.as_str()))
        .map(|(k, n)| (k.clone(), n.clone()))
        .collect();
}

/// wasm 다운로드 펌프 — JS fetch → 메모리 Dir 삽입 → 로드(파일 캐시 없음).
#[cfg(target_arch = "wasm32")]
fn pump_downloads_wasm(
    inv: Res<InventionModels>,
    mut pending: ResMut<PendingDownloads>,
    mut ready: ResMut<ReadyModels>,
    assets: Res<AssetServer>,
) {
    for (name, url) in &inv.by_name {
        if ready.by_name.contains_key(name) || url.is_empty() {
            continue;
        }
        let file = super::remote_assets::file_name_pub(url);
        if super::remote_assets::mem::has(&file) {
            let handle = super::remote_assets::load_remote(
                &assets,
                std::path::Path::new(&file),
            );
            web_sys::console::log_1(&format!("[inventions] wasm 모델 로드: {name}").into());
            ready.by_name.insert(name.clone(), handle);
        } else if pending.started.insert(url.clone()) {
            web_sys::console::log_1(&format!("[inventions] wasm 다운로드: {name}").into());
            super::remote_assets::spawn_fetch(url.clone(), file);
        }
    }
    ready.key_to_name = inv
        .key_to_name
        .iter()
        .filter(|(_, n)| ready.by_name.contains_key(n.as_str()))
        .map(|(k, n)| (k.clone(), n.clone()))
        .collect();
}

/// 진단 훅 ?tfdbg=glbload — 검증용 GLB를 맵 중심에 스폰(원격 로드 통로
/// 실측. 운영 무해 — 플래그 없으면 미동작).
#[cfg(target_arch = "wasm32")]
fn dbg_glbload(
    mut commands: Commands,
    world: Res<crate::world::data::WorldData>,
    assets: Res<AssetServer>,
    mut spawned: Local<bool>,
) {
    if !crate::net::dbg_gate::on("glbload") || *spawned {
        return;
    }
    let url = "https://assetgen.pages.dev/api/f/e496bf9a951de50a.glb";
    let file = super::remote_assets::file_name_pub(url);
    if super::remote_assets::mem::has(&file) {
        // 카메라 초점(룸 중심) 근처 3점 — 물속·화면 밋 진단 겸용
        let c = world.size as i32 / 2;
        for (dx, dz) in [(0i32, 0i32), (3, 2), (-3, 3)] {
            let (x, z) = (c + dx, c + dz);
            let ground = world.height_at(x, z);
            commands.spawn((
                crate::world::prop_fit::DynProp {
                    target_h: 2.2,
                    ground_y: ground,
                    rot_y: 0.0,
                },
                bevy::world_serialization::WorldAssetRoot(super::remote_assets::load_remote(
                    &assets,
                    std::path::Path::new(&file),
                )),
                Transform::from_translation(Vec3::new(x as f32, ground, z as f32)),
            ));
        }
        web_sys::console::log_1(&format!("[dbg] glbload 스폰 3점 @({}, {}) ground={}", c, c, world.height_at(c, c)).into());
        *spawned = true;
    } else {
        super::remote_assets::spawn_fetch(url.to_string(), file);
    }
}
