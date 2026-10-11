use pulse_protocol::Envelope;
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
