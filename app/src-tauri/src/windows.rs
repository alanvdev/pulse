use std::sync::{atomic::{AtomicU32, Ordering}, Mutex};
use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

pub fn create_all(app: &AppHandle) -> tauri::Result<()> {
    sync_overlays(app)?;
    create_island(app)
}

/// Tanda de bordes: al cambiar los monitores se rehacen con otra etiqueta, porque la ventana que se destruye sigue
/// ocupando la suya un momento.
static OVERLAY_GEN: AtomicU32 = AtomicU32::new(0);

/// Un borde por monitor, en el orden de ahora (`?m=` es el índice que usa "monitor activo"). Si ya había, se destruyen y
/// se rehacen: conectaste o quitaste un monitor, o el dock cambió el área libre. Corre en el hilo principal.
pub fn sync_overlays(app: &AppHandle) -> tauri::Result<()> {
    for (label, w) in app.webview_windows() { if label.starts_with("overlay-") { let _ = w.destroy(); } }
    let gen = OVERLAY_GEN.fetch_add(1, Ordering::SeqCst);
    for (i, m) in app.available_monitors()?.iter().enumerate() {
        let scale = m.scale_factor();
        // Linux: ventana normal "siempre encima", del tamaño del área libre del monitor (sin la barra superior ni el dock):
        // así GNOME la deja en su monitor. Puede quedar encima de la isla (GNOME pone docks y "siempre encima" en la misma
        // capa), y da igual: no acepta foco, deja pasar los clics y la isla se sigue pintando sin sincronización de cuadros.
        #[cfg(windows)] let (pos, size) = (*m.position(), *m.size());
        #[cfg(not(windows))] let (pos, size) = (m.work_area().position, m.work_area().size);
        // Linux: sin aceptar el foco desde que nace (si no, tao se lo devuelve al primer dibujo) y en todos los
        // escritorios: GNOME deja una ventana normal del monitor principal en un solo escritorio.
        let w = WebviewWindowBuilder::new(app, format!("overlay-{gen}-{i}"), WebviewUrl::App(format!("overlay.html?m={i}").into()))
            .transparent(true).decorations(false).always_on_top(true).skip_taskbar(true).focused(false)
            .focusable(cfg!(windows)).visible_on_all_workspaces(cfg!(not(windows)))
            .resizable(false).shadow(false).visible(cfg!(windows))
            .position(pos.x as f64 / scale, pos.y as f64 / scale)
            .inner_size(size.width as f64 / scale, size.height as f64 / scale)
            .build()?;
        #[cfg(not(windows))] { let g = w.gtk_window()?; no_frame_sync(&g); no_take_focus(&g); w.show()?; }
        w.set_ignore_cursor_events(true)?;
        no_activate(&w);
    }
    Ok(())
}

/// Linux: monitores y su área libre (posición y tamaño de cada uno). Si cambia, hay que rehacer bordes y recolocar la isla.
#[cfg(not(windows))]
pub fn monitor_signature(app: &AppHandle) -> Vec<[i64; 8]> {
    app.available_monitors().unwrap_or_default().iter().map(|m| { let (p, s, w) = (m.position(), m.size(), m.work_area());
        [p.x as i64, p.y as i64, s.width as i64, s.height as i64, w.position.x as i64, w.position.y as i64, w.size.width as i64, w.size.height as i64] }).collect()
}
/// Linux: los monitores cambiaron. Bordes nuevos y la isla en su lugar (si la tenías en el de siempre, sigue al área libre;
/// si la moviste y su monitor ya no está, vuelve a casa). Va al hilo principal: ahí viven las ventanas GTK.
#[cfg(not(windows))]
pub fn monitors_changed(app: &AppHandle) {
    let a = app.clone();
    let _ = app.run_on_main_thread(move || { let _ = sync_overlays(&a); place_island(&a); });
}

/// Linux: la isla, Bolsillo y el menú rápido son de tipo "dock". GNOME acomoda a su gusto una ventana normal (la manda a
/// otro monitor si no cabe junto a la barra superior y el dock) y le niega el foco cuando lo pide sola; a un dock lo deja
/// donde se pone y sí le da el teclado. Los bordes no: GNOME reordena los docks entre sí al cambiar el foco y podría
/// dejar la isla debajo de ellos, sin clics y sin pintarse. El tipo se fija antes de mostrarla.
#[cfg(not(windows))]
fn as_dock(w: &tauri::WebviewWindow) -> tauri::Result<()> {
    use gtk::prelude::*;
    let g = w.gtk_window()?;
    g.set_type_hint(gtk::gdk::WindowTypeHint::Dock);
    no_frame_sync(&g);
    Ok(())
}
/// GNOME no siempre avisa "cuadro mostrado" a estas ventanas (en la pantalla de la laptop, con dos tarjetas gráficas,
/// nunca llega); con la sincronización de cuadros activa, GTK y WebKit se quedan esperando ese aviso y la ventana deja de
/// pintarse a media animación. Sin ella pinta a su ritmo.
#[cfg(not(windows))]
fn no_frame_sync(g: &gtk::ApplicationWindow) {
    use gtk::prelude::*;
    g.realize();   // crea la ventana X sin mostrarla: hasta entonces no hay a quién apagarle la sincronización
    if let Some(x) = g.window().and_then(|x| x.downcast::<gdkx11::X11Window>().ok()) { x.set_frame_sync_enabled(false); }
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

/// Linux: GDK anuncia WM_TAKE_FOCUS en toda ventana, y con eso GNOME cuenta el borde como enfocable aunque no acepte el
/// teclado: al aparecer se lo quita a tu terminal y, al cerrar una ventana u ocultar Bolsillo o el menú rápido, se lo da
/// al borde invisible en vez de a la ventana de abajo. Sin ese protocolo GNOME nunca lo enfoca. Va antes de mostrarla.
#[cfg(not(windows))]
fn no_take_focus(g: &gtk::ApplicationWindow) {
    use gtk::prelude::*;
    let Some(x) = g.window().and_then(|x| x.downcast::<gdkx11::X11Window>().ok()) else { return };
    // los átomos se piden directo a X: los atajos de gdkx11 exigen que GDK se haya iniciado desde gtk-rs (Tauri no lo marca) y entran en pánico
    unsafe {
        let dpy = gdkx11::ffi::gdk_x11_get_default_xdisplay() as *mut x11::xlib::Display;
        if dpy.is_null() { return; }
        let mut keep = [c"WM_DELETE_WINDOW", c"_NET_WM_PING"].map(|a| x11::xlib::XInternAtom(dpy, a.as_ptr(), 0));
        x11::xlib::XSetWMProtocols(dpy, x.xid(), keep.as_mut_ptr(), keep.len() as i32);
    }
}

/// Muestra la ventana y le da el teclado. Linux: GNOME solo le da el foco a una ventana de Pulse que ya está visible y lo
/// pide con una hora reciente del servidor X (si no, lo toma por un robo de foco). Se muestra, y un momento después pide
/// el foco con la hora actual del servidor.
pub fn bring_to_front(w: &tauri::WebviewWindow) {
    let _ = w.show();
    #[cfg(windows)] { let _ = w.set_focus(); }
    #[cfg(not(windows))] {
        let w2 = w.clone();
        let _ = w.run_on_main_thread(move || {
            use gtk::{glib, prelude::*};
            let Ok(g) = w2.gtk_window() else { return };
            glib::timeout_add_local_once(std::time::Duration::from_millis(80), move || {
                let ts = g.window().and_then(|x| x.downcast::<gdkx11::X11Window>().ok()).map(|x| gdkx11::functions::x11_get_server_time(&x));
                match ts { Some(t) => g.present_with_time(t), None => g.present() }
            });
        });
    }
}

/// Monitor donde está la ventana en primer plano (para "monitor activo").
#[cfg(windows)]
pub fn active_monitor_index(app: &AppHandle) -> Option<usize> {
    use windows_sys::Win32::{Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST}, UI::WindowsAndMessaging::GetForegroundWindow};
    let Ok(mons) = app.available_monitors() else { return Some(0) };
    unsafe {
        let hmon = MonitorFromWindow(GetForegroundWindow(), MONITOR_DEFAULTTONEAREST);
        let mut mi: MONITORINFO = std::mem::zeroed(); mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(hmon, &mut mi) == 0 { return Some(0); }
        Some(mons.iter().position(|m| m.position().x == mi.rcMonitor.left && m.position().y == mi.rcMonitor.top).unwrap_or(0))
    }
}

/// Linux: no se puede saber. Wayland no deja ver la ventana activa de otra app, y el cursor que ve XWayland se queda
/// donde pasó por última vez sobre una ventana X11 (no se mueve sobre la terminal ni el navegador). `None`: el pulso
/// va a todos los monitores.
#[cfg(not(windows))]
pub fn active_monitor_index(_app: &AppHandle) -> Option<usize> { None }

#[derive(Default)] pub struct Hitbox(pub Mutex<Option<(f64, f64, f64, f64)>>);

/// Isla: franja transparente arriba del monitor principal (todo el ancho, para que el Orbe pueda ir a la izquierda, centro o derecha).
/// No recibe clics salvo dentro del rectángulo que reporta la interfaz.
pub fn create_island(app: &AppHandle) -> tauri::Result<()> {
    let m = app.primary_monitor()?.ok_or(tauri::Error::WindowNotFound)?;
    let (scale, pos, (w, h)) = (m.scale_factor(), m.position(), island_size(&m));
    let win = WebviewWindowBuilder::new(app, "island", WebviewUrl::App("island.html".into()))
        .transparent(true).decorations(false).always_on_top(true).skip_taskbar(true).focused(false)
        .resizable(false).shadow(false).visible(cfg!(windows)).position(pos.x as f64 / scale, pos.y as f64 / scale).inner_size(w, h).build()?;
    let (x, y) = island_spot(app, w * scale, scale);
    win.set_position(tauri::PhysicalPosition::new(x, y))?;
    // Linux: tipo dock como los bordes, para que GNOME no la mande a otro monitor y quede por encima de ellos.
    #[cfg(not(windows))] { as_dock(&win)?; win.show()?; }
    win.set_ignore_cursor_events(true)?;
    app.manage(Hitbox::default());
    #[cfg(windows)] poll_island(app);
    #[cfg(not(windows))] shape_island(app);
    Ok(())
}

/// Tamaño lógico de la isla. Windows: todo el ancho del monitor principal. Linux: el de su área libre, para que el Orbe
/// a la izquierda no quede bajo el dock de Ubuntu (que GNOME pinta encima de todo) y la pastilla se centre en lo que se ve.
/// Alto de sobra para tarjetas de preguntas largas: la zona sin contenido es transparente y no recibe clics.
fn island_size(m: &tauri::Monitor) -> (f64, f64) {
    let scale = m.scale_factor();
    #[cfg(windows)] let width = m.size().width;
    #[cfg(not(windows))] let width = m.work_area().size.width;
    (width as f64 / scale, (m.size().height as f64 / scale * 0.8).min(860.0))
}

/// Esquina (física) donde va la isla: donde la dejaste, si lo que se ve de ella sigue cayendo en algún monitor (pudiste
/// desconectar el que la tenía); si no, su lugar de siempre. `width` es el ancho físico de la ventana.
fn island_spot(app: &AppHandle, width: f64, scale: f64) -> (i32, i32) {
    let st = app.state::<crate::AppState>();
    let (saved, mode, side) = { let c = st.config.lock().unwrap(); (c.island.pos, c.island.mode, c.island.orb_side) };
    let dx = island_anchor(mode, side, width, scale);
    saved.map(|[x, y]| (x, y)).filter(|&(x, y)| on_screen(app, x as f64 + dx, y as f64 + 20.0 * scale)).unwrap_or_else(|| island_home(app))
}
/// Distancia (física) del borde izquierdo de la ventana a lo que se ve de la isla: el Orbe va a 16 px de su lado (más
/// medio grano) y la pastilla de Comando al centro. No sirve el centro de la ventana: es tan ancha como el monitor, y un
/// Orbe llevado a la orilla contraria deja ese centro fuera de toda pantalla.
fn island_anchor(mode: crate::config::IslandMode, side: crate::config::Side, width: f64, scale: f64) -> f64 {
    use crate::config::{IslandMode, Side};
    match (mode, side) {
        (IslandMode::Orbe, Side::Left) => 28.0 * scale,
        (IslandMode::Orbe, Side::Right) => width - 28.0 * scale,
        _ => width / 2.0,
    }
}
/// Linux, los monitores cambiaron: la isla con el tamaño de ahora y en su lugar.
#[cfg(not(windows))]
fn place_island(app: &AppHandle) {
    let (Some(win), Ok(Some(m))) = (app.get_webview_window("island"), app.primary_monitor()) else { return };
    let (scale, (w, h)) = (m.scale_factor(), island_size(&m));
    let _ = win.set_size(tauri::LogicalSize::new(w, h));
    let (x, y) = island_spot(app, w * scale, scale);
    let _ = win.set_position(tauri::PhysicalPosition::new(x, y));
}

/// Lugar de siempre de la isla (físico): arriba del monitor principal; en Linux, en la esquina de su área libre (bajo la
/// barra superior de GNOME y a la derecha del dock).
fn island_home(app: &AppHandle) -> (i32, i32) {
    let Ok(Some(m)) = app.primary_monitor() else { return (0, 0) };
    if cfg!(windows) { (m.position().x, m.position().y) } else { let (l, t, _, _) = work_area(app); (l as i32, t as i32) }
}
fn on_screen(app: &AppHandle, x: f64, y: f64) -> bool {
    app.available_monitors().unwrap_or_default().iter().any(|m| { let (p, s) = (m.position(), m.size()); x >= p.x as f64 && x < p.x as f64 + s.width as f64 && y >= p.y as f64 && y < p.y as f64 + s.height as f64 })
}

/// Ctrl + arrastrar mueve la isla a cualquier lugar, en cualquier monitor: la ventana sigue al cursor desde donde empezó.
/// (cursor al empezar, esquina de la ventana al empezar), en píxeles físicos.
static DRAG: Mutex<Option<((f64, f64), (i32, i32))>> = Mutex::new(None);
/// Esquina a la que va la ventana: la del inicio más lo que se movió el cursor.
fn drag_target((cx, cy): (f64, f64), (px, py): (i32, i32), (x, y): (f64, f64)) -> (i32, i32) { (px + (x - cx).round() as i32, py + (y - cy).round() as i32) }
/// `start`, `move`, `end` (guarda la posición) o `reset` (vuelve a su lugar de siempre).
pub fn island_drag(app: &AppHandle, phase: &str) {
    let Some(w) = app.get_webview_window("island") else { return };
    match phase {
        "start" => if let (Ok(c), Ok(p)) = (app.cursor_position(), w.outer_position()) { *DRAG.lock().unwrap() = Some(((c.x, c.y), (p.x, p.y))); },
        "move" => {
            let d = *DRAG.lock().unwrap();
            if let (Some((c0, p0)), Ok(c)) = (d, app.cursor_position()) { let (x, y) = drag_target(c0, p0, (c.x, c.y)); let _ = w.set_position(tauri::PhysicalPosition::new(x, y)); }
        }
        // Se guarda el destino calculado y no `outer_position`: en Linux esa se actualiza cuando GNOME confirma el último
        // movimiento, que suele llegar después. Un Ctrl + clic sin mover no guarda nada: la isla sigue en su lugar de
        // siempre, que acompaña al área libre.
        "end" => {
            let d = DRAG.lock().unwrap().take();
            if let (Some((c0, p0)), Ok(c)) = (d, app.cursor_position()) {
                let (x, y) = drag_target(c0, p0, (c.x, c.y));
                if (x, y) != p0 { let _ = w.set_position(tauri::PhysicalPosition::new(x, y)); save_island_pos(app, Some([x, y])); }
            }
        }
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
            None => Region::create_rectangle(&RectangleInt::new(0, 0, 1, 1)),
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

/// Linux: la isla pide el teclado y, al soltarlo (si todavía lo tiene), se oculta y vuelve a aparecer en el acto: una app
/// no puede elegir a qué ventana devolverle el teclado en Wayland, pero GNOME se lo da a la última que usaste. Al reaparecer
/// puede quedar debajo de los bordes, y no importa: los bordes dejan pasar los clics y no le quitan el pintado.
/// Los clics siguen limitados al rectángulo visible (que ya incluye la paleta o la tarjeta): GNOME puede negarle el foco
/// a una ventana que lo pide sola, y entonces nunca llega el "perdió el foco" que la cierra; con toda la franja recibiendo
/// clics, lo que hay debajo quedaría inalcanzable.
#[cfg(not(windows))]
pub fn island_keyboard(app: &AppHandle, on: bool) {
    let Some(w) = app.get_webview_window("island") else { return };
    if on { bring_to_front(&w); }
    else if w.is_focused().unwrap_or(false) { let _ = w.hide(); let _ = w.show(); shape_island(app); }
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
    bring_to_front(&win);
}
/// Linux la crea al arrancar (hilo principal): sus ajustes de GTK no pueden hacerse desde el comando asíncrono que la abre.
pub(crate) fn create_panel(app: &AppHandle) -> tauri::Result<tauri::WebviewWindow> {
    let w = WebviewWindowBuilder::new(app, "panel", WebviewUrl::App("panel.html".into()))
        .transparent(true).decorations(false).always_on_top(true).skip_taskbar(true).resizable(false).shadow(false)
        .inner_size(380.0, 580.0).visible(false).build()?;
    #[cfg(not(windows))] as_dock(&w)?;
    let w2 = w.clone();
    w.on_window_event(move |e| if let tauri::WindowEvent::Focused(false) = e { if w2.is_visible().unwrap_or(false) { PANEL_HIDDEN_AT.store(crate::now_ms(), std::sync::atomic::Ordering::SeqCst); } let _ = w2.hide(); });
    Ok(w)
}
/// Abre Mosaico; `view` es la ruta dentro de Mosaico ("ajustes", "sesiones/<id>", ...).
pub fn open_dashboard(app: &AppHandle, view: Option<&str>) {
    use tauri::Emitter;
    if let Some(w) = app.get_webview_window("dashboard") {
        if let Some(v) = view { let _ = app.emit_to("dashboard", "pulse://route", v); }
        let _ = w.unminimize(); bring_to_front(&w); return;
    }
    let route = serde_json::to_string(view.unwrap_or("")).unwrap_or_else(|_| "\"\"".into());
    // Linux: se pide el foco con la hora del servidor X como la que ya estaba abierta; si no, desde el menú del ícono
    // GNOME la deja detrás de tu ventana y solo avisa "Pulse está listo". Antes de mostrarla se le apaga la sincronización
    // de cuadros (en la pantalla de la laptop se congela igual que la isla), en el hilo principal: esto llega desde un
    // comando asíncrono.
    let Ok(w) = WebviewWindowBuilder::new(app, "dashboard", WebviewUrl::App("dashboard.html".into()))
        .initialization_script(&format!("window.__pulseRoute = {route};"))
        .title("Pulse").inner_size(1120.0, 760.0).min_inner_size(720.0, 520.0).center().visible(cfg!(windows)).build() else { return };
    #[cfg(windows)] bring_to_front(&w);
    #[cfg(not(windows))] { let w2 = w.clone(); let _ = w.run_on_main_thread(move || { if let Ok(g) = w2.gtk_window() { no_frame_sync(&g); } bring_to_front(&w2); }); }
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
    #[cfg(not(windows))] as_dock(&w)?;
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
    bring_to_front(&win);
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
    use crate::config::{IslandMode, Side};
    #[test] fn drag_target_follows_the_cursor_from_the_start_corner() {
        assert_eq!(drag_target((100.0, 50.0), (0, 32), (100.0, 50.0)), (0, 32));          // Ctrl + clic sin mover: el mismo lugar
        assert_eq!(drag_target((100.0, 50.0), (0, 32), (-1480.4, 650.6)), (-1580, 633));  // al otro monitor, redondeado
    }
    #[test] fn a_saved_island_is_checked_where_its_content_is() {
        // ventana de 1920 px físicos a escala 1.25: el Orbe derecho cerca de la orilla, el izquierdo junto a la suya
        assert_eq!(island_anchor(IslandMode::Orbe, Side::Right, 1920.0, 1.25), 1885.0);
        assert_eq!(island_anchor(IslandMode::Orbe, Side::Left, 1920.0, 1.25), 35.0);
        assert_eq!(island_anchor(IslandMode::Orbe, Side::Center, 1920.0, 1.25), 960.0);
        assert_eq!(island_anchor(IslandMode::Comando, Side::Left, 1920.0, 1.25), 960.0);   // la pastilla siempre va al centro
    }
    #[test] fn flyout_sits_above_the_click_inside_the_work_area() {
        let work = (0.0, 0.0, 1920.0, 1032.0);
        assert_eq!(flyout_origin(1700.0, (324.0, 400.0), work), (1538.0, 632.0));   // centrado sobre el clic
        assert_eq!(flyout_origin(1900.0, (324.0, 400.0), work), (1596.0, 632.0));   // no se sale por la derecha
        assert_eq!(flyout_origin(40.0, (324.0, 400.0), work), (0.0, 632.0));        // ni por la izquierda
        assert_eq!(flyout_origin(500.0, (324.0, 2000.0), work), (338.0, 0.0));      // ni por arriba
    }
}
