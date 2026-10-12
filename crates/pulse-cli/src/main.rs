mod pipe;
use std::{io::Read, time::Duration};
use pulse_protocol::*;

fn now_ms() -> i64 { std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0) }

fn read_input() -> Result<HookInput, String> {
    let mut raw = String::new();
    std::io::stdin().read_to_string(&mut raw).map_err(|e| e.to_string())?;
    serde_json::from_str(&raw).map_err(|e| e.to_string())
}

/// El evento del hook, con el Claude Code que lo mandó.
fn stamped(i: &HookInput, ask: bool) -> Option<Envelope> { envelope_from_hook(i, now_ms(), ask).map(|e| Envelope { owner: owner::find(), ..e }) }

fn read_envelope(ask: bool) -> Result<Option<Envelope>, String> { Ok(stamped(&read_input()?, ask)) }

fn app_path() -> Option<std::path::PathBuf> { std::env::current_exe().ok().map(|p| p.with_file_name("pulse-app.exe")) }

fn emit() -> Result<(), String> {
    let Some(env) = read_envelope(false)? else { return Ok(()) };
    pipe::send_line(PIPE_NAME, &(serde_json::to_string(&env).map_err(|e| e.to_string())? + "\n"), app_path().as_deref())
}

fn ask() -> Result<(), String> {
    let input = read_input()?;
    // Pregunta de Claude (PreToolUse de AskUserQuestion): la isla puede contestarla.
    if let Some(event) = question_hook(&input) {
        let PulseEvent::Question { questions } = &event else { return Ok(()) };
        let env = Envelope { v: PROTOCOL_VERSION, session_id: input.session_id.clone(), project: project_name(&input.cwd), cwd: input.cwd.clone(), at_ms: now_ms(), ask: true, owner: owner::find(), event: event.clone() };
        let line = serde_json::to_string(&env).map_err(|e| e.to_string())? + "\n";
        let reply = pipe::request(PIPE_NAME, &line, app_path().as_deref(), Duration::from_secs(QUESTION_MAX_WAIT_S)).and_then(|r| serde_json::from_str::<Reply>(&r).ok());
        if let Some(Reply { decision: Decision::Allow, answers: Some(a) }) = reply { println!("{}", answer_output(questions, &a)); }
        return Ok(());
    }
    let Some(mut env) = stamped(&input, true) else { return Ok(()) };
    // Preguntas y planes que llegan como permiso: solo avisar a la isla; se responden en la terminal.
    if matches!(env.event, PulseEvent::Question { .. } | PulseEvent::Plan) {
        env.ask = false;
        return pipe::send_line(PIPE_NAME, &(serde_json::to_string(&env).map_err(|e| e.to_string())? + "\n"), app_path().as_deref());
    }
    let line = serde_json::to_string(&env).map_err(|e| e.to_string())? + "\n";
    let reply = pipe::request(PIPE_NAME, &line, app_path().as_deref(), Duration::from_secs(ASK_TIMEOUT_S + 2));
    let decision = reply.and_then(|r| serde_json::from_str::<Reply>(&r).ok()).map(|r| r.decision).unwrap_or(Decision::None);
    if let Some(out) = permission_output(decision) { println!("{out}"); }
    Ok(())
}

fn main() {
    let r = match std::env::args().nth(1).as_deref() {
        Some("emit") => emit(),
        Some("ask") => ask(),
        Some("--version") => { println!("pulse {}", env!("CARGO_PKG_VERSION")); Ok(()) }
        _ => Err("uso: pulse emit | pulse ask  (leen el JSON del hook por stdin)".into()),
    };
    if let Err(e) = r { eprintln!("pulse: {e}"); }
    std::process::exit(0);
}
