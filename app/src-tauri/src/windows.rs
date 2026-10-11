use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

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

fn create_island(_: &AppHandle) -> tauri::Result<()> { Ok(()) }
pub fn toggle_panel(_: &AppHandle) {}
pub fn open_dashboard(_: &AppHandle) {}
