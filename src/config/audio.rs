/// APU 和音效相關配置
#[derive(Debug, Clone)]
pub struct AudioConfig {
    pub enable_sound: bool,
    pub master_volume: f32,
    pub channel1_enabled: bool,
    pub channel2_enabled: bool,
    pub channel3_enabled: bool,
    pub channel4_enabled: bool,
}

impl Default for AudioConfig {
    fn default() -> Self {
        AudioConfig {
            enable_sound: true,
            master_volume: 1.0,
            channel1_enabled: true,
            channel2_enabled: true,
            channel3_enabled: true,
            channel4_enabled: true,
        }
    }
}
