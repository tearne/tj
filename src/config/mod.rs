use crossterm::event::KeyCode;

fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from)
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Action {
    Quit, Help, TerminalRefresh, VinylModeToggle,
    ZoomIn, ZoomOut, HeightIncrease, HeightDecrease,
    LatencyIncrease, LatencyDecrease,
    PaletteCycle,
    NudgeModeToggle,
    ArtCycle,
    // Deck 1
    Deck1PlayPause, Deck1OpenBrowser,
    Deck1LevelUp, Deck1LevelDown, Deck1LevelMax, Deck1LevelMin,
    Deck1FilterIncrease, Deck1FilterDecrease, Deck1FilterReset,
    Deck1FilterSlopeIncrease, Deck1FilterSlopeDecrease,
    Deck1BpmTap, Deck1MetronomeToggle, Deck1RedetectBpm,
    Deck1BpmIncrease, Deck1BpmDecrease,
    Deck1BaseBpmIncrease, Deck1BaseBpmDecrease, Deck1TempoReset,
    Deck1NudgeForward, Deck1NudgeBackward,
    Deck1OffsetIncrease, Deck1OffsetDecrease,
    Deck1JumpForward4b, Deck1JumpBackward4b,
    Deck1JumpForward8b, Deck1JumpBackward8b,
    Deck1JumpForward1bt, Deck1JumpBackward1bt,
    Deck1JumpForward4bt, Deck1JumpBackward4bt,
    Deck1Cue, Deck1CuePlay,
    Deck1PflToggle,
    Deck1GainIncrease, Deck1GainDecrease,
    Deck1PitchUp, Deck1PitchDown,
    // Deck 2
    Deck2PlayPause, Deck2OpenBrowser,
    Deck2LevelUp, Deck2LevelDown, Deck2LevelMax, Deck2LevelMin,
    Deck2FilterIncrease, Deck2FilterDecrease, Deck2FilterReset,
    Deck2FilterSlopeIncrease, Deck2FilterSlopeDecrease,
    Deck2BpmTap, Deck2MetronomeToggle, Deck2RedetectBpm,
    Deck2BpmIncrease, Deck2BpmDecrease,
    Deck2BaseBpmIncrease, Deck2BaseBpmDecrease, Deck2TempoReset,
    Deck2NudgeForward, Deck2NudgeBackward,
    Deck2OffsetIncrease, Deck2OffsetDecrease,
    Deck2JumpForward4b, Deck2JumpBackward4b,
    Deck2JumpForward8b, Deck2JumpBackward8b,
    Deck2JumpForward1bt, Deck2JumpBackward1bt,
    Deck2JumpForward4bt, Deck2JumpBackward4bt,
    Deck2Cue, Deck2CuePlay,
    Deck2PflToggle,
    Deck2GainIncrease, Deck2GainDecrease,
    Deck2PitchUp, Deck2PitchDown,
}

pub(crate) static ACTION_NAMES: &[(&str, Action)] = &[
    ("quit",              Action::Quit),
    ("help",              Action::Help),
    ("terminal_refresh",  Action::TerminalRefresh),
    ("vinyl_mode_toggle", Action::VinylModeToggle),
    ("zoom_in",           Action::ZoomIn),
    ("zoom_out",          Action::ZoomOut),
    ("height_increase",   Action::HeightIncrease),
    ("height_decrease",   Action::HeightDecrease),
    ("latency_increase",  Action::LatencyIncrease),
    ("latency_decrease",  Action::LatencyDecrease),
    ("palette_cycle",     Action::PaletteCycle),
    ("nudge_mode_toggle", Action::NudgeModeToggle),
    ("art_cycle",         Action::ArtCycle),
    // Deck 1
    ("deck1_play_pause",        Action::Deck1PlayPause),
    ("deck1_open_browser",      Action::Deck1OpenBrowser),
    ("deck1_level_up",          Action::Deck1LevelUp),
    ("deck1_level_down",        Action::Deck1LevelDown),
    ("deck1_level_max",         Action::Deck1LevelMax),
    ("deck1_level_min",         Action::Deck1LevelMin),
    ("deck1_filter_increase",        Action::Deck1FilterIncrease),
    ("deck1_filter_decrease",        Action::Deck1FilterDecrease),
    ("deck1_filter_reset",           Action::Deck1FilterReset),
    ("deck1_filter_slope_increase",  Action::Deck1FilterSlopeIncrease),
    ("deck1_filter_slope_decrease",  Action::Deck1FilterSlopeDecrease),
    ("deck1_bpm_tap",           Action::Deck1BpmTap),
    ("deck1_metronome",         Action::Deck1MetronomeToggle),
    ("deck1_redetect_bpm",      Action::Deck1RedetectBpm),
    ("deck1_bpm_increase",      Action::Deck1BpmIncrease),
    ("deck1_bpm_decrease",      Action::Deck1BpmDecrease),
    ("deck1_base_bpm_increase", Action::Deck1BaseBpmIncrease),
    ("deck1_base_bpm_decrease", Action::Deck1BaseBpmDecrease),
    ("deck1_tempo_reset",       Action::Deck1TempoReset),
    ("deck1_nudge_forward",     Action::Deck1NudgeForward),
    ("deck1_nudge_backward",    Action::Deck1NudgeBackward),
    ("deck1_offset_increase",   Action::Deck1OffsetIncrease),
    ("deck1_offset_decrease",   Action::Deck1OffsetDecrease),
    ("deck1_jump_forward_4b",   Action::Deck1JumpForward4b),
    ("deck1_jump_backward_4b",  Action::Deck1JumpBackward4b),
    ("deck1_jump_forward_8b",   Action::Deck1JumpForward8b),
    ("deck1_jump_backward_8b",  Action::Deck1JumpBackward8b),
    ("deck1_jump_forward_1bt",  Action::Deck1JumpForward1bt),
    ("deck1_jump_backward_1bt", Action::Deck1JumpBackward1bt),
    ("deck1_jump_forward_4bt",  Action::Deck1JumpForward4bt),
    ("deck1_jump_backward_4bt", Action::Deck1JumpBackward4bt),
    ("deck1_cue",               Action::Deck1Cue),
    ("deck1_cue_play",          Action::Deck1CuePlay),
    ("deck1_pfl_toggle",        Action::Deck1PflToggle),
    ("deck1_gain_increase",     Action::Deck1GainIncrease),
    ("deck1_gain_decrease",     Action::Deck1GainDecrease),
    ("deck1_pitch_up",          Action::Deck1PitchUp),
    ("deck1_pitch_down",        Action::Deck1PitchDown),
    // Deck 2
    ("deck2_play_pause",        Action::Deck2PlayPause),
    ("deck2_open_browser",      Action::Deck2OpenBrowser),
    ("deck2_level_up",          Action::Deck2LevelUp),
    ("deck2_level_down",        Action::Deck2LevelDown),
    ("deck2_level_max",         Action::Deck2LevelMax),
    ("deck2_level_min",         Action::Deck2LevelMin),
    ("deck2_filter_increase",        Action::Deck2FilterIncrease),
    ("deck2_filter_decrease",        Action::Deck2FilterDecrease),
    ("deck2_filter_reset",           Action::Deck2FilterReset),
    ("deck2_filter_slope_increase",  Action::Deck2FilterSlopeIncrease),
    ("deck2_filter_slope_decrease",  Action::Deck2FilterSlopeDecrease),
    ("deck2_bpm_tap",           Action::Deck2BpmTap),
    ("deck2_metronome",         Action::Deck2MetronomeToggle),
    ("deck2_redetect_bpm",      Action::Deck2RedetectBpm),
    ("deck2_bpm_increase",      Action::Deck2BpmIncrease),
    ("deck2_bpm_decrease",      Action::Deck2BpmDecrease),
    ("deck2_base_bpm_increase", Action::Deck2BaseBpmIncrease),
    ("deck2_base_bpm_decrease", Action::Deck2BaseBpmDecrease),
    ("deck2_tempo_reset",       Action::Deck2TempoReset),
    ("deck2_nudge_forward",     Action::Deck2NudgeForward),
    ("deck2_nudge_backward",    Action::Deck2NudgeBackward),
    ("deck2_offset_increase",   Action::Deck2OffsetIncrease),
    ("deck2_offset_decrease",   Action::Deck2OffsetDecrease),
    ("deck2_jump_forward_4b",   Action::Deck2JumpForward4b),
    ("deck2_jump_backward_4b",  Action::Deck2JumpBackward4b),
    ("deck2_jump_forward_8b",   Action::Deck2JumpForward8b),
    ("deck2_jump_backward_8b",  Action::Deck2JumpBackward8b),
    ("deck2_jump_forward_1bt",  Action::Deck2JumpForward1bt),
    ("deck2_jump_backward_1bt", Action::Deck2JumpBackward1bt),
    ("deck2_jump_forward_4bt",  Action::Deck2JumpForward4bt),
    ("deck2_jump_backward_4bt", Action::Deck2JumpBackward4bt),
    ("deck2_cue",               Action::Deck2Cue),
    ("deck2_cue_play",          Action::Deck2CuePlay),
    ("deck2_pfl_toggle",        Action::Deck2PflToggle),
    ("deck2_gain_increase",     Action::Deck2GainIncrease),
    ("deck2_gain_decrease",     Action::Deck2GainDecrease),
    ("deck2_pitch_up",          Action::Deck2PitchUp),
    ("deck2_pitch_down",        Action::Deck2PitchDown),
];

#[derive(Hash, Eq, PartialEq)]
pub(crate) enum KeyBinding {
    Key(KeyCode),
    SpaceChord(KeyCode),
}

pub(crate) fn parse_key(s: &str) -> Option<KeyBinding> {
    if let Some(rest) = s.strip_prefix("space+") {
        return parse_bare_key(rest).map(KeyBinding::SpaceChord);
    }
    parse_bare_key(s).map(KeyBinding::Key)
}

pub(crate) fn parse_bare_key(s: &str) -> Option<KeyCode> {
    match s {
        "space"     => Some(KeyCode::Char(' ')),
        "left"      => Some(KeyCode::Left),
        "right"     => Some(KeyCode::Right),
        "up"        => Some(KeyCode::Up),
        "down"      => Some(KeyCode::Down),
        "enter"     => Some(KeyCode::Enter),
        "backspace" => Some(KeyCode::Backspace),
        "esc"       => Some(KeyCode::Esc),
        s if s.chars().count() == 1 => Some(KeyCode::Char(s.chars().next().unwrap())),
        other => {
            eprintln!("deck: unknown key {:?} in config — binding skipped", other);
            None
        }
    }
}

pub(crate) const DEFAULT_CONFIG: &str = include_str!("../../resources/config.toml");

pub(crate) struct DisplayConfig {
    pub(crate) playhead_position: u8,      // 0–100, clamped
    pub(crate) warning_threshold_secs: f32, // seconds before end to activate warning flash
    pub(crate) detail_height: usize,       // total rows per detail waveform (including 2-row tick area)
}

impl Default for DisplayConfig {
    fn default() -> Self { Self { playhead_position: 20, warning_threshold_secs: 30.0, detail_height: 6 } }
}

/// Finds or creates the config file and returns its text plus an optional notice.
pub(crate) fn resolve_config() -> (String, Option<String>) {
    // Check next to the binary first, then ~/.config/tj/config.toml, then auto-create.
    let adjacent = std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(|d| d.join("config.toml")))
        .filter(|p| p.exists());
    if let Some(path) = adjacent {
        return (std::fs::read_to_string(&path).unwrap_or_default(), None);
    }
    let user_path = match home_dir() {
        Some(h) => h.join(".config/deck/config.toml"),
        None => return (DEFAULT_CONFIG.to_string(), None),
    };
    if user_path.exists() {
        (std::fs::read_to_string(&user_path).unwrap_or_default(), None)
    } else {
        // Auto-create from embedded default.
        if let Some(dir) = user_path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let notice = if std::fs::write(&user_path, DEFAULT_CONFIG).is_ok() {
            Some(format!("config created: {}", user_path.display()))
        } else {
            None
        };
        (DEFAULT_CONFIG.to_string(), notice)
    }
}

pub(crate) fn load_config() -> (std::collections::HashMap<KeyBinding, Action>, DisplayConfig, Option<String>) {
    let (text, notice) = resolve_config();
    // Seed with defaults so any keys absent from the user config still work.
    let mut map = parse_keymap(DEFAULT_CONFIG, &mut std::collections::HashMap::new());
    let keymap = parse_keymap(&text, &mut map);
    let display = parse_display_config(&text);
    (keymap, display, notice)
}

pub(crate) fn parse_display_config(text: &str) -> DisplayConfig {
    let parsed: toml::Value = match toml::from_str(text) {
        Ok(v) => v,
        Err(_) => return DisplayConfig::default(),
    };
    let display = parsed.get("display");
    let pos = display
        .and_then(|v| v.get("playhead_position"))
        .and_then(|v| v.as_integer())
        .unwrap_or(20)
        .clamp(0, 100) as u8;
    let warning_threshold_secs = display
        .and_then(|v| v.get("warning_threshold_secs"))
        .and_then(|v| v.as_float().or_else(|| v.as_integer().map(|i| i as f64)))
        .unwrap_or(30.0)
        .max(0.0) as f32;
    let detail_height = display
        .and_then(|v| v.get("detail_height"))
        .and_then(|v| v.as_integer())
        .unwrap_or(6)
        .max(3) as usize;
    DisplayConfig { playhead_position: pos, warning_threshold_secs, detail_height }
}

pub(crate) fn parse_keymap(text: &str, map: &mut std::collections::HashMap<KeyBinding, Action>)
    -> std::collections::HashMap<KeyBinding, Action>
{
    let parsed: toml::Value = match toml::from_str(text) {
        Ok(v) => v,
        Err(e) => { eprintln!("deck: failed to parse config: {e}"); return std::mem::take(map); }
    };
    let keys = match parsed.get("keys").and_then(|v| v.as_table()) {
        Some(t) => t,
        None => return std::mem::take(map),
    };
    for (name, val) in keys {
        let action = match ACTION_NAMES.iter().find(|(n, _)| *n == name.as_str()) {
            Some((_, a)) => *a,
            None => { eprintln!("deck: unknown function {name:?} in config — skipped"); continue; }
        };
        let key_strs: Vec<&str> = if let Some(s) = val.as_str() {
            vec![s]
        } else if let Some(arr) = val.as_array() {
            arr.iter().filter_map(|v| v.as_str()).collect()
        } else {
            eprintln!("deck: key value for {name:?} must be a string or array of strings");
            continue;
        };
        for key_str in key_strs {
            if let Some(binding) = parse_key(key_str) {
                map.insert(binding, action);
            }
        }
    }
    std::mem::take(map)
}
