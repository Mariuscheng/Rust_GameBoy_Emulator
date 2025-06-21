//! PPU 暫存器定義

// PPU registers addresses
pub const LCDC: u16 = 0xFF40;
pub const STAT: u16 = 0xFF41;
pub const SCY: u16 = 0xFF42;
pub const SCX: u16 = 0xFF43;
pub const LY: u16 = 0xFF44;
pub const LYC: u16 = 0xFF45;
pub const DMA: u16 = 0xFF46;
pub const BGP: u16 = 0xFF47;
pub const OBP0: u16 = 0xFF48;
pub const OBP1: u16 = 0xFF49;
pub const WY: u16 = 0xFF4A;
pub const WX: u16 = 0xFF4B;

// LCDC bits
pub const LCDC_DISPLAY_ENABLE: u8 = 1 << 7;
pub const LCDC_WINDOW_TILE_MAP: u8 = 1 << 6;
pub const LCDC_WINDOW_ENABLE: u8 = 1 << 5;
pub const LCDC_BG_TILE_DATA: u8 = 1 << 4;
pub const LCDC_BG_TILE_MAP: u8 = 1 << 3;
pub const LCDC_OBJ_SIZE: u8 = 1 << 2;
pub const LCDC_OBJ_ENABLE: u8 = 1 << 1;
pub const LCDC_BG_ENABLE: u8 = 1 << 0;

// STAT bits
pub const STAT_LYC_INTERRUPT: u8 = 1 << 6;
pub const STAT_OAM_INTERRUPT: u8 = 1 << 5;
pub const STAT_VBLANK_INTERRUPT: u8 = 1 << 4;
pub const STAT_HBLANK_INTERRUPT: u8 = 1 << 3;
pub const STAT_LYC_EQUAL: u8 = 1 << 2;
pub const STAT_MODE_BITS: u8 = 0x03;

// Default values
pub const DEFAULT_LCDC: u8 = 0x91;
pub const DEFAULT_STAT: u8 = 0x00;
pub const DEFAULT_SCY: u8 = 0x00;
pub const DEFAULT_SCX: u8 = 0x00;
pub const DEFAULT_LY: u8 = 0x00;
pub const DEFAULT_LYC: u8 = 0x00;
pub const DEFAULT_BGP: u8 = 0xFC;
pub const DEFAULT_OBP0: u8 = 0xFF;
pub const DEFAULT_OBP1: u8 = 0xFF;
pub const DEFAULT_WY: u8 = 0x00;
pub const DEFAULT_WX: u8 = 0x00;

pub trait RegisterAccess {
    fn read_register(&self, addr: u16) -> u8;
    fn write_register(&mut self, addr: u16, value: u8);
}

#[derive(Debug)]
pub struct PPURegisters {
    pub lcdc: u8,    // LCD Control
    pub stat: u8,    // LCD Status
    pub scy: u8,     // Scroll Y
    pub scx: u8,     // Scroll X
    pub ly: u8,      // LCD Y-Coordinate
    pub lyc: u8,     // LY Compare
    pub dma: u8,     // DMA Transfer
    pub bgp: u8,     // BG Palette Data
    pub obp0: u8,    // Object Palette 0 Data
    pub obp1: u8,    // Object Palette 1 Data
    pub wy: u8,      // Window Y Position
    pub wx: u8,      // Window X Position
}

impl PPURegisters {
    pub fn new() -> Self {
        Self {
            lcdc: 0x91,  // 預設值
            stat: 0x00,
            scy: 0x00,
            scx: 0x00,
            ly: 0x00,
            lyc: 0x00,
            dma: 0x00,
            bgp: 0xFC,   // 預設值
            obp0: 0xFF,
            obp1: 0xFF,
            wy: 0x00,
            wx: 0x00,
        }
    }
}

impl RegisterAccess for PPURegisters {
    fn read_register(&self, addr: u16) -> u8 {
        match addr {
            0xFF40 => self.lcdc,
            0xFF41 => self.stat,
            0xFF42 => self.scy,
            0xFF43 => self.scx,
            0xFF44 => self.ly,
            0xFF45 => self.lyc,
            0xFF46 => self.dma,
            0xFF47 => self.bgp,
            0xFF48 => self.obp0,
            0xFF49 => self.obp1,
            0xFF4A => self.wy,
            0xFF4B => self.wx,
            _ => 0xFF,
        }
    }

    fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF40 => self.lcdc = value,
            0xFF41 => self.stat = (self.stat & 0x07) | (value & 0xF8),
            0xFF42 => self.scy = value,
            0xFF43 => self.scx = value,
            0xFF44 => (), // LY is read-only
            0xFF45 => self.lyc = value,
            0xFF46 => self.dma = value,
            0xFF47 => self.bgp = value,
            0xFF48 => self.obp0 = value,
            0xFF49 => self.obp1 = value,
            0xFF4A => self.wy = value,
            0xFF4B => self.wx = value,
            _ => (),
        }
    }
}
