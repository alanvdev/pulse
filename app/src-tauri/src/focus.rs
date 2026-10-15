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

/// Una ventana que cubre el monitor es pantalla completa si no está maximizada, o si lo está pero perdió la barra de
/// título y el borde (así queda un navegador con un video en pantalla completa). Maximizada con título no cuenta:
/// con la barra de tareas oculta también cubre todo el monitor.
pub fn is_fullscreen(covers_monitor: bool, zoomed: bool, style: u32) -> bool {
    const WS_CAPTION: u32 = 0x00C0_0000; const WS_THICKFRAME: u32 = 0x0004_0000;
    covers_monitor && (!zoomed || style & (WS_CAPTION | WS_THICKFRAME) == 0)
}

/// Ventanas del sistema que cubren el monitor sin ser una app en pantalla completa: escritorio, barra de tareas,
/// Alt+Tab y las capas de Windows (Inicio, búsqueda, emojis, portapapeles, teclado táctil: TextInputHost).
pub fn is_shell_surface(class: &str) -> bool {
    matches!(class, "Progman" | "WorkerW" | "Shell_TrayWnd" | "Shell_SecondaryTrayWnd" | "Windows.UI.Core.CoreWindow" | "XamlExplorerHostIslandWindow" | "ForegroundStaging" | "MultitaskingViewFrame")
}

/// Detección estable: un cambio cuenta solo si se repite en `n` revisiones seguidas (una ventana de paso no enciende el foco).
#[derive(Default)] pub struct Debounce { last: bool, seen: bool, count: u32 }
impl Debounce {
    /// Devuelve el valor estable después de ver `raw`.
    pub fn feed(&mut self, raw: bool, n: u32) -> bool {
        if raw == self.last { self.count = 0; return self.last; }
        if raw == self.seen { self.count += 1 } else { self.seen = raw; self.count = 1 }
        if self.count >= n { self.last = raw; self.count = 0; }
        self.last
    }
}

/// ¿La ventana activa es de otra app y cubre todo su monitor (video, juego, presentación)?
/// No se usa SHQueryUserNotificationState: las ventanas de borde de Pulse cubren el monitor y Windows las toma por pantalla completa.
#[cfg(windows)]
pub fn other_app_fullscreen() -> bool {
    use windows_sys::Win32::{Foundation::RECT, Graphics::Gdi::{GetMonitorInfoW, MonitorFromWindow, MONITORINFO, MONITOR_DEFAULTTONEAREST}, UI::WindowsAndMessaging::{GetClassNameW, GetForegroundWindow, GetWindowLongW, GetWindowRect, GetWindowThreadProcessId, IsZoomed, GWL_STYLE}};
    unsafe {
        let h = GetForegroundWindow();
        if h.is_null() { return false; }
        let mut pid = 0u32; GetWindowThreadProcessId(h, &mut pid);
        if pid == std::process::id() { return false; }
        let mut name = [0u16; 64]; let n = GetClassNameW(h, name.as_mut_ptr(), name.len() as i32);
        if is_shell_surface(&String::from_utf16_lossy(&name[..n.max(0) as usize])) { return false; }
        let mut r: RECT = std::mem::zeroed();
        if GetWindowRect(h, &mut r) == 0 { return false; }
        let mut mi: MONITORINFO = std::mem::zeroed(); mi.cbSize = std::mem::size_of::<MONITORINFO>() as u32;
        if GetMonitorInfoW(MonitorFromWindow(h, MONITOR_DEFAULTTONEAREST), &mut mi) == 0 { return false; }
        let m = mi.rcMonitor;
        is_fullscreen(covers((r.left, r.top, r.right, r.bottom), (m.left, m.top, m.right, m.bottom)), IsZoomed(h) != 0, GetWindowLongW(h, GWL_STYLE) as u32)
    }
}

/// Linux (Wayland): una app no puede ver las ventanas de las demás, así que no hay detección de pantalla completa.
#[cfg(not(windows))]
pub fn other_app_fullscreen() -> bool { false }

/// Micrófono en uso por otra app: alguna entrada de ConsentStore con LastUsedTimeStop == 0.
#[cfg(windows)]
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

/// Linux: alguien graba del micrófono si PipeWire/PulseAudio tiene una captura activa (no pausada) de un micrófono,
/// sin contar las que graban lo que suena en la PC (fuentes `.monitor`).
#[cfg(not(windows))]
pub fn mic_in_use() -> bool {
    // con `timeout`: un PipeWire trabado no puede congelar la detección del foco (ni el hilo que la corre)
    let run = |args: &[&str]| std::process::Command::new("timeout").arg("2").arg("pactl").env("LC_ALL", "C").args(args).output().ok().filter(|o| o.status.success()).and_then(|o| serde_json::from_slice::<serde_json::Value>(&o.stdout).ok());
    let Some(outputs) = run(&["-f", "json", "list", "source-outputs"]) else { return false };
    let monitors: Vec<u64> = run(&["-f", "json", "list", "sources"]).and_then(|v| v.as_array().cloned()).unwrap_or_default().iter()
        .filter(|s| s["name"].as_str().is_some_and(|n| n.ends_with(".monitor"))).filter_map(|s| s["index"].as_u64()).collect();
    outputs.as_array().is_some_and(|a| a.iter().any(|o| !o["corked"].as_bool().unwrap_or(false) && !o["source"].as_u64().is_some_and(|i| monitors.contains(&i))))
}

fn local_minutes() -> u32 {
    let now = crate::now_ms();
    ((now + crate::daylog::utc_offset_ms(now)).rem_euclid(86_400_000) / 60_000) as u32
}

pub fn start(app: &AppHandle) {
    let a = app.clone();
    tauri::async_runtime::spawn(async move { let (mut last, mut stable) = (false, Debounce::default()); loop {
        tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        let st = a.state::<AppState>();
        let cfg = st.config.lock().unwrap().focus.clone();
        // la pantalla completa tiene que durar dos revisiones (~6 s) para encender o apagar el foco
        let fullscreen = stable.feed(cfg.auto && cfg.detect_fullscreen && other_app_fullscreen(), 2);
        let auto = detect(&cfg, local_minutes(), cfg.auto && cfg.detect_call && mic_in_use(), fullscreen);
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
    #[test] fn maximized_is_not_fullscreen_but_video_fullscreen_is() {
        // valores reales medidos en este equipo (barra de tareas en ocultar automáticamente)
        assert!(!is_fullscreen(true, true, 0x17CF0000));   // Warp o Brave maximizados: con barra de título y borde
        assert!(is_fullscreen(true, true, 0x170B0000));    // Brave con YouTube en pantalla completa: sigue "maximizado", sin título ni borde
        assert!(is_fullscreen(true, false, 0x16CF0000));   // pantalla completa sin maximizar
        assert!(!is_fullscreen(false, false, 0x170B0000)); // no cubre el monitor
    }
    #[test] fn fullscreen_means_covering_the_whole_monitor() {
        let mon = (0, 0, 1920, 1080);
        assert!(covers(mon, mon));
        assert!(covers((-8, -8, 1928, 1088), mon));    // ventana maximizada sin marco que se sale un poco
        assert!(!covers((0, 0, 1920, 1040), mon));     // maximizada normal: deja ver la barra de tareas
        assert!(!covers((100, 100, 900, 700), mon));
    }
    #[test] fn system_layers_are_not_fullscreen_apps() {
        assert!(is_shell_surface("Windows.UI.Core.CoreWindow"));   // TextInputHost, Inicio, búsqueda
        assert!(is_shell_surface("XamlExplorerHostIslandWindow")); // Alt+Tab
        assert!(!is_shell_surface("Chrome_WidgetWin_1"));
    }
    #[test] fn a_passing_window_does_not_switch_focus() {
        let mut d = Debounce::default();
        assert!(!d.feed(true, 2));     // un instante: no cuenta
        assert!(!d.feed(false, 2));
        assert!(!d.feed(true, 2));
        assert!(d.feed(true, 2));      // dos seguidas: sí
        assert!(d.feed(false, 2));     // para apagarlo también hacen falta dos
        assert!(!d.feed(false, 2));
    }

}
