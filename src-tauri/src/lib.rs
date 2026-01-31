use tauri::Manager;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

// Learn more about Tauri commands at https://tauri.app/develop/calling-rust/
#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();
            
            // 注册全局快捷键 Cmd+Shift+R (macOS) 或 Ctrl+Shift+R (Windows/Linux)
            #[cfg(target_os = "macos")]
            let shortcut = "Command+Shift+R";
            
            #[cfg(not(target_os = "macos"))]
            let shortcut = "Ctrl+Shift+R";
            
            app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, event| {
                // 只在按键按下时触发，避免释放时也触发
                if event.state == ShortcutState::Pressed {
                    if window.is_visible().unwrap_or(false) {
                        let _ = window.hide();
                    } else {
                        let _ = window.show();
                        let _ = window.set_focus();
                    }
                }
            })?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![greet])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
