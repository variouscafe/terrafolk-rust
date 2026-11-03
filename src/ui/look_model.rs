//! 외형 편집기 도메인 모델 — look.rs에서 분리(500줄 규칙·기능별 분리,
//! 2026-11-03). 편집 상태(EditorData — /me 원본·버킷 규칙·소지 파생 옷
//! 패치·저장 본문 직렬화)와 그 단위 테스트를 소유한다. UI 컴포넌트·옵저버는
//! look.rs, 행 스폰은 look_view.rs, 옵션 정본은 look_data.rs(자동 생성).

use super::look::Tab;
use super::look_data;
use crate::net::look_sync::MeAgent;

pub struct EditorData {
    pub agent_id: String,
    pub name: String,
    female: bool,
    age: i64,
    pub tab: Tab,
    pub parts: std::collections::BTreeMap<String, String>,
    pub(super) wears: std::collections::BTreeMap<String, String>,
    inventory: std::collections::BTreeMap<String, f64>,
    pub(super) inv_clothes: Vec<crate::net::look_sync::InvClothes>,
}

impl EditorData {
    pub(super) fn from_me(m: &MeAgent) -> Self {
        Self {
            agent_id: m.id.clone(),
            name: m.name.clone(),
            female: matches!(m.sex.as_str(), "female" | "여자" | "f" | "F"),
            age: m.age,
            tab: Tab::Clothes,
            parts: m.parts.clone(),
            wears: m.wears.clone(),
            inventory: m.inventory.clone(),
            inv_clothes: m.clothing_inventions.clone(),
        }
    }
    /// 연령 클래스·버킷(tf-look.js gOf·lookPartOptions 규칙).
    fn age_class(&self) -> &'static str {
        if self.age < 20 {
            "teen"
        } else if self.age < 65 {
            "adult"
        } else {
            "senior"
        }
    }
    fn bucket(&self) -> String {
        format!(
            "{}_{}",
            if self.female { 'f' } else { 'm' },
            self.age_class()
        )
    }
    pub(super) fn hairs(&self) -> &'static [&'static str] {
        lookup_bucket(look_data::HAIRS, &self.bucket()).unwrap_or(&[])
    }
    pub(super) fn hair_colors(&self) -> &'static [&'static str] {
        lookup_bucket(look_data::HAIR_COLORS, self.age_class()).unwrap_or(&[])
    }
    pub(super) fn glasses(&self) -> &'static [&'static str] {
        lookup_bucket(look_data::GLASSES, self.age_class()).unwrap_or(&[])
    }
    pub(super) fn beards(&self) -> &'static [&'static str] {
        if self.female {
            &["none"]
        } else {
            look_data::BEARDS
        }
    }
    /// parts 값 — 없으면 첫 옵션(웹의 buildLook base 병행 역할).
    pub(super) fn part(&self, key: &str, opts: &[&str]) -> String {
        self.parts
            .get(key)
            .cloned()
            .or_else(|| opts.first().map(|s| s.to_string()))
            .unwrap_or_else(|| "none".into())
    }
    /// 웹 prettyPart — 마지막 세그먼트 한글표, 미등록 영문 대문자화.
    pub(super) fn pretty(&self, v: &str) -> String {
        if v == "none" {
            return "없음".into();
        }
        let last = v.split('_').next_back().unwrap_or(v);
        if let Some((_, ko)) = look_data::KO.iter().find(|(k, _)| *k == last) {
            return (*ko).into();
        }
        let mut s = String::new();
        for (i, ch) in last.chars().enumerate() {
            if i > 0 && ch.is_uppercase() {
                s.push(' ');
            }
            s.push(ch);
        }
        let mut c = s.chars();
        match c.next() {
            Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
            None => String::new(),
        }
    }
    /// 소지 여부 — 카탈로그·발명품 잠금 판정.
    pub(super) fn owned(&self, item: &str) -> bool {
        self.inventory.get(item).is_some_and(|n| *n > 0.0)
    }
    /// 소지 파생 옷 패치(effectiveWearPatch 포트) — 미리보기 fullParts용.
    fn wear_patch(&self) -> Vec<(String, String)> {
        let mut patch = Vec::new();
        for slot in ["top", "bottom", "feet"] {
            // (item, cf, look[]) — 카탈로그 소지 + 발명(등가 look)
            let mut cands: Vec<(String, f32, Vec<(String, String)>)> = look_data::CLOTHES
                .iter()
                .filter(|c| c.slot == slot && self.owned(c.item))
                .map(|c| {
                    (
                        c.item.to_owned(),
                        c.cf,
                        c.look
                            .iter()
                            .map(|(k, v)| ((*k).to_owned(), (*v).to_owned()))
                            .collect(),
                    )
                })
                .collect();
            cands.extend(
                self.inv_clothes
                    .iter()
                    .filter(|c| self.owned(&c.name))
                    .map(|c| {
                        (
                            c.name.clone(),
                            c.cf,
                            vec![
                                ("top".to_owned(), "torso_clothes_longsleeve".to_owned()),
                                ("topColor".to_owned(), c.color.clone()),
                            ],
                        )
                    }),
            );
            let pick = self.wears.get(slot).cloned().unwrap_or_default();
            let chosen = if !pick.is_empty() && pick != "auto" {
                cands.iter().find(|(i, _, _)| *i == pick)
            } else {
                None
            };
            let Some((_, _, look)) = chosen.or_else(|| {
                cands
                    .iter()
                    .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
            }) else {
                continue;
            };
            patch.extend(look.iter().cloned());
        }
        patch
    }
    /// 미리보기 parts — 편집 parts + 소지 파생 옷(웹 fullParts).
    pub(super) fn full_parts_json(&self) -> String {
        let mut m = self.parts.clone();
        for (k, v) in self.wear_patch() {
            m.insert(k, v);
        }
        serde_json::to_string(&m).unwrap_or_else(|_| "{}".into())
    }
    /// 저장 본문 — 웹 lookSave 계약(from_clothes 구분).
    pub(super) fn save_body(&self, from_clothes: bool) -> String {
        let mut parts_out = self.parts.clone();
        let wears_out: std::collections::BTreeMap<String, String> = if from_clothes {
            for k in [
                "top",
                "topColor",
                "bottom",
                "bottomColor",
                "feet",
                "feetColor",
            ] {
                parts_out.remove(k);
            }
            ["top", "bottom", "feet"]
                .iter()
                .map(|s| {
                    (
                        (*s).to_owned(),
                        self.wears.get(*s).cloned().unwrap_or("auto".into()),
                    )
                })
                .collect()
        } else {
            self.wears.clone()
        };
        serde_json::json!({ "parts": parts_out, "wears": wears_out }).to_string()
    }
}

fn lookup_bucket(
    table: &[(&'static str, &'static [&'static str])],
    key: &str,
) -> Option<&'static [&'static str]> {
    table.iter().find(|(k, _)| *k == key).map(|(_, v)| *v)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::net::look_sync::InvClothes;

    fn me_fixture() -> MeAgent {
        MeAgent {
            id: "shared-5".into(),
            name: "테스터".into(),
            sex: "female".into(),
            age: 28,
            parts: [("skin".to_owned(), "olive".to_owned())]
                .into_iter()
                .collect(),
            wears: [].into_iter().collect(),
            inventory: [("leather_wrap".to_owned(), 1.0)].into_iter().collect(),
            clothing_inventions: vec![InvClothes {
                name: "가죽 망토".into(),
                cf: 0.4,
                color: "red".into(),
            }],
        }
    }

    /// 소지 파생 옷 패치 — 카탈로그 소지(가죽 걸치개=top)만, 미소지 슬롯은 공백.
    #[test]
    fn wear_patch_owned_only() {
        let d = EditorData::from_me(&me_fixture());
        let patch = d.wear_patch();
        assert!(patch.contains(&("top".to_owned(), "torso_clothes_sleeveless2".to_owned())));
        assert!(patch.contains(&("topColor".to_owned(), "leather".to_owned())));
        assert!(!patch.iter().any(|(k, _)| k == "bottom" || k == "feet"));
    }

    /// 옷 탭 저장 — parts의 옷 키 삭제 + wears 3슬롯 auto 명시(웹 lookSave 계약).
    #[test]
    fn save_body_clothes_strips_parts() {
        let mut d = EditorData::from_me(&me_fixture());
        d.parts.insert("top".to_owned(), "torso_x".into());
        d.parts.insert("feetColor".to_owned(), "brown".into());
        let body: serde_json::Value = serde_json::from_str(&d.save_body(true)).unwrap_or_default();
        let parts = body["parts"].as_object().unwrap();
        assert!(parts.contains_key("skin") && parts.contains_key("top") == false);
        let wears = body["wears"].as_object().unwrap();
        assert_eq!(wears["top"], "auto");
        assert_eq!(wears["feet"], "auto");
    }

    /// 헤어·얼굴 탭 저장 — parts 통과 + 기존 wears 그대로.
    #[test]
    fn save_body_face_passthrough() {
        let mut d = EditorData::from_me(&me_fixture());
        d.wears.insert("feet".to_owned(), "leather_shoes".into());
        d.parts.insert("hairColor".to_owned(), "chestnut".into());
        let body: serde_json::Value = serde_json::from_str(&d.save_body(false)).unwrap_or_default();
        assert_eq!(body["parts"]["skin"], "olive");
        assert_eq!(body["wears"]["feet"], "leather_shoes");
    }

    /// 스타일 한글화 — 마지막 세그먼트 KO 표 조회.
    #[test]
    fn pretty_ko_lookup() {
        let d = EditorData::from_me(&me_fixture());
        assert_eq!(d.pretty("hair_long"), "롱헤어");
        assert_eq!(d.pretty("none"), "없음");
    }
}

