use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub mod owner;

pub const PIPE_NAME: &str = r"\\.\pipe\pulse";

/// Dónde escucha la app: el named pipe en Windows; en Linux un socket Unix en `$XDG_RUNTIME_DIR`.
#[cfg(windows)]
pub fn endpoint() -> String { PIPE_NAME.to_string() }

/// Sin `$XDG_RUNTIME_DIR` (SSH, `sudo -u`, un cron) se busca la misma carpeta por el uid, para que el hook y la app
/// coincidan. Nunca una carpeta compartida como `/tmp`: quien escucha en el socket puede aprobar permisos, y otro usuario
/// podría crearlo primero. Último recurso: `~/.claude/pulse/`, que es tuya.
#[cfg(unix)]
pub fn endpoint() -> String {
    use std::os::unix::fs::MetadataExt;
    if let Some(d) = std::env::var_os("XDG_RUNTIME_DIR").filter(|d| !d.is_empty()) { return format!("{}/pulse.sock", d.to_string_lossy()); }
    let uid = std::fs::metadata("/proc/self").map(|m| m.uid()).ok();
    let run = uid.map(|u| format!("/run/user/{u}")).filter(|d| std::fs::metadata(d).is_ok_and(|m| m.is_dir() && Some(m.uid()) == uid && m.mode() & 0o077 == 0));
    if let Some(d) = run { return format!("{d}/pulse.sock"); }
    match data_dir() {
        Some(d) => { let _ = std::fs::create_dir_all(&d); d.join("pulse.sock").to_string_lossy().into_owned() }
        None => std::env::temp_dir().join(format!("pulse-{}.sock", uid.unwrap_or(0))).to_string_lossy().into_owned(),
    }
}

/// Un atajo de teclado que llega por `pulse key <acción>`: en Linux los atajos globales los registra el escritorio, no la app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyPress { pub key: String }

pub const PROTOCOL_VERSION: u32 = 1;
/// "Tiempo para responder" de la isla: de fábrica, y el rango del control en Ajustes.
pub const ASK_TIMEOUT_S: u64 = 30;
pub const ASK_TIMEOUT_MIN_S: u64 = 10;
pub const ASK_TIMEOUT_MAX_S: u64 = 120;
/// Cuánto espera la isla una respuesta a una pregunta de Claude antes de dejarla en la terminal.
pub const QUESTION_TIMEOUT_S: u64 = 60;
/// Lo más que `pulse ask` espera una pregunta, contando pausas mientras la contestas. El hook se registra con un poco más.
pub const QUESTION_MAX_WAIT_S: u64 = 300;
const MAX_TEXT: usize = 120;

/// Un "Tiempo para responder" dentro del rango; app y `pulse ask` lo acotan igual para esperar lo mismo.
pub fn clamp_ask_timeout(s: u64) -> u64 { s.clamp(ASK_TIMEOUT_MIN_S, ASK_TIMEOUT_MAX_S) }

/// El "Tiempo para responder" guardado en un `config.json` de Pulse (`island.ask_timeout_s`), acotado. Sin él: 30 s.
pub fn ask_timeout_from_config(json: &str) -> u64 {
    clamp_ask_timeout(serde_json::from_str::<Value>(json).ok().and_then(|v| v["island"]["ask_timeout_s"].as_u64()).unwrap_or(ASK_TIMEOUT_S))
}

/// Carpeta de datos de Pulse (`~/.claude/pulse`): config, bitácora y el registro de la app.
pub fn data_dir() -> Option<std::path::PathBuf> {
    std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }).filter(|h| !h.is_empty()).map(|h| std::path::PathBuf::from(h).join(".claude").join("pulse"))
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct HookInput {
    pub session_id: String,
    #[serde(default)] pub cwd: String,
    pub hook_event_name: String,
    #[serde(default)] pub prompt: Option<String>,
    #[serde(default)] pub tool_name: Option<String>,
    #[serde(default)] pub tool_input: Option<Value>,
    #[serde(default)] pub notification_type: Option<String>,
    #[serde(default)] pub error: Option<String>,
    #[serde(default)] pub agent_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "kebab-case")]
pub enum PulseEvent {
    SessionStart, SessionEnd,
    Start { prompt: String },
    Tool { tool: String, target: String },
    /// `dangerous` se decide con el comando completo, antes de recortarlo para mostrarlo.
    Permission { tool: String, command: String, #[serde(default)] description: String, #[serde(default)] dangerous: bool },
    Idle, Done,
    Error { message: String },
    SubagentStart { agent: String },
    SubagentStop { agent: String },
    /// Claude te pregunta algo (`AskUserQuestion`): `questions` es el arreglo original de la herramienta.
    Question { questions: Value },
    /// Claude terminó un plan y pide aprobarlo (`ExitPlanMode`).
    Plan,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    pub v: u32,
    pub session_id: String,
    pub project: String,
    pub cwd: String,
    pub at_ms: i64,
    #[serde(default)] pub ask: bool,
    /// El Claude Code que mandó el evento, para quitar la sesión si se cierra sin avisar.
    #[serde(default)] pub owner: owner::Owner,
    pub event: PulseEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision { Allow, Deny, None }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Reply { pub decision: Decision, #[serde(default, skip_serializing_if = "Option::is_none")] pub answers: Option<Value> }

pub fn project_name(cwd: &str) -> String {
    cwd.trim_end_matches(['\\', '/']).rsplit(['\\', '/']).next().filter(|s| !s.is_empty()).unwrap_or("sesión").to_string()
}

fn truncate(s: &str) -> String { s.chars().take(MAX_TEXT).collect() }

fn tool_target(tool: &str, input: Option<&Value>) -> String { truncate(&tool_target_full(tool, input)) }

fn tool_target_full(tool: &str, input: Option<&Value>) -> String {
    let f = |k: &str| input.and_then(|v| v.get(k)).and_then(Value::as_str).map(str::to_string);
    match tool {
        "Bash" | "PowerShell" => f("command"),
        "Read" | "Edit" | "Write" | "MultiEdit" | "NotebookEdit" => f("file_path"),
        "Grep" | "Glob" => f("pattern"),
        "Agent" | "Task" => f("description"),
        "WebFetch" => f("url"),
        "WebSearch" => f("query"),
        _ => None,
    }.unwrap_or_default()
}

/// Qué quiere hacer un permiso, en una línea. Shell: el comando tal cual. Otras herramientas: su nombre y el objetivo
/// (o un resumen de sus argumentos si Pulse no la conoce: Artifact, herramientas MCP, etc.).
fn permission_label_full(tool: &str, input: Option<&Value>) -> String {
    if matches!(tool, "Bash" | "PowerShell") { return tool_target_full(tool, input); }
    let target = tool_target_full(tool, input);
    let detail = if !target.is_empty() { target } else {
        input.and_then(Value::as_object).map(|m| m.iter().filter_map(|(k, v)| match v {
            Value::String(s) => Some(format!("{k}: {s}")), Value::Number(n) => Some(format!("{k}: {n}")), Value::Bool(b) => Some(format!("{k}: {b}")), _ => None,
        }).take(3).collect::<Vec<_>>().join(" · ")).unwrap_or_default()
    };
    if detail.is_empty() { tool.to_string() } else { format!("{tool}: {detail}") }
}

/// El permiso para mostrar (120 caracteres) y si es delicado, mirando el texto completo. Si lo delicado quedó
/// fuera del recorte (`npm ci && … && rm -rf dist`), se muestra el final, que es donde está.
fn permission(tool: &str, input: Option<&Value>) -> (String, bool) {
    let full = permission_label_full(tool, input);
    let dangerous = danger_applies(tool) && is_dangerous(&full);
    let short = truncate(&full);
    if !dangerous || is_dangerous(&short) { return (short, dangerous); }
    let n = full.chars().count();
    (std::iter::once('…').chain(full.chars().skip(n.saturating_sub(MAX_TEXT - 1))).collect(), true)
}

pub fn map_hook(i: &HookInput) -> Option<PulseEvent> {
    let tool = i.tool_name.clone().unwrap_or_default();
    Some(match i.hook_event_name.as_str() {
        "SessionStart" => PulseEvent::SessionStart,
        "SessionEnd" => PulseEvent::SessionEnd,
        "UserPromptSubmit" => PulseEvent::Start { prompt: truncate(i.prompt.as_deref().unwrap_or("")) },
        "PreToolUse" => PulseEvent::Tool { target: tool_target(&tool, i.tool_input.as_ref()), tool },
        "PermissionRequest" if tool == "AskUserQuestion" => PulseEvent::Question { questions: questions(i) },
        "PermissionRequest" if tool == "ExitPlanMode" => PulseEvent::Plan,
        "PermissionRequest" => { let (command, dangerous) = permission(&tool, i.tool_input.as_ref()); PulseEvent::Permission { command, description: truncate(i.tool_input.as_ref().and_then(|v| v.get("description")).and_then(Value::as_str).unwrap_or("")), tool, dangerous } }
        "Notification" if i.notification_type.as_deref() == Some("idle_prompt") => PulseEvent::Idle,
        "Stop" => PulseEvent::Done,
        "PostToolUseFailure" => PulseEvent::Error { message: truncate(i.error.as_deref().unwrap_or("falló una herramienta")) },
        "SubagentStart" => PulseEvent::SubagentStart { agent: i.agent_type.clone().unwrap_or_default() },
        "SubagentStop" => PulseEvent::SubagentStop { agent: i.agent_type.clone().unwrap_or_default() },
        _ => return None,
    })
}

fn questions(i: &HookInput) -> Value { i.tool_input.as_ref().and_then(|v| v.get("questions")).cloned().unwrap_or(Value::Array(vec![])) }

/// `PreToolUse` de `AskUserQuestion`: la pregunta que la isla puede contestar.
pub fn question_hook(i: &HookInput) -> Option<PulseEvent> {
    (i.hook_event_name == "PreToolUse" && i.tool_name.as_deref() == Some("AskUserQuestion")).then(|| PulseEvent::Question { questions: questions(i) })
}

pub fn envelope_from_hook(i: &HookInput, at_ms: i64, ask: bool) -> Option<Envelope> {
    Some(Envelope { v: PROTOCOL_VERSION, session_id: i.session_id.clone(), project: project_name(&i.cwd), cwd: i.cwd.clone(), at_ms, ask, owner: owner::Owner::default(), event: map_hook(i)? })
}

/// Si el evento lanza la app cuando no está abierta. `SessionEnd` no: cerrar Claude Code no debe hacer aparecer Pulse.
pub fn wakes_app(e: &PulseEvent) -> bool { !matches!(e, PulseEvent::SessionEnd) }

/// Si la revisión de "delicado" aplica a la herramienta. Las que leen o editan archivos y las de la web no: su texto es
/// una ruta o una URL, y `db/migrate/…` o `…/how-to-migrate` no son comandos. Bash, PowerShell y las de MCP (un SQL con
/// `drop table`) sí.
pub fn danger_applies(tool: &str) -> bool {
    !matches!(tool, "Edit" | "MultiEdit" | "Write" | "NotebookEdit" | "Read" | "Glob" | "Grep" | "WebFetch" | "WebSearch")
}

/// Comandos que se aprueban manteniendo presionado (Windows y Linux). Va sobre el comando completo.
pub fn is_dangerous(cmd: &str) -> bool {
    let c = cmd.to_lowercase();
    let sub = ["push --force", "push -f", "rm -rf", "rm -fr", "rm -r ", "rm --recursive", "migrate", "reset --hard", "remove-item -recurse", "drop table", "drop database",
        "mkfs", "of=/dev/", "> /dev/sd", "> /dev/nvme", "git clean -f", " -delete"];
    if sub.iter().any(|p| c.contains(p)) { return true; }
    // `format C:` de Windows; `--format json` o `clang-format` no cuentan
    let t: Vec<&str> = c.split_whitespace().collect();
    t.windows(2).any(|w| w[0] == "format" && { let b = w[1].as_bytes(); b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' })
}

/// JSON que `pulse ask` imprime para Claude Code. `None` = no decidir (Claude pregunta en la terminal).
pub fn permission_output(d: Decision) -> Option<String> {
    let behavior = match d { Decision::Allow => "allow", Decision::Deny => "deny", Decision::None => return None };
    let mut decision = json!({ "behavior": behavior });
    if d == Decision::Deny { decision["message"] = json!("Denegado desde Pulse"); }
    Some(json!({ "hookSpecificOutput": { "hookEventName": "PermissionRequest", "decision": decision } }).to_string())
}

/// JSON de `PreToolUse` que contesta un `AskUserQuestion`: repite las preguntas y agrega `answers` (pregunta → opción).
pub fn answer_output(questions: &Value, answers: &Value) -> String {
    json!({ "hookSpecificOutput": { "hookEventName": "PreToolUse", "permissionDecision": "allow", "updatedInput": { "questions": questions, "answers": answers } } }).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    fn hook(j: &str) -> HookInput { serde_json::from_str(j).unwrap() }

    #[test] fn project_name_is_last_segment() {
        assert_eq!(project_name(r"D:\Documents\dev\tienda\tienda-app"), "tienda-app");
        assert_eq!(project_name("/x/proppie-api/"), "proppie-api");
        assert_eq!(project_name(""), "sesión");
    }
    #[test] fn maps_every_hook_we_use() {
        let c = |j| map_hook(&hook(j));
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"UserPromptSubmit","prompt":"hola"}"#), Some(PulseEvent::Start { prompt: "hola".into() }));
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"npm run lint"}}"#), Some(PulseEvent::Tool { tool: "Bash".into(), target: "npm run lint".into() }));
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"PreToolUse","tool_name":"Edit","tool_input":{"file_path":"src/H.tsx"}}"#), Some(PulseEvent::Tool { tool: "Edit".into(), target: "src/H.tsx".into() }));
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"npx prisma migrate dev"}}"#), Some(PulseEvent::Permission { tool: "Bash".into(), command: "npx prisma migrate dev".into(), description: String::new(), dangerous: true }));
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"rm -rf node_modules","description":"Remove node_modules directory"}}"#), Some(PulseEvent::Permission { tool: "Bash".into(), command: "rm -rf node_modules".into(), description: "Remove node_modules directory".into(), dangerous: true }));
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"Notification","notification_type":"idle_prompt"}"#), Some(PulseEvent::Idle));
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"Notification","notification_type":"permission_prompt"}"#), None);
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"Stop"}"#), Some(PulseEvent::Done));
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"PostToolUseFailure","error":"exit 1"}"#), Some(PulseEvent::Error { message: "exit 1".into() }));
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"SubagentStart","agent_type":"Explore"}"#), Some(PulseEvent::SubagentStart { agent: "Explore".into() }));
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"SubagentStop","agent_type":"Explore"}"#), Some(PulseEvent::SubagentStop { agent: "Explore".into() }));
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"SessionStart"}"#), Some(PulseEvent::SessionStart));
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"PreCompact"}"#), None);
    }
    #[test] fn text_is_truncated_to_120_chars() {
        let h = hook(&format!(r#"{{"session_id":"a","hook_event_name":"UserPromptSubmit","prompt":"{}"}}"#, "x".repeat(300)));
        match map_hook(&h) { Some(PulseEvent::Start { prompt }) => assert_eq!(prompt.chars().count(), 120), o => panic!("{o:?}") }
    }
    #[test] fn envelope_is_one_json_line_and_round_trips() {
        let env = envelope_from_hook(&hook(r#"{"session_id":"s1","cwd":"C:\\p\\mcrefacciones","hook_event_name":"Stop"}"#), 7, false).unwrap();
        let line = serde_json::to_string(&env).unwrap();
        assert!(!line.contains('\n') && line.contains(r#""kind":"done""#));
        assert_eq!(serde_json::from_str::<Envelope>(&line).unwrap(), env);
        assert_eq!(env.project, "mcrefacciones");
    }
    #[test] fn dangerous_commands() {
        for c in ["git push --force origin main", "git push -f", "rm -rf tmp", "npx prisma migrate dev", "git reset --hard HEAD~1", "Remove-Item -Recurse x", "DROP TABLE users"] { assert!(is_dangerous(c), "{c}"); }
        for c in ["npm run build", "git push origin main", "ls -la"] { assert!(!is_dangerous(c), "{c}"); }
        // Linux
        for c in ["rm -fr build", "rm -R x", "rm --recursive x", "sudo mkfs.ext4 /dev/sdb1", "dd if=a.img of=/dev/sda", "git clean -fdx", "find . -name '*.o' -delete", "format c:"] { assert!(is_dangerous(c), "{c}"); }
        for c in ["docker ps --format json", "gh pr list --format json", "clang-format -i x.c", "rm build.log"] { assert!(!is_dangerous(c), "{c}"); }
    }
    #[test] fn danger_is_decided_on_the_whole_command() {
        let long = format!("cd /home/u/{} && npm ci && npm run build && rm -rf dist", "a".repeat(150));
        let h = hook(&format!(r#"{{"session_id":"a","hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{{"command":"{long}"}}}}"#));
        match map_hook(&h) {
            Some(PulseEvent::Permission { command, dangerous, .. }) => {
                assert!(dangerous);
                assert_eq!(command.chars().count(), 120);
                assert!(command.starts_with('…') && command.ends_with("rm -rf dist"), "{command}");   // se ve la parte delicada
            }
            o => panic!("{o:?}"),
        }
        // un sobre viejo sin `dangerous` se sigue leyendo
        let e: PulseEvent = serde_json::from_str(r#"{"kind":"permission","tool":"Bash","command":"ls"}"#).unwrap();
        assert_eq!(e, PulseEvent::Permission { tool: "Bash".into(), command: "ls".into(), description: String::new(), dangerous: false });
    }
    #[test] fn file_and_web_tools_are_never_dangerous() {
        let perm = |tool: &str, input: &str| match map_hook(&hook(&format!(r#"{{"session_id":"a","hook_event_name":"PermissionRequest","tool_name":"{tool}","tool_input":{input}}}"#))) { Some(PulseEvent::Permission { dangerous, .. }) => dangerous, o => panic!("{o:?}") };
        assert!(!perm("Edit", r#"{"file_path":"db/migrate/2024_add_users.rb"}"#));
        assert!(!perm("WebFetch", r#"{"url":"https://x.dev/how-to-migrate"}"#));
        assert!(perm("Bash", r#"{"command":"rails db:migrate"}"#));
        assert!(perm("mcp__db__execute_sql", r#"{"query":"DROP TABLE users"}"#));
        assert!(is_dangerous("cat img > /dev/nvme0n1"));
    }
    #[test] fn ask_timeout_comes_from_the_config_and_is_clamped() {
        assert_eq!(ask_timeout_from_config(r#"{"island":{"ask_timeout_s":60}}"#), 60);
        assert_eq!(ask_timeout_from_config(r#"{"island":{"ask_timeout_s":500}}"#), ASK_TIMEOUT_MAX_S);
        assert_eq!(ask_timeout_from_config(r#"{"island":{"ask_timeout_s":1}}"#), ASK_TIMEOUT_MIN_S);
        assert_eq!(ask_timeout_from_config(r#"{"island":{}}"#), ASK_TIMEOUT_S);
        assert_eq!(ask_timeout_from_config("basura"), ASK_TIMEOUT_S);
    }
    #[test] fn session_end_does_not_wake_the_app() {
        assert!(!wakes_app(&PulseEvent::SessionEnd));
        assert!(wakes_app(&PulseEvent::Done) && wakes_app(&PulseEvent::SessionStart));
    }
    #[test] fn permissions_always_say_what_they_want_to_do() {
        let c = |j| match map_hook(&hook(j)) { Some(PulseEvent::Permission { command, .. }) => command, o => panic!("{o:?}") };
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"PermissionRequest","tool_name":"Edit","tool_input":{"file_path":"src/x.ts","old_string":"a"}}"#), "Edit: src/x.ts");
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"PermissionRequest","tool_name":"Artifact","tool_input":{"action":"delete","url":"https://claude.ai/artifact/X"}}"#), "Artifact: action: delete · url: https://claude.ai/artifact/X");
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"PermissionRequest","tool_name":"mcp__slack__send","tool_input":{}}"#), "mcp__slack__send");
    }
    #[test] fn questions_and_plans_are_not_permissions() {
        let q = r#"{"session_id":"a","hook_event_name":"PermissionRequest","tool_name":"AskUserQuestion","tool_input":{"questions":[{"question":"¿Qué framework?","header":"Framework","options":[{"label":"React","description":"x"},{"label":"Vue","description":"y"}],"multiSelect":false}]}}"#;
        match map_hook(&hook(q)) { Some(PulseEvent::Question { questions }) => assert_eq!(questions[0]["header"], "Framework"), o => panic!("{o:?}") }
        assert_eq!(map_hook(&hook(r#"{"session_id":"a","hook_event_name":"PermissionRequest","tool_name":"ExitPlanMode","tool_input":{}}"#)), Some(PulseEvent::Plan));
        // la pregunta se responde en PreToolUse (la vía documentada); el resto de PreToolUse no pasa por aquí
        let pre = r#"{"session_id":"a","hook_event_name":"PreToolUse","tool_name":"AskUserQuestion","tool_input":{"questions":[{"question":"¿Ya?","header":"H","options":[],"multiSelect":false}]}}"#;
        assert!(matches!(question_hook(&hook(pre)), Some(PulseEvent::Question { .. })));
        assert_eq!(question_hook(&hook(r#"{"session_id":"a","hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"ls"}}"#)), None);
    }
    #[test] fn answer_output_echoes_questions_and_adds_answers() {
        let qs = json!([{ "question": "¿Qué framework?", "header": "Framework", "options": [{ "label": "React", "description": "x" }], "multiSelect": false }]);
        let out: Value = serde_json::from_str(&answer_output(&qs, &json!({ "¿Qué framework?": "React" }))).unwrap();
        assert_eq!(out["hookSpecificOutput"]["hookEventName"], "PreToolUse");
        assert_eq!(out["hookSpecificOutput"]["permissionDecision"], "allow");
        assert_eq!(out["hookSpecificOutput"]["updatedInput"]["questions"], qs);
        assert_eq!(out["hookSpecificOutput"]["updatedInput"]["answers"]["¿Qué framework?"], "React");
        let r: Reply = serde_json::from_str(r#"{"decision":"allow"}"#).unwrap();
        assert!(r.answers.is_none());
    }
    #[test] fn permission_output_shapes() {
        let allow: serde_json::Value = serde_json::from_str(&permission_output(Decision::Allow).unwrap()).unwrap();
        assert_eq!(allow["hookSpecificOutput"]["hookEventName"], "PermissionRequest");
        assert_eq!(allow["hookSpecificOutput"]["decision"]["behavior"], "allow");
        let deny: serde_json::Value = serde_json::from_str(&permission_output(Decision::Deny).unwrap()).unwrap();
        assert_eq!(deny["hookSpecificOutput"]["decision"]["behavior"], "deny");
        assert_eq!(permission_output(Decision::None), None);
    }
}
