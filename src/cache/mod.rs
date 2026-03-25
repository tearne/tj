use color_eyre::Result as EyreResult;
use serde::{Deserialize, Serialize};
use stratum_dsp::{analyze_audio, AnalysisConfig};

fn home_dir() -> Option<std::path::PathBuf> {
    std::env::var_os("HOME").map(std::path::PathBuf::from)
}

pub(crate) fn hash_mono(samples: &[f32]) -> String {
    let bytes = unsafe {
        std::slice::from_raw_parts(samples.as_ptr() as *const u8, samples.len() * 4)
    };
    blake3::Hasher::new().update(bytes).finalize().to_hex().to_string()
}

pub(crate) fn cache_path() -> std::path::PathBuf {
    home_dir()
        .unwrap_or_else(|| std::path::PathBuf::from("."))
        .join(".local/share/tj/cache.json")
}

#[derive(Serialize, Deserialize, Clone)]
pub(crate) struct CacheEntry {
    pub(crate) bpm: f32,
    pub(crate) offset_ms: i64,
    /// Filename at time of first detection — informational only, not used as key.
    pub(crate) name: String,
    #[serde(default)]
    pub(crate) cue_sample: Option<usize>,
}

#[derive(Serialize, Deserialize, Default)]
pub(crate) struct CacheFile {
    #[serde(default)]
    pub(crate) last_browser_path: Option<String>,
    #[serde(default)]
    pub(crate) audio_latency_ms: i64,
    #[serde(default)]
    pub(crate) vinyl_mode: bool,
    #[serde(default)]
    pub(crate) entries: std::collections::HashMap<String, CacheEntry>,
}

pub(crate) struct Cache {
    pub(crate) path: std::path::PathBuf,
    pub(crate) last_browser_path: Option<std::path::PathBuf>,
    pub(crate) audio_latency_ms: i64,
    pub(crate) vinyl_mode: bool,
    pub(crate) entries: std::collections::HashMap<String, CacheEntry>,
}

impl Cache {
    pub(crate) fn load(path: std::path::PathBuf) -> Self {
        let file: CacheFile = std::fs::read_to_string(&path)
            .ok()
            .and_then(|text| {
                // Try new wrapped format first; fall back to legacy flat HashMap.
                serde_json::from_str::<CacheFile>(&text).ok().or_else(|| {
                    serde_json::from_str::<std::collections::HashMap<String, CacheEntry>>(&text)
                        .ok()
                        .map(|entries| CacheFile { entries, ..Default::default() })
                })
            })
            .unwrap_or_default();
        Self {
            path,
            last_browser_path: file.last_browser_path.map(std::path::PathBuf::from),
            audio_latency_ms: file.audio_latency_ms,
            vinyl_mode: file.vinyl_mode,
            entries: file.entries,
        }
    }

    pub(crate) fn get(&self, hash: &str) -> Option<&CacheEntry> {
        self.entries.get(hash)
    }

    pub(crate) fn set(&mut self, hash: String, entry: CacheEntry) {
        self.entries.insert(hash, entry);
    }

    pub(crate) fn last_browser_path(&self) -> Option<&std::path::Path> {
        self.last_browser_path.as_deref()
    }

    pub(crate) fn set_last_browser_path(&mut self, p: &std::path::Path) {
        self.last_browser_path = Some(p.to_path_buf());
    }

    pub(crate) fn get_latency(&self) -> i64 {
        self.audio_latency_ms
    }

    pub(crate) fn set_latency(&mut self, ms: i64) {
        self.audio_latency_ms = ms;
    }

    pub(crate) fn get_vinyl_mode(&self) -> bool {
        self.vinyl_mode
    }

    pub(crate) fn set_vinyl_mode(&mut self, mode: bool) {
        self.vinyl_mode = mode;
    }

    pub(crate) fn entries_snapshot(&self) -> std::collections::HashMap<String, CacheEntry> {
        self.entries.clone()
    }

    pub(crate) fn save(&self) {
        if let Some(dir) = self.path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        let tmp = self.path.with_extension("tmp");
        let file = CacheFile {
            last_browser_path: self.last_browser_path
                .as_ref()
                .and_then(|p| p.to_str().map(|s| s.to_string())),
            audio_latency_ms: self.audio_latency_ms,
            vinyl_mode: self.vinyl_mode,
            entries: self.entries.clone(),
        };
        if let Ok(text) = serde_json::to_string_pretty(&file) {
            if std::fs::write(&tmp, text).is_ok() {
                let _ = std::fs::rename(&tmp, &self.path);
            }
        }
    }
}

// ---------------------------------------------------------------------------
// BPM detection
// ---------------------------------------------------------------------------

pub(crate) fn detect_bpm(samples: &[f32], sample_rate: u32) -> EyreResult<f32> {
    let result = analyze_audio(samples, sample_rate, AnalysisConfig::default())
        .map_err(|e| color_eyre::eyre::eyre!("stratum-dsp: {e:?}"))?;
    Ok(result.bpm)
}
