//! UI 다국어(ko/en) — 단일 번역 테이블. 언어는 UiState.lang(메뉴 토글).
//! 데이터 기반 라벨(아이템·시대·계절 등)은 vocab.rs(어셋 로드)가 담당 —
//! 여긴 UI 크롬 문장만 둔다. 서버 데이터(주민 이름)는 정본 그대로 통과.

/// 언어
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    Ko,
    En,
}

/// UI 문자열 키 — 전 UI 크롬의 단일 목록(미사용 키도 사전으로 유지)
#[derive(Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum Tr {
    // 상단바
    Demo,
    Connecting,
    Reconnecting,
    Participating,
    Spectating,
    ToggleVillagers,
    ToggleChat,
    ToggleEvents,
    ToggleMinimap,
    ToggleCiv,
    ToggleRelations,
    ToggleEconomy,
    ToggleResources,
    ToggleSettings,
    Population, // "{n}명"
    // 주민 패널
    Villagers,
    VillagersN, // "주민 ({n})"
    Act,
    Goal,
    Hunger,
    Thirst,
    Fatigue,
    Warmth,
    History,
    Inventory,
    InventoryEmpty,
    Carry,     // "짐 {w}/{cap}"
    Age,       // "나이 {n}"
    Sex,
    Female,
    Male,
    Unknown,
    SelectHint,
    LiveDetailHint,
    // 채팅
    Chat,
    ChatHint,
    ChatHintMine,
    ChatTalk, // "{}에게 말걸기…" — 참여 캐릭터가 아닌 주민
    ChatSend,
    ChatWaiting,
    ChatEmpty, // 대화창 빈 로그 안내
    ChatOffline, // 라이브 미접속 — 컴포저 비활성 안내
    Deselect,
    // 기록(연표)
    Events,
    EventsEmpty,
    EvTabAll,
    EvTabHighlight,
    EvTabSocial,
    EvTabLiving,
    EvTabKnowledge,
    EvTabLife,
    EvTabWorld,
    // 문명
    Civ,
    CivCount, // "발명 {n}건"
    CivScore,
    CivEmpty,
    CivDayN,
    CivMakers,
    CivTraditions,
    CivTradDepth, // "{n}회 거행 · 깊이 {d}"
    CivLastPerform, // "마지막 거행 D+{n}"
    CivChronicle,
    CivInventor,
    CivMaterials,
    CivImproved, // "{x}의 개선형"
    CivDate,     // "{y}년 {m}월 {d}일"
    // 관계
    Relations,
    RelFor,   // "{name}의 인연"
    RelAll,
    RelWaitLive,
    RelWaitData,
    RelChatsTrades, // "대화 {n}회 · 거래 {m}회"
    // 자원
    ResUsable, // "채집 가능 {} / {}"
    ResEmpty,
    // 경제
    Economy,
    EcoPrices,
    EcoDeals,
    EcoWait,
    EcoGoldShare, // "금 결제 {p}%"
    EcoVolume,
    EcoGoldDeals,
    EcoUpdated, // "{d}일차 시세"
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
    TipBuildPct, // "공사: {s} {p}%"
    TipGraveAge, // "향년 {n}세"
    TipBuildOwner, // "{name}의 집"
    TipBuildEmpty, // "빈집"
    TipBuildSite,  // "{name}의 부지"
    // 주민 상태(서버 고정 어휘 idle|moving|acting|thinking|sleeping)
    StIdle,
    StMoving,
    StActing,
    StThinking,
    StSleeping,
    // 다이얼 툴팁 — 일출/일몰(분 단위 → HH:MM은 호출측)
    SunTimes, // "일출 {} · 일몰 {}"
    // 관계 온기 등급(레거시 REL_BUCKET: <30/<45/<65/<80/≥80)
    WarmBad,
    WarmAwkward,
    WarmNormal,
    WarmGood,
    WarmBest,
    // 계절(폴백 — vocab 로드 전)
    SeasonSpring,
    SeasonSummer,
    SeasonAutumn,
    SeasonWinter,
    // 날씨
    WeatherClear,
    WeatherOvercast,
    WeatherRain,
    WeatherStorm,
    WeatherSnow,
    WeatherFog,
}

/// 번역 조회 — 키별 (ko, en) 쌍
pub fn tr(lang: Lang, key: Tr) -> &'static str {
    let (ko, en) = match key {
        Tr::Demo => ("데모", "Demo"),
        Tr::Connecting => ("◌ 접속중", "◌ Connecting"),
        Tr::Reconnecting => ("◌ 재접속", "◌ Reconnecting"),
        Tr::Participating => ("참여 중", "Participating"),
        Tr::Spectating => ("관전", "Spectating"),
        Tr::ToggleVillagers => ("캐릭터", "Characters"),
        Tr::ToggleChat => ("대화", "Chat"),
        Tr::ToggleEvents => ("기록", "Log"),
        Tr::ToggleMinimap => ("지도", "Map"),
        Tr::ToggleCiv => ("문명", "Civ"),
        Tr::ToggleRelations => ("인연", "Bonds"),
        Tr::ToggleEconomy => ("시세", "Trade"),
        Tr::ToggleResources => ("자원", "Resources"),
        Tr::ToggleSettings => ("설정", "Settings"),
        Tr::Population => ("{}명", "{} here"),
        Tr::Villagers => ("주민", "Villagers"),
        Tr::VillagersN => ("주민 ({})", "Residents ({})"),
        Tr::Act => ("활동", "Act"),
        Tr::Goal => ("목표", "Goal"),
        Tr::Hunger => ("배고픔", "Hunger"),
        Tr::Thirst => ("갈증", "Thirst"),
        Tr::Fatigue => ("피로", "Fatigue"),
        Tr::Warmth => ("추위", "Cold"),
        Tr::History => ("생활기록", "History"),
        Tr::Inventory => ("소지품", "Inventory"),
        Tr::InventoryEmpty => ("(비어 있음)", "(empty)"),
        Tr::Carry => ("짐 {} / {}", "Carry {} / {}"),
        Tr::Age => ("나이 {}", "Age {}"),
        Tr::Sex => ("성별", "Sex"),
        Tr::Female => ("여", "F"),
        Tr::Male => ("남", "M"),
        Tr::Unknown => ("?", "?"),
        Tr::SelectHint => ("목록에서 캐릭터를 눌러 상세를 보세요",
            "Pick a resident to see details"),
        Tr::LiveDetailHint => ("라이브 접속 시 상세·욕구가 표시됩니다",
            "Connect live to see needs & details"),
        Tr::Chat => ("대화", "Chat"),
        Tr::ChatHint => ("선택한 주민에게 대화…", "Talk to selected villager…"),
        Tr::ChatHintMine => ("{}에게 지시…", "Direct {}…"),
        Tr::ChatTalk => ("{}에게 말걸기…", "Talk to {}…"),
        Tr::ChatSend => ("보내기", "Send"),
        Tr::ChatEmpty => ("아직 대화가 없습니다 — 첫 마디를 건네보세요", "No talk yet — say hello!"),
        Tr::ChatOffline => ("라이브 접속 후 대화할 수 있습니다", "Connect live to chat"),
        Tr::ChatWaiting => ("(대기 중… 주민들이 대화하면 여기에 표시됩니다)",
            "(Waiting… villager chatter appears here)"),
        Tr::Deselect => ("선택 해제", "Deselect"),
        Tr::Events => ("기록", "Log"),
        Tr::EventsEmpty => ("(아직 기록이 없다)", "(No records yet)"),
        Tr::EvTabAll => ("전체", "All"),
        Tr::EvTabHighlight => ("하이라이트", "Highlights"),
        Tr::EvTabSocial => ("사회·인연", "Social"),
        Tr::EvTabLiving => ("생활·교역", "Living"),
        Tr::EvTabKnowledge => ("지식·기술", "Knowledge"),
        Tr::EvTabLife => ("생애", "Life"),
        Tr::EvTabWorld => ("세계", "World"),
        Tr::Civ => ("문명", "Civilization"),
        Tr::CivCount => ("발명 {}건", "{} inventions"),
        Tr::CivScore => ("문명 점수 {}", "Civ score {}"),
        Tr::CivEmpty => ("(아직 발명이 없다 — 주민들이 연구 중…)",
            "(No inventions yet — villagers are researching…)"),
        Tr::CivDayN => ("{}일차", "Day {}"),
        Tr::CivMakers => ("만든 이들", "Makers"),
        Tr::CivTraditions => ("살아있는 전통", "Living traditions"),
        Tr::CivTradDepth => ("{}회 거행 · 깊이 {}", "{} performs · depth {}"),
        Tr::CivLastPerform => ("마지막 거행 D+{}", "Last performed D+{}"),
        Tr::CivChronicle => ("마을 연대기", "Village chronicle"),
        Tr::CivInventor => ("발명가", "Inventor"),
        Tr::CivMaterials => ("재료", "Materials"),
        Tr::CivImproved => ("{}의 개선형", "Improved on {}"),
        Tr::CivDate => ("{}년 {}월 {}일", "Y{} · {}/{}"),
        Tr::Relations => ("인연", "Bonds"),
        Tr::RelFor => ("{}의 인연", "{}'s bonds"),
        Tr::RelAll => ("마을 전체", "Whole village"),
        Tr::RelWaitLive => ("라이브 접속 시 표시됩니다", "Available when live-connected"),
        Tr::RelWaitData => ("(인연 집계 대기 중…)", "(Gathering bonds…)"),
        Tr::RelChatsTrades => ("대화 {}회 · 거래 {}회", "{} chats · {} trades"),
        Tr::ResUsable => ("채집 가능 {} / {}", "Harvestable {} / {}"),
        Tr::ResEmpty => ("(주변에 채집할 것이 없다)", "(Nothing to gather nearby)"),
        Tr::Economy => ("시세", "Economy"),
        Tr::EcoPrices => ("시세 — 참조", "Reference prices"),
        Tr::EcoDeals => ("최근 성사", "Recent deals"),
        Tr::EcoWait => ("(거래가 성사되면 시세가 올라온다)", "(Prices appear once deals happen)"),
        Tr::EcoGoldShare => ("금 결제 {}%", "Gold share {}%"),
        Tr::EcoVolume => ("누적 거래 {}", "{} trades"),
        Tr::EcoGoldDeals => ("금 거래 {}건", "{} gold deals"),
        Tr::EcoUpdated => ("{}일차 기준", "As of day {}"),
        Tr::Menu => ("메뉴", "Menu"),
        Tr::MenuSnow => ("눈 스킨(강제)", "Snow skin (force)"),
        Tr::MenuClose => ("닫기", "Close"),
        Tr::MenuHint1 => ("Terrafolk 2D — 라이브 관전", "Terrafolk 2D — Live spectator"),
        Tr::MenuHint2 => ("드래그 팬 · 휠 줌 · 주민 클릭 선택/추적",
            "Drag pan · Wheel zoom · Click villager to select/follow"),
        Tr::Minimap => ("미니맵", "Minimap"),
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
        Tr::TipBuildPct => ("공사: {} {}%", "Build: {} {}%"),
        Tr::TipGraveAge => ("향년 {}세", "Aged {}"),
        Tr::TipBuildOwner => ("{}의 집", "{}'s home"),
        Tr::TipBuildEmpty => ("빈집", "Empty house"),
        Tr::TipBuildSite => ("{}의 부지", "{}'s site"),
        Tr::StIdle => ("휴식", "Idle"),
        Tr::StMoving => ("이동", "Moving"),
        Tr::StActing => ("행동", "Acting"),
        Tr::StThinking => ("생각", "Thinking"),
        Tr::StSleeping => ("취침", "Sleeping"),
        Tr::SunTimes => ("일출 {} · 일몰 {}", "Sunrise {} · Sunset {}"),
        Tr::WarmBad => ("사이가 나쁨", "Bad terms"),
        Tr::WarmAwkward => ("어색", "Awkward"),
        Tr::WarmNormal => ("보통", "Neutral"),
        Tr::WarmGood => ("친함", "Close"),
        Tr::WarmBest => ("베프", "Best friends"),
        Tr::SeasonSpring => ("봄", "Spring"),
        Tr::SeasonSummer => ("여름", "Summer"),
        Tr::SeasonAutumn => ("가을", "Autumn"),
        Tr::SeasonWinter => ("겨울", "Winter"),
        Tr::WeatherClear => ("맑음", "Clear"),
        Tr::WeatherOvercast => ("흐림", "Overcast"),
        Tr::WeatherRain => ("비", "Rain"),
        Tr::WeatherStorm => ("폭풍", "Storm"),
        Tr::WeatherSnow => ("눈", "Snow"),
        Tr::WeatherFog => ("안개", "Fog"),
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

/// 서버 상태 문자열 → 현지화 라벨(미지 어휘는 원문 통과)
pub fn status_label(lang: Lang, status: &str) -> &'static str {
    match status {
        "idle" => tr(lang, Tr::StIdle),
        "moving" => tr(lang, Tr::StMoving),
        "acting" => tr(lang, Tr::StActing),
        "thinking" => tr(lang, Tr::StThinking),
        "sleeping" => tr(lang, Tr::StSleeping),
        // 서버가 새 어휘를 보내면 원문 그대로(깨짐 없음)
        _ => "",
    }
}

/// 상태 → 색(레거시 STATUS_COLOR 근사)
pub fn status_color(status: &str) -> bevy_egui::egui::Color32 {
    use super::theme;
    match status {
        "idle" => theme::ST_IDLE,
        "moving" => theme::ST_MOVING,
        "acting" => theme::ST_ACTING,
        "thinking" => theme::ST_THINKING,
        "sleeping" => theme::ST_SLEEPING,
        _ => theme::ST_IDLE,
    }
}

/// 활동 actType → 한자 아이콘(레거시 ACT_ICON 한자 치환 — 이모지 폰트 없음)
pub fn act_icon(act: &str) -> &'static str {
    match act {
        "chop" => "伐",
        "mine" => "採",
        "forage" => "集",
        "hunt" => "狩",
        "eat" => "食",
        "drink" => "水",
        "campfire" => "火",
        "warm" => "溫",
        "wear" => "衣",
        "give" => "贈",
        "trade" => "換",
        "gather" => "拾",
        "build" => "建",
        "invent" => "創",
        "craft" => "作",
        "perform" => "戲",
        _ => "·",
    }
}

/// 이벤트 kind → 한자 아이콘(레거시 EV_KIND_ICON 한자 치환)
pub fn ev_kind_icon(kind: &str) -> &'static str {
    match kind {
        "act" => "動",
        "think" => "思",
        "result" => "成",
        "resource" => "採",
        "trade" => "換",
        "give" => "贈",
        "relation" => "♥",
        "skill" => "技",
        "spawn" => "迎",
        "death" => "逝",
        "invention" => "器",
        "talk" => "話",
        "teach" => "敎",
        "inherit" => "傳",
        "birth" => "生",
        "routine" => "常",
        "system" => "世",
        "chat" => "言",
        "perform" => "戲",
        "chronicle" => "史",
        "home" => "家",
        _ => "·",
    }
}

/// 이벤트 kind → 본문색(레거시 EVENT_COLOR 준거 — 다크 글래스 위 비비드)
pub fn ev_color(kind: &str) -> bevy_egui::egui::Color32 {
    use bevy_egui::egui::Color32;
    match kind {
        "act" => Color32::from_rgb(125, 190, 250),    // sky-300
        "result" => Color32::from_rgb(178, 186, 200), // ink-300
        "think" => Color32::from_rgb(196, 160, 255), // violet-300
        "trade" => Color32::from_rgb(252, 211, 77),  // amber-300
        "skill" => Color32::from_rgb(110, 231, 183), // emerald-300
        "chat" => Color32::from_rgb(178, 186, 200),
        "resource" => Color32::from_rgb(190, 242, 100), // lime-300
        "spawn" => Color32::from_rgb(150, 158, 176),
        "system" => Color32::from_rgb(140, 148, 166),
        "relation" | "give" => Color32::from_rgb(249, 168, 212), // pink-300
        "death" => Color32::from_rgb(212, 212, 216), // zinc-300
        "invention" => Color32::from_rgb(254, 240, 138), // amber-200
        "teach" => Color32::from_rgb(167, 243, 208), // emerald-200
        "inherit" => Color32::from_rgb(186, 230, 253), // sky-200
        "birth" => Color32::from_rgb(251, 207, 232), // pink-200
        "routine" => Color32::from_rgb(221, 214, 254), // violet-200
        "talk" => Color32::from_rgb(254, 240, 160),
        "perform" => Color32::from_rgb(245, 208, 254), // fuchsia-200
        "chronicle" => Color32::from_rgb(254, 240, 160),
        _ => Color32::from_rgb(178, 186, 200),
    }
}

/// 이벤트 kind → 기록 탭 분류(레거시 EV_CAT_OF 직역)
pub fn ev_cat_of(kind: &str) -> &'static str {
    match kind {
        "relation" | "give" | "talk" | "chat" | "perform" => "social",
        "act" | "result" | "trade" | "think" => "living",
        "skill" | "invention" | "teach" | "inherit" | "routine" => "knowledge",
        "spawn" | "birth" | "death" => "life",
        _ => "world",
    }
}

/// 생활기록 type → 색(레거시 HIST_COLOR 준거 — 다크 위 비비드)
pub fn hist_color(type_: &str) -> bevy_egui::egui::Color32 {
    use bevy_egui::egui::Color32;
    match type_ {
        "gain" | "획득" => Color32::from_rgb(110, 231, 183), // emerald-300
        "trade" | "거래" => Color32::from_rgb(252, 211, 77), // amber-300
        "craft" | "제작" => Color32::from_rgb(125, 190, 250), // sky-300
        "consume" | "소모" => Color32::from_rgb(248, 128, 116),
        _ => Color32::from_rgb(178, 186, 200),
    }
}
