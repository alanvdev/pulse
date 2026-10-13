use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::{Path, PathBuf}, sync::Mutex};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "lowercase")] pub enum IslandMode { Comando, Orbe }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "lowercase")] pub enum Side { Left, Center, Right }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "lowercase")] pub enum Placement { Active, All }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "lowercase")] pub enum Rule { Normal, Mudo, Nada, Despues }
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)] #[serde(rename_all = "lowercase")] pub enum Theme { Dark, Light, System }

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(default)]
pub struct IslandCfg { pub mode: IslandMode, pub orb_side: Side, pub dots: bool, pub keys: bool, pub hold: bool, pub countdown: bool, pub beam: bool, pub ask_timeout_s: u32,
    /// Esquina de la ventana de la isla en píxeles físicos, si la moviste con Ctrl + arrastrar. `None`: arriba del monitor principal.
    pub pos: Option<[i32; 2]> }
impl Default for IslandCfg { fn default() -> Self { Self { mode: IslandMode::Comando, orb_side: Side::Right, dots: true, keys: true, hold: true, countdown: true, beam: true, ask_timeout_s: 30, pos: None } } }

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(default)]
pub struct PulseCfg { pub color: String, pub shape: String, pub duration_ms: u32, pub intensity: u32, pub thickness: u32, pub persistent: bool }
impl Default for PulseCfg { fn default() -> Self { Self { color: "#b3bab5".into(), shape: "respira".into(), duration_ms: 1400, intensity: 55, thickness: 110, persistent: false } } }
fn pc(color: &str, shape: &str, duration_ms: u32, intensity: u32, thickness: u32, persistent: bool) -> PulseCfg { PulseCfg { color: color.into(), shape: shape.into(), duration_ms, intensity, thickness, persistent } }

#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(default)]
pub struct PulsesCfg { pub enabled: bool, pub placement: Placement, pub crisp: bool, pub corners: bool, pub adapt: bool, pub dither: bool, pub events: BTreeMap<String, PulseCfg> }
impl Default for PulsesCfg { fn default() -> Self { Self { enabled: true, placement: Placement::Active, crisp: true, corners: true, adapt: true, dither: true, events: BTreeMap::from([
    ("start".into(), pc("#b3bab5", "respira", 1400, 55, 110, false)),
    ("permission".into(), pc("#e3b05c", "latido", 1100, 90, 150, true)),
    ("done".into(), pc("#8fc3a0", "ola", 1900, 85, 130, false)),
    ("idle".into(), pc("#b3bab5", "respira", 2600, 40, 100, true)),
    ("error".into(), pc("#e08a7e", "lateral", 650, 90, 120, false)),
    ("subagent".into(), pc("#8fb0d6", "origen", 950, 55, 80, false)),
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

/// Versión de Grafito en Bolsillo, el flyout y Mosaico. La isla siempre es oscura.
#[derive(Debug, Clone, Serialize, Deserialize)] #[serde(default)]
pub struct AppearanceCfg { pub theme: Theme }
impl Default for AppearanceCfg { fn default() -> Self { Self { theme: Theme::Dark } } }

#[derive(Debug, Clone, Default, Serialize, Deserialize)] #[serde(default)]
pub struct Config { pub island: IslandCfg, pub pulses: PulsesCfg, pub sounds: SoundsCfg, pub focus: FocusCfg, pub keyboard: KeyboardCfg, pub mosaico: MosaicoCfg, pub appearance: AppearanceCfg }

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
pub fn load() -> Config { path().map(|p| load_at(&p)).unwrap_or_default() }
/// Un config.json que no se entiende se aparta (`config.json.bad-<segundos>`) antes de usar los de fábrica:
/// el siguiente guardado no borra tus ajustes y puedes recuperarlos a mano.
fn load_at(p: &Path) -> Config {
    let Ok(s) = std::fs::read_to_string(p) else { return Config::default() };
    serde_json::from_str(&s).unwrap_or_else(|_| {
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        let _ = std::fs::rename(p, p.with_extension(format!("json.bad-{ts}")));
        Config::default()
    })
}
pub fn save(c: &Config) { if let Some(p) = path() { save_at(&p, c); } }
/// Se escribe aparte y se renombra encima: un corte a medio guardar deja el archivo anterior, nunca uno a medias.
fn save_at(p: &Path, c: &Config) {
    static SAVING: Mutex<()> = Mutex::new(());   // arrastre de la isla y Ajustes pueden guardar a la vez
    let _g = SAVING.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(d) = p.parent() { let _ = std::fs::create_dir_all(d); }
    let Ok(s) = serde_json::to_string_pretty(c) else { return };
    let tmp = p.with_extension("json.tmp");
    if std::fs::write(&tmp, s).is_ok() { let _ = std::fs::rename(&tmp, p); }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test] fn defaults_follow_the_spec() {
        let c = Config::default();
        assert_eq!(c.island.mode, IslandMode::Comando);
        assert_eq!(c.island.ask_timeout_s, 30);
        assert_eq!(c.pulses.events["permission"].color, "#e3b05c");
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
    #[test] fn redesign_defaults() {
        let c = Config::default();
        assert!(c.island.dots);
        assert_eq!(c.appearance.theme, Theme::Dark);
        let col = |k: &str| c.pulses.events[k].color.clone();
        assert_eq!([col("start"), col("permission"), col("done"), col("idle"), col("error"), col("subagent")],
            ["#b3bab5", "#e3b05c", "#8fc3a0", "#b3bab5", "#e08a7e", "#8fb0d6"].map(String::from));
        // las formas, duraciones e intensidades no cambian
        assert_eq!((c.pulses.events["permission"].shape.as_str(), c.pulses.events["permission"].duration_ms), ("latido", 1100));
    }
    #[test] fn old_config_without_new_keys_still_loads() {
        let c: Config = serde_json::from_str(r##"{"island":{"mode":"orbe","keys":false},"pulses":{"events":{"done":{"color":"#4fe0a0"}}}}"##).unwrap();
        assert!(c.island.dots && !c.island.keys);
        assert_eq!(c.appearance.theme, Theme::Dark);
        assert_eq!(c.pulses.events["done"].color, "#4fe0a0");   // quien ya tenía config.json conserva sus colores
    }
    #[test] fn save_is_atomic_and_a_broken_file_is_kept_aside() {
        let dir = std::env::temp_dir().join(format!("pulse-test-config-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let p = dir.join("config.json");
        let mut c = Config::default(); c.island.ask_timeout_s = 60;
        save_at(&p, &c);
        assert_eq!(load_at(&p).island.ask_timeout_s, 60);
        assert!(!p.with_extension("json.tmp").exists());
        std::fs::write(&p, r#"{"island":{"mode":"rara"}}"#).unwrap();
        assert_eq!(load_at(&p).island.ask_timeout_s, 30);
        assert!(!p.exists(), "el archivo roto se aparta");
        let kept: Vec<_> = std::fs::read_dir(&dir).unwrap().filter_map(|e| e.ok()).filter(|e| e.file_name().to_string_lossy().starts_with("config.json.bad-")).collect();
        assert_eq!(kept.len(), 1);
        assert!(std::fs::read_to_string(kept[0].path()).unwrap().contains("rara"));
        let _ = std::fs::remove_dir_all(&dir);
    }
    #[test] fn theme_is_lowercase_in_json() {
        assert_eq!(serde_json::to_string(&Theme::System).unwrap(), "\"system\"");
        let c: Config = serde_json::from_str(r#"{"appearance":{"theme":"light"}}"#).unwrap();
        assert_eq!(c.appearance.theme, Theme::Light);
    }
}
