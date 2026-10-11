pub fn plugin<R: tauri::Runtime>() -> tauri::plugin::TauriPlugin<R> { tauri_plugin_global_shortcut::Builder::new().build() }
