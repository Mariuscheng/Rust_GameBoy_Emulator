/// 中斷優先級
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Interrupt {
    VBlank = 0,  // 0x40
    LCDStat = 1, // 0x48
    Timer = 2,   // 0x50
    Serial = 3,  // 0x58
    Joypad = 4,  // 0x60
}

impl Interrupt {
    pub fn vector(self) -> u16 {
        match self {
            Interrupt::VBlank => 0x40,
            Interrupt::LCDStat => 0x48,
            Interrupt::Timer => 0x50,
            Interrupt::Serial => 0x58,
            Interrupt::Joypad => 0x60,
        }
    }

    pub fn from_bit(bit: u8) -> Option<Self> {
        match bit {
            0 => Some(Interrupt::VBlank),
            1 => Some(Interrupt::LCDStat),
            2 => Some(Interrupt::Timer),
            3 => Some(Interrupt::Serial),
            4 => Some(Interrupt::Joypad),
            _ => None,
        }
    }
}

/// 中斷控制器
pub struct InterruptController {
    pub ime: bool,            // 中斷主啟用標誌
    pub enable: u8,           // 中斷啟用寄存器 (IE)
    pub flags: u8,            // 中斷標誌寄存器 (IF)
    pub pending_enable: bool, // 延遲 IME 啟用
}

impl InterruptController {
    pub fn new() -> Self {
        Self {
            ime: false,
            enable: 0,
            flags: 0,
            pending_enable: false,
        }
    }

    pub fn is_interrupt_pending(&self) -> bool {
        self.ime && (self.enable & self.flags & 0x1F) != 0
    }

    pub fn get_highest_priority_interrupt(&self) -> Option<Interrupt> {
        if !self.ime {
            return None;
        }

        let active = self.enable & self.flags & 0x1F;
        if active == 0 {
            return None;
        }

        // 檢查每個中斷位，按優先級順序
        for i in 0..5 {
            if active & (1 << i) != 0 {
                return Interrupt::from_bit(i);
            }
        }

        None
    }

    pub fn request_interrupt(&mut self, interrupt: Interrupt) {
        self.flags |= 1 << (interrupt as u8);
    }

    pub fn acknowledge_interrupt(&mut self, interrupt: Interrupt) {
        self.flags &= !(1 << (interrupt as u8));
    }
}
