//! 외형 편집 팝업 — 웹 LookEditor(tf-ui.js)의 네이티브 포트(docs/parity-3d.md
//! 유일 격차 해소). 옵션 데이터는 look_data.rs(정본 생성), 저장 계약은
//! lookSave 그대로: 옷 탭은 parts의 옷 키 삭제 + wears 3슬롯 명시, 그 외
//! 탭은 parts + 기존 wears. 선택 즉시 LocalLooks 낙관적 재스폰(look_edit)
//! + PATCH 저장(net::look_sync).

use bevy::prelude::*;

use super::look_data;
use super::theme::{self, UiTheme};
use super::{CloseButton, Popup, UiState};
use crate::net::look_sync::{LookStore, MeAgent};

/// 편집 탭 — 웹과 동일 3종(옷·머리·얼굴).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Clothes,
    Hair,
    Face,
}

/// 슬라이드 행 식별 — 옷 슬롯 또는 parts 키.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum RowTarget {
    Wear(&'static str),
    Part(&'static str),
}

/// 편집기 상태(자원) — Some이면 편집 중. mine 폴 갱신은 반영하지 않는다
/// (저장 확정 응답이 와야 사용자 선택과 섞이지 않는다).
#[derive(Resource, Default)]
pub struct LookEditor {
    pub data: Option<EditorData>,
}

pub struct EditorData {
    pub agent_id: String,
    pub name: String,
    female: bool,
    age: i64,
    pub tab: Tab,
    pub parts: std::collections::BTreeMap<String, String>,
    pub(super) wears: std::collections::BTreeMap<String, String>,
    inventory: std::collections::BTreeMap<String, f64>,
    inv_clothes: Vec<crate::net::look_sync::InvClothes>,
}

impl EditorData {
    fn from_me(m: &MeAgent) -> Self {
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
    fn hair_colors(&self) -> &'static [&'static str] {
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
    fn owned(&self, item: &str) -> bool {
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
    fn full_parts_json(&self) -> String {
        let mut m = self.parts.clone();
        for (k, v) in self.wear_patch() {
            m.insert(k, v);
        }
        serde_json::to_string(&m).unwrap_or_else(|_| "{}".into())
    }
    /// 저장 본문 — 웹 lookSave 계약(from_clothes 구분).
    fn save_body(&self, from_clothes: bool) -> String {
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

// ── UI 컴포넌트 ──
#[derive(Component)]
pub struct LookPopupRoot;
#[derive(Component)]
pub struct LookRows;
#[derive(Component)]
pub struct LookTabButton(pub Tab);
#[derive(Component)]
pub struct LookStatus;
#[derive(Component)]
pub struct LookTitle;
/// 색 점 버튼 — (parts 키, 색 키)
#[derive(Component)]
pub struct LookSwatch(pub &'static str, pub String);
/// ◀▶ 버튼 — (행, 방향)
#[derive(Component)]
pub struct LookSlide(pub RowTarget, pub i8);
/// 행 재구성 서명(오픈·탭·선택·상태)
#[derive(Resource, Default)]
pub struct LookUiSig(pub String);

pub fn spawn(mut commands: Commands, theme: Res<UiTheme>) {
    commands.init_resource::<LookEditor>();
    commands
        .spawn((
            LookPopupRoot,
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                align_items: AlignItems::Center,
                justify_content: JustifyContent::Center,
                ..default()
            },
            BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.5)),
            Visibility::Hidden,
        ))
        .observe(on_backdrop_close)
        .with_children(|m| {
            m.spawn((
                Node {
                    flex_direction: FlexDirection::Column,
                    padding: UiRect::all(Val::Px(12.0)),
                    row_gap: Val::Px(8.0),
                    width: Val::Px(300.0),
                    max_height: Val::Percent(80.0),
                    ..default()
                },
                BackgroundColor(theme::SURFACE),
            ))
            .with_children(|card| {
                card.spawn((LookTitle, Text::new("외형 편집"), theme.title(13.0)));
                // 탭 바
                card.spawn(Node {
                    flex_direction: FlexDirection::Row,
                    column_gap: Val::Px(6.0),
                    ..default()
                })
                .with_children(|tabs| {
                    for (t, label) in [
                        (Tab::Clothes, "옷"),
                        (Tab::Hair, "머리"),
                        (Tab::Face, "얼굴"),
                    ] {
                        tabs.spawn((
                            LookTabButton(t),
                            Button,
                            Node {
                                padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                                ..default()
                            },
                            BackgroundColor(theme::INK800),
                            Interaction::default(),
                        ))
                        .observe(on_tab_click)
                        .with_children(|b| {
                            b.spawn((Text::new(label), theme.label(11.0)));
                        });
                    }
                });
                card.spawn((
                    LookRows,
                    Node {
                        flex_direction: FlexDirection::Column,
                        row_gap: Val::Px(6.0),
                        ..default()
                    },
                ));
                card.spawn((LookStatus, Text::new(""), theme.text(9.0, theme::INK400)));
                card.spawn((
                    CloseButton,
                    Button,
                    Node {
                        padding: UiRect::axes(Val::Px(10.0), Val::Px(6.0)),
                        ..default()
                    },
                    BackgroundColor(theme::INK700),
                    Interaction::default(),
                ))
                .observe(on_close_click)
                .with_children(|b| {
                    b.spawn((Text::new("닫기"), theme.label(11.0)));
                });
            });
        });
    commands.init_resource::<LookUiSig>();
}

fn on_backdrop_close(_: On<Pointer<Click>>, mut state: ResMut<UiState>) {
    state.popup = None; // 종료 정리(포커스 해제)는 refresh의 에지 검출이 단일 소유
}

fn on_close_click(_: On<Pointer<Click>>, mut state: ResMut<UiState>) {
    state.popup = None;
}

fn on_tab_click(
    trigger: On<Pointer<Click>>,
    tabs: Query<&LookTabButton>,
    mut editor: ResMut<LookEditor>,
) {
    let Ok(t) = tabs.get(trigger.entity) else {
        return;
    };
    if let Some(d) = editor.data.as_mut() {
        d.tab = t.0;
    }
}

/// 선택 패널 "외형" 버튼 — 소유 주민 선택 시에만 노출(chars.rs가 스폰).
#[derive(Component)]
pub struct SelLookButton;

/// 외형 버튼 클릭 — 편집기 오픈(상태 초기화 + 카메라 포커스).
pub fn on_sel_look_click(
    _: On<Pointer<Click>>,
    sel: Res<crate::world::pick::Selection>,
    store: Res<LookStore>,
    mut editor: ResMut<LookEditor>,
    mut state: ResMut<UiState>,
    bridge: Res<crate::world::look_edit::LookEditBridge>,
) {
    let Some(id) = sel.id.as_deref() else { return };
    let Some(m) = store.find(id) else { return };
    editor.data = Some(EditorData::from_me(&m));
    state.popup = Some(Popup::Look);
    bridge.push_focus(true);
}

/// 색 점 선택 — parts 갱신 + 낙관적 재스폰 + 저장.
pub(super) fn on_swatch_click(
    trigger: On<Pointer<Click>>,
    dots: Query<&LookSwatch>,
    mut editor: ResMut<LookEditor>,
    bridge: Res<crate::world::look_edit::LookEditBridge>,
    store: Res<LookStore>,
) {
    let Ok(dot) = dots.get(trigger.entity) else {
        return;
    };
    let Some(d) = editor.data.as_mut() else {
        return;
    };
    d.parts.insert(dot.0.to_owned(), dot.1.clone());
    apply_change(d, &bridge, &store);
}

/// ◀▶ 슬라이드 — 행 대상 목록에서 순환(잠긴 옵션 스킵 — 웹과 동일).
pub(super) fn on_slide_click(
    trigger: On<Pointer<Click>>,
    navs: Query<&LookSlide>,
    mut editor: ResMut<LookEditor>,
    bridge: Res<crate::world::look_edit::LookEditBridge>,
    store: Res<LookStore>,
) {
    let Ok(nav) = navs.get(trigger.entity) else {
        return;
    };
    let target = nav.0.clone();
    let Some(d) = editor.data.as_mut() else {
        return;
    };
    let opts: Vec<(String, bool)> = match &target {
        RowTarget::Wear(slot) => wear_options(d, slot),
        RowTarget::Part(key) => part_options(d, key)
            .iter()
            .map(|k| ((*k).to_owned(), false))
            .collect(),
    };
    let current = match &target {
        RowTarget::Wear(slot) => d.wears.get(*slot).cloned().unwrap_or("auto".into()),
        RowTarget::Part(key) => d.part(key, &part_options(d, key)),
    };
    let i = opts.iter().position(|(k, _)| *k == current).unwrap_or(0) as i32;
    let n = opts.len() as i32;
    for step in 1..=n {
        let cand = &opts[((i + nav.1 as i32 * step).rem_euclid(n)) as usize];
        if !cand.1 {
            match &target {
                RowTarget::Wear(slot) => {
                    if cand.0 == "auto" {
                        d.wears.remove(*slot);
                    } else {
                        d.wears.insert((*slot).to_owned(), cand.0.clone());
                    }
                }
                RowTarget::Part(key) => {
                    d.parts.insert((*key).to_owned(), cand.0.clone());
                }
            }
            break;
        }
    }
    apply_change(d, &bridge, &store);
}

/// 착용 옵션 — (키, 잠김). 웹 wearOptions: auto + 카탈로그(미소지 잠금) + 발명.
pub(super) fn wear_options(d: &EditorData, slot: &str) -> Vec<(String, bool)> {
    let mut out = vec![("auto".to_owned(), false)];
    for c in look_data::CLOTHES.iter().filter(|c| c.slot == slot) {
        out.push((c.item.to_owned(), !d.owned(c.item)));
    }
    for c in &d.inv_clothes {
        out.push((c.name.clone(), false));
    }
    out
}

/// parts 옵션 목록 — 키별 버킷 표 조회.
pub(super) fn part_options<'a>(d: &'a EditorData, key: &str) -> Vec<&'a str> {
    match key {
        "hair" => d.hairs().to_vec(),
        "hairColor" => d.hair_colors().to_vec(),
        "skin" => look_data::SKINS.to_vec(),
        "eye" => look_data::EYES.to_vec(),
        "beard" => d.beards().to_vec(),
        "glasses" => d.glasses().to_vec(),
        _ => vec![],
    }
}

/// 선택 변경 공통 처리 — 낙관적 look push + 저장 적재.
fn apply_change(
    d: &EditorData,
    bridge: &crate::world::look_edit::LookEditBridge,
    store: &LookStore,
) {
    let parts = d.full_parts_json();
    bridge.push_look(&d.agent_id, &parts);
    store.enqueue_save(&d.agent_id, d.save_body(d.tab == Tab::Clothes));
}

/// 팝업 표시·행 재구성·종료 에지 정리.
#[allow(clippy::type_complexity)]
pub fn refresh(
    state: Res<UiState>,
    mut editor: ResMut<LookEditor>,
    store: Res<LookStore>,
    theme: Res<UiTheme>,
    mut commands: Commands,
    mut q_root: Query<&mut Visibility, (With<LookPopupRoot>, Without<LookRows>)>,
    q_rows: Query<(Entity, Option<&Children>), (With<LookRows>, Without<LookPopupRoot>)>,
    mut q_status: Query<&mut Text, (With<LookStatus>, Without<LookRows>)>,
    mut q_title: Query<&mut Text, (With<LookTitle>, Without<LookStatus>, Without<LookRows>)>,
    mut sig: ResMut<LookUiSig>,
    bridge: Res<crate::world::look_edit::LookEditBridge>,
) {
    let open = state.popup == Some(Popup::Look) && editor.data.is_some();
    if let Ok(mut v) = q_root.single_mut() {
        *v = if open {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    // 종료 에지 — 편집 상태·포커스 해제(look 오버라이드는 서버 수렴 시 apply 제거).
    if !open && !sig.0.is_empty() {
        bridge.push_focus(false);
        editor.data = None;
        sig.0.clear();
    }
    // 상태 라벨 — look_sync 저장 결과.
    let status = store
        .inner
        .status
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .clone();
    let label = match status.as_str() {
        "saving" => "저장 중…".to_owned(),
        "ok" => "✓ 자동 저장됨".to_owned(),
        s if s.starts_with("err") => format!("저장 실패({}) — 잠시 후 재시도", &s[4..]),
        _ => String::new(),
    };
    if let Ok(mut t) = q_status.single_mut()
        && **t != label
    {
        **t = label;
    }
    if !open {
        return;
    }
    let Some(d) = editor.data.as_ref() else {
        return;
    };
    if let Ok(mut q_title) = q_title.single_mut() {
        let title = format!("외형 편집 — {}", d.name);
        if **q_title != title {
            **q_title = title;
        }
    }
    // 재구성 서명 — 데이터·탭이 바뀔 때만 행 스폰(멱등).
    let mut new_sig = format!("{}|{:?}|", d.agent_id, d.tab);
    for (k, v) in &d.parts {
        new_sig.push_str(&format!("{k}={v};"));
    }
    for (k, v) in &d.wears {
        new_sig.push_str(&format!("w{k}={v};"));
    }
    if new_sig == sig.0 {
        return;
    }
    sig.0 = new_sig;
    let Ok((rows_e, children)) = q_rows.single() else {
        return;
    };
    for c in children.map(|c| c.to_vec()).unwrap_or_default() {
        commands.entity(c).despawn();
    }
    commands.entity(rows_e).with_children(|rows| {
        super::look_view::build_rows(rows, &theme, d);
    });
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
