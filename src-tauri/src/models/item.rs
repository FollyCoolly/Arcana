use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Config file ──

#[derive(Deserialize)]
pub struct ItemSourceFile {
    pub version: u32,
    pub sources: Vec<ItemSource>,
}

#[derive(Deserialize)]
pub struct ItemSource {
    pub id: String,
    pub name: String,
    pub path: String,
}

// ── Response structures ──

#[derive(Serialize)]
pub struct ItemSourceInfo {
    pub id: String,
    pub name: String,
    pub item_count: usize,
}

#[derive(Serialize)]
pub struct ItemWithComputed {
    pub id: String,
    pub source_id: String,
    pub name: String,
    pub brand: Option<String>,
    pub price: Option<f64>,
    pub purchase_date: Option<String>,
    pub purchase_channel: Option<String>,
    pub category: Option<String>,
    pub color: Option<String>,
    pub image: Option<String>,
    pub extra: HashMap<String, serde_json::Value>,
    pub days_owned: Option<u64>,
    pub daily_cost: Option<f64>,
}

#[derive(Serialize)]
pub struct SourceStats {
    pub source_id: String,
    pub source_name: String,
    pub item_count: usize,
    pub total_value: f64,
}

#[derive(Serialize)]
pub struct CategoryStats {
    pub name: String,
    pub item_count: usize,
    pub total_value: f64,
}

#[derive(Serialize)]
pub struct ItemStats {
    pub total_items: usize,
    pub total_value: f64,
    pub average_daily_cost: f64,
    pub by_source: Vec<SourceStats>,
    pub by_category: Vec<CategoryStats>,
}

#[derive(Serialize)]
pub struct ItemData {
    pub sources: Vec<ItemSourceInfo>,
    pub items: Vec<ItemWithComputed>,
    pub stats: ItemStats,
}
