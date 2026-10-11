use std::{fs::{File, OpenOptions}, io::{BufRead, BufReader, Write}, path::Path, process::{Command, Stdio}, sync::mpsc, thread, time::{Duration, Instant}};

const ERROR_FILE_NOT_FOUND: i32 = 2;
const ERROR_PIPE_BUSY: i32 = 231;
const DETACHED_PROCESS: u32 = 0x0000_0008;
const CREATE_NEW_PROCESS_GROUP: u32 = 0x0000_0200;

fn try_open(pipe: &str) -> std::io::Result<File> { OpenOptions::new().read(true).write(true).open(pipe) }

fn spawn_app(app: &Path) -> Result<(), String> {
    use std::os::windows::process::CommandExt;
    Command::new(app).creation_flags(DETACHED_PROCESS | CREATE_NEW_PROCESS_GROUP)
        .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
        .spawn().map(|_| ()).map_err(|e| format!("no se pudo lanzar {}: {e}", app.display()))
}

/// Abre el pipe. Si nadie escucha y hay `app`, la lanza y reintenta hasta 3 s.
pub fn open(pipe: &str, app: Option<&Path>) -> Result<File, String> {
    match try_open(pipe) {
        Ok(f) => return Ok(f),
        Err(e) if e.raw_os_error() == Some(ERROR_FILE_NOT_FOUND) => match app { Some(a) => spawn_app(a)?, None => return Err("no hay servidor".into()) },
        Err(e) if e.raw_os_error() == Some(ERROR_PIPE_BUSY) => {}
        Err(e) => return Err(e.to_string()),
    }
    let deadline = Instant::now() + Duration::from_millis(3000);
    while Instant::now() < deadline {
        thread::sleep(Duration::from_millis(80));
        if let Ok(f) = try_open(pipe) { return Ok(f); }
    }
    Err("la app no respondió".into())
}

pub fn send_line(pipe: &str, line: &str, app: Option<&Path>) -> Result<(), String> {
    open(pipe, app)?.write_all(line.as_bytes()).map_err(|e| e.to_string())
}

/// Manda `line` y espera una línea de respuesta hasta `timeout`. `None` si no hubo respuesta.
pub fn request(pipe: &str, line: &str, app: Option<&Path>, timeout: Duration) -> Option<String> {
    let mut f = open(pipe, app).ok()?;
    f.write_all(line.as_bytes()).ok()?;
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || { let mut s = String::new(); if BufReader::new(f).read_line(&mut s).is_ok() && !s.is_empty() { let _ = tx.send(s); } });
    rx.recv_timeout(timeout).ok().map(|s| s.trim_end().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};

    #[test] fn gives_up_fast_without_server_or_app() {
        let t = std::time::Instant::now();
        assert!(send_line(r"\\.\pipe\pulse-test-nadie", "{}\n", None).is_err());
        assert!(t.elapsed().as_millis() < 500);
    }
    #[test] fn send_line_writes_to_listener() {
        let name = format!(r"\\.\pipe\pulse-test-w-{}", std::process::id());
        let srv = test_server::create(&name);
        let n = name.clone();
        let h = std::thread::spawn(move || send_line(&n, "hola\n", None));
        let mut buf = String::new();
        test_server::accept(&srv).read_to_string(&mut buf).unwrap();
        assert!(h.join().unwrap().is_ok());
        assert_eq!(buf, "hola\n");
    }
    #[test] fn request_reads_the_reply_line() {
        let name = format!(r"\\.\pipe\pulse-test-r-{}", std::process::id());
        let srv = test_server::create(&name);
        let n = name.clone();
        let h = std::thread::spawn(move || request(&n, "pregunta\n", None, std::time::Duration::from_secs(2)));
        let mut f = test_server::accept(&srv);
        let mut line = String::new();
        BufReader::new(&f).read_line(&mut line).unwrap();
        assert_eq!(line, "pregunta\n");
        f.write_all(b"{\"decision\":\"allow\"}\n").unwrap();
        assert_eq!(h.join().unwrap().as_deref(), Some(r#"{"decision":"allow"}"#));
    }
    #[test] fn request_times_out_quietly() {
        let name = format!(r"\\.\pipe\pulse-test-t-{}", std::process::id());
        let srv = test_server::create(&name);
        let n = name.clone();
        let h = std::thread::spawn(move || request(&n, "x\n", None, std::time::Duration::from_millis(300)));
        let _f = test_server::accept(&srv);
        assert_eq!(h.join().unwrap(), None);
    }
}

#[cfg(test)]
mod test_server {
    use std::{fs::File, os::windows::io::FromRawHandle};
    use windows_sys::Win32::{Storage::FileSystem::PIPE_ACCESS_DUPLEX, System::Pipes::*};
    pub struct Server(isize);
    pub fn create(name: &str) -> Server {
        let w: Vec<u16> = name.encode_utf16().chain(std::iter::once(0)).collect();
        let h = unsafe { CreateNamedPipeW(w.as_ptr(), PIPE_ACCESS_DUPLEX, PIPE_TYPE_BYTE | PIPE_WAIT, 1, 4096, 4096, 0, std::ptr::null()) };
        assert!(h as isize != -1);
        Server(h as isize)
    }
    pub fn accept(s: &Server) -> File {
        unsafe { ConnectNamedPipe(s.0 as _, std::ptr::null_mut()) };
        unsafe { File::from_raw_handle(s.0 as _) }
    }
}
