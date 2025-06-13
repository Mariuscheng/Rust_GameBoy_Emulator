/// 中斷向量地址
pub const VBLANK_VECTOR: u16 = 0x0040;
pub const LCD_STAT_VECTOR: u16 = 0x0048;
pub const TIMER_VECTOR: u16 = 0x0050;
pub const SERIAL_VECTOR: u16 = 0x0058;
pub const JOYPAD_VECTOR: u16 = 0x0060;

/// 中斷寄存器地址
pub const IF_REGISTER: u16 = 0xFF0F; // 中斷標誌
pub const IE_REGISTER: u16 = 0xFFFF; // 中斷啟用

/// 中斷標誌位
pub const VBLANK_BIT: u8 = 1 << 0; // V-Blank
pub const LCD_STAT_BIT: u8 = 1 << 1; // LCD STAT
pub const TIMER_BIT: u8 = 1 << 2; // Timer
pub const SERIAL_BIT: u8 = 1 << 3; // Serial
pub const JOYPAD_BIT: u8 = 1 << 4; // Joypad
