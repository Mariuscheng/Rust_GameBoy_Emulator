pub mod audio;
pub mod system;
pub mod video;

pub use audio::AudioConfig;
pub use system::SystemConfig;
pub use video::VideoConfig;

/// 全局配置結構
#[derive(Debug, Clone)]
pub struct Config {
    pub video: VideoConfig,
    pub audio: AudioConfig,
    pub system: SystemConfig,
}

impl Config {
    pub fn new() -> Self {
        Config {
            video: VideoConfig::default(),
            audio: AudioConfig::default(),
            system: SystemConfig::default(),
        }
    }
}

/// 配置構建器
pub struct ConfigBuilder {
    config: Config,
}

impl ConfigBuilder {
    pub fn new() -> Self {
        ConfigBuilder {
            config: Config::new(),
        }
    }

    pub fn video_config(mut self, config: VideoConfig) -> Self {
        self.config.video = config;
        self
    }

    pub fn audio_config(mut self, config: AudioConfig) -> Self {
        self.config.audio = config;
        self
    }

    pub fn system_config(mut self, config: SystemConfig) -> Self {
        self.config.system = config;
        self
    }

    pub fn build(self) -> Config {
        self.config
    }
}
