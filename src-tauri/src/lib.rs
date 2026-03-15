mod commands;
mod models;
mod storage;

use tauri::{Emitter, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            let _ = window.set_decorations(false);
            let _ = window.set_shadow(false);

            // 注册全局快捷键 Cmd+Shift+R (macOS) 或 Ctrl+Shift+R (Windows/Linux)
            #[cfg(target_os = "macos")]
            let shortcut = "Command+Shift+R";

            #[cfg(not(target_os = "macos"))]
            let shortcut = "Ctrl+Shift+R";

            app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, event| {
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
                            let _ = window.set_position(tauri::PhysicalPosition::new(pos.x, pos.y));
                            let _ = window.set_size(tauri::PhysicalSize::new(size.width, size.height));
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
            commands::crafting::load_crafting,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
