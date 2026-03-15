use std::collections::HashMap;
use std::fs;
use std::path::Path;

use regex::Regex;

use crate::models::item::*;
use crate::storage::date_utils::calculate_days_since;
use crate::storage::json_store::{read_json_file, resolve_data_dir};

/// Known frontmatter keys that map to public fields (Chinese keys).
const KNOWN_KEYS: &[&str] = &[
    "名称", "品牌", "价格", "购入日期", "购入方式", "主类", "品类", "颜色",
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

fn get_string(map: &HashMap<String, serde_yaml::Value>, key: &str) -> Option<String> {
    match map.get(key)? {
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

fn get_number(map: &HashMap<String, serde_yaml::Value>, key: &str) -> Option<f64> {
    match map.get(key)? {
        serde_yaml::Value::Number(n) => n.as_f64(),
        serde_yaml::Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
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

fn parse_md_file(
    path: &Path,
    source_id: &str,
) -> Option<ItemWithComputed> {
    let content = fs::read_to_string(path).ok()?;
    let fm_str = extract_frontmatter(&content)?;

    let frontmatter: HashMap<String, serde_yaml::Value> =
        serde_yaml::from_str(fm_str).ok()?;

    let file_stem = path.file_stem()?.to_string_lossy().to_string();
    let md_dir = path.parent()?;

    let name = get_string(&frontmatter, "名称").unwrap_or_else(|| file_stem.clone());
    let brand = get_string(&frontmatter, "品牌");
    let price = get_number(&frontmatter, "价格");
    let purchase_date = get_string(&frontmatter, "购入日期");
    let purchase_channel = get_string(&frontmatter, "购入方式");
    let main_category = get_string(&frontmatter, "主类");
    let sub_category = get_string(&frontmatter, "品类");
    let color = get_string(&frontmatter, "颜色");

    // Extra: all keys not in KNOWN_KEYS
    let mut extra = HashMap::new();
    for (k, v) in &frontmatter {
        if !KNOWN_KEYS.contains(&k.as_str()) {
            extra.insert(k.clone(), yaml_value_to_json(v));
        }
    }

    let image = extract_image(&content, md_dir);

    // Computed fields
    let days_owned = purchase_date
        .as_ref()
        .and_then(|d| calculate_days_since(d).ok());

    let daily_cost = match (price, days_owned) {
        (Some(p), Some(d)) => Some(p / (d.max(1) as f64)),
        _ => None,
    };

    let id = format!("{}::{}", source_id, file_stem);

    Some(ItemWithComputed {
        id,
        source_id: source_id.to_string(),
        name,
        brand,
        price,
        purchase_date,
        purchase_channel,
        main_category,
        sub_category,
        color,
        image,
        extra,
        days_owned,
        daily_cost,
    })
}

#[tauri::command]
pub fn load_items() -> Result<ItemData, String> {
    let data_dir = resolve_data_dir()?;
    let sources_path = data_dir.join("item_sources.json");

    let source_file: ItemSourceFile = read_json_file(&sources_path)?;

    let mut all_items: Vec<ItemWithComputed> = Vec::new();
    let mut source_infos: Vec<ItemSourceInfo> = Vec::new();

    for source in &source_file.sources {
        let source_path = Path::new(&source.path);
        let icon = source.icon.clone().unwrap_or_else(|| "📦".to_string());

        if !source_path.is_dir() {
            eprintln!(
                "[items] Warning: source directory not found: {}",
                source.path
            );
            source_infos.push(ItemSourceInfo {
                id: source.id.clone(),
                name: source.name.clone(),
                icon,
                item_count: 0,
            });
            continue;
        }

        let entries = fs::read_dir(source_path).map_err(|e| {
            format!("Failed to read directory '{}': {}", source.path, e)
        })?;

        let mut source_items: Vec<ItemWithComputed> = Vec::new();

        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }

            match parse_md_file(&path, &source.id) {
                Some(item) => source_items.push(item),
                None => {
                    eprintln!(
                        "[items] Warning: failed to parse {}",
                        path.display()
                    );
                }
            }
        }

        source_infos.push(ItemSourceInfo {
            id: source.id.clone(),
            name: source.name.clone(),
            icon,
            item_count: source_items.len(),
        });

        all_items.extend(source_items);
    }

    // Compute stats
    let total_items = all_items.len();
    let total_value: f64 = all_items.iter().filter_map(|i| i.price).sum();

    let daily_costs: Vec<f64> = all_items.iter().filter_map(|i| i.daily_cost).collect();
    let average_daily_cost = if daily_costs.is_empty() {
        0.0
    } else {
        daily_costs.iter().sum::<f64>() / daily_costs.len() as f64
    };

    // By source
    let by_source: Vec<SourceStats> = source_infos
        .iter()
        .map(|si| {
            let items_in_source: Vec<&ItemWithComputed> =
                all_items.iter().filter(|i| i.source_id == si.id).collect();
            SourceStats {
                source_id: si.id.clone(),
                source_name: si.name.clone(),
                source_icon: si.icon.clone(),
                item_count: items_in_source.len(),
                total_value: items_in_source.iter().filter_map(|i| i.price).sum(),
            }
        })
        .collect();

    // By main category
    let mut cat_map: HashMap<String, (usize, f64)> = HashMap::new();
    for item in &all_items {
        let cat = item
            .main_category
            .clone()
            .unwrap_or_else(|| "未分类".to_string());
        let entry = cat_map.entry(cat).or_insert((0, 0.0));
        entry.0 += 1;
        if let Some(p) = item.price {
            entry.1 += p;
        }
    }
    let mut by_main_category: Vec<CategoryStats> = cat_map
        .into_iter()
        .map(|(name, (item_count, total_value))| CategoryStats {
            name,
            item_count,
            total_value,
        })
        .collect();
    by_main_category.sort_by(|a, b| b.item_count.cmp(&a.item_count));

    let stats = ItemStats {
        total_items,
        total_value,
        average_daily_cost,
        by_source,
        by_main_category,
    };

    Ok(ItemData {
        sources: source_infos,
        items: all_items,
        stats,
    })
}
