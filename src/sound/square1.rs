use super::common::{DutyCycle, Envelope, FrequencySweep};
use super::registers::*;

/// 方波聲道 1（帶頻率掃描）
#[derive(Debug, Default)]
pub struct SquareChannel1 {
    enabled: bool,
    duty: DutyCycle,
    envelope: Envelope,
    frequency: u16,
    length_counter: u8,
    length_enable: bool,
    sweep: FrequencySweep,
    phase: u8,
}

impl SquareChannel1 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            duty: DutyCycle::Duty50,
            envelope: Envelope::new(),
            frequency: 0,
            length_counter: 0,
            length_enable: false,
            sweep: FrequencySweep::new(),
            phase: 0,
        }
    }

    pub fn read_register(&self, addr: u16) -> u8 {
        match addr {
            NR10 => (self.sweep.period << 4) | ((self.sweep.negate as u8) << 3) | self.sweep.shift,
            NR11 => ((self.duty as u8) << 6) | (64 - self.length_counter),
            NR12 => {
                (self.envelope.initial_volume << 4)
                    | ((self.envelope.direction as u8) << 3)
                    | self.envelope.sweep_pace
            }
            NR13 => self.frequency as u8,
            NR14 => ((self.length_enable as u8) << 6) | ((self.frequency >> 8) as u8),
            _ => 0xFF,
        }
    }

    pub fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            NR10 => {
                self.sweep.period = (value >> 4) & 0x07;
                self.sweep.negate = (value & 0x08) != 0;
                self.sweep.shift = value & 0x07;
            }
            NR11 => {
                self.duty = match (value >> 6) & 0x03 {
                    0 => DutyCycle::Duty125,
                    1 => DutyCycle::Duty25,
                    2 => DutyCycle::Duty50,
                    3 => DutyCycle::Duty75,
                    _ => unreachable!(),
                };
                self.length_counter = 64 - (value & 0x3F);
            }
            NR12 => {
                self.envelope.initial_volume = value >> 4;
                self.envelope.direction = (value & 0x08) != 0;
                self.envelope.sweep_pace = value & 0x07;

                if value >> 3 == 0 {
                    self.enabled = false;
                }
            }
            NR13 => {
                self.frequency = (self.frequency & 0x700) | value as u16;
            }
            NR14 => {
                self.frequency = (self.frequency & 0xFF) | ((value as u16 & 0x07) << 8);
                self.length_enable = (value & 0x40) != 0;

                if value & 0x80 != 0 {
                    self.trigger();
                }
            }
            _ => {}
        }
    }

    pub fn write_registers(&mut self, addr: u16, value: u8) {
        match addr {
            NR10 => {
                self.sweep.period = (value >> 4) & 0x07;
                self.sweep.negate = (value & 0x08) != 0;
                self.sweep.shift = value & 0x07;
            }
            NR11 => {
                self.duty = DutyCycle::from_bits((value >> 6) & 0x03);
                self.length_counter = 64 - (value & 0x3F);
            }
            NR12 => {
                self.envelope.initial_volume = (value >> 4) & 0x0F;
                self.envelope.direction = (value & 0x08) != 0;
                self.envelope.sweep_pace = value & 0x07;
                if value & 0xF8 == 0 {
                    self.enabled = false;
                }
            }
            NR13 => {
                self.frequency = (self.frequency & 0x700) | value as u16;
            }
            NR14 => {
                self.frequency = (self.frequency & 0xFF) | (((value & 0x07) as u16) << 8);
                self.length_enable = (value & 0x40) != 0;
                if value & 0x80 != 0 {
                    self.trigger();
                }
            }
            _ => {}
        }
    }

    // 寄存器讀取方法
    pub fn read_sweep(&self) -> u8 {
        (self.sweep.period << 4) | ((self.sweep.negate as u8) << 3) | self.sweep.shift
    }

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
    pub fn write_sweep(&mut self, value: u8) {
        self.sweep.period = (value >> 4) & 0x07;
        self.sweep.negate = (value & 0x08) != 0;
        self.sweep.shift = value & 0x07;
    }

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

    // 觸發聲道
    fn trigger(&mut self) {
        self.enabled = true;
        if self.length_counter == 0 {
            self.length_counter = 64;
        }
        self.envelope.reset();
        self.sweep.reset();
        self.phase = 0;
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
        self.sweep = FrequencySweep::new();
        self.phase = 0;
    }

    // 電源開啟初始化
    pub fn power_on(&mut self) {
        self.reset();
    }

    // 電源關閉處理
    pub fn power_off(&mut self) {
        self.enabled = false;
    }

    // 更新聲道
    pub fn step(&mut self) {
        if !self.enabled {
            return;
        }

        self.phase = (self.phase + 1) & 7;
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
}
