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
#[tauri::command] pub fn answer(app: AppHandle, id: u64, answers: serde_json::Value) { crate::answer(&app, id, answers); }
/// Pausa la cuenta regresiva de una pregunta mientras la contestas en la isla.
#[tauri::command] pub fn hold_question(app: AppHandle, st: State<AppState>, id: u64, on: bool) { st.store.lock().unwrap().pause(id, on, now_ms()); broadcast(&app); }
/// Deja la pregunta para la terminal (no decide nada).
#[tauri::command] pub fn skip(app: AppHandle, id: u64) { crate::resolve_as(&app, id, Decision::None, "skip"); }
#[tauri::command] pub fn allow_all(app: AppHandle, st: State<AppState>) {
    let ids: Vec<u64> = st.store.lock().unwrap().snapshot().pending.iter().filter(|p| p.kind == "permission").map(|p| p.id).collect();
    for id in ids { resolve(&app, id, Decision::Allow); }
}
#[tauri::command] pub fn set_focus(app: AppHandle, st: State<AppState>, on: bool) {
    let released = { let mut s = st.store.lock().unwrap(); let auto = s.snapshot().focus_auto; s.set_focus(Some(on), auto) };
    if !released.is_empty() {
        let text = serde_json::to_string(&released).unwrap_or_default();
        crate::emit_fx(&app, crate::Fx { kind: "release".into(), session_id: released[0].session_id.clone(), project: released[0].project.clone(), slot: 0, text, visual: true, sound: false, monitor: crate::windows::active_monitor_index(&app) });
    }
    broadcast(&app);
}
#[tauri::command] pub fn mute_session(app: AppHandle, st: State<AppState>, session_id: String, minutes: i64) {
    st.store.lock().unwrap().mute(&session_id, if minutes > 0 { now_ms() + minutes * 60_000 } else { 0 });
    broadcast(&app);
}

#[tauri::command] pub fn log_today() -> Vec<crate::logbook::LogItem> { crate::logbook::today(500) }
#[tauri::command] pub fn daily_summary() -> crate::daylog::DaySummary { crate::daylog::today() }
#[derive(serde::Deserialize)] pub struct Rect { x: f64, y: f64, w: f64, h: f64 }
#[tauri::command] pub fn set_island_hitbox(hb: State<crate::windows::Hitbox>, rect: Option<Rect>) { *hb.0.lock().unwrap() = rect.map(|r| (r.x - 6.0, r.y - 6.0, r.w + 12.0, r.h + 12.0)); }
#[tauri::command] pub fn island_focus(app: AppHandle, on: bool) { crate::windows::island_keyboard(&app, on); }
// Async: en Windows, crear una ventana desde un comando síncrono bloquea el hilo principal.
#[tauri::command] pub async fn toggle_panel(app: AppHandle) { crate::windows::toggle_panel(&app) }
#[tauri::command] pub async fn open_dashboard(app: AppHandle, view: Option<String>) { crate::windows::open_dashboard(&app, view.as_deref()) }
#[tauri::command] pub fn flyout_fit(app: AppHandle, height: f64) { crate::windows::fit_flyout(&app, height) }
#[tauri::command] pub fn flyout_hide(app: AppHandle) { crate::windows::hide_flyout(&app) }
/// Lo mismo que Ctrl+Alt+Espacio: la isla abre su paleta (Comando) o su menú (Orbe).
#[tauri::command] pub fn open_palette(app: AppHandle) { let _ = app.emit("pulse://palette", ()); }
#[tauri::command] pub fn quit(app: AppHandle) { crate::chroma::shutdown(); app.exit(0); }
