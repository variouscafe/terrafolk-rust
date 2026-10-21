//! 원격 에셋(GLB) 캐시·로드 — 발명품 3D 모델(§17.86 확장)의 클라이언트측
//! 기반. URL → 캐시 파일(sha16 파일명) → bevy "remote" 소스(FileAssetReader)
//! 로드. 네이티브 전용(안드로이드 우선 계약 — wasm은 JS fetch 경로 후속).

use bevy::prelude::*;

/// 캐시 루트 — 진단 스크린샷(shot_path)과 동일한 플랫폼 쓰기 경로 규칙.
pub(crate) fn cache_dir() -> std::path::PathBuf {
    let dir = crate::platform::diagnostics::shot_path()
        .parent()
        .map(|p| p.join("remote_assets"))
        .unwrap_or_else(|| std::env::temp_dir().join("terrafolk_remote_assets"));
    let _ = std::fs::create_dir_all(&dir);
    dir
}

/// "remote" 소스 등록 — main.rs 플러그인 구성에서 1회 호출(프로세서 이전).
/// wasm은 파일 리더가 없어 미등록(원격 로드 JS 경로 후속 — 지금은 비활성).
#[cfg(not(target_arch = "wasm32"))]
pub fn register_source(app: &mut App) {
    use bevy::asset::io::AssetSourceBuilder;
    use bevy::asset::io::file::FileAssetReader;
    let dir = cache_dir();
    let _ = std::fs::create_dir_all(&dir);
    let reader_dir = dir.clone();
    app.register_asset_source(
        "remote",
        AssetSourceBuilder::new(move || Box::new(FileAssetReader::new(reader_dir.clone()))),
    );
    info!("[remote] 원격 에셋 소스 등록: {}", dir.display());
}

/// URL → 캐시 경로(파일 있으면 Some).
pub(crate) fn cached_path(url: &str) -> Option<std::path::PathBuf> {
    let p = cache_dir().join(file_name(url));
    p.is_file().then_some(p)
}

fn file_name(url: &str) -> String {
    // sha8(url) + .glb — URL 길이·특수문자 무관 안전 파일명.
    let mut h: u64 = 0xcbf29ce484222325;
    for b in url.as_bytes() {
        h ^= *b as u64;
        h = h.wrapping_mul(0x100000001b3);
    }
    format!("{h:016x}.glb")
}

/// 다운로드 스레드(http_poll 패턴 — 전용 스레드+current-thread tokio).
/// 결과는 캐시 파일로만 전달(비블로킹 — pump가 완료를 파일로 감지).
#[cfg(not(target_arch = "wasm32"))]
pub(crate) fn spawn_download(url: String) {
    std::thread::Builder::new()
        .name("remote-glb".into())
        .spawn(move || {
            let Ok(rt) = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
            else {
                return;
            };
            rt.block_on(async move {
                let Ok(http) = reqwest::Client::builder()
                    .user_agent("terrafolk-client/1.0")
                    .timeout(std::time::Duration::from_secs(120))
                    .build()
                else {
                    return;
                };
                match http.get(&url).send().await {
                    Ok(r) if r.status().is_success() => match r.bytes().await {
                        Ok(bytes) => {
                            let tmp = cache_dir().join(format!(
                                "{}.part",
                                file_name(&url).trim_end_matches(".glb")
                            ));
                            if std::fs::write(&tmp, &bytes).is_ok() {
                                let fin = cache_dir().join(file_name(&url));
                                let _ = std::fs::rename(&tmp, &fin);
                            }
                        }
                        Err(e) => warn!("[remote] 본문 오류 {url}: {e}"),
                    },
                    Ok(r) => warn!("[remote] {} {url}", r.status()),
                    Err(e) => warn!("[remote] 다운로드 실패 {url}: {e}"),
                }
            });
        })
        .ok();
}

/// 캐시 경로 → remote 소스 씬 핸들(Gltf Scene0 — 기존 프롭 규격 동일).
pub(crate) fn load_remote(
    assets: &AssetServer,
    path: &std::path::Path,
) -> Handle<bevy::world_serialization::WorldAsset> {
    let rel = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or_default();
    let asset_path = bevy::asset::AssetPath::from(rel.to_string()).with_source("remote");
    assets.load(bevy::gltf::GltfAssetLabel::Scene(0).from_asset(asset_path))
}
