use super::super::common::{AudioChannel, Envelope};

/// Noise 通道是 Game Boy 的第四個音效通道，用於產生白噪音
#[derive(Debug)]
pub struct Noise {
    enabled: bool,
    length_enabled: bool,
    length_counter: u8,
    envelope: Envelope,
    shift_clock_frequency: u8,
    counter_width: bool, // true = 7 位, false = 15 位
    divisor_code: u8,
    dac_enabled: bool,
    lfsr: u16, // 線性反饋移位暫存器
    timer: u32,
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
            lfsr: 0x7FFF, // 初始值
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
            _ => 8,
        }
    }

    fn get_period(&self) -> u32 {
        let divisor = self.get_divisor();
        divisor << self.shift_clock_frequency
    }

    fn update_lfsr(&mut self) {
        let bit0 = self.lfsr & 1;
        let bit1 = (self.lfsr >> 1) & 1;
        let new_bit = bit0 ^ bit1;

        self.lfsr >>= 1;
        self.lfsr |= new_bit << 14;

        if self.counter_width {
            // 7 位模式：將新位元也寫入位置 6
            if new_bit == 1 {
                self.lfsr |= 0x40;
            } else {
                self.lfsr &= !0x40;
            }
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

        // 更新包絡
        self.envelope.step(cycles);

        // 更新 LFSR
        self.timer = self.timer.saturating_sub(cycles);
        while self.timer == 0 {
            self.update_lfsr();
            self.timer = self.get_period();
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

        let raw_amp = if (self.lfsr & 1) == 0 {
            self.envelope.current_volume as f32 / 15.0
        } else {
            0.0
        };

        raw_amp * 2.0 - 1.0 // 將 0.0-1.0 映射到 -1.0-1.0
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
            0x20 => {
                // NR41: Length timer
                self.length_counter = 64 - (value & 0x3F);
            }
            0x21 => {
                // NR42: Volume & envelope
                let initial_volume = value >> 4;
                let direction = (value & 0x08) != 0;
                let period = value & 0x07;
                self.dac_enabled = (value & 0xF8) != 0;
                self.set_envelope(initial_volume, direction, period);
                if !self.dac_enabled {
                    self.enabled = false;
                }
            }
            0x22 => {
                // NR43: Frequency & randomness
                self.shift_clock_frequency = value >> 4;
                self.counter_width = (value & 0x08) != 0;
                self.divisor_code = value & 0x07;
                self.timer = self.get_period();
            }
            0x23 => {
                // NR44: Control
                self.length_enabled = (value & 0x40) != 0;

                if value & 0x80 != 0 {
                    // 觸發位
                    self.enabled = self.dac_enabled;
                    if self.length_counter == 0 {
                        self.length_counter = 64;
                    }
                    self.envelope.reset();
                    self.lfsr = 0x7FFF; // 重置 LFSR
                    self.timer = self.get_period();
                }
            }
            _ => {}
        }
    }

    fn read_register(&self, addr: u16) -> u8 {
        match addr & 0xFF {
            0x20 => {
                // NR41
                0xFF // 只能寫入
            }
            0x21 => {
                // NR42
                (self.envelope.initial_volume << 4)
                    | ((self.envelope.direction as u8) << 3)
                    | self.envelope.sweep_pace
            }
            0x22 => {
                // NR43
                (self.shift_clock_frequency << 4)
                    | ((self.counter_width as u8) << 3)
                    | self.divisor_code
            }
            0x23 => {
                // NR44
                ((self.length_enabled as u8) << 6)
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
