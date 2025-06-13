use super::common::Envelope;
use super::registers::{NR41, NR42, NR43, NR44};

const DIVISOR_TABLE: [u32; 8] = [8, 16, 32, 48, 64, 80, 96, 112];

pub struct NoiseChannel {
    enabled: bool,
    envelope: Envelope,
    clock_shift: u8,
    width_mode: bool,
    divisor_code: u8,
    length_counter: u8,
    length_enable: bool,
    lfsr: u16, // Linear Feedback Shift Register
    timer: u32,
    dac_enabled: bool, // 新增 DAC 狀態追蹤
}

impl NoiseChannel {
    pub fn new() -> Self {
        Self {
            enabled: false,
            envelope: Envelope::new(),
            clock_shift: 0,
            width_mode: false,
            divisor_code: 0,
            length_counter: 0,
            length_enable: false,
            lfsr: 0x7FFF,
            timer: 0,
            dac_enabled: false,
        }
    }

    // 寄存器讀取方法
    pub fn read_length(&self) -> u8 {
        64 - self.length_counter
    }

    pub fn read_envelope(&self) -> u8 {
        (self.envelope.initial_volume << 4)
            | ((self.envelope.direction as u8) << 3)
            | self.envelope.sweep_pace
    }

    pub fn read_polynomial(&self) -> u8 {
        (self.clock_shift << 4) | ((self.width_mode as u8) << 3) | self.divisor_code
    }

    pub fn read_counter(&self) -> u8 {
        ((self.length_enable as u8) << 6) | 0x3F
    }

    // 寄存器寫入方法
    pub fn write_length(&mut self, value: u8) {
        self.length_counter = 64 - (value & 0x3F);
    }

    pub fn write_envelope(&mut self, value: u8) {
        self.envelope.initial_volume = (value >> 4) & 0x0F;
        self.envelope.direction = (value & 0x08) != 0;
        self.envelope.sweep_pace = value & 0x07;

        // 檢查 DAC 電源狀態
        self.dac_enabled = value & 0xF8 != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    pub fn write_polynomial(&mut self, value: u8) {
        self.clock_shift = (value >> 4) & 0x0F;
        self.width_mode = (value & 0x08) != 0;
        self.divisor_code = value & 0x07;
        self.timer = self.get_period();
    }

    pub fn write_counter(&mut self, value: u8) {
        let trigger = (value & 0x80) != 0;
        self.length_enable = (value & 0x40) != 0;

        if trigger {
            self.trigger();
        } else if self.length_enable && self.length_counter > 0 {
            // 當設置長度啟用且計數器不為零時，立即檢查長度
            self.step_length();
        }
    }

    // 觸發通道
    fn trigger(&mut self) {
        if self.dac_enabled {
            self.enabled = true;
            if self.length_counter == 0 {
                self.length_counter = 64;
            }
            self.envelope.reset();
            self.lfsr = 0x7FFF;
            self.timer = self.get_period();
        }
    }

    // 更新通道狀態
    pub fn step(&mut self) {
        if !self.enabled {
            return;
        }

        if self.timer > 0 {
            self.timer -= 1;
            return;
        }

        self.timer = self.get_period();

        // 更新線性回饋移位暫存器
        let xor_result = if self.width_mode {
            ((self.lfsr & 0x1) ^ ((self.lfsr >> 1) & 0x1)) != 0
        } else {
            ((self.lfsr & 0x1) ^ ((self.lfsr >> 1) & 0x1)) != 0
        };

        self.lfsr >>= 1;
        if xor_result {
            self.lfsr |= 0x4000; // 設置第 14 位
            if self.width_mode {
                self.lfsr |= 0x40; // 設置第 6 位（7 位模式）
            }
        }
    }

    // 取得輸出值
    pub fn get_output(&self) -> i8 {
        if !self.enabled || !self.dac_enabled {
            return 0;
        }

        let amplitude = self.envelope.current_volume as i8;
        if (self.lfsr & 0x1) == 0 {
            amplitude.saturating_sub(8)
        } else {
            -8
        }
    }

    // 更新長度計數器
    pub fn step_length(&mut self) {
        if self.length_enable && self.length_counter > 0 {
            self.length_counter -= 1;
            if self.length_counter == 0 {
                self.enabled = false;
            }
        }
    }

    // 更新包絡
    pub fn step_envelope(&mut self) {
        if self.enabled {
            self.envelope.step(1);
        }
    }

    // 檢查通道是否啟用
    pub fn is_enabled(&self) -> bool {
        self.enabled && self.dac_enabled
    }

    // 重置通道
    pub fn reset(&mut self) {
        self.enabled = false;
        self.envelope = Envelope::new();
        self.clock_shift = 0;
        self.width_mode = false;
        self.divisor_code = 0;
        self.length_counter = 0;
        self.length_enable = false;
        self.lfsr = 0x7FFF;
        self.timer = 0;
        self.dac_enabled = false;
    }

    // 開機初始化
    pub fn power_on(&mut self) {
        self.reset();
    }

    // 關機處理
    pub fn power_off(&mut self) {
        self.enabled = false;
        self.dac_enabled = false;
    }

    // 輔助方法：計算噪音週期
    fn get_period(&self) -> u32 {
        let divisor = DIVISOR_TABLE[self.divisor_code as usize];
        divisor << self.clock_shift
    }
}
