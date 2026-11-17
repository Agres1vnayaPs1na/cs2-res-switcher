//! Loading and creating `config.toml` next to the executable.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};

/// How a non-native mode is presented on the panel (DEVMODE dmDisplayFixedOutput).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Scaling {
    /// Don't request anything; use whatever the driver/monitor is set to.
    Driver,
    /// Stretch to full screen (classic 4:3 stretched).
    Stretch,
    /// Centered 1:1 with black borders.
    Center,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Executable name of the game to watch, e.g. "cs2.exe" (case-insensitive).
    pub process_name: String,
    /// Target width in pixels.
    pub width: u32,
    /// Target height in pixels.
    pub height: u32,
    /// Target refresh rate in Hz.
    pub refresh_rate: u32,
    /// How often to check for the process, in milliseconds.
    pub poll_interval_ms: u64,
    /// "stretch" | "center" | "driver". Asks the GPU driver to scale the
    /// game mode itself, which can avoid a full monitor re-sync.
    pub scaling: Scaling,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            process_name: "cs2.exe".into(),
            width: 1280,
            height: 960,
            refresh_rate: 0,
            poll_interval_ms: 1000,
            scaling: Scaling::Stretch,
        }
    }
}

const HEADER: &str = "\
# cs2-res-switcher config
# process_name     - game executable to watch (case-insensitive)
# width / height   - resolution applied to the PRIMARY monitor while the game runs
# refresh_rate     - refresh rate in Hz for that mode (0 = highest the monitor supports)
# poll_interval_ms - how often to check for the process (ms, min 100)
# scaling          - \"stretch\" (4:3 stretched), \"center\" (black bars) or \"driver\" (don't touch)
#                    For the fastest switch also set NVIDIA \"Perform scaling on: GPU\".

";

/// Directory that contains the running executable (falls back to the current dir).
pub fn exe_dir() -> PathBuf {
    std::env::current_exe()
        .ok()
        .and_then(|p| p.parent().map(Path::to_path_buf))
        .unwrap_or_else(|| PathBuf::from("."))
}

pub fn config_path() -> PathBuf {
    exe_dir().join("config.toml")
}

/// Loads the config, creating a default file on first run.
/// Returns the config plus an optional warning to log.
pub fn load() -> (Config, Option<String>) {
    let path = config_path();

    if !path.exists() {
        let cfg = Config::default();
        let warning = match write_default(&path, &cfg) {
            Ok(()) => Some(format!("Created default config at {}", path.display())),
            Err(e) => Some(format!("Could not create {}: {e}", path.display())),
        };
        return (cfg, warning);
    }

    match fs::read_to_string(&path)
        .map_err(|e| e.to_string())
        .and_then(|s| toml::from_str::<Config>(&s).map_err(|e| e.to_string()))
    {
        Ok(mut cfg) => {
            let warning = cfg.sanitize();
            (cfg, warning)
        }
        Err(e) => (
            Config::default(),
            Some(format!("Invalid {}: {e}. Using defaults.", path.display())),
        ),
    }
}

fn write_default(path: &Path, cfg: &Config) -> std::io::Result<()> {
    let body = toml::to_string_pretty(cfg).map_err(std::io::Error::other)?;
    fs::write(path, format!("{HEADER}{body}"))
}

impl Config {
    /// Clamps obviously wrong values; returns a warning if anything changed.
    fn sanitize(&mut self) -> Option<String> {
        let mut notes = Vec::new();
        if self.poll_interval_ms < 100 {
            notes.push(format!("poll_interval_ms {} -> 100", self.poll_interval_ms));
            self.poll_interval_ms = 100;
        }
        if self.process_name.trim().is_empty() {
            notes.push("empty process_name -> cs2.exe".to_string());
            self.process_name = "cs2.exe".into();
        }
        if self.width == 0 || self.height == 0 {
            let d = Config::default();
            notes.push("zero width/height -> defaults".to_string());
            self.width = d.width;
            self.height = d.height;
        }
        (!notes.is_empty()).then(|| format!("Config adjusted: {}", notes.join(", ")))
    }
}
