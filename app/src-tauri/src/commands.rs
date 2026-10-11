use crate::{broadcast, config::{self, Config}, now_ms, resolve, AppState};
use pulse_protocol::Decision;
use tauri::{AppHandle, Emitter, State};

#[tauri::command] pub fn get_snapshot(st: State<AppState>) -> crate::state::Snapshot { st.store.lock().unwrap().snapshot() }
#[tauri::command] pub fn get_config(st: State<AppState>) -> Config { st.config.lock().unwrap().clone() }
#[tauri::command] pub fn set_config(app: AppHandle, st: State<AppState>, config: Config) {
    config::save(&config);
    *st.config.lock().unwrap() = config.clone();
    let _ = app.emit("pulse://config", &config);
}
#[tauri::command] pub fn decide(app: AppHandle, id: u64, allow: bool) { resolve(&app, id, if allow { Decision::Allow } else { Decision::Deny }); }
#[tauri::command] pub fn allow_all(app: AppHandle, st: State<AppState>) {
    let ids: Vec<u64> = st.store.lock().unwrap().snapshot().pending.iter().map(|p| p.id).collect();
    for id in ids { resolve(&app, id, Decision::Allow); }
}
#[tauri::command] pub fn set_focus(app: AppHandle, st: State<AppState>, on: bool) {
    let released = { let mut s = st.store.lock().unwrap(); let auto = s.snapshot().focus_auto; s.set_focus(Some(on), auto) };
    if !released.is_empty() {
        let text = serde_json::to_string(&released).unwrap_or_default();
        crate::emit_fx(&app, crate::Fx { kind: "release".into(), session_id: released[0].session_id.clone(), project: released[0].project.clone(), slot: 0, text, visual: true, sound: false });
    }
    broadcast(&app);
}
#[tauri::command] pub fn mute_session(app: AppHandle, st: State<AppState>, session_id: String, minutes: i64) {
    st.store.lock().unwrap().mute(&session_id, if minutes > 0 { now_ms() + minutes * 60_000 } else { 0 });
    broadcast(&app);
}

// Se completan en las Tasks 4 a 6.
#[tauri::command] pub fn daily_summary() {}
#[tauri::command] pub fn set_island_hitbox() {}
#[tauri::command] pub fn island_focus() {}
#[tauri::command] pub fn open_dashboard() {}
#[tauri::command] pub fn toggle_panel() {}
