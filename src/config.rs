use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub system: SystemConfig,
    pub audio: AudioConfig,
    pub video: VideoConfig,
    pub input: InputConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemConfig {
    pub debug_mode: bool,
    pub save_state_path: String,
    pub rom_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AudioConfig {
    pub enabled: bool,
    pub volume: f32,
    pub sample_rate: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VideoConfig {
    pub scale: u32,
    pub color_correction: bool,
    pub frame_blend: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InputConfig {
    pub keyboard_mapping: KeyboardMapping,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyboardMapping {
    pub up: String,
    pub down: String,
    pub left: String,
    pub right: String,
    pub a: String,
    pub b: String,
    pub start: String,
    pub select: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            system: SystemConfig::default(),
            audio: AudioConfig::default(),
            video: VideoConfig::default(),
            input: InputConfig::default(),
        }
    }
}

impl Default for SystemConfig {
    fn default() -> Self {
        Self {
            debug_mode: false,
            save_state_path: "saves".to_string(),
            rom_path: "roms".to_string(),
        }
    }
}

impl Default for AudioConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            volume: 1.0,
            sample_rate: 44100,
        }
    }
}

impl Default for VideoConfig {
    fn default() -> Self {
        Self {
            scale: 3,
            color_correction: true,
            frame_blend: false,
        }
    }
}

impl Default for InputConfig {
    fn default() -> Self {
        Self {
            keyboard_mapping: KeyboardMapping::default(),
        }
    }
}

impl Default for KeyboardMapping {
    fn default() -> Self {
        Self {
            up: "Up".to_string(),
            down: "Down".to_string(),
            left: "Left".to_string(),
            right: "Right".to_string(),
            a: "X".to_string(),
            b: "Z".to_string(),
            start: "Return".to_string(),
            select: "RShift".to_string(),
        }
    }
}

impl Config {
    pub fn load() -> Result<Self, anyhow::Error> {
        // TODO: Implement config loading from file
        Ok(Self::default())
    }

    pub fn save(&self) -> Result<(), anyhow::Error> {
        // TODO: Implement config saving to file
        Ok(())
    }
}
