//! 룸 채팅 → 말풍선 — 주민 머리 위 텍스트 버블(6초·마지막 1초 페이드).
/// 주민당 1개(새 채팅이 이전 버블 교체)·전체 상한 8(오래된 것부터 퇴장).
/// 채팅 송신 UI는 스트레치 — 수신 관전이 MVP(북극성: 주민 생명감).

use bevy::prelude::*;
use bevy::text::{FontSize, FontSource};

use super::{ChatQueue, ChatRow};
use crate::agents::Agent;
use crate::net::apply::LiveAgentTag;

#[derive(Component)]
pub struct SpeechBubble {
    pub ttl: f32,
}

#[derive(Component)]
pub struct BubbleOf(pub Entity);

const TTL: f32 = 6.0;
const MAX_BUBBLES: usize = 8;

/// 채팅 큐 drain → 말풍선 스폰(스피커 agent_id 매칭. 미매칭은 버림)
pub fn drain_and_bubble(
    mut commands: Commands,
    chats: Res<ChatQueue>,
    assets: Res<AssetServer>,
    q_agents: Query<(Entity, &LiveAgentTag), With<Agent>>,
    q_bubbles: Query<(Entity, &BubbleOf)>,
) {
    let rows: Vec<ChatRow> = {
        let Ok(mut q) = chats.inner.lock() else {
            return;
        };
        q.drain(..).collect()
    };
    if rows.is_empty() {
        return;
    }
    let font = assets.load("fonts/NotoSansKR-Regular.otf");
    for row in rows {
        let Some((agent_e, _)) = q_agents.iter().find(|(_, t)| t.0 == row.agent_id) else {
            continue;
        };
        // 같은 주민의 이전 버블 제거
        for (be, of) in &q_bubbles {
            if of.0 == agent_e {
                commands.entity(be).despawn();
            }
        }
        // 상한 — 만료 임박(가장 오래된) 버블부터 퇴장. SpeechBubble 질의가
        // 이 시스템에 없으니 엔티티 인덱스(≈생성순) 휴리스틱으로 충분하다.
        if q_bubbles.iter().count() >= MAX_BUBBLES {
            if let Some((oldest, _)) = q_bubbles.iter().min_by_key(|(be, _)| be.index()) {
                commands.entity(oldest).despawn();
            }
        }
        // 폭 추정 — 한글 전각 ≈ 1em, 라틴/공백 ≈ 0.55em
        let em = row.text.chars().count() as f32;
        let wide = row
            .text
            .chars()
            .filter(|c| (*c as u32) > 0x2E80)
            .count() as f32;
        let narrow = em - wide;
        let w = wide * 24.0 + narrow * 13.0 + 28.0;
        commands
            .entity(agent_e)
            .with_child((
                SpeechBubble { ttl: TTL },
                BubbleOf(agent_e),
                Transform::from_xyz(0.0, 236.0, 5.0),
                Sprite {
                    color: Color::srgba(0.99, 0.97, 0.90, 0.94),
                    custom_size: Some(Vec2::new(w, 40.0)),
                    ..default()
                },
            ))
            .with_child((
                BubbleOf(agent_e),
                Transform::from_xyz(0.0, 236.0, 5.1),
                Text2d::new(row.text.clone()),
                TextFont {
                    font: FontSource::Handle(font.clone()),
                    font_size: FontSize::Px(24.0),
                    ..default()
                },
                TextColor(Color::srgb(0.16, 0.12, 0.08)),
                TextLayout::justify(bevy::text::Justify::Center),
            ));
    }
}

/// 말풍선 수명 — ttl 감소·마지막 1초 페이드(배경·텍스트)·만료 철거
pub fn fade(
    time: Res<Time>,
    mut commands: Commands,
    mut q: Query<(Entity, &mut SpeechBubble, &mut Sprite)>,
    mut texts: Query<(Entity, &BubbleOf, &mut TextColor), With<Text2d>>,
    bubbles: Query<&BubbleOf, With<SpeechBubble>>,
) {
    for (e, mut b, mut s) in &mut q {
        b.ttl -= time.delta_secs();
        if b.ttl <= 0.0 {
            // 배경 + 같은 주민의 형제 텍스트까지 철거(텍스트는 BubbleOf만 달고 있음)
            if let Ok(of) = bubbles.get(e) {
                for (te, to, _) in &texts {
                    if to.0 == of.0 {
                        commands.entity(te).despawn();
                    }
                }
            }
            commands.entity(e).despawn();
            continue;
        }
        let a = b.ttl.min(1.0);
        s.color.set_alpha(0.94 * a);
        // 형제 텍스트도 동일 페이드 — BubbleOf로 주인 매칭
        if let Ok(of) = bubbles.get(e) {
            for (_, to, mut tc) in &mut texts {
                if to.0 == of.0 {
                    tc.0.set_alpha(a);
                }
            }
        }
    }
}
