//! 라이브 자원 동기 — 자원 엔티티 생성·갱신·제거와 동물 배위·건축 발판·
//! 성장 단계 적용(apply.rs에서 기능별 분리, 파일 500줄 규칙). 서버
//! resources 행을 소품 GLB·절차 발판으로 투영한다.

use bevy::prelude::*;

use super::visual_map::{ResState, bld_size, live_visual};
use super::{NetState, NetStatus};
use crate::world::data::{PropKind, WorldData};

#[derive(Component)]
pub struct LiveResourceTag(pub u32);

/// 구조물 kind — 타일을 선점해 나무·장식이 같은 타일에 뚫고 서지 못하게 한다.
pub(crate) fn structural_kind(kind: &str) -> bool {
    matches!(kind, "hut" | "campfire" | "stash" | "grave" | "relic")
}

/// 구조물 판정(kind+state) — 개굴 행(kind=hut·생태 state)은 제외. 청소 반경
/// 집합이 이 판정을 쓴다(visual_map::is_den 주석 — den은 숲 속 굴이라
/// 주변 나무·장식을 밀어내지 않는다).
pub(crate) fn is_structure(r: &super::LiveResource) -> bool {
    if !structural_kind(&r.kind) {
        return false;
    }
    if r.kind == "hut" {
        let st: super::visual_map::ResState = r
            .state
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        return !super::visual_map::is_den(&st);
    }
    true
}

/// 구조물 청소 반경(타일, 체비셰프) — 주변까지 포함해 나무·덤불 스폰을 양보한다.
/// 에셋이 늦게 로딩돼도(나무 GLB가 마지막에 뜸) 기본 뷰에서 집이 숲에 묻히지
/// 않게 하는 클라이언트 투영. 구조물 집합의 3×3 확장 좌표를 반환.
pub(crate) fn cleared_tiles(
    structural: &std::collections::HashSet<(u16, u16)>,
) -> std::collections::HashSet<(i32, i32)> {
    let mut out = std::collections::HashSet::new();
    for &(x, y) in structural {
        for dy in -1..=1i32 {
            for dx in -1..=1i32 {
                out.insert((x as i32 + dx, y as i32 + dy));
            }
        }
    }
    out
}

/// 자원 상태 서명 — 변화(사망·건축 단계 진행) 감지용.
/// hut은 정체성 필드(stage·type·w·h)만 — work/progress는 스냅숏마다 변해
/// 재스폰 깜빡임을 일으키므로 제외하고 진행률은 SiteMeta를 통해 갱신한다.
/// (동물 x/y는 행 필드라 애초에 서명에 없다 — 배회는 RoamTo로 부드럽게.)
pub(crate) fn res_sig(r: &super::LiveResource, vis: bool) -> u64 {
    let mut h = 1469598103934665603u64;
    let mut feed = String::new();
    if r.kind == "hut" {
        let st: ResState = r
            .state
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        feed.push_str(st.stage.as_deref().unwrap_or(""));
        feed.push('|');
        feed.push_str(st.bld_type.as_deref().unwrap_or(""));
        feed.push_str(&format!("|{}|{}", st.w.unwrap_or(0), st.h.unwrap_or(0)));
    } else {
        feed.push_str(r.state.as_deref().unwrap_or(""));
    }
    for b in r
        .kind
        .bytes()
        .chain(r.species.bytes())
        .chain(feed.bytes())
        .chain(std::iter::once(if vis { 1 } else { 0 }))
    {
        h ^= b as u64;
        h = h.wrapping_mul(1099511628211);
    }
    h
}

/// 자원 상태 서명 컴포넌트 — 변화 시 재스폰 트리거.
#[derive(Component)]
pub struct LiveResMeta(pub u64);

/// 동물 배위 목적지(§17.64) — 서버 x/y diff를 재스폰 없이 부드러운 이동으로.
#[derive(Component)]
pub struct RoamTo(pub Vec3);

/// 자원 동기화 — 신규 스폰 / 사라진 것·상태 변화(사망·건축 단계) 재스폰.
#[allow(clippy::too_many_arguments)] // Bevy 시스템 파라미터 — 메시·재질은 스폰 직접 주입용
#[allow(clippy::type_complexity)] // 동기 루프 쿼리 — 배위·발판 메타까지 읽는 6튜플
pub fn sync_resources(
    mut commands: Commands,
    mut net: ResMut<NetState>,
    world: Res<WorldData>,
    assets: Res<AssetServer>,
    ready: Option<Res<super::inventions::ReadyModels>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut std_mats: ResMut<Assets<bevy::pbr::StandardMaterial>>,
    mut q_live: Query<(
        Entity,
        &LiveResourceTag,
        Option<&LiveResMeta>,
        Option<&crate::world::models::PropRoot>,
        Option<&mut RoamTo>,
        Option<&mut crate::world::bld::SiteMeta>,
    )>,
    mut first_log: Local<u32>,
    // 구조물 청소 집합 캐시 — (서명, 3×3 확장 좌표). 매 프레임 재구축(구조물×9
    // 삽입+할당) 대신 자원 목록 서명이 바뀔 때만 다시 계산한다.
    mut cleared_cache: Local<(u64, std::collections::HashSet<(i32, i32)>)>,
    // 스폰 영구 스킵 id 캐시 — 아래 루프의 skip_cache.insert 참조(주석 상세).
    mut skip_cache: Local<std::collections::HashSet<u32>>,
    // 모닥불 동적 상태(§17.25) — 이벤트로 태어난/꺼진 불의 id 집합. 프레임
    // 행과 병합해 스냅숏 스폰 경로(발판·불꽃·flicker)를 그대로 재사용한다.
    mut camp_live: Local<std::collections::HashMap<u32, (u16, u16)>>,
    mut camp_dead: Local<std::collections::HashSet<u32>>,
    // 동물 좌표 갱신(§17.43) — 어류 유영 이벤트의 최신 목적지(id 키).
    mut move_targets: Local<std::collections::HashMap<u32, (f32, f32)>>,
    // 진단(§17.43) — 어류 스폰 수 카운트(hutprobe 로그에 병기).
    q_fish: Query<&crate::world::fish::FishRoot>,
) {
    if net.status != NetStatus::Connected {
        return;
    }
    // 모닥불 이벤트 소비 — 프레임 대여 전에(net 가변 대여 충돌 방지).
    for ev in std::mem::take(&mut net.pending_campfires) {
        if ev.remove.unwrap_or(false) {
            camp_dead.insert(ev.id);
            camp_live.remove(&ev.id);
        } else if let (Some(x), Some(y)) = (ev.x, ev.y) {
            camp_dead.remove(&ev.id);
            camp_live.insert(ev.id, (x, y));
        } else {
            camp_dead.remove(&ev.id); // refuel 등 — 이미 있으면 무해 통과
        }
    }
    // 동물 좌표 갱신(§17.43) — 같은 id의 연속 갱신은 최신만 남는다.
    for mv in std::mem::take(&mut net.pending_moves) {
        move_targets.insert(mv.id, (mv.x, mv.y));
    }
    let Some(frame) = net.frame.as_ref() else {
        return;
    };
    // 병합 행 — 서버 프레임(꺼진 불 제외) + 이벤트 태생 불(서버 행에 없을 때만).
    // 참조 모음이라 프레임당 문자열 복제가 없다.
    let camp_synth: Vec<super::LiveResource> = camp_live
        .iter()
        .filter(|(id, _)| {
            !camp_dead.contains(*id)
                && !frame
                    .resources
                    .iter()
                    .any(|r| r.id == **id && r.kind == "campfire")
        })
        .map(|(id, (x, y))| super::LiveResource {
            id: *id,
            kind: "campfire".to_owned(),
            species: String::new(),
            x: *x,
            y: *y,
            state: None,
        })
        .collect();
    let rows: Vec<&super::LiveResource> = frame
        .resources
        .iter()
        .filter(|r| !(r.kind == "campfire" && camp_dead.contains(&r.id)))
        .chain(camp_synth.iter())
        .collect();

    // §17.92 집주소 필터 — 살아 있는 주민의 homeId 집합. 죽은 주민의 유령
    // 완공 hut(terrafolk-web 실측 48채)을 렌더·나무 청소 반경 모두에서
    // 제외한다. 프레임마다 재판정(입주·사망으로 가시성이 역전되면 그 프레임에
    // 반영 — skip_cache의 영구 스킵과 다른 계약). owner 필드 기반 필터는
    // 상속 유령 93채 사고로 terrafolk-web에서 폐기 — 집주소가 정답.
    let home_ids: std::collections::HashSet<String> = frame
        .agents
        .iter()
        .filter_map(|a| a.home_id.clone())
        .collect();
    let ghost_hut = |r: &super::LiveResource, st: &ResState| {
        r.kind == "hut"
            && super::visual_map::hut_completed(st)
            && !home_ids.contains(&r.id.to_string())
    };

    // 갱신·제거 — 서명이 바뀌면 despawn(다음 프레임 새 시각으로 재스폰).
    // 동물 좌표 이동(§17.64 배회)은 서명과 무관 — RoamTo 목적지만 갱신하고,
    // 공사 진행률은 SiteMeta에 흘려 발판을 재스폰 없이 바만 움직인다.
    for (e, tag, meta, _, roam, site) in &mut q_live {
        let Some(r) = rows.iter().find(|r| r.id == tag.0).copied() else {
            commands.entity(e).despawn();
            continue;
        };
        let st: ResState = r
            .state
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        // §17.92 — 거주자가 사라(죽거나 이사) 완공 hut은 즉시 철거.
        if ghost_hut(r, &st) {
            commands.entity(e).despawn();
            continue;
        }
        // hut은 스폰 경로가 언제나 res_sig(r, true)로 서명한다(공사 발판·완공
        // GLB 모두 실체 있음). 갱신 루프가 live_visual(is_some=false for 공사
        // 중)으로 서명하면 매 프레임 불일치 → despawn/respawn 폭풍이 된다.
        let vis = if r.kind == "hut" {
            true
        } else {
            live_visual(&r.kind, &r.species, &st).is_some()
        };
        let sig = res_sig(r, vis);
        match meta {
            Some(m) if m.0 == sig => {
                if r.kind == "animal" && st.alive != Some(false) {
                    let ground = world.height_at(r.x as i32, r.y as i32);
                    // §17.43 — 어류 유영 등 이벤트 좌표가 행 x/y보다 우선.
                    let (tx, tz) = match move_targets.get(&r.id) {
                        Some((x, z)) => (*x, *z),
                        None => (r.x as f32 + 0.5, r.y as f32 + 0.5),
                    };
                    let to = Vec3::new(tx, ground, tz);
                    match roam {
                        Some(mut rt) => rt.0 = to,
                        None => {
                            commands.entity(e).insert(RoamTo(to));
                        }
                    }
                }
                if r.kind == "hut"
                    && let Some(mut m) = site
                {
                    m.progress = super::visual_map::hut_progress(&st);
                }
            }
            _ => commands.entity(e).despawn(),
        }
    }

    // 신규 스폰
    *first_log += 1;
    // 진단 게이트 ?tfdbg=hutprobe — §17.92 유령 스킵·공사 진행률 데이터 흐름
    // 실측(2.5s 주기. 판정: 유령>0이면 필터 동작, 샘플에 w>0·%>0이면
    // work 기반 진행률 계산 동작).
    if super::dbg_gate::on("hutprobe") && *first_log % 150 == 0 {
        let (mut rows, mut done, mut ghost) = (0usize, 0usize, 0usize);
        let (animal_rows, aq_rows) = frame
            .resources
            .iter()
            .fold((0usize, 0usize), |(a, q), r| {
                if r.kind != "animal" {
                    return (a, q);
                }
                let st: ResState = r
                    .state
                    .as_deref()
                    .and_then(|s| serde_json::from_str(s).ok())
                    .unwrap_or_default();
                (a + 1, q + (st.aquatic == Some(true)) as usize)
            });
        let mut samples: Vec<String> = Vec::new();
        for r in &frame.resources {
            if r.kind != "hut" {
                continue;
            }
            let st: ResState = r
                .state
                .as_deref()
                .and_then(|s| serde_json::from_str(s).ok())
                .unwrap_or_default();
            if super::visual_map::is_den(&st) {
                continue;
            }
            rows += 1;
            if ghost_hut(r, &st) {
                ghost += 1;
            } else if super::visual_map::hut_completed(&st) {
                done += 1;
            } else if samples.len() < 3 {
                samples.push(format!(
                    "{}:{} w{:.0} {:.0}%",
                    r.id,
                    st.stage.as_deref().unwrap_or("?"),
                    st.work.unwrap_or(0.0),
                    super::visual_map::hut_progress(&st) * 100.0
                ));
            }
        }
        info!(
            "[net] hutprobe hut 행 {rows} · 완공(거주) {done} · 유령 스킵 {ghost} · 공사 {} · 동물 {animal_rows}(수중 {aq_rows}) · 어류 {} · 샘플 {samples:?}",
            rows - done - ghost,
            q_fish.iter().count()
        );
    }
    if *first_log == 2 {
        let mut total = 0;
        let mut animals = 0;
        let mut buildings = 0;
        for (_, _, _, prop, _, _) in &q_live {
            total += 1;
            if let Some(p) = prop {
                if matches!(
                    p.kind,
                    PropKind::AnimalBoar
                        | PropKind::AnimalDeer
                        | PropKind::AnimalWaterDeer
                        | PropKind::AnimalSquirrel
                        | PropKind::AnimalPheasant
                ) {
                    animals += 1;
                }
                if matches!(
                    p.kind,
                    PropKind::Hut | PropKind::StorageHut | PropKind::Tent
                ) {
                    buildings += 1;
                }
            }
        }
        info!("[net] 라이브 자원 실체 {total}개 (동물 {animals}·건물 {buildings})");
    }
    // 구조물 청소 반경 집합(3×3) — 나무·덤불이 구조물을 뒤덮는 것 방지.
    // 서버가 같은 타일·인접 타일에 나무와 오두막을 함께 두는 데이터에서도 건물이
    // 이긴다. 나무 GLB가 가장 늦게 로딩되므로 기본 뷰의 집이 숲에 묻히지 않는다.
    // 서명(구조물 좌표 폴드 + §17.92 거주지 집합)이 바뀔 때만 확장 집합을
    // 재계산 — 프레임당 할당 제거. 거주지 변화(입주·사망)도 유령 여부를
    // 바꾸므로 서명에 포함한다.
    let homes_sig = home_ids.iter().fold(home_ids.len() as u64, |a, s| {
        s.bytes()
            .fold(a.rotate_left(5), |b, c| b ^ (c as u64).wrapping_mul(1099511628211))
    });
    let sig = frame
        .resources
        .iter()
        .filter(|r| is_structure(r))
        .fold(frame.resources.len() as u64 ^ homes_sig, |a, r| {
            a.rotate_left(5) ^ (r.x as u64).wrapping_mul(31) ^ (r.y as u64).wrapping_mul(97)
        });
    if cleared_cache.0 != sig {
        let structural: std::collections::HashSet<(u16, u16)> = frame
            .resources
            .iter()
            .filter(|r| {
                if !is_structure(r) {
                    return false;
                }
                // §17.92 — 유령 hut은 보이지도 않으니 나무도 밀어내지 않는다.
                if r.kind == "hut" {
                    let st_c: ResState = r
                        .state
                        .as_deref()
                        .and_then(|s| serde_json::from_str(s).ok())
                        .unwrap_or_default();
                    return !ghost_hut(r, &st_c);
                }
                true
            })
            .map(|r| (r.x, r.y))
            .collect();
        cleared_cache.1 = cleared_tiles(&structural);
        cleared_cache.0 = sig;
    }
    let cleared = &cleared_cache.1;
    // 기존 실체 id 사전 — 신규 판정을 O(N×M) 순회 대신 해시 조회로.
    let live_ids: std::collections::HashSet<u32> =
        q_live.iter().map(|(_, t, _, _, _, _)| t.0).collect();
    // 스폰 영구 스킵 캐시 — 씬닝된 나무·물 타일·개굴·비표시 종처 '스폰 없이
    // 스킵'되는 행은 매 프레임 재평가되며 스폰 예산(64/프레임)을 반복 소비해
    // 프론티어를 그 행들에 영구 가둔다(2026-10-26 '집이 안 보임' 근본 원인 —
    // 실측: 씬닝 나무 64그루가 idx 93에 예산을 가둬 뒤 5,400행이 끝내 스폰
    // 못 함. live_ids는 매 프레임 q_live로 재구축되므로 스킵 표시가 프레임을
    // 못 넘긴다). 세션 수명 캐시로 예산이 행마다 정확히 1회만 소비되게 한다.
    // 스킵 판정 근거(씬닝 해시·물 타일·사망·개굴)는 세션 내 정지 상태라 무해.
    //
    // 스폰 예산 — 수천 자원이 한 프레임에 몰리면 씬 인스턴스화가 메인 스레드를
    // 수십 초 묶는다(2026-10-17 프리즈). 64/프레임 분산 — 잔여분은 다음 프레임
    // 호출에서 자동 이어진다(태그 없음 = 신규 판정).
    let mut budget: usize = 64;
    for r in &rows {
        if live_ids.contains(&r.id) || skip_cache.contains(&r.id) {
            continue;
        }
        // ── 예산 밖 무료 판정(스킵) — 예산을 소비하면 프론티어가 이 행들에
        //    영구 갇힌다(2026-10-26 '집이 안 보임' · 2026-11-04 '동물 0' —
        //    동물 행은 idx 3700+ 뒤라 청소 반경 나무가 64예산을 다 태우면
        //    육지 동물·어류가 끝내 못 스폰했다). 세션 내 불변 근거는 영구
        //    캐시, 가시성 역전 가능(유령 hut·청소 반경)은 매 프레임 무료
        //    재판정.
        let st: ResState = r
            .state
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        // §17.92 유령 완공 hut — 입주로 살아날 수 있어 캐시 금지.
        if ghost_hut(r, &st) {
            continue;
        }
        // 구조물 청소 반경 양보(나무·덤불) — hut 소멸 시 되살아나야 해
        // 캐시 금지·매 프레임 무료 재판정. 예산을 태우면 프론티어가 청소
        // 반경에 갇혀 뒤쪽 동물 행이 끝내 스폰 못 한다(2026-11-04 실측).
        if let Some((k, _)) = live_visual(&r.kind, &r.species, &st)
            && matches!(
                k,
                PropKind::TreeBroadleaf
                    | PropKind::TreePine
                    | PropKind::TreePalm
                    | PropKind::TreeSapling
                    | PropKind::BerryBush
            )
            && cleared.contains(&(r.x as i32, r.y as i32))
        {
            continue;
        }
        if budget == 0 {
            break;
        }
        budget -= 1;
        // 동물 개굴(den) — kind='hut'이지만 건축 아님. 유령 오두막·공사 발판
        // 모두 스킵(visual_map::is_den 주석).
        if r.kind == "hut" && super::visual_map::is_den(&st) {
            skip_cache.insert(r.id);
            continue;
        }
        // 수중 동물(§17.43) — 어류·가재·조개는 절차 물고기로 수면 유영
        // (world/fish.rs). 잡힌 개체(alive=false)는 스킵. 이 분기가 물 타일
        // 스킵보다 앞서야 어류가 살아난다.
        if r.kind == "animal" && st.aquatic == Some(true) {
            if st.alive == Some(false) || super::dbg_gate::off("no_animal") {
                skip_cache.insert(r.id);
            } else {
                crate::world::fish::spawn_live_fish(&mut commands, &mut meshes, &mut std_mats, r);
            }
            continue;
        }
        // 물 타일 자원(어류 서식 등) — 지형은 세션 내 불변이라 영구 스킵.
        if world.terrain_at(r.x as i32, r.y as i32) == "water" {
            skip_cache.insert(r.id);
            continue;
        }
        // 나무 씬닝(2026-10-22 사용자 지시 — 라이브 5,576그루 과밀 · 2026-10-26
        // 사용자 지시 듬성듬성 완화 35%→15%): 결정론 위치 해시로 15%만 렌더.
        // 데이터는 온전(호버·수확은 서버 정본).
        if r.kind == "tree"
            && crate::world::models::pos_hash(r.x as usize, r.y as usize, 7) % 100 >= 15
        {
            skip_cache.insert(r.id);
            continue;
        }
        // ── hut — 다중 타일 발판·시공 스테이지·완공 GLB·창불·내 집 마커.
        //    스폰 본체는 world::bld::spawn_live_hut(§17.75·§17.78·§5 갭2) —
        //    500줄 규칙으로 2026-11-03 이관.
        if r.kind == "hut" {
            crate::world::bld_spawn::spawn_live_hut(
                &mut commands, &mut meshes, &mut std_mats, &assets,
                r, &st, &world, ready.as_deref(),
            );
            continue;
        }
        let Some((kind, target_h)) = live_visual(&r.kind, &r.species, &st) else {
            skip_cache.insert(r.id); // 비표시 종(미지 kind·사망 동물) — 1회 소비 표시
            continue;
        };
        let vis = true;
        let sig = res_sig(r, vis);
        let ground = world.height_at(r.x as i32, r.y as i32);
        let (variants, card) = {
            let spec = kind.model();
            (spec.variants, spec.card)
        };
        let e = commands
            .spawn((
                crate::world::models::PropRoot {
                    kind,
                    target_h,
                    ground_y: ground,
                    // 카드 프롭은 요 0 고정(정면 +Z = 카메라) — models.rs 계약
                    rot_y: if card {
                        crate::world::models::CARD_YAW
                    } else {
                        (r.id % 360) as f32 / 360.0 * std::f32::consts::TAU
                    },
                    scale_jitter: 1.0,
                    variants,
                },
                bevy::world_serialization::WorldAssetRoot(crate::world::models::prop_scene(
                    kind, &assets,
                )),
                LiveResourceTag(r.id),
                LiveResMeta(sig),
                Transform::from_translation(Vec3::new(r.x as f32 + 0.5, ground, r.y as f32 + 0.5)),
            ))
            .id();
        // 유물 — 지면 위 부유+자전(주민 시선 유도, FE 아이콘 sin 트윈)
        if kind == PropKind::Relic {
            commands.entity(e).insert(crate::world::propsfx::FloatBob {
                base_y: ground + 0.22,
                phase: (r.id % 628) as f32 / 100.0,
            });
        }
        // 광맥 — 귀한 자원 점멸 스파크(FE spark, 위상은 id로 결정론 분산)
        if kind == PropKind::GoldVein {
            let spark = crate::world::propsfx::spawn_vein_spark(
                &mut commands,
                &mut meshes,
                &mut std_mats,
                r.x as f32 + 0.5,
                ground,
                r.y as f32 + 0.5,
                (r.id % 628) as f32 / 100.0,
            );
            commands
                .entity(spark)
                .insert((LiveResourceTag(r.id), LiveResMeta(sig)));
        }
        // 모닥불 — 발광 불꽃·라이트는 PropRoot와 별도 엔티티(데모 경로와 공통 헬퍼).
        // 동일 동기화 태그+서명 부여로 제거·상태변화 시 모델과 함께 철거·재생성.
        if kind == crate::world::data::PropKind::Campfire {
            let (flame, glow) = crate::world::models::spawn_campfire_fire(
                &mut commands,
                &mut meshes,
                &mut std_mats,
                r.x as f32 + 0.5,
                r.y as f32 + 0.5,
                ground,
            );
            commands
                .entity(flame)
                .insert((LiveResourceTag(r.id), LiveResMeta(sig)));
            commands
                .entity(glow)
                .insert((LiveResourceTag(r.id), LiveResMeta(sig)));
        }
    }
}

/// 동물 배위 이동(§17.64) — RoamTo 목적지로 등속 보간 + 이동 방향 바라보기.
/// FE의 좌우 반환 이동에 해당하는 3D 환 — 재스폰 깜빡임 없이 자연스럽게 걷는다.
pub fn live_res_move(time: Res<Time>, mut q: Query<(&RoamTo, &mut Transform)>) {
    const SPEED: f32 = 0.6; // 타일/초 — 배위 동물은 주민보다 느리게
    let dt = time.delta_secs();
    for (to, mut tr) in &mut q {
        let dx = to.0.x - tr.translation.x;
        let dz = to.0.z - tr.translation.z;
        let dist = (dx * dx + dz * dz).sqrt();
        if dist < 1e-4 {
            continue;
        }
        let step = SPEED * dt;
        if dist <= step {
            tr.translation.x = to.0.x;
            tr.translation.z = to.0.z;
        } else {
            tr.translation.x += dx / dist * step;
            tr.translation.z += dz / dist * step;
            tr.rotation = Quat::from_rotation_y(dx.atan2(dz));
        }
    }
}
