use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub mod owner;

pub const PIPE_NAME: &str = r"\\.\pipe\pulse";

/// Dónde escucha la app: el named pipe en Windows; en Linux un socket Unix en `$XDG_RUNTIME_DIR` (o en el temporal, por usuario).
pub fn endpoint() -> String {
    if cfg!(windows) { return PIPE_NAME.to_string(); }
    match std::env::var("XDG_RUNTIME_DIR") {
        Ok(d) if !d.is_empty() => format!("{d}/pulse.sock"),
        _ => std::env::temp_dir().join(format!("pulse-{}.sock", std::env::var("USER").unwrap_or_default())).to_string_lossy().into_owned(),
    }
}

/// Un atajo de teclado que llega por `pulse key <acción>`: en Linux los atajos globales los registra el escritorio, no la app.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct KeyPress { pub key: String }

pub const PROTOCOL_VERSION: u32 = 1;
pub const ASK_TIMEOUT_S: u64 = 30;
/// Cuánto espera la isla una respuesta a una pregunta de Claude antes de dejarla en la terminal.
pub const QUESTION_TIMEOUT_S: u64 = 60;
/// Lo más que `pulse ask` espera una pregunta, contando pausas mientras la contestas. El hook se registra con un poco más.
pub const QUESTION_MAX_WAIT_S: u64 = 300;
const MAX_TEXT: usize = 120;

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
    Permission { tool: String, command: String, #[serde(default)] description: String },
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

fn tool_target(tool: &str, input: Option<&Value>) -> String {
    let f = |k: &str| input.and_then(|v| v.get(k)).and_then(Value::as_str).map(str::to_string);
    truncate(&match tool {
        "Bash" | "PowerShell" => f("command"),
        "Read" | "Edit" | "Write" | "MultiEdit" | "NotebookEdit" => f("file_path"),
        "Grep" | "Glob" => f("pattern"),
        "Agent" | "Task" => f("description"),
        "WebFetch" => f("url"),
        "WebSearch" => f("query"),
        _ => None,
    }.unwrap_or_default())
}

/// Qué quiere hacer un permiso, en una línea. Shell: el comando tal cual. Otras herramientas: su nombre y el objetivo
/// (o un resumen de sus argumentos si Pulse no la conoce: Artifact, herramientas MCP, etc.).
fn permission_label(tool: &str, input: Option<&Value>) -> String {
    if matches!(tool, "Bash" | "PowerShell") { return tool_target(tool, input); }
    let target = tool_target(tool, input);
    let detail = if !target.is_empty() { target } else {
        input.and_then(Value::as_object).map(|m| m.iter().filter_map(|(k, v)| match v {
            Value::String(s) => Some(format!("{k}: {s}")), Value::Number(n) => Some(format!("{k}: {n}")), Value::Bool(b) => Some(format!("{k}: {b}")), _ => None,
        }).take(3).collect::<Vec<_>>().join(" · ")).unwrap_or_default()
    };
    truncate(&if detail.is_empty() { tool.to_string() } else { format!("{tool}: {detail}") })
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
        "PermissionRequest" => PulseEvent::Permission { command: permission_label(&tool, i.tool_input.as_ref()), description: truncate(i.tool_input.as_ref().and_then(|v| v.get("description")).and_then(Value::as_str).unwrap_or("")), tool },
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

pub fn is_dangerous(cmd: &str) -> bool {
    let c = cmd.to_lowercase();
    ["push --force", "push -f", "rm -rf", "rm -r ", "migrate", "reset --hard", "remove-item -recurse", "drop table", "drop database", "format "]
        .iter().any(|p| c.contains(p))
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
        assert_eq!(project_name(r"D:\Documents\Hyper\dev\hyperflow\hyperflow-app"), "hyperflow-app");
        assert_eq!(project_name("/x/proppie-api/"), "proppie-api");
        assert_eq!(project_name(""), "sesión");
    }
    #[test] fn maps_every_hook_we_use() {
        let c = |j| map_hook(&hook(j));
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"UserPromptSubmit","prompt":"hola"}"#), Some(PulseEvent::Start { prompt: "hola".into() }));
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"PreToolUse","tool_name":"Bash","tool_input":{"command":"npm run lint"}}"#), Some(PulseEvent::Tool { tool: "Bash".into(), target: "npm run lint".into() }));
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"PreToolUse","tool_name":"Edit","tool_input":{"file_path":"src/H.tsx"}}"#), Some(PulseEvent::Tool { tool: "Edit".into(), target: "src/H.tsx".into() }));
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"npx prisma migrate dev"}}"#), Some(PulseEvent::Permission { tool: "Bash".into(), command: "npx prisma migrate dev".into(), description: String::new() }));
        assert_eq!(c(r#"{"session_id":"a","hook_event_name":"PermissionRequest","tool_name":"Bash","tool_input":{"command":"rm -rf node_modules","description":"Remove node_modules directory"}}"#), Some(PulseEvent::Permission { tool: "Bash".into(), command: "rm -rf node_modules".into(), description: "Remove node_modules directory".into() }));
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
