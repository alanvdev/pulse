use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

pub const PIPE_NAME: &str = r"\\.\pipe\pulse";
pub const PROTOCOL_VERSION: u32 = 1;
pub const ASK_TIMEOUT_S: u64 = 30;
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
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Envelope {
    pub v: u32,
    pub session_id: String,
    pub project: String,
    pub cwd: String,
    pub at_ms: i64,
    #[serde(default)] pub ask: bool,
    pub event: PulseEvent,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Decision { Allow, Deny, None }

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Reply { pub decision: Decision }

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

pub fn map_hook(i: &HookInput) -> Option<PulseEvent> {
    let tool = i.tool_name.clone().unwrap_or_default();
    Some(match i.hook_event_name.as_str() {
        "SessionStart" => PulseEvent::SessionStart,
        "SessionEnd" => PulseEvent::SessionEnd,
        "UserPromptSubmit" => PulseEvent::Start { prompt: truncate(i.prompt.as_deref().unwrap_or("")) },
        "PreToolUse" => PulseEvent::Tool { target: tool_target(&tool, i.tool_input.as_ref()), tool },
        "PermissionRequest" => PulseEvent::Permission { command: tool_target(&tool, i.tool_input.as_ref()), description: truncate(i.tool_input.as_ref().and_then(|v| v.get("description")).and_then(Value::as_str).unwrap_or("")), tool },
        "Notification" if i.notification_type.as_deref() == Some("idle_prompt") => PulseEvent::Idle,
        "Stop" => PulseEvent::Done,
        "PostToolUseFailure" => PulseEvent::Error { message: truncate(i.error.as_deref().unwrap_or("falló una herramienta")) },
        "SubagentStart" => PulseEvent::SubagentStart { agent: i.agent_type.clone().unwrap_or_default() },
        "SubagentStop" => PulseEvent::SubagentStop { agent: i.agent_type.clone().unwrap_or_default() },
        _ => return None,
    })
}

pub fn envelope_from_hook(i: &HookInput, at_ms: i64, ask: bool) -> Option<Envelope> {
    Some(Envelope { v: PROTOCOL_VERSION, session_id: i.session_id.clone(), project: project_name(&i.cwd), cwd: i.cwd.clone(), at_ms, ask, event: map_hook(i)? })
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
    #[test] fn permission_output_shapes() {
        let allow: serde_json::Value = serde_json::from_str(&permission_output(Decision::Allow).unwrap()).unwrap();
        assert_eq!(allow["hookSpecificOutput"]["hookEventName"], "PermissionRequest");
        assert_eq!(allow["hookSpecificOutput"]["decision"]["behavior"], "allow");
        let deny: serde_json::Value = serde_json::from_str(&permission_output(Decision::Deny).unwrap()).unwrap();
        assert_eq!(deny["hookSpecificOutput"]["decision"]["behavior"], "deny");
        assert_eq!(permission_output(Decision::None), None);
    }
}
