#[allow(dead_code)]
#[allow(unused_imports)]
use super::registers::*;

/// Game Boy 音效通道的通用介面
pub trait AudioChannel {
    /// 初始化聲道
    fn init(&mut self);

    /// 重置聲道狀態
    fn reset(&mut self);

    /// 更新聲道狀態
    fn step(&mut self, cycles: u32);

    /// 取得當前樣本值 (-1.0 到 1.0 之間)
    fn get_sample(&self) -> f32;

    /// 聲道是否已啟用
    fn is_enabled(&self) -> bool;

    /// 設定聲道啟用狀態
    fn set_enabled(&mut self, enabled: bool);

    /// 寫入聲道寄存器
    fn write_register(&mut self, addr: u16, value: u8);

    /// 讀取聲道寄存器
    fn read_register(&self, addr: u16) -> u8;

    /// 取得聲道長度計數器值
    fn get_length(&self) -> u8;

    /// 設定聲道長度計數器值
    fn set_length(&mut self, value: u8);

    /// 設定音量包絡參數
    fn set_envelope(&mut self, initial_volume: u8, direction: bool, period: u8);

    /// 取得當前音量 (0-15)
    fn get_volume(&self) -> u8;
}

/// 方波聲道的佔空比配置
#[derive(Clone, Copy, Debug, Default)]
pub enum DutyCycle {
    #[default]
    Duty125 = 0, // 12.5%
    Duty25 = 1, // 25%
    Duty50 = 2, // 50%
    Duty75 = 3, // 75%
}

impl DutyCycle {
    pub fn from_bits(bits: u8) -> Self {
        match bits & 0x03 {
            0 => DutyCycle::Duty125,
            1 => DutyCycle::Duty25,
            2 => DutyCycle::Duty50,
            3 => DutyCycle::Duty75,
            _ => unreachable!(),
        }
    }

    pub fn get_sample(&self, phase: u8) -> bool {
        let pattern = self.get_pattern();
        (pattern & (1 << (phase & 7))) != 0
    }

    pub fn get_pattern(&self) -> u8 {
        match self {
            DutyCycle::Duty125 => 0b00000001,
            DutyCycle::Duty25 => 0b00000011,
            DutyCycle::Duty50 => 0b00001111,
            DutyCycle::Duty75 => 0b11111100,
        }
    }

    pub fn set_pattern(&mut self, value: u8) {
        *self = DutyCycle::from_bits(value);
    }
}

/// 音量包絡配置
#[derive(Clone, Debug, Default)]
pub struct Envelope {
    pub initial_volume: u8,
    pub direction: bool, // true = 增加，false = 減少
    pub current_volume: u8,
    pub period: u8,
    timer: u32,
    sweep_timer: u8,
}

impl Envelope {
    pub fn new() -> Self {
        Self {
            initial_volume: 0,
            direction: false,
            current_volume: 0,
            period: 0,
            timer: 0,
            sweep_timer: 0,
        }
    }

    pub fn get_register(&self) -> u8 {
        (self.initial_volume << 4) | ((self.direction as u8) << 3) | self.period
    }

    pub fn set_register(&mut self, value: u8) {
        self.initial_volume = (value >> 4) & 0x0F;
        self.direction = (value & 0x08) != 0;
        self.period = value & 0x07;
        self.current_volume = self.initial_volume;
    }

    pub fn trigger(&mut self) {
        self.current_volume = self.initial_volume;
        self.timer = 0;
        self.sweep_timer = 0;
    }

    pub fn step(&mut self, cycles: u32) {
        if self.period == 0 {
            return;
        }

        self.timer = self.timer.wrapping_add(cycles);

        // 每64個週期檢查一次
        if self.timer >= 64 {
            self.timer -= 64;
            self.sweep_timer = self.sweep_timer.wrapping_add(1);

            if self.sweep_timer >= self.period {
                self.sweep_timer = 0;

                // 更新音量
                if self.direction && self.current_volume < 15 {
                    self.current_volume += 1;
                } else if !self.direction && self.current_volume > 0 {
                    self.current_volume -= 1;
                }
            }
        }
    }
}

/// 頻率掃描配置
#[derive(Clone, Debug, Default)]
pub struct FrequencySweep {
    pub enabled: bool,
    pub period: u8,
    pub negate: bool,
    pub shift: u8,
    shadow_freq: u16,
    timer: u32,
}

impl FrequencySweep {
    pub fn new() -> Self {
        Self {
            enabled: false,
            period: 0,
            negate: false,
            shift: 0,
            shadow_freq: 0,
            timer: 0,
        }
    }

    pub fn get_register(&self) -> u8 {
        (self.period << 4) | ((self.negate as u8) << 3) | self.shift
    }

    pub fn set_register(&mut self, value: u8) {
        self.period = (value >> 4) & 0x07;
        self.negate = (value & 0x08) != 0;
        self.shift = value & 0x07;
    }

    pub fn trigger(&mut self, freq: u16) {
        self.shadow_freq = freq;
        self.enabled = self.period != 0 || self.shift != 0;
        self.timer = 0;
    }

    pub fn step(&mut self, freq: &mut u16) -> bool {
        if !self.enabled || self.period == 0 {
            return true;
        }

        self.timer += 1;
        if self.timer >= 128 {
            self.timer = 0;

            let offset = self.shadow_freq >> self.shift;
            let new_freq = if self.negate {
                self.shadow_freq.wrapping_sub(offset)
            } else {
                self.shadow_freq.wrapping_add(offset)
            };

            // 檢查頻率是否有效
            if new_freq > 2047 {
                return false;
            }

            self.shadow_freq = new_freq;
            *freq = new_freq;
        }

        true
    }
}
