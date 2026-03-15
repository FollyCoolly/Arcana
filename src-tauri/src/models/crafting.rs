use serde::{Deserialize, Serialize};
use std::collections::HashMap;

// ── Config file ──

#[derive(Deserialize)]
pub struct RecipeSourceFile {
    pub version: u32,
    pub sources: Vec<RecipeSource>,
}

#[derive(Deserialize)]
pub struct RecipeSource {
    pub id: String,
    pub name: String,
    pub path: String,
    pub icon: Option<String>,
}

// ── Response structures ──

#[derive(Serialize)]
pub struct RecipeSourceInfo {
    pub id: String,
    pub name: String,
    pub icon: String,
    pub recipe_count: usize,
}

#[derive(Serialize)]
pub struct RecipeWithComputed {
    pub id: String,
    pub source_id: String,
    pub name: String,
    pub recipe_type: Option<String>,
    pub servings: Option<String>,
    pub difficulty: Option<String>,
    pub time: Option<String>,
    pub ingredients: Vec<String>,
    pub steps: Vec<String>,
    pub tags: Vec<String>,
    pub source: Option<String>,
    pub image: Option<String>,
    pub extra: HashMap<String, serde_json::Value>,
    pub ingredient_count: usize,
    pub step_count: usize,
}

#[derive(Serialize)]
pub struct RecipeSourceStats {
    pub source_id: String,
    pub source_name: String,
    pub source_icon: String,
    pub recipe_count: usize,
}

#[derive(Serialize)]
pub struct RecipeTypeStats {
    pub name: String,
    pub recipe_count: usize,
}

#[derive(Serialize)]
pub struct RecipeStats {
    pub total_recipes: usize,
    pub by_source: Vec<RecipeSourceStats>,
    pub by_type: Vec<RecipeTypeStats>,
}

#[derive(Serialize)]
pub struct CraftingData {
    pub sources: Vec<RecipeSourceInfo>,
    pub recipes: Vec<RecipeWithComputed>,
    pub stats: RecipeStats,
}
