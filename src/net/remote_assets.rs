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

pub(crate) fn file_name_pub(url: &str) -> String {
    file_name(url)
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

// ── wasm 경로 — 메모리 에셋 소스 + JS fetch ─────────────────────────────
// 파일 시스템이 없는 wasm은 bevy의 Dir(메모리)에 GLB 바이트를 넣고
// MemoryAssetReader로 "remote" 소스를 밀어준다(네이티브와 동일 로드 경로).
#[cfg(target_arch = "wasm32")]
pub mod mem {
    use bevy::asset::io::memory::{Dir, MemoryAssetReader};
    use std::sync::OnceLock;

    static DIR: OnceLock<Dir> = OnceLock::new();

    fn dir() -> &'static Dir {
        DIR.get_or_init(Dir::default)
    }

    /// "remote" 소스 등록 — main에서 AssetPlugin 이전 1회.
    pub fn register_source(app: &mut bevy::prelude::App) {
        use bevy::asset::AssetApp;
        use bevy::asset::io::AssetSourceBuilder;
        let root = dir().clone();
        app.register_asset_source(
            "remote",
            AssetSourceBuilder::new(move || {
                Box::new(MemoryAssetReader { root: root.clone() })
            }),
        );
    }

    /// 다운로드 완료 바이트 삽입 — 이후 load_remote로 로드 가능.
    pub fn insert(name: &str, bytes: Vec<u8>) {
        dir().insert_asset(std::path::Path::new(name), bytes);
    }

    pub fn has(name: &str) -> bool {
        // Dir에 존재 검색 API가 제한적 — 삽입 기록으로 판정.
        INSERTED.lock().unwrap().iter().any(|n| n == name)
    }

    use std::sync::Mutex;
    pub struct Inserted(pub Mutex<Vec<String>>);
    static INSERTED: Mutex<Vec<String>> = Mutex::new(Vec::new());

    pub fn mark_inserted(name: &str) {
        INSERTED.lock().unwrap().push(name.to_string());
    }
}

/// wasm fetch — URL → 바이트(wasm-bindgen-futures, spawn_local으로 기동).
#[cfg(target_arch = "wasm32")]
pub fn spawn_fetch(url: String, name: String) {
    wasm_bindgen_futures::spawn_local(async move {
        let window = web_sys::window().unwrap();
        let resp = match js_sys::Promise::from(window.fetch_with_str(&url)) {
            p => match wasm_bindgen_futures::JsFuture::from(p).await {
                Ok(r) => r,
                Err(e) => {
                    web_sys::console::log_1(&format!("[remote] fetch 실패 {url}: {e:?}").into());
                    return;
                }
            },
        };
        let resp: web_sys::Response = wasm_bindgen::JsCast::dyn_into(resp).unwrap();
        let buf = match wasm_bindgen_futures::JsFuture::from(resp.array_buffer().unwrap()).await {
            Ok(b) => b,
            Err(e) => {
                web_sys::console::log_1(&format!("[remote] 본문 실패 {url}: {e:?}").into());
                return;
            }
        };
        let bytes = js_sys::Uint8Array::new(&buf).to_vec();
        mem::insert(&name, bytes);
        mem::mark_inserted(&name);
        web_sys::console::log_1(&format!("[remote] wasm 다운로드 완료 {name}").into());
    });
}
