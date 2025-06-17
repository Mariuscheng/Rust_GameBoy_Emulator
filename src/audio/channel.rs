//! Game Boy 音效通道實現
//! 包含四種基本通道：Square1、Square2、Wave 和 Noise

use super::common::{AudioChannel, DutyCycle, Envelope, FrequencySweep};

/// Square1 通道是 Game Boy 的第一個方波通道，具有頻率掃描功能
#[derive(Debug)]
pub struct Square1 {
    enabled: bool,
    length_enabled: bool,
    length_counter: u8,
    duty: DutyCycle,
    envelope: Envelope,
    frequency_sweep: FrequencySweep,
    frequency: u16,
    phase: u8,
    dac_enabled: bool,
}

/// Square2 通道是 Game Boy 的第二個方波通道，與 Square1 類似但沒有頻率掃描功能
#[derive(Debug)]
pub struct Square2 {
    enabled: bool,
    length_enabled: bool,
    length_counter: u8,
    duty: DutyCycle,
    envelope: Envelope,
    frequency: u16,
    phase: u8,
    dac_enabled: bool,
}

/// Wave 通道是 Game Boy 的第三個音效通道，可以播放 32 字節的自定義波形
#[derive(Debug)]
pub struct Wave {
    enabled: bool,
    length_enabled: bool,
    length_counter: u8,
    output_level: u8,
    frequency: u16,
    position: u8,
    wave_ram: [u8; 32],
    dac_enabled: bool,
    frame_cycles: u32,
}

/// Noise 通道是 Game Boy 的第四個音效通道，用於產生白噪音
#[derive(Debug)]
pub struct Noise {
    enabled: bool,
    length_enabled: bool,
    length_counter: u8,
    envelope: Envelope,
    shift_clock_frequency: u8,
    counter_width: bool,
    divisor_code: u8,
    dac_enabled: bool,
    lfsr: u16,
    timer: u32,
}

// 實現初始化方法
impl Square1 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            length_enabled: false,
            length_counter: 0,
            duty: DutyCycle::default(),
            envelope: Envelope::new(),
            frequency_sweep: FrequencySweep::new(),
            frequency: 0,
            phase: 0,
            dac_enabled: false,
        }
    }
}

impl Square2 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            length_enabled: false,
            length_counter: 0,
            duty: DutyCycle::default(),
            envelope: Envelope::new(),
            frequency: 0,
            phase: 0,
            dac_enabled: false,
        }
    }
}

impl Wave {
    pub fn new() -> Self {
        Self {
            enabled: false,
            length_enabled: false,
            length_counter: 0,
            output_level: 0,
            frequency: 0,
            position: 0,
            wave_ram: [0; 32],
            dac_enabled: false,
            frame_cycles: 0,
        }
    }
}

impl Noise {
    pub fn new() -> Self {
        Self {
            enabled: false,
            length_enabled: false,
            length_counter: 0,
            envelope: Envelope::new(),
            shift_clock_frequency: 0,
            counter_width: false,
            divisor_code: 0,
            dac_enabled: false,
            lfsr: 0x7FFF,
            timer: 0,
        }
    }

    fn get_divisor(&self) -> u32 {
        match self.divisor_code {
            0 => 8,
            1 => 16,
            2 => 32,
            3 => 48,
            4 => 64,
            5 => 80,
            6 => 96,
            7 => 112,
            _ => unreachable!(),
        }
    }
}

// AudioChannel trait 實作
impl AudioChannel for Square1 {
    fn init(&mut self) {
        self.reset();
    }

    fn reset(&mut self) {
        self.enabled = false;
        self.length_counter = 0;
        self.duty = DutyCycle::default();
        self.envelope = Envelope::new();
        self.frequency_sweep = FrequencySweep::new();
        self.frequency = 0;
        self.phase = 0;
        self.dac_enabled = false;
    }

    fn step(&mut self, cycles: u32) {
        if !self.enabled {
            return;
        }

        // 計算頻率掃描
        if !self.frequency_sweep.step(&mut self.frequency) {
            self.enabled = false;
            return;
        }

        // 更新音量包絡
        self.envelope.step(cycles);

        // 更新相位
        let period = (2048 - self.frequency) as u32 * 4;
        if period > 0 {
            self.phase = ((self.phase as u32 + cycles / period) % 8) as u8;
        }

        // 更新長度計數器
        if self.length_enabled && self.length_counter > 0 {
            if cycles >= 64 {
                self.length_counter = self.length_counter.saturating_sub(1);
                if self.length_counter == 0 {
                    self.enabled = false;
                }
            }
        }
    }

    fn get_sample(&self) -> f32 {
        if !self.enabled || !self.dac_enabled {
            return 0.0;
        }

        let amplitude = if self.duty.get_sample(self.phase) {
            self.envelope.current_volume as f32 / 15.0
        } else {
            0.0
        };

        amplitude * 2.0 - 1.0
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn read_register(&self, addr: u16) -> u8 {
        match addr {
            0xFF10 => self.frequency_sweep.get_register(),
            0xFF11 => (self.duty.get_pattern() << 6) | (64 - self.length_counter),
            0xFF12 => self.envelope.get_register(),
            0xFF13 => self.frequency as u8,
            0xFF14 => ((self.length_enabled as u8) << 6) | ((self.frequency >> 8) as u8 & 0x07),
            _ => 0xFF,
        }
    }

    fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF10 => self.frequency_sweep.set_register(value),
            0xFF11 => {
                self.duty.set_pattern(value >> 6);
                self.length_counter = 64 - (value & 0x3F);
            }
            0xFF12 => {
                self.envelope.set_register(value);
                self.dac_enabled = value & 0xF8 != 0;
                if !self.dac_enabled {
                    self.enabled = false;
                }
            }
            0xFF13 => self.frequency = (self.frequency & 0x0700) | value as u16,
            0xFF14 => {
                self.frequency = (self.frequency & 0x00FF) | ((value as u16 & 0x07) << 8);
                self.length_enabled = value & 0x40 != 0;
                if value & 0x80 != 0 {
                    self.enabled = self.dac_enabled;
                    if self.length_counter == 0 {
                        self.length_counter = 64;
                    }
                    self.envelope.trigger();
                    self.frequency_sweep.trigger(self.frequency);
                }
            }
            _ => {}
        }
    }

    fn get_length(&self) -> u8 {
        self.length_counter
    }

    fn set_length(&mut self, value: u8) {
        self.length_counter = value;
    }

    fn set_envelope(&mut self, initial_volume: u8, direction: bool, period: u8) {
        self.envelope.initial_volume = initial_volume;
        self.envelope.direction = direction;
        self.envelope.period = period;
        self.envelope.current_volume = initial_volume;
    }

    fn get_volume(&self) -> u8 {
        self.envelope.current_volume
    }
}

impl AudioChannel for Square2 {
    fn init(&mut self) {
        self.reset();
    }

    fn reset(&mut self) {
        self.enabled = false;
        self.length_counter = 0;
        self.duty = DutyCycle::default();
        self.envelope = Envelope::new();
        self.frequency = 0;
        self.phase = 0;
        self.dac_enabled = false;
    }

    fn step(&mut self, cycles: u32) {
        if !self.enabled {
            return;
        }

        // 更新音量包絡
        self.envelope.step(cycles);

        // 更新相位
        let period = (2048 - self.frequency) as u32 * 4;
        if period > 0 {
            self.phase = ((self.phase as u32 + cycles / period) % 8) as u8;
        }

        // 更新長度計數器
        if self.length_enabled && self.length_counter > 0 {
            if cycles >= 64 {
                self.length_counter = self.length_counter.saturating_sub(1);
                if self.length_counter == 0 {
                    self.enabled = false;
                }
            }
        }
    }

    fn get_sample(&self) -> f32 {
        if !self.enabled || !self.dac_enabled {
            return 0.0;
        }

        let amplitude = if self.duty.get_sample(self.phase) {
            self.envelope.current_volume as f32 / 15.0
        } else {
            0.0
        };

        amplitude * 2.0 - 1.0
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn read_register(&self, addr: u16) -> u8 {
        match addr {
            0xFF16 => (self.duty.get_pattern() << 6) | (64 - self.length_counter),
            0xFF17 => self.envelope.get_register(),
            0xFF18 => self.frequency as u8,
            0xFF19 => ((self.length_enabled as u8) << 6) | ((self.frequency >> 8) as u8 & 0x07),
            _ => 0xFF,
        }
    }

    fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF16 => {
                self.duty.set_pattern(value >> 6);
                self.length_counter = 64 - (value & 0x3F);
            }
            0xFF17 => {
                self.envelope.set_register(value);
                self.dac_enabled = value & 0xF8 != 0;
                if !self.dac_enabled {
                    self.enabled = false;
                }
            }
            0xFF18 => self.frequency = (self.frequency & 0x0700) | value as u16,
            0xFF19 => {
                self.frequency = (self.frequency & 0x00FF) | ((value as u16 & 0x07) << 8);
                self.length_enabled = value & 0x40 != 0;
                if value & 0x80 != 0 {
                    self.enabled = self.dac_enabled;
                    if self.length_counter == 0 {
                        self.length_counter = 64;
                    }
                    self.envelope.trigger();
                }
            }
            _ => {}
        }
    }

    fn get_length(&self) -> u8 {
        self.length_counter
    }

    fn set_length(&mut self, value: u8) {
        self.length_counter = value;
    }

    fn set_envelope(&mut self, initial_volume: u8, direction: bool, period: u8) {
        self.envelope.initial_volume = initial_volume;
        self.envelope.direction = direction;
        self.envelope.period = period;
        self.envelope.current_volume = initial_volume;
    }

    fn get_volume(&self) -> u8 {
        self.envelope.current_volume
    }
}

#[allow(unused_variables)]
impl AudioChannel for Wave {
    fn init(&mut self) {
        self.reset();
    }

    fn reset(&mut self) {
        self.enabled = false;
        self.length_counter = 0;
        self.output_level = 0;
        self.frequency = 0;
        self.position = 0;
        self.wave_ram = [0; 32];
        self.dac_enabled = false;
        self.frame_cycles = 0;
    }

    fn step(&mut self, cycles: u32) {
        if !self.enabled {
            return;
        }

        // 更新相位
        let period = (2048 - self.frequency as u32) * 2;
        if period > 0 {
            self.frame_cycles += cycles;
            while self.frame_cycles >= period {
                self.frame_cycles -= period;
                self.position = (self.position + 1) % 32;
            }
        }

        // 更新長度計數器
        if self.length_enabled && self.length_counter > 0 {
            if cycles >= 64 {
                self.length_counter = self.length_counter.saturating_sub(1);
                if self.length_counter == 0 {
                    self.enabled = false;
                }
            }
        }
    }

    fn get_sample(&self) -> f32 {
        if !self.enabled || !self.dac_enabled {
            return 0.0;
        }

        let sample = self.wave_ram[self.position as usize];
        let volume = match self.output_level {
            0 => sample >> 4,
            1 => sample,
            2 => sample >> 1,
            3 => sample >> 2,
            _ => unreachable!(),
        };

        (volume as f32 / 7.5) - 1.0
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn read_register(&self, addr: u16) -> u8 {
        match addr {
            0xFF1A => (self.dac_enabled as u8) << 7,
            0xFF1B => 255 - self.length_counter as u8,
            0xFF1C => self.output_level << 5,
            0xFF1D => self.frequency as u8,
            0xFF1E => ((self.length_enabled as u8) << 6) | ((self.frequency >> 8) as u8 & 0x07),
            0xFF30..=0xFF3F => self.wave_ram[(addr - 0xFF30) as usize],
            _ => 0xFF,
        }
    }

    fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF1A => {
                self.dac_enabled = value & 0x80 != 0;
                if !self.dac_enabled {
                    self.enabled = false;
                }
            }
            0xFF1B => self.length_counter = 255 - value,
            0xFF1C => self.output_level = (value >> 5) & 0x03,
            0xFF1D => self.frequency = (self.frequency & 0x0700) | value as u16,
            0xFF1E => {
                self.frequency = (self.frequency & 0x00FF) | ((value as u16 & 0x07) << 8);
                self.length_enabled = value & 0x40 != 0;
                if value & 0x80 != 0 {
                    self.enabled = self.dac_enabled;
                    if self.length_counter == 0 {
                        self.length_counter = 255;
                    }
                }
            }
            0xFF30..=0xFF3F => self.wave_ram[(addr - 0xFF30) as usize] = value,
            _ => {}
        }
    }

    fn get_length(&self) -> u8 {
        self.length_counter
    }

    fn set_length(&mut self, value: u8) {
        self.length_counter = value;
    }

    fn set_envelope(&mut self, initial_volume: u8, direction: bool, period: u8) {
        // Wave 通道不使用包絡，但保留參數以符合 trait 要求
    }

    fn get_volume(&self) -> u8 {
        match self.output_level {
            0 => 0,
            1 => 15,
            2 => 7,
            3 => 3,
            _ => unreachable!(),
        }
    }
}

impl AudioChannel for Noise {
    fn init(&mut self) {
        self.reset();
    }

    fn reset(&mut self) {
        self.enabled = false;
        self.length_counter = 0;
        self.envelope = Envelope::new();
        self.shift_clock_frequency = 0;
        self.counter_width = false;
        self.divisor_code = 0;
        self.dac_enabled = false;
        self.lfsr = 0x7FFF;
        self.timer = 0;
    }

    fn step(&mut self, cycles: u32) {
        if !self.enabled {
            return;
        }

        // 更新音量包絡
        self.envelope.step(cycles);

        // 更新 LFSR
        let divisor = self.get_divisor();
        let period = divisor << self.shift_clock_frequency;
        if period > 0 {
            self.timer += cycles;
            while self.timer >= period {
                self.timer -= period;
                let xor_result = (self.lfsr & 0x1) ^ ((self.lfsr >> 1) & 0x1);
                self.lfsr >>= 1;
                self.lfsr |= xor_result << 14;
                if self.counter_width {
                    self.lfsr &= !0x40;
                    self.lfsr |= xor_result << 6;
                }
            }
        }

        // 更新長度計數器
        if self.length_enabled && self.length_counter > 0 {
            if cycles >= 64 {
                self.length_counter = self.length_counter.saturating_sub(1);
                if self.length_counter == 0 {
                    self.enabled = false;
                }
            }
        }
    }

    fn get_sample(&self) -> f32 {
        if !self.enabled || !self.dac_enabled {
            return 0.0;
        }

        let amplitude = if self.lfsr & 0x1 == 0 {
            self.envelope.current_volume as f32 / 15.0
        } else {
            0.0
        };

        amplitude * 2.0 - 1.0
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }

    fn read_register(&self, addr: u16) -> u8 {
        match addr {
            0xFF20 => 0xFF, // 無效寄存器
            0xFF21 => self.envelope.get_register(),
            0xFF22 => {
                (self.shift_clock_frequency << 4)
                    | ((self.counter_width as u8) << 3)
                    | self.divisor_code
            }
            0xFF23 => (self.length_enabled as u8) << 6,
            _ => 0xFF,
        }
    }

    fn write_register(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF20 => {
                self.length_counter = 64 - (value & 0x3F);
            }
            0xFF21 => {
                self.envelope.set_register(value);
                self.dac_enabled = value & 0xF8 != 0;
                if !self.dac_enabled {
                    self.enabled = false;
                }
            }
            0xFF22 => {
                self.shift_clock_frequency = (value >> 4) & 0x0F;
                self.counter_width = value & 0x08 != 0;
                self.divisor_code = value & 0x07;
            }
            0xFF23 => {
                self.length_enabled = value & 0x40 != 0;
                if value & 0x80 != 0 {
                    self.enabled = self.dac_enabled;
                    if self.length_counter == 0 {
                        self.length_counter = 64;
                    }
                    self.envelope.trigger();
                    self.lfsr = 0x7FFF;
                }
            }
            _ => {}
        }
    }

    fn get_length(&self) -> u8 {
        self.length_counter
    }

    fn set_length(&mut self, value: u8) {
        self.length_counter = value;
    }

    fn set_envelope(&mut self, initial_volume: u8, direction: bool, period: u8) {
        self.envelope.initial_volume = initial_volume;
        self.envelope.direction = direction;
        self.envelope.period = period;
        self.envelope.current_volume = initial_volume;
    }

    fn get_volume(&self) -> u8 {
        self.envelope.current_volume
    }
}
