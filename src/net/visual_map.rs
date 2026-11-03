//! 라이브 자원 → 비주얼 매핑 — 서버 kind/species/stateJson을 PropKind로
//! 환산하는 순수 매핑 계층(resources.rs에서 분리, 500줄 규칙).

use crate::world::data::PropKind;

/// 자원 stateJson 최소 파싱 — 동물 {alive}·건축 {stage,type,w,h,progress}·
/// 나무 {maturity}(§17.75 다중 타일·성장 단계).
#[derive(Default, serde::Deserialize)]
pub(crate) struct ResState {
    #[serde(default)]
    pub(crate) alive: Option<bool>,
    /// 수중 서식(§17.43) — 어류·가재·조개. 물 타일 스킵 대신 수면 유영.
    #[serde(default)]
    pub(crate) aquatic: Option<bool>,
    #[serde(default)]
    pub(crate) stage: Option<String>,
    #[serde(default, rename = "type")]
    pub(crate) bld_type: Option<String>,
    #[serde(default)]
    pub(crate) w: Option<u16>,
    #[serde(default)]
    pub(crate) h: Option<u16>,
    #[serde(default)]
    pub(crate) progress: Option<f32>,
    /// 현재 단계 누적 작업량(게임분) — 서버는 progress를 안 보내므로
    /// hut_progress()가 stage·work·w·h로 클라 계산한다.
    #[serde(default)]
    pub(crate) work: Option<f32>,
    #[serde(default)]
    pub(crate) maturity: Option<u8>,
    // ---- 툴팁 표시 전용(§17.42·§17.78) — 렌더링에는 관여 않는다 ----
    /// hut 소유자 id(빈집이면 null)
    #[serde(default)]
    pub(crate) owner: Option<String>,
    /// hut 소유자 표시 캐시(사망자도 표시)
    #[serde(default)]
    pub(crate) owner_name: Option<String>,
    /// 공사 중 건축자 id
    #[serde(default)]
    pub(crate) by: Option<String>,
    /// grave 묘주 이름
    #[serde(default)]
    pub(crate) name: Option<String>,
    /// grave 향년
    #[serde(default)]
    pub(crate) age: Option<u16>,
    /// grave 비문(60자 — 각인 전 null)
    #[serde(default)]
    pub(crate) epitaph: Option<String>,
    /// relic 발명가 설명(§17.38 툴팁)
    #[serde(default)]
    pub(crate) purpose: Option<String>,
}

/// 동물 개굴(den) 행 판정 — kind='hut'이지만 state에 건축 정체성 필드
/// (stage·type·w) 없이 생태 필드(alive)만 있는 행. 서버가 동물 서식지를
/// hut kind로 심는다(2026-10-26 라이브 실측 — kind=hut 136행 중 개굴 117행).
/// 건축 폴백(stage 기본 COMPLETED)이 개굴마다 유령 오두막을 세우던 결함
/// 차단. 개굴은 지하 굴 — 3D 표현 없음(레거시 FE는 부지 텍스처로 그리나
/// 숲 속 공사 발판 소음이 될 뿐이라 따르지 않는다).
pub(crate) fn is_den(st: &ResState) -> bool {
    st.stage.is_none() && st.bld_type.is_none() && st.w.is_none() && st.alive.is_some()
}

/// hut 완공 판정 — 스테이지 없는 레거시 완공 행도 완공 취급(§17.92 필터·
/// 발판/완공 분기 공용).
pub(crate) fn hut_completed(st: &ResState) -> bool {
    st.stage.as_deref().unwrap_or("COMPLETED") == "COMPLETED"
}

/// 공사 진행률(0..1) — terrafolk-web hutProgress(resources.ts:184-202)와
/// 동일식. 단계별 소요 게임분 × 부지 규모 계수로 요구량을 정규화하고
/// (완료단계 누적 + 현재단계 work) / 전체. 서버가 progress를 직송하는
/// 미래에는 그 값이 우선한다.
const STAGE_DUR: [(&str, f32); 5] = [
    ("SITE_PREPARED", 30.0),
    ("FOUNDATION_READY", 60.0),
    ("FRAME_READY", 60.0),
    ("ENCLOSURE_READY", 90.0),
    ("COMPLETED", 30.0),
];

pub(crate) fn hut_progress(st: &ResState) -> f32 {
    if let Some(p) = st.progress.filter(|p| *p > 0.0) {
        return p.clamp(0.0, 1.0);
    }
    if hut_completed(st) {
        return 1.0;
    }
    let Some(stage) = st.stage.as_deref() else {
        return 1.0;
    };
    let (w, h) = (st.w.unwrap_or(2) as f32, st.h.unwrap_or(2) as f32);
    let scale = (w * h / 4.0).max(0.25);
    let reqs: Vec<f32> = STAGE_DUR
        .iter()
        .map(|&(_, d)| (d * scale).round().max(1.0))
        .collect();
    let total: f32 = reqs.iter().sum();
    let mut done = 0.0;
    let mut cur_req = None;
    for (i, &(name, _)) in STAGE_DUR.iter().enumerate() {
        if name == stage {
            cur_req = Some(reqs[i]);
            break;
        }
        done += reqs[i];
    }
    let Some(cur_req) = cur_req else {
        return 0.0; // 미지 단계 — 0으로 두고 다음 스냅숏 계약 갱신을 기다린다
    };
    let work = st.work.unwrap_or(0.0);
    ((done + work.min(cur_req)) / total).clamp(0.0, 1.0)
}

/// 발판 크기 — state.w/h 우선, 레거시 스냅숏은 타입 기본값(FE BLD_SEED_SIZE).
pub(crate) fn bld_size(state: &ResState, bld_type: &str) -> (u16, u16) {
    if let (Some(w), Some(h)) = (state.w, state.h)
        && w >= 1
        && h >= 1
    {
        return (w, h);
    }
    match bld_type {
        "WINDBREAK" => (1, 1),
        "LONGHOUSE" => (3, 2),
        "WAREHOUSE" => (3, 3), // §17.81 창고
        _ => (2, 2),           // HUT·미지 타입
    }
}

/// 서버 kind → (소품 종, 목표 높이). None = 비표시(공사 중 hut은 발판 경로).
/// 동물은 사냥 사망(alive=false) 시 소멸, 나무는 성장 단계(maturity 0..3)별
/// 종·스케일(FE 4단계 성장), 완공 건물은 타입별 GLB.
pub(crate) fn live_visual(kind: &str, species: &str, state: &ResState) -> Option<(PropKind, f32)> {
    if super::dbg_gate::off(&format!("no_{kind}")) {
        return None; // 진단 게이트 — ?tfdbg=no_animals 등
    }
    match kind {
        "animal" => {
            if state.alive == Some(false) {
                return None;
            }
            let k = match species {
                "boar" => PropKind::AnimalBoar,
                "deer" => PropKind::AnimalDeer,
                "water_deer" => PropKind::AnimalWaterDeer,
                "squirrel" => PropKind::AnimalSquirrel,
                _ => PropKind::AnimalPheasant, // pheasant·기타 조류
            };
            Some((k, crate::world::models::prop_model(k).0))
        }
        "hut" => {
            if is_den(state) {
                return None; // 동물 개굴 — 유령 오두막 방지(is_den 주석)
            }
            // 완공만 GLB — 공사 중(PLANNED~ROOF_READY)은 bld::spawn_site 발판.
            let stage = state.stage.as_deref().unwrap_or("COMPLETED");
            if stage != "COMPLETED" {
                return None;
            }
            let bld_type = state.bld_type.as_deref().unwrap_or("HUT");
            let (k, h) = match bld_type {
                "WINDBREAK" => (
                    PropKind::Windbreak,
                    crate::world::models::prop_model(PropKind::Windbreak).0,
                ),
                "LONGHOUSE" => (PropKind::Hut, 4.2 * 1.5),
                "WAREHOUSE" => (
                    PropKind::StorageHut,
                    crate::world::models::prop_model(PropKind::StorageHut).0 * 1.35,
                ),
                _ => (PropKind::Hut, 4.2),
            };
            Some((k, h))
        }
        "tree" => {
            // 성장 단계 — 0:묘목(전용 GLB) 1:어린 나무 0.5× 2:반성장 0.78× 3:성목
            let m = state.maturity.unwrap_or(3).min(3);
            let pine = species.contains("pine");
            let k = if m == 0 {
                PropKind::TreeSapling
            } else if pine {
                PropKind::TreePine
            } else {
                PropKind::TreeBroadleaf
            };
            let (h, _) = crate::world::models::prop_model(k);
            let scale = match m {
                1 => 0.5,
                2 => 0.78,
                _ => 1.0,
            };
            Some((k, h * scale))
        }
        _ => {
            let k = live_prop(kind, species)?;
            let (h, _) = crate::world::models::prop_model(k);
            Some((k, h))
        }
    }
}

/// 서버 kind → 소품 종(정적 자원).
pub(crate) fn live_prop(kind: &str, species: &str) -> Option<PropKind> {
    match kind {
        "tree" => Some(if species.contains("pine") {
            PropKind::TreePine
        } else {
            PropKind::TreeBroadleaf
        }),
        "plant" => Some(PropKind::BerryBush),
        "gold_vein" => Some(PropKind::GoldVein),
        "campfire" => Some(PropKind::Campfire),
        "stash" => Some(PropKind::Stash),
        "grave" => Some(PropKind::Grave),
        "relic" => Some(PropKind::Relic),
        _ => None,
    }
}
