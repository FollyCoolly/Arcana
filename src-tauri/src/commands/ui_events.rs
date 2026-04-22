use crate::models::ui_event::UiEvent;
use crate::services::ui_events;
use crate::storage::json_store::resolve_data_dir;

#[tauri::command]
pub fn get_pending_events(event_type: Option<String>) -> Result<Vec<UiEvent>, String> {
    let data_dir = resolve_data_dir()?;
    ui_events::consume_events(&data_dir, event_type.as_deref())
}
