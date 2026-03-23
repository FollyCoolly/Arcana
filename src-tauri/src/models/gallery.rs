use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Config file ──

#[derive(Deserialize)]
pub struct GallerySourceFile {
    pub version: u32,
    pub sources: Vec<GallerySource>,
}

#[derive(Deserialize)]
pub struct GallerySource {
    pub id: String,
    pub name: String,
    pub path: String,
    pub icon: Option<String>,
    pub media_type: String,
}

// ── Per-source JSON ──

#[derive(Deserialize)]
pub struct GalleryItemFile {
    pub version: u32,
    pub items: Vec<GalleryRawItem>,
}

#[derive(Deserialize)]
pub struct GalleryRawItem {
    pub name: String,
    pub name_original: Option<String>,
    pub cover: Option<String>,
    pub rating: Option<f64>,
    pub my_rating: Option<f64>,
    pub date_started: Option<String>,
    pub date_finished: Option<String>,
    pub tags: Option<Vec<String>>,
    pub episodes: Option<u32>,
    #[serde(default)]
    pub extra: HashMap<String, serde_json::Value>,
}

// ── Response structures ──

#[derive(Serialize)]
pub struct GallerySourceInfo {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub media_type: String,
    pub item_count: usize,
}

#[derive(Serialize)]
pub struct MediaItem {
    pub id: String,
    pub source_id: String,
    pub name: String,
    pub name_original: Option<String>,
    pub cover: Option<String>,
    pub rating: Option<f64>,
    pub my_rating: Option<f64>,
    pub date_started: Option<String>,
    pub date_finished: Option<String>,
    pub tags: Vec<String>,
    pub episodes: Option<u32>,
    pub extra: HashMap<String, serde_json::Value>,
}

#[derive(Serialize)]
pub struct GallerySourceStats {
    pub source_id: String,
    pub source_name: String,
    pub source_icon: String,
    pub item_count: usize,
}

#[derive(Serialize)]
pub struct GalleryStats {
    pub total_items: usize,
    pub by_source: Vec<GallerySourceStats>,
}

#[derive(Serialize)]
pub struct GalleryData {
    pub sources: Vec<GallerySourceInfo>,
    pub items: Vec<MediaItem>,
    pub stats: GalleryStats,
}
