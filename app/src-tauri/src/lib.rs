pub mod chroma; pub mod commands; pub mod config; pub mod daylog; pub mod focus; pub mod logbook;
pub mod server; pub mod shortcuts; pub mod sound; pub mod state; pub mod tray; pub mod windows;

use pulse_protocol::{Decision, PulseEvent, Reply};
use serde::Serialize;
use state::{Cue, Store};
use std::{collections::HashMap, sync::{atomic::{AtomicU64, Ordering}, Mutex}};
use tauri::{AppHandle, Emitter, Manager};
use tokio::sync::oneshot;

pub struct AppState {
    pub store: Mutex<Store>,
    pub config: Mutex<config::Config>,
    pub waiters: Mutex<HashMap<u64, oneshot::Sender<Reply>>>,
    pub next_id: AtomicU64,
    pub player: sound::Player,
}

#[derive(Clone, Serialize)]
pub struct Fx { pub kind: String, pub session_id: String, pub project: String, pub slot: u8, pub text: String, pub visual: bool, pub sound: bool, pub monitor: usize }

pub fn now_ms() -> i64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0) }

pub fn broadcast(app: &AppHandle) {
    let st = app.state::<AppState>();
    let (snap, tip) = { let s = st.store.lock().unwrap(); (s.snapshot(), s.summary()) };
    let _ = app.emit("pulse://state", &snap);
    tray::update(app, &snap.sessions, &tip);
}

pub fn emit_fx(app: &AppHandle, fx: Fx) { let _ = app.emit("pulse://fx", fx); }

/// Resuelve un permiso pendiente: contesta a `pulse ask`, registra y avisa a las ventanas.
pub fn resolve(app: &AppHandle, id: u64, decision: Decision) { resolve_as(app, id, decision, match decision { Decision::Allow => "allow", Decision::Deny => "deny", Decision::None => "none" }); }

/// Como `resolve`, con la etiqueta que verá la isla (`terminal` = se respondió en la terminal).
pub fn resolve_as(app: &AppHandle, id: u64, decision: Decision, label: &str) { finish(app, id, Reply { decision, answers: None }, label, None); }

/// Contesta una pregunta de Claude con `answers` (pregunta → opción elegida).
pub fn answer(app: &AppHandle, id: u64, answers: serde_json::Value) {
    let summary = answers.as_object().map(|m| m.values().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" · ")).unwrap_or_default();
    finish(app, id, Reply { decision: Decision::Allow, answers: Some(answers) }, "answer", Some(summary));
}

fn finish(app: &AppHandle, id: u64, reply: Reply, label: &str, text: Option<String>) {
    let decision = reply.decision;
    let st = app.state::<AppState>();
    let pending = st.store.lock().unwrap().resolve(id);
    if let Some(tx) = st.waiters.lock().unwrap().remove(&id) { let _ = tx.send(reply); }
    if let Some(p) = pending {
        logbook::append_resolve(&p.session_id, &p.project, id, match decision { Decision::Allow => Some(true), Decision::Deny => Some(false), Decision::None => None }, now_ms());
        emit_fx(app, Fx { kind: "resolved".into(), session_id: p.session_id, project: p.project, slot: 0, text: format!("{label}|{}", text.unwrap_or(p.command)), visual: false, sound: false, monitor: windows::active_monitor_index(app) });
    }
    broadcast(app);
}

/// Quita sesiones de Pulse. Sus permisos o preguntas pendientes se dejan a la terminal.
pub fn forget(app: &AppHandle, ids: &[String]) {
    if ids.is_empty() { return; }
    let st = app.state::<AppState>();
    let pending: Vec<u64> = { let mut s = st.store.lock().unwrap(); ids.iter().flat_map(|id| s.forget(id, now_ms())).collect() };
    for id in pending { resolve_as(app, id, Decision::None, "terminal"); }
    broadcast(app);
}

/// Sesiones cuyo Claude Code ya se cerró (cerrar la pestaña de la terminal no manda `SessionEnd`).
fn closed_sessions(app: &AppHandle) -> Vec<String> {
    let owners = app.state::<AppState>().store.lock().unwrap().owners();
    owners.into_iter().filter(|(_, o)| !pulse_protocol::owner::is_alive(*o)).map(|(id, _)| id).collect()
}

fn handle(app: &AppHandle, inc: server::Incoming) {
    let st = app.state::<AppState>();
    let env = st.store.lock().unwrap().canonical(inc.env);
    logbook::append(&env);
    let cfg = st.config.lock().unwrap().clone();
    let answered = st.store.lock().unwrap().answered_elsewhere(&env);
    for id in answered { resolve_as(app, id, Decision::None, "terminal"); }
    let (cue, focus_on, muted, slot) = { let mut s = st.store.lock().unwrap(); let c = s.apply(&env); (c, s.focus(), s.is_muted(&env.session_id, now_ms()), s.slot_of(&env.session_id)) };
    if let Some(reply) = inc.reply {
        let id = st.next_id.fetch_add(1, Ordering::SeqCst) + 1;
        let timeout_s = if matches!(env.event, PulseEvent::Question { .. }) { pulse_protocol::QUESTION_TIMEOUT_S as i64 } else { cfg.island.ask_timeout_s as i64 };
        st.store.lock().unwrap().add_pending(id, &env, timeout_s * 1000);
        st.waiters.lock().unwrap().insert(id, reply);
    }
    if let Some(cue) = cue {
        let kind = cue.key();
        let mut r = config::route(&cfg, focus_on, kind);
        if muted && cue != Cue::Permission { r.sound = false; r.visual = false; }
        if r.defer { st.store.lock().unwrap().defer(&env, kind); }
        if r.sound { st.player.play(cue, cfg.sounds.volume as f32 / 100.0); }
        let text = match &env.event {
            PulseEvent::Error { message } => message.clone(),
            PulseEvent::Permission { command, .. } => command.clone(),
            PulseEvent::Done => st.store.lock().unwrap().turn_ms(&env.session_id, env.at_ms).map(|m| m.to_string()).unwrap_or_default(),
            _ => String::new(),
        };
        // preguntas y planes que se contestan en la terminal: aviso para la isla
        let notice = match &env.event { PulseEvent::Plan => Some("plan|".to_string()), PulseEvent::Question { questions } if !env.ask => Some(format!("question|{}", questions.get(0).and_then(|q| q.get("question")).and_then(|q| q.as_str()).unwrap_or(""))), _ => None };
        if let Some(t) = notice { emit_fx(app, Fx { kind: "notice".into(), session_id: env.session_id.clone(), project: env.project.clone(), slot, text: t, visual: false, sound: false, monitor: windows::active_monitor_index(app) }); }
        emit_fx(app, Fx { kind: kind.into(), session_id: env.session_id.clone(), project: env.project.clone(), slot, text, visual: r.visual, sound: r.sound, monitor: windows::active_monitor_index(app) });
        if r.visual { chroma::pulse(app, kind, slot); }
    }
    broadcast(app);
}

/// Linux con Wayland: la app corre por XWayland, porque Wayland no deja que una ventana se ponga arriba de las demás
/// ni elija su lugar (bordes, isla y Bolsillo lo necesitan). Va antes de iniciar GTK y antes de crear hilos.
#[cfg(target_os = "linux")]
fn linux_env() {
    if std::env::var_os("WAYLAND_DISPLAY").is_some() && std::env::var_os("GDK_BACKEND").is_none() { std::env::set_var("GDK_BACKEND", "x11"); }
}

pub fn run() {
    #[cfg(target_os = "linux")] linux_env();
    let endpoint = pulse_protocol::endpoint();
    let first = match tauri::async_runtime::block_on(async { server::create_first(&endpoint) }) { Ok(s) => s, Err(_) => return };
    let builder = tauri::Builder::default();
    // En Linux los atajos globales los registra el escritorio (scripts/install.sh) y llegan por `pulse key`.
    #[cfg(windows)] let builder = builder.plugin(shortcuts::plugin());
    builder
        .manage(AppState { store: Mutex::new(Store::new(now_ms())), config: Mutex::new(config::load()), waiters: Mutex::new(HashMap::new()), next_id: AtomicU64::new(0), player: sound::Player::start() })
        .invoke_handler(tauri::generate_handler![commands::get_snapshot, commands::get_config, commands::set_config, commands::decide, commands::answer, commands::skip, commands::hold_question, commands::allow_all, commands::set_focus, commands::mute_session, commands::forget_sessions, commands::daily_summary, commands::log_today, commands::set_island_hitbox, commands::island_focus, commands::open_dashboard, commands::toggle_panel, commands::flyout_fit, commands::flyout_hide, commands::open_palette, commands::quit])
        .setup(move |app| {
            let h = app.handle().clone();
            tray::create(&h, windows::toggle_panel, windows::toggle_flyout)?;
            windows::create_all(&h)?;
            windows::create_flyout(&h)?;
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let (ktx, mut krx) = tokio::sync::mpsc::unbounded_channel::<String>();
            let _ = server::KEYS.set(ktx);
            tauri::async_runtime::spawn(server::serve(first, endpoint, tx));
            let hk = h.clone();
            // `pulse key` es un golpe de tecla sin "soltar": se manda presionar y soltar, así un comando delicado no se aprueba solo.
            tauri::async_runtime::spawn(async move { while let Some(k) = krx.recv().await { if let Some(k) = shortcuts::Key::parse(&k) { shortcuts::key(&hk, k, true); shortcuts::key(&hk, k, false); } } });
            let h1 = h.clone();
            tauri::async_runtime::spawn(async move { while let Some(inc) = rx.recv().await { handle(&h1, inc); } });
            let h2 = h.clone();
            tauri::async_runtime::spawn(async move { for tick in 0u64.. {   // vencimiento de permisos, sesiones cerradas y autocierre
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                if tick % 3 == 0 { let closed = closed_sessions(&h2); forget(&h2, &closed); }
                let expired = h2.state::<AppState>().store.lock().unwrap().expire(now_ms());
                for id in expired { resolve(&h2, id, Decision::None); }
                // `pulse ask` se cerró (Claude Code dejó de esperar al hook): se respondió en la terminal
                let gone: Vec<u64> = h2.state::<AppState>().waiters.lock().unwrap().iter().filter(|(_, tx)| tx.is_closed()).map(|(id, _)| *id).collect();
                for id in gone { resolve_as(&h2, id, Decision::None, "terminal"); }
                if tray::theme_changed() { broadcast(&h2); }   // barra de tareas clara u oscura: otro color de tinta
                if h2.state::<AppState>().store.lock().unwrap().should_exit(now_ms()) { chroma::shutdown(); h2.exit(0); }
            } });
            focus::start(&h);
            chroma::start(&h);
            Ok(())
        })
        .build(tauri::generate_context!()).expect("no se pudo iniciar Pulse")
        .run(|_app, e| if let tauri::RunEvent::ExitRequested { api, code: None, .. } = e { api.prevent_exit(); });
}
