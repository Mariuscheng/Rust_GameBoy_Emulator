/// PPU 和顯示相關配置
#[derive(Debug, Clone)]
pub struct VideoConfig {
    pub scale: u32,
    pub force_dmg_mode: bool,
    pub enable_bg: bool,
    pub enable_window: bool,
    pub enable_sprites: bool,
}

impl Default for VideoConfig {
    fn default() -> Self {
        VideoConfig {
            scale: 4,
            force_dmg_mode: false,
            enable_bg: true,
            enable_window: true,
            enable_sprites: true,
        }
    }
}
