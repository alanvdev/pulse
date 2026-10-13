use pulse_protocol::{Decision, Envelope, KeyPress, Reply};
use std::sync::OnceLock;
use tokio::{io::{AsyncBufReadExt, AsyncRead, AsyncWrite, AsyncWriteExt, BufReader}, sync::{mpsc::UnboundedSender, oneshot}};

pub struct Incoming { pub env: Envelope, pub reply: Option<oneshot::Sender<Reply>> }

/// Atajos que llegan por `pulse key` (Linux). La app pone aquí a quién mandárselos.
pub static KEYS: OnceLock<UnboundedSender<String>> = OnceLock::new();

#[cfg(windows)]
mod imp {
    use tokio::net::windows::named_pipe::{NamedPipeServer, ServerOptions};
    pub type Listener = NamedPipeServer;
    pub fn create_first(name: &str) -> std::io::Result<Listener> { ServerOptions::new().first_pipe_instance(true).create(name) }
    pub async fn serve(first: Listener, name: String, tx: super::UnboundedSender<super::Incoming>) {
        let mut server = first;
        loop {
            if server.connect().await.is_err() { continue; }
            let conn = server;
            server = match ServerOptions::new().create(&name) { Ok(s) => s, Err(_) => return };
            tokio::spawn(super::connection(conn, tx.clone()));
        }
    }
}

#[cfg(unix)]
mod imp {
    use tokio::net::UnixListener;
    pub type Listener = UnixListener;
    /// El candado de `<socket>.lock`: se queda abierto mientras viva la app y el sistema lo suelta al salir (aun con un pánico).
    static LOCK: std::sync::OnceLock<std::fs::File> = std::sync::OnceLock::new();
    /// Una sola instancia. Dos hooks a la vez lanzan dos apps: el candado decide cuál se queda, así la otra no puede
    /// borrar el socket vivo de la primera. Si alguien ya contesta en el socket, falla; un archivo viejo sin nadie detrás se borra.
    pub fn create_first(name: &str) -> std::io::Result<Listener> {
        use std::os::fd::AsRawFd;
        let f = std::fs::OpenOptions::new().create(true).truncate(false).write(true).open(format!("{name}.lock"))?;
        if unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) } != 0 { return Err(std::io::ErrorKind::AddrInUse.into()); }
        let _ = LOCK.set(f);
        if std::os::unix::net::UnixStream::connect(name).is_ok() { return Err(std::io::ErrorKind::AddrInUse.into()); }
        let _ = std::fs::remove_file(name);
        UnixListener::bind(name)
    }
    pub async fn serve(first: Listener, _name: String, tx: super::UnboundedSender<super::Incoming>) {
        loop {
            let Ok((conn, _)) = first.accept().await else { continue };
            tokio::spawn(super::connection(conn, tx.clone()));
        }
    }
}

pub use imp::{create_first, serve, Listener};

async fn connection<C: AsyncRead + AsyncWrite>(conn: C, tx: UnboundedSender<Incoming>) {
    let (r, mut w) = tokio::io::split(conn);
    let mut lines = BufReader::new(r).lines();
    while let Ok(Some(line)) = lines.next_line().await {
        let Ok(env) = serde_json::from_str::<Envelope>(line.trim()) else {
            if let (Ok(k), Some(keys)) = (serde_json::from_str::<KeyPress>(line.trim()), KEYS.get()) { let _ = keys.send(k.key); }
            continue;
        };
        if env.ask {
            let (otx, orx) = oneshot::channel();
            if tx.send(Incoming { env, reply: Some(otx) }).is_err() { return; }
            // Si el cliente cierra la conexión antes de la decisión, se suelta `orx` y la app lo detecta con `is_closed()`.
            tokio::select! {
                r = orx => { let out = serde_json::to_string(&r.unwrap_or(Reply { decision: Decision::None, answers: None })).unwrap() + "\n"; let _ = w.write_all(out.as_bytes()).await; }
                _ = async { while let Ok(Some(_)) = lines.next_line().await {} } => {}
            }
            return;
        }
        let _ = tx.send(Incoming { env, reply: None });
    }
}

#[cfg(all(test, unix))]
mod tests {
    #[tokio::test] async fn only_one_instance_holds_the_socket() {
        let name = std::env::temp_dir().join(format!("pulse-test-single-{}.sock", std::process::id())).to_string_lossy().into_owned();
        let first = super::create_first(&name).unwrap();
        // la segunda pierde por el candado, aunque el socket aún no contestara, y no borra el de la primera
        assert!(super::create_first(&name).is_err());
        assert!(std::path::Path::new(&name).exists());
        drop(first);
        let _ = std::fs::remove_file(&name);
        let _ = std::fs::remove_file(format!("{name}.lock"));
    }
}
