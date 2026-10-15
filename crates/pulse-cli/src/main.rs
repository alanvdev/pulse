mod pipe;
use std::{io::{Read, Write}, thread, time::Duration};
use pulse_protocol::*;

/// Sin `println!`: si Claude Code ya cerró la salida, `println!` entra en pánico y el proceso sale con 101.
fn out(s: &str) { let _ = writeln!(std::io::stdout().lock(), "{s}"); }

fn now_ms() -> i64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0) }

fn read_input() -> Result<HookInput, String> {
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

/// El evento del hook, con el Claude Code que lo mandó.
fn stamped(i: &HookInput, ask: bool) -> Option<Envelope> { envelope_from_hook(i, now_ms(), ask).map(|e| Envelope { owner: owner::find(), ..e }) }

fn read_envelope(ask: bool) -> Result<Option<Envelope>, String> { Ok(stamped(&read_input()?, ask)) }

fn app_path() -> Option<std::path::PathBuf> { std::env::current_exe().ok().map(|p| p.with_file_name(if cfg!(windows) { "pulse-app.exe" } else { "pulse-app" })) }

/// Tope duro: pase lo que pase (un `connect` que no vuelve, una app trabada), el proceso sale con 0 a los `ms`.
fn deadline(ms: u64) { thread::spawn(move || { thread::sleep(Duration::from_millis(ms)); std::process::exit(0); }); }

/// Lo que `pulse ask` espera un permiso: el "Tiempo para responder" de config.json más un margen, porque la app
/// contesta "nada" justo al vencer su plazo. El hook `PermissionRequest` se registra con 130 s, más que el máximo.
fn ask_wait() -> Duration {
    let cfg = data_dir().and_then(|d| std::fs::read_to_string(d.join("config.json")).ok()).unwrap_or_default();
    Duration::from_secs(ask_timeout_from_config(&cfg) + 2)
}

fn emit() -> Result<(), String> {
    let Some(env) = read_envelope(false)? else { return Ok(()) };
    // `SessionEnd` con Pulse cerrado no lo abre: no hay nada que mostrar
    let app = if wakes_app(&env.event) { app_path() } else { None };
    pipe::send_line(&endpoint(), &(serde_json::to_string(&env).map_err(|e| e.to_string())? + "\n"), app.as_deref())
}

fn ask() -> Result<(), String> {
    let input = read_input()?;
    // Pregunta de Claude (PreToolUse de AskUserQuestion): la isla puede contestarla.
    if let Some(event) = question_hook(&input) {
        let PulseEvent::Question { questions } = &event else { return Ok(()) };
        let env = Envelope { v: PROTOCOL_VERSION, session_id: input.session_id.clone(), project: project_name(&input.cwd), cwd: input.cwd.clone(), at_ms: now_ms(), ask: true, owner: owner::find(), event: event.clone() };
        let line = serde_json::to_string(&env).map_err(|e| e.to_string())? + "\n";
        let reply = pipe::request(&endpoint(), &line, app_path().as_deref(), Duration::from_secs(QUESTION_MAX_WAIT_S)).and_then(|r| serde_json::from_str::<Reply>(&r).ok());
        if let Some(Reply { decision: Decision::Allow, answers: Some(a) }) = reply { out(&answer_output(questions, &a)); }
        return Ok(());
    }
    let Some(mut env) = stamped(&input, true) else { return Ok(()) };
    // Preguntas y planes que llegan como permiso: solo avisar a la isla; se responden en la terminal.
    if matches!(env.event, PulseEvent::Question { .. } | PulseEvent::Plan) {
        env.ask = false;
        return pipe::send_line(&endpoint(), &(serde_json::to_string(&env).map_err(|e| e.to_string())? + "\n"), app_path().as_deref());
    }
    let line = serde_json::to_string(&env).map_err(|e| e.to_string())? + "\n";
    let wait = ask_wait();
    deadline(wait.as_millis() as u64 + 4_000);   // 3 s para que arranque la app y un poco más; sigue por debajo de los 130 s del hook
    let reply = pipe::request(&endpoint(), &line, app_path().as_deref(), wait);
    let decision = reply.and_then(|r| serde_json::from_str::<Reply>(&r).ok()).map(|r| r.decision).unwrap_or(Decision::None);
    if let Some(o) = permission_output(decision) { out(&o); }
    Ok(())
}

/// Atajo de teclado (`allow`, `deny`, `palette`): en Linux lo lanza el escritorio. Sin la app abierta no hace nada.
fn key(action: Option<String>) -> Result<(), String> {
    let key = action.filter(|a| matches!(a.as_str(), "allow" | "deny" | "palette")).ok_or("uso: pulse key allow | deny | palette")?;
    pipe::send_line(&endpoint(), &(serde_json::to_string(&KeyPress { key }).map_err(|e| e.to_string())? + "\n"), None)
}

fn main() {
    match std::env::args().nth(1).as_deref() {
        Some("emit" | "key") => deadline(3_500),   // la regla: `pulse emit` tarda menos de 4 s
        Some("ask") => deadline((QUESTION_MAX_WAIT_S + 4) * 1000),   // el hook de preguntas se registra con 310 s
        _ => {}
    }
    let r = match std::env::args().nth(1).as_deref() {
        Some("emit") => emit(),
        Some("ask") => ask(),
        Some("key") => key(std::env::args().nth(2)),
        Some("--version") => { out(&format!("pulse {}", env!("CARGO_PKG_VERSION"))); Ok(()) }
        _ => Err("uso: pulse emit | pulse ask  (leen el JSON del hook por stdin) | pulse key <acción>".into()),
    };
    if let Err(e) = r { let _ = writeln!(std::io::stderr().lock(), "pulse: {e}"); }
    std::process::exit(0);
}
