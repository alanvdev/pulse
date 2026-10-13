use std::sync::Mutex;
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub fn create_all(app: &AppHandle) -> tauri::Result<()> {
    for (i, m) in app.available_monitors()?.iter().enumerate() {
        let (pos, size, scale) = (m.position(), m.size(), m.scale_factor());
        let w = WebviewWindowBuilder::new(app, format!("overlay-{i}"), WebviewUrl::App(format!("overlay.html?m={i}").into()))
            .transparent(true).decorations(false).always_on_top(true).skip_taskbar(true).focused(false)
            .resizable(false).shadow(false).visible(cfg!(windows))
            .position(pos.x as f64 / scale, pos.y as f64 / scale)
            .inner_size(size.width as f64 / scale, size.height as f64 / scale)
            .build()?;
        #[cfg(not(windows))] as_dock(&w)?;
        w.set_ignore_cursor_events(true)?;
        no_activate(&w);
    }
    crate::windows::create_island(app)?;        // Task 4
    Ok(())
}

/// Linux: GNOME no deja una ventana normal del tamaño del monitor donde no cabe (la barra superior y el dock le quitan
/// espacio) y la manda a otro monitor. Las de tipo "dock" se quedan donde se ponen. El tipo se fija antes de mostrarla.
#[cfg(not(windows))]
fn as_dock(w: &tauri::WebviewWindow) -> tauri::Result<()> {
    use gtk::prelude::*;
    w.gtk_window()?.set_type_hint(gtk::gdk::WindowTypeHint::Dock);
    w.show()
}

/// Las ventanas de borde nunca deben quedar como ventana activa (ni robar el foco ni contar como "pantalla completa").
#[cfg(windows)]
fn no_activate(w: &tauri::WebviewWindow) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetWindowLongPtrW, SetWindowLongPtrW, GWL_EXSTYLE, WS_EX_NOACTIVATE};
    if let Ok(h) = w.hwnd() {
        let h = h.0 as windows_sys::Win32::Foundation::HWND;
        unsafe { SetWindowLongPtrW(h, GWL_EXSTYLE, GetWindowLongPtrW(h, GWL_EXSTYLE) | WS_EX_NOACTIVATE as isize); }
    }
}

#[cfg(not(windows))]
fn no_activate(w: &tauri::WebviewWindow) { let _ = w.set_focusable(false); }

/// Monitor donde está la ventana en primer plano (para "monitor activo").
#[cfg(windows)]
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

/// Linux: una app no sabe qué ventana de otra está activa; se usa el monitor donde está el cursor.
#[cfg(not(windows))]
pub fn active_monitor_index(app: &AppHandle) -> usize {
    let (Ok(mons), Ok(c)) = (app.available_monitors(), app.cursor_position()) else { return 0 };
    mons.iter().position(|m| { let (p, s) = (m.position(), m.size()); c.x >= p.x as f64 && c.x < (p.x + s.width as i32) as f64 && c.y >= p.y as f64 && c.y < (p.y + s.height as i32) as f64 }).unwrap_or(0)
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
        .resizable(false).shadow(false).visible(cfg!(windows)).position(pos.x as f64 / scale, pos.y as f64 / scale).inner_size(w, h).build()?;
    let saved = app.state::<crate::AppState>().config.lock().unwrap().island.pos;
    // donde la dejaste, si su centro sigue cayendo en algún monitor (pudiste desconectar el que la tenía)
    let (x, y) = saved.map(|[x, y]| (x, y)).filter(|&(x, y)| on_screen(app, x as f64 + size.width as f64 / 2.0, y as f64 + 20.0)).unwrap_or_else(|| island_home(app));
    win.set_position(tauri::PhysicalPosition::new(x, y))?;
    // Linux: tipo dock como los bordes, para que GNOME no la mande a otro monitor y quede por encima de ellos.
    #[cfg(not(windows))] as_dock(&win)?;
    win.set_ignore_cursor_events(true)?;
    app.manage(Hitbox::default());
    #[cfg(windows)] poll_island(app);
    #[cfg(not(windows))] shape_island(app);
    Ok(())
}

/// Lugar de siempre de la isla (físico): arriba del monitor principal; en Linux, bajo la barra superior de GNOME.
fn island_home(app: &AppHandle) -> (i32, i32) {
    let Ok(Some(m)) = app.primary_monitor() else { return (0, 0) };
    let top = if cfg!(windows) { m.position().y } else { work_area(app).1 as i32 };
    (m.position().x, top)
}
fn on_screen(app: &AppHandle, x: f64, y: f64) -> bool {
    app.available_monitors().unwrap_or_default().iter().any(|m| { let (p, s) = (m.position(), m.size()); x >= p.x as f64 && x < p.x as f64 + s.width as f64 && y >= p.y as f64 && y < p.y as f64 + s.height as f64 })
}

/// Ctrl + arrastrar mueve la isla a cualquier lugar, en cualquier monitor: la ventana sigue al cursor desde donde empezó.
/// (cursor al empezar, esquina de la ventana al empezar), en píxeles físicos.
static DRAG: Mutex<Option<((f64, f64), (i32, i32))>> = Mutex::new(None);
/// `start`, `move`, `end` (guarda la posición) o `reset` (vuelve a su lugar de siempre).
pub fn island_drag(app: &AppHandle, phase: &str) {
    let Some(w) = app.get_webview_window("island") else { return };
    match phase {
        "start" => if let (Ok(c), Ok(p)) = (app.cursor_position(), w.outer_position()) { *DRAG.lock().unwrap() = Some(((c.x, c.y), (p.x, p.y))); },
        "move" => {
            let d = *DRAG.lock().unwrap();
            if let (Some(((cx, cy), (px, py))), Ok(c)) = (d, app.cursor_position()) { let _ = w.set_position(tauri::PhysicalPosition::new(px + (c.x - cx).round() as i32, py + (c.y - cy).round() as i32)); }
        }
        "end" => if DRAG.lock().unwrap().take().is_some() { if let Ok(p) = w.outer_position() { save_island_pos(app, Some([p.x, p.y])); } },
        "reset" => { *DRAG.lock().unwrap() = None; let (x, y) = island_home(app); let _ = w.set_position(tauri::PhysicalPosition::new(x, y)); save_island_pos(app, None); }
        _ => {}
    }
}
fn save_island_pos(app: &AppHandle, pos: Option<[i32; 2]>) {
    use tauri::Emitter;
    let cfg = { let st = app.state::<crate::AppState>(); let mut c = st.config.lock().unwrap(); c.island.pos = pos; c.clone() };
    crate::config::save(&cfg);
    let _ = app.emit("pulse://config", &cfg);
}

/// Windows: cada 40 ms la isla deja pasar los clics o no según dónde está el cursor.
#[cfg(windows)]
fn poll_island(app: &AppHandle) {
    let a = app.clone();
    tauri::async_runtime::spawn(async move { loop {
        tokio::time::sleep(std::time::Duration::from_millis(40)).await;
        let Some(win) = a.get_webview_window("island") else { continue };
        let hb = *a.state::<Hitbox>().0.lock().unwrap();   // soltar el lock antes de llamar al hilo principal
        let ignore = match (hb, a.cursor_position(), win.outer_position(), win.scale_factor()) {
            (Some((x, y, w, h)), Ok(c), Ok(o), Ok(s)) => { let (cx, cy) = ((c.x - o.x as f64) / s, (c.y - o.y as f64) / s); !(cx >= x && cx <= x + w && cy >= y && cy <= y + h) }
            _ => true,
        };
        if ignore != ISLAND_IGNORES.load(std::sync::atomic::Ordering::SeqCst) { let _ = win.set_ignore_cursor_events(ignore); ISLAND_IGNORES.store(ignore, std::sync::atomic::Ordering::SeqCst); }
        // recuerda la última ventana activa que no es de Pulse, para devolverle el teclado
        let fg = unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetForegroundWindow() } as isize;
        if fg != 0 && !is_ours(fg) { PREV_FOREGROUND.store(fg, std::sync::atomic::Ordering::SeqCst); }
    } });
}

/// Linux: la isla solo recibe clics dentro del rectángulo que reporta la interfaz (región de entrada de la ventana).
/// El resto deja pasar los clics a lo que esté debajo.
#[cfg(not(windows))]
pub fn shape_island(app: &AppHandle) {
    use gtk::{cairo::{RectangleInt, Region}, prelude::*};
    let Some(w) = app.get_webview_window("island") else { return };
    let hb = *app.state::<Hitbox>().0.lock().unwrap();
    let w2 = w.clone();
    let _ = w.run_on_main_thread(move || {
        let Some(gdk) = w2.gtk_window().ok().and_then(|g| g.window()) else { return };
        let region = match hb {
            Some((x, y, w, h)) => Region::create_rectangle(&RectangleInt::new(x.floor() as i32, y.floor() as i32, w.ceil() as i32 + 1, h.ceil() as i32 + 1)),
            None => Region::create(),
        };
        gdk.input_shape_combine_region(&region, 0, 0);
    });
}
/// La interfaz movió o cambió el rectángulo de la isla.
pub fn hitbox_changed(_app: &AppHandle) { #[cfg(not(windows))] shape_island(_app); }
/// Área de trabajo del monitor principal (sin la barra de tareas), en píxeles físicos: (izq, arriba, der, abajo).
#[cfg(windows)]
fn work_area(_app: &AppHandle) -> (f64, f64, f64, f64) {
    use windows_sys::Win32::{Foundation::RECT, UI::WindowsAndMessaging::{SystemParametersInfoW, SPI_GETWORKAREA}};
    let mut r: RECT = unsafe { std::mem::zeroed() };
    unsafe { SystemParametersInfoW(SPI_GETWORKAREA, 0, &mut r as *mut _ as _, 0) };
    (r.left as f64, r.top as f64, r.right as f64, r.bottom as f64)
}
/// Linux: sin la barra superior de GNOME ni el dock.
#[cfg(not(windows))]
fn work_area(app: &AppHandle) -> (f64, f64, f64, f64) {
    let Ok(Some(m)) = app.primary_monitor() else { return (0.0, 0.0, 1920.0, 1080.0) };
    let w = m.work_area();
    (w.position.x as f64, w.position.y as f64, w.position.x as f64 + w.size.width as f64, w.position.y as f64 + w.size.height as f64)
}
/// Momento en que el panel se ocultó por perder el foco: el clic en la bandeja que lo provocó no debe volver a abrirlo.
static PANEL_HIDDEN_AT: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);
/// Si la isla deja pasar los clics ahora mismo. Lo comparten el ciclo de 40 ms y `island_keyboard`.
#[cfg(windows)]
static ISLAND_IGNORES: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(true);
/// Ventana que tenía el teclado antes de que la isla lo tomara (paleta, "Otra…").
#[cfg(windows)]
static PREV_FOREGROUND: std::sync::atomic::AtomicIsize = std::sync::atomic::AtomicIsize::new(0);
#[cfg(windows)]
fn is_ours(h: isize) -> bool {
    let mut pid = 0u32;
    unsafe { windows_sys::Win32::UI::WindowsAndMessaging::GetWindowThreadProcessId(h as _, &mut pid) };
    pid == std::process::id()
}
/// La isla toma el teclado (`on`) o se lo devuelve a la ventana donde estabas.
#[cfg(windows)]
pub fn island_keyboard(app: &AppHandle, on: bool) {
    use windows_sys::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, IsWindow, SetForegroundWindow};
    use std::sync::atomic::Ordering;
    let Some(w) = app.get_webview_window("island") else { return };
    let island = w.hwnd().map(|h| h.0 as isize).unwrap_or(0);
    // al tomar el teclado recibe clics; al soltarlo, el ciclo de 40 ms decide según dónde está el cursor
    if on { let _ = w.set_ignore_cursor_events(false); ISLAND_IGNORES.store(false, Ordering::SeqCst); }
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

/// Linux: la isla pide el teclado; devolverlo a la ventana anterior queda en manos del escritorio.
/// Los clics siguen limitados al rectángulo visible (que ya incluye la paleta o la tarjeta): GNOME puede negarle el foco
/// a una ventana que lo pide sola, y entonces nunca llega el "perdió el foco" que la cierra; con toda la franja recibiendo
/// clics, lo que hay debajo quedaría inalcanzable.
#[cfg(not(windows))]
pub fn island_keyboard(app: &AppHandle, on: bool) {
    if on { if let Some(w) = app.get_webview_window("island") { let _ = w.set_focus(); } }
}

/// Esquina superior izquierda de Bolsillo (lógica): en Windows sobre la barra de tareas, abajo a la derecha;
/// en Linux bajo la barra superior, arriba a la derecha, donde está el ícono.
fn panel_origin(work: (f64, f64, f64, f64)) -> (f64, f64) {
    let (_, t, r, b) = work;
    if cfg!(windows) { (r - 380.0 - 12.0, (b - 580.0 - 12.0).max(t)) } else { (r - 380.0 - 12.0, t + 12.0) }
}

pub fn toggle_panel(app: &AppHandle) {
    let win = match app.get_webview_window("panel") { Some(w) => w, None => match create_panel(app) { Ok(w) => w, Err(_) => return } };
    if win.is_visible().unwrap_or(false) { let _ = win.hide(); return; }
    if crate::now_ms() - PANEL_HIDDEN_AT.load(std::sync::atomic::Ordering::SeqCst) < 300 { return; }
    let scale = win.scale_factor().unwrap_or(1.0);
    let (l, t, r, b) = work_area(app);
    let (x, y) = panel_origin((l / scale, t / scale, r / scale, b / scale));
    let _ = win.set_position(tauri::LogicalPosition::new(x, y));
    let _ = win.show(); let _ = win.set_focus();
}
fn create_panel(app: &AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    let w = WebviewWindowBuilder::new(app, "panel", WebviewUrl::App("panel.html".into()))
        .transparent(true).decorations(false).always_on_top(true).skip_taskbar(true).resizable(false).shadow(false)
        .inner_size(380.0, 580.0).visible(false).build()?;
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
    let (l, t, r, b) = work_area(win.app_handle());
    let (w, h) = (size.width as f64 / scale, size.height as f64 / scale);
    // Linux: se abre desde el menú del ícono, arriba a la derecha, bajo la barra superior
    let (x, y) = if cfg!(windows) { flyout_origin(*FLYOUT_CLICK_X.lock().unwrap(), (w, h), (l / scale, t / scale, r / scale, b / scale)) } else { ((r / scale - w).max(l / scale), t / scale) };
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
