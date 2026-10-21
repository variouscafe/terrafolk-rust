//! UI 다국어(ko/en) — 단일 번역 테이블. 언어는 UiState.lang(메뉴 토글).
//! 데이터 기반 라벨(아이템·시대·계절 등)은 vocab.rs(어셋 로드)가 담당 —
//! 여긴 UI 크롬 문장만 둔다. 서버 데이터(주민 이름)는 정본 그대로 통과.

/// 언어 — 레거시 FE 4종(ko/en/zh/ja, i18n.ts LOCAILES 순 = labels[] 인덱스)
#[derive(Clone, Copy, PartialEq, Eq, Hash, Default, Debug)]
pub enum Lang {
    #[default]
    Ko,
    En,
    Zh,
    Ja,
}

impl Lang {
    /// MT 타깃 코드(/worldsim/translate _TARGETS와 동일)
    pub fn code(self) -> &'static str {
        match self {
            Lang::Ko => "ko",
            Lang::En => "en",
            Lang::Zh => "zh",
            Lang::Ja => "ja",
        }
    }

    /// labels[4] 인덱스(vocab 로케일 순서)
    pub fn idx(self) -> usize {
        match self {
            Lang::Ko => 0,
            Lang::En => 1,
            Lang::Zh => 2,
            Lang::Ja => 3,
        }
    }
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
    ResInventions, // "마을의 발명"
    ResInvEmpty,
    // 유물 카드(§17.38 — 맵 발명품 탭)
    RelicCard, // "발명품"
    RelicBy, // "만든 이"
    // 경제
    Economy,
    EcoPrices,
    EcoDeals,
    EcoWait,
    EcoGoldShare, // "금 결제 {p}%"
    EcoVolume,
    EcoGoldDeals,
    EcoUpdated, // "{d}일차 시세"
    EcoColItem,
    EcoColPrice,
    EcoColTrades,
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

/// 번역 조회 — 키별 (ko, en, zh, ja) 사중목
pub fn tr(lang: Lang, key: Tr) -> &'static str {
    let (ko, en, zh, ja) = match key {
        Tr::Demo => ("데모", "Demo", "演示", "デモ"),
        Tr::Connecting => ("◌ 접속중", "◌ Connecting", "◌ 连接中", "◌ 接続中"),
        Tr::Reconnecting => ("◌ 재접속", "◌ Reconnecting", "◌ 重新连接", "◌ 再接続"),
        Tr::Participating => ("참여 중", "Participating", "参与中", "参加中"),
        Tr::Spectating => ("관전", "Spectating", "观战", "観戦"),
        Tr::ToggleVillagers => ("캐릭터", "Characters", "角色", "住民"),
        Tr::ToggleChat => ("대화", "Chat", "对话", "会話"),
        Tr::ToggleEvents => ("기록", "Log", "记录", "記録"),
        Tr::ToggleMinimap => ("지도", "Map", "地图", "地図"),
        Tr::ToggleCiv => ("문명", "Civ", "文明", "文明"),
        Tr::ToggleRelations => ("인연", "Bonds", "羁绊", "絆"),
        Tr::ToggleEconomy => ("시세", "Trade", "行情", "相場"),
        Tr::ToggleResources => ("자원", "Resources", "资源", "資源"),
        Tr::ToggleSettings => ("설정", "Settings", "设置", "設定"),
        Tr::Population => ("{}명", "{} here", "{}人", "{}人"),
        Tr::Villagers => ("주민", "Villagers", "居民", "住民"),
        Tr::VillagersN => ("주민 ({})", "Residents ({})", "居民 ({})", "住民 ({})"),
        Tr::Act => ("활동", "Act", "活动", "行動"),
        Tr::Goal => ("목표", "Goal", "目标", "目標"),
        Tr::Hunger => ("배고픔", "Hunger", "饥饿", "空腹"),
        Tr::Thirst => ("갈증", "Thirst", "口渴", "喉の渇き"),
        Tr::Fatigue => ("피로", "Fatigue", "疲劳", "疲労"),
        Tr::Warmth => ("추위", "Cold", "寒冷", "寒さ"),
        Tr::History => ("생활기록", "History", "生活记录", "生活記録"),
        Tr::Inventory => ("소지품", "Inventory", "携带品", "所持品"),
        Tr::InventoryEmpty => ("(비어 있음)", "(empty)", "（空）", "（空）"),
        Tr::Carry => ("짐 {} / {}", "Carry {} / {}", "负重 {} / {}", "積載 {} / {}"),
        Tr::Age => ("나이 {}", "Age {}", "年龄 {}", "年齢 {}"),
        Tr::Sex => ("성별", "Sex", "性别", "性別"),
        Tr::Female => ("여", "F", "女", "女性"),
        Tr::Male => ("남", "M", "男", "男性"),
        Tr::Unknown => ("?", "?", "未知", "不明"),
        Tr::SelectHint => ("목록에서 캐릭터를 눌러 상세를 보세요", "Pick a resident to see details", "从列表中选择居民即可查看详情", "リストから住民を選ぶと詳細が見られます"),
        Tr::LiveDetailHint => ("라이브 접속 시 상세·욕구가 표시됩니다", "Connect live to see needs & details", "等待同步…", "同期待ち…"),
        Tr::Chat => ("대화", "Chat", "聊天", "チャット"),
        Tr::ChatHint => ("선택한 주민에게 대화…", "Talk to selected villager…", "输入消息…", "メッセージを入力…"),
        Tr::ChatHintMine => ("{}에게 지시…", "Direct {}…", "和你的角色说话…", "あなたの住民に話しかける…"),
        Tr::ChatTalk => ("{}에게 말걸기…", "Talk to {}…", "向 {} 搭话…", "{} に話しかける…"),
        Tr::ChatSend => ("보내기", "Send", "发送", "送信"),
        Tr::ChatEmpty => ("아직 대화가 없습니다 — 첫 마디를 건네보세요", "No talk yet — say hello!", "（还没有对话）", "（まだ会話はありません）"),
        Tr::ChatOffline => ("라이브 접속 후 대화할 수 있습니다", "Connect live to chat", "未连接 — 无法对话", "未接続 — 会話できません"),
        Tr::ChatWaiting => ("(대기 중… 주민들이 대화하면 여기에 표시됩니다)", "(Waiting… villager chatter appears here)", "正在连接村庄…", "村に接続中…"),
        Tr::Deselect => ("선택 해제", "Deselect", "取消选择", "選択解除"),
        Tr::Events => ("기록", "Log", "记录", "記録"),
        Tr::EventsEmpty => ("(아직 기록이 없다)", "(No records yet)", "（还没有记录）", "（まだ記録はありません）"),
        Tr::EvTabAll => ("전체", "All", "全部", "すべて"),
        Tr::EvTabHighlight => ("하이라이트", "Highlights", "精彩", "ハイライト"),
        Tr::EvTabSocial => ("사회·인연", "Social", "社会", "社会"),
        Tr::EvTabLiving => ("생활·교역", "Living", "生活", "生活"),
        Tr::EvTabKnowledge => ("지식·기술", "Knowledge", "知识", "知識"),
        Tr::EvTabLife => ("생애", "Life", "起居", "暮らし"),
        Tr::EvTabWorld => ("세계", "World", "世界", "世界"),
        Tr::Civ => ("문명", "Civilization", "文明", "文明"),
        Tr::CivCount => ("발명 {}건", "{} inventions", "发明 {} 件", "発明 {}件"),
        Tr::CivScore => ("문명 점수 {}", "Civ score {}", "文明分数 {}", "文明スコア {}"),
        Tr::CivEmpty => ("(아직 발명이 없다 — 주민들이 연구 중…)", "(No inventions yet — villagers are researching…)", "（还没有发明 — 居民们正在研究…）", "（まだ発明がありません — 住民たちは研究中…）"),
        Tr::CivDayN => ("{}일차", "Day {}", "第 {} 天", "{}日目"),
        Tr::CivMakers => ("만든 이들", "Makers", "发明者们", "発明者たち"),
        Tr::CivTraditions => ("살아있는 전통", "Living traditions", "活着的传统", "生きる伝統"),
        Tr::CivTradDepth => ("{}회 거행 · 깊이 {}", "{} performs · depth {}", "举行 {} 次 · 深度 {}", "施行 {}回 · 深さ {}"),
        Tr::CivLastPerform => ("마지막 거행 D+{}", "Last performed D+{}", "最后施行 D+{}", "最後の施行 D+{}"),
        Tr::CivChronicle => ("마을 연대기", "Village chronicle", "村庄编年史", "村の年代記"),
        Tr::CivInventor => ("발명가", "Inventor", "发明者", "発明者"),
        Tr::CivMaterials => ("재료", "Materials", "材料", "材料"),
        Tr::CivImproved => ("{}의 개선형", "Improved on {}", "改进自 {}", "{} の改良型"),
        Tr::CivDate => ("{}년 {}월 {}일", "Y{} · {}/{}", "{}年{}月{}日", "{}年{}月{}日"),
        Tr::Relations => ("인연", "Bonds", "羁绊", "絆"),
        Tr::RelFor => ("{}의 인연", "{}'s bonds", "{} 的羁绊", "{} の絆"),
        Tr::RelAll => ("마을 전체", "Whole village", "全体", "全員"),
        Tr::RelWaitLive => ("라이브 접속 시 표시됩니다", "Available when live-connected", "连接后可见", "接続すると表示されます"),
        Tr::RelWaitData => ("(인연 집계 대기 중…)", "(Gathering bonds…)", "（还没有羁绊数据）", "（まだ絆のデータがありません）"),
        Tr::RelChatsTrades => ("대화 {}회 · 거래 {}회", "{} chats · {} trades", "对话 {} 次 · 交易 {} 次", "会話 {}回 · 取引 {}回"),
        Tr::ResUsable => ("채집 가능 {} / {}", "Harvestable {} / {}", "可采集 {} / {}", "採集可能 {} / {}"),
        Tr::ResEmpty => ("(주변에 채집할 것이 없다)", "(Nothing to gather nearby)", "（附近没有可采集的东西）", "（近くに採れるものがありません）"),
        Tr::ResInventions => ("마을의 발명", "Village inventions", "村庄的发明", "村の発明"),
        Tr::ResInvEmpty => ("(아직 없다)", "(None yet)", "（还没有）", "（まだありません）"),
        Tr::RelicCard => ("발명품", "Invention", "发明物", "発明品"),
        Tr::RelicBy => ("만든 이", "Made by", "制作者", "作り手"),
        Tr::Economy => ("시세", "Economy", "行情", "経済"),
        Tr::EcoPrices => ("시세 — 참조", "Reference prices", "物价", "物价"),
        Tr::EcoDeals => ("최근 성사", "Recent deals", "最近的交易", "最近の取引"),
        Tr::EcoWait => ("(거래가 성사되면 시세가 올라온다)", "(Prices appear once deals happen)", "等待行情数据…", "相場データ待ち…"),
        Tr::EcoGoldShare => ("금 결제 {}%", "Gold share {}%", "金币结算 {}%", "金貨決済 {}%"),
        Tr::EcoVolume => ("누적 거래 {}", "{} trades", "交易量 {}", "取引量 {}"),
        Tr::EcoGoldDeals => ("금 거래 {}건", "{} gold deals", "金币交易 {}", "金貨取引 {}"),
        Tr::EcoUpdated => ("{}일차 기준", "As of day {}", "第 {} 天行情", "{}日目の相場"),
        Tr::EcoColItem => ("품목", "Item", "品目", "品目"),
        Tr::EcoColPrice => ("금가", "Gold", "金价", "金額"),
        Tr::EcoColTrades => ("성사", "Trades", "成交", "成約"),
        Tr::Menu => ("메뉴", "Menu", "菜单", "メニュー"),
        Tr::MenuSnow => ("눈 스킨(강제)", "Snow skin (force)", "雪景（覆盖季节）", "雪景色（季節を上書き）"),
        Tr::MenuClose => ("닫기", "Close", "关闭", "閉じる"),
        Tr::MenuHint1 => ("Terrafolk 2D — 라이브 관전", "Terrafolk 2D — Live spectator", "拖动地图 · 滚轮/双指缩放", "地図をドラッグ · ホイール/ピンチで拡大"),
        Tr::MenuHint2 => ("드래그 팬 · 휠 줌 · 주민 클릭 선택/추적", "Drag pan · Wheel zoom · Click villager to select/follow", "点击居民即可选择并跟随", "住民をタップすると選択して追従"),
        Tr::Minimap => ("미니맵", "Minimap", "小地图", "ミニマップ"),
        Tr::TipTree => ("나무", "Tree", "树", "木"),
        Tr::TipPlant => ("식물", "Plant", "植物", "植物"),
        Tr::TipGoldVein => ("금맥", "Gold vein", "金矿", "金脈"),
        Tr::TipAnimal => ("동물", "Animal", "动物", "動物"),
        Tr::TipCampfire => ("모닥불", "Campfire", "篝火", "たき火"),
        Tr::TipHut => ("건축물", "Building", "小屋", "小屋"),
        Tr::TipGrave => ("무덤", "Grave", "坟墓", "墓"),
        Tr::TipStash => ("저장고", "Stash", "储藏点", "隠し場所"),
        Tr::TipRelic => ("유물", "Relic", "遗物", "遺物"),
        Tr::TipName => ("이름: {}", "Name: {}", "名字", "名前"),
        Tr::TipOwner => ("주인: {}", "Owner: {}", "主人", "持ち主"),
        Tr::TipMaturity => ("성장: {}/3", "Growth: {}/3", "成长：{}/3", "育ち：{}/3"),
        Tr::TipHarvest => ("수확 가능", "Harvestable", "可采集", "採集できる"),
        Tr::TipRegrow => ("재생 중", "Regrowing", "再生长中", "再生中"),
        Tr::TipAlive => ("생명: 살아있음", "Alive", "存活", "生存"),
        Tr::TipDead => ("생명: 죽음", "Dead", "死亡", "死亡"),
        Tr::TipBuild => ("공사: {}", "Build: {}", "施工：{}", "建設中：{}"),
        Tr::TipBuildPct => ("공사: {} {}%", "Build: {} {}%", "施工：{} {}%", "建設中：{} {}%"),
        Tr::TipGraveAge => ("향년 {}세", "Aged {}", "享年 {} 岁", "享年 {}歳"),
        Tr::TipBuildOwner => ("{}의 집", "{}'s home", "{} 的家", "{} の家"),
        Tr::TipBuildEmpty => ("빈집", "Empty house", "空屋", "空き家"),
        Tr::TipBuildSite => ("{}의 부지", "{}'s site", "{} 的地基", "{} の建設地"),
        Tr::StIdle => ("휴식", "Idle", "待机", "待機"),
        Tr::StMoving => ("이동", "Moving", "移动中", "移動中"),
        Tr::StActing => ("행동", "Acting", "行动中", "行動中"),
        Tr::StThinking => ("생각", "Thinking", "思考中", "思考中"),
        Tr::StSleeping => ("취침", "Sleeping", "睡眠中", "睡眠中"),
        Tr::SunTimes => ("일출 {} · 일몰 {}", "Sunrise {} · Sunset {}", "日出 {} · 日落 {}", "日の出 {} · 日の入り {}"),
        Tr::WarmBad => ("사이가 나쁨", "Bad terms", "关系不好", "不仲"),
        Tr::WarmAwkward => ("어색", "Awkward", "有些尴尬", "ぎこちない"),
        Tr::WarmNormal => ("보통", "Neutral", "普通", "普通"),
        Tr::WarmGood => ("친함", "Close", "亲近", "親しい"),
        Tr::WarmBest => ("베프", "Best friends", "深厚", "深い絆"),
        Tr::SeasonSpring => ("봄", "Spring", "春", "春"),
        Tr::SeasonSummer => ("여름", "Summer", "夏", "夏"),
        Tr::SeasonAutumn => ("가을", "Autumn", "秋", "秋"),
        Tr::SeasonWinter => ("겨울", "Winter", "冬", "冬"),
        Tr::WeatherClear => ("맑음", "Clear", "晴", "晴れ"),
        Tr::WeatherOvercast => ("흐림", "Overcast", "阴", "曇り"),
        Tr::WeatherRain => ("비", "Rain", "雨", "雨"),
        Tr::WeatherStorm => ("폭풍", "Storm", "暴风雨", "嵐"),
        Tr::WeatherSnow => ("눈", "Snow", "雪", "雪"),
        Tr::WeatherFog => ("안개", "Fog", "雾", "霧"),
    };
    match lang {
        Lang::Ko => ko,
        Lang::En => en,
        Lang::Zh => zh,
        Lang::Ja => ja,
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
    use egui_phosphor::regular as ph;
    match act {
        "chop" => ph::AXE,
        "mine" => ph::MOUNTAINS,
        "forage" => ph::PLANT,
        "hunt" => ph::CROSSHAIR,
        "eat" => ph::FORK_KNIFE,
        "drink" => ph::DROP,
        "campfire" => ph::CAMPFIRE,
        "warm" => ph::FIRE,
        "wear" => ph::T_SHIRT,
        "give" => ph::GIFT,
        "trade" => ph::ARROWS_LEFT_RIGHT,
        "gather" => ph::HAND,
        "build" => ph::HAMMER,
        "invent" => ph::LIGHTBULB,
        "craft" => ph::WRENCH,
        "perform" => ph::MASK_HAPPY,
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
