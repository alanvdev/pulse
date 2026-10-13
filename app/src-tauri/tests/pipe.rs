use std::io::{BufRead, BufReader, Write};
use pulse_app_lib::server;
use pulse_protocol::{Decision, Envelope, PulseEvent};

#[cfg(windows)] fn name(tag: &str) -> String { format!(r"\\.\pipe\pulse-{tag}-{}", std::process::id()) }
#[cfg(unix)] fn name(tag: &str) -> String { std::env::temp_dir().join(format!("pulse-{tag}-{}.sock", std::process::id())).to_string_lossy().into_owned() }
#[cfg(windows)] fn connect(n: &str) -> std::fs::File { std::fs::OpenOptions::new().read(true).write(true).open(n).unwrap() }
#[cfg(unix)] fn connect(n: &str) -> std::os::unix::net::UnixStream { std::os::unix::net::UnixStream::connect(n).unwrap() }

fn env(ask: bool) -> Envelope { Envelope { v: 1, session_id: "s1".into(), project: "demo".into(), cwd: String::new(), at_ms: 5, ask, owner: Default::default(), event: PulseEvent::Done } }

#[tokio::test(flavor = "multi_thread")]
async fn delivers_plain_events() {
    let name = name("srv");
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(server::serve(server::create_first(&name).unwrap(), name.clone(), tx));
    let line = serde_json::to_string(&env(false)).unwrap() + "\n";
    let n = name.clone();
    tokio::task::spawn_blocking(move || { let mut f = connect(&n); f.write_all(line.as_bytes()).unwrap(); f.write_all(b"no es json\n").unwrap(); }).await.unwrap();
    let got = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
    assert_eq!(got.env, env(false));
    assert!(got.reply.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn answers_ask_requests() {
    let name = name("ask");
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(server::serve(server::create_first(&name).unwrap(), name.clone(), tx));
    let line = serde_json::to_string(&env(true)).unwrap() + "\n";
    let n = name.clone();
    let client = tokio::task::spawn_blocking(move || {
        let mut f = connect(&n);
        f.write_all(line.as_bytes()).unwrap();
        let mut out = String::new(); BufReader::new(f).read_line(&mut out).unwrap(); out
    });
    let inc = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
    inc.reply.unwrap().send(pulse_protocol::Reply { decision: Decision::Allow, answers: None }).unwrap();
    assert_eq!(client.await.unwrap().trim(), r#"{"decision":"allow"}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn notices_when_ask_client_goes_away() {
    let name = name("gone");
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(server::serve(server::create_first(&name).unwrap(), name.clone(), tx));
    let line = serde_json::to_string(&env(true)).unwrap() + "\n";
    let n = name.clone();
    tokio::task::spawn_blocking(move || { let mut f = connect(&n); f.write_all(line.as_bytes()).unwrap(); }).await.unwrap();   // se cierra sin esperar respuesta
    let inc = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
    let mut reply = inc.reply.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), reply.closed()).await.expect("el servidor debe soltar la espera");
}

#[tokio::test(flavor = "multi_thread")]
async fn single_instance() {
    let name = name("dup");
    let _a = server::create_first(&name).unwrap();
    assert!(server::create_first(&name).is_err());
}
