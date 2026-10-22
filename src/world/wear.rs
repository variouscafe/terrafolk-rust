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
            // 손은 어깨 본에서 아래(침비 팔은 거의 발끝까지) — 몸 옆 살짝 전방
            Self::HandRight => Vec3::new(0.0, -0.09, 0.06),
            Self::HandLeft => Vec3::new(0.0, -0.09, 0.06),
        }
    }

    /// 부착물 자체 스케일 — 원천 GLB 스케일 차이 흡수(필요 시 슬롯별 지정).
    fn scale(self) -> f32 {
        match self {
            Self::Hair => 0.85,
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
        "cane" | "cane-blind" => Some(("models/Kenney_aid-cane-blind.glb", WearSlot::HandRight)),
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

/// lookJson(원문)에서 부착형 착용 item 추출. 현재 서버 정본 의상 슬롯
/// (top/bottom/feet — 치비 변형이 담당)엔 부착물이 없어 대부분 비고,
/// wears에 등록된 액세서리 키가 늘어나면 여기 매칭만 확장한다.
/// 헤어(2026-10-23) — parts.hair 스타일 id를 보유 GLB 2종(long·buns)으로
/// 결정론 분배. 스타일이 바뀌면 헤어가 달라 붙는다(편집 미리보기 연동).
pub fn wear_ids_from_look(look: &str) -> Vec<String> {
    let mut out = Vec::new();
    for id in ["sunglasses", "cane", "cane-blind"] {
        if look.contains(&format!("\"{id}\"")) {
            out.push(id.to_string());
        }
    }
    // 헤어 스타일 — parts.hair(사용자 편집) 우선. 봇 주민 대부분은 parts.hair가
    // 없다(서버가 오버라이드만 저장) → lookJson uuid로 결정론 기본 분배해
    // 전 주민이 헤어를 쓴다(2026-10-23 '실제 라이브 캐릭터에 적용' 요구).
    let hair_key = serde_json::from_str::<serde_json::Value>(look)
        .ok()
        .and_then(|d| {
            d.pointer("/parts/hair")
                .and_then(|v| v.as_str().map(str::to_string))
                .or_else(|| {
                    d.pointer("/uuid")
                        .and_then(|v| v.as_str().map(str::to_string))
                })
        });
    if let Some(hair) = hair_key {
        let h = hair.bytes().fold(0u64, |a, b| a.wrapping_mul(31) + b as u64);
        out.push(if h % 2 == 0 {
            "hair-long".to_string()
        } else {
            "hair-buns".to_string()
        });
    }
    out
}

/// 데모 주민 착용(결정론) — 착용 시스템 실증: 1번 선글라스·4번 지팡이.
fn demo_wears(idx: usize) -> Vec<String> {
    match idx {
        1 => vec!["sunglasses".into()],
        4 => vec!["cane".into()],
        _ => Vec::new(),
    }
}

/// 데모 스폰 지원 — spawn_legacy_villagers가 주민마다 호출.
pub fn demo_wears_component(idx: usize) -> Wears {
    Wears(demo_wears(idx))
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
                ))
                .id();
            commands.entity(*bone_e).add_child(wear_e);
        }
        commands.entity(entity).insert(WearApplied);
    }
}
