use pulse_protocol::Envelope;
use serde::Serialize;
use std::{fs::{create_dir_all, File, OpenOptions}, io::{BufRead, BufReader, Write}, path::{Path, PathBuf}};
pub fn path() -> Option<PathBuf> { dirs::home_dir().map(|h| h.join(".claude").join("pulse").join("log.jsonl")) }
/// Una línea en una sola escritura en modo "append": dos registros a la vez (un evento y una respuesta) no se mezclan.
fn write_line(p: &Path, mut line: String) {
    line.push('\n');
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(p) { let _ = f.write_all(line.as_bytes()); }
}
pub fn append(env: &Envelope) {
    let Some(p) = path() else { return };
    if let Some(d) = p.parent() { let _ = create_dir_all(d); }
    if let Ok(l) = serde_json::to_string(env) { write_line(&p, l); }
}
/// Registra la resolución de un permiso con el mismo formato (kind "resolve").
pub fn append_resolve(session_id: &str, project: &str, id: u64, allow: Option<bool>, at_ms: i64) {
    let Some(p) = path() else { return };
    let line = serde_json::json!({ "v": 1, "session_id": session_id, "project": project, "cwd": "", "at_ms": at_ms, "event": { "kind": "resolve", "id": id, "decision": match allow { Some(true) => "allow", Some(false) => "deny", None => "none" } } });
    write_line(&p, line.to_string());
}

/// Días que guarda la bitácora. Hoy y la Bitácora de Mosaico solo leen el día; lo demás es margen.
pub const KEEP_DAYS: i64 = 7;

/// Quita de log.jsonl lo anterior a `KEEP_DAYS` días (cada hook agrega una línea y no tenía fin). Va al arrancar,
/// antes de escuchar hooks. Se escribe aparte y se renombra encima; las líneas sin fecha legible se quedan.
pub fn prune(keep_days: i64) {
    if let Some(p) = path() { prune_at(&p, crate::daylog::local_midnight_ms(crate::now_ms()) - keep_days * 86_400_000); }
}
fn prune_at(p: &Path, cutoff_ms: i64) {
    let Ok(f) = File::open(p) else { return };
    // por bytes: una línea dañada (UTF-8 roto por un disco lleno) se conserva tal cual; un error de lectura no reescribe nada
    let (mut kept, mut dropped) = (Vec::<u8>::new(), 0usize);
    for l in BufReader::new(f).split(b'\n') {
        let Ok(l) = l else { return };
        if quick_at(&String::from_utf8_lossy(&l)).is_some_and(|at| at < cutoff_ms) { dropped += 1; } else { kept.extend_from_slice(&l); kept.push(b'\n'); }
    }
    if dropped == 0 { return; }
    let tmp = p.with_extension("jsonl.tmp");
    if std::fs::write(&tmp, kept).is_ok() { let _ = std::fs::rename(&tmp, p); }
}

/// Las líneas de log.jsonl una por una, sin cargar todo el archivo.
pub(crate) fn lines() -> impl Iterator<Item = String> {
    // una línea con UTF-8 roto no corta la lectura del resto del día
    path().and_then(|p| File::open(p).ok()).into_iter().flat_map(|f| BufReader::new(f).split(b'\n').map_while(Result::ok).map(|l| String::from_utf8_lossy(&l).into_owned()))
}

/// Tope de eventos de hoy para la Bitácora (un día normal tiene unos pocos miles de líneas).
pub const TODAY_LIMIT: usize = 5000;

/// `at_ms` de una línea sin interpretar todo el JSON: la mayoría de las líneas son de otros días.
pub(crate) fn quick_at(l: &str) -> Option<i64> {
    let i = l.find("\"at_ms\":")? + 8;
    let rest = &l[i..];
    rest[..rest.find(|c: char| !(c.is_ascii_digit() || c == '-')).unwrap_or(rest.len())].parse().ok()
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
        if quick_at(&l).is_some_and(|at| at < from_ms || at > to_ms) { return None; }
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
    parse_events(lines(), crate::daylog::local_midnight_ms(now), now, limit)
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
    #[test] fn prune_keeps_recent_and_unreadable_lines() {
        let p = std::env::temp_dir().join(format!("pulse-test-log-{}.jsonl", std::process::id()));
        std::fs::write(&p, [line(10, r#"{"kind":"done"}"#), "basura".into(), line(500, r#"{"kind":"done"}"#)].join("\n") + "\n").unwrap();
        prune_at(&p, 100);
        assert_eq!(std::fs::read_to_string(&p).unwrap(), format!("basura\n{}\n", line(500, r#"{"kind":"done"}"#)));
        prune_at(&p, 100);   // nada que quitar: no reescribe
        assert!(!p.with_extension("jsonl.tmp").exists());
        let _ = std::fs::remove_file(&p);
    }
    #[test] fn prune_never_loses_what_follows_a_broken_line() {
        let p = std::env::temp_dir().join(format!("pulse-test-log-utf8-{}.jsonl", std::process::id()));
        let mut bytes = line(10, r#"{"kind":"done"}"#).into_bytes(); bytes.push(b'\n');
        bytes.extend_from_slice(b"{\"at_ms\":200,\"x\":\"\xC3\"}\n");   // UTF-8 roto a media línea
        bytes.extend_from_slice((line(500, r#"{"kind":"done"}"#) + "\n").as_bytes());
        std::fs::write(&p, &bytes).unwrap();
        prune_at(&p, 100);
        let out = std::fs::read(&p).unwrap();
        assert!(out.ends_with((line(500, r#"{"kind":"done"}"#) + "\n").as_bytes()), "lo de hoy sigue ahí");
        assert!(out.windows(2).any(|w| w == b"\xC3\""), "la línea dañada se conserva tal cual");
        assert!(!String::from_utf8_lossy(&out).contains("\"at_ms\":10,"), "lo viejo se quitó");
        let _ = std::fs::remove_file(&p);
    }
    #[test] fn quick_date_check_reads_at_ms() {
        assert_eq!(quick_at(&line(1234, r#"{"kind":"done"}"#)), Some(1234));
        assert_eq!(quick_at("basura"), None);
    }
}
