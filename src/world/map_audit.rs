//! 맵↔객체 관계 감사 — `?tfdbg=mapaudit` (평시 비용 0, 운영 무해).
//! Update 단계(PreUpdate restore 이후)라 Transform은 평면 좌표. 첫 감사 후 600프레임마다
//! 한 줄 JSON `[mapaudit] {...}`. 항목 A0 맵 정체성(FNV-1a 해시)·A1 물 위 프롭·A2 부유/매몰·
//! A3 맵 밖·A4 핏 클램프 경계 스케일·A5 건물 반경 1.6m 안 장식 프롭.
//! 대상은 PropRoot 루트(물고기·새·달/별·주민은 PropRoot가 아니라 자동 제외).

use bevy::prelude::*;

use super::data::{PropKind, WorldData};
use super::ground::WATER_STAND_Y;
use super::models::PropRoot;

fn fnv1a(bytes: &[u8]) -> u32 {
    let mut h: u32 = 0x811c9dc5;
    for b in bytes {
        h ^= *b as u32;
        h = h.wrapping_mul(16777619);
    }
    h
}

fn is_building(k: PropKind) -> bool {
    matches!(k, PropKind::Hut | PropKind::StorageHut | PropKind::Tent)
}

pub fn map_audit(mut frame: Local<u32>, world: Res<WorldData>, q: Query<(&PropRoot, &Transform)>) {
    if !crate::net::dbg_gate::on("mapaudit") {
        return;
    }
    *frame += 1;
    if *frame != 300 && !(*frame).is_multiple_of(600) {
        return;
    }
    let size = world.size as f32;
    let water_n = world.terrain.iter().filter(|g| **g == 1).count();
    let (mut n, mut on_water, mut float, mut sink, mut oob, mut fit_edge) = (0, 0, 0, 0, 0, 0);
    let mut samples: Vec<String> = Vec::new();
    let mut edge_kinds: Vec<String> = Vec::new();
    let mut builds: Vec<Vec3> = Vec::new();
    let mut decor: Vec<Vec3> = Vec::new();
    for (p, t) in &q {
        n += 1;
        let (x, z, y) = (t.translation.x, t.translation.z, t.translation.y);
        let kind = format!("{:?}", p.kind);
        if x < 0.0 || z < 0.0 || x > size || z > size {
            oob += 1;
            if samples.len() < 5 {
                samples.push(format!("A3 {kind}@{x:.1},{z:.1}"));
            }
            continue;
        }
        if world.terrain_at(x.floor() as i32, z.floor() as i32) == "water" {
            on_water += 1;
            if samples.len() < 5 {
                samples.push(format!("A1 {kind}@{x:.1},{z:.1}"));
            }
        }
        let ground = world.surface_height(x, z).max(WATER_STAND_Y);
        if y - ground > 0.6 {
            float += 1;
            if samples.len() < 5 {
                samples.push(format!("A2+ {kind}@{x:.1},{z:.1} dy={:.2}", y - ground));
            }
        } else if y - ground < -0.6 {
            sink += 1;
            if samples.len() < 5 {
                samples.push(format!("A2- {kind}@{x:.1},{z:.1} dy={:.2}", y - ground));
            }
        }
        let s = t.scale.max_element();
        if !(0.021..=5.99).contains(&s) {
            fit_edge += 1;
            if edge_kinds.len() < 5 {
                edge_kinds.push(kind.clone());
            }
        }
        if is_building(p.kind) {
            builds.push(t.translation);
        } else if !matches!(p.kind, PropKind::GrassClump | PropKind::FlowerClump) {
            decor.push(t.translation);
        }
    }
    let overlap = builds
        .iter()
        .map(|b| {
            decor
                .iter()
                .filter(|d| d.xz().distance(b.xz()) < 1.6)
                .count()
        })
        .sum::<usize>();
    info!(
        "[mapaudit] {{\"A0\":{{\"size\":{},\"hash\":\"{:08x}\",\"water\":{:.3}}},\"props\":{n},\"A1\":{on_water},\"A2float\":{float},\"A2sink\":{sink},\"A3\":{oob},\"A4\":{fit_edge},\"A4kinds\":{:?},\"A5\":{overlap},\"samples\":{:?}}}",
        world.size,
        fnv1a(&world.terrain),
        water_n as f32 / world.terrain.len().max(1) as f32,
        edge_kinds,
        samples
    );
}
