use crate::audio::common::{AudioChannel, DutyCycle, Envelope, FrequencySweep};

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
    counter_width: bool, // true = 7 位, false = 15 位
    divisor_code: u8,
    dac_enabled: bool,
    lfsr: u16, // 線性反饋移位暫存器
    timer: u32,
}

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
            lfsr: 0xFFFF,
            timer: 0,
        }
    }
}

impl AudioChannel for Square1 {
    fn init(&mut self) {
        self.reset();
    }
    fn reset(&mut self) {
        *self = Self::new();
    }
    fn step(&mut self, cycles: u32) {}
    fn get_sample(&self) -> f32 {
        0.0
    }
    fn is_enabled(&self) -> bool {
        self.enabled
    }
    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    fn write_register(&mut self, addr: u16, value: u8) {}
    fn read_register(&self, addr: u16) -> u8 {
        0
    }
    fn get_length(&self) -> u8 {
        self.length_counter
    }
    fn set_length(&mut self, value: u8) {
        self.length_counter = value;
    }
    fn set_envelope(&mut self, initial_volume: u8, direction: bool, period: u8) {
        self.envelope = Envelope::new();
    }
    fn get_volume(&self) -> u8 {
        0
    }
}

impl AudioChannel for Square2 {
    fn init(&mut self) {
        self.reset();
    }
    fn reset(&mut self) {
        *self = Self::new();
    }
    fn step(&mut self, cycles: u32) {}
    fn get_sample(&self) -> f32 {
        0.0
    }
    fn is_enabled(&self) -> bool {
        self.enabled
    }
    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    fn write_register(&mut self, addr: u16, value: u8) {}
    fn read_register(&self, addr: u16) -> u8 {
        0
    }
    fn get_length(&self) -> u8 {
        self.length_counter
    }
    fn set_length(&mut self, value: u8) {
        self.length_counter = value;
    }
    fn set_envelope(&mut self, initial_volume: u8, direction: bool, period: u8) {
        self.envelope = Envelope::new();
    }
    fn get_volume(&self) -> u8 {
        0
    }
}

impl AudioChannel for Wave {
    fn init(&mut self) {
        self.reset();
    }
    fn reset(&mut self) {
        *self = Self::new();
    }
    fn step(&mut self, cycles: u32) {
        if !self.enabled {
            return;
        }
    }
    fn get_sample(&self) -> f32 {
        0.0
    }
    fn is_enabled(&self) -> bool {
        self.enabled && self.dac_enabled
    }
    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    fn write_register(&mut self, addr: u16, value: u8) {}
    fn read_register(&self, addr: u16) -> u8 {
        0
    }
    fn get_length(&self) -> u8 {
        self.length_counter
    }
    fn set_length(&mut self, value: u8) {
        self.length_counter = value;
    }
    fn set_envelope(&mut self, _: u8, _: bool, _: u8) { /* Wave channel has no envelope */
    }
    fn get_volume(&self) -> u8 {
        0
    }
}

impl AudioChannel for Noise {
    fn init(&mut self) {
        self.reset();
    }
    fn reset(&mut self) {
        *self = Self::new();
    }
    fn step(&mut self, cycles: u32) {}
    fn get_sample(&self) -> f32 {
        0.0
    }
    fn is_enabled(&self) -> bool {
        self.enabled
    }
    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
    fn write_register(&mut self, addr: u16, value: u8) {}
    fn read_register(&self, addr: u16) -> u8 {
        0
    }
    fn get_length(&self) -> u8 {
        self.length_counter
    }
    fn set_length(&mut self, value: u8) {
        self.length_counter = value;
    }
    fn set_envelope(&mut self, initial_volume: u8, direction: bool, period: u8) {
        self.envelope = Envelope::new();
    }
    fn get_volume(&self) -> u8 {
        0
    }
}
