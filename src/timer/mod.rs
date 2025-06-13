pub mod registers;

use self::registers::*;

const CPU_CLOCK_SPEED: u32 = 4_194_304; // 4.194304 MHz

pub struct Timer {
    div: u16,             // DIV 寄存器實際上是 16 位元
    div_cycles: u32,      // DIV 的週期計數器
    tima: u8,             // TIMA 寄存器
    tma: u8,              // TMA 寄存器
    tac: u8,              // TAC 寄存器
    timer_cycles: u32,    // TIMA 的週期計數器
    timer_overflow: bool, // TIMA 溢位標誌
}

impl Timer {
    pub fn new() -> Timer {
        Timer {
            div: 0,
            div_cycles: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            timer_cycles: 0,
            timer_overflow: false,
        }
    }

    pub fn update(&mut self, cycles: u8) -> bool {
        let mut interrupt_requested = false;

        // 更新 DIV (16384Hz, CPU Clock/256)
        self.div_cycles += cycles as u32;
        while self.div_cycles >= 256 {
            self.div = self.div.wrapping_add(1);
            self.div_cycles -= 256;
        }

        // 只有在定時器啟用時才更新 TIMA
        if self.tac & TAC_ENABLE != 0 {
            self.timer_cycles += cycles as u32;

            // 根據 TAC 取得目前的定時器頻率
            let clock_select = self.tac & TAC_CLOCK_SELECT;
            let cycles_per_increment = CPU_CLOCK_SPEED / CLOCK_FREQUENCIES[clock_select as usize];

            while self.timer_cycles >= cycles_per_increment {
                // TIMA 遞增
                let (new_tima, did_overflow) = self.tima.overflowing_add(1);

                if did_overflow {
                    self.timer_overflow = true;
                    self.tima = self.tma; // 重載 TMA 的值
                    interrupt_requested = true;
                } else {
                    self.tima = new_tima;
                }

                self.timer_cycles -= cycles_per_increment;
            }
        }

        interrupt_requested
    }

    // DIV 讀寫
    pub fn read_div(&self) -> u8 {
        (self.div >> 8) as u8 // 只返回高 8 位
    }

    pub fn write_div(&mut self, _value: u8) {
        // 寫入 DIV 時，它會被重置為 0
        self.div = 0;
        self.div_cycles = 0;
    }

    // TIMA 讀寫
    pub fn read_tima(&self) -> u8 {
        self.tima
    }

    pub fn write_tima(&mut self, value: u8) {
        self.tima = value;
        self.timer_overflow = false; // 清除溢位標誌
    }

    // TMA 讀寫
    pub fn read_tma(&self) -> u8 {
        self.tma
    }

    pub fn write_tma(&mut self, value: u8) {
        self.tma = value;
    }

    // TAC 讀寫
    pub fn read_tac(&self) -> u8 {
        self.tac
    }

    pub fn write_tac(&mut self, value: u8) {
        let old_enable = self.tac & TAC_ENABLE;
        self.tac = value & 0x07; // 只使用低 3 位

        // 如果定時器從啟用變為禁用，重置計數器
        if old_enable != 0 && self.tac & TAC_ENABLE == 0 {
            self.timer_cycles = 0;
        }
    }
}
