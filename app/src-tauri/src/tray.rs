use tauri::{image::Image, tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent}, AppHandle, PhysicalPosition};
use crate::state::{Session, SessionState};
use std::sync::Mutex;
pub const TRAY_ID: &str = "pulse";

#[derive(Debug, Clone, Copy, PartialEq)] pub enum DotShape { Fill, Halo, Ring }
pub type TrayDot = (DotShape, [u8; 3]);
const TX: [u8; 3] = [234, 237, 234];
const PERM: [u8; 3] = [227, 176, 92];
const DONE: [u8; 3] = [143, 195, 160];
const CALM: [u8; 3] = [158, 163, 158];

/// Un punto por sesión (hasta tres, en orden de llegada) con las formas de la isla (spec §5). Sin sesiones: un anillo gris.
pub fn tray_dots(sessions: &[Session]) -> Vec<TrayDot> {
    if sessions.is_empty() { return vec![(DotShape::Ring, CALM)]; }
    let mut v: Vec<&Session> = sessions.iter().collect();
    v.sort_by_key(|s| s.slot);
    v.iter().take(3).map(|s| match s.state {
        SessionState::Working => (DotShape::Fill, TX),
        SessionState::Waiting => (DotShape::Halo, PERM),
        SessionState::Done => (DotShape::Ring, DONE),
        SessionState::IdleWait | SessionState::Idle => (DotShape::Ring, CALM),
    }).collect()
}

/// Dibuja los puntos centrados en un ícono cuadrado de `size` px (RGBA, con borde suavizado).
pub fn dots_rgba(size: u32, dots: &[TrayDot]) -> Vec<u8> {
    let s = size as f32; let n = dots.len().max(1) as f32;
    let (r, gap, ring) = (s * 0.15, s * 0.33, s * 0.06);   // en 32 px: radio 4.8, centros a 10.6, anillo de 1.9
    let x0 = s / 2.0 - gap * (n - 1.0) / 2.0;
    let mut px = vec![0u8; (size * size * 4) as usize];
    for (i, (shape, rgb)) in dots.iter().enumerate() {
        let (cx, cy) = (x0 + gap * i as f32, s / 2.0);
        for y in 0..size { for x in 0..size {
            let d = ((x as f32 + 0.5 - cx).powi(2) + (y as f32 + 0.5 - cy).powi(2)).sqrt();
            let a = match shape {
                DotShape::Fill => (r - d + 0.5).clamp(0.0, 1.0),
                DotShape::Halo => (r - d + 0.5).clamp(0.0, 1.0).max(0.35 * (r * 1.45 - d + 0.5).clamp(0.0, 1.0)),
                DotShape::Ring => ((d - (r - ring)).min(r - d) + 0.5).clamp(0.0, 1.0),
            };
            let o = ((y * size + x) * 4) as usize;
            if a > 0.0 && a * 255.0 > px[o + 3] as f32 { px[o..o + 3].copy_from_slice(rgb); px[o + 3] = (a * 255.0) as u8; }
        } }
    }
    px
}
fn icon(dots: &[TrayDot]) -> Image<'static> { Image::new_owned(dots_rgba(32, dots), 32, 32) }
/// Último ícono puesto: no se redibuja si no cambió (broadcast llega con cada evento).
static LAST: Mutex<Vec<TrayDot>> = Mutex::new(Vec::new());
/// Clic izquierdo: Bolsillo. Clic derecho: el flyout (no hay menú nativo).
pub fn create(app: &AppHandle, on_left_click: fn(&AppHandle), on_right_click: fn(&AppHandle, PhysicalPosition<f64>)) -> tauri::Result<()> {
    TrayIconBuilder::with_id(TRAY_ID).icon(icon(&tray_dots(&[]))).tooltip("Pulse · sin sesiones")
        .on_tray_icon_event(move |t, e| if let TrayIconEvent::Click { button, button_state: MouseButtonState::Up, position, .. } = e {
            match button { MouseButton::Left => on_left_click(t.app_handle()), MouseButton::Right => on_right_click(t.app_handle(), position), _ => {} }
        })
        .build(app)?;
    Ok(())
}
pub fn update(app: &AppHandle, sessions: &[Session], tip: &str) {
    let Some(t) = app.tray_by_id(TRAY_ID) else { return };
    let dots = tray_dots(sessions);
    let changed = { let mut last = LAST.lock().unwrap(); if *last != dots { *last = dots.clone(); true } else { false } };
    if changed { let _ = t.set_icon(Some(icon(&dots))); }
    let _ = t.set_tooltip(Some(tip));
}
#[cfg(test)]
mod tests { use super::*; use crate::state::{Session, SessionState};
    fn s(slot: u8, state: SessionState) -> Session { Session { id: format!("s{slot}"), project: "p".into(), cwd: String::new(), slot, state, last: String::new(), subagents: 0, since_ms: 0, last_event_ms: 0, actions: 0, muted_until_ms: 0 } }
    #[test] fn no_sessions_is_one_gray_ring() {
        let d = tray_dots(&[]);
        assert_eq!(d, vec![(DotShape::Ring, CALM)]);
        let px = dots_rgba(32, &d); let a = |x: usize, y: usize| px[(y * 32 + x) * 4 + 3];
        assert_eq!(a(16, 16), 0);          // hueco al centro
        assert_eq!(a(19, 16), 255);        // borde del anillo
    }
    #[test] fn one_dot_per_session_in_slot_order_up_to_three() {
        let d = tray_dots(&[s(1, SessionState::Done), s(0, SessionState::Waiting), s(2, SessionState::Working), s(3, SessionState::Idle)]);
        assert_eq!(d, vec![(DotShape::Halo, PERM), (DotShape::Ring, DONE), (DotShape::Fill, TX)]);
        let px = dots_rgba(32, &d); let at = |x: usize, y: usize| px[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4].to_vec();
        // centros en x ≈ 5.4, 16 y 26.6
        assert_eq!(at(5, 16), vec![227, 176, 92, 255]);
        assert_eq!(at(16, 16)[3], 0);
        assert_eq!(at(26, 16), vec![234, 237, 234, 255]);
    }
    #[test] fn calm_states_are_gray_rings() {
        assert_eq!(tray_dots(&[s(0, SessionState::IdleWait)]), vec![(DotShape::Ring, CALM)]);
        assert_eq!(tray_dots(&[s(0, SessionState::Idle)]), vec![(DotShape::Ring, CALM)]);
    }
}
