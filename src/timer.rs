// Game Boy Timer 模組
// 提供基本計時器功能

pub struct Timer {
    div: u8,
    tima: u8,
    tma: u8,
    tac: u8,
    cycles: u32,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            cycles: 0,
        }
    }

    pub fn step(&mut self, cycles: u32) -> bool {
        self.cycles += cycles;

        // 更新 DIV 寄存器（16384 Hz）
        if self.cycles >= 256 {
            self.div = self.div.wrapping_add(1);
            self.cycles -= 256;
        }

        // 檢查計時器是否啟用
        if self.tac & 0x04 != 0 {
            let threshold = match self.tac & 0x03 {
                0 => 1024, // 4096 Hz
                1 => 16,   // 262144 Hz
                2 => 64,   // 65536 Hz
                3 => 256,  // 16384 Hz
                _ => 1024,
            };

            if self.cycles >= threshold {
                self.tima = self.tima.wrapping_add(1);
                self.cycles -= threshold;

                // 檢查溢出
                if self.tima == 0 {
                    self.tima = self.tma;
                    return true; // 產生計時器中斷
                }
            }
        }

        false
    }

    pub fn read(&self, addr: u16) -> u8 {
        match addr {
            0xFF04 => self.div,
            0xFF05 => self.tima,
            0xFF06 => self.tma,
            0xFF07 => self.tac,
            _ => 0xFF,
        }
    }

    pub fn write(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF04 => {
                self.div = 0;
                self.cycles = 0;
            }
            0xFF05 => self.tima = value,
            0xFF06 => self.tma = value,
            0xFF07 => self.tac = value & 0x07,
            _ => {}
        }
    }
    pub fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF04 => {
                // DIV register (reset to 0 on write)
                // Implement your logic here if needed
            }
            0xFF05 => {
                // TIMA register
                // Implement your logic here if needed
            }
            0xFF06 => {
                // TMA register
                // Implement your logic here if needed
            }
            0xFF07 => {
                // TAC register
                // Implement your logic here if needed
            }
            _ => {}
        }
    }
}
