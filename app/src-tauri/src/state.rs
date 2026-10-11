use std::collections::{BTreeMap, VecDeque};
use pulse_protocol::{is_dangerous, Envelope, PulseEvent};
use serde::Serialize;

pub const IDLE_EXIT_MS: i64 = 5 * 60_000;
pub const STALE_SESSION_MS: i64 = 2 * 3_600_000;
const FEED_MAX: usize = 40;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub enum SessionState { Idle, Done, Working, IdleWait, Waiting }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Cue { Start, Permission, Done, Idle, Error, Subagent }

impl Cue { pub fn key(self) -> &'static str { match self { Cue::Start => "start", Cue::Permission => "permission", Cue::Done => "done", Cue::Idle => "idle", Cue::Error => "error", Cue::Subagent => "subagent" } } }

#[derive(Debug, Clone, Serialize)]
pub struct Session { pub id: String, pub project: String, pub slot: u8, pub state: SessionState, pub last: String, pub subagents: u32, pub since_ms: i64, pub last_event_ms: i64, pub actions: u32, pub muted_until_ms: i64 }
#[derive(Debug, Clone, Serialize)]
pub struct Pending { pub id: u64, pub session_id: String, pub project: String, pub tool: String, pub command: String, pub danger: bool, pub at_ms: i64, pub timeout_ms: i64 }
#[derive(Debug, Clone, Serialize)]
pub struct FeedItem { pub session_id: String, pub project: String, pub text: String, pub at_ms: i64 }
#[derive(Debug, Clone, Serialize)]
pub struct Deferred { pub session_id: String, pub project: String, pub kind: String }
#[derive(Debug, Clone, Serialize)]
pub struct Snapshot { pub sessions: Vec<Session>, pub pending: Vec<Pending>, pub feed: Vec<FeedItem>, pub focus: bool, pub focus_auto: bool, pub deferred: Vec<Deferred> }

#[derive(Debug, Default)]
pub struct Store {
    sessions: BTreeMap<String, Session>,
    pending: Vec<Pending>,
    feed: VecDeque<FeedItem>,
    deferred: Vec<Deferred>,
    focus_manual: Option<bool>,
    focus_auto: bool,
    last_event_ms: i64,
}

impl Store {
    fn free_slot(&self) -> u8 { (0u8..).find(|n| !self.sessions.values().any(|s| s.slot == *n)).unwrap() }

    pub fn focus(&self) -> bool { self.focus_manual.unwrap_or(self.focus_auto) }

    pub fn apply(&mut self, e: &Envelope) -> Option<Cue> {
        self.last_event_ms = self.last_event_ms.max(e.at_ms);
        if e.event == PulseEvent::SessionEnd { self.sessions.remove(&e.session_id); self.pending.retain(|p| p.session_id != e.session_id); return None; }
        let slot = self.free_slot();
        let s = self.sessions.entry(e.session_id.clone()).or_insert_with(|| Session {
            id: e.session_id.clone(), project: e.project.clone(), slot, state: SessionState::Idle, last: String::new(),
            subagents: 0, since_ms: e.at_ms, last_event_ms: e.at_ms, actions: 0, muted_until_ms: 0 });
        s.last_event_ms = e.at_ms;
        let set = |s: &mut Session, st: SessionState| if s.state != st { s.state = st; s.since_ms = e.at_ms; };
        let (cue, feed) = match &e.event {
            PulseEvent::SessionStart | PulseEvent::SessionEnd => (None, None),
            PulseEvent::Start { prompt } => { set(s, SessionState::Working); s.last = prompt.clone(); s.actions += 1; (Some(Cue::Start), Some(format!("Prompt {prompt}"))) }
            PulseEvent::Tool { tool, target } => {
                if s.state != SessionState::Waiting { set(s, SessionState::Working); }
                s.last = if target.is_empty() { tool.clone() } else { format!("{tool} {target}") };
                s.actions += 1;
                (None, Some(s.last.clone()))
            }
            PulseEvent::Permission { command, .. } => { set(s, SessionState::Waiting); s.last = command.clone(); (Some(Cue::Permission), Some(format!("Permiso {command}"))) }
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
        if let PulseEvent::Permission { tool, command } = &e.event {
            self.pending.push(Pending { id, session_id: e.session_id.clone(), project: e.project.clone(), tool: tool.clone(), command: command.clone(), danger: is_dangerous(command), at_ms: e.at_ms, timeout_ms });
        }
    }

    pub fn resolve(&mut self, id: u64) -> Option<Pending> {
        let i = self.pending.iter().position(|p| p.id == id)?;
        let p = self.pending.remove(i);
        if let Some(s) = self.sessions.get_mut(&p.session_id) { if !self.pending.iter().any(|q| q.session_id == p.session_id) { s.state = SessionState::Working; } }
        Some(p)
    }

    pub fn expire(&mut self, now_ms: i64) -> Vec<u64> {
        self.pending.iter().filter(|p| now_ms - p.at_ms >= p.timeout_ms).map(|p| p.id).collect()
    }

    pub fn set_focus(&mut self, manual: Option<bool>, auto: bool) -> Vec<Deferred> {
        let was = self.focus();
        if manual.is_some() { self.focus_manual = manual; }
        self.focus_auto = auto;
        if was && !self.focus() { std::mem::take(&mut self.deferred) } else { Vec::new() }
    }

    pub fn defer(&mut self, e: &Envelope, kind: &str) { self.deferred.push(Deferred { session_id: e.session_id.clone(), project: e.project.clone(), kind: kind.into() }); }

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
    fn perm(sid: &str, at: i64, cmd: &str) -> Envelope { env(sid, at, PulseEvent::Permission { tool: "Bash".into(), command: cmd.into() }) }

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
        assert!(s.snapshot().deferred.is_empty() && !s.snapshot().focus);
    }
    #[test] fn auto_focus_counts_unless_manual_off() {
        let mut s = Store::default();
        s.set_focus(None, true);
        assert!(s.snapshot().focus);
        s.set_focus(Some(false), true);
        assert!(!s.snapshot().focus);
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
    }
    #[test] fn summary_in_spanish() {
        let mut s = Store::default();
        assert_eq!(s.summary(), "Pulse · sin sesiones");
        s.apply(&env("a", 1, PulseEvent::Start { prompt: "x".into() }));
        s.apply(&perm("b", 1, "ls"));
        assert_eq!(s.summary(), "Pulse · 1 trabajando · 1 te necesita");
    }
}
