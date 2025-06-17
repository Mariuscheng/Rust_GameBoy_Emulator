#[allow(dead_code)] // 忽略未使用代碼警告

// LCD 控制器地址
pub const LCD_CONTROL: u16 = 0xFF40; // LCD 控制寄存器
pub const LCD_STATUS: u16 = 0xFF41; // LCD 狀態寄存器
pub const SCY: u16 = 0xFF42; // 背景捲動 Y
pub const SCX: u16 = 0xFF43; // 背景捲動 X
pub const LY: u16 = 0xFF44; // LCD Y 座標
pub const LYC: u16 = 0xFF45; // LCD Y 比較
pub const DMA: u16 = 0xFF46; // DMA 傳輸
pub const BGP: u16 = 0xFF47; // 背景調色板
pub const OBP0: u16 = 0xFF48; // 精靈調色板 0
pub const OBP1: u16 = 0xFF49; // 精靈調色板 1
pub const WINDOW_Y: u16 = 0xFF4A; // 視窗 Y 位置
pub const WINDOW_X: u16 = 0xFF4B; // 視窗 X 位置 + 7

// LCD 控制寄存器 (LCDC) 位元
pub const LCDC_ENABLE: u8 = 1 << 7; // LCD 顯示開啟/關閉
pub const LCDC_WIN_MAP: u8 = 1 << 6; // 視窗圖塊映射選擇
pub const LCDC_WIN_ENABLE: u8 = 1 << 5; // 視窗顯示開啟/關閉
pub const LCDC_TILE_DATA: u8 = 1 << 4; // 背景與視窗圖塊數據選擇
pub const LCDC_BG_MAP: u8 = 1 << 3; // 背景圖塊映射選擇
pub const LCDC_OBJ_SIZE: u8 = 1 << 2; // 精靈大小 (8x8 或 8x16)
pub const LCDC_OBJ_ENABLE: u8 = 1 << 1; // 精靈顯示開啟/關閉
pub const LCDC_BG_ENABLE: u8 = 1 << 0; // 背景顯示開啟/關閉

// 顯示相關常量
pub const SCREEN_WIDTH: usize = 160; // 螢幕寬度
pub const SCREEN_HEIGHT: usize = 144; // 螢幕高度
pub const SCANLINES_TOTAL: u8 = 154; // 總掃描線數
pub const VBLANK_START: u8 = 144; // V-Blank 開始的掃描線

// PPU 時序常量
pub const CYCLES_OAM: u32 = 80; // OAM 掃描時間 (Mode 2)
pub const CYCLES_TRANSFER: u32 = 172; // 像素傳輸時間 (Mode 3)
pub const CYCLES_HBLANK: u32 = 204; // H-Blank 時間 (Mode 0)
pub const CYCLES_PER_LINE: u32 = 456; // 每掃描線週期
pub const CYCLES_VBLANK: u32 = CYCLES_PER_LINE * 10; // V-Blank 總時間

// VRAM 地址範圍
pub const VRAM_TILE_DATA_0: u16 = 0x8000; // 圖塊數據區域 0 (使用無符號偏移)
pub const VRAM_TILE_DATA_1: u16 = 0x8800; // 圖塊數據區域 1 (使用有符號偏移)
pub const VRAM_MAP_0: u16 = 0x9800; // 圖塊地圖 0
pub const VRAM_MAP_1: u16 = 0x9C00; // 圖塊地圖 1

// LCD 狀態寄存器 (STAT) 位元
pub const STAT_LYC_INT: u8 = 1 << 6; // LYC=LY 中斷使能
pub const STAT_OAM_INT: u8 = 1 << 5; // Mode 2 OAM 中斷使能
pub const STAT_VBLANK_INT: u8 = 1 << 4; // Mode 1 V-Blank 中斷使能
pub const STAT_HBLANK_INT: u8 = 1 << 3; // Mode 0 H-Blank 中斷使能
pub const STAT_LYC_FLAG: u8 = 1 << 2; // LYC=LY 標誌
pub const STAT_MODE_MASK: u8 = 0x03; // 模式標誌遮罩

// PPU 模式常量
pub const MODE_HBLANK: u8 = 0;
pub const MODE_VBLANK: u8 = 1;
pub const MODE_OAM: u8 = 2;
pub const MODE_DRAWING: u8 = 3;

// Gameboy 調色板顏色（使用 8 位元格式：RGB332）
pub const COLOR_WHITE: u8 = 0b11111111; // 白色
pub const COLOR_LIGHT_GRAY: u8 = 0b10110110; // 淺灰
pub const COLOR_DARK_GRAY: u8 = 0b01001001; // 深灰
pub const COLOR_BLACK: u8 = 0b00000000; // 黑色

/// 應用調色板到顏色值
pub fn apply_palette(color: u8, palette: u8) -> u8 {
    (palette >> (color * 2)) & 0x03
}

/// 將 GameBoy 的顏色轉換為 8 位元 RGB332 格式
pub fn convert_color(color: u8) -> u8 {
    match color {
        0 => COLOR_WHITE,
        1 => COLOR_LIGHT_GRAY,
        2 => COLOR_DARK_GRAY,
        3 => COLOR_BLACK,
        _ => COLOR_WHITE,
    }
}

/// LCD 控制寄存器結構
#[derive(Debug)]
pub struct LCDControl {
    value: u8,
}

impl LCDControl {
    pub fn new() -> Self {
        Self { value: 0x91 } // 預設值
    }

    pub fn lcd_enable(&self) -> bool {
        (self.value & LCDC_ENABLE) != 0
    }

    pub fn window_tilemap_select(&self) -> bool {
        (self.value & LCDC_WIN_MAP) != 0
    }

    pub fn window_enable(&self) -> bool {
        (self.value & LCDC_WIN_ENABLE) != 0
    }

    pub fn bg_window_tiledata_select(&self) -> bool {
        (self.value & LCDC_TILE_DATA) != 0
    }

    pub fn bg_tilemap_select(&self) -> bool {
        (self.value & LCDC_BG_MAP) != 0
    }

    pub fn obj_size(&self) -> bool {
        (self.value & LCDC_OBJ_SIZE) != 0
    }

    pub fn obj_enable(&self) -> bool {
        (self.value & LCDC_OBJ_ENABLE) != 0
    }

    pub fn bg_window_enable(&self) -> bool {
        (self.value & LCDC_BG_ENABLE) != 0
    }

    pub fn write(&mut self, value: u8) {
        self.value = value;
    }

    pub fn read(&self) -> u8 {
        self.value
    }
}

/// LCD 狀態寄存器結構
#[derive(Debug)]
pub struct LCDStatus {
    value: u8,
}

impl LCDStatus {
    pub fn new() -> Self {
        Self { value: 0 }
    }

    pub fn lyc_interrupt_enable(&self) -> bool {
        (self.value & STAT_LYC_INT) != 0
    }

    pub fn oam_interrupt_enable(&self) -> bool {
        (self.value & STAT_OAM_INT) != 0
    }

    pub fn vblank_interrupt_enable(&self) -> bool {
        (self.value & STAT_VBLANK_INT) != 0
    }

    pub fn hblank_interrupt_enable(&self) -> bool {
        (self.value & STAT_HBLANK_INT) != 0
    }

    pub fn lyc_flag(&self) -> bool {
        (self.value & STAT_LYC_FLAG) != 0
    }

    pub fn get_mode(&self) -> u8 {
        self.value & STAT_MODE_MASK
    }

    pub fn update_mode(&mut self, mode: u8) {
        self.value = (self.value & !STAT_MODE_MASK) | (mode & STAT_MODE_MASK);
    }

    pub fn update_lyc_flag(&mut self, matched: bool) {
        if matched {
            self.value |= STAT_LYC_FLAG;
        } else {
            self.value &= !STAT_LYC_FLAG;
        }
    }

    pub fn write(&mut self, value: u8) {
        self.value = (value & 0xF8) | (self.value & STAT_MODE_MASK);
    }

    pub fn read(&self) -> u8 {
        self.value
    }
}

/// 捲動寄存器結構
#[derive(Debug)]
pub struct ScrollRegisters {
    pub scx: u8,
    pub scy: u8,
}

impl ScrollRegisters {
    pub fn new() -> Self {
        Self { scx: 0, scy: 0 }
    }
}

/// 視窗寄存器結構
#[derive(Debug)]
pub struct WindowRegisters {
    pub wx: u8,
    pub wy: u8,
}

impl WindowRegisters {
    pub fn new() -> Self {
        Self { wx: 0, wy: 0 }
    }
}

/// 調色板寄存器結構
#[derive(Debug)]
pub struct ColorPalettes {
    pub bgp: u8,
    pub obp0: u8,
    pub obp1: u8,
}

impl ColorPalettes {
    pub fn new() -> Self {
        Self {
            bgp: 0xFC,  // 預設值
            obp0: 0xFF, // 預設值
            obp1: 0xFF, // 預設值
        }
    }

    pub fn get_color(&self, palette: u8, color_id: u8) -> u32 {
        let shade = (palette >> (color_id * 2)) & 0x03;
        match shade {
            0 => 0xFFFFFFFF, // White
            1 => 0xFFAAAAAA, // Light gray
            2 => 0xFF555555, // Dark gray
            3 => 0xFF000000, // Black
            _ => 0xFFFFFFFF, // Shouldn't happen
        }
    }
}
