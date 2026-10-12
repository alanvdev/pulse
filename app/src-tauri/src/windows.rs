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
        no_activate(&w);
    }
    crate::windows::create_island(app)?;        // Task 4
    Ok(())
}

/// Las ventanas de borde nunca deben quedar como ventana activa (ni robar el foco ni contar como "pantalla completa").
fn no_activate(w: &tauri::WebviewWindow) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE};
    if let Ok(h) = w.hwnd() {
        let h = h.0 as windows_sys::Win32::Foundation::HWND;
        unsafe { SetWindowLongPtrW(h, GWL_EXSTYLE, GetWindowLongPtrW(h, GWL_EXSTYLE) | WS_EX_NOACTIVATE as isize); }
    }
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
    // Alto de sobra para tarjetas de preguntas largas: la zona sin contenido es transparente y no recibe clics.
    let (w, h) = (size.width as f64 / scale, (size.height as f64 / scale * 0.8).min(860.0));
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
        // recuerda la última ventana activa que no es de Pulse, para devolverle el teclado
        let fg = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() } as isize;
        if fg != 0 && !is_ours(fg) { PREV_FOREGROUND.store(fg, std::sync::atomic::Ordering::SeqCst); }
    } });
    Ok(())
}
/// Área de trabajo del monitor principal (sin la barra de tareas), en píxeles físicos: (izq, arriba, der, abajo).
fn work_area() -> (f64, f64, f64, f64) {
    use windows_sys::Win32::{Foundation::RECT, UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA}};
    let mut r: RECT = unsafe { std::mem::zeroed() };
    unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut r as *mut _ as _, 0) };
    (r.left as f64, r.top as f64, r.right as f64, r.bottom as f64)
}
/// Momento en que el panel se ocultó por perder el foco: el clic en la bandeja que lo provocó no debe volver a abrirlo.
static PANEL_HIDDEN_AT: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
/// Ventana que tenía el teclado antes de que la isla lo tomara (paleta, "Otra…").
static PREV_FOREGROUND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
fn is_ours(h: isize) -> bool {
    let mut pid = 0u32;
    unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(h as _, &mut pid) };
    pid == std::process::id()
}
/// La isla toma el teclado (`on`) o se lo devuelve a la ventana donde estabas.
pub fn island_keyboard(app: &AppHandle, on: bool) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, IsWindow, SetForegroundWindow};
    use std::sync::atomic::Ordering;
    let Some(w) = app.get_webview_window("island") else { return };
    let island = w.hwnd().map(|h| h.0 as isize).unwrap_or(0);
    let _ = w.set_ignore_cursor_events(!on);
    unsafe {
        let fg = GetForegroundWindow() as isize;
        if on {
            if fg != 0 && !is_ours(fg) { PREV_FOREGROUND.store(fg, Ordering::SeqCst); }
            let _ = w.set_focus();
        } else if fg == island {
            let prev = PREV_FOREGROUND.load(Ordering::SeqCst);
            if prev != 0 && IsWindow(prev as _) != 0 { SetForegroundWindow(prev as _); }
        }
    }
}

pub fn toggle_panel(app: &AppHandle) {
    let win = match app.get_webview_window("panel") { Some(w) => w, None => match create_panel(app) { Ok(w) => w, Err(_) => return } };
    if win.is_visible().unwrap_or(false) { let _ = win.hide(); return; }
    if crate::now_ms() - PANEL_HIDDEN_AT.load(std::sync::atomic::Ordering::SeqCst) < 300 { return; }
    let scale = win.scale_factor().unwrap_or(1.0);
    let (_, _, r, b) = work_area();
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
/// Abre Mosaico; `view` es la ruta dentro de Mosaico ("ajustes", "sesiones/<id>", ...).
pub fn open_dashboard(app: &AppHandle, view: Option<&str>) {
    use tauri::Emitter;
    if let Some(w) = app.get_webview_window("dashboard") {
        if let Some(v) = view { let _ = app.emit_to("dashboard", "pulse://route", v); }
        let _ = w.unminimize(); let _ = w.show(); let _ = w.set_focus(); return;
    }
    let route = serde_json::to_string(view.unwrap_or("")).unwrap_or_else(|_| "\"\"".into());
    let _ = WebviewWindowBuilder::new(app, "dashboard", WebviewUrl::App("dashboard.html".into()))
        .initialization_script(&format!("window.__pulseRoute = {route};"))
        .title("Pulse").inner_size(1120.0, 760.0).min_inner_size(720.0, 520.0).center().build();
}

/// Flyout del clic derecho: 300 px de menú con 12 px de margen transparente alrededor para su sombra.
pub const FLYOUT_W: f64 = 324.0;
static FLYOUT_HIDDEN_AT: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
/// X lógica del último clic derecho en la bandeja: el flyout se centra sobre ella.
static FLYOUT_CLICK_X: Mutex<f64> = Mutex::new(0.0);

/// Esquina superior izquierda del flyout (lógica): centrado sobre el clic, encima de la barra de tareas y dentro del área de trabajo.
pub fn flyout_origin(click_x: f64, size: (f64, f64), work: (f64, f64, f64, f64)) -> (f64, f64) {
    let ((w, h), (l, t, r, b)) = (size, work);
    ((click_x - w / 2.0).clamp(l, (r - w).max(l)), (b - h).max(t))
}
/// Se crea al arrancar, oculto, para que el primer clic derecho sea instantáneo.
pub fn create_flyout(app: &AppHandle) -> tauri::Result<()> {
    let w = WebviewWindowBuilder::new(app, "flyout", WebviewUrl::App("flyout.html".into()))
        .transparent(true).decorations(false).always_on_top(true).skip_taskbar(true).resizable(false).shadow(false)
        .inner_size(FLYOUT_W, 420.0).visible(false).build()?;
    let w2 = w.clone();
    w.on_window_event(move |e| if let tauri::WindowEvent::Focused(false) = e {
        if w2.is_visible().unwrap_or(false) { FLYOUT_HIDDEN_AT.store(crate::now_ms(), std::sync::atomic::Ordering::SeqCst); }
        let _ = w2.hide();
    });
    Ok(())
}
fn place_flyout(win: &tauri::WebviewWindow) {
    let scale = win.scale_factor().unwrap_or(1.0);
    let Ok(size) = win.inner_size() else { return };
    let (l, t, r, b) = work_area();
    let (x, y) = flyout_origin(*FLYOUT_CLICK_X.lock().unwrap(), (size.width as f64 / scale, size.height as f64 / scale), (l / scale, t / scale, r / scale, b / scale));
    let _ = win.set_position(tauri::LogicalPosition::new(x, y));
}
pub fn toggle_flyout(app: &AppHandle, click: tauri::PhysicalPosition<f64>) {
    use tauri::Emitter;
    let Some(win) = app.get_webview_window("flyout") else { return };
    if win.is_visible().unwrap_or(false) { let _ = win.hide(); return; }
    // el clic en la bandeja que le quitó el foco no debe volver a abrirlo
    if crate::now_ms() - FLYOUT_HIDDEN_AT.load(std::sync::atomic::Ordering::SeqCst) < 300 { return; }
    *FLYOUT_CLICK_X.lock().unwrap() = click.x / win.scale_factor().unwrap_or(1.0);
    place_flyout(&win);
    let _ = app.emit_to("flyout", "pulse://flyout-open", ());
    let _ = win.show(); let _ = win.set_focus();
}
/// La interfaz avisa su alto (cambia con la fila "Responder permiso"); se recoloca para que el borde de abajo no se mueva.
pub fn fit_flyout(app: &AppHandle, height: f64) {
    let Some(win) = app.get_webview_window("flyout") else { return };
    let _ = win.set_size(tauri::LogicalSize::new(FLYOUT_W, height.ceil()));
    place_flyout(&win);
}
pub fn hide_flyout(app: &AppHandle) { if let Some(w) = app.get_webview_window("flyout") { let _ = w.hide(); } }

#[cfg(test)]
mod tests { use super::*;
    #[test] fn flyout_sits_above_the_click_inside_the_work_area() {
        let work = (0.0, 0.0, 1920.0, 1032.0);
        assert_eq!(flyout_origin(1700.0, (324.0, 400.0), work), (1538.0, 632.0));   // centrado sobre el clic
        assert_eq!(flyout_origin(1900.0, (324.0, 400.0), work), (1596.0, 632.0));   // no se sale por la derecha
        assert_eq!(flyout_origin(40.0, (324.0, 400.0), work), (0.0, 632.0));        // ni por la izquierda
        assert_eq!(flyout_origin(500.0, (324.0, 2000.0), work), (338.0, 0.0));      // ni por arriba
    }
}
