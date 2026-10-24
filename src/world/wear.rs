//! 착용 시스템 — 몸통(기본 캐릭터 GLB) 위에 얹히는 아이템 레이어.
//! "육체 몸통 먼저, 그 위에 옷·아이템" 원칙(2026-10-18): 몸통은 Kenney
//! 캐릭터 씬이 담당하고, 이 모듈은 본(관절)에 앵커된 부착물만 관장한다.
//! 의상(top/bottom/feet)은 서버 파생이 lookJson.parts로 오는데 치비
//! 변형(a~f)이 의상 다양성을 이미 담당 — 색상 정밀 매핑은 후속 과제.
//!
//! 부착물 GLB(Kenney aid-*)는 뼈대 없는 정적 메시로 **발 원점에서 위로
//! 모델링**되어 있어, 본 원점 기준 오프셋만으로 착용 위치가 결정된다.
//! 애니메이션이 본을 움직이면 부착물도 따라간다(자식 계층).

use bevy::gltf::GltfAssetLabel;
use bevy::prelude::*;

use super::villager::{Villager, VillagerAnims};

/// 착용 슬롯 — 부착 본(관절)이 슬롯을 정의한다.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum WearSlot {
    /// 눈(얼굴) — 선글라스 등: head 본
    Eyes,
    /// 머리 — 헤어스타일 GLB(qubase): head 본
    Hair,
    /// 턱·콧수염 — LPC 절출 GLB(meshgen): head 본
    Beard,
    /// 턱수염 — qubase 입체 GLB(ubase Head 본 리깅, 헤어와 동일 계열): head 본
    Beard3d,
    /// 안경 — LPC 절출 GLB(meshgen): head 본(눈 위치)
    Glasses,
    /// 오른손 — 지팡이·도구: arm-right 본
    HandRight,
    /// 왼손: arm-left 본
    HandLeft,
}

impl WearSlot {
    /// 부착 대상 본 이름 후보 — Quaternius 표준 리그 우선, UE5 마네킹
    /// (Universal Base Characters)·Kenney 7본 폴백.
    fn bones(self) -> &'static [&'static str] {
        match self {
            Self::Eyes => &["Head", "head"],
            Self::Hair => &["Head", "head"],
            Self::Beard => &["Head", "head"],
            Self::Beard3d => &["Head", "head"],
            Self::Glasses => &["Head", "head"],
            Self::HandRight => &["LowerArm.R", "hand_r", "arm-right"],
            Self::HandLeft => &["LowerArm.L", "hand_l", "arm-left"],
        }
    }

    /// 본 원점 기준 부착 오프셋(모델 단위 — 전체 신장 0.67).
    /// 눈높이(세로)·얼굴 정면(전후)·손끝(하강) 캘리브레이션값.
    fn offset(self) -> Vec3 {
        match self {
            // head 본은 목 뿌리(y≈0.34) — 눈 위치까지 상승 + 정면 전진
            Self::Eyes => Vec3::new(0.0, 0.13, 0.13),
            // 헤어 — qubase 헤어 GLB는 ubase Head 본(목 뿌리) 기준 모델링.
            // qchar 치비 Head(스케일 2.05) 원점과 원점 높이가 달라 그대로
            // 두면 두피 위로 뜬다(라이브 줌 인 실측 10~20px 간극) — 하강
            // 보정. 스케일도 치비 머리보다 살짝 크게 나와 축소.
            Self::Hair => Vec3::new(0.0, -0.10, 0.0),
            // 수염 — meshgen 절출 GLB(원점=발 앵커, 실루엣이 y 0~0.5·폭 ~1.2로
            // 생성됨)을 턱 위치로 끌어내리고 치비 얼굴폭에 축소.
            Self::Beard => Vec3::new(0.0, -0.32, 0.10),
            // 입체 수염 — 헤어와 동일 ubase 캐릭터 공간(Head 본=목 뿌리,
            // 수염 밴드 y 1.55~1.69)이라 헤어 캘리브레이션 그대로.
            Self::Beard3d => Vec3::new(0.0, -0.10, 0.0),
            // 안경 — 같은 절출 계약. 렌즈 중심(실루엣 y≈0.35)을 눈 높이(0.13)로.
            Self::Glasses => Vec3::new(0.0, -0.22, 0.12),
            // 손은 어깨 본에서 아래(침비 팔은 거의 발끝까지) — 몸 옆 살짝 전방
            Self::HandRight => Vec3::new(0.0, -0.09, 0.06),
            Self::HandLeft => Vec3::new(0.0, -0.09, 0.06),
        }
    }

    /// 부착물 자체 스케일 — 원천 GLB 스케일 차이 흡수(필요 시 슬롯별 지정).
    fn scale(self) -> f32 {
        match self {
            Self::Hair => 0.85,
            Self::Beard => 0.30,
            Self::Beard3d => 0.85,
            Self::Glasses => 0.28,
            _ => 1.0,
        }
    }
}

/// 착용 item ID → 부착 GLB + 슬롯. 새 아이템은 이 레지스트리만 늘린다
/// (서버 vocab에 액세서리·도구 item이 생기면 ID를 여기에 연결).
pub fn wear_asset(item_id: &str) -> Option<(&'static str, WearSlot)> {
    match item_id {
        "sunglasses" => Some(("models/Kenney_aid-sunglasses.glb", WearSlot::Eyes)),
        "hair-long" => Some(("models/qubase/hair-long.glb", WearSlot::Hair)),
        "hair-buns" => Some(("models/qubase/hair-buns.glb", WearSlot::Hair)),
        // 헤어 3종 확장(2026-10-24) — ubase 풀 Head 본 리깅 변형 그대로
        // 파생(텍스처 512 다이어트 임베드, 스타일당 ~0.7MB).
        "hair-buzzed" => Some(("models/qubase/hair-buzzed.glb", WearSlot::Hair)),
        "hair-buzzed-f" => Some(("models/qubase/hair-buzzed-f.glb", WearSlot::Hair)),
        "hair-parted" => Some(("models/qubase/hair-parted.glb", WearSlot::Hair)),
        // 눈썹 2종(2026-10-25) — 헤어와 동일 Head 본 리깅·부착 경로.
        "brows-regular" => Some(("models/qubase/brows-regular.glb", WearSlot::Hair)),
        "brows-female" => Some(("models/qubase/brows-female.glb", WearSlot::Hair)),
        "cane" | "cane-blind" => Some(("models/Kenney_aid-cane-blind.glb", WearSlot::HandRight)),
        // 수염 — LPC 부품 id를 절출 GLB(meshgen, 2026-10-23 행2=정면 프레임 0)
        // 로. handlebar는 실루엣 3px 미만이라 미생성(매칭 실패=자연 스킵).
        "beards_5oclock_shadow" => Some(("models/lpc/beards_5oclock_shadow.glb", WearSlot::Beard)),
        "beards_beard" => Some(("models/lpc/beards_beard.glb", WearSlot::Beard)),
        "beards_chevron" => Some(("models/lpc/beards_chevron.glb", WearSlot::Beard)),
        "beards_lampshade" => Some(("models/lpc/beards_lampshade.glb", WearSlot::Beard)),
        "beards_medium" => Some(("models/lpc/beards_medium.glb", WearSlot::Beard)),
        "beards_mustache" => Some(("models/lpc/beards_mustache.glb", WearSlot::Beard)),
        "beards_trimmed" => Some(("models/lpc/beards_trimmed.glb", WearSlot::Beard)),
        // 입체 수염 — ubase 풀 마지막 미사용 변형 Hair_Beard 파생(2026-10-25).
        // LPC 절출 6종과 달리 입체적 볼륨(헤어와 동일 T_Hair 재질로 색 정합).
        "beard-full" => Some(("models/qubase/beard-full.glb", WearSlot::Beard3d)),
        // 안경 — LPC 절출 GLB(meshgen, black 스프라이트 행2=정면). halfmoon은
        // 분리 렌즈(멀티폴리곤)라 미생성.
        "facial_glasses" => Some(("models/lpc/facial_glasses.glb", WearSlot::Glasses)),
        "facial_glasses_nerd" => Some(("models/lpc/facial_glasses_nerd.glb", WearSlot::Glasses)),
        "facial_glasses_round" => Some(("models/lpc/facial_glasses_round.glb", WearSlot::Glasses)),
        "facial_glasses_halfmoon" => Some((
            "models/lpc/facial_glasses_halfmoon.glb",
            WearSlot::Glasses,
        )),
        _ => None,
    }
}

/// 주민 착용 목록(item ID) — 스폰 시 부여. 몸통 씬과 독립적으로
/// 추가·교체되며, 변경 시 WearApplied 제거로 재적용할 수 있다.
#[derive(Component, Default)]
pub struct Wears(pub Vec<String>);

/// 착용 적용 완료 마커(중복 스폰 방지).
#[derive(Component)]
pub struct WearApplied;

/// 봇 기본 수염 해시 풀(wear_asset에 등록된 GLB만).
const DEFAULT_BEARDS: &[&str] = &[
    "beards_beard",
    "beards_medium",
    "beards_trimmed",
    "beards_mustache",
    "beards_chevron",
    "beard-full",
];
/// 봇 기본 안경 해시 풀.
const DEFAULT_GLASSES: &[&str] = &[
    "facial_glasses",
    "facial_glasses_nerd",
    "facial_glasses_round",
    "facial_glasses_halfmoon",
];
/// 봇 기본 헤어 해시 풀 — qubase GLB 5종(Head 본 리깅 동일 리그).
const DEFAULT_HAIRS: &[&str] = &[
    "hair-long",
    "hair-buns",
    "hair-buzzed",
    "hair-buzzed-f",
    "hair-parted",
];

/// lookJson(원문)에서 부착형 착용 item 추출. 현재 서버 정본 의상 슬롯
/// (top/bottom/feet — 치비 변형이 담당)엔 부착물이 없어 대부분 비고,
/// wears에 등록된 액세서리 키가 늘어나면 여기 매칭만 확장한다.
/// 사용자 편집(parts)이 항상 우선하고, 봇은 lookJson에 parts가 실리지
/// 않으므로(서버=오버라이드만 저장) uuid 해시로 결정론 기본 외형을 입힌다
/// (2026-10-23 '실제 라이브 캐릭터에 적용' — 전 주민 헤어 + 남성 35% 수염
/// + 전체 25% 안경. 슬라이드 편집은 그 즉시 기본값을 대체).
pub fn wear_ids_from_look(look: &str) -> Vec<String> {
    let mut out = Vec::new();
    for id in ["sunglasses", "cane", "cane-blind"] {
        if look.contains(&format!("\"{id}\"")) {
            out.push(id.to_string());
        }
    }
    let Some(doc) = serde_json::from_str::<serde_json::Value>(look).ok() else {
        return out;
    };
    let part = |k: &str| {
        doc.pointer(&format!("/parts/{k}"))
            .and_then(|v| v.as_str())
            .map(str::to_string)
    };
    let uuid = doc
        .pointer("/uuid")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();
    let male = doc
        .pointer("/sex")
        .and_then(|v| v.as_str())
        .is_some_and(|s| s.starts_with('m') || s.starts_with('M') || s.starts_with('남'));
    let h = uuid.bytes().fold(0u64, |a, b| a.wrapping_mul(31) + b as u64);

    // 수염 — 편집값 우선, 없으면 남성 uuid 해시 35%에 기본 풀 분배.
    // 진단 게이트 ?tfdbg=wearbeard: 남성 전원 beard-full 강제(입체 수염
    // 부착 캘리브레이션 컷 판정용 — URL 플래그 없으면 무동작, 운영 무해).
    let beard = part("beard").filter(|b| b != "none").or_else(|| {
        if male && crate::net::dbg_gate::on("wearbeard") {
            return Some("beard-full".to_string());
        }
        (male && h % 100 < 35)
            .then(|| DEFAULT_BEARDS[(h / 100) as usize % DEFAULT_BEARDS.len()].to_string())
    });
    if let Some(b) = beard {
        out.push(b);
    }
    // 안경 — 편집값 우선, 없으면 uuid 해시 25%(성별 무관, 슬롯은 해시 재파생).
    let glasses = part("glasses").filter(|g| g != "none").or_else(|| {
        (h % 100 >= 25 && h % 100 < 50)
            .then(|| DEFAULT_GLASSES[(h / 7) as usize % DEFAULT_GLASSES.len()].to_string())
    });
    if let Some(g) = glasses {
        out.push(g);
    }
    // 헤어 — 편집 슬롯이 qubase id 그대로면 존중, 아니면 스타일 문자열
    // 해시로 5종 분배(전 주민 — 스타일명·uuid 어느 쪽이어도 결정론).
    let hair = part("hair").unwrap_or_else(|| uuid.clone());
    if hair.starts_with("hair-") {
        out.push(hair);
    } else {
        let hh = hair.bytes().fold(0u64, |a, b| a.wrapping_mul(31) + b as u64);
        out.push(DEFAULT_HAIRS[(hh % DEFAULT_HAIRS.len() as u64) as usize].to_string());
    }
    // 눈썹 — 전 주민, 성별 변형(Head 본 부착 — 헤어와 동일 경로).
    out.push(if male {
        "brows-regular".to_string()
    } else {
        "brows-female".to_string()
    });
    out
}

/// 착용 적용 — 씬 완결(애님 연결 = 오토폿까지 끝난 상태) 주민의 본을
/// 찾아 부착물 씬을 자식으로 스폰한다. 오토폿 AABB가 몸통만 측정하게
/// 하기 위해 VillagerAnims(세팅 완료 신호)를 게이트로 쓴다.
#[allow(clippy::type_complexity)] // Bevy 시스템 쿼리 — 프로젝트 관례(villager.rs와 동일)
pub fn apply_wears(
    mut commands: Commands,
    assets: Res<AssetServer>,
    q: Query<(Entity, &Wears), (With<Villager>, With<VillagerAnims>, Without<WearApplied>)>,
    children: Query<&Children>,
    names: Query<&Name>,
) {
    for (entity, wears) in &q {
        if wears.0.is_empty() {
            commands.entity(entity).insert(WearApplied);
            continue;
        }
        // 본 엔티티 탐색 — 이 주민 서브트리 안의 이름 매칭(타 주민과 무관)
        let wanted: Vec<WearSlot> = wears
            .0
            .iter()
            .filter_map(|id| wear_asset(id).map(|(_, s)| s))
            .collect();
        let mut bones: Vec<(WearSlot, Entity)> = Vec::new();
        for child in children.iter_descendants(entity) {
            let Ok(name) = names.get(child) else {
                continue;
            };
            for slot in &wanted {
                if slot.bones().contains(&name.as_str())
                    && !bones.iter().any(|(b, _)| b == slot)
                {
                    bones.push((*slot, child));
                }
            }
        }
        // 부착 전제 본이 하나라도 못 보이면 씬 아직 스폰 전 — 다음 프레임 재시도
        if !wanted.iter().all(|w| bones.iter().any(|(b, _)| b == w)) {
            continue;
        }
        for id in &wears.0 {
            let Some((path, slot)) = wear_asset(id) else {
                continue;
            };
            let Some((_, bone_e)) = bones.iter().find(|(b, _)| *b == slot) else {
                continue;
            };
            let scene = assets.load(GltfAssetLabel::Scene(0).from_asset(path));
            let wear_e = commands
                .spawn((
                    Name::new(format!("wear-{id}")),
                    bevy::world_serialization::WorldAssetRoot(scene),
                    Transform::from_translation(slot.offset())
                        .with_scale(Vec3::splat(slot.scale())),
                    // PBR 선행 프레임 방지 — toonify 스왑 확정 프레임에 표시
                    Visibility::Hidden,
                ))
                .id();
            commands.entity(*bone_e).add_child(wear_e);
        }
        commands.entity(entity).insert(WearApplied);
    }
}
