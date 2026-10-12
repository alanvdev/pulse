use tauri::{image::Image, tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent}, AppHandle, PhysicalPosition};
use crate::state::{Session, SessionState};
use std::sync::Mutex;
pub const TRAY_ID: &str = "pulse";

#[derive(Debug, Clone, Copy, PartialEq)] pub enum DotShape { Fill, Halo, Ring }
pub type TrayDot = (DotShape, [u8; 3]);
/// Colores para barra de tareas oscura y clara (spec §2.2): en barra clara, "trabaja" va en gris oscuro para que se vea.
struct Palette { tx: [u8; 3], perm: [u8; 3], done: [u8; 3], calm: [u8; 3] }
const DARK: Palette = Palette { tx: [234, 237, 234], perm: [227, 176, 92], done: [143, 195, 160], calm: [158, 163, 158] };
const LIGHT: Palette = Palette { tx: [31, 36, 32], perm: [154, 101, 18], done: [47, 110, 74], calm: [103, 109, 103] };

/// Un punto por sesión (hasta tres, en orden de llegada) con las formas de la isla (spec §5). Sin sesiones: un anillo gris.
pub fn tray_dots(sessions: &[Session], light: bool) -> Vec<TrayDot> {
    let c = if light { &LIGHT } else { &DARK };
    if sessions.is_empty() { return vec![(DotShape::Ring, c.calm)]; }
    let mut v: Vec<&Session> = sessions.iter().collect();
    v.sort_by_key(|s| s.slot);
    v.iter().take(3).map(|s| match s.state {
        SessionState::Working => (DotShape::Fill, c.tx),
        SessionState::Waiting => (DotShape::Halo, c.perm),
        SessionState::Done => (DotShape::Ring, c.done),
        SessionState::IdleWait | SessionState::Idle => (DotShape::Ring, c.calm),
    }).collect()
}

/// Dónde va cada punto según cuántos hay (centro x, centro y, radio, grosor del anillo; fracciones del ícono).
/// Uno solo es grande, como el anillo de antes; dos van lado a lado; tres en triángulo.
fn layout(n: usize) -> &'static [(f32, f32, f32, f32)] {
    match n {
        0 | 1 => &[(0.5, 0.5, 0.32, 0.10)],
        2 => &[(0.27, 0.5, 0.21, 0.09), (0.73, 0.5, 0.21, 0.09)],
        _ => &[(0.5, 0.29, 0.19, 0.09), (0.27, 0.71, 0.19, 0.09), (0.73, 0.71, 0.19, 0.09)],
    }
}

/// Dibuja los puntos en un ícono cuadrado de `size` px (RGBA, con borde suavizado).
pub fn dots_rgba(size: u32, dots: &[TrayDot]) -> Vec<u8> {
    let s = size as f32;
    let mut px = vec![0u8; (size * size * 4) as usize];
    for (&(fx, fy, fr, ft), (shape, rgb)) in layout(dots.len()).iter().zip(dots) {
        let (cx, cy, ring) = (fx * s, fy * s, ft * s);
        let r = fr * s * if *shape == DotShape::Halo { 0.8 } else { 1.0 };   // el halo ocupa el resto
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

/// Si la barra de tareas de Windows está en tema claro (Personalize › SystemUsesLightTheme).
pub fn taskbar_is_light() -> bool {
    use windows_sys::Win32::System::Registry::{RegGetValueW, HKEY_CURRENT_USER, RRF_RT_REG_DWORD};
    let w = |s: &str| s.encode_utf16().chain([0]).collect::<Vec<u16>>();
    let (key, name) = (w(r"Software\Microsoft\Windows\CurrentVersion\Themes\Personalize"), w("SystemUsesLightTheme"));
    let (mut v, mut sz) = (0u32, 4u32);
    unsafe { RegGetValueW(HKEY_CURRENT_USER, key.as_ptr(), name.as_ptr(), RRF_RT_REG_DWORD, std::ptr::null_mut(), &mut v as *mut _ as _, &mut sz) == 0 && v == 1 }
}
static LIGHT_BAR: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
/// Lee otra vez el tema de la barra; `true` si cambió desde la última vez (hay que redibujar el ícono).
pub fn theme_changed() -> bool { let now = taskbar_is_light(); LIGHT_BAR.swap(now, std::sync::atomic::Ordering::SeqCst) != now }
fn icon(dots: &[TrayDot]) -> Image<'static> { Image::new_owned(dots_rgba(32, dots), 32, 32) }
/// Último ícono puesto: no se redibuja si no cambió (broadcast llega con cada evento).
static LAST: Mutex<Vec<TrayDot>> = Mutex::new(Vec::new());
/// Clic izquierdo: Bolsillo. Clic derecho: el flyout (no hay menú nativo).
pub fn create(app: &AppHandle, on_left_click: fn(&AppHandle), on_right_click: fn(&AppHandle, PhysicalPosition<f64>)) -> tauri::Result<()> {
    TrayIconBuilder::with_id(TRAY_ID).icon(icon(&tray_dots(&[], { theme_changed(); LIGHT_BAR.load(std::sync::atomic::Ordering::SeqCst) }))).tooltip("Pulse · sin sesiones")
        .on_tray_icon_event(move |t, e| if let TrayIconEvent::Click { button, button_state: MouseButtonState::Up, position, rect, .. } = e {
            // el flyout se centra sobre el ícono (no sobre el cursor, que puede estar en otro lado si se abrió con el teclado)
            let (p, s) = (rect.position.to_physical::<f64>(1.0), rect.size.to_physical::<f64>(1.0));
            let anchor = if s.width > 0.0 { PhysicalPosition::new(p.x + s.width / 2.0, p.y) } else { position };
            match button { MouseButton::Left => on_left_click(t.app_handle()), MouseButton::Right => on_right_click(t.app_handle(), anchor), _ => {} }
        })
        .build(app)?;
    Ok(())
}
pub fn update(app: &AppHandle, sessions: &[Session], tip: &str) {
    let Some(t) = app.tray_by_id(TRAY_ID) else { return };
    let dots = tray_dots(sessions, LIGHT_BAR.load(std::sync::atomic::Ordering::SeqCst));
    let changed = { let mut last = LAST.lock().unwrap(); if *last != dots { *last = dots.clone(); true } else { false } };
    if changed { let _ = t.set_icon(Some(icon(&dots))); }
    let _ = t.set_tooltip(Some(tip));
}
#[cfg(test)]
mod tests { use super::*; use crate::state::{Session, SessionState};
    fn s(slot: u8, state: SessionState) -> Session { Session { id: format!("s{slot}"), project: "p".into(), cwd: String::new(), slot, state, last: String::new(), subagents: 0, since_ms: 0, last_event_ms: 0, actions: 0, muted_until_ms: 0 } }
    fn at(px: &[u8], x: usize, y: usize) -> Vec<u8> { px[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4].to_vec() }
    #[test] fn no_sessions_is_one_big_gray_ring() {
        let d = tray_dots(&[], false);
        assert_eq!(d, vec![(DotShape::Ring, DARK.calm)]);
        let px = dots_rgba(32, &d);
        assert_eq!(at(&px, 16, 16)[3], 0);     // hueco al centro
        assert_eq!(at(&px, 25, 16)[3], 255);   // borde del anillo (radio 10 px, como el anillo de antes)
    }
    #[test] fn one_session_is_one_big_dot() {
        let px = dots_rgba(32, &tray_dots(&[s(0, SessionState::Working)], false));
        assert_eq!(at(&px, 16, 16), vec![234, 237, 234, 255]);
        assert_eq!(at(&px, 24, 16)[3], 255);
    }
    #[test] fn three_sessions_in_a_triangle_in_slot_order() {
        let d = tray_dots(&[s(1, SessionState::Done), s(0, SessionState::Waiting), s(2, SessionState::Working), s(3, SessionState::Idle)], false);
        assert_eq!(d, vec![(DotShape::Halo, DARK.perm), (DotShape::Ring, DARK.done), (DotShape::Fill, DARK.tx)]);
        let px = dots_rgba(32, &d);
        assert_eq!(at(&px, 16, 9), vec![227, 176, 92, 255]);   // arriba: te necesita
        assert_eq!(at(&px, 8, 22)[3], 0);                      // abajo a la izquierda: anillo (hueco)
        assert_eq!(at(&px, 23, 22), vec![234, 237, 234, 255]); // abajo a la derecha: trabaja
    }
    #[test] fn calm_states_are_gray_rings_and_light_bar_uses_dark_ink() {
        assert_eq!(tray_dots(&[s(0, SessionState::IdleWait)], false), vec![(DotShape::Ring, DARK.calm)]);
        assert_eq!(tray_dots(&[s(0, SessionState::Idle)], false), vec![(DotShape::Ring, DARK.calm)]);
        assert_eq!(tray_dots(&[s(0, SessionState::Working)], true), vec![(DotShape::Fill, LIGHT.tx)]);
    }
}
