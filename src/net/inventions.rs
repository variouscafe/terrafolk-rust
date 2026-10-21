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
}

/// 발명명 → 원격 GLB URL(모델 있는 행만).
#[derive(Resource, Default)]
pub(crate) struct InventionModels {
    pub(crate) by_name: std::collections::HashMap<String, String>,
}

/// 다운로드+로드 완료된 모델 — 발명명 → 씬 핸들. resources.rs가 소비.
#[derive(Resource, Default)]
pub(crate) struct ReadyModels {
    pub(crate) by_name: std::collections::HashMap<String, Handle<bevy::world_serialization::WorldAsset>>,
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
    #[cfg(not(target_arch = "wasm32"))]
    app.add_systems(Update, (sync_models, pump_downloads));
    #[cfg(target_arch = "wasm32")]
    app.add_systems(Update, sync_models);
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
        .into_iter()
        .filter_map(|r| r.model.map(|m| (r.name, m)))
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
}
