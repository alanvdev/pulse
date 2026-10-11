use std::io::{BufRead, BufReader, Write};
use pulse_app_lib::server;
use pulse_protocol::{Decision, Envelope, PulseEvent};

fn env(ask: bool) -> Envelope { Envelope { v: 1, session_id: "s1".into(), project: "demo".into(), cwd: String::new(), at_ms: 5, ask, event: PulseEvent::Done } }

#[tokio::test(flavor = "multi_thread")]
async fn delivers_plain_events() {
    let name = format!(r"\\.\pipe\pulse-srv-{}", std::process::id());
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(server::serve(server::create_first(&name).unwrap(), name.clone(), tx));
    let line = serde_json::to_string(&env(false)).unwrap() + "\n";
    let n = name.clone();
    tokio::task::spawn_blocking(move || { let mut f = std::fs::OpenOptions::new().write(true).open(&n).unwrap(); f.write_all(line.as_bytes()).unwrap(); f.write_all(b"no es json\n").unwrap(); }).await.unwrap();
    let got = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
    assert_eq!(got.env, env(false));
    assert!(got.reply.is_none());
}

#[tokio::test(flavor = "multi_thread")]
async fn answers_ask_requests() {
    let name = format!(r"\\.\pipe\pulse-ask-{}", std::process::id());
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(server::serve(server::create_first(&name).unwrap(), name.clone(), tx));
    let line = serde_json::to_string(&env(true)).unwrap() + "\n";
    let n = name.clone();
    let client = tokio::task::spawn_blocking(move || {
        let mut f = std::fs::OpenOptions::new().read(true).write(true).open(&n).unwrap();
        f.write_all(line.as_bytes()).unwrap();
        let mut out = String::new(); BufReader::new(f).read_line(&mut out).unwrap(); out
    });
    let inc = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
    inc.reply.unwrap().send(pulse_protocol::Reply { decision: Decision::Allow, answers: None }).unwrap();
    assert_eq!(client.await.unwrap().trim(), r#"{"decision":"allow"}"#);
}

#[tokio::test(flavor = "multi_thread")]
async fn notices_when_ask_client_goes_away() {
    let name = format!(r"\\.\pipe\pulse-gone-{}", std::process::id());
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel();
    tokio::spawn(server::serve(server::create_first(&name).unwrap(), name.clone(), tx));
    let line = serde_json::to_string(&env(true)).unwrap() + "\n";
    let n = name.clone();
    tokio::task::spawn_blocking(move || { let mut f = std::fs::OpenOptions::new().read(true).write(true).open(&n).unwrap(); f.write_all(line.as_bytes()).unwrap(); }).await.unwrap();   // se cierra sin esperar respuesta
    let inc = tokio::time::timeout(std::time::Duration::from_secs(2), rx.recv()).await.unwrap().unwrap();
    let mut reply = inc.reply.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(2), reply.closed()).await.expect("el servidor debe soltar la espera");
}

#[tokio::test(flavor = "multi_thread")]
async fn single_instance() {
    let name = format!(r"\\.\pipe\pulse-dup-{}", std::process::id());
    let _a = server::create_first(&name).unwrap();
    assert!(server::create_first(&name).is_err());
}
