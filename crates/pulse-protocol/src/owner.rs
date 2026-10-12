//! El proceso de Claude Code dueño de una sesión. `pulse.exe` lo busca entre sus ancestros (Claude → shell → pulse)
//! y la app revisa si sigue vivo: cerrar la pestaña de la terminal no manda `SessionEnd`.
use serde::{Deserialize, Serialize};

/// `pid` 0 = desconocido (pulse.exe viejo, o lanzado fuera de Claude Code, como el demo).
/// `started` (hora de creación en FILETIME) distingue al proceso de otro que reciba el mismo pid después.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct Owner { pub pid: u32, pub started: u64 }

/// Nombres del ejecutable de Claude Code: el instalador nativo trae `claude.exe`; la versión de npm corre en `node.exe`.
#[cfg_attr(not(windows), allow(dead_code))]
const NAMES: [&str; 2] = ["claude.exe", "node.exe"];

#[cfg(windows)]
mod imp {
    use super::{Owner, NAMES};
    use std::collections::HashMap;
    use windows_sys::Win32::Foundation::{CloseHandle, GetLastError, ERROR_ACCESS_DENIED, FILETIME, INVALID_HANDLE_VALUE, STILL_ACTIVE};
    use windows_sys::Win32::System::Diagnostics::ToolHelp::{CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, PROCESSENTRY32W, TH32CS_SNAPPROCESS};
    use windows_sys::Win32::System::Threading::{GetCurrentProcessId, GetExitCodeProcess, GetProcessTimes, OpenProcess, PROCESS_QUERY_LIMITED_INFORMATION};

    /// pid → (pid del padre, nombre del ejecutable en minúsculas).
    fn processes() -> HashMap<u32, (u32, String)> {
        let mut out = HashMap::new();
        unsafe {
            let snap = CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0);
            if snap == INVALID_HANDLE_VALUE { return out; }
            let mut e: PROCESSENTRY32W = std::mem::zeroed();
            e.dwSize = std::mem::size_of::<PROCESSENTRY32W>() as u32;
            let mut ok = Process32FirstW(snap, &mut e) != 0;
            while ok {
                let len = e.szExeFile.iter().position(|&c| c == 0).unwrap_or(e.szExeFile.len());
                out.insert(e.th32ProcessID, (e.th32ParentProcessID, String::from_utf16_lossy(&e.szExeFile[..len]).to_lowercase()));
                ok = Process32NextW(snap, &mut e) != 0;
            }
            CloseHandle(snap);
        }
        out
    }

    /// Hora de creación del proceso, o `None` si ya no existe. `Some(0)`: existe pero Windows no deja abrirlo.
    pub(super) fn started(pid: u32) -> Option<u64> {
        unsafe {
            let h = OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION, 0, pid);
            if h.is_null() { return (GetLastError() == ERROR_ACCESS_DENIED).then_some(0); }
            let (mut code, z) = (0u32, FILETIME { dwLowDateTime: 0, dwHighDateTime: 0 });
            let (mut c, mut x, mut k, mut u) = (z, z, z, z);
            let alive = GetExitCodeProcess(h, &mut code) != 0 && code == STILL_ACTIVE as u32;
            let ok = GetProcessTimes(h, &mut c, &mut x, &mut k, &mut u) != 0;
            CloseHandle(h);
            alive.then_some(if ok { (c.dwHighDateTime as u64) << 32 | c.dwLowDateTime as u64 } else { 0 })
        }
    }

    pub fn find() -> Owner {
        let all = processes();
        let mut pid = unsafe { GetCurrentProcessId() };
        for _ in 0..16 {
            let Some((parent, _)) = all.get(&pid) else { break };
            pid = *parent;
            match all.get(&pid) {
                Some((_, name)) if NAMES.contains(&name.as_str()) => return Owner { pid, started: started(pid).unwrap_or(0) },
                Some(_) => {}
                None => break,
            }
        }
        Owner::default()
    }

    pub fn is_alive(o: Owner) -> bool {
        match started(o.pid) { None => false, Some(t) => o.started == 0 || t == 0 || t == o.started }
    }
}

#[cfg(not(windows))]
mod imp {
    use super::Owner;
    pub fn find() -> Owner { Owner::default() }
    pub fn is_alive(_: Owner) -> bool { true }
}

/// El Claude Code que lanzó a este proceso (o `Owner::default()` si no hay uno entre sus ancestros).
pub fn find() -> Owner { imp::find() }

/// ¿Sigue abierto? Un dueño desconocido (pid 0) siempre cuenta como vivo.
pub fn is_alive(o: Owner) -> bool { o.pid == 0 || imp::is_alive(o) }

#[cfg(all(test, windows))]
mod tests {
    use super::*;
    #[test] fn this_process_is_alive_and_a_finished_one_is_not() {
        let me = Owner { pid: std::process::id(), started: imp::started(std::process::id()).unwrap() };
        assert!(is_alive(me));
        assert!(!is_alive(Owner { started: me.started + 1, ..me }), "otro proceso con el mismo pid");
        let mut child = std::process::Command::new("cmd").args(["/C", "exit"]).spawn().unwrap();
        let pid = child.id();
        child.wait().unwrap();
        drop(child);   // suelta el handle: así Windows libera el pid
        assert!(!is_alive(Owner { pid, started: 1 }));
    }
    #[test] fn unknown_owner_counts_as_alive() { assert!(is_alive(Owner::default())); }
}
