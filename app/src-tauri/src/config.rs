use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::PathBuf};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "lowercase")] pub enum IslandMode { Comando, Orbe }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "lowercase")] pub enum Side { Left, Center, Right }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "lowercase")] pub enum Placement { Active, All }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "lowercase")] pub enum Rule { Normal, Mudo, Nada, Despues }

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(default)]
pub struct IslandCfg { pub mode: IslandMode, pub orb_side: Side, pub keys: bool, pub hold: bool, pub countdown: bool, pub beam: bool, pub ask_timeout_s: u32 }
impl Default for IslandCfg { fn default() -> Self { Self { mode: IslandMode::Comando, orb_side: Side::Right, keys: true, hold: true, countdown: true, beam: true, ask_timeout_s: 30 } } }

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(default)]
pub struct PulseCfg { pub color: String, pub shape: String, pub duration_ms: u32, pub intensity: u32, pub thickness: u32, pub persistent: bool }
impl Default for PulseCfg { fn default() -> Self { Self { color: "#e6eeff".into(), shape: "respira".into(), duration_ms: 1400, intensity: 55, thickness: 110, persistent: false } } }
fn pc(color: &str, shape: &str, duration_ms: u32, intensity: u32, thickness: u32, persistent: bool) -> PulseCfg { PulseCfg { color: color.into(), shape: shape.into(), duration_ms, intensity, thickness, persistent } }

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(default)]
pub struct PulsesCfg { pub enabled: bool, pub placement: Placement, pub crisp: bool, pub corners: bool, pub adapt: bool, pub dither: bool, pub events: BTreeMap<String, PulseCfg> }
impl Default for PulsesCfg { fn default() -> Self { Self { enabled: true, placement: Placement::Active, crisp: true, corners: true, adapt: true, dither: true, events: BTreeMap::from([
    ("start".into(), pc("#e6eeff", "respira", 1400, 55, 110, false)),
    ("permission".into(), pc("#ffb02e", "latido", 1100, 90, 150, true)),
    ("done".into(), pc("#4fe0a0", "ola", 1900, 85, 130, false)),
    ("idle".into(), pc("#b6c0d4", "respira", 2600, 40, 100, true)),
    ("error".into(), pc("#ff5a52", "lateral", 650, 90, 120, false)),
    ("subagent".into(), pc("#5cc8ff", "origen", 950, 55, 80, false)),
]) } } }

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(default)]
pub struct SoundsCfg { pub enabled: bool, pub volume: u32, pub per_event: BTreeMap<String, bool> }
impl Default for SoundsCfg { fn default() -> Self { Self { enabled: true, volume: 80, per_event: ["start", "permission", "done", "idle", "error"].iter().map(|k| (k.to_string(), true)).chain([("subagent".to_string(), false)]).collect() } } }

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(default)]
pub struct FocusCfg { pub auto: bool, pub detect_call: bool, pub detect_fullscreen: bool, pub schedule: bool, pub from: String, pub to: String, pub rules: BTreeMap<String, Rule> }
impl Default for FocusCfg { fn default() -> Self { Self { auto: true, detect_call: true, detect_fullscreen: true, schedule: false, from: "09:00".into(), to: "11:00".into(), rules: BTreeMap::from([
    ("start".into(), Rule::Nada), ("permission".into(), Rule::Normal), ("done".into(), Rule::Despues),
    ("idle".into(), Rule::Despues), ("error".into(), Rule::Mudo), ("subagent".into(), Rule::Nada)]) } } }

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(default)]
pub struct KeyboardCfg { pub enabled: bool, pub session_keys: bool, pub base: u32 }
impl Default for KeyboardCfg { fn default() -> Self { Self { enabled: true, session_keys: true, base: 20 } } }

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(default)]
pub struct MosaicoCfg { pub order: Vec<String>, pub hidden: Vec<String> }
impl Default for MosaicoCfg { fn default() -> Self { Self { order: ["sess", "pend", "hoy", "acc", "foco", "pulso", "keys", "bitacora"].map(String::from).to_vec(), hidden: vec![] } } }

#[derive(Debug, Clone, Default, Serialize, Deserialize)] #[serde(default)]
pub struct Config { pub island: IslandCfg, pub pulses: PulsesCfg, pub sounds: SoundsCfg, pub focus: FocusCfg, pub keyboard: KeyboardCfg, pub mosaico: MosaicoCfg }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Route { pub visual: bool, pub sound: bool, pub defer: bool }

pub fn route(c: &Config, focus_on: bool, kind: &str) -> Route {
    let sound_on = c.sounds.enabled && c.sounds.per_event.get(kind).copied().unwrap_or(false);
    let visual_on = c.pulses.enabled;
    let rule = if focus_on { c.focus.rules.get(kind).copied().unwrap_or(Rule::Normal) } else { Rule::Normal };
    match rule {
        Rule::Normal => Route { visual: visual_on, sound: sound_on, defer: false },
        Rule::Mudo => Route { visual: visual_on, sound: false, defer: false },
        Rule::Nada => Route { visual: false, sound: false, defer: false },
        Rule::Despues => Route { visual: false, sound: false, defer: true },
    }
}

pub fn path() -> Option<PathBuf> { dirs::home_dir().map(|h| h.join(".claude").join("pulse").join("config.json")) }
pub fn load() -> Config { path().and_then(|p| std::fs::read_to_string(p).ok()).and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default() }
pub fn save(c: &Config) {
    let Some(p) = path() else { return };
    if let Some(d) = p.parent() { let _ = std::fs::create_dir_all(d); }
    if let Ok(s) = serde_json::to_string_pretty(c) { let _ = std::fs::write(p, s); }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn defaults_follow_the_spec() {
        let c = Config::default();
        assert_eq!(c.island.mode, IslandMode::Comando);
        assert_eq!(c.island.ask_timeout_s, 30);
        assert_eq!(c.pulses.events["permission"].color, "#ffb02e");
        assert!(c.pulses.events["permission"].persistent);
        assert!(!c.sounds.per_event["subagent"]);
        assert_eq!(c.focus.rules["permission"], Rule::Normal);
        assert_eq!(c.focus.rules["done"], Rule::Despues);
    }
    #[test] fn partial_json_keeps_defaults() {
        let c: Config = serde_json::from_str(r#"{"island":{"mode":"orbe"}}"#).unwrap();
        assert_eq!(c.island.mode, IslandMode::Orbe);
        assert_eq!(c.island.ask_timeout_s, 30);
        assert_eq!(c.pulses.events.len(), 6);
    }
    #[test] fn routing_by_focus_rule() {
        let c = Config::default();
        assert_eq!(route(&c, false, "done"), Route { visual: true, sound: true, defer: false });
        assert_eq!(route(&c, true, "done"), Route { visual: false, sound: false, defer: true });
        assert_eq!(route(&c, true, "error"), Route { visual: true, sound: false, defer: false });
        assert_eq!(route(&c, true, "start"), Route { visual: false, sound: false, defer: false });
        assert_eq!(route(&c, true, "permission"), Route { visual: true, sound: true, defer: false });
    }
    #[test] fn sounds_off_globally_or_per_event() {
        let mut c = Config::default();
        assert!(!route(&c, false, "subagent").sound);
        c.sounds.enabled = false;
        assert!(!route(&c, false, "done").sound);
    }
}
