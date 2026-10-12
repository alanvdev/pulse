use std::collections::{BTreeMap, VecDeque};
use pulse_protocol::{is_dangerous, project_name, Envelope, PulseEvent};
use serde::Serialize;

fn first_question(q: &serde_json::Value) -> String { q.get(0).and_then(|x| x.get("question")).and_then(|x| x.as_str()).unwrap_or("Claude te hace una pregunta").to_string() }

/// Tope absoluto para una pregunta en la isla (aunque esté en pausa): `pulse ask` espera un poco más y el hook de Claude Code, más.
pub const QUESTION_MAX_MS: i64 = (pulse_protocol::QUESTION_MAX_WAIT_S as i64 - 10) * 1000;

pub const IDLE_EXIT_MS: i64 = 5 * 60_000;
pub const STALE_SESSION_MS: i64 = 2 * 3_600_000;
const FEED_MAX: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum SessionState { Idle, Done, Working, IdleWait, Waiting }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue { Start, Permission, Done, Idle, Error, Subagent }

impl Cue { pub fn key(self) -> &'static str { match self { Cue::Start => "start", Cue::Permission => "permission", Cue::Done => "done", Cue::Idle => "idle", Cue::Error => "error", Cue::Subagent => "subagent" } } }

#[derive(Debug, Clone, Serialize)]
pub struct Session { pub id: String, pub project: String, #[serde(skip)] pub cwd: String, pub slot: u8, pub state: SessionState, pub last: String, pub subagents: u32, pub since_ms: i64, pub last_event_ms: i64, pub actions: u32, pub muted_until_ms: i64 }
#[derive(Debug, Clone, Serialize)]
/// `deadline_ms`: cuándo se va a la terminal. Las preguntas se pausan mientras las contestas (`paused`, con `left_ms` restantes).
pub struct Pending { pub id: u64, pub kind: String, pub session_id: String, pub project: String, pub tool: String, pub command: String, pub description: String, pub danger: bool, pub at_ms: i64, pub timeout_ms: i64, pub deadline_ms: i64, pub paused: bool, pub left_ms: i64, pub questions: Option<serde_json::Value> }
#[derive(Debug, Clone, Serialize)]
pub struct FeedItem { pub session_id: String, pub project: String, pub text: String, pub at_ms: i64 }
#[derive(Debug, Clone, Serialize)]
pub struct Deferred { pub session_id: String, pub project: String, pub kind: String, pub at_ms: i64 }
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot { pub sessions: Vec<Session>, pub pending: Vec<Pending>, pub feed: Vec<FeedItem>, pub focus: bool, pub focus_auto: bool, pub deferred: Vec<Deferred> }

#[derive(Debug, Default)]
pub struct Store {
    sessions: BTreeMap<String, Session>,
    pending: Vec<Pending>,
    feed: VecDeque<FeedItem>,
    deferred: Vec<Deferred>,
    turn_start: BTreeMap<String, i64>,
    focus_manual: Option<bool>,
    focus_auto: bool,
    last_event_ms: i64,
}

impl Store {
    /// Store que cuenta el autocierre desde `now_ms` (el arranque de la app), no desde 1970.
    pub fn new(now_ms: i64) -> Self { Self { last_event_ms: now_ms, ..Default::default() } }

    fn free_slot(&self) -> u8 { (0u8..).find(|n| !self.sessions.values().any(|s| s.slot == *n)).unwrap() }

    pub fn focus(&self) -> bool { self.focus_manual.unwrap_or(self.focus_auto) }

    pub fn apply(&mut self, e: &Envelope) -> Option<Cue> {
        self.last_event_ms = self.last_event_ms.max(e.at_ms);
        if e.event == PulseEvent::SessionEnd { self.sessions.remove(&e.session_id); self.turn_start.remove(&e.session_id); self.pending.retain(|p| p.session_id != e.session_id); return None; }
        let slot = self.free_slot();
        let s = self.sessions.entry(e.session_id.clone()).or_insert_with(|| Session {
            id: e.session_id.clone(), project: e.project.clone(), cwd: e.cwd.clone(), slot, state: SessionState::Idle, last: String::new(),
            subagents: 0, since_ms: e.at_ms, last_event_ms: e.at_ms, actions: 0, muted_until_ms: 0 });
        s.last_event_ms = e.at_ms;
        let set = |s: &mut Session, st: SessionState| if s.state != st { s.state = st; s.since_ms = e.at_ms; };
        let (cue, feed) = match &e.event {
            PulseEvent::SessionStart | PulseEvent::SessionEnd => (None, None),
            PulseEvent::Start { prompt } => { set(s, SessionState::Working); s.last = prompt.clone(); s.actions += 1; self.turn_start.insert(e.session_id.clone(), e.at_ms); (Some(Cue::Start), Some(format!("Prompt {prompt}"))) }
            PulseEvent::Tool { tool, target } => {
                if s.state != SessionState::Waiting { set(s, SessionState::Working); }
                s.last = if target.is_empty() { tool.clone() } else { format!("{tool} {target}") };
                s.actions += 1;
                (None, Some(s.last.clone()))
            }
            PulseEvent::Permission { command, .. } => { set(s, SessionState::Waiting); s.last = command.clone(); (Some(Cue::Permission), Some(format!("Permiso {command}"))) }
            PulseEvent::Question { questions } => {
                let q = first_question(questions);
                if e.ask { set(s, SessionState::Waiting); s.last = format!("Te pregunta: {q}"); (Some(Cue::Permission), Some(format!("Pregunta {q}"))) }
                else { set(s, SessionState::IdleWait); s.last = format!("Te pregunta en la terminal: {q}"); (Some(Cue::Idle), None) }
            }
            PulseEvent::Plan => { set(s, SessionState::IdleWait); s.last = "Plan listo para revisar".into(); (Some(Cue::Idle), Some("Plan listo".into())) }
            PulseEvent::Idle => { set(s, SessionState::IdleWait); (Some(Cue::Idle), None) }
            PulseEvent::Done => { set(s, SessionState::Done); s.subagents = 0; (Some(Cue::Done), Some("Terminó".into())) }
            PulseEvent::Error { message } => { s.last = format!("Falló: {message}"); (Some(Cue::Error), Some(format!("Error {message}"))) }
            PulseEvent::SubagentStart { agent } => { s.subagents += 1; set(s, SessionState::Working); (None, Some(format!("Agent {agent}"))) }
            PulseEvent::SubagentStop { agent } => { s.subagents = s.subagents.saturating_sub(1); (Some(Cue::Subagent), Some(format!("Agent terminó: {agent}"))) }
        };
        if let Some(text) = feed {
            self.feed.push_front(FeedItem { session_id: e.session_id.clone(), project: e.project.clone(), text, at_ms: e.at_ms });
            self.feed.truncate(FEED_MAX);
        }
        cue
    }

    pub fn add_pending(&mut self, id: u64, e: &Envelope, timeout_ms: i64) {
        let base = |kind: &str, tool: &str, command: String, description: String, danger: bool, questions: Option<serde_json::Value>| Pending { id, kind: kind.into(), session_id: e.session_id.clone(), project: e.project.clone(), tool: tool.into(), command, description, danger, at_ms: e.at_ms, timeout_ms, deadline_ms: e.at_ms + timeout_ms, paused: false, left_ms: timeout_ms, questions };
        match &e.event {
            PulseEvent::Permission { tool, command, description } => self.pending.push(base("permission", tool, command.clone(), description.clone(), is_dangerous(command), None)),
            PulseEvent::Question { questions } => self.pending.push(base("question", "AskUserQuestion", first_question(questions), String::new(), false, Some(questions.clone()))),
            _ => {}
        }
    }

    /// El primer permiso de verdad (los atajos Ctrl+Alt+Y/N no contestan preguntas).
    pub fn first_permission(&self) -> Option<&Pending> { self.pending.iter().find(|p| p.kind == "permission") }

    /// Fija el proyecto de la sesión en su carpeta raíz: Claude hace `cd` a subcarpetas y el `cwd` del hook cambia.
    /// La raíz es el `cwd` más corto visto que contiene a los demás.
    pub fn canonical(&mut self, mut e: Envelope) -> Envelope {
        if let Some(s) = self.sessions.get_mut(&e.session_id) {
            let (old, new) = (s.cwd.to_lowercase().replace('/', "\\"), e.cwd.to_lowercase().replace('/', "\\"));
            if !new.is_empty() && (old.is_empty() || (old.starts_with(&new) && old[new.len()..].starts_with('\\'))) { s.cwd = e.cwd.clone(); s.project = project_name(&e.cwd); }
            e.project = s.project.clone();
        }
        e
    }

    pub fn resolve(&mut self, id: u64) -> Option<Pending> {
        let i = self.pending.iter().position(|p| p.id == id)?;
        let p = self.pending.remove(i);
        if let Some(s) = self.sessions.get_mut(&p.session_id) { if !self.pending.iter().any(|q| q.session_id == p.session_id) { s.state = SessionState::Working; } }
        Some(p)
    }

    /// Permisos de la sesión de `e` que ya se respondieron en la terminal: si Claude siguió (empezó otro turno, terminó,
    /// falló, quedó esperando), ya no espera a la isla. Las herramientas no cuentan: pueden correr en paralelo al permiso.
    pub fn answered_elsewhere(&self, e: &Envelope) -> Vec<u64> {
        if matches!(e.event, PulseEvent::Permission { .. } | PulseEvent::Question { .. } | PulseEvent::Plan | PulseEvent::Tool { .. } | PulseEvent::SessionStart | PulseEvent::SubagentStart { .. }) { return Vec::new(); }
        self.pending.iter().filter(|p| p.session_id == e.session_id).map(|p| p.id).collect()
    }

    pub fn expire(&mut self, now_ms: i64) -> Vec<u64> {
        self.pending.iter().filter(|p| (!p.paused && now_ms >= p.deadline_ms) || (p.kind == "question" && now_ms - p.at_ms >= QUESTION_MAX_MS)).map(|p| p.id).collect()
    }

    /// Pausa (o reanuda) la cuenta regresiva de una pregunta mientras la contestas en la isla.
    pub fn pause(&mut self, id: u64, on: bool, now_ms: i64) {
        let Some(p) = self.pending.iter_mut().find(|p| p.id == id && p.kind == "question") else { return };
        if on && !p.paused { p.left_ms = (p.deadline_ms - now_ms).max(0); p.paused = true; }
        if !on && p.paused { p.deadline_ms = now_ms + p.left_ms; p.paused = false; }
    }

    pub fn set_focus(&mut self, manual: Option<bool>, auto: bool) -> Vec<Deferred> {
        let was = self.focus();
        if manual.is_some() { self.focus_manual = manual; }
        // un "apagar a mano" vale solo mientras dure lo que activó el foco automático
        else if auto != self.focus_auto && self.focus_manual == Some(false) { self.focus_manual = None; }
        self.focus_auto = auto;
        if was && !self.focus() { std::mem::take(&mut self.deferred) } else { Vec::new() }
    }

    pub fn defer(&mut self, e: &Envelope, kind: &str) { self.deferred.push(Deferred { session_id: e.session_id.clone(), project: e.project.clone(), kind: kind.into(), at_ms: e.at_ms }); }

    /// Cuánto lleva el turno de la sesión (desde su último prompt) en `at_ms`, para el aviso "terminó 6m 12s".
    pub fn turn_ms(&self, session_id: &str, at_ms: i64) -> Option<i64> { self.turn_start.get(session_id).map(|t| at_ms - t) }

    pub fn mute(&mut self, session_id: &str, until_ms: i64) { if let Some(s) = self.sessions.get_mut(session_id) { s.muted_until_ms = until_ms; } }

    pub fn is_muted(&self, session_id: &str, now_ms: i64) -> bool { self.sessions.get(session_id).is_some_and(|s| s.muted_until_ms > now_ms) }

    pub fn slot_of(&self, session_id: &str) -> u8 { self.sessions.get(session_id).map(|s| s.slot).unwrap_or(0) }

    pub fn aggregate(&self) -> SessionState { self.sessions.values().map(|s| s.state).max().unwrap_or(SessionState::Idle) }

    pub fn summary(&self) -> String {
        if self.sessions.is_empty() { return "Pulse · sin sesiones".into(); }
        let w = self.sessions.values().filter(|s| s.state == SessionState::Working).count();
        let n = self.sessions.values().filter(|s| matches!(s.state, SessionState::Waiting | SessionState::IdleWait)).count();
        let mut out = String::from("Pulse");
        if w > 0 { out += &format!(" · {w} trabajando"); }
        if n > 0 { out += &format!(" · {n} te necesita{}", if n > 1 { "n" } else { "" }); }
        if w == 0 && n == 0 { out += &format!(" · {} en calma", self.sessions.len()); }
        out
    }

    pub fn should_exit(&mut self, now_ms: i64) -> bool {
        let mut freed_at = self.last_event_ms;
        self.sessions.retain(|_, s| { let alive = now_ms - s.last_event_ms < STALE_SESSION_MS; if !alive { freed_at = freed_at.max(s.last_event_ms + STALE_SESSION_MS); } alive });
        self.sessions.is_empty() && self.pending.is_empty() && now_ms - freed_at >= IDLE_EXIT_MS
    }

    pub fn snapshot(&self) -> Snapshot {
        let mut sessions: Vec<Session> = self.sessions.values().cloned().collect();
        sessions.sort_by_key(|s| s.slot);
        Snapshot { sessions, pending: self.pending.clone(), feed: self.feed.iter().cloned().collect(), focus: self.focus(), focus_auto: self.focus_auto, deferred: self.deferred.clone() }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use pulse_protocol::{Envelope, PulseEvent};
    fn env(sid: &str, at: i64, event: PulseEvent) -> Envelope { Envelope { v: 1, session_id: sid.into(), project: format!("p-{sid}"), cwd: String::new(), at_ms: at, ask: false, event } }
    fn perm(sid: &str, at: i64, cmd: &str) -> Envelope { env(sid, at, PulseEvent::Permission { tool: "Bash".into(), command: cmd.into(), description: String::new() }) }

    #[test] fn lifecycle_of_a_session() {
        let mut s = Store::default();
        assert_eq!(s.apply(&env("a", 1, PulseEvent::Start { prompt: "hola".into() })), Some(Cue::Start));
        assert_eq!(s.apply(&env("a", 2, PulseEvent::Tool { tool: "Edit".into(), target: "a.ts".into() })), None);
        let a = s.snapshot().sessions[0].clone();
        assert_eq!((a.state, a.last.as_str(), a.actions, a.slot), (SessionState::Working, "Edit a.ts", 2, 0));
        assert_eq!(s.apply(&env("a", 3, PulseEvent::Done)), Some(Cue::Done));
        assert_eq!(s.snapshot().sessions[0].state, SessionState::Done);
        s.apply(&env("a", 4, PulseEvent::SessionEnd));
        assert!(s.snapshot().sessions.is_empty());
    }
    #[test] fn slots_are_assigned_in_arrival_order_and_reused() {
        let mut s = Store::default();
        for (i, id) in ["a", "b", "c"].iter().enumerate() { s.apply(&env(id, i as i64, PulseEvent::SessionStart)); }
        s.apply(&env("b", 9, PulseEvent::SessionEnd));
        s.apply(&env("d", 10, PulseEvent::SessionStart));
        let slot = |id: &str| s.snapshot().sessions.iter().find(|x| x.id == id).unwrap().slot;
        assert_eq!((slot("a"), slot("c"), slot("d")), (0, 2, 1));
    }
    #[test] fn feed_keeps_last_40_newest_first() {
        let mut s = Store::default();
        for i in 0..45 { s.apply(&env("a", i, PulseEvent::Tool { tool: "Bash".into(), target: format!("cmd {i}") })); }
        let f = s.snapshot().feed;
        assert_eq!(f.len(), 40);
        assert_eq!(f[0].text, "Bash cmd 44");
    }
    #[test] fn pending_permissions() {
        let mut s = Store::default();
        let e = perm("a", 10, "git push --force");
        s.apply(&e);
        s.add_pending(7, &e, 30_000);
        let p = &s.snapshot().pending[0];
        assert!(p.danger && p.id == 7 && p.timeout_ms == 30_000);
        assert_eq!(s.aggregate(), SessionState::Waiting);
        assert_eq!(s.resolve(7).unwrap().command, "git push --force");
        assert!(s.snapshot().pending.is_empty());
        assert_eq!(s.snapshot().sessions[0].state, SessionState::Working);
    }
    fn question(sid: &str, at: i64, ask: bool) -> Envelope { let mut e = env(sid, at, PulseEvent::Question { questions: serde_json::json!([{ "question": "¿Qué framework?", "header": "Framework", "options": [], "multiSelect": false }]) }); e.ask = ask; e }
    #[test] fn questions_wait_for_an_answer_and_are_not_permissions() {
        let mut s = Store::default();
        let q = question("a", 0, true);
        assert_eq!(s.apply(&q), Some(Cue::Permission));
        s.add_pending(5, &q, 60_000);
        let p = &s.snapshot().pending[0];
        assert_eq!((p.kind.as_str(), p.command.as_str(), p.danger), ("question", "¿Qué framework?", false));
        assert!(p.questions.is_some());
        assert_eq!(s.snapshot().sessions[0].state, SessionState::Waiting);
        assert!(s.first_permission().is_none(), "Ctrl+Alt+Y/N no deben contestar preguntas");
        let e = perm("a", 1, "ls"); s.apply(&e); s.add_pending(6, &e, 30_000);
        assert_eq!(s.first_permission().unwrap().id, 6);
    }
    #[test] fn question_countdown_pauses_while_answering() {
        let mut s = Store::default();
        let q = question("a", 0, true);
        s.apply(&q); s.add_pending(5, &q, 60_000);
        assert_eq!(s.snapshot().pending[0].deadline_ms, 60_000);
        s.pause(5, true, 20_000);                         // entras a la tarjeta a los 20 s: quedan 40 s
        assert!(s.expire(90_000).is_empty());             // en pausa no expira
        s.pause(5, false, 100_000);                       // sales a los 100 s: vuelven a quedar 40 s
        assert_eq!(s.snapshot().pending[0].deadline_ms, 140_000);
        assert!(s.expire(139_999).is_empty());
        assert_eq!(s.expire(140_000), vec![5]);
        // tope absoluto: aunque sigas en pausa, se va a la terminal antes de que Claude Code corte el hook
        let q2 = question("b", 0, true); s.apply(&q2); s.add_pending(6, &q2, 60_000);
        s.pause(6, true, 1_000);
        assert!(s.expire(QUESTION_MAX_MS - 1).iter().all(|id| *id != 6));
        assert!(s.expire(QUESTION_MAX_MS).contains(&6));
    }
    #[test] fn questions_and_plans_in_the_terminal_only_notify() {
        let mut s = Store::default();
        assert_eq!(s.apply(&question("a", 0, false)), Some(Cue::Idle));
        assert_eq!(s.snapshot().sessions[0].state, SessionState::IdleWait);
        assert_eq!(s.apply(&env("b", 0, PulseEvent::Plan)), Some(Cue::Idle));
        assert_eq!(s.snapshot().sessions[1].last, "Plan listo para revisar");
    }
    #[test] fn project_name_stays_at_the_session_root() {
        let mut s = Store::default();
        let mut at = |cwd: &str, t: i64| { let e = s.canonical(Envelope { cwd: cwd.into(), project: pulse_protocol::project_name(cwd), ..env("a", t, PulseEvent::Done) }); s.apply(&e); e.project };   // como en handle()
        assert_eq!(at(r"D:\p\pulse\app\src", 0), "src");                // primer evento desde una subcarpeta
        assert_eq!(at(r"D:\p\pulse", 1), "pulse");                      // aparece la raíz
        assert_eq!(at(r"D:\p\pulse\app\ui\src\island", 2), "pulse");   // cd a subcarpetas no lo cambia
        assert_eq!(s.snapshot().sessions[0].project, "pulse");
    }
    #[test] fn later_events_mean_it_was_answered_in_the_terminal() {
        let mut s = Store::default();
        let e = perm("a", 0, "npm test");
        s.apply(&e); s.add_pending(3, &e, 30_000);
        let other = perm("b", 0, "ls"); s.apply(&other); s.add_pending(4, &other, 30_000);
        assert!(s.answered_elsewhere(&env("a", 1, PulseEvent::Tool { tool: "Bash".into(), target: "x".into() })).is_empty());   // herramientas en paralelo: no cuenta
        assert_eq!(s.answered_elsewhere(&env("a", 2, PulseEvent::Done)), vec![3]);
        assert_eq!(s.answered_elsewhere(&env("a", 2, PulseEvent::Start { prompt: "x".into() })), vec![3]);
        assert!(s.answered_elsewhere(&env("c", 2, PulseEvent::Done)).is_empty());
    }
    #[test] fn pending_expire_after_timeout() {
        let mut s = Store::default();
        let e = perm("a", 0, "npm run build");
        s.apply(&e); s.add_pending(1, &e, 30_000);
        assert!(s.expire(29_999).is_empty());
        assert_eq!(s.expire(30_000), vec![1]);
    }
    #[test] fn focus_defers_and_releases() {
        let mut s = Store::default();
        s.set_focus(Some(true), false);
        s.defer(&env("a", 1, PulseEvent::Done), "done");
        assert_eq!(s.snapshot().deferred.len(), 1);
        let released = s.set_focus(Some(false), false);
        assert_eq!(released.len(), 1);
        assert_eq!(released[0].at_ms, 1);
        assert!(s.snapshot().deferred.is_empty() && !s.snapshot().focus);
    }
    #[test] fn done_knows_how_long_the_turn_took() {
        let mut s = Store::default();
        s.apply(&env("a", 1_000, PulseEvent::Start { prompt: "x".into() }));
        s.apply(&env("a", 5_000, PulseEvent::Tool { tool: "Bash".into(), target: "ls".into() }));
        s.apply(&env("a", 373_000, PulseEvent::Done));
        assert_eq!(s.turn_ms("a", 373_000), Some(372_000));
        assert_eq!(s.turn_ms("b", 1), None);
        s.apply(&env("a", 400_000, PulseEvent::SessionEnd));
        assert_eq!(s.turn_ms("a", 400_000), None);
    }
    #[test] fn auto_focus_counts_unless_manual_off() {
        let mut s = Store::default();
        s.set_focus(None, true);
        assert!(s.snapshot().focus);
        s.set_focus(Some(false), true);
        assert!(!s.snapshot().focus);
        // el "apagar a mano" vale solo mientras dure lo que lo activó
        s.set_focus(None, false);
        s.set_focus(None, true);
        assert!(s.snapshot().focus);
        // un "encender a mano" sí se queda aunque cambie lo automático
        s.set_focus(Some(true), false);
        s.set_focus(None, true); s.set_focus(None, false);
        assert!(s.snapshot().focus);
    }
    #[test] fn exit_rules() {
        let mut s = Store::default();
        s.apply(&env("a", 0, PulseEvent::Start { prompt: "x".into() }));
        assert!(!s.should_exit(IDLE_EXIT_MS * 3));
        assert!(!s.should_exit(STALE_SESSION_MS - 1));
        assert!(s.should_exit(STALE_SESSION_MS + IDLE_EXIT_MS));
        let mut t = Store::default();
        t.apply(&env("a", 1_000, PulseEvent::SessionEnd));
        assert!(!t.should_exit(1_000 + IDLE_EXIT_MS - 1));
        assert!(t.should_exit(1_000 + IDLE_EXIT_MS));
        let mut fresh = Store::new(50_000);
        assert!(!fresh.should_exit(50_000 + 1_000));
        assert!(fresh.should_exit(50_000 + IDLE_EXIT_MS));
    }
    #[test] fn summary_in_spanish() {
        let mut s = Store::default();
        assert_eq!(s.summary(), "Pulse · sin sesiones");
        s.apply(&env("a", 1, PulseEvent::Start { prompt: "x".into() }));
        s.apply(&perm("b", 1, "ls"));
        assert_eq!(s.summary(), "Pulse · 1 trabajando · 1 te necesita");
    }
}
