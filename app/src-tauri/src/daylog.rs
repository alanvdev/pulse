use serde::Serialize;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Serialize, Default)]
pub struct SessionDay { pub project: String, pub work_ms: i64, pub waited_ms: i64, pub max_wait_ms: i64, pub perms: u32, pub errors: u32, pub subagents: u32, pub actions: u32, pub lanes: Vec<(f64, f64, char)> }
#[derive(Debug, Clone, Serialize, Default)]
pub struct DaySummary { pub sessions: Vec<SessionDay>, pub total_work_ms: i64, pub total_waited_ms: i64, pub actions_by_hour: [u32; 24] }

#[derive(Default)] struct Open { work: Option<i64>, wait: Option<i64>, last: i64 }
/// Una sesión sin eventos por más de esto ya no está trabajando ni esperando: lo abierto se cierra en su último evento.
const STALE_MS: i64 = 15 * 60_000;

pub fn summarize(lines: impl Iterator<Item = String>, day_start_ms: i64, now_ms: i64) -> DaySummary {
    let span = (now_ms - day_start_ms).max(1) as f64;
    let frac = |t: i64| (t - day_start_ms) as f64 / span;
    let mut days: BTreeMap<String, (SessionDay, Open)> = BTreeMap::new();
    let mut out = DaySummary::default();
    for line in lines {
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else { continue };
        let (Some(sid), Some(at), Some(kind)) = (v["session_id"].as_str(), v["at_ms"].as_i64(), v["event"]["kind"].as_str()) else { continue };
        if at < day_start_ms || at > now_ms { continue; }
        let (d, o) = days.entry(sid.to_string()).or_default();
        o.last = o.last.max(at);
        if d.project.is_empty() { d.project = v["project"].as_str().unwrap_or("sesión").to_string(); }
        let close_wait = |d: &mut SessionDay, o: &mut Open, t: i64| if let Some(w) = o.wait.take() { d.waited_ms += t - w; d.max_wait_ms = d.max_wait_ms.max(t - w); d.lanes.push((frac(w), frac(t), 'p')); };
        match kind {
            "start" => { close_wait(d, o, at); if o.work.is_none() { o.work = Some(at); } d.actions += 1; out.actions_by_hour[((at - day_start_ms) / 3_600_000).clamp(0, 23) as usize] += 1; }
            "tool" => { if o.work.is_none() { o.work = Some(at); } d.actions += 1; out.actions_by_hour[((at - day_start_ms) / 3_600_000).clamp(0, 23) as usize] += 1; }
            "permission" => { d.perms += 1; o.wait = Some(at); if o.work.is_none() { o.work = Some(at); } }
            "resolve" => close_wait(d, o, at),
            "done" => { close_wait(d, o, at); if let Some(w) = o.work.take() { d.work_ms += at - w; d.lanes.push((frac(w), frac(at), 'w')); } }
            "error" => { d.errors += 1; d.lanes.push((frac(at), frac(at), 'e')); }
            "subagent-stop" => d.subagents += 1,
            _ => {}
        }
    }
    // lo que quedó abierto llega hasta ahora si la sesión sigue viva; si no, hasta su último evento
    let mut projects: BTreeMap<String, (SessionDay, i64)> = BTreeMap::new();
    for (_, (mut d, mut o)) in days {
        let end = if now_ms - o.last > STALE_MS { o.last } else { now_ms };
        if let Some(w) = o.wait.take() { d.waited_ms += end - w; d.max_wait_ms = d.max_wait_ms.max(end - w); d.lanes.push((frac(w), frac(end), 'p')); }
        if let Some(w) = o.work.take() { d.work_ms += end - w; d.lanes.push((frac(w), frac(end), 'w')); }
        out.total_work_ms += d.work_ms; out.total_waited_ms += d.waited_ms;
        if d.lanes.is_empty() { continue; }   // solo se abrió: no hay nada que dibujar
        // una fila por proyecto (spec §8): las sesiones del mismo proyecto se juntan
        let (p, last) = projects.entry(d.project.clone()).or_insert_with(|| (SessionDay { project: d.project.clone(), ..Default::default() }, 0));
        p.work_ms += d.work_ms; p.waited_ms += d.waited_ms; p.max_wait_ms = p.max_wait_ms.max(d.max_wait_ms);
        p.perms += d.perms; p.errors += d.errors; p.subagents += d.subagents; p.actions += d.actions; p.lanes.extend(d.lanes);
        *last = (*last).max(o.last);
    }
    let mut rows: Vec<(SessionDay, i64)> = projects.into_values().collect();
    rows.sort_by(|a, b| b.1.cmp(&a.1));   // el más reciente arriba
    out.sessions = rows.into_iter().map(|(d, _)| d).collect();
    out
}

pub fn today() -> DaySummary {
    let now = crate::now_ms();
    let start = local_midnight_ms(now);
    let lines = crate::logbook::path().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    summarize(lines.lines().map(str::to_string), start, now)
}

/// Cuánto hay que sumarle a UTC para tener la hora local, en ms.
#[cfg(windows)]
pub(crate) fn utc_offset_ms(_now_ms: i64) -> i64 {
    use windows_sys::Win32::System::Time::{GetTimeZoneInformation, TIME_ZONE_INFORMATION};
    let mut tz: TIME_ZONE_INFORMATION = unsafe { std::mem::zeroed() };
    unsafe { GetTimeZoneInformation(&mut tz) };
    -(tz.Bias as i64) * 60_000   // Bias está en minutos, UTC = local + Bias
}

#[cfg(unix)]
pub(crate) fn utc_offset_ms(now_ms: i64) -> i64 {
    let t = now_ms.div_euclid(1000) as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    if unsafe { libc::localtime_r(&t, &mut tm) }.is_null() { return 0; }
    tm.tm_gmtoff as i64 * 1000
}

pub(crate) fn local_midnight_ms(now_ms: i64) -> i64 {
    let offset_ms = utc_offset_ms(now_ms);
    let local = now_ms + offset_ms;
    local - local.rem_euclid(86_400_000) - offset_ms
}

#[cfg(test)]
mod tests {
    use super::*;
    fn l(sid: &str, at: i64, kind: &str) -> String { format!(r#"{{"v":1,"session_id":"{sid}","project":"p-{sid}","cwd":"","at_ms":{at},"event":{{"kind":"{kind}"}}}}"#) }
    fn tool(sid: &str, at: i64) -> String { format!(r#"{{"v":1,"session_id":"{sid}","project":"p-{sid}","cwd":"","at_ms":{at},"event":{{"kind":"tool","tool":"Bash","target":"x"}}}}"#) }
    fn perm(sid: &str, at: i64) -> String { format!(r#"{{"v":1,"session_id":"{sid}","project":"p-{sid}","cwd":"","at_ms":{at},"event":{{"kind":"permission","tool":"Bash","command":"ls"}}}}"#) }
    fn res(sid: &str, at: i64) -> String { format!(r#"{{"v":1,"session_id":"{sid}","project":"p-{sid}","cwd":"","at_ms":{at},"event":{{"kind":"resolve","id":1,"decision":"allow"}}}}"#) }
    const H: i64 = 3_600_000;

    #[test] fn work_and_waiting_time() {
        let lines = vec![
            format!(r#"{{"v":1,"session_id":"a","project":"p-a","cwd":"","at_ms":{},"event":{{"kind":"start","prompt":"x"}}}}"#, 9 * H),
            tool("a", 9 * H + 60_000), perm("a", 9 * H + 600_000), res("a", 9 * H + 900_000), l("a", 10 * H, "done"),
        ];
        let d = summarize(lines.into_iter(), 0, 11 * H);
        let a = &d.sessions[0];
        assert_eq!(a.project, "p-a");
        assert_eq!(a.work_ms, H);
        assert_eq!(a.waited_ms, 300_000);
        assert_eq!(a.max_wait_ms, 300_000);
        assert_eq!((a.perms, a.actions), (1, 2));
        assert_eq!(d.actions_by_hour[9], 2);
        assert!(a.lanes.iter().any(|x| x.2 == 'p'));
    }
    #[test] fn open_work_counts_until_now_and_bad_lines_are_skipped() {
        let lines = vec![format!(r#"{{"v":1,"session_id":"b","project":"p-b","cwd":"","at_ms":{},"event":{{"kind":"start","prompt":"x"}}}}"#, 10 * H), "basura".to_string()];
        let d = summarize(lines.into_iter(), 0, 10 * H + 120_000);
        assert_eq!(d.sessions[0].work_ms, 120_000);
    }
    #[test] fn one_row_per_project_newest_first() {
        let lines = vec![l("a", H, "start"), l("a", 2 * H, "done"), l("b", 3 * H, "start"), l("b", 4 * H, "done"), l("c", 5 * H, "session-start"),
            // otra sesión del mismo proyecto que "a"
            format!(r#"{{"v":1,"session_id":"a2","project":"p-a","cwd":"","at_ms":{},"event":{{"kind":"start","prompt":"x"}}}}"#, 5 * H), l("a2", 6 * H, "done")];
        let d = summarize(lines.into_iter(), 0, 7 * H);
        // p-c solo se abrió: no tiene actividad y no se muestra
        assert_eq!(d.sessions.iter().map(|s| s.project.as_str()).collect::<Vec<_>>(), vec!["p-a", "p-b"]);
        assert_eq!(d.sessions[0].work_ms, 2 * H);
        assert_eq!(d.sessions[0].lanes.len(), 2);
    }
    #[test] fn abandoned_work_ends_at_the_last_event() {
        // empezó a las 10 y no volvió a dar señales: no cuenta como trabajando hasta las 18
        let lines = vec![format!(r#"{{"v":1,"session_id":"z","project":"p-z","cwd":"","at_ms":{},"event":{{"kind":"start","prompt":"x"}}}}"#, 10 * H), tool("z", 10 * H + 60_000)];
        let d = summarize(lines.into_iter(), 0, 18 * H);
        assert_eq!(d.sessions[0].work_ms, 60_000);
        assert!(d.sessions[0].lanes.iter().all(|x| x.1 < 0.6));
    }
    #[test] fn ignores_other_days() {
        let d = summarize(vec![l("c", 5, "done")].into_iter(), 1_000, 2_000);
        assert!(d.sessions.is_empty());
    }
}
