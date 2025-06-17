#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Interrupt {
    VBlank = 0x40, // V-Blank 中斷
    LCDC = 0x48,   // LCDC 狀態中斷
    Timer = 0x50,  // Timer 溢出中斷
    Serial = 0x58, // Serial 傳輸完成中斷
    Joypad = 0x60, // Joypad 按鍵中斷
}

#[derive(Debug)]
pub struct InterruptRegisters {
    pub ime: bool,  // 中斷主開關
    pub ie: u8,     // 中斷啟用寄存器 (IE)
    pub if_reg: u8, // 中斷標誌寄存器 (IF)
}

impl InterruptRegisters {
    pub fn new() -> Self {
        Self {
            ime: false,
            ie: 0,
            if_reg: 0,
        }
    }

    /// 取得啟用的中斷
    pub fn get_enabled(&self) -> u8 {
        self.ie
    }

    /// 取得中斷標誌
    pub fn get_flags(&self) -> u8 {
        self.if_reg
    }

    /// 檢查中斷是否待處理
    pub fn is_pending(&self, interrupt: u8) -> bool {
        (self.ie & self.if_reg & interrupt) != 0
    }

    /// 設置中斷標誌
    pub fn set_interrupt_flag(&mut self, interrupt: u8, value: bool) {
        if value {
            self.if_reg |= interrupt;
        } else {
            self.if_reg &= !interrupt;
        }
    }

    /// 清除中斷標誌
    pub fn clear_interrupt_flag(&mut self, interrupt: u8) {
        self.if_reg &= !interrupt;
    }

    /// 請求中斷
    pub fn request_interrupt(&mut self, interrupt: u8) {
        self.if_reg |= interrupt;
    }
}

// 中斷常量定義
pub const VBLANK_BIT: u8 = 1 << 0;
pub const LCDC_BIT: u8 = 1 << 1;
pub const TIMER_BIT: u8 = 1 << 2;
pub const SERIAL_BIT: u8 = 1 << 3;
pub const JOYPAD_BIT: u8 = 1 << 4;
