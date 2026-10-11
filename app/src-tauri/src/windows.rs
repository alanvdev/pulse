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
fn work_area_right_bottom() -> (f64, f64) {
    use windows_sys::Win32::{Foundation::RECT, UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA}};
    let mut r: RECT = unsafe { std::mem::zeroed() };
    unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut r as *mut _ as _, 0) };
    (r.right as f64, r.bottom as f64)
}
/// Momento en que el panel se ocultó por perder el foco: el clic en la bandeja que lo provocó no debe volver a abrirlo.
static PANEL_HIDDEN_AT: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
pub fn toggle_panel(app: &AppHandle) {
    let win = match app.get_webview_window("panel") { Some(w) => w, None => match create_panel(app) { Ok(w) => w, Err(_) => return } };
    if win.is_visible().unwrap_or(false) { let _ = win.hide(); return; }
    if crate::now_ms() - PANEL_HIDDEN_AT.load(std::sync::atomic::Ordering::SeqCst) < 300 { return; }
    let scale = win.scale_factor().unwrap_or(1.0);
    let (r, b) = work_area_right_bottom();
    let _ = win.set_position(tauri::LogicalPosition::new(r / scale - 380.0 - 12.0, b / scale - 540.0 - 12.0));
    let _ = win.show(); let _ = win.set_focus();
}
fn create_panel(app: &AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    let w = WebviewWindowBuilder::new(app, "panel", WebviewUrl::App("panel.html".into()))
        .transparent(true).decorations(false).always_on_top(true).skip_taskbar(true).resizable(false).shadow(false)
        .inner_size(380.0, 540.0).visible(false).build()?;
    let w2 = w.clone();
    w.on_window_event(move |e| if let tauri::WindowEvent::Focused(false) = e { if w2.is_visible().unwrap_or(false) { PANEL_HIDDEN_AT.store(crate::now_ms(), std::sync::atomic::Ordering::SeqCst); } let _ = w2.hide(); });
    Ok(w)
}
pub fn open_dashboard(_: &AppHandle) {}
