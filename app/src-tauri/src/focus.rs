use crate::{broadcast, config::FocusCfg, AppState};
use tauri::{AppHandle, Manager};

fn hm(s: &str) -> u32 { let mut it = s.split(':').map(|x| x.parse::<u32>().unwrap_or(0)); it.next().unwrap_or(0) * 60 + it.next().unwrap_or(0) }

pub fn detect(c: &FocusCfg, now_min: u32, mic: bool, quns: i32) -> bool {
    let full = matches!(quns, 2 | 3 | 4);
    let sched = c.schedule && { let (a, b) = (hm(&c.from), hm(&c.to)); if a <= b { now_min >= a && now_min < b } else { now_min >= a || now_min < b } };
    (c.detect_call && mic) || (c.detect_fullscreen && full) || sched
}

pub fn notification_state() -> i32 {
    let mut s = 0i32;
    unsafe { windows_sys::Win32::UI::Shell::SHQueryUserNotificationState(&mut s) };
    s
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
        let auto = detect(&cfg, local_minutes(), mic_in_use(), notification_state());
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
        assert!(!detect(&c, 600, false, 5));          // QUNS_ACCEPTS_NOTIFICATIONS = 5
        assert!(detect(&c, 600, true, 5));            // llamada
        assert!(detect(&c, 600, false, 3));           // QUNS_RUNNING_D3D_FULL_SCREEN
        assert!(detect(&c, 600, false, 4));           // QUNS_PRESENTATION_MODE
        assert!(detect(&c, 600, false, 2));           // QUNS_BUSY (pantalla completa)
        c.detect_fullscreen = false; assert!(!detect(&c, 600, false, 3));
        c.schedule = true; c.from = "09:00".into(); c.to = "11:00".into();
        assert!(detect(&c, 9 * 60 + 30, false, 5)); assert!(!detect(&c, 11 * 60, false, 5));
        c.from = "22:00".into(); c.to = "02:00".into();
        assert!(detect(&c, 23 * 60, false, 5)); assert!(detect(&c, 60, false, 5)); assert!(!detect(&c, 12 * 60, false, 5));
    }
}
