use super::registers::*;

#[derive(Debug, Clone, Copy)]
pub enum InterruptType {
    VBlank,
    LCDStat,
    Timer,
    Serial,
    Joypad,
}

impl InterruptType {
    pub fn vector(&self) -> u16 {
        match self {
            InterruptType::VBlank => VBLANK_VECTOR,
            InterruptType::LCDStat => LCD_STAT_VECTOR,
            InterruptType::Timer => TIMER_VECTOR,
            InterruptType::Serial => SERIAL_VECTOR,
            InterruptType::Joypad => JOYPAD_VECTOR,
        }
    }

    pub fn bit(&self) -> u8 {
        match self {
            InterruptType::VBlank => VBLANK_BIT,
            InterruptType::LCDStat => LCD_STAT_BIT,
            InterruptType::Timer => TIMER_BIT,
            InterruptType::Serial => SERIAL_BIT,
            InterruptType::Joypad => JOYPAD_BIT,
        }
    }
}

pub struct InterruptController {
    /// 中斷啟用寄存器 (IE)
    ie: u8,
    /// 中斷標誌寄存器 (IF)
    if_: u8,
    /// 中斷主使能標誌 (IME)
    ime: bool,
    /// 計劃啟用 IME 的標誌
    ime_scheduled: bool,
}

impl InterruptController {
    pub fn new() -> Self {
        Self {
            ie: 0,
            if_: 0,
            ime: false,
            ime_scheduled: false,
        }
    }

    /// 檢查是否有待處理的中斷
    pub fn has_pending_interrupts(&self) -> bool {
        self.ime && (self.ie & self.if_) != 0
    }

    /// 獲取最高優先級的待處理中斷
    pub fn get_highest_priority_interrupt(&self) -> Option<InterruptType> {
        if !self.ime {
            return None;
        }

        let pending = self.ie & self.if_;
        if pending & VBLANK_BIT != 0 {
            Some(InterruptType::VBlank)
        } else if pending & LCD_STAT_BIT != 0 {
            Some(InterruptType::LCDStat)
        } else if pending & TIMER_BIT != 0 {
            Some(InterruptType::Timer)
        } else if pending & SERIAL_BIT != 0 {
            Some(InterruptType::Serial)
        } else if pending & JOYPAD_BIT != 0 {
            Some(InterruptType::Joypad)
        } else {
            None
        }
    }

    /// 請求中斷
    pub fn request_interrupt(&mut self, interrupt: InterruptType) {
        self.if_ |= interrupt.bit();
    }

    /// 確認中斷已處理
    pub fn acknowledge_interrupt(&mut self, interrupt: InterruptType) {
        self.if_ &= !interrupt.bit();
    }

    /// 啟用中斷主使能（IME）
    pub fn enable_ime(&mut self) {
        self.ime = true;
    }

    /// 停用中斷主使能（IME）
    pub fn disable_ime(&mut self) {
        self.ime = false;
        self.ime_scheduled = false;
    }

    /// 計劃啟用 IME（用於 EI 指令）
    pub fn schedule_ime(&mut self) {
        self.ime_scheduled = true;
    }

    /// 更新 IME 狀態（每個指令週期調用）
    pub fn update_ime(&mut self) {
        if self.ime_scheduled {
            self.ime = true;
            self.ime_scheduled = false;
        }
    }

    /// 讀取中斷寄存器
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            IF_REGISTER => self.if_ | 0xE0, // 未使用的位讀取為 1
            IE_REGISTER => self.ie,
            _ => 0xFF,
        }
    }

    /// 寫入中斷寄存器
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            IF_REGISTER => self.if_ = value & 0x1F, // 只有低 5 位有效
            IE_REGISTER => self.ie = value,
            _ => {}
        }
    }

    /// 獲取 IME 狀態
    pub fn get_ime(&self) -> bool {
        self.ime
    }

    /// 處理 HALT 指令
    pub fn handle_halt(&self) -> bool {
        // 如果有未處理的已啟用中斷，或者發生 HALT bug，返回 false
        // HALT bug: 當 IME=0 且有未處理的已啟用中斷時發生
        let pending = self.ie & self.if_;
        !(pending != 0 && (!self.ime || pending == 0))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interrupt_request_and_acknowledge() {
        let mut controller = InterruptController::new();
        controller.enable_ime();
        controller.write(IE_REGISTER, VBLANK_BIT);

        // 請求中斷
        controller.request_interrupt(InterruptType::VBlank);
        assert_eq!(controller.if_ & VBLANK_BIT, VBLANK_BIT);

        // 確認有待處理的中斷
        assert!(controller.has_pending_interrupts());

        // 確認中斷類型
        assert!(matches!(
            controller.get_highest_priority_interrupt(),
            Some(InterruptType::VBlank)
        ));

        // 確認中斷
        controller.acknowledge_interrupt(InterruptType::VBlank);
        assert_eq!(controller.if_ & VBLANK_BIT, 0);
    }

    #[test]
    fn test_interrupt_priority() {
        let mut controller = InterruptController::new();
        controller.enable_ime();
        controller.write(IE_REGISTER, VBLANK_BIT | TIMER_BIT);

        // 同時請求兩個中斷
        controller.request_interrupt(InterruptType::Timer);
        controller.request_interrupt(InterruptType::VBlank);

        // VBlank 應該優先
        assert!(matches!(
            controller.get_highest_priority_interrupt(),
            Some(InterruptType::VBlank)
        ));

        // 確認 VBlank
        controller.acknowledge_interrupt(InterruptType::VBlank);

        // 現在應該是 Timer
        assert!(matches!(
            controller.get_highest_priority_interrupt(),
            Some(InterruptType::Timer)
        ));
    }
}
