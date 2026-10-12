use pulse_protocol::Envelope;
use serde::Serialize;
use std::{fs::{create_dir_all, OpenOptions}, io::Write, path::PathBuf};
pub fn path() -> Option<PathBuf> { dirs::home_dir().map(|h| h.join(".claude").join("pulse").join("log.jsonl")) }
pub fn append(env: &Envelope) {
    let Some(p) = path() else { return };
    if let Some(d) = p.parent() { let _ = create_dir_all(d); }
    if let (Ok(mut f), Ok(l)) = (OpenOptions::new().create(true).append(true).open(&p), serde_json::to_string(env)) { let _ = writeln!(f, "{l}"); }
}
/// Registra la resolución de un permiso con el mismo formato (kind "resolve").
pub fn append_resolve(session_id: &str, project: &str, id: u64, allow: Option<bool>, at_ms: i64) {
    let Some(p) = path() else { return };
    let line = serde_json::json!({ "v": 1, "session_id": session_id, "project": project, "cwd": "", "at_ms": at_ms, "event": { "kind": "resolve", "id": id, "decision": match allow { Some(true) => "allow", Some(false) => "deny", None => "none" } } });
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&p) { let _ = writeln!(f, "{line}"); }
}

/// Un evento de hoy para la Bitácora de Mosaico.
#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct LogItem { pub at_ms: i64, pub session_id: String, pub project: String, pub kind: String, pub text: String }

/// Qué pasó, en palabras. `None` si la línea no es un evento conocido.
fn describe(e: &serde_json::Value) -> Option<(String, String)> {
    let kind = e["kind"].as_str()?;
    let s = |k: &str| e[k].as_str().unwrap_or("").to_string();
    let text = match kind {
        "start" => format!("Prompt: {}", s("prompt")),
        "tool" => { let t = s("target"); if t.is_empty() { s("tool") } else { format!("{} {t}", s("tool")) } }
        "permission" => format!("Permiso: {}", s("command")),
        "question" => format!("Pregunta: {}", e["questions"][0]["question"].as_str().unwrap_or("")),
        "resolve" => match e["decision"].as_str() { Some("allow") => "Permitido".into(), Some("deny") => "Denegado".into(), _ => "Sin respuesta en Pulse".into() },
        "done" => "Terminó".into(),
        "idle" => "Te espera".into(),
        "plan" => "Plan listo para revisar".into(),
        "error" => format!("Falló: {}", s("message")),
        "subagent-start" => format!("Subagente: {}", s("agent")),
        "subagent-stop" => format!("Subagente terminó: {}", s("agent")),
        "session-start" => "Sesión abierta".into(),
        "session-end" => "Sesión cerrada".into(),
        _ => return None,
    };
    Some((kind.to_string(), text))
}

/// Eventos entre `from_ms` y `to_ms`, del más nuevo al más viejo, hasta `limit`. Las líneas que no se entienden se saltan.
pub fn parse_events(lines: impl Iterator<Item = String>, from_ms: i64, to_ms: i64, limit: usize) -> Vec<LogItem> {
    let mut out: Vec<LogItem> = lines.filter_map(|l| {
        let v: serde_json::Value = serde_json::from_str(&l).ok()?;
        let at = v["at_ms"].as_i64()?;
        if at < from_ms || at > to_ms { return None; }
        let (kind, text) = describe(&v["event"])?;
        Some(LogItem { at_ms: at, session_id: v["session_id"].as_str().unwrap_or("").into(), project: v["project"].as_str().unwrap_or("sesión").into(), kind, text })
    }).collect();
    out.sort_by(|a, b| b.at_ms.cmp(&a.at_ms));
    out.truncate(limit);
    out
}

/// Eventos de hoy (desde la medianoche local).
pub fn today(limit: usize) -> Vec<LogItem> {
    let now = crate::now_ms();
    let lines = path().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    parse_events(lines.lines().map(str::to_string), crate::daylog::local_midnight_ms(now), now, limit)
}

#[cfg(test)]
mod tests { use super::*;
    fn line(at: i64, ev: &str) -> String { format!(r#"{{"v":1,"session_id":"s","project":"pulse","cwd":"","at_ms":{at},"event":{ev}}}"#) }
    #[test] fn events_in_words_newest_first() {
        let lines = vec![
            line(10, r#"{"kind":"start","prompt":"arregla el test"}"#),
            line(20, r#"{"kind":"tool","tool":"Bash","target":"npm test"}"#),
            line(30, r#"{"kind":"permission","tool":"Bash","command":"git push"}"#),
            line(40, r#"{"kind":"resolve","id":1,"decision":"deny"}"#),
            line(50, r#"{"kind":"error","message":"exit 1"}"#),
            "basura".to_string(),
            line(60, r#"{"kind":"done"}"#),
        ];
        let v = parse_events(lines.into_iter(), 0, 100, 10);
        let t: Vec<(&str, &str)> = v.iter().map(|x| (x.kind.as_str(), x.text.as_str())).collect();
        assert_eq!(t, vec![("done", "Terminó"), ("error", "Falló: exit 1"), ("resolve", "Denegado"), ("permission", "Permiso: git push"), ("tool", "Bash npm test"), ("start", "Prompt: arregla el test")]);
        assert_eq!((v[0].project.as_str(), v[0].session_id.as_str()), ("pulse", "s"));
    }
    #[test] fn only_the_window_and_up_to_the_limit() {
        let v = parse_events((0..10).map(|i| line(i * 10, r#"{"kind":"done"}"#)), 20, 70, 3);
        assert_eq!(v.iter().map(|x| x.at_ms).collect::<Vec<_>>(), vec![70, 60, 50]);
    }
    #[test] fn questions_show_the_first_question_and_unknown_kinds_are_skipped() {
        let v = parse_events(vec![line(1, r#"{"kind":"question","questions":[{"question":"¿Qué base?"}]}"#), line(2, r#"{"kind":"rara"}"#)].into_iter(), 0, 5, 5);
        assert_eq!(v.len(), 1);
        assert_eq!(v[0].text, "Pregunta: ¿Qué base?");
    }
}
