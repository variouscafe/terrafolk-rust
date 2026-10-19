//! 지형 — mewki 블록 타일(정준 캔버스 256×312, tiles/BLOCK_SPEC.md) 전개.
//! 블록은 측벽이 아래로 내려오므로 **py y-sort**(z=py·0.001 — 소품 z_of보다
//! 항상 아래)로 겹침을 정렬한다. 같은 텍스처끼리 자동 배칭. 물은 4프레임 순환.
use bevy::prelude::*;
use crate::mapdata::*;

/// 블록 스프라이트 월드 크기 — 캔버스 256×312 ÷2(고밀도)
const SPR_W: f32 = 128.0;
const SPR_H: f32 = 156.0;
/// 캔버스 중심(156px)이 다이아몬드 중심(116px)보다 40px(=20 월드) 아래 →
/// 다이아몬드 중심을 타일 중심에 맞추려면 스프라이트를 20만큼 내린다.
const SPR_Y_OFF: f32 = 20.0;
/// 물 평면 낙차(월드) — 육지 블럭 측벽 상단이 드러나 해안이 '의도된 절벽'으로
/// 보이게 한다(레퍼런스의 섬 슬래브). 내부 육지는 하나의 이어진 평면.
const WATER_DROP: f32 = 12.0;
/// GID별 변형 수 — forest는 3(큰 나무 장식 forest_3은 평면 계약 밖)
const VARIANTS: [usize; 6] = [4, 3, 4, 4, 4, 4];

#[derive(Component)]
pub struct WaterTile {
    pub frames: [Handle<Image>; 4],
}

pub fn spawn(mut commands: Commands, assets: Res<AssetServer>, exp: Res<crate::Exp>) {
    // GID 순서로 변형 4종 핸들 (water는 애님 프레임 0..3)
    let variants: Vec<[Handle<Image>; 4]> = GIDS
        .iter()
        .map(|g| [
            assets.load(format!("tiles/iso_{g}_0.png")),
            assets.load(format!("tiles/iso_{g}_1.png")),
            assets.load(format!("tiles/iso_{g}_2.png")),
            assets.load(format!("tiles/iso_{g}_3.png")),
        ])
        .collect();
    // 실험 b/d — 마을 중심 49×49만 전개(스프라이트 수 16k→2.4k)
    let (vcx, vcy) = village_center();
    let tile_r = if exp.0 == 'b' || exp.0 == 'd' { 24 } else { i32::MAX };
    for y in 0..MAP_H {
        for x in 0..MAP_W {
            if (x - vcx).abs() > tile_r || (y - vcy).abs() > tile_r {
                continue;
            }
            let g = (gid(x, y).clamp(1, 6) - 1) as usize;
            let (px, py) = tile_center(x, y);
            let v = (x as usize * 7 + y as usize * 13) % VARIANTS[g];
            // 물은 낮은 평면 — 해안에서 육지 벽이 절벽으로 드러난다
            let drop = if g == 0 { WATER_DROP } else { 0.0 };
            let mut e = commands.spawn((
                Transform::from_xyz(px, -py - SPR_Y_OFF - drop, py * 0.001),
                Sprite {
                    image: variants[g][v].clone(),
                    custom_size: Some(Vec2::new(SPR_W, SPR_H)),
                    ..default()
                },
            ));
            if g == 0 {
                e.insert(WaterTile {
                    frames: variants[0].clone(),
                });
            }
        }
    }
}

/// 물 애님 — 0.6초/프레임 순환
pub fn water_frames(time: Res<Time>, mut q: Query<(&WaterTile, &mut Sprite)>) {
    let f = (time.elapsed_secs() / 0.45) as usize % 4;
    for (w, mut s) in &mut q {
        s.image = w.frames[f].clone();
    }
}
