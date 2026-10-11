use crate::state::SessionState;
use tauri::{image::Image, menu::{MenuBuilder, MenuItemBuilder}, tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent}, AppHandle};
pub const TRAY_ID: &str = "pulse";
pub fn state_color(s: SessionState) -> [u8; 3] { match s { SessionState::Idle => [105,113,124], SessionState::Done => [79,224,160], SessionState::Working => [230,238,255], SessionState::IdleWait => [182,192,212], SessionState::Waiting => [255,176,46] } }
pub fn ring_rgba(size: u32, rgb: [u8; 3]) -> Vec<u8> {
    let s = size as f32; let (ro, ri) = (s * 0.41, s * 0.25);
    let mut px = vec![0u8; (size * size * 4) as usize];
    for y in 0..size { for x in 0..size {
        let (dx, dy) = (x as f32 + 0.5 - s / 2.0, y as f32 + 0.5 - s / 2.0); let r = (dx * dx + dy * dy).sqrt();
        let a = ((r - ri).min(ro - r) + 0.5).clamp(0.0, 1.0); let o = ((y * size + x) * 4) as usize;
        px[o..o + 3].copy_from_slice(&rgb); px[o + 3] = (a * 255.0) as u8; } }
    px
}
fn icon(s: SessionState) -> Image<'static> { Image::new_owned(ring_rgba(32, state_color(s)), 32, 32) }
/// `on_left_click` abre/cierra Bolsillo (se conecta en la Task 5).
pub fn create(app: &AppHandle, on_left_click: fn(&AppHandle)) -> tauri::Result<()> {
    let open = MenuItemBuilder::with_id("dashboard", "Abrir tablero").build(app)?;
    let quit = MenuItemBuilder::with_id("quit", "Salir de Pulse").build(app)?;
    let menu = MenuBuilder::new(app).items(&[&open, &quit]).build()?;
    TrayIconBuilder::with_id(TRAY_ID).icon(icon(SessionState::Idle)).tooltip("Pulse · sin sesiones").menu(&menu).show_menu_on_left_click(false)
        .on_menu_event(|app, e| match e.id().as_ref() { "quit" => app.exit(0), "dashboard" => crate::windows::open_dashboard(app), _ => {} })
        .on_tray_icon_event(move |t, e| if let TrayIconEvent::Click { button: MouseButton::Left, button_state: MouseButtonState::Up, .. } = e { on_left_click(t.app_handle()) })
        .build(app)?;
    Ok(())
}
pub fn update(app: &AppHandle, s: SessionState, tip: &str) { if let Some(t) = app.tray_by_id(TRAY_ID) { let _ = t.set_icon(Some(icon(s))); let _ = t.set_tooltip(Some(tip)); } }
#[cfg(test)]
mod tests { use super::*;
    #[test] fn ring_has_hole_and_colored_edge() {
        let px = ring_rgba(32, [255, 176, 46]); let at = |x: usize, y: usize| &px[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4];
        assert_eq!(at(16, 16)[3], 0); assert_eq!(&at(16, 4)[..3], &[255, 176, 46]); assert!(at(16, 4)[3] > 200);
    } }
