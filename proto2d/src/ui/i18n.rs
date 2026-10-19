//! UI 다국어(ko/en) — 단일 번역 테이블. 언어는 UiState.lang(메뉴 토글).
//! 서버 데이터(주민 이름·목표·이벤트 본문)는 정본 그대로 통과.

/// 언어
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    Ko,
    En,
}

/// UI 문자열 키 — 전 UI 크롬의 단일 목록
#[derive(Clone, Copy)]
pub enum Tr {
    // 상단바
    Demo,
    Connecting,
    Reconnecting,
    ToggleVillagers,
    ToggleChat,
    ToggleEvents,
    ToggleMinimap,
    ToggleCiv,
    ToggleRelations,
    DayN, // "{d}일차"/"Day {d}" — d는 별도 format
    // 주민 패널
    Villagers,
    Act,
    Goal,
    Hunger,
    Thirst,
    Fatigue,
    Warmth,
    History,
    // 채팅
    Chat,
    ChatHint,
    ChatSend,
    ChatWaiting,
    // 이벤트
    Events,
    EventsEmpty,
    // 문명
    Civ,
    CivCount,   // "발명 {n}건"
    CivEmpty,
    CivDayN,
    // 관계
    Relations,
    RelWaitLive,
    RelWaitData,
    RelNativeOnly,
    // 메뉴
    Menu,
    MenuSnow,
    MenuClose,
    MenuHint1,
    MenuHint2,
    // 미니맵
    Minimap,
    // 툴팁
    TipTree,
    TipPlant,
    TipGoldVein,
    TipAnimal,
    TipCampfire,
    TipHut,
    TipGrave,
    TipStash,
    TipRelic,
    TipName,
    TipOwner,
    TipMaturity, // "성장: {m}/3"
    TipHarvest,
    TipRegrow,
    TipAlive,
    TipDead,
    TipBuild, // "공사: {s}"
}

/// 번역 조회 — 키별 (ko, en) 쌍
pub fn tr(lang: Lang, key: Tr) -> &'static str {
    let (ko, en) = match key {
        Tr::Demo => ("데모", "Demo"),
        Tr::Connecting => ("◌ 접속중", "◌ Connecting"),
        Tr::Reconnecting => ("◌ 재접속", "◌ Reconnecting"),
        Tr::ToggleVillagers => ("👥 주민", "👥 Villagers"),
        Tr::ToggleChat => ("💬 채팅", "💬 Chat"),
        Tr::ToggleEvents => ("📜 이벤트", "📜 Events"),
        Tr::ToggleMinimap => ("🗺 미니맵", "🗺 Map"),
        Tr::ToggleCiv => ("🏛 문명", "🏛 Civ"),
        Tr::ToggleRelations => ("🤝 관계", "🤝 Bonds"),
        Tr::DayN => ("{}일차", "Day {}"),
        Tr::Villagers => ("주민", "Villagers"),
        Tr::Act => ("활동", "Act"),
        Tr::Goal => ("목표", "Goal"),
        Tr::Hunger => ("허기", "Hunger"),
        Tr::Thirst => ("갈증", "Thirst"),
        Tr::Fatigue => ("피로", "Fatigue"),
        Tr::Warmth => ("체온", "Warmth"),
        Tr::History => ("생활기록", "History"),
        Tr::Chat => ("💬 채팅", "💬 Chat"),
        Tr::ChatHint => ("선택한 주민에게 말걸기…", "Talk to selected villager…"),
        Tr::ChatSend => ("보내기", "Send"),
        Tr::ChatWaiting => ("(대기 중… 주민들이 대화하면 여기에 표시됩니다)",
            "(Waiting… villager chatter appears here)"),
        Tr::Events => ("📜 이벤트", "📜 Events"),
        Tr::EventsEmpty => ("(이벤트 없음)", "(No events)"),
        Tr::Civ => ("🏛 문명", "🏛 Civilization"),
        Tr::CivCount => ("발명 {}건", "{} inventions"),
        Tr::CivEmpty => ("(아직 발명이 없다 — 주민들이 연구 중…)",
            "(No inventions yet — villagers are researching…)"),
        Tr::CivDayN => ("{}일차", "Day {}"),
        Tr::Relations => ("🤝 관계", "🤝 Bonds"),
        Tr::RelWaitLive => ("라이브 접속 시 표시됩니다", "Available when live-connected"),
        Tr::RelWaitData => ("(관계 집계 대기 중…)", "(Gathering bonds…)"),
        Tr::RelNativeOnly => ("관계 상세는 네이티브 앱에서 제공(웹은 라이브 관전만)",
            "Bond details in the native app (web is live spectate only)"),
        Tr::Menu => ("메뉴", "Menu"),
        Tr::MenuSnow => ("❄ 눈 스킨(강제)", "❄ Snow skin (force)"),
        Tr::MenuClose => ("닫기", "Close"),
        Tr::MenuHint1 => ("Terrafolk 2D proto — 라이브 관전", "Terrafolk 2D proto — Live spectator"),
        Tr::MenuHint2 => ("드래그 팬 · 휠 줌 · 주민 클릭 선택/추적",
            "Drag pan · Wheel zoom · Click villager to select/follow"),
        Tr::Minimap => ("🗺 미니맵", "🗺 Minimap"),
        Tr::TipTree => ("나무", "Tree"),
        Tr::TipPlant => ("식물", "Plant"),
        Tr::TipGoldVein => ("금맥", "Gold vein"),
        Tr::TipAnimal => ("동물", "Animal"),
        Tr::TipCampfire => ("모닥불", "Campfire"),
        Tr::TipHut => ("건축물", "Building"),
        Tr::TipGrave => ("무덤", "Grave"),
        Tr::TipStash => ("저장고", "Stash"),
        Tr::TipRelic => ("유물", "Relic"),
        Tr::TipName => ("이름: {}", "Name: {}"),
        Tr::TipOwner => ("주인: {}", "Owner: {}"),
        Tr::TipMaturity => ("성장: {}/3", "Growth: {}/3"),
        Tr::TipHarvest => ("수확 가능", "Harvestable"),
        Tr::TipRegrow => ("재생 중", "Regrowing"),
        Tr::TipAlive => ("생명: 살아있음", "Alive"),
        Tr::TipDead => ("생명: 죽음", "Dead"),
        Tr::TipBuild => ("공사: {}", "Build: {}"),
    };
    match lang {
        Lang::Ko => ko,
        Lang::En => en,
    }
}

/// 편의 — 언어로 바로 포맷되는 문자열(자리표 비포함 키)
pub fn trf(lang: Lang, key: Tr, arg: &str) -> String {
    tr(lang, key).replace("{}", arg)
}
