#[derive(Default)]
pub struct LCDRegisters {
    pub lcdc: u8, // 0xFF40 - LCD Control
    pub stat: u8, // 0xFF41 - LCDC Status
    pub scy: u8,  // 0xFF42 - Scroll Y
    pub scx: u8,  // 0xFF43 - Scroll X
    pub ly: u8,   // 0xFF44 - LCD Y Coordinate
    pub lyc: u8,  // 0xFF45 - LY Compare
    pub dma: u8,  // 0xFF46 - DMA Transfer
    pub bgp: u8,  // 0xFF47 - BG Palette Data
    pub obp0: u8, // 0xFF48 - Object Palette 0
    pub obp1: u8, // 0xFF49 - Object Palette 1
    pub wy: u8,   // 0xFF4A - Window Y
    pub wx: u8,   // 0xFF4B - Window X
}

impl LCDRegisters {
    pub fn new() -> Self {
        Self {
            lcdc: 0x91, // LCD 和背景顯示開啟
            stat: 0,
            scy: 0,
            scx: 0,
            ly: 0,
            lyc: 0,
            dma: 0,
            bgp: 0xFC,  // 預設背景調色板
            obp0: 0xFF, // 預設 Sprite 調色板 0
            obp1: 0xFF, // 預設 Sprite 調色板 1
            wy: 0,
            wx: 0,
        }
    }
}
