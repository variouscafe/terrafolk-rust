//! 경제 관측(§17.84) — REST /economy 스냅숏 자료형·파서.
//! net/mod.rs 500줄 규칙 분리(2026-10-20). price_g는 0.1g 실수 —
//! '명령 가격'이 아닌 지난 성사 통계(economy.rs 라벨 참조).

use bevy::prelude::*;
use std::sync::{Arc, Mutex};

#[derive(Debug, Clone, serde::Deserialize, Default)]
pub struct EcoPrice {
    pub item: String,
    #[serde(default)]
    pub price_g: f64,
    #[serde(default)]
    #[allow(dead_code)] // 계약 정본 필드 — 표시에 골라 쓴다
    pub updated_day: Option<u32>,
    #[serde(default)]
    pub trades_seen: Option<u32>,
}

#[derive(Debug, Clone, serde::Deserialize, Default)]
pub struct EcoKpi {
    pub day: Option<u32>,
    #[serde(default, rename = "goldShare")]
    pub gold_share: Option<f64>,
    #[serde(default)]
    pub volume: Option<u32>,
    #[serde(default, rename = "goldDeals")]
    pub gold_deals: Option<u32>,
}

#[derive(Debug, Clone, serde::Deserialize, Default)]
pub struct EcoDeal {
    #[serde(default)]
    #[allow(dead_code)] // 계약 정본 필드 — 표시에 골라 쓴다
    pub tick: Option<u32>,
    #[serde(rename = "a_id", default)]
    pub a_id: String,
    #[serde(rename = "b_id", default)]
    pub b_id: String,
    #[serde(default)]
    pub give_item: String,
    #[serde(default)]
    pub give_qty: Option<u32>,
    #[serde(default)]
    pub get_item: String,
    #[serde(default)]
    pub get_qty: Option<u32>,
}

/// 경제 스냅숏 — 폴러(네이티브 스레드/웹 JS)가 통째로 교체
#[derive(Debug, Clone, Default)]
pub struct EcoData {
    pub prices: Vec<EcoPrice>,
    pub kpi: Option<EcoKpi>,
    pub deals: Vec<EcoDeal>,
}

#[derive(Resource, Clone, Default)]
pub struct EcoQueue(pub Arc<Mutex<Option<EcoData>>>);

/// /economy 응답 → EcoData(kpi는 null 가능).
/// prices·recent_deals는 키가 비어도(거래 0건 어린 월드) 빈 배열로
/// 완화 — 한쪽만 와도 반쪽 데이터를 보여준다(2026-10-20 리뷰).
pub fn eco_from_json(v: serde_json::Value) -> Option<EcoData> {
    let arr = |k: &str| {
        v.get(k)
            .cloned()
            .unwrap_or(serde_json::Value::Array(vec![]))
    };
    let prices: Vec<EcoPrice> = serde_json::from_value(arr("prices")).ok()?;
    let deals: Vec<EcoDeal> = serde_json::from_value(arr("recent_deals")).ok()?;
    let kpi = v
        .get("kpi")
        .filter(|k| !k.is_null())
        .and_then(|k| serde_json::from_value::<EcoKpi>(k.clone()).ok());
    Some(EcoData { prices, kpi, deals })
}
