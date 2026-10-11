use crate::{broadcast, config::FocusCfg, AppState};
use tauri::{AppHandle, Manager};

fn hm(s: &str) -> u32 { let mut it = s.split(':').map(|x| x.parse::<u32>().unwrap_or(0)); it.next().unwrap_or(0) * 60 + it.next().unwrap_or(0) }

pub fn detect(c: &FocusCfg, now_min: u32, mic: bool, fullscreen: bool) -> bool {
    if !c.auto { return false; }
    let sched = c.schedule && { let (a, b) = (hm(&c.from), hm(&c.to)); if a <= b { now_min >= a && now_min < b } else { now_min >= a || now_min < b } };
    (c.detect_call && mic) || (c.detect_fullscreen && fullscreen) || sched
}

/// `win` cubre todo `mon` (rectángulos left, top, right, bottom).
pub fn covers(win: (i32, i32, i32, i32), mon: (i32, i32, i32, i32)) -> bool { win.0 <= mon.0 && win.1 <= mon.1 && win.2 >= mon.2 && win.3 >= mon.3 }

/// ¿La ventana activa es de otra app y cubre todo su monitor (video, juego, presentación)?
/// No se usa SHQueryUserNotificationState: las ventanas de borde de Pulse cubren el monitor y Windows las toma por pantalla completa.
pub fn other_app_fullscreen() -> bool {
    use windows_sys::Win32::{Foundation::RECT, Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST}, UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow, GetWindowRect, GetWindowThreadProcessId, IsZoomed}};
    unsafe {
        let h = GetForegroundWindow();
        if h.is_null() { return false; }
        let mut pid = 0u32; GetWindowThreadProcessId(h, &mut pid);
        if pid == std::process::id() { return false; }
        let mut name = [0u16; 64]; let n = GetClassNameW(h, name.as_mut_ptr(), name.len() as i32);
        if matches!(String::from_utf16_lossy(&name[..n.max(0) as usize]).as_str(), "Progman" | "WorkerW" | "Shell_TrayWnd") { return false; }   // escritorio y barra de tareas
        if IsZoomed(h) != 0 { return false; }   // maximizada no es pantalla completa (con la barra de tareas oculta cubre todo el monitor)
        let mut r: RECT = std::mem::zeroed();
        if GetWindowRect(h, &mut r) == 0 { return false; }
        let mut mi: MONITORINFO = std::mem::zeroed(); mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST), &mut mi) == 0 { return false; }
        let m = mi.rcMonitor;
        covers((r.left, r.top, r.right, r.bottom), (m.left, m.top, m.right, m.bottom))
    }
}

/// Micrófono en uso por otra app: alguna entrada de ConsentStore con LastUsedTimeStop == 0.
pub fn mic_in_use() -> bool {
    use windows_sys::Win32::System::Registry::*;
    unsafe fn any_active(root: HKEY, path: &str) -> bool {
        let wide: Vec<u16> = path.encode_utf16().chain(Some(0)).collect();
        let mut key: HKEY = std::ptr::null_mut();
        if RegOpenKeyExW(root, wide.as_ptr(), 0, KEY_READ, &mut key) != 0 { return false; }
        let mut i = 0u32; let mut found = false;
        loop {
            let mut name = [0u16; 512]; let mut len = name.len() as u32;
            if RegEnumKeyExW(key, i, name.as_mut_ptr(), &mut len, std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut(), std::ptr::null_mut()) != 0 { break; }
            let sub = String::from_utf16_lossy(&name[..len as usize]);
            if sub == "NonPackaged" { if any_active(root, &format!("{path}\\NonPackaged")) { found = true; break; } }
            else {
                let sw: Vec<u16> = format!("{path}\\{sub}").encode_utf16().chain(Some(0)).collect();
                let vname: Vec<u16> = "LastUsedTimeStop".encode_utf16().chain(Some(0)).collect();
                let mut v: u64 = 1; let mut sz = 8u32;
                if RegGetValueW(root, sw.as_ptr(), vname.as_ptr(), RRF_RT_REG_QWORD, std::ptr::null_mut(), &mut v as *mut _ as _, &mut sz) == 0 && v == 0 { found = true; break; }
            }
            i += 1;
        }
        RegCloseKey(key);
        found
    }
    unsafe { any_active(HKEY_CURRENT_USER, r"Software\Microsoft\Windows\CurrentVersion\CapabilityAccessManager\ConsentStore\microphone") }
}

fn local_minutes() -> u32 {
    let mut t: windows_sys::Win32::Foundation::SYSTEMTIME = unsafe { std::mem::zeroed() };
    unsafe { windows_sys::Win32::System::SystemInformation::GetLocalTime(&mut t) };
    t.wHour as u32 * 60 + t.wMinute as u32
}

pub fn start(app: &AppHandle) {
    let a = app.clone();
    tauri::async_runtime::spawn(async move { let mut last = false; loop {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        let st = a.state::<AppState>();
        let cfg = st.config.lock().unwrap().focus.clone();
        let auto = detect(&cfg, local_minutes(), cfg.auto && cfg.detect_call && mic_in_use(), cfg.auto && cfg.detect_fullscreen && other_app_fullscreen());
        if auto == last { continue; }
        last = auto;
        let released = st.store.lock().unwrap().set_focus(None, auto);
        if !released.is_empty() {
            crate::emit_fx(&a, crate::Fx { kind: "release".into(), session_id: released[0].session_id.clone(), project: released[0].project.clone(), slot: 0, text: serde_json::to_string(&released).unwrap_or_default(), visual: true, sound: false, monitor: crate::windows::active_monitor_index(&a) });
        }
        broadcast(&a);
    } });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::FocusCfg;
    #[test] fn detection_rules() {
        let mut c = FocusCfg::default();
        assert!(!detect(&c, 600, false, false));
        assert!(detect(&c, 600, true, false));        // llamada
        assert!(detect(&c, 600, false, true));        // otra app en pantalla completa
        c.detect_fullscreen = false; assert!(!detect(&c, 600, false, true));
        c.schedule = true; c.from = "09:00".into(); c.to = "11:00".into();
        assert!(detect(&c, 9 * 60 + 30, false, false)); assert!(!detect(&c, 11 * 60, false, false));
        c.from = "22:00".into(); c.to = "02:00".into();
        assert!(detect(&c, 23 * 60, false, false)); assert!(detect(&c, 60, false, false)); assert!(!detect(&c, 12 * 60, false, false));
    }
    #[test] fn auto_switch_turns_everything_off() {
        let mut c = FocusCfg::default();
        c.auto = false; c.schedule = true; c.from = "00:00".into(); c.to = "23:59".into();
        assert!(!detect(&c, 600, true, true));
    }
    #[test] fn fullscreen_means_covering_the_whole_monitor() {
        let mon = (0, 0, 1920, 1080);
        assert!(covers(mon, mon));
        assert!(covers((-8, -8, 1928, 1088), mon));    // ventana maximizada sin marco que se sale un poco
        assert!(!covers((0, 0, 1920, 1040), mon));     // maximizada normal: deja ver la barra de tareas
        assert!(!covers((100, 100, 900, 700), mon));
    }
}
