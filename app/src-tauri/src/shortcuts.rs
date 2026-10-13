use crate::{resolve, AppState};
use pulse_protocol::Decision;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, Manager, Wry};
use tauri_plugin_global_shortcut::{Code, Modifiers, Shortcut, ShortcutState};

static HOLDING: AtomicBool = AtomicBool::new(false);
const HOLD_MS: u64 = 900;

#[derive(Clone, Copy, PartialEq)] pub enum Key { Allow, Deny, Palette }

impl Key {
    /// Nombre que manda `pulse key <acción>`.
    pub fn parse(s: &str) -> Option<Key> { match s { "allow" => Some(Key::Allow), "deny" => Some(Key::Deny), "palette" => Some(Key::Palette), _ => None } }
}

/// Windows: la app registra Ctrl+Alt+Y / N / Espacio / Enter. En Linux los registra el escritorio y llegan por `pulse key`.
#[cfg_attr(not(windows), allow(dead_code))]
pub fn plugin() -> tauri::plugin::TauriPlugin<Wry> {
    let m = Modifiers::CONTROL | Modifiers::ALT;
    let (yes, no, pal, pal2) = (Shortcut::new(Some(m), Code::KeyY), Shortcut::new(Some(m), Code::KeyN), Shortcut::new(Some(m), Code::Space), Shortcut::new(Some(m), Code::Enter));
    tauri_plugin_global_shortcut::Builder::new()
        .with_shortcuts([yes, no, pal, pal2]).expect("atajos")
        .with_handler(move |app, sc, ev| {
            let k = if sc == &yes { Key::Allow } else if sc == &no { Key::Deny } else { Key::Palette };
            key(app.app_handle(), k, ev.state == ShortcutState::Pressed);
        })
        .build()
}

/// Un atajo se presionó (`pressed`) o se soltó. Permitir un comando delicado pide mantenerlo presionado.
pub fn key(app: &AppHandle, k: Key, pressed: bool) {
    let app = app.clone();
    let st = app.state::<AppState>();
    let (keys, hold) = { let c = st.config.lock().unwrap(); (c.island.keys, c.island.hold) };
    if k == Key::Palette { if pressed { let _ = app.emit("pulse://palette", ()); } return; }
    if !keys { return; }
    let top = st.store.lock().unwrap().first_permission().cloned();
    let Some(p) = top else { return };
    if k == Key::Deny && pressed { resolve(&app, p.id, Decision::Deny); }
    if k == Key::Allow {
        match pressed {
            true if p.danger && hold => {
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
            true => resolve(&app, p.id, Decision::Allow),
            false => HOLDING.store(false, Ordering::SeqCst),
        }
    }
}
