pub mod agent;
mod commands;
pub mod models;
pub mod services;
pub mod storage;

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::PathBuf;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

fn infer_referer(url: &str) -> Option<&'static str> {
    if url.contains("doubanio.com") {
        Some("https://movie.douban.com/")
    } else {
        None
    }
}

fn url_to_cache_path(cache_dir: &std::path::Path, url: &str) -> PathBuf {
    let mut hasher = DefaultHasher::new();
    url.hash(&mut hasher);
    let hash = hasher.finish();
    let ext = url.rsplit('.').next().unwrap_or("jpg");
    cache_dir.join(format!("{:016x}.{}", hash, ext))
}

fn do_fetch(
    client: &reqwest::blocking::Client,
    url: &str,
) -> Result<(u16, String, Vec<u8>), String> {
    let mut req = client.get(url);
    if let Some(referer) = infer_referer(url) {
        req = req.header("Referer", referer);
    }
    let resp = req.send().map_err(|e| e.to_string())?;
    let content_type = resp
        .headers()
        .get("content-type")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("image/jpeg")
        .to_string();
    let status = resp.status().as_u16();
    let bytes = resp.bytes().map_err(|e| e.to_string())?;
    Ok((status, content_type, bytes.to_vec()))
}

fn resolve_cache_dir() -> PathBuf {
    let cwd = std::env::current_dir().unwrap_or_default();
    let candidates = [cwd.join("data"), cwd.join("..").join("data")];
    for c in &candidates {
        if c.is_dir() {
            let cache = c.join("gallery").join(".imgcache");
            let _ = std::fs::create_dir_all(&cache);
            return cache;
        }
    }
    let fallback = cwd.join(".imgcache");
    let _ = std::fs::create_dir_all(&fallback);
    fallback
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let proxy_client = Arc::new(
        reqwest::blocking::Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36")
            .build()
            .expect("Failed to build HTTP client"),
    );
    let inflight = Arc::new(AtomicUsize::new(0));
    let cache_dir = Arc::new(resolve_cache_dir());
    const MAX_CONCURRENT: usize = 8;

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .register_asynchronous_uri_scheme_protocol("imgproxy", move |_ctx, request, responder| {
            let uri = request.uri().to_string();
            let encoded = match uri.find("localhost/") {
                Some(pos) => &uri[pos + "localhost/".len()..],
                None => {
                    responder.respond(
                        tauri::http::Response::builder()
                            .status(400)
                            .body(b"Bad request".to_vec())
                            .unwrap(),
                    );
                    return;
                }
            };
            let original_url = percent_encoding::percent_decode_str(encoded)
                .decode_utf8_lossy()
                .to_string();

            let client = Arc::clone(&proxy_client);
            let counter = Arc::clone(&inflight);
            let cache = Arc::clone(&cache_dir);

            std::thread::spawn(move || {
                let cache_path = url_to_cache_path(&cache, &original_url);

                // Serve from disk cache if available
                if cache_path.exists() {
                    if let Ok(bytes) = std::fs::read(&cache_path) {
                        let ext = cache_path
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("jpg");
                        let ct = match ext {
                            "png" => "image/png",
                            "webp" => "image/webp",
                            "gif" => "image/gif",
                            _ => "image/jpeg",
                        };
                        responder.respond(
                            tauri::http::Response::builder()
                                .status(200)
                                .header("Content-Type", ct)
                                .body(bytes)
                                .unwrap(),
                        );
                        return;
                    }
                }

                // Wait until a slot is available
                loop {
                    let current = counter.load(Ordering::Relaxed);
                    if current < MAX_CONCURRENT {
                        if counter
                            .compare_exchange(
                                current,
                                current + 1,
                                Ordering::AcqRel,
                                Ordering::Relaxed,
                            )
                            .is_ok()
                        {
                            break;
                        }
                    }
                    std::thread::sleep(std::time::Duration::from_millis(50));
                }

                // Try up to 2 times
                let result = do_fetch(&client, &original_url).or_else(|_| {
                    std::thread::sleep(std::time::Duration::from_secs(1));
                    do_fetch(&client, &original_url)
                });

                counter.fetch_sub(1, Ordering::AcqRel);

                match result {
                    Ok((status, content_type, bytes)) => {
                        // Write to cache (only for successful image responses)
                        if status == 200 {
                            let _ = std::fs::write(&cache_path, &bytes);
                        }
                        responder.respond(
                            tauri::http::Response::builder()
                                .status(status)
                                .header("Content-Type", &content_type)
                                .body(bytes)
                                .unwrap(),
                        );
                    }
                    Err(e) => {
                        eprintln!("[imgproxy] Failed: {} — {}", original_url, e);
                        responder.respond(
                            tauri::http::Response::builder()
                                .status(502)
                                .body(b"Fetch failed".to_vec())
                                .unwrap(),
                        );
                    }
                }
            });
        })
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            let _ = window.set_decorations(false);
            let _ = window.set_shadow(false);

            // 注册全局快捷键 Cmd+Shift+R (macOS) 或 Ctrl+Shift+R (Windows/Linux)
            #[cfg(target_os = "macos")]
            let shortcut = "Command+Shift+R";

            #[cfg(not(target_os = "macos"))]
            let shortcut = "Ctrl+Shift+R";

            app.global_shortcut()
                .on_shortcut(shortcut, move |_app, _shortcut, event| {
                    // 只在按键按下时触发，避免释放时也触发
                    if event.state == ShortcutState::Pressed {
                        if window.is_visible().unwrap_or(false) {
                            let _ = window.set_always_on_top(false);
                            let _ = window.hide();
                        } else {
                            // 展开到主显示器全屏
                            if let Ok(Some(monitor)) = window.primary_monitor() {
                                let size = monitor.size();
                                let pos = monitor.position();
                                let _ =
                                    window.set_position(tauri::PhysicalPosition::new(pos.x, pos.y));
                                let _ = window
                                    .set_size(tauri::PhysicalSize::new(size.width, size.height));
                            }
                            let _ = window.set_always_on_top(true);
                            let _ = window.show();
                            let _ = window.set_focus();
                            let _ = window.emit("reality://summoned", ());
                        }
                    }
                })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::status::load_status_data,
            commands::weather::get_weather,
            commands::achievements::load_achievements,
            commands::skills::load_skills,
            commands::items::load_items,
            commands::gallery::load_gallery,
            commands::missions::load_missions,
            commands::missions::load_main_menu_missions,
            commands::missions::update_mission_status,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
