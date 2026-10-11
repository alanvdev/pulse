use pulse_protocol::{Decision, Envelope, Reply};
use tokio::{io::{AsyncBufReadExt, AsyncWriteExt, BufReader}, net::windows::named_pipe::{NamedPipeServer, ServerOptions}, sync::{mpsc::UnboundedSender, oneshot}};

pub struct Incoming { pub env: Envelope, pub reply: Option<oneshot::Sender<Reply>> }

pub fn create_first(name: &str) -> std::io::Result<NamedPipeServer> { ServerOptions::new().first_pipe_instance(true).create(name) }

pub async fn serve(first: NamedPipeServer, name: String, tx: UnboundedSender<Incoming>) {
    let mut server = first;
    loop {
        if server.connect().await.is_err() { continue; }
        let conn = server;
        server = match ServerOptions::new().create(&name) { Ok(s) => s, Err(_) => return };
        let tx = tx.clone();
        tokio::spawn(async move {
            let (r, mut w) = tokio::io::split(conn);
            let mut lines = BufReader::new(r).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                let Ok(env) = serde_json::from_str::<Envelope>(line.trim()) else { continue };
                if env.ask {
                    let (otx, orx) = oneshot::channel();
                    if tx.send(Incoming { env, reply: Some(otx) }).is_err() { return; }
                    // Si el cliente cierra el pipe antes de la decisión, se suelta `orx` y la app lo detecta con `is_closed()`.
                    tokio::select! {
                        r = orx => { let out = serde_json::to_string(&r.unwrap_or(Reply { decision: Decision::None, answers: None })).unwrap() + "\n"; let _ = w.write_all(out.as_bytes()).await; }
                        _ = async { while let Ok(Some(_)) = lines.next_line().await {} } => {}
                    }
                    return;
                }
                let _ = tx.send(Incoming { env, reply: None });
            }
        });
    }
}
