use crate::AppState;
use std::sync::{Mutex, OnceLock};
use tauri::{AppHandle, Manager};

const ROWS: usize = 6; const COLS: usize = 22;
pub struct KeyFx { pub shape: String, pub color: [u8; 3], pub intensity: f32, pub start_ms: i64, pub dur_ms: i64, pub ox: f32, pub oy: f32, pub left: bool }
static URI: OnceLock<Mutex<Option<String>>> = OnceLock::new();
static FX: OnceLock<Mutex<Vec<KeyFx>>> = OnceLock::new();
fn uri() -> &'static Mutex<Option<String>> { URI.get_or_init(|| Mutex::new(None)) }
fn fxs() -> &'static Mutex<Vec<KeyFx>> { FX.get_or_init(|| Mutex::new(Vec::new())) }

pub fn bgr(c: [u8; 3]) -> u32 { (c[2] as u32) << 16 | (c[1] as u32) << 8 | c[0] as u32 }
fn lerp(pts: &[(f32, f32)], p: f32) -> f32 { for w in pts.windows(2) { if p <= w[1].0 { return w[0].1 + (w[1].1 - w[0].1) * (p - w[0].0) / (w[1].0 - w[0].0); } } 0.0 }

/// Port de `fxAt` del diseño original. x, y, ox, oy en 0..1; p progreso 0..1.
#[allow(clippy::too_many_arguments)]
pub fn fx_at(shape: &str, x: f32, y: f32, ox: f32, oy: f32, left: bool, i: f32, p: f32) -> f32 {
    match shape {
        "latido" => i * lerp(&[(0.0, 0.0), (0.12, 1.0), (0.32, 0.1), (0.46, 0.9), (1.0, 0.0)], p),
        "lateral" => if (left && x < 0.16) || (!left && x > 0.84) { i * lerp(&[(0.0, 0.0), (0.1, 1.0), (0.25, 0.2), (0.4, 1.0), (1.0, 0.0)], p) } else { 0.0 },
        "ola" => { let d = if left { x } else { 1.0 - x }; let fr = -0.1 + p * 1.25; let fd = if p < 0.15 { p / 0.15 } else if p > 0.85 { (1.0 - p) / 0.15 } else { 1.0 }; i * (-((d - fr) / 0.08).powi(2)).exp() * fd }
        "barrido" => i * (-((y - (1.15 - 1.3 * p)) / 0.16).powi(2)).exp(),
        "origen" => { let d = ((x - ox).powi(2) + ((y - oy) * 0.38).powi(2)).sqrt(); i * (-((d - p * 1.15) / 0.05).powi(2)).exp() * (1.0 - p).powf(0.6) + if d < 0.03 && p < 0.3 { i * (1.0 - p / 0.3) } else { 0.0 } }
        _ => i * if p < 0.35 { p / 0.35 } else { (1.0 - p) / 0.65 },
    }
}

pub fn grid(base: [u8; 3], level: f32, fx: &[KeyFx], waiting: &[(u8, [u8; 3])], session_keys: bool, now_ms: i64) -> [[u32; COLS]; ROWS] {
    let mut g = [[0u32; COLS]; ROWS];
    let breath = 0.5 + 0.5 * ((now_ms as f32) / 600.0).cos();   // 1 en t = 0: la tecla de la sesión arranca encendida a pleno
    for r in 0..ROWS { for c in 0..COLS {
        let (x, y) = (c as f32 / (COLS - 1) as f32, r as f32 / (ROWS - 1) as f32);
        let mut best = (level, base);
        for f in fx { let p = ((now_ms - f.start_ms) as f32 / f.dur_ms as f32).clamp(0.0, 1.0); let v = fx_at(&f.shape, x, y, f.ox, f.oy, f.left, f.intensity, p); if v > best.0 { best = (v, f.color); } }
        if session_keys && r == 0 { for (slot, col) in waiting { if c == 3 + *slot as usize { let v = 0.35 + 0.65 * breath; if v > best.0 { best = (v, *col); } } } }
        let k = best.0.clamp(0.0, 1.0);
        g[r][c] = if k <= 0.0 { 0 } else { bgr([(best.1[0] as f32 * k) as u8, (best.1[1] as f32 * k) as u8, (best.1[2] as f32 * k) as u8]) };
    } }
    g
}

fn put(path: &str, body: serde_json::Value) { if let Some(u) = uri().lock().unwrap().clone() { let _ = ureq::put(&format!("{u}{path}")).send_json(body); } }

pub fn start(app: &AppHandle) {
    if !app.state::<AppState>().config.lock().unwrap().keyboard.enabled { return; }
    let a = app.clone();
    std::thread::spawn(move || {
        let init = serde_json::json!({ "title": "Pulse", "description": "Avisos de Claude Code", "author": { "name": "Pulse", "contact": "local" }, "device_supported": ["keyboard"], "category": "application" });
        let Ok(resp) = ureq::post("http://localhost:54235/razer/chromasdk").send_json(init) else { return };   // Chroma no instalado: no hacer nada
        let Ok(v) = resp.into_json::<serde_json::Value>() else { return };
        *uri().lock().unwrap() = v["uri"].as_str().map(str::to_string);
        std::thread::sleep(std::time::Duration::from_millis(500));   // Chroma necesita un momento tras el init
        let mut last_beat = std::time::Instant::now();
        loop {
            let st = a.state::<AppState>();
            let kb = st.config.lock().unwrap().keyboard.clone();
            let snap = st.store.lock().unwrap().snapshot();
            let now = crate::now_ms();
            fxs().lock().unwrap().retain(|f| now - f.start_ms < f.dur_ms);
            let waiting: Vec<(u8, [u8; 3])> = snap.sessions.iter().filter_map(|s| match s.state {
                crate::state::SessionState::Waiting => Some((s.slot, [255, 176, 46])), crate::state::SessionState::IdleWait => Some((s.slot, [182, 192, 212])), _ => None }).collect();
            let g = if kb.enabled { grid([120, 140, 175], kb.base as f32 / 200.0, &fxs().lock().unwrap(), &waiting, kb.session_keys, now) } else { [[0; COLS]; ROWS] };
            put("/keyboard", serde_json::json!({ "effect": "CHROMA_CUSTOM", "param": g }));
            if last_beat.elapsed().as_secs() >= 10 { put("/heartbeat", serde_json::json!({})); last_beat = std::time::Instant::now(); }
            let busy = !fxs().lock().unwrap().is_empty() || !waiting.is_empty();
            std::thread::sleep(std::time::Duration::from_millis(if busy { 33 } else { 1000 }));
        }
    });
}

pub fn pulse(app: &AppHandle, kind: &str, slot: u8) {
    let cfg = app.state::<AppState>().config.lock().unwrap().clone();
    if !cfg.keyboard.enabled { return; }
    let Some(p) = cfg.pulses.events.get(kind) else { return };
    let hex = u32::from_str_radix(p.color.trim_start_matches('#'), 16).unwrap_or(0xffffff);
    let ox = if cfg.keyboard.session_keys { (3 + slot as usize) as f32 / (COLS - 1) as f32 } else { [0.18, 0.5, 0.82][slot as usize % 3] };
    fxs().lock().unwrap().push(KeyFx { shape: p.shape.clone(), color: [(hex >> 16) as u8, (hex >> 8) as u8, hex as u8], intensity: p.intensity as f32 / 100.0, start_ms: crate::now_ms(), dur_ms: p.duration_ms.max(700) as i64, ox, oy: if cfg.keyboard.session_keys { 0.0 } else { 0.5 }, left: ox < 0.5 });
}

pub fn shutdown() { if let Some(u) = uri().lock().unwrap().take() { let _ = ureq::delete(&u).call(); } }

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn bgr_packing() { assert_eq!(bgr([255, 176, 46]), 0x2EB0FF); }
    #[test] fn session_keys_light_f1_to_f3() {
        let g = grid([120, 140, 175], 0.0, &[], &[(1, [255, 176, 46])], true, 0);
        assert_eq!(g[0][4], bgr([255, 176, 46]) & 0xFFFFFF, "F2 = slot 1, fila 0 columna 4");
        assert_eq!(g[0][3], 0);
    }
    #[test] fn latido_has_two_peaks() {
        let v: Vec<f32> = (0..=20).map(|i| fx_at("latido", 0.5, 0.5, 0.5, 0.5, true, 1.0, i as f32 / 20.0)).collect();
        let peaks = v.windows(3).filter(|w| w[1] > w[0] && w[1] > w[2] && w[1] > 0.6).count();
        assert_eq!(peaks, 2);
    }
}
