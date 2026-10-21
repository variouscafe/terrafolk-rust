//! 채팅 송신(F1) — 선택 주민과의 대화 입력 바. 안드로이드 소프트키보드는
//! winit `ime_enabled`(→show_soft_input)로 띄우고, 한글 조합은 Ime::Preedit,
//! 확정은 Ime::Commit로 수신한다(하드웨어 키보드는 KeyboardInput.text).
//! 서버 계약(room/chat.ts handleChat): 관전자 agentId 지정 → converse.
//! 송신하면 서버가 전체 방송(chat) — 기존 말풍선·피드 경로로 화면 반영.

use bevy::input::keyboard::KeyboardInput;
use bevy::picking::Pickable;
use bevy::prelude::*;

use super::theme::{self, UiTheme};
use super::world_overlay::OverlayRoot;
use crate::net::native_conn::send_chat;
use crate::world::pick::Selection;

/// 채팅 입력 상태(모달 단일 — 한 번에 한 대화).
#[derive(Resource, Default)]
pub struct ChatState {
    pub active: bool,
    buffer: String,
    preedit: String,
}

/// 입력 바 루트·본문 텍스트 마커.
#[derive(Component)]
pub struct ChatBar;
#[derive(Component)]
pub struct ChatText;
#[derive(Component)]
pub struct ChatSend;

/// 채팅 바 스폰 — 선택 패널 위 하단. 기본 숨김(전 노드 IGNORE — 월드 탭 보존).
pub fn spawn(mut commands: Commands, theme: Res<UiTheme>) {
    commands.init_resource::<ChatState>();
    commands
        .spawn((
            ChatBar,
            Node {
                position_type: PositionType::Absolute,
                bottom: Val::Px(96.0),
                left: Val::Px(8.0),
                right: Val::Px(8.0),
                padding: UiRect::axes(Val::Px(8.0), Val::Px(6.0)),
                column_gap: Val::Px(6.0),
                ..default()
            },
            BackgroundColor(theme::INK900),
            Visibility::Hidden,
            Pickable::IGNORE,
        ))
        .with_children(|bar| {
            bar.spawn((
                ChatText,
                Text::new(" "),
                theme.text(11.0, theme::INK50),
                Node {
                    flex_grow: 1.0,
                    min_height: Val::Px(18.0),
                    ..default()
                },
            ));
            bar.spawn((
                Button,
                Interaction::default(),
                Text::new("인사"),
                theme.label(10.0),
                Node {
                    padding: UiRect::axes(Val::Px(8.0), Val::Px(5.0)),
                    ..default()
                },
                BackgroundColor(theme::INK600),
            ))
            .observe(on_greet);
            bar.spawn((
                ChatSend,
                Button,
                Interaction::default(),
                Text::new("보내기"),
                theme.label(10.0),
                Node {
                    padding: UiRect::axes(Val::Px(9.0), Val::Px(5.0)),
                    ..default()
                },
                BackgroundColor(theme::ACCENT),
            ))
            .observe(on_send);
            bar.spawn((
                Button,
                Interaction::default(),
                Text::new("닫기"),
                theme.label(10.0),
                Node {
                    padding: UiRect::axes(Val::Px(9.0), Val::Px(5.0)),
                    ..default()
                },
                BackgroundColor(theme::INK700),
            ))
            .observe(on_close);
        });
}

/// 선택 패널 💬 칩 → 채팅 열기(chars.rs가 스폰·observe).
pub fn open_chat(
    _: On<Pointer<Click>>,
    mut chat: ResMut<ChatState>,
    mut window: Single<&mut Window>,
) {
    chat.active = true;
    window.ime_enabled = true; // 소프트키보드
}

fn on_close(
    _: On<Pointer<Click>>,
    mut chat: ResMut<ChatState>,
    mut window: Single<&mut Window>,
) {
    close(&mut chat, &mut window);
}

/// 빠른 인사 — 키보드 없이 한 번에 송신(모바일 진입 장벽 완화 + E2E 검증 경로).
fn on_greet(_: On<Pointer<Click>>, sel: Res<Selection>, mut window: Single<&mut Window>) {
    let chat = ChatState {
        active: true,
        buffer: "안녕! 반가워".into(),
        preedit: String::new(),
    };
    if send(&chat, &sel) {
        window.ime_enabled = false;
    }
}

fn on_send(
    _: On<Pointer<Click>>,
    mut chat: ResMut<ChatState>,
    sel: Res<Selection>,
    mut window: Single<&mut Window>,
) {
    if send(&chat, &sel) {
        close(&mut chat, &mut window);
    }
}

fn close(chat: &mut ChatState, window: &mut Window) {
    chat.active = false;
    chat.buffer.clear();
    chat.preedit.clear();
    window.ime_enabled = false;
}

/// 송신 공용 — Enter(하드웨어)·보내기 버튼 양쪽이 호출.
fn send(chat: &ChatState, sel: &Selection) -> bool {
    let text = format!("{}{}", chat.buffer, chat.preedit);
    let text = text.trim();
    if text.is_empty() {
        return false;
    }
    let Some(agent) = sel.id.as_deref() else {
        info!("[chat] 대상 주민 없음 — 송신 스킵");
        return false;
    };
    let ok = send_chat(agent, text);
    if ok {
        info!(
            "[chat] 송신 → {agent}: {}",
            text.chars().take(40).collect::<String>()
        );
    }
    ok
}

/// IME·키보드 입력 수집(안드로이드 확정·하드웨어 타이핑·백스페이스·Enter).
pub fn chat_input(
    mut chat: ResMut<ChatState>,
    mut ime: MessageReader<bevy::window::Ime>,
    mut keys: MessageReader<KeyboardInput>,
    sel: Res<Selection>,
    mut window: Single<&mut Window>,
) {
    if !chat.active {
        ime.clear();
        keys.clear();
        return;
    }
    for ev in ime.read() {
        match ev {
            bevy::window::Ime::Commit { value, .. } => {
                chat.buffer.push_str(value);
                chat.preedit.clear();
            }
            bevy::window::Ime::Preedit { value, .. } => chat.preedit = value.clone(),
            _ => {}
        }
    }
    for ev in keys.read() {
        if !ev.state.is_pressed() {
            continue;
        }
        match ev.key_code {
            KeyCode::Backspace => {
                chat.preedit.clear();
                chat.buffer.pop();
            }
            KeyCode::Enter | KeyCode::NumpadEnter => {
                if send(&chat, &sel) {
                    close(&mut chat, &mut window);
                }
            }
            KeyCode::Escape => close(&mut chat, &mut window),
            _ => {
                if let Some(t) = ev.text.as_ref()
                    && !t.is_empty()
                {
                    chat.buffer.push_str(t);
                }
            }
        }
    }
}

/// 자동 인사(TF_AUTO_GREET) — 부팅 후 첫 주민 선택 + 인사 송신(채팅 E2E
/// 개발 스위치 — UI 입력 없이 송신→서버 방송→수신 경로 전체를 로그로 검증).
pub fn auto_greet(
    time: Res<Time>,
    villagers: Query<(Entity, &crate::world::villager::Villager, Option<&crate::net::apply::LiveAgentTag>)>,
    mut sel: ResMut<Selection>,
    mut done: Local<bool>,
) {
    if *done || std::env::var("TF_AUTO_GREET").is_err() || time.elapsed_secs() < 8.0 {
        return;
    }
    // 첫 라이브 주민(없으면 데모) 선택 — 목록 1행과 동일 순서(이름순 아님 단순 첫)
    let pick = villagers
        .iter()
        .find(|(_, _, t)| t.is_some())
        .map(|(e, _, t)| (e, t.unwrap().0.clone()))
        .or_else(|| villagers.iter().next().map(|(e, _v, _)| (e, format!("demo:{}", e.index()))));
    let Some((entity, id)) = pick else { return; };
    sel.id = Some(id.clone());
    sel.entity = Some(entity);
    sel.epoch = sel.epoch.wrapping_add(1);
    info!("[chat] TF_AUTO_GREET — {id} 선택, 송신 개시");
    let chat = ChatState {
        active: true,
        buffer: "안녕! 반가워".into(),
        preedit: String::new(),
    };
    send(&chat, &sel);
    *done = true;
}

/// 빠른 인사 키(G) — 선택 주민에게 즉시 인사 송신(키보드·바 없는 검증 경로).
pub fn quick_greet_key(keys: Res<ButtonInput<KeyCode>>, sel: Res<Selection>) {
    if !keys.just_pressed(KeyCode::KeyG) {
        return;
    }
    let chat = ChatState {
        active: true,
        buffer: "안녕! 반가워".into(),
        preedit: String::new(),
    };
    if sel.id.is_some() {
        send(&chat, &sel);
    }
}

/// 바 본문 갱신 — 버퍼 + 조합중(‸ 접두로 구분) + 미선택 안내.
pub fn refresh_chat_bar(
    chat: Res<ChatState>,
    sel: Res<Selection>,
    mut q_text: Query<&mut Text, With<ChatText>>,
    mut q_bar: Query<&mut Visibility, (With<ChatBar>, Without<OverlayRoot>)>,
) {
    let show = chat.active;
    if let Ok(mut v) = q_bar.single_mut() {
        *v = if show {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };
    }
    if let Ok(mut t) = q_text.single_mut() {
        let composing = if chat.preedit.is_empty() {
            String::new()
        } else {
            format!("‸{}", chat.preedit)
        };
        let hint = if sel.id.is_some() {
            ""
        } else {
            " (주민을 먼저 선택)"
        };
        **t = format!("{}{}{}", chat.buffer, composing, hint);
    }
}
