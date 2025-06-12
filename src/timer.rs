// Game Boy Timer 模組
// 提供基本計時器功能

pub struct Timer {
    div: u8,                     // 除頻器 (DIV)
    tima: u8,                    // 計時器計數器 (TIMA)
    tma: u8,                     // 計時器模數 (TMA)
    tac: u8,                     // 計時器控制 (TAC)
    div_cycles: u32,             // DIV 內部週期計數器
    tima_cycles: u32,            // TIMA 內部週期計數器
    tima_overflow: bool,         // TIMA 溢出標誌
    tima_reload_scheduled: bool, // TIMA 重載計劃標誌
}

impl Timer {
    pub fn new() -> Self {
        Self {
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            div_cycles: 0,
            tima_cycles: 0,
            tima_overflow: false,
            tima_reload_scheduled: false,
        }
    }

    /// 獲取當前 TIMA 週期閾值
    fn get_tima_threshold(&self) -> u32 {
        match self.tac & 0x03 {
            0 => 1024, // 4096 Hz = 4194304 / 1024
            1 => 16,   // 262144 Hz = 4194304 / 16
            2 => 64,   // 65536 Hz = 4194304 / 64
            3 => 256,  // 16384 Hz = 4194304 / 256
            _ => unreachable!(),
        }
    }

    /// 更新計時器狀態並返回是否觸發中斷
    pub fn step(&mut self, cycles: u32) -> bool {
        let mut interrupt = false;

        // 更新 DIV (每 256 機器週期遞增)
        let prev_div = self.div_cycles;
        self.div_cycles = self.div_cycles.wrapping_add(cycles);
        if self.div_cycles < prev_div {
            // 檢查溢出
            self.div = self.div.wrapping_add(1);
        }

        // 如果有計劃的 TIMA 重載，執行它
        if self.tima_reload_scheduled {
            self.tima = self.tma;
            self.tima_reload_scheduled = false;
            self.tima_overflow = false;
        }

        // 檢查計時器是否啟用
        if self.tac & 0x04 != 0 {
            let threshold = self.get_tima_threshold();
            let prev_tima_cycles = self.tima_cycles;
            self.tima_cycles = self.tima_cycles.wrapping_add(cycles);

            // 如果週期計數器溢出或達到閾值
            while self.tima_cycles >= threshold || self.tima_cycles < prev_tima_cycles {
                if self.tima_cycles >= threshold {
                    self.tima_cycles -= threshold;
                }

                // TIMA 遞增並檢查溢出
                let (new_tima, did_overflow) = self.tima.overflowing_add(1);
                if did_overflow {
                    self.tima = 0; // TIMA 變為 0
                    self.tima_overflow = true;
                    self.tima_reload_scheduled = true; // 計劃在下一個週期重載
                    interrupt = true;
                } else {
                    self.tima = new_tima;
                }
            }
        }

        interrupt
    }

    /// 讀取計時器寄存器
    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF04 => self.div,        // DIV
            0xFF05 => self.tima,       // TIMA
            0xFF06 => self.tma,        // TMA
            0xFF07 => self.tac | 0xF8, // TAC (未使用的位返回 1)
            _ => 0xFF,
        }
    }

    /// 寫入計時器寄存器
    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF04 => {
                // 寫入 DIV 時重置所有計數器
                self.div = 0;
                self.div_cycles = 0;
                // 重置 DIV 也會影響 TIMA 的計數
                if self.tac & 0x04 != 0 {
                    self.tima_cycles = 0;
                }
            }
            0xFF05 => {
                // 只有在非重載狀態下才能寫入 TIMA
                if !self.tima_reload_scheduled {
                    self.tima = value;
                    self.tima_overflow = false;
                }
            }
            0xFF06 => {
                self.tma = value;
                // 如果正在重載，立即使用新的 TMA 值
                if self.tima_reload_scheduled {
                    self.tima = value;
                }
            }
            0xFF07 => {
                let old_tac = self.tac;
                self.tac = value & 0x07;

                // 檢查是否改變了計時器狀態或頻率
                if old_tac != self.tac {
                    let old_enabled = old_tac & 0x04 != 0;
                    let new_enabled = self.tac & 0x04 != 0;

                    if !new_enabled {
                        // 禁用計時器時重置週期計數
                        self.tima_cycles = 0;
                    } else if !old_enabled || (old_tac & 0x03) != (self.tac & 0x03) {
                        // 啟用計時器或改變頻率時重置週期計數
                        self.tima_cycles = 0;
                    }
                }
            }
            _ => {}
        }
    }

    /// 重置溢出標誌
    pub fn clear_overflow(&mut self) {
        self.tima_overflow = false;
    }

    // Getter方法
    pub fn get_div(&self) -> u8 {
        self.div
    }

    pub fn get_tima(&self) -> u8 {
        self.tima
    }

    pub fn get_tma(&self) -> u8 {
        self.tma
    }

    pub fn get_tac(&self) -> u8 {
        self.tac
    }

    // 輔助方法
    pub fn reset_div(&mut self) {
        self.div = 0;
        self.div_cycles = 0;
    }

    pub fn set_tima(&mut self, value: u8) {
        if !self.tima_reload_scheduled {
            self.tima = value;
        }
    }

    pub fn set_tma(&mut self, value: u8) {
        self.tma = value;
    }

    pub fn set_tac(&mut self, value: u8) {
        self.write(0xFF07, value);
    }

    /// 檢查計時器是否啟用
    pub fn is_enabled(&self) -> bool {
        self.tac & 0x04 != 0
    }

    /// 獲取當前頻率設置
    pub fn get_frequency(&self) -> u32 {
        match self.tac & 0x03 {
            0 => 4096,   // 4096 Hz
            1 => 262144, // 262144 Hz
            2 => 65536,  // 65536 Hz
            3 => 16384,  // 16384 Hz
            _ => unreachable!(),
        }
    }
}
