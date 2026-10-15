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

/// `monitor`: dónde pulsa con "monitor activo"; `None` (Linux, donde no se puede saber): en todos.
#[derive(Clone, Serialize)]
pub struct Fx { pub kind: String, pub session_id: String, pub project: String, pub slot: u8, pub text: String, pub visual: bool, pub sound: bool, pub monitor: Option<usize> }

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
    // venció o lo saltaste: Claude lo pregunta ahora en la terminal y la sesión te espera ahí
    let pending = st.store.lock().unwrap().resolve(id, matches!(label, "none" | "skip"), now_ms());
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
        let timeout_s = if matches!(env.event, PulseEvent::Question { .. }) { pulse_protocol::QUESTION_TIMEOUT_S as i64 } else { pulse_protocol::clamp_ask_timeout(cfg.island.ask_timeout_s as u64) as i64 };   // `pulse ask` espera lo mismo
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

/// `KEY=VALOR` de `systemctl --user show-environment`: solo las variables de la pantalla.
#[cfg(target_os = "linux")]
fn display_vars(out: &str) -> Vec<(&str, &str)> {
    out.lines().filter_map(|l| l.split_once('=')).filter(|(k, v)| matches!(*k, "DISPLAY" | "XAUTHORITY" | "WAYLAND_DISPLAY") && !v.is_empty()).collect()
}

/// Linux con Wayland: la app corre por XWayland, porque Wayland no deja que una ventana se ponga arriba de las demás
/// ni elija su lugar (bordes, isla y Bolsillo lo necesitan). Va antes de iniciar GTK y antes de crear hilos.
/// La app hereda el entorno del hook: un tmux de una sesión anterior trae un `XAUTHORITY` que ya no existe (o ningún
/// `DISPLAY`), y GTK no arranca. Entonces se toman los de la sesión gráfica actual, que GNOME deja en systemd.
#[cfg(target_os = "linux")]
fn linux_env() {
    let xauth_ok = || std::env::var_os("XAUTHORITY").is_none_or(|p| std::path::Path::new(&p).exists());
    if std::env::var_os("DISPLAY").is_none() || !xauth_ok() {
        if let Ok(o) = std::process::Command::new("systemctl").args(["--user", "show-environment"]).output() {
            for (k, v) in display_vars(&String::from_utf8_lossy(&o.stdout)) { std::env::set_var(k, v); }
        }
    }
    // sin un X que acepte la conexión, mejor Wayland (sin bordes encima) que no aparecer
    if std::env::var_os("WAYLAND_DISPLAY").is_some() && std::env::var_os("GDK_BACKEND").is_none() && std::env::var_os("DISPLAY").is_some() && xauth_ok() { std::env::set_var("GDK_BACKEND", "x11"); }
}

pub fn run() {
    #[cfg(target_os = "linux")] linux_env();
    let endpoint = pulse_protocol::endpoint();
    // Otra instancia ya es dueña del canal (la instalada, o una de desarrollo olvidada): se avisa en vez de cerrarse en silencio.
    let first = match tauri::async_runtime::block_on(async { server::create_first(&endpoint) }) {
        Ok(s) => s,
        Err(e) => { eprintln!("Pulse ya está abierto: otra instancia escucha en {endpoint} ({e}). Ciérrala desde la bandeja › Salir o con `pkill -x pulse-app`."); return }
    };
    let builder = tauri::Builder::default();
    // En Linux los atajos globales los registra el escritorio (scripts/install.sh) y llegan por `pulse key`.
    #[cfg(windows)] let builder = builder.plugin(shortcuts::plugin());
    builder
        .manage(AppState { store: Mutex::new(Store::new(now_ms())), config: Mutex::new(config::load()), waiters: Mutex::new(HashMap::new()), next_id: AtomicU64::new(0), player: sound::Player::start() })
        .invoke_handler(tauri::generate_handler![commands::get_snapshot, commands::get_config, commands::set_config, commands::decide, commands::answer, commands::skip, commands::hold_question, commands::allow_all, commands::set_focus, commands::mute_session, commands::forget_sessions, commands::daily_summary, commands::log_today, commands::set_island_hitbox, commands::island_focus, commands::island_shown, commands::island_drag, commands::open_dashboard, commands::toggle_panel, commands::flyout_fit, commands::flyout_hide, commands::open_palette, commands::quit])
        .setup(move |app| {
            logbook::prune(logbook::KEEP_DAYS);   // antes de escuchar: nadie escribe en la bitácora mientras se recorta
            let h = app.handle().clone();
            tray::create(&h, windows::toggle_panel, windows::toggle_flyout)?;
            windows::create_all(&h)?;
            windows::create_flyout(&h)?;
            #[cfg(not(windows))] windows::create_panel(&h)?;
            let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
            let (ktx, mut krx) = tokio::sync::mpsc::unbounded_channel::<String>();
            let _ = server::KEYS.set(ktx);
            tauri::async_runtime::spawn(server::serve(first, endpoint, tx));
            let hk = h.clone();
            // `pulse key` es un golpe de tecla sin "soltar" (ver shortcuts::tap)
            tauri::async_runtime::spawn(async move { while let Some(k) = krx.recv().await { if let Some(k) = shortcuts::Key::parse(&k) { shortcuts::tap(&hk, k); } } });
            let h1 = h.clone();
            tauri::async_runtime::spawn(async move { while let Some(inc) = rx.recv().await { handle(&h1, inc); } });
            let h2 = h.clone();
            // Linux: nadie avisa si conectas un monitor o cambia el dock; se compara cada 2 s y se rehacen bordes e isla.
            #[cfg(not(windows))] let mut mons = windows::monitor_signature(&h);
            tauri::async_runtime::spawn(async move { for tick in 0u64.. {   // vencimiento de permisos, sesiones cerradas y autocierre
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
                if tick % 3 == 0 { let closed = closed_sessions(&h2); forget(&h2, &closed); }
                #[cfg(not(windows))] if tick % 2 == 1 { let now = windows::monitor_signature(&h2); if now != mons { mons = now; windows::monitors_changed(&h2); } }
                let expired = h2.state::<AppState>().store.lock().unwrap().expire(now_ms());
                for id in expired { resolve(&h2, id, Decision::None); }
                // `pulse ask` se cerró (Claude Code dejó de esperar al hook): se respondió en la terminal
                let gone: Vec<u64> = h2.state::<AppState>().waiters.lock().unwrap().iter().filter(|(_, tx)| tx.is_closed()).map(|(id, _)| *id).collect();
                for id in gone { resolve_as(&h2, id, Decision::None, "terminal"); }
                if tray::theme_changed() { broadcast(&h2); }   // barra de tareas clara u oscura: otro color de tinta
                // con Bolsillo o Mosaico abiertos no se cierra en tu cara: el autocierre cuenta desde que los cierras
                let ui_open = ["dashboard", "panel"].iter().any(|l| h2.get_webview_window(l).is_some_and(|w| w.is_visible().unwrap_or(false)));
                let st = h2.state::<AppState>();
                let exit = { let mut s = st.store.lock().unwrap(); if ui_open { s.touch(now_ms()); } s.should_exit(now_ms()) };
                if exit { chroma::shutdown(); h2.exit(0); }
            } });
            focus::start(&h);
            chroma::start(&h);
            Ok(())
        })
        .build(tauri::generate_context!()).expect("no se pudo iniciar Pulse")
        .run(|_app, e| if let tauri::RunEvent::ExitRequested { api, code: None, .. } = e { api.prevent_exit(); });
}

#[cfg(all(test, target_os = "linux"))]
mod tests {
    #[test] fn takes_only_the_display_vars_from_systemd() {
        let out = "HOME=/home/u\nDISPLAY=:0\nXAUTHORITY=/run/user/1000/.mutter-Xwaylandauth.ABC\nWAYLAND_DISPLAY=wayland-0\nGDK_BACKEND=\n";
        assert_eq!(super::display_vars(out), vec![("DISPLAY", ":0"), ("XAUTHORITY", "/run/user/1000/.mutter-Xwaylandauth.ABC"), ("WAYLAND_DISPLAY", "wayland-0")]);
    }
}
