use tauri::AppHandle;

pub fn create_all(_: &AppHandle) -> tauri::Result<()> { Ok(()) }
pub fn toggle_panel(_: &AppHandle) {}
pub fn open_dashboard(_: &AppHandle) {}
