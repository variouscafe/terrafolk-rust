//! 데모 월드 맵 데이터 — shared-map.json(레거시 FE·3D 클라이언트와 동일 맵)을
//! 빌드 시 포함. GID 1..6 = water/sand/grass/forest/rock/dirt(client-meta 어휘).
//! 아이소 투영: screen = ((x−y)·TW/2, (x+y)·TH/2) — 스프라이트에 투영 내장.
use std::sync::OnceLock;

pub const MAP_W: i32 = 128;
pub const MAP_H: i32 = 128;
pub const TW: f32 = 128.0; // 타일 폭(다이아몬드)
pub const TH: f32 = 100.0; // 타일 높이 — mewki 블록 비율 0.78125(tiles/BLOCK_SPEC.md)
pub const GIDS: [&str; 6] = ["water", "sand", "grass", "forest", "rock", "dirt"];

static MAP: OnceLock<Vec<u8>> = OnceLock::new();

pub fn map() -> &'static [u8] {
    MAP.get_or_init(|| {
        include_str!("../assets/map.txt")
            .split(',')
            .map(|s| s.trim().parse::<u8>().unwrap_or(4))
            .collect()
    })
}

pub fn gid(x: i32, y: i32) -> u8 {
    // 라이브 맵 우선 — 프라임 시 1회 세팅(이후 불변, fullStateOnly 계약)
    if let Some(m) = LIVE.get() {
        if (0..m.w).contains(&x) && (0..m.h).contains(&y) {
            return m.gids[(y * m.w + x) as usize];
        }
        return 0;
    }
    if (0..MAP_W).contains(&x) && (0..MAP_H).contains(&y) {
        map()[(y * MAP_W + x) as usize]
    } else {
        0
    }
}

/// 라이브 맵 — mapJson(Tiled) 프라임 시 1회 적재. 이후 불변이라 OnceLock.
pub struct LiveMap {
    pub w: i32,
    pub h: i32,
    pub gids: Vec<u8>,
}

static LIVE: OnceLock<LiveMap> = OnceLock::new();

/// 라이브 맵 설치(1회) — 성공 여부 반환(이미 설치됐으면 false)
pub fn set_live_map(w: usize, h: usize, gids: Vec<u8>) -> bool {
    LIVE.set(LiveMap {
        w: w as i32,
        h: h as i32,
        gids,
    })
    .is_ok()
}

/// 현재 맵 크기 — 라이브 우선, 데모 폴백
pub fn dims() -> (i32, i32) {
    LIVE.get().map(|m| (m.w, m.h)).unwrap_or((MAP_W, MAP_H))
}

/// 물(1)·범위 밖(0) 제외 — 주민·마을 배치 기준
pub fn walkable(x: i32, y: i32) -> bool {
    matches!(gid(x, y), 2..=6)
}

/// 그리드(연속) → 월드 좌표. 화면 y는 아래로 갈수록 +py (2D y축 반전은 스폰 시)
pub fn project(gx: f32, gy: f32) -> (f32, f32) {
    ((gx - gy) * TW * 0.5, (gx + gy) * TH * 0.5)
}

pub fn tile_center(x: i32, y: i32) -> (f32, f32) {
    project(x as f32, y as f32)
}

/// 월드 경계 반경: (|x| 최대, y 최대) + 여유
pub fn world_extent() -> (f32, f32) {
    (MAP_W as f32 * TW * 0.5 + 320.0, (MAP_W + MAP_H) as f32 * TH * 0.5 + 320.0)
}

/// 마을 중심 — 맵 중앙에서 가장 가까운, 5×5가 전부 풀(3)인 지점
pub fn village_center() -> (i32, i32) {
    let all_grass = |cx: i32, cy: i32| {
        (-2..=2).all(|dy| (-2..=2).all(|dx| gid(cx + dx, cy + dy) == 3))
    };
    let (mx, my) = (MAP_W / 2, MAP_H / 2);
    for r in 0..48i32 {
        for dy in -r..=r {
            for dx in -r..=r {
                if r != dx.abs().max(dy.abs()) {
                    continue; // 링 순회
                }
                if all_grass(mx + dx, my + dy) {
                    return (mx + dx, my + dy);
                }
            }
        }
    }
    (mx, my)
}

/// 경량 결정론 RNG(xorshift32) — 산포·배회 재현성용, 외부 크레이트 없음
pub struct XorShift(u32);

impl XorShift {
    pub fn new(seed: u32) -> Self {
        Self(seed | 1)
    }

    pub fn next_u32(&mut self) -> u32 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.0 = x;
        x
    }

    pub fn next_f32(&mut self) -> f32 {
        self.next_u32() as f32 / u32::MAX as f32
    }

    pub fn range_i32(&mut self, lo: i32, hi: i32) -> i32 {
        lo + (self.next_u32() as i64 % (hi - lo + 1) as i64) as i32
    }
}
