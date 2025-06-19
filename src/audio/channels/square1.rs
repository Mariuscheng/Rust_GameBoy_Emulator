use super::super::common::{AudioChannel, DutyCycle, Envelope, FrequencySweep};

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

impl Square1 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            length_enabled: false,
            length_counter: 0,
            duty: DutyCycle::default(),
            envelope: Envelope::new(),
            frequency_sweep: FrequencySweep::default(),
            frequency: 0,
            phase: 0,
            dac_enabled: false,
        }
    }
}

impl AudioChannel for Square1 {
    fn init(&mut self) {
        self.reset();
    }

    fn reset(&mut self) {
        self.enabled = false;
        self.length_counter = 0;
        self.duty = DutyCycle::default();
        self.envelope = Envelope::new();
        self.frequency_sweep = FrequencySweep::default();
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

        // 更新波相位
        let period = (2048 - self.frequency) as u32 * 4; // 以 CPU 時脈計算週期
        if period > 0 {
            self.phase = ((self.phase as u32 + cycles / period) % 8) as u8;
        }

        // 更新長度計數器
        if self.length_enabled && self.length_counter > 0 {
            if cycles >= 64 {
                // 64 CPU 週期為一個幀
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

        amplitude * 2.0 - 1.0 // 將 0.0-1.0 映射到 -1.0-1.0
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.length_counter = 0;
        }
    }

    fn write_register(&mut self, addr: u16, value: u8) {
        match addr & 0xFF {
            0x10 => {
                // NR10: Sweep
                self.frequency_sweep.period = (value >> 4) & 0x07;
                self.frequency_sweep.negate = (value & 0x08) != 0;
                self.frequency_sweep.shift = value & 0x07;
            }
            0x11 => {
                // NR11: Length timer & duty cycle
                self.duty = DutyCycle::from_bits((value >> 6) & 0x03);
                self.length_counter = 64 - (value & 0x3F);
            }
            0x12 => {
                // NR12: Volume & envelope
                let initial_volume = value >> 4;
                let direction = (value & 0x08) != 0;
                let period = value & 0x07;
                self.dac_enabled = (value & 0xF8) != 0;
                self.set_envelope(initial_volume, direction, period);
                if !self.dac_enabled {
                    self.enabled = false;
                }
            }
            0x13 => {
                // NR13: Frequency LSB
                self.frequency = (self.frequency & 0x700) | value as u16;
            }
            0x14 => {
                // NR14: Frequency MSB & control
                self.frequency = (self.frequency & 0xFF) | ((value as u16 & 0x07) << 8);
                self.length_enabled = (value & 0x40) != 0;

                if value & 0x80 != 0 {
                    // 觸發位
                    self.enabled = self.dac_enabled;
                    if self.length_counter == 0 {
                        self.length_counter = 64;
                    }
                    self.envelope.reset();
                    self.frequency_sweep.reset();
                }
            }
            _ => {}
        }
    }

    fn read_register(&self, addr: u16) -> u8 {
        match addr & 0xFF {
            0x10 => {
                // NR10
                (self.frequency_sweep.period << 4)
                    | ((self.frequency_sweep.negate as u8) << 3)
                    | self.frequency_sweep.shift
            }
            0x11 => {
                // NR11
                ((self.duty as u8) << 6) | (64 - self.length_counter)
            }
            0x12 => {
                // NR12
                (self.envelope.initial_volume << 4)
                    | ((self.envelope.direction as u8) << 3)
                    | self.envelope.sweep_pace
            }
            0x13 => {
                // NR13
                self.frequency as u8
            }
            0x14 => {
                // NR14
                ((self.length_enabled as u8) << 6) | ((self.frequency >> 8) as u8 & 0x07)
            }
            _ => 0xFF,
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
        self.envelope.sweep_pace = period;
        self.envelope.reset();
    }

    fn get_volume(&self) -> u8 {
        self.envelope.current_volume
    }
}
