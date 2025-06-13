use super::registers::*;

/// 方波聲道的佔空比配置
#[derive(Clone, Copy, Debug, Default)]
pub enum DutyCycle {
    #[default]
    Duty125 = 0, // 12.5%
    Duty25 = 1,  // 25%
    Duty50 = 2,  // 50%
    Duty75 = 3,  // 75%
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
            DutyCycle::Duty25  => 0b00000011,
            DutyCycle::Duty50  => 0b00001111,
            DutyCycle::Duty75  => 0b11111100,
        }
    }
}

/// 音量包絡配置
#[derive(Clone, Debug, Default)]
pub struct Envelope {
    pub initial_volume: u8,
    pub direction: bool, // true = 增加，false = 減少
    pub sweep_pace: u8,
    pub current_volume: u8,
    period: u8,
    timer: u32,
}

impl Envelope {
    pub fn new() -> Self {
        Self {
            initial_volume: 0,
            direction: false,
            sweep_pace: 0,
            current_volume: 0,
            period: 0,
            timer: 0,
        }
    }

    pub fn reset(&mut self) {
        self.current_volume = self.initial_volume;
        self.period = self.sweep_pace;
        self.timer = 0;
    }

    pub fn step(&mut self, cycles: u32) {
        if self.sweep_pace == 0 {
            return;
        }

        self.timer += cycles;
        let envelope_period = self.sweep_pace as u32 * 64; // 每個封套步驟是64個時鐘週期
        
        while self.timer >= envelope_period {
            self.timer -= envelope_period;
            
            if self.period > 0 {
                self.period -= 1;
                if self.period == 0 {
                    if self.direction && self.current_volume < 15 {
                        self.current_volume += 1;
                    } else if !self.direction && self.current_volume > 0 {
                        self.current_volume -= 1;
                    }
                    self.period = self.sweep_pace;
                }
            }
        }
    }
}

/// 頻率掃描配置
#[derive(Clone, Debug, Default)]
pub struct FrequencySweep {
    pub period: u8,
    pub negate: bool,
    pub shift: u8,
    enabled: bool,
    timer: u8,
    shadow_freq: u16,
}

impl FrequencySweep {
    pub fn new() -> Self {
        Self {
            period: 0,
            negate: false,
            shift: 0,
            enabled: false,
            timer: 0,
            shadow_freq: 0,
        }
    }

    pub fn reset(&mut self) {
        self.timer = self.period;
        self.enabled = self.period > 0 || self.shift > 0;
    }

    pub fn step(&mut self, frequency: &mut u16) -> bool {
        if !self.enabled || self.period == 0 {
            return true;
        }

        if self.timer > 0 {
            self.timer -= 1;
            return true;
        }

        self.timer = self.period;

        let delta = self.shadow_freq >> self.shift;
        let new_freq = if self.negate {
            self.shadow_freq - delta
        } else {
            self.shadow_freq + delta
        };

        if new_freq > 2047 {
            return false;
        }

        if self.shift > 0 {
            self.shadow_freq = new_freq;
            *frequency = new_freq;
        }

        true
    }
}
