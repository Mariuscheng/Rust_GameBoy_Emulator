mod common;
mod noise;
mod registers;
mod square1;
mod square2;
mod wave;

use noise::NoiseChannel;
use registers::*;
use square1::SquareChannel1;
use square2::SquareChannel2;
use wave::WaveChannel;

/// Sound 控制寄存器
const NR10: u16 = 0xFF10;
const NR11: u16 = 0xFF11;
const NR12: u16 = 0xFF12;
const NR13: u16 = 0xFF13;
const NR14: u16 = 0xFF14;
const NR21: u16 = 0xFF16;
const NR22: u16 = 0xFF17;
const NR23: u16 = 0xFF18;
const NR24: u16 = 0xFF19;
const NR30: u16 = 0xFF1A;
const NR31: u16 = 0xFF1B;
const NR32: u16 = 0xFF1C;
const NR33: u16 = 0xFF1D;
const NR34: u16 = 0xFF1E;
const NR41: u16 = 0xFF20;
const NR42: u16 = 0xFF21;
const NR43: u16 = 0xFF22;
const NR44: u16 = 0xFF23;
const NR50: u16 = 0xFF24;
const NR51: u16 = 0xFF25;
const NR52: u16 = 0xFF26;

/// Wave RAM 地址範圍
const WAVE_RAM_START: u16 = 0xFF30;
const WAVE_RAM_END: u16 = 0xFF3F;

pub struct APU {
    enabled: bool,
    channel1: SquareChannel1,
    channel2: SquareChannel2,
    channel3: WaveChannel,
    channel4: NoiseChannel,
    /// 左右聲道音量 (0-7)
    left_volume: u8,
    right_volume: u8,
    /// 聲道路由
    channel_enables: u8, // NR51
    /// 採樣計數器
    sample_counter: u32,
    sample_rate: u32,
    /// 音頻時鐘
    cycles: u32,
    frame_cycles: u32,
    frame_sequencer: u8,
    /// 聲道輸出緩衝
    output_buffer: Vec<(i16, i16)>,
}

impl APU {
    pub fn new() -> Self {
        Self {
            enabled: false,
            channel1: SquareChannel1::new(),
            channel2: SquareChannel2::new(),
            channel3: WaveChannel::new(),
            channel4: NoiseChannel::new(),
            left_volume: 0,
            right_volume: 0,
            channel_enables: 0,
            sample_counter: 0,
            sample_rate: 44100, // 預設採樣率
            cycles: 0,
            frame_cycles: 0,
            frame_sequencer: 0,
            output_buffer: Vec::new(),
        }
    }

    pub fn step(&mut self, cycles: u32) {
        if !self.enabled {
            return;
        }

        // 更新音頻時鐘
        self.cycles += cycles;
        self.frame_cycles += cycles;

        // 幀序列器更新 (512Hz)
        if self.frame_cycles >= 8192 {
            self.frame_cycles -= 8192;
            self.frame_sequencer = (self.frame_sequencer + 1) & 7;

            // 根據幀序列器步驟更新不同的音頻參數
            match self.frame_sequencer {
                2 | 6 => {
                    // 掃頻 - 只有通道1有這個功能
                    if self.channel1.is_enabled() {
                        self.channel1.step();
                    }
                }
                0 | 4 => {
                    // 長度計數器
                    if self.channel1.is_enabled() {
                        self.channel1.step();
                    }
                    if self.channel2.is_enabled() {
                        self.channel2.step();
                    }
                    if self.channel3.is_enabled() {
                        self.channel3.step();
                    }
                    if self.channel4.is_enabled() {
                        self.channel4.step();
                    }
                }
                7 => {
                    // 音量包絡
                    if self.channel1.is_enabled() {
                        self.channel1.step();
                    }
                    if self.channel2.is_enabled() {
                        self.channel2.step();
                    }
                    if self.channel4.is_enabled() {
                        self.channel4.step();
                    }
                }
                _ => {}
            }
        }

        // 更新採樣和混音
        self.update_samples();
    }

    fn update_samples(&mut self) {
        while self.cycles >= 95 {
            // 約44100Hz的採樣率
            self.cycles -= 95;
            self.mix_samples();
        }
    }

    fn mix_samples(&mut self) {
        if !self.enabled {
            self.output_buffer.push((0, 0));
            return;
        }

        let mut left = 0i16;
        let mut right = 0i16;

        // Channel 1 (方波1)
        if self.channel1.is_enabled() {
            let sample = self.channel1.get_output() as i16;
            if (self.channel_enables & 0x10) != 0 {
                left += sample;
            }
            if (self.channel_enables & 0x01) != 0 {
                right += sample;
            }
        }

        // Channel 2 (方波2)
        if self.channel2.is_enabled() {
            let sample = self.channel2.get_output() as i16;
            if (self.channel_enables & 0x20) != 0 {
                left += sample;
            }
            if (self.channel_enables & 0x02) != 0 {
                right += sample;
            }
        }

        // Channel 3 (波形)
        if self.channel3.is_enabled() {
            let sample = self.channel3.get_output() as i16;
            if (self.channel_enables & 0x40) != 0 {
                left += sample;
            }
            if (self.channel_enables & 0x04) != 0 {
                right += sample;
            }
        }

        // Channel 4 (噪音)
        if self.channel4.is_enabled() {
            let sample = self.channel4.get_output() as i16;
            if (self.channel_enables & 0x80) != 0 {
                left += sample;
            }
            if (self.channel_enables & 0x08) != 0 {
                right += sample;
            }
        }

        // 應用主音量控制並歸一化
        left = ((left * (self.left_volume as i16)) / 28) * 128;
        right = ((right * (self.right_volume as i16)) / 28) * 128;

        // 限制在有效範圍內
        left = left.max(-32768).min(32767);
        right = right.max(-32768).min(32767);

        self.output_buffer.push((left, right));
    }

    pub fn get_audio_buffer(&mut self) -> Vec<(i16, i16)> {
        std::mem::take(&mut self.output_buffer)
    }

    // 寄存器讀取
    pub fn read_byte(&self, addr: u16) -> u8 {
        match addr {
            NR10 => self.channel1.read_sweep(),
            NR11 => self.channel1.read_length_duty(),
            NR12 => self.channel1.read_envelope(),
            NR13 => self.channel1.read_frequency_lo(),
            NR14 => self.channel1.read_frequency_hi(),
            NR21 => self.channel2.read_length_duty(),
            NR22 => self.channel2.read_envelope(),
            NR23 => self.channel2.read_frequency_lo(),
            NR24 => self.channel2.read_frequency_hi(),
            NR30 => self.channel3.read_enable(),
            NR31 => self.channel3.read_length(),
            NR32 => self.channel3.read_volume(),
            NR33 => self.channel3.read_frequency_lo(),
            NR34 => self.channel3.read_frequency_hi(),
            NR41 => self.channel4.read_length(),
            NR42 => self.channel4.read_envelope(),
            NR43 => self.channel4.read_polynomial(),
            NR44 => self.channel4.read_counter(),
            NR50 => (self.left_volume << 4) | self.right_volume,
            NR51 => self.channel_enables,
            NR52 => {
                let mut value = if self.enabled { 0x80 } else { 0x00 };
                value |= if self.channel4.is_enabled() {
                    0x08
                } else {
                    0x00
                };
                value |= if self.channel3.is_enabled() {
                    0x04
                } else {
                    0x00
                };
                value |= if self.channel2.is_enabled() {
                    0x02
                } else {
                    0x00
                };
                value |= if self.channel1.is_enabled() {
                    0x01
                } else {
                    0x00
                };
                value
            }
            WAVE_RAM_START..=WAVE_RAM_END => self.channel3.read_wave_ram(addr - WAVE_RAM_START),
            _ => 0xFF,
        }
    }

    // 寄存器寫入
    pub fn write_byte(&mut self, addr: u16, value: u8) {
        if !self.enabled && addr != NR52 && addr < WAVE_RAM_START {
            return;
        }

        match addr {
            NR10 => self.channel1.write_sweep(value),
            NR11 => self.channel1.write_length_duty(value),
            NR12 => self.channel1.write_envelope(value),
            NR13 => self.channel1.write_frequency_lo(value),
            NR14 => self.channel1.write_frequency_hi(value),
            NR21 => self.channel2.write_length_duty(value),
            NR22 => self.channel2.write_envelope(value),
            NR23 => self.channel2.write_frequency_lo(value),
            NR24 => self.channel2.write_frequency_hi(value),
            NR30 => self.channel3.write_enable(value),
            NR31 => self.channel3.write_length(value),
            NR32 => self.channel3.write_volume(value),
            NR33 => self.channel3.write_frequency_lo(value),
            NR34 => self.channel3.write_frequency_hi(value),
            NR41 => self.channel4.write_length(value),
            NR42 => self.channel4.write_envelope(value),
            NR43 => self.channel4.write_polynomial(value),
            NR44 => self.channel4.write_counter(value),
            NR50 => {
                self.left_volume = (value >> 4) & 0x7;
                self.right_volume = value & 0x7;
            }
            NR51 => self.channel_enables = value,
            NR52 => {
                let was_enabled = self.enabled;
                self.enabled = (value & 0x80) != 0;
                if !was_enabled && self.enabled {
                    self.power_on();
                } else if was_enabled && !self.enabled {
                    self.power_off();
                }
            }
            WAVE_RAM_START..=WAVE_RAM_END => {
                self.channel3.write_wave_ram(addr - WAVE_RAM_START, value);
            }
            _ => {}
        }
    }

    // 開機初始化
    fn power_on(&mut self) {
        self.frame_sequencer = 0;
        self.frame_cycles = 0;
        self.channel1.power_on();
        self.channel2.power_on();
        self.channel3.power_on();
        self.channel4.power_on();
    }

    // 關機處理
    fn power_off(&mut self) {
        self.channel1.power_off();
        self.channel2.power_off();
        self.channel3.power_off();
        self.channel4.power_off();
        self.left_volume = 0;
        self.right_volume = 0;
        self.channel_enables = 0;
    }

    // 重置APU
    pub fn reset(&mut self) {
        self.enabled = false;
        self.channel1.reset();
        self.channel2.reset();
        self.channel3.reset();
        self.channel4.reset();
        self.left_volume = 0;
        self.right_volume = 0;
        self.channel_enables = 0;
        self.sample_counter = 0;
        self.cycles = 0;
        self.frame_cycles = 0;
        self.frame_sequencer = 0;
        self.output_buffer.clear();
    }
}
