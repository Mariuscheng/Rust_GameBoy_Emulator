// LCD 控制器和狀態寄存器
#[derive(Debug, Clone)]
pub struct LCD {
    pub enabled: bool,           // LCDC.7
    pub window_map_select: bool, // LCDC.6
    pub window_enabled: bool,    // LCDC.5
    pub tile_data_select: bool,  // LCDC.4
    pub bg_map_select: bool,     // LCDC.3
    pub sprite_size: bool,       // LCDC.2
    pub sprites_enabled: bool,   // LCDC.1
    pub bg_enabled: bool,        // LCDC.0

    // LCD 狀態寄存器 (STAT)
    pub lyc_interrupt: bool,    // STAT.6
    pub oam_interrupt: bool,    // STAT.5
    pub vblank_interrupt: bool, // STAT.4
    pub hblank_interrupt: bool, // STAT.3
    pub lyc_match: bool,        // STAT.2
    pub mode: u8,               // STAT.1-0
}

impl LCD {
    pub fn new() -> Self {
        LCD {
            enabled: true,            // 開啟 LCD
            window_map_select: false, // 使用 9800-9BFF
            window_enabled: false,    // 預設關閉視窗
            tile_data_select: true,   // 使用 8000-8FFF
            bg_map_select: false,     // 使用 9800-9BFF
            sprite_size: false,       // 8x8 精靈
            sprites_enabled: false,   // 暫時關閉精靈
            bg_enabled: true,         // 啟用背景
            lyc_interrupt: false,
            oam_interrupt: false,
            vblank_interrupt: false,
            hblank_interrupt: false,
            lyc_match: false,
            mode: 0,
        }
    }

    pub fn read_lcdc(&self) -> u8 {
        (if self.enabled { 0x80 } else { 0 })
            | (if self.window_map_select { 0x40 } else { 0 })
            | (if self.window_enabled { 0x20 } else { 0 })
            | (if self.tile_data_select { 0x10 } else { 0 })
            | (if self.bg_map_select { 0x08 } else { 0 })
            | (if self.sprite_size { 0x04 } else { 0 })
            | (if self.sprites_enabled { 0x02 } else { 0 })
            | (if self.bg_enabled { 0x01 } else { 0 })
    }

    pub fn write_lcdc(&mut self, value: u8) {
        self.enabled = value & 0x80 != 0;
        self.window_map_select = value & 0x40 != 0;
        self.window_enabled = value & 0x20 != 0;
        self.tile_data_select = value & 0x10 != 0;
        self.bg_map_select = value & 0x08 != 0;
        self.sprite_size = value & 0x04 != 0;
        self.sprites_enabled = value & 0x02 != 0;
        self.bg_enabled = value & 0x01 != 0;
    }

    pub fn read_stat(&self) -> u8 {
        0x80 // Unused bit is always 1
            | (if self.lyc_interrupt { 0x40 } else { 0 })
            | (if self.oam_interrupt { 0x20 } else { 0 })
            | (if self.vblank_interrupt { 0x10 } else { 0 })
            | (if self.hblank_interrupt { 0x08 } else { 0 })
            | (if self.lyc_match { 0x04 } else { 0 })
            | self.mode
    }

    pub fn write_stat(&mut self, value: u8) {
        self.lyc_interrupt = value & 0x40 != 0;
        self.oam_interrupt = value & 0x20 != 0;
        self.vblank_interrupt = value & 0x10 != 0;
        self.hblank_interrupt = value & 0x08 != 0;
        // lyc_match and mode are read-only
    }
}
