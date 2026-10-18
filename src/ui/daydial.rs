//! DayDial — 해/달 위치계 위젯(FE parts.tsx DayDial·tf-i18n.js dayDialSvg 직역).
//! HUD 시계 옆 68×18 캔버스: 지평선·13점 호, 낮엔 해(광선 4), 밤엔 달(크레이터).
//! 서명(시·10분·일출/일몰) 변화 시에만 다시 그린다(minimap 베이크 패턴).

use bevy::asset::RenderAssetUsages;
use bevy::image::Image;
use bevy::picking::Pickable;
use bevy::prelude::*;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};

use super::theme::UiTheme;
use crate::world::daynight::DayTime;

const W: usize = 68;
const H: usize = 18;

#[derive(Component)]
pub struct DayDialImg;

/// 재그리기 서명 — (hour, minute/10, sunrise, sunset) 팩.
#[derive(Resource, Default)]
pub struct DayDialState {
    image: Option<Handle<Image>>,
    sig: u64,
}

/// HUD 시계 칩 뒤에 위젯 노드를 심는다(hud.rs가 위치를 제공).
pub fn mount(p: &mut ChildSpawnerCommands, _theme: &UiTheme) {
    p.spawn((
        DayDialImg,
        Pickable::IGNORE, // 표시 전용 — 월드 탭·UI 픽킹에서 제외
        ImageNode::default(),
        Node {
            width: Val::Px(W as f32),
            height: Val::Px(H as f32),
            ..default()
        },
    ));
}

pub fn update_daydial(
    day: Res<DayTime>,
    mut images: ResMut<Assets<Image>>,
    mut q_img: Query<&mut ImageNode, (With<DayDialImg>, Without<super::minimap::MinimapMap>)>,
    mut st: ResMut<DayDialState>,
) {
    if !day.is_changed() && st.image.is_some() {
        return;
    }
    let (rise_min, set_min) = if day.sunrise_min > 0.0 && day.sunset_min > day.sunrise_min {
        (day.sunrise_min, day.sunset_min)
    } else {
        (5.0 * 60.0, 20.0 * 60.0) // FE 폴백 — 미수신 시 05/20시
    };
    let hour = day.hour;
    let minute = (hour.fract() * 60.0) as u32;
    // 서명 — 10분 양자화(그 이하로는 위치 변화가 서브픽셀).
    let sig = ((hour as u64) << 24)
        | ((minute / 10) as u64) << 20
        | ((rise_min as u64) & 0xffff) << 4
        | ((set_min as u64) & 0xf);
    if sig == st.sig && st.image.is_some() {
        return;
    }
    st.sig = sig;

    let buf = draw(hour, rise_min, set_min);
    match &st.image {
        Some(h) => {
            if let Some(mut img) = images.get_mut(h) {
                img.data = Some(buf);
            }
        }
        None => {
            let image = Image::new(
                Extent3d {
                    width: W as u32,
                    height: H as u32,
                    depth_or_array_layers: 1,
                },
                TextureDimension::D2,
                buf,
                TextureFormat::Rgba8UnormSrgb,
                RenderAssetUsages::default(),
            );
            st.image = Some(images.add(image));
        }
    }
    if let Some(h) = &st.image
        && let Ok(mut node) = q_img.single_mut()
    {
        node.image = h.clone();
    }
}

/// FE 수학 직역 — 낮은 일출~일몰 정규화, 밤은 그 나머지. 해는 #ffd166,
/// 달은 #dbe4f0, 호·지평선은 슬레이트 반투명.
fn draw(hour: f32, rise_min: f32, set_min: f32) -> Vec<u8> {
    let mut buf = vec![0u8; W * H * 4];
    let rise = rise_min / 60.0;
    let set = set_min / 60.0;
    let day_len = (set - rise).max(1.0);
    let night_len = 24.0 - day_len;
    let h = hour.rem_euclid(24.0);
    let is_day = h >= rise && h < set;
    let t = if is_day {
        (h - rise) / day_len
    } else {
        (h - set).rem_euclid(24.0) / night_len
    }
    .clamp(0.0, 1.0);
    let pos = |t: f32| -> Vec2 {
        Vec2::new(
            3.0 + t * (W as f32 - 6.0),
            H as f32 - 3.0 - (t * std::f32::consts::PI).sin() * (H as f32 - 8.0),
        )
    };
    let arc = [0.354f32, 0.427, 0.494, 0.35]; // #5a6d7e 반투명
    let horizon = [0.354f32, 0.427, 0.494, 0.22];
    // 지평선
    for x in 0..W {
        px(&mut buf, x as i32, H as i32 - 2, horizon);
    }
    // 13점 호(FE 폴리라인)
    for i in 0..=12 {
        let p = pos(i as f32 / 12.0);
        px(&mut buf, p.x.round() as i32, p.y.round() as i32, arc);
    }
    let at = pos(t);
    if is_day {
        // 해 — 원 + 십자 광선(FE: r3.4, 광선 4.6→5.8)
        disc(&mut buf, at, 3.4, [1.0, 0.820, 0.400, 1.0]);
        for a in [0.0f32, 90.0, 180.0, 270.0] {
            let rad = a.to_radians();
            let d = Vec2::new(rad.cos(), rad.sin());
            line(&mut buf, at + d * 4.6, at + d * 5.8, [1.0, 0.820, 0.400, 1.0]);
        }
    } else {
        // 달 — 원 + 크레이터 2
        disc(&mut buf, at, 3.2, [0.859, 0.894, 0.941, 1.0]);
        disc(&mut buf, at + Vec2::new(1.0, -0.8), 0.8, [0.604, 0.655, 0.741, 1.0]);
        disc(&mut buf, at + Vec2::new(-0.9, 0.9), 0.6, [0.604, 0.655, 0.741, 1.0]);
    }
    buf
}

fn px(buf: &mut [u8], x: i32, y: i32, c: [f32; 4]) {
    if x < 0 || y < 0 || x >= W as i32 || y >= H as i32 {
        return;
    }
    let i = (y as usize * W + x as usize) * 4;
    let over = |dst: u8, src: f32, a: f32| {
        let d = dst as f32 / 255.0;
        ((d * (1.0 - a) + src * a) * 255.0) as u8
    };
    buf[i] = over(buf[i], c[0], c[3]);
    buf[i + 1] = over(buf[i + 1], c[1], c[3]);
    buf[i + 2] = over(buf[i + 2], c[2], c[3]);
    buf[i + 3] = buf[i + 3].max((c[3] * 255.0) as u8);
}

fn disc(buf: &mut [u8], c: Vec2, r: f32, color: [f32; 4]) {
    let r2 = r * r;
    for y in (c.y - r).floor() as i32..=(c.y + r).ceil() as i32 {
        for x in (c.x - r).floor() as i32..=(c.x + r).ceil() as i32 {
            let d = Vec2::new(x as f32 - c.x, y as f32 - c.y).length_squared();
            if d <= r2 {
                px(buf, x, y, color);
            }
        }
    }
}

fn line(buf: &mut [u8], from: Vec2, to: Vec2, color: [f32; 4]) {
    let steps = (from.distance(to) * 2.0).ceil() as i32;
    for i in 0..=steps.max(1) {
        let p = from.lerp(to, i as f32 / steps.max(1) as f32);
        px(buf, p.x.round() as i32, p.y.round() as i32, color);
    }
}
