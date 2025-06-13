use super::common::{DutyCycle, Envelope};
use super::registers::*;

/// 方波聲道 2（不帶頻率掃描）
pub struct SquareChannel2 {
    enabled: bool,
    duty: DutyCycle,
    envelope: Envelope,
    frequency: u16,
    length_counter: u8,
    length_enable: bool,
    phase: u8,
    timer: u32,
}

impl SquareChannel2 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            duty: DutyCycle::Duty50,
            envelope: Envelope::new(),
            frequency: 0,
            length_counter: 0,
            length_enable: false,
            phase: 0,
            timer: 0,
        }
    }

    // 寄存器讀取方法
    pub fn read_length_duty(&self) -> u8 {
        ((self.duty as u8) << 6) | (64 - self.length_counter)
    }

    pub fn read_envelope(&self) -> u8 {
        (self.envelope.initial_volume << 4)
            | ((self.envelope.direction as u8) << 3)
            | self.envelope.sweep_pace
    }

    pub fn read_frequency_lo(&self) -> u8 {
        self.frequency as u8
    }

    pub fn read_frequency_hi(&self) -> u8 {
        ((self.length_enable as u8) << 6) | ((self.frequency >> 8) as u8)
    }

    // 寄存器寫入方法
    pub fn write_length_duty(&mut self, value: u8) {
        self.duty = DutyCycle::from_bits((value >> 6) & 0x03);
        self.length_counter = 64 - (value & 0x3F);
    }

    pub fn write_envelope(&mut self, value: u8) {
        self.envelope.initial_volume = (value >> 4) & 0x0F;
        self.envelope.direction = (value & 0x08) != 0;
        self.envelope.sweep_pace = value & 0x07;
        if value & 0xF8 == 0 {
            self.enabled = false;
        }
    }

    pub fn write_frequency_lo(&mut self, value: u8) {
        self.frequency = (self.frequency & 0x700) | value as u16;
    }

    pub fn write_frequency_hi(&mut self, value: u8) {
        self.frequency = (self.frequency & 0xFF) | (((value & 0x07) as u16) << 8);
        self.length_enable = (value & 0x40) != 0;
        if value & 0x80 != 0 {
            self.trigger();
        }
    }

    pub fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            NR21 => self.write_length_duty(value),
            NR22 => self.write_envelope(value),
            NR23 => self.write_frequency_lo(value),
            NR24 => self.write_frequency_hi(value),
            _ => {}
        }
    }

    // 觸發聲道
    fn trigger(&mut self) {
        self.enabled = true;
        if self.length_counter == 0 {
            self.length_counter = 64;
        }
        self.envelope.reset();
        self.phase = 0;

        // 重置定時器
        let freq_hz = 131072 / (2048 - self.frequency) as u32;
        self.timer = 4194304 / freq_hz; // CPU頻率/音頻頻率
    }

    // 音頻生成
    pub fn step(&mut self) {
        if !self.enabled {
            return;
        }

        // 更新定時器
        if self.timer > 0 {
            self.timer -= 1;
        }

        if self.timer == 0 {
            // 更新相位
            self.phase = (self.phase + 1) & 7;

            // 重置定時器
            let freq_hz = 131072 / (2048 - self.frequency) as u32;
            self.timer = 4194304 / freq_hz;
        } // 更新包絡
        self.envelope.step(1);
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

    // 取得輸出值
    pub fn get_output(&self) -> i8 {
        if !self.enabled {
            return 0;
        }

        if self.duty.get_sample(self.phase) {
            self.envelope.current_volume as i8
        } else {
            0
        }
    }

    // 檢查聲道是否啟用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    // 重置聲道
    pub fn reset(&mut self) {
        self.enabled = false;
        self.duty = DutyCycle::Duty50;
        self.envelope = Envelope::new();
        self.frequency = 0;
        self.length_counter = 0;
        self.length_enable = false;
        self.phase = 0;
        self.timer = 0;
    }

    // 開機初始化
    pub fn power_on(&mut self) {
        self.reset();
    }

    // 關機處理
    pub fn power_off(&mut self) {
        self.enabled = false;
    }
}
