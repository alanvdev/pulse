use crate::{resolve, AppState};
use pulse_protocol::Decision;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{Emitter, Manager, Wry};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};

static HOLDING: AtomicBool = AtomicBool::new(false);
const HOLD_MS: u64 = 900;

pub fn plugin() -> tauri::plugin::TauriPlugin<Wry> {
    let m = Modifiers::CONTROL | Modifiers::ALT;
    let (yes, no, pal, pal2) = (Shortcut::new(Some(m), Code::KeyY), Shortcut::new(Some(m), Code::KeyN), Shortcut::new(Some(m), Code::Space), Shortcut::new(Some(m), Code::Enter));
    tauri_plugin_global_shortcut::Builder::new()
        .with_shortcuts([yes, no, pal, pal2]).expect("atajos")
        .with_handler(move |app, sc, ev| {
            let app = app.app_handle().clone();
            let st = app.state::<AppState>();
            let (keys, hold) = { let c = st.config.lock().unwrap(); (c.island.keys, c.island.hold) };
            if (sc == &pal || sc == &pal2) && ev.state == ShortcutState::Pressed { let _ = app.emit("pulse://palette", ()); return; }
            if !keys { return; }
            let top = st.store.lock().unwrap().first_permission().cloned();
            let Some(p) = top else { return };
            if sc == &no && ev.state == ShortcutState::Pressed { resolve(&app, p.id, Decision::Deny); }
            if sc == &yes {
                match ev.state {
                    ShortcutState::Pressed if p.danger && hold => {
                        HOLDING.store(true, Ordering::SeqCst);
                        let a = app.clone();
                        tauri::async_runtime::spawn(async move {
                            for i in 1..=18u64 {
                                tokio::time::sleep(std::time::Duration::from_millis(HOLD_MS / 18)).await;
                                if !HOLDING.load(Ordering::SeqCst) { let _ = a.emit("pulse://hold", serde_json::json!({ "progress": 0 })); return; }
                                let _ = a.emit("pulse://hold", serde_json::json!({ "progress": i as f64 / 18.0 }));
                            }
                            HOLDING.store(false, Ordering::SeqCst);
                            let _ = a.emit("pulse://hold", serde_json::json!({ "progress": 0 }));
                            resolve(&a, p.id, Decision::Allow);
                        });
                    }
                    ShortcutState::Pressed => resolve(&app, p.id, Decision::Allow),
                    ShortcutState::Released => HOLDING.store(false, Ordering::SeqCst),
                }
            }
        })
        .build()
}
