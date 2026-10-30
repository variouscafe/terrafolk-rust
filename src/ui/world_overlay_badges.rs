//! 월드 오버레이 배지 담당 — 활동 진행 링·zZz·❗ 애니메이션(world_overlay에서
//! 분리, 500줄 규칙). 링은 양자화 굵기 변화 시만 캔버스 재그리기(minimap 패턴).

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::world_overlay::{AnchorCol, BubbleNode, Slot, SlotMark, NAME_CREAM, NAME_GOLD};
use crate::net::agent_hud::AgentHud;
use crate::world::pick::Selection;
use crate::world::agent::Villager;

/// 링 캔버스 재그리기 상태(양자화 굵기 — 1/10 단위).
#[derive(Component)]
pub struct BadgeCanvas {
    pub(crate) handle: Option<Handle<Image>>,
    pub(crate) last_width: u32,
}
/// 배지·zZz·❗ 표시 갱신 — AgentHud(라이브) 기반. 조건은 FE updateActivity 직역:
/// 배지 actType·아이콘·prog>0 / zZz sleeping / ❗ 욕구 최댓값 ≥90.
pub fn update_badges(
    sel: Res<Selection>,
    mut images: ResMut<Assets<Image>>,
    mut q_cols: Query<&mut AnchorCol>,
    huds: Query<(&AgentHud, &Villager)>,
    mut q_slots: Query<(&SlotMark, &mut Visibility), Without<BubbleNode>>,
    mut q_texts: Query<(&SlotMark, &mut Text, &mut TextColor), Without<BubbleNode>>,
    mut q_canvas: Query<(&mut BadgeCanvas, &mut ImageNode), Without<AnchorCol>>,
) {
    for col in &mut q_cols {
        let Ok((hud, v)) = huds.get(col.target) else {
            continue;
        };
        // 이름 — 선택 강조(금색)·이름 변화(스폰 이후 패치)
        let selected = sel.id.as_deref() == Some(col.id.as_str());
        for (mark, mut text, mut color) in &mut q_texts {
            match mark.0 {
                Slot::NameText => {
                    if **text != v.name {
                        **text = v.name.clone();
                    }
                    color.0 = if selected { NAME_GOLD } else { NAME_CREAM };
                }
                Slot::IconText => {
                    if let Some(icon) = super::i18n::act_icon(&hud.act_type)
                        && **text != icon
                    {
                        **text = icon.into();
                    }
                }
                _ => {}
            }
        }
        let badge_on = !hud.act_type.is_empty()
            && super::i18n::act_icon(&hud.act_type).is_some()
            && hud.act_progress > 0.0;
        let sleeping = v.status == "sleeping";
        let critical = hud.needs.iter().cloned().fold(0.0f32, f32::max) >= 90.0;
        for (mark, mut vis) in &mut q_slots {
            *vis = match mark.0 {
                Slot::Badge | Slot::Canvas => {
                    if badge_on {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    }
                }
                Slot::Zzz => {
                    if sleeping {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    }
                }
                Slot::Alert => {
                    if critical {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    }
                }
                Slot::Row => {
                    if badge_on || sleeping || critical {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    }
                }
                _ => continue,
            };
        }
        // 링 캔버스 — 양자화 굵이 변화 시만(minimap 베이크 패턴)
        if badge_on
            && let Ok((mut canvas, mut img_node)) = q_canvas.get_mut(col.canvas)
        {
            let width10 = (hud.act_progress.clamp(0.02, 1.0) * 10.0).round() as u32;
            if canvas.last_width != width10 {
                canvas.last_width = width10;
                match &canvas.handle {
                    Some(h) => {
                        if let Some(mut img) = images.get_mut(h) {
                            img.data = Some(ring_buf(hud.act_progress));
                            // CPU 재편집 = 1레벨(밉 체인 카운트 정합 — texture_mips 참조)
                            img.texture_descriptor.mip_level_count = 1;
                        }
                    }
                    None => {
                        canvas.handle = Some(images.add(ring_image(hud.act_progress)));
                    }
                }
            }
            if let Some(h) = &canvas.handle {
                img_node.image = h.clone();
            }
        }
    }
}

/// zZz 부유·❗ 바운스 — 프레임 애니메이션(쿼리 단일화로 B0001 회피).
pub fn animate_slots(time: Res<Time>, mut q: Query<(&SlotMark, &mut Node), Without<BadgeCanvas>>) {
    let t = time.elapsed_secs();
    for (mark, mut node) in &mut q {
        match mark.0 {
            // 상승 부유(sin ±2px — FE 1.6s 루프 근사)
            Slot::Zzz => node.top = Val::Px((t * 2.0).sin() * 2.0),
            // 바운스 ±4px(0.32s yoyo 근사)
            Slot::Alert => node.top = Val::Px((t * 19.6).sin() * 4.0),
            _ => {}
        }
    }
}

/// 진행 링 픽셀 버퍼 — r12, 굵기 2+frac*3, #8fe08a(tf-world-ui ringSvg 직역).
pub(crate) fn ring_buf(frac: f32) -> Vec<u8> {
    const S: u32 = 30;
    let mut buf = vec![0u8; (S * S * 4) as usize];
    let cx = 14.5f32;
    let cy = 14.5f32;
    let stroke = 2.0 + frac.clamp(0.02, 1.0) * 3.0;
    for y in 0..S {
        for x in 0..S {
            let d = ((x as f32 - cx).powi(2) + (y as f32 - cy).powi(2)).sqrt();
            let i = ((y * S + x) * 4) as usize;
            if (d - 12.0).abs() <= stroke * 0.5 {
                buf[i] = 143; // #8fe08a
                buf[i + 1] = 224;
                buf[i + 2] = 138;
                buf[i + 3] = 242; // opacity .95
            } else if d <= 12.0 {
                buf[i + 3] = 115; // rgba(0,0,0,.45)
            }
        }
    }
    buf
}

fn ring_image(frac: f32) -> Image {
    Image::new(
        Extent3d {
            width: 30,
            height: 30,
            depth_or_array_layers: 1,
        },
        TextureDimension::D2,
        ring_buf(frac),
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::default(),
    )
}
