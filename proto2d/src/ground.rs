//! 지형 — mewki 블록 타일(정준 캔버스 256×312, tiles/BLOCK_SPEC.md) 전개.
//! 블록은 측벽이 아래로 내려오므로 **py y-sort**(z=py·0.001 — 소품 z_of보다
//! 항상 아래)로 겹침을 정렬한다. 같은 텍스처끼리 자동 배칭. 물은 4프레임 순환.
//! 라이브: mapJson(Tiled) 프라임이 spawn_live로 전개(net/apply.rs).
//! 미지 GID(어휘 확장 대비)는 잔디 타일+결정론 색조 폴백 — 패닉 없이 관전 유지.
//! 설원 스킨: 계절 winter·날씨 snow 때 육지 GID를 iso_snow 패밀리로 재할당.
use bevy::prelude::*;
use crate::mapdata::*;
use crate::net::TiledMap;

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

/// 지면 타일 태그 — 프라임 철거·설원 스킨 재할당 대상
#[derive(Component)]
pub struct GroundTile {
    pub gid: u8,
    pub variant: usize,
}

/// 지면 스킨 — 계절·날씨(겨울/폭설) 투영
#[derive(Resource, PartialEq, Eq, Clone, Copy, Default, Debug)]
pub enum GroundSkin {
    #[default]
    Normal,
    Snow,
}

/// GID별 변형 PNG 핸들 (맵 로드 시 1회 — AssetServer가 경로 캐시)
#[derive(Resource, Default)]
pub struct TileSet {
    variants: Vec<Vec<Handle<Image>>>,
    snow: Vec<Handle<Image>>,
}

impl TileSet {
    pub fn load(assets: &AssetServer) -> Self {
        let variants = GIDS
            .iter()
            .enumerate()
            .map(|(gi, g)| {
                (0..VARIANTS[gi])
                    .map(|v| assets.load(format!("tiles/iso_{g}_{v}.png")))
                    .collect()
            })
            .collect();
        let snow = (0..4)
            .map(|v| assets.load(format!("tiles/iso_snow_{v}.png")))
            .collect();
        TileSet { variants, snow }
    }

    /// (gid, variant, skin) → 이미지 핸들. 설원은 육지 GID 전부 눈 패밀리.
    fn image(&self, gid: u8, variant: usize, skin: GroundSkin) -> Handle<Image> {
        if skin == GroundSkin::Snow && matches!(gid, 2..=6) {
            return self.snow[variant % 4].clone();
        }
        let gi = gid.clamp(1, 6) as usize - 1;
        self.variants[gi][variant % VARIANTS[gi]].clone()
    }
}

/// 미지 GID 폴백 색조 — GID 해시로 결정론적 색상(매번 같은 색)
fn unknown_tint(raw: u8) -> Color {
    let mut h: u32 = raw as u32;
    h = h.wrapping_mul(2654435761);
    let hue = (h % 360) as f32;
    Color::hsl(hue, 0.45, 0.55)
}

pub fn spawn(mut commands: Commands, assets: Res<AssetServer>, skin: Option<Res<GroundSkin>>) {
    let (mw, mh) = dims();
    let gids: Vec<u8> = (0..mw * mh).map(|i| gid(i % mw, i / mw)).collect();
    spawn_tiles(
        &mut commands,
        &gids,
        mw as usize,
        skin.map(|s| *s).unwrap_or_default(),
        &TileSet::load(&assets),
    );
}

/// 라이브 지형 스폰(프라임 전용) — mapJson layers[0] 그대로 전개.
pub fn spawn_live(
    commands: &mut Commands,
    assets: &AssetServer,
    map: &TiledMap,
    skin: GroundSkin,
) {
    spawn_tiles(
        commands,
        &map.layers[0].data,
        map.width,
        skin,
        &TileSet::load(assets),
    );
}

/// 타일 전개 공용 — 데모(dims)·라이브(mapJson) 동일 경로
fn spawn_tiles(commands: &mut Commands, gids: &[u8], w: usize, skin: GroundSkin, ts: &TileSet) {
    let mut unknown: std::collections::BTreeMap<u8, usize> = std::collections::BTreeMap::new();
    for (i, &raw) in gids.iter().enumerate() {
        let (x, y) = ((i % w) as i32, (i / w) as i32);
        let gid = raw.clamp(1, 6);
        let variant = (x as usize * 7 + y as usize * 13) % VARIANTS[(gid - 1) as usize];
        // raw 0(빈 인코딩)은 물로 채움(지면 구멍 방지), 색조는 미지 GID(>6)만
        let tint = (raw > 6).then(|| unknown_tint(raw));
        if tint.is_some() {
            *unknown.entry(raw).or_insert(0) += 1;
        }
        let (px, py) = project(x as f32, y as f32);
        let drop = if gid == 1 { WATER_DROP } else { 0.0 };
        let mut e = commands.spawn((
            GroundTile { gid, variant },
            Transform::from_xyz(px, -py - SPR_Y_OFF - drop, py * 0.001),
            Sprite {
                image: ts.image(gid, variant, skin),
                custom_size: Some(Vec2::new(SPR_W, SPR_H)),
                color: tint.unwrap_or(Color::WHITE),
                ..default()
            },
        ));
        if gid == 1 && raw == 1 {
            // 물 애님 — 변형 4장을 프레임으로 순환
            let frames = [
                ts.variants[0][0].clone(),
                ts.variants[0][1].clone(),
                ts.variants[0][2].clone(),
                ts.variants[0][3].clone(),
            ];
            e.insert(WaterTile { frames });
        }
    }
    if !unknown.is_empty() {
        let hist: Vec<String> = unknown
            .iter()
            .map(|(g, n)| format!("gid{g}×{n}"))
            .collect();
        warn!("[ground] 미지 GID 폴백(잔디+색조) — {}", hist.join(" · "));
    }
}

/// 물 애님 — 0.45초/프레임 순환
pub fn water_frames(time: Res<Time>, mut q: Query<(&WaterTile, &mut Sprite)>) {
    let f = (time.elapsed_secs() / 0.45) as usize % 4;
    for (w, mut s) in &mut q {
        s.image = w.frames[f].clone();
    }
}

/// 스킨 적용 — 목표 스킨과 다를 때만 전 타일 이미지 재할당(1회 리배칭).
/// net/apply::sync_daynight가 계절·날씨를 GroundSkin에 반영하고
/// ?exp=w(TF_SKIN=snow)는 부팅 시 강제.
pub fn apply_skin(
    assets: Res<AssetServer>,
    skin: Res<GroundSkin>,
    mut applied: Local<Option<GroundSkin>>,
    mut q: Query<(&GroundTile, &mut Sprite)>,
) {
    if *applied == Some(*skin) {
        return;
    }
    *applied = Some(*skin);
    // 핸들 캐시 — 매 타일 assets.load 금지(경로 기준 중복 로드는 없지만 1회 확보)
    let ts = TileSet::load(&assets);
    let n = q.iter_mut().count();
    for (t, mut s) in &mut q {
        s.image = ts.image(t.gid, t.variant, *skin);
    }
    info!("[ground] 스킨 → {skin:?} ({n}타일 재할당)");
}
