use std::path::PathBuf;

/// System-related configuration
#[derive(Debug, Clone)]
pub struct SystemConfig {
    pub bootrom_enabled: bool,
    pub debug_mode: bool,
    pub log_level: LogLevel,
    pub save_dir: String,
    pub rom_path: PathBuf,
}

#[derive(Debug, Clone, Copy)]
pub enum LogLevel {
    Error,
    Warn,
    Info,
    Debug,
    Trace,
}

impl Default for SystemConfig {
    fn default() -> Self {
        SystemConfig {
            bootrom_enabled: false,
            debug_mode: false,
            log_level: LogLevel::Info,
            save_dir: String::from("saves"),
            rom_path: PathBuf::from("rom/tetris.gb"),
        }
    }
}
