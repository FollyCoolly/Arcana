use std::path::Path;

use crate::models::gallery::*;
use crate::storage::json_store::{read_json_file, resolve_data_dir};

#[tauri::command]
pub fn load_gallery() -> Result<GalleryData, String> {
    let data_dir = resolve_data_dir()?;
    let sources_path = data_dir.join("gallery_sources.json");

    let source_file: GallerySourceFile = read_json_file(&sources_path)?;

    let mut all_items: Vec<MediaItem> = Vec::new();
    let mut source_infos: Vec<GallerySourceInfo> = Vec::new();

    for source in &source_file.sources {
        let source_path = if Path::new(&source.path).is_absolute() {
            Path::new(&source.path).to_path_buf()
        } else {
            data_dir.join(&source.path)
        };
        let icon = source.icon.clone().unwrap_or_else(|| "🎬".to_string());

        let item_file: GalleryItemFile = match read_json_file(&source_path) {
            Ok(f) => f,
            Err(e) => {
                eprintln!(
                    "[gallery] Warning: failed to read source '{}': {}",
                    source.id, e
                );
                source_infos.push(GallerySourceInfo {
                    id: source.id.clone(),
                    name: source.name.clone(),
                    icon,
                    media_type: source.media_type.clone(),
                    item_count: 0,
                });
                continue;
            }
        };

        let mut source_items: Vec<MediaItem> = Vec::new();

        for (idx, raw) in item_file.items.iter().enumerate() {
            let id = format!("{}::{}", source.id, idx);

            source_items.push(MediaItem {
                id,
                source_id: source.id.clone(),
                name: raw.name.clone(),
                name_original: raw.name_original.clone(),
                cover: raw.cover.clone(),
                rating: raw.rating,
                my_rating: raw.my_rating,
                date_started: raw.date_started.clone(),
                date_finished: raw.date_finished.clone(),
                tags: raw.tags.clone().unwrap_or_default(),
                episodes: raw.episodes,
                extra: raw.extra.clone(),
            });
        }

        source_infos.push(GallerySourceInfo {
            id: source.id.clone(),
            name: source.name.clone(),
            icon,
            media_type: source.media_type.clone(),
            item_count: source_items.len(),
        });

        all_items.extend(source_items);
    }

    // Compute stats
    let total_items = all_items.len();

    let by_source: Vec<GallerySourceStats> = source_infos
        .iter()
        .map(|si| GallerySourceStats {
            source_id: si.id.clone(),
            source_name: si.name.clone(),
            source_icon: si.icon.clone(),
            item_count: si.item_count,
        })
        .collect();

    let stats = GalleryStats {
        total_items,
        by_source,
    };

    Ok(GalleryData {
        sources: source_infos,
        items: all_items,
        stats,
    })
}
