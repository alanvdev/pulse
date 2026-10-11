use crate::state::Cue;
use rodio::{source::{SineWave, Source}, Decoder, OutputStream, Sink};
use std::{collections::HashMap, io::Cursor, sync::mpsc, time::{Duration, Instant}};
static START: &[u8] = include_bytes!("../../../assets/sounds/start.wav");
static PERMISSION: &[u8] = include_bytes!("../../../assets/sounds/permission.wav");
static DONE: &[u8] = include_bytes!("../../../assets/sounds/done.wav");
static IDLE: &[u8] = include_bytes!("../../../assets/sounds/idle.wav");
pub struct Player { tx: mpsc::Sender<(Cue, f32)> }
impl Player {
    pub fn start() -> Player {
        let (tx, rx) = mpsc::channel::<(Cue, f32)>();
        std::thread::spawn(move || {
            let Ok((_stream, handle)) = OutputStream::try_default() else { return };
            let mut last: HashMap<&'static str, Instant> = HashMap::new();
            for (cue, vol) in rx {
                if last.get(cue.key()).is_some_and(|t| t.elapsed() < Duration::from_millis(300)) { continue; }
                last.insert(cue.key(), Instant::now());
                let Ok(sink) = Sink::try_new(&handle) else { continue };
                let wav = |b: &'static [u8]| Decoder::new(Cursor::new(b)).ok().map(|d| d.amplify(vol));
                let tone = |f: f32, ms: u64, g: f32| SineWave::new(f).take_duration(Duration::from_millis(ms)).amplify(g * vol);
                match cue {
                    Cue::Start => if let Some(d) = wav(START) { sink.append(d) },
                    Cue::Permission => if let Some(d) = wav(PERMISSION) { sink.append(d) },
                    Cue::Done => if let Some(d) = wav(DONE) { sink.append(d) },
                    Cue::Idle => if let Some(d) = wav(IDLE) { sink.append(d) },
                    Cue::Error => { sink.append(tone(220.0, 90, 0.18)); sink.append(tone(170.0, 120, 0.18)); }
                    Cue::Subagent => { sink.append(tone(1320.0, 60, 0.08)); sink.append(tone(1760.0, 70, 0.08)); }
                }
                sink.detach();
            }
        });
        Player { tx }
    }
    pub fn play(&self, cue: Cue, volume: f32) { let _ = self.tx.send((cue, volume)); }
}
