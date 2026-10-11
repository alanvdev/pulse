use std::sync::Mutex;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub fn create_all(app: &AppHandle) -> tauri::Result<()> {
    for (i, m) in app.available_monitors()?.iter().enumerate() {
        let (pos, size, scale) = (m.position(), m.size(), m.scale_factor());
        let w = WebviewWindowBuilder::new(app, format!("overlay-{i}"), WebviewUrl::App(format!("overlay.html?m={i}").into()))
            .transparent(true).decorations(false).always_on_top(true).skip_taskbar(true).focused(false)
            .resizable(false).shadow(false).visible(true)
            .position(pos.x as f64 / scale, pos.y as f64 / scale)
            .inner_size(size.width as f64 / scale, size.height as f64 / scale)
            .build()?;
        w.set_ignore_cursor_events(true)?;
    }
    crate::windows::create_island(app)?;        // Task 4
    Ok(())
}

/// Monitor donde está la ventana en primer plano (para "monitor activo").
pub fn active_monitor_index(app: &AppHandle) -> usize {
    use windows_sys::Win32::{Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST}, UI::WindowsAndMessaging::GetForegroundWindow};
    let Ok(mons) = app.available_monitors() else { return 0 };
    unsafe {
        let hmon = MonitorFromWindow(GetForegroundWindow(), MONITOR_DEFAULTTONEAREST);
        let mut mi: MONITORINFO = std::mem::zeroed(); mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(hmon, &mut mi) == 0 { return 0; }
        mons.iter().position(|m| m.position().x == mi.rcMonitor.left && m.position().y == mi.rcMonitor.top).unwrap_or(0)
    }
}

#[derive(Default)] pub struct Hitbox(pub Mutex<Option<(f64, f64, f64, f64)>>);

/// Isla: franja transparente arriba del monitor principal (todo el ancho, para que el Orbe pueda ir a la izquierda, centro o derecha).
/// No recibe clics salvo dentro del rectángulo que reporta la interfaz.
pub fn create_island(app: &AppHandle) -> tauri::Result<()> {
    let m = app.primary_monitor()?.ok_or(tauri::Error::WindowNotFound)?;
    let (scale, pos, size) = (m.scale_factor(), m.position(), m.size());
    let (w, h) = (size.width as f64 / scale, 480.0);
    let win = WebviewWindowBuilder::new(app, "island", WebviewUrl::App("island.html".into()))
        .transparent(true).decorations(false).always_on_top(true).skip_taskbar(true).focused(false)
        .resizable(false).shadow(false).position(pos.x as f64 / scale, pos.y as f64 / scale).inner_size(w, h).build()?;
    win.set_ignore_cursor_events(true)?;
    app.manage(Hitbox::default());
    let a = app.clone();
    tauri::async_runtime::spawn(async move { let mut last = true; loop {
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;
        let Some(win) = a.get_webview_window("island") else { continue };
        let hb = *a.state::<Hitbox>().0.lock().unwrap();   // soltar el lock antes de llamar al hilo principal
        let ignore = match (hb, a.cursor_position(), win.outer_position(), win.scale_factor()) {
            (Some((x, y, w, h)), Ok(c), Ok(o), Ok(s)) => { let (cx, cy) = ((c.x - o.x as f64) / s, (c.y - o.y as f64) / s); !(cx >= x && cx <= x + w && cy >= y && cy <= y + h) }
            _ => true,
        };
        if ignore != last { let _ = win.set_ignore_cursor_events(ignore); last = ignore; }
    } });
    Ok(())
}
pub fn toggle_panel(_: &AppHandle) {}
pub fn open_dashboard(_: &AppHandle) {}
