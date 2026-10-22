//! 외형 틴트 — 레거시 FE 외형 편집(lookJson parts, §17.32)을 3D 주민에 반영.
//! qchar GLB는 텍스처 없는 색상 재질(baseColorFactor)이라 재질 이름 슬롯으로
//! 피부·머리색·눈·옷(상의·하의·신발) 색을 재착색한다. 팔레트는 레거시
//! palettes.ts(Universal-LPC, CC-BY-SA/GPL) ramp[3] 상향 중간톤 추출 —
//! 툰 셰이더가 그림진 밴드를 만들므로 밝은 톤 기준.
//! 지오메트리 교체(헤어스타일·수염·안경 부착)는 별도 과제 — 색상 축 담당.

use bevy::color::Color;

/// 틴트 슬롯 — lookJson parts 키와 1:1.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Slot {
    Skin,
    Hair,
    Eye,
    Top,
    Bottom,
    Feet,
}

/// 팔레트 키 → linear RGB(레거시 palettes.ts 추출).
fn table(slot: Slot) -> &'static [(&'static str, [f32; 3])] {
    match slot {
        Slot::Skin => &[
            ("light", [0.894, 0.643, 0.486]),
            ("amber", [0.918, 0.624, 0.329]),
            ("olive", [0.682, 0.420, 0.247]),
            ("taupe", [0.576, 0.408, 0.286]),
            ("bronze", [0.498, 0.298, 0.192]),
            ("brown", [0.463, 0.318, 0.227]),
            ("black", [0.267, 0.153, 0.145]),
        ],
        Slot::Hair => &[
            ("black", [0.110, 0.133, 0.133]),
            ("raven", [0.028, 0.122, 0.165]),
            ("dark_brown", [0.259, 0.086, 0.012]),
            ("chestnut", [0.506, 0.192, 0.039]),
            ("light_brown", [0.490, 0.271, 0.075]),
            ("dark_gray", [0.231, 0.231, 0.231]),
            ("ash", [0.757, 0.561, 0.541]),
            ("gray", [0.467, 0.467, 0.467]),
            ("blonde", [0.878, 0.620, 0.169]),
        ],
        Slot::Eye => &[
            ("brown", [0.494, 0.306, 0.125]),
            ("gray", [0.678, 0.631, 0.561]),
            ("green", [0.518, 0.925, 0.314]),
            ("blue", [0.341, 0.808, 0.894]),
        ],
        // 옷 팔레트 — top·bottom·feet가 공용 키 대부분 공유(룸 equippedLook 참조)
        _ => &[
            ("white", [0.769, 0.710, 0.624]),
            ("sky", [0.624, 0.733, 0.796]),
            ("teal", [0.082, 0.424, 0.600]),
            ("blue", [0.235, 0.286, 0.678]),
            ("navy", [0.196, 0.176, 0.416]),
            ("lavender", [0.443, 0.255, 0.698]),
            ("purple", [0.255, 0.075, 0.341]),
            ("rose", [0.467, 0.216, 0.169]),
            ("pink", [0.682, 0.259, 0.290]),
            ("red", [0.510, 0.090, 0.110]),
            ("maroon", [0.408, 0.129, 0.129]),
            ("orange", [0.843, 0.357, 0.102]),
            ("yellow", [0.851, 0.580, 0.192]),
            ("tan", [0.718, 0.549, 0.255]),
            ("brown", [0.384, 0.208, 0.110]),
            ("leather", [0.439, 0.263, 0.145]),
            ("walnut", [0.455, 0.294, 0.188]),
            ("gray", [0.345, 0.333, 0.380]),
            ("slate", [0.506, 0.545, 0.545]),
            ("bluegray", [0.192, 0.357, 0.286]),
            ("forest", [0.043, 0.169, 0.157]),
            ("green", [0.129, 0.267, 0.216]),
            ("charcoal", [0.165, 0.188, 0.204]),
            ("black", [0.133, 0.157, 0.165]),
        ],
    }
}

fn rgb_of(slot: Slot, key: &str) -> Option<[f32; 3]> {
    table(slot)
        .iter()
        .find(|(k, _)| *k == key)
        .map(|(_, v)| *v)
}

/// lookJson에서 틴트 추출 — parts.{skin,eye,hairColor,topColor,bottomColor,feetColor}.
/// 파싱 실패·미지정은 그 슬롯 원본 유지(부분 적용).
#[derive(Clone, Debug, Default)]
pub struct LookTints {
    pub skin: Option<[f32; 3]>,
    pub hair: Option<[f32; 3]>,
    pub eye: Option<[f32; 3]>,
    pub top: Option<[f32; 3]>,
    pub bottom: Option<[f32; 3]>,
    pub feet: Option<[f32; 3]>,
}

pub fn parse(look_json: &str) -> LookTints {
    // lookJson parts 키는 camelCase(hairColor) — serde rename으로 흡수.
    #[derive(serde::Deserialize, Default)]
    #[serde(default, rename_all = "camelCase")]
    struct Parts {
        skin: Option<String>,
        eye: Option<String>,
        hair_color: Option<String>,
        top_color: Option<String>,
        bottom_color: Option<String>,
        feet_color: Option<String>,
    }
    #[derive(serde::Deserialize)]
    struct Doc {
        #[serde(default)]
        parts: Parts,
    }
    let Ok(doc) = serde_json::from_str::<Doc>(look_json) else {
        return LookTints::default();
    };
    let p = doc.parts;
    LookTints {
        skin: p.skin.as_deref().and_then(|k| rgb_of(Slot::Skin, k)),
        hair: p.hair_color.as_deref().and_then(|k| rgb_of(Slot::Hair, k)),
        eye: p.eye.as_deref().and_then(|k| rgb_of(Slot::Eye, k)),
        top: p.top_color.as_deref().and_then(|k| rgb_of(Slot::Top, k)),
        bottom: p.bottom_color.as_deref().and_then(|k| rgb_of(Slot::Bottom, k)),
        feet: p.feet_color.as_deref().and_then(|k| rgb_of(Slot::Feet, k)),
    }
}

/// 재질 이름 → 슬롯(모델별 의상 재질표). None이면 원본색 유지.
/// 헬멧(m-worker Worker_Yellow)·밴다나·배납 등 악세서리는 염색하지 않는다.
pub fn slot_for(model: &str, mat: &str) -> Option<(Slot, f32)> {
    // 반환 튜플의 f32 = 명도 배율(같은 슬롯의 음영·트림 재질용).
    match mat {
        // 공통 얼굴 — 모든 모델
        "Skin" => return Some((Slot::Skin, 1.0)),
        "Skin_Darker" => return Some((Slot::Skin, 0.82)),
        "Eyebrows" | "Moustache" | "Beard" => return Some((Slot::Hair, 1.0)),
        "Eye" => return Some((Slot::Eye, 1.0)),
        _ => {}
    }
    match (model, mat) {
        // m-farmer — 밀짚모자 아래 머리(Beige)
        ("m-farmer", "Beige") => Some((Slot::Hair, 1.0)),
        ("m-farmer", "Brown") => Some((Slot::Top, 1.0)),
        ("m-farmer", "LightBlue") => Some((Slot::Bottom, 1.0)),
        ("m-farmer", "Brown2") => Some((Slot::Feet, 1.0)),
        // m-worker — 안전모(Worker_Yellow) 제외
        ("m-worker", "Worker_Vest") => Some((Slot::Top, 1.0)),
        ("m-worker", "LightBrown") => Some((Slot::Top, 1.25)),
        ("m-worker", "Brown2") => Some((Slot::Bottom, 1.0)),
        ("m-worker", "Black") => Some((Slot::Feet, 1.0)),
        // m-adventurer
        ("m-adventurer", "Hair") => Some((Slot::Hair, 1.0)),
        ("m-adventurer", "Green") => Some((Slot::Top, 1.0)),
        ("m-adventurer", "LightGreen") => Some((Slot::Top, 1.3)),
        ("m-adventurer", "Brown") => Some((Slot::Bottom, 1.0)),
        ("m-adventurer", "Black") => Some((Slot::Feet, 1.0)),
        // m-casual-character
        ("m-casual-character", "Hair") => Some((Slot::Hair, 1.0)),
        ("m-casual-character", "LightBrown") => Some((Slot::Top, 1.0)),
        ("m-casual-character", "LightBlue") => Some((Slot::Bottom, 1.0)),
        ("m-casual-character", "White") => Some((Slot::Feet, 1.15)),
        ("m-casual-character", "Red_Dark") => Some((Slot::Feet, 0.75)),
        // f-adventurer
        ("f-adventurer", "Hair_Brown") => Some((Slot::Hair, 1.0)),
        ("f-adventurer", "Brown") => Some((Slot::Hair, 0.8)),
        ("f-adventurer", "LightGreen") => Some((Slot::Top, 1.0)),
        ("f-adventurer", "Green") => Some((Slot::Top, 0.8)),
        ("f-adventurer", "Brown_02") => Some((Slot::Bottom, 1.0)),
        ("f-adventurer", "Brown2") => Some((Slot::Feet, 1.0)),
        // f-worker — 머리수건(Worker_Yellow) 제외, DarkBrown=머리
        ("f-worker", "DarkBrown") => Some((Slot::Hair, 1.0)),
        ("f-worker", "Worker_Vest") => Some((Slot::Top, 1.0)),
        ("f-worker", "Worker_Yellow") if model == "f-worker" => None, // 몸통 밝은 셔츠도 노란색이지만 원본 유지
        ("f-worker", "Brown_02") => Some((Slot::Bottom, 1.0)),
        ("f-worker", "Black") => Some((Slot::Feet, 1.0)),
        _ => None,
    }
}

/// 슬롯 틴트 → 최종 색(rgb 배율 적용, 알파 1).
pub fn tint_color(t: &LookTints, slot: Slot, gain: f32) -> Option<Color> {
    let base = match slot {
        Slot::Skin => t.skin,
        Slot::Hair => t.hair,
        Slot::Eye => t.eye,
        Slot::Top => t.top,
        Slot::Bottom => t.bottom,
        Slot::Feet => t.feet,
    }?;
    Some(Color::srgb(
        (base[0] * gain).min(1.0),
        (base[1] * gain).min(1.0),
        (base[2] * gain).min(1.0),
    ))
}
