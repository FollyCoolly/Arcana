use std::collections::HashMap;
use std::fs;
use std::path::Path;

use regex::Regex;

use crate::models::crafting::*;
use crate::storage::json_store::{read_json_file, resolve_data_dir};

/// Known frontmatter keys (Chinese → English pairs) that map to public fields.
const KNOWN_KEYS_ZH: &[&str] = &[
    "名称", "类型", "份数", "难度", "时间", "材料", "步骤", "标签", "来源",
];
const KNOWN_KEYS_EN: &[&str] = &[
    "name", "type", "servings", "difficulty", "time", "ingredients", "steps", "tags", "source",
];

fn yaml_value_to_json(val: &serde_yaml::Value) -> serde_json::Value {
    match val {
        serde_yaml::Value::Null => serde_json::Value::Null,
        serde_yaml::Value::Bool(b) => serde_json::Value::Bool(*b),
        serde_yaml::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                serde_json::Value::Number(i.into())
            } else if let Some(f) = n.as_f64() {
                serde_json::json!(f)
            } else {
                serde_json::Value::Null
            }
        }
        serde_yaml::Value::String(s) => serde_json::Value::String(s.clone()),
        serde_yaml::Value::Sequence(seq) => {
            serde_json::Value::Array(seq.iter().map(yaml_value_to_json).collect())
        }
        serde_yaml::Value::Mapping(map) => {
            let mut obj = serde_json::Map::new();
            for (k, v) in map {
                let key = match k {
                    serde_yaml::Value::String(s) => s.clone(),
                    _ => format!("{:?}", k),
                };
                obj.insert(key, yaml_value_to_json(v));
            }
            serde_json::Value::Object(obj)
        }
        serde_yaml::Value::Tagged(tagged) => yaml_value_to_json(&tagged.value),
    }
}

fn get_string(map: &HashMap<String, serde_yaml::Value>, zh: &str, en: &str) -> Option<String> {
    let val = map.get(zh).or_else(|| map.get(en))?;
    match val {
        serde_yaml::Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(trimmed.to_string())
            }
        }
        serde_yaml::Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn get_string_list(map: &HashMap<String, serde_yaml::Value>, zh: &str, en: &str) -> Vec<String> {
    let val = match map.get(zh).or_else(|| map.get(en)) {
        Some(v) => v,
        None => return Vec::new(),
    };
    match val {
        serde_yaml::Value::Sequence(seq) => seq
            .iter()
            .filter_map(|v| match v {
                serde_yaml::Value::String(s) => {
                    let trimmed = s.trim();
                    if trimmed.is_empty() {
                        None
                    } else {
                        Some(trimmed.to_string())
                    }
                }
                serde_yaml::Value::Number(n) => Some(n.to_string()),
                _ => None,
            })
            .collect(),
        serde_yaml::Value::String(s) => {
            // Single string → treat as one-element list
            let trimmed = s.trim();
            if trimmed.is_empty() {
                Vec::new()
            } else {
                vec![trimmed.to_string()]
            }
        }
        _ => Vec::new(),
    }
}

fn is_known_key(key: &str) -> bool {
    KNOWN_KEYS_ZH.contains(&key) || KNOWN_KEYS_EN.contains(&key)
}

fn extract_frontmatter(content: &str) -> Option<&str> {
    let trimmed = content.trim_start();
    if !trimmed.starts_with("---") {
        return None;
    }
    let after_first = &trimmed[3..];
    let end = after_first.find("\n---")?;
    Some(&after_first[..end])
}

fn extract_image(content: &str, md_dir: &Path) -> Option<String> {
    let re = Regex::new(r"!\[\[(.+?\.(png|jpg|jpeg|webp))\]\]").ok()?;
    let caps = re.captures(content)?;
    let filename = caps.get(1)?.as_str();
    let abs_path = md_dir.join(filename);
    Some(abs_path.to_string_lossy().replace('\\', "/"))
}

fn parse_recipe_md(path: &Path, source_id: &str) -> Option<RecipeWithComputed> {
    let content = fs::read_to_string(path).ok()?;
    let fm_str = extract_frontmatter(&content)?;

    let frontmatter: HashMap<String, serde_yaml::Value> =
        serde_yaml::from_str(fm_str).ok()?;

    let file_stem = path.file_stem()?.to_string_lossy().to_string();
    let md_dir = path.parent()?;

    let name = get_string(&frontmatter, "名称", "name")
        .unwrap_or_else(|| file_stem.clone());
    let recipe_type = get_string(&frontmatter, "类型", "type");
    let servings = get_string(&frontmatter, "份数", "servings");
    let difficulty = get_string(&frontmatter, "难度", "difficulty");
    let time = get_string(&frontmatter, "时间", "time");
    let ingredients = get_string_list(&frontmatter, "材料", "ingredients");
    let steps = get_string_list(&frontmatter, "步骤", "steps");
    let tags = get_string_list(&frontmatter, "标签", "tags");
    let source = get_string(&frontmatter, "来源", "source");

    // Extra: all keys not in known lists
    let mut extra = HashMap::new();
    for (k, v) in &frontmatter {
        if !is_known_key(k.as_str()) {
            extra.insert(k.clone(), yaml_value_to_json(v));
        }
    }

    let image = extract_image(&content, md_dir);

    let ingredient_count = ingredients.len();
    let step_count = steps.len();

    let id = format!("{}::{}", source_id, file_stem);

    Some(RecipeWithComputed {
        id,
        source_id: source_id.to_string(),
        name,
        recipe_type,
        servings,
        difficulty,
        time,
        ingredients,
        steps,
        tags,
        source,
        image,
        extra,
        ingredient_count,
        step_count,
    })
}

#[tauri::command]
pub fn load_crafting() -> Result<CraftingData, String> {
    let data_dir = resolve_data_dir()?;
    let sources_path = data_dir.join("recipe_sources.json");

    let source_file: RecipeSourceFile = read_json_file(&sources_path)?;

    let mut all_recipes: Vec<RecipeWithComputed> = Vec::new();
    let mut source_infos: Vec<RecipeSourceInfo> = Vec::new();

    for source in &source_file.sources {
        let source_path = if Path::new(&source.path).is_absolute() {
            Path::new(&source.path).to_path_buf()
        } else {
            data_dir.join(&source.path)
        };
        let icon = source.icon.clone().unwrap_or_else(|| "📖".to_string());

        if !source_path.is_dir() {
            eprintln!(
                "[crafting] Warning: source directory not found: {}",
                source.path
            );
            source_infos.push(RecipeSourceInfo {
                id: source.id.clone(),
                name: source.name.clone(),
                icon,
                recipe_count: 0,
            });
            continue;
        }

        let entries = fs::read_dir(&source_path).map_err(|e| {
            format!("Failed to read directory '{}': {}", source.path, e)
        })?;

        let mut source_recipes: Vec<RecipeWithComputed> = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }

            match parse_recipe_md(&path, &source.id) {
                Some(recipe) => source_recipes.push(recipe),
                None => {
                    eprintln!(
                        "[crafting] Warning: failed to parse {}",
                        path.display()
                    );
                }
            }
        }

        source_infos.push(RecipeSourceInfo {
            id: source.id.clone(),
            name: source.name.clone(),
            icon,
            recipe_count: source_recipes.len(),
        });

        all_recipes.extend(source_recipes);
    }

    // Compute stats
    let total_recipes = all_recipes.len();

    let by_source: Vec<RecipeSourceStats> = source_infos
        .iter()
        .map(|si| RecipeSourceStats {
            source_id: si.id.clone(),
            source_name: si.name.clone(),
            source_icon: si.icon.clone(),
            recipe_count: si.recipe_count,
        })
        .collect();

    // By type
    let mut type_map: HashMap<String, usize> = HashMap::new();
    for recipe in &all_recipes {
        let t = recipe
            .recipe_type
            .clone()
            .unwrap_or_else(|| "未分类".to_string());
        *type_map.entry(t).or_insert(0) += 1;
    }
    let mut by_type: Vec<RecipeTypeStats> = type_map
        .into_iter()
        .map(|(name, recipe_count)| RecipeTypeStats { name, recipe_count })
        .collect();
    by_type.sort_by(|a, b| b.recipe_count.cmp(&a.recipe_count));

    let stats = RecipeStats {
        total_recipes,
        by_source,
        by_type,
    };

    Ok(CraftingData {
        sources: source_infos,
        recipes: all_recipes,
        stats,
    })
}
