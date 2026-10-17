//! 부팅 진단 — "켜면 파란/검은 화면"류의 원인을 기기에서 바로 좁히는 도구.
//! 범용 설계: 게임 마커 컴포넌트를 모르고도 창·카메라·가시 메시 수만으로
//! 원인을 3분법한다(스폰 문제 / 컬링·프레이밍 문제 / 렌더·present 문제).
//! 인앱 스크린샷은 카메라 패스 전체를 통과한 픽셀을 저장하므로, 화면 캡처와
//! 비교해 present 경로 결함도 격리된다.

use bevy::prelude::*;
use std::path::PathBuf;

/// 프로브 주기(초) — 첫 회는 부팅 1.2초, 최대 4회.
const PROBE_FIRST: f32 = 1.2;
const PROBE_INTERVAL: f32 = 2.5;
const PROBE_MAX: u32 = 4;

/// 인앱 스크린샷 저장 경로 — 플랫폼별 쓰기 가능 위치 해석.
/// 안드로이드는 /tmp가 없다. 앱 전용 외부 캐시(adb pull로 직접 회수 가능)를
/// 1순위, 내부 캐시(회수는 run-as — debug 빌드만)를 2순위로 쓴다.
/// 패키지명은 /proc/self/cmdline에서 자동 취득(게임·패키지 무관 재사용).
pub fn shot_path() -> PathBuf {
    if cfg!(target_os = "android") {
        let pkg = std::fs::read_to_string("/proc/self/cmdline")
            .ok()
            .and_then(|s| s.split('\0').next().map(str::trim).map(str::to_owned))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "unknown".into());
        let external = PathBuf::from("/storage/emulated/0/Android/data")
            .join(&pkg)
            .join("cache");
        let dir = if std::fs::create_dir_all(&external).is_ok() {
            external
        } else {
            let internal = PathBuf::from("/data/data").join(&pkg).join("cache");
            let _ = std::fs::create_dir_all(&internal);
            internal
        };
        dir.join("game_shot.png")
    } else {
        std::env::temp_dir().join("game_shot.png")
    }
}

/// 창·카메라·가시 메시 프로브 — 스폰 수는 세는데 화면이 비었을 때 어느
/// 단계가 죽었는지 로그로 가른다. Query 실패(카메라 없음 등)도 그 자체가 단서.
#[allow(clippy::type_complexity)]
pub fn boot_probe(
    time: Res<Time>,
    window: Single<&Window>,
    cams: Query<(&Camera, &GlobalTransform, &Projection), With<Camera3d>>,
    all_meshes: Query<(), With<Mesh3d>>,
    visible_meshes: Query<(), (With<Mesh3d>, With<ViewVisibility>)>,
    mut st: Local<(u32, f32)>, // (남은 발사 수, 다음 발사 시각)
) {
    let t = time.elapsed_secs();
    if st.1 == 0.0 {
        st.1 = PROBE_FIRST;
    }
    if st.0 >= PROBE_MAX || t < st.1 {
        return;
    }
    st.0 += 1;
    st.1 = t + PROBE_INTERVAL;
    let (w, h) = (window.physical_width(), window.physical_height());
    let cam = cams
        .iter()
        .next()
        .map(|(c, gt, p)| {
            let (kind, val) = match p {
                Projection::Orthographic(o) => ("ortho", o.scale),
                Projection::Perspective(p) => ("persp", p.fov),
                Projection::Custom(_) => ("custom", 0.0),
            };
            format!(
                "active={} pos=({:.1},{:.1},{:.1}) fwd=({:.2},{:.2},{:.2}) {kind} scale/fov={val:.2}",
                c.is_active,
                gt.translation().x, gt.translation().y, gt.translation().z,
                gt.forward().x, gt.forward().y, gt.forward().z,
            )
        })
        .unwrap_or_else(|| "없음(카메라 스폰 실패)".into());
    info!(
        "[diag] t={t:.1}s window={w}x{h}(물리) mesh총={} mesh가시={} cam: {cam}",
        all_meshes.iter().count(),
        visible_meshes.iter().count(),
    );
}

/// 검증용 인앱 스크린샷 — 6초·14초 2회(P키 수동 추가는 데스크톱).
/// 저장 결과는 bevy가 로그로 남긴다(성공/IO 에러).
pub fn debug_screenshot(
    mut commands: Commands,
    time: Res<Time>,
    keys: Res<ButtonInput<KeyCode>>,
    mut shots: Local<u32>,
) {
    const SHOT_TIMES: [f32; 2] = [6.0, 14.0];
    if *shots >= SHOT_TIMES.len() as u32 {
        return;
    }
    let due = SHOT_TIMES[*shots as usize];
    if time.elapsed_secs() > due || keys.just_pressed(KeyCode::KeyP) {
        *shots += 1;
        let path = shot_path();
        info!(
            "[diag] 인앱 스크린샷 요청 t={:.1}s → {}",
            time.elapsed_secs(),
            path.display()
        );
        commands
            .spawn(bevy::render::view::screenshot::Screenshot::primary_window())
            .observe(bevy::render::view::screenshot::save_to_disk(path));
    }
}

