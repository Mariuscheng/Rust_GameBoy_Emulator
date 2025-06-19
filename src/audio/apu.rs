/*
================================================================================
Game Boy 模擬器 - 音頻處理單元 (APU)
================================================================================
實現Game Boy的4個音頻通道和音頻寄存器管理

功能：
- Channel 1: 方波生成器 (帶掃頻功能)
- Channel 2: 方波生成器
- Channel 3: 可編程波形表
- Channel 4: 噪音生成器
- 調試報告和音頻狀態監控

日期: 2025年6月9日
================================================================================
*/

use crate::mmu::MMU;
use crate::utils::Logger;
use chrono::Local;
use std::cell::RefCell;
use std::fs::File;
use std::io::Write;
use std::rc::Rc;

// ============================================================================
// 音頻通道結構體
// ============================================================================

pub struct Channel1 {
    // 方波生成器 (帶掃頻)
    pub enabled: bool,
    pub length_counter: u8,
    pub volume: u8,
    pub frequency: u16,
    pub duty_cycle: u8,
    pub sample: f32,
    pub phase: f32,
    pub sweep_enabled: bool,
    pub sweep_period: u8,
    pub sweep_shift: u8,
    pub sweep_direction: bool, // false = 增加, true = 減少
}

impl Channel1 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            length_counter: 0,
            volume: 0,
            frequency: 0,
            duty_cycle: 0,
            sample: 0.0,
            phase: 0.0,
            sweep_enabled: false,
            sweep_period: 0,
            sweep_shift: 0,
            sweep_direction: false,
        }
    }

    pub fn get_sample(&self) -> f32 {
        if !self.enabled || self.volume == 0 {
            return 0.0;
        }
        self.sample
    }

    pub fn step(&mut self) {
        if !self.enabled {
            return;
        }

        // 更新相位
        let freq_hz = 131072.0 / (2048.0 - self.frequency as f32);
        self.phase += freq_hz / 4194304.0; // Game Boy CPU頻率

        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        // 生成方波樣本
        let duty_threshold = match self.duty_cycle {
            0 => 0.125, // 12.5%
            1 => 0.25,  // 25%
            2 => 0.5,   // 50%
            3 => 0.75,  // 75%
            _ => 0.5,
        };

        self.sample = if self.phase < duty_threshold {
            (self.volume as f32 / 15.0) * 0.5
        } else {
            -(self.volume as f32 / 15.0) * 0.5
        };
    }

    pub fn trigger(&mut self) {
        self.enabled = true;
        self.phase = 0.0;
    }
}

pub struct Channel2 {
    // 方波生成器 (無掃頻)
    pub enabled: bool,
    pub length_counter: u8,
    pub volume: u8,
    pub frequency: u16,
    pub duty_cycle: u8,
    pub sample: f32,
    pub phase: f32,
}

impl Channel2 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            length_counter: 0,
            volume: 0,
            frequency: 0,
            duty_cycle: 0,
            sample: 0.0,
            phase: 0.0,
        }
    }

    pub fn get_sample(&self) -> f32 {
        if !self.enabled || self.volume == 0 {
            return 0.0;
        }
        self.sample
    }

    pub fn step(&mut self) {
        if !self.enabled {
            return;
        }

        let freq_hz = 131072.0 / (2048.0 - self.frequency as f32);
        self.phase += freq_hz / 4194304.0;

        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        let duty_threshold = match self.duty_cycle {
            0 => 0.125,
            1 => 0.25,
            2 => 0.5,
            3 => 0.75,
            _ => 0.5,
        };

        self.sample = if self.phase < duty_threshold {
            (self.volume as f32 / 15.0) * 0.5
        } else {
            -(self.volume as f32 / 15.0) * 0.5
        };
    }

    pub fn trigger(&mut self) {
        self.enabled = true;
        self.phase = 0.0;
    }
}

pub struct Channel3 {
    // 波形表通道
    pub enabled: bool,
    pub length_counter: u16,
    pub volume: u8,
    pub frequency: u16,
    pub sample: f32,
    pub phase: f32,
    pub wave_ram: [u8; 32], // 16字節的波形表，每字節包含2個4位樣本
}

impl Channel3 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            length_counter: 0,
            volume: 0,
            frequency: 0,
            sample: 0.0,
            phase: 0.0,
            wave_ram: [0; 32],
        }
    }

    pub fn get_sample(&self) -> f32 {
        if !self.enabled || self.volume == 0 {
            return 0.0;
        }
        self.sample
    }

    pub fn step(&mut self) {
        if !self.enabled {
            return;
        }

        // 正確的頻率計算
        let freq_hz = 65536.0 / (2048.0 - self.frequency as f32);
        self.phase += freq_hz / 4194304.0;

        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        // 獲取波形表中的樣本
        let pos = (self.phase * 32.0) as usize;
        let sample = if pos % 2 == 0 {
            self.wave_ram[pos / 2] >> 4
        } else {
            self.wave_ram[pos / 2] & 0x0F
        };

        // 根據音量設置進行縮放
        self.sample = match self.volume {
            0 => 0.0,
            1 => (sample as f32 / 15.0) * 1.0,  // 100%
            2 => (sample as f32 / 15.0) * 0.5,  // 50%
            3 => (sample as f32 / 15.0) * 0.25, // 25%
            _ => 0.0,
        };
    }

    pub fn write_wave_ram(&mut self, addr: usize, value: u8) {
        if addr < self.wave_ram.len() {
            self.wave_ram[addr] = value;
        }
    }

    pub fn read_wave_ram(&self, addr: usize) -> u8 {
        if addr < self.wave_ram.len() {
            self.wave_ram[addr]
        } else {
            0
        }
    }

    pub fn trigger(&mut self) {
        self.enabled = true;
        self.phase = 0.0;
    }
}

pub struct Channel4 {
    // 噪音生成器
    pub enabled: bool,
    pub length_counter: u8,
    pub volume: u8,
    pub envelope_period: u8,
    pub envelope_direction: bool,
    pub divisor_code: u8,
    pub width_mode: bool,
    pub clock_shift: u8,
    pub sample: f32,
    lfsr: u16, // 線性反饋移位寄存器
    timer: u32,
}

impl Channel4 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            length_counter: 0,
            volume: 0,
            envelope_period: 0,
            envelope_direction: false,
            divisor_code: 0,
            width_mode: false,
            clock_shift: 0,
            sample: 0.0,
            lfsr: 0x7FFF,
            timer: 0,
        }
    }

    pub fn get_sample(&self) -> f32 {
        if !self.enabled || self.volume == 0 {
            return 0.0;
        }
        self.sample
    }

    pub fn step(&mut self) {
        if !self.enabled {
            return;
        }

        // 計算分頻比
        let divisor = match self.divisor_code {
            0 => 8,
            1 => 16,
            2 => 32,
            3 => 48,
            4 => 64,
            5 => 80,
            6 => 96,
            7 => 112,
            _ => 8,
        };

        // 更新定時器
        self.timer += 1;
        let period = divisor << self.clock_shift;

        if self.timer >= period {
            self.timer = 0;

            // 更新 LFSR
            let bit = (self.lfsr & 1) ^ ((self.lfsr >> 1) & 1);
            self.lfsr >>= 1;
            self.lfsr |= bit << 14;

            if self.width_mode {
                self.lfsr &= !(1 << 6);
                self.lfsr |= bit << 6;
            }

            // 生成輸出
            self.sample = if self.lfsr & 1 == 0 {
                (self.volume as f32 / 15.0) * 0.5
            } else {
                -(self.volume as f32 / 15.0) * 0.5
            };
        }
    }

    pub fn trigger(&mut self) {
        self.enabled = true;
        self.lfsr = 0x7FFF;
        self.timer = 0;
    }

    pub fn write_nr41(&mut self, value: u8) {
        self.length_counter = value & 0x3F;
    }

    pub fn write_nr42(&mut self, value: u8) {
        self.volume = value >> 4;
        self.envelope_direction = (value & 0x08) != 0;
        self.envelope_period = value & 0x07;
    }

    pub fn write_nr43(&mut self, value: u8) {
        self.clock_shift = (value >> 4) & 0x0F;
        self.width_mode = (value & 0x08) != 0;
        self.divisor_code = value & 0x07;
    }

    pub fn write_nr44(&mut self, value: u8) {
        if value & 0x80 != 0 {
            self.trigger();
        }
    }
}

// ============================================================================
// APU 主結構体
// ============================================================================

pub struct APU {
    // 音頻通道
    pub ch1: Channel1,
    pub ch2: Channel2,
    pub ch3: Channel3,
    pub ch4: Channel4,

    // Logger
    pub logger: RefCell<Logger>,

    // 寄存器
    pub nr10: u8, // NR10 - 聲道1掃頻控制
    pub nr11: u8, // NR11 - 聲道1長度/工作週期
    pub nr12: u8, // NR12 - 聲道1音量包絡
    pub nr13: u8, // NR13 - 聲道1頻率低位
    pub nr14: u8, // NR14 - 聲道1頻率高位/控制

    pub nr21: u8, // NR21 - 聲道2長度/工作週期
    pub nr22: u8, // NR22 - 聲道2音量包絡
    pub nr23: u8, // NR23 - 聲道2頻率低位
    pub nr24: u8, // NR24 - 聲道2頻率高位/控制

    pub nr30: u8, // NR30 - 聲道3啟用
    pub nr31: u8, // NR31 - 聲道3長度
    pub nr32: u8, // NR32 - 聲道3音量
    pub nr33: u8, // NR33 - 聲道3頻率低位
    pub nr34: u8, // NR34 - 聲道3頻率高位/控制

    pub nr41: u8, // NR41 - 聲道4長度
    pub nr42: u8, // NR42 - 聲道4音量包絡
    pub nr43: u8, // NR43 - 聲道4頻率/隨機性
    pub nr44: u8, // NR44 - 聲道4控制

    pub nr50: u8, // NR50 - 主音量/VIN
    pub nr51: u8, // NR51 - 聲道混音
    pub nr52: u8, // NR52 - 聲音啟用/狀態

    // 調試相關
    pub debug_enabled: bool,
    pub debug_file: Option<File>,
    pub register_write_count: u64,
    pub audio_step_count: u64,

    // APU 主控制
    pub control: APUControl,

    frame_sequencer: u8,
    frame_cycles: u32,

    mmu: Rc<RefCell<MMU>>,
}

pub struct APUControl {
    pub enabled: bool,
    pub left_volume: u8,
    pub right_volume: u8,
    pub left_enables: [bool; 4],  // 左聲道啟用狀態
    pub right_enables: [bool; 4], // 右聲道啟用狀態
    pub vin_left_enable: bool,
    pub vin_right_enable: bool,
}

impl APUControl {
    pub fn new() -> Self {
        Self {
            enabled: false,
            left_volume: 0,
            right_volume: 0,
            left_enables: [false; 4],
            right_enables: [false; 4],
            vin_left_enable: false,
            vin_right_enable: false,
        }
    }

    pub fn write_nr50(&mut self, value: u8) {
        self.left_volume = (value >> 4) & 0x7;
        self.right_volume = value & 0x7;
        self.vin_left_enable = value & 0x80 != 0;
        self.vin_right_enable = value & 0x08 != 0;
    }

    pub fn write_nr51(&mut self, value: u8) {
        for i in 0..4 {
            self.left_enables[i] = value & (0x10 << i) != 0;
            self.right_enables[i] = value & (0x01 << i) != 0;
        }
    }

    pub fn write_nr52(&mut self, value: u8) {
        self.enabled = value & 0x80 != 0;
    }

    pub fn read_nr50(&self) -> u8 {
        (if self.vin_left_enable { 0x80 } else { 0 })
            | (self.left_volume << 4)
            | (if self.vin_right_enable { 0x08 } else { 0 })
            | self.right_volume
    }

    pub fn read_nr51(&self) -> u8 {
        let mut value = 0;
        for i in 0..4 {
            if self.left_enables[i] {
                value |= 0x10 << i;
            }
            if self.right_enables[i] {
                value |= 0x01 << i;
            }
        }
        value
    }

    pub fn read_nr52(&self) -> u8 {
        if self.enabled {
            0x80
        } else {
            0
        }
    }
}

impl APU {
    pub fn new(mmu: Rc<RefCell<MMU>>, logger: RefCell<Logger>) -> Self {
        let mut apu = Self {
            ch1: Channel1::new(),
            ch2: Channel2::new(),
            ch3: Channel3::new(),
            ch4: Channel4::new(),
            logger,
            nr10: 0,
            nr11: 0,
            nr12: 0,
            nr13: 0,
            nr14: 0,
            nr21: 0,
            nr22: 0,
            nr23: 0,
            nr24: 0,
            nr30: 0,
            nr31: 0,
            nr32: 0,
            nr33: 0,
            nr34: 0,
            nr41: 0,
            nr42: 0,
            nr43: 0,
            nr44: 0,
            nr50: 0,
            nr51: 0,
            nr52: 0,
            debug_enabled: cfg!(debug_assertions),
            debug_file: None,
            register_write_count: 0,
            audio_step_count: 0,
            control: APUControl::new(),
            frame_sequencer: 0,
            frame_cycles: 0,
            mmu, // 使用傳入的 MMU
        };
        apu.reset();
        apu
    }

    pub fn reset(&mut self) {
        // 重置 APU 狀態
        self.ch1 = Channel1::new();
        self.ch2 = Channel2::new();
        self.ch3 = Channel3::new();
        self.ch4 = Channel4::new();

        // 重置除NR52外的所有寄存器
        self.nr10 = 0x80;
        self.nr11 = 0xBF;
        self.nr12 = 0xF3;
        self.nr13 = 0xFF;
        self.nr14 = 0xBF;

        self.nr21 = 0x3F;
        self.nr22 = 0x00;
        self.nr23 = 0xFF;
        self.nr24 = 0xBF;

        self.nr30 = 0x7F;
        self.nr31 = 0xFF;
        self.nr32 = 0x9F;
        self.nr33 = 0xFF;
        self.nr34 = 0xBF;

        self.nr41 = 0xFF;
        self.nr42 = 0x00;
        self.nr43 = 0x00;
        self.nr44 = 0xBF;

        self.nr50 = 0x77;
        self.nr51 = 0xF3;

        if self.debug_enabled {
            let mut logger = self.logger.borrow_mut();
            logger.debug("APU狀態重置");
        }
    }

    pub fn step(&mut self) -> Result<(), String> {
        if (self.nr52 & 0x80) == 0 {
            // APU關閉時不處理音頻
            return Ok(());
        }

        self.audio_step_count += 1;

        // 更新框架定序器 (每 8192 個時鐘週期)
        self.frame_cycles += 1;
        if self.frame_cycles >= 8192 {
            self.frame_cycles = 0;
            self.frame_sequencer = (self.frame_sequencer + 1) % 8;

            match self.frame_sequencer {
                0 | 4 => self.clock_length(), // 長度計數器 (256Hz)
                2 | 6 => {
                    self.clock_length();
                    self.clock_sweep(); // 掃頻 (128Hz)
                }
                7 => self.clock_envelope(), // 音量包絡 (64Hz)
                _ => {}
            }
        }

        // 更新所有通道
        self.ch1.step();
        self.ch2.step();
        self.ch3.step();
        self.ch4.step();

        // 每10000步記錄一次調試信息
        if self.debug_enabled && self.audio_step_count % 10000 == 0 {
            self.log_audio_state();
        }

        Ok(())
    }

    fn clock_length(&mut self) {
        // 實現長度計數器更新邏輯
        if self.ch1.length_counter > 0 {
            self.ch1.length_counter -= 1;
            if self.ch1.length_counter == 0 {
                self.ch1.enabled = false;
            }
        }
        // 對其他通道執行相同操作...
    }

    fn clock_sweep(&mut self) {
        // 實現 Channel 1 的掃頻功能
        if self.ch1.sweep_enabled && self.ch1.sweep_period > 0 {
            let delta = self.ch1.frequency >> self.ch1.sweep_shift;
            if self.ch1.sweep_direction {
                self.ch1.frequency = self.ch1.frequency.saturating_sub(delta);
            } else {
                self.ch1.frequency = self.ch1.frequency.saturating_add(delta);
            }
        }
    }

    fn clock_envelope(&mut self) {
        // 實現音量包絡更新邏輯
    }

    pub fn get_samples(&self) -> (f32, f32) {
        if !self.control.enabled {
            return (0.0, 0.0);
        }

        let mut left = 0.0;
        let mut right = 0.0;

        // 混音左聲道
        if self.control.left_enables[0] {
            left += self.ch1.get_sample();
        }
        if self.control.left_enables[1] {
            left += self.ch2.get_sample();
        }
        if self.control.left_enables[2] {
            left += self.ch3.get_sample();
        }
        if self.control.left_enables[3] {
            left += self.ch4.get_sample();
        }

        // 混音右聲道
        if self.control.right_enables[0] {
            right += self.ch1.get_sample();
        }
        if self.control.right_enables[1] {
            right += self.ch2.get_sample();
        }
        if self.control.right_enables[2] {
            right += self.ch3.get_sample();
        }
        if self.control.right_enables[3] {
            right += self.ch4.get_sample();
        }

        // 應用主音量控制
        left *= (self.control.left_volume as f32 + 1.0) / 8.0;
        right *= (self.control.right_volume as f32 + 1.0) / 8.0;

        (left, right)
    }

    pub fn read_reg(&self, addr: u16) -> u8 {
        match addr {
            0xFF10..=0xFF14 => self.read_ch1_reg(addr),
            0xFF15..=0xFF19 => self.read_ch2_reg(addr),
            0xFF1A..=0xFF1E => self.read_ch3_reg(addr),
            0xFF1F..=0xFF23 => self.read_ch4_reg(addr),
            0xFF24 => self.control.read_nr50(),
            0xFF25 => self.control.read_nr51(),
            0xFF26 => self.control.read_nr52(),
            0xFF30..=0xFF3F => self.ch3.read_wave_ram((addr - 0xFF30) as usize),
            _ => 0xFF,
        }
    }

    pub fn write_reg(&mut self, addr: u16, value: u8) {
        if !self.control.enabled && addr != 0xFF26 && addr < 0xFF30 {
            return;
        }

        match addr {
            0xFF10..=0xFF14 => self.write_ch1_reg(addr, value),
            0xFF15..=0xFF19 => self.write_ch2_reg(addr, value),
            0xFF1A..=0xFF1E => self.write_ch3_reg(addr, value),
            0xFF1F..=0xFF23 => self.write_ch4_reg(addr, value),
            0xFF24 => self.control.write_nr50(value),
            0xFF25 => self.control.write_nr51(value),
            0xFF26 => self.control.write_nr52(value),
            0xFF30..=0xFF3F => self.ch3.write_wave_ram((addr - 0xFF30) as usize, value),
            _ => {}
        }
    }

    // 通道1寄存器訪問方法
    fn read_ch1_reg(&self, addr: u16) -> u8 {
        match addr {
            0xFF10 => {
                ((self.ch1.sweep_period << 4)
                    | (if self.ch1.sweep_direction { 0x08 } else { 0 })
                    | self.ch1.sweep_shift)
                    | 0x80
            }
            0xFF11 => (self.ch1.duty_cycle << 6) | 0x3F,
            0xFF12 => (self.ch1.volume << 4) | 0x00,
            0xFF13 => 0xFF,
            0xFF14 => {
                if self.ch1.enabled {
                    0xFF
                } else {
                    0x7F
                }
            }
            _ => 0xFF,
        }
    }

    fn write_ch1_reg(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF10 => {
                self.ch1.sweep_period = (value >> 4) & 0x07;
                self.ch1.sweep_direction = (value & 0x08) != 0;
                self.ch1.sweep_shift = value & 0x07;
            }
            0xFF11 => {
                self.ch1.duty_cycle = (value >> 6) & 0x03;
                self.ch1.length_counter = value & 0x3F;
            }
            0xFF12 => {
                self.ch1.volume = value >> 4;
            }
            0xFF13 => {
                self.ch1.frequency = (self.ch1.frequency & 0x700) | value as u16;
            }
            0xFF14 => {
                self.ch1.frequency = (self.ch1.frequency & 0xFF) | ((value as u16 & 0x07) << 8);
                if value & 0x80 != 0 {
                    self.ch1.trigger();
                }
            }
            _ => {}
        }
    }

    // 通道2寄存器訪問方法
    fn read_ch2_reg(&self, addr: u16) -> u8 {
        match addr {
            0xFF16 => (self.ch2.duty_cycle << 6) | 0x3F,
            0xFF17 => (self.ch2.volume << 4) | 0x00,
            0xFF18 => 0xFF,
            0xFF19 => {
                if self.ch2.enabled {
                    0xFF
                } else {
                    0x7F
                }
            }
            _ => 0xFF,
        }
    }

    fn write_ch2_reg(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF16 => {
                self.ch2.duty_cycle = (value >> 6) & 0x03;
                self.ch2.length_counter = value & 0x3F;
            }
            0xFF17 => {
                self.ch2.volume = value >> 4;
            }
            0xFF18 => {
                self.ch2.frequency = (self.ch2.frequency & 0x700) | value as u16;
            }
            0xFF19 => {
                self.ch2.frequency = (self.ch2.frequency & 0xFF) | ((value as u16 & 0x07) << 8);
                if value & 0x80 != 0 {
                    self.ch2.trigger();
                }
            }
            _ => {}
        }
    }

    // 通道3寄存器訪問方法
    fn read_ch3_reg(&self, addr: u16) -> u8 {
        match addr {
            0xFF1A => {
                if self.ch3.enabled {
                    0xFF
                } else {
                    0x7F
                }
            }
            0xFF1B => 0xFF,
            0xFF1C => (self.ch3.volume << 5) | 0x9F,
            0xFF1D => 0xFF,
            0xFF1E => {
                if self.ch3.enabled {
                    0xFF
                } else {
                    0x7F
                }
            }
            _ => 0xFF,
        }
    }

    fn write_ch3_reg(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF1A => {
                self.ch3.enabled = value & 0x80 != 0;
            }
            0xFF1B => {
                self.ch3.length_counter = value as u16;
            }
            0xFF1C => {
                self.ch3.volume = (value >> 5) & 0x03;
            }
            0xFF1D => {
                self.ch3.frequency = (self.ch3.frequency & 0x700) | value as u16;
            }
            0xFF1E => {
                self.ch3.frequency = (self.ch3.frequency & 0xFF) | ((value as u16 & 0x07) << 8);
                if value & 0x80 != 0 {
                    self.ch3.trigger();
                }
            }
            _ => {}
        }
    }

    // 通道4寄存器訪問方法
    fn read_ch4_reg(&self, addr: u16) -> u8 {
        match addr {
            0xFF20 => 0xFF,
            0xFF21 => {
                (self.ch4.volume << 4)
                    | (if self.ch4.envelope_direction { 0x08 } else { 0 })
                    | self.ch4.envelope_period
            }
            0xFF22 => {
                (self.ch4.clock_shift << 4)
                    | (if self.ch4.width_mode { 0x08 } else { 0 })
                    | self.ch4.divisor_code
            }
            0xFF23 => {
                if self.ch4.enabled {
                    0xFF
                } else {
                    0x7F
                }
            }
            _ => 0xFF,
        }
    }

    fn write_ch4_reg(&mut self, addr: u16, value: u8) {
        match addr {
            0xFF20 => self.ch4.write_nr41(value),
            0xFF21 => self.ch4.write_nr42(value),
            0xFF22 => self.ch4.write_nr43(value),
            0xFF23 => self.ch4.write_nr44(value),
            _ => {}
        }
    }

    fn reset_apu_state(&mut self) {
        // 重置APU到初始狀態
        self.ch1 = Channel1::new();
        self.ch2 = Channel2::new();
        self.ch3 = Channel3::new();
        self.ch4 = Channel4::new();

        // 重置除NR52外的所有寄存器
        self.nr10 = 0x80;
        self.nr11 = 0xBF;
        self.nr12 = 0xF3;
        self.nr13 = 0xFF;
        self.nr14 = 0xBF;

        self.nr21 = 0x3F;
        self.nr22 = 0x00;
        self.nr23 = 0xFF;
        self.nr24 = 0xBF;

        self.nr30 = 0x7F;
        self.nr31 = 0xFF;
        self.nr32 = 0x9F;
        self.nr33 = 0xFF;
        self.nr34 = 0xBF;

        self.nr41 = 0xFF;
        self.nr42 = 0x00;
        self.nr43 = 0x00;
        self.nr44 = 0xBF;

        self.nr50 = 0x77;
        self.nr51 = 0xF3;

        if self.debug_enabled {
            self.logger.borrow_mut().debug("APU狀態重置");
        }
    }

    fn log_register_write(&mut self, addr: u16, value: u8) {
        if let Some(ref mut file) = self.debug_file {
            let timestamp = Local::now().format("%H:%M:%S%.3f");
            let register_name = match addr {
                0xFF10 => "NR10",
                0xFF11 => "NR11",
                0xFF12 => "NR12",
                0xFF13 => "NR13",
                0xFF14 => "NR14",
                0xFF16 => "NR21",
                0xFF17 => "NR22",
                0xFF18 => "NR23",
                0xFF19 => "NR24",
                0xFF1A => "NR30",
                0xFF1B => "NR31",
                0xFF1C => "NR32",
                0xFF1D => "NR33",
                0xFF1E => "NR34",
                0xFF20 => "NR41",
                0xFF21 => "NR42",
                0xFF22 => "NR43",
                0xFF23 => "NR44",
                0xFF24 => "NR50",
                0xFF25 => "NR51",
                0xFF26 => "NR52",
                0xFF30..=0xFF3F => "WAVE",
                _ => "UNK",
            };

            let log_entry = format!(
                "[{}] 寄存器寫入: {} (0x{:04X}) = 0x{:02X}\n",
                timestamp, register_name, addr, value
            );
            let _ = file.write_all(log_entry.as_bytes());
            let _ = file.flush();
        }
    }

    fn log_audio_state(&mut self) {
        if let Some(ref mut file) = self.debug_file {
            let timestamp = Local::now().format("%H:%M:%S%.3f");
            let log_entry = format!(
                "[{}] 音頻狀態: CH1={:.3} CH2={:.3} CH3={:.3} CH4={:.3} (步驟: {})\n",
                timestamp,
                self.ch1.get_sample(),
                self.ch2.get_sample(),
                self.ch3.get_sample(),
                self.ch4.get_sample(),
                self.audio_step_count
            );
            let _ = file.write_all(log_entry.as_bytes());
            let _ = file.flush();
        }
    }

    fn log_debug_message(&mut self, message: &str) {
        if let Some(ref mut file) = self.debug_file {
            let timestamp = Local::now().format("%H:%M:%S%.3f");
            let log_entry = format!("[{}] {}\n", timestamp, message);
            let _ = file.write_all(log_entry.as_bytes());
            let _ = file.flush();
        }
    }
    pub fn set_debug_mode(&mut self, enabled: bool) {
        self.debug_enabled = enabled;
        if enabled && self.debug_file.is_none() {
            self.debug_file = File::create("debug_report/apu_debug.txt").ok();
        }
    }

    pub fn generate_status_report(&self) -> String {
        format!(
            "================================================================================\n\
            Game Boy APU 狀態報告\n\
            ================================================================================\n\
            \n\
            APU 主控制:\n\
            - 啟用狀態: {}\n\
            - 主音量: L={}, R={}\n\
            - 通道混音: 0x{:02X}\n\
            \n\
            通道狀態:\n\
            - 通道1 (方波+掃頻): {} (音量: {}, 頻率: {}, 樣本: {:.3})\n\
            - 通道2 (方波): {} (音量: {}, 頻率: {}, 樣本: {:.3})\n\
            - 通道3 (波形表): {} (音量等級: {}, 頻率: {}, 樣本: {:.3})\n\
            - 通道4 (噪音): {} (音量: {}, 樣本: {:.3})\n\
            \n\
            統計信息:\n\
            - 寄存器寫入次數: {}\n\
            - 音頻步驟計數: {}\n\
            \n\
            ================================================================================\n",
            if (self.nr52 & 0x80) != 0 {
                "啟用"
            } else {
                "禁用"
            },
            (self.nr50 >> 4) & 0x07,
            self.nr50 & 0x07,
            self.nr51,
            if self.ch1.enabled { "啟用" } else { "禁用" },
            self.ch1.volume,
            self.ch1.frequency,
            self.ch1.get_sample(),
            if self.ch2.enabled { "啟用" } else { "禁用" },
            self.ch2.volume,
            self.ch2.frequency,
            self.ch2.get_sample(),
            if self.ch3.enabled { "啟用" } else { "禁用" },
            self.ch3.volume,
            self.ch3.frequency,
            self.ch3.get_sample(),
            if self.ch4.enabled { "啟用" } else { "禁用" },
            self.ch4.volume,
            self.ch4.get_sample(),
            self.register_write_count,
            self.audio_step_count
        )
    }
    pub fn save_final_report(&self) {
        let report_path = "debug_report/apu_final_report.txt";
        if let Ok(mut file) = File::create(report_path) {
            let report = self.generate_status_report();
            let _ = file.write_all(report.as_bytes());
            let _ = file.flush();
            if let Some(logger) = self.logger.as_mut() {
                logger.info("APU最終報告已生成: {}", report_path);
            }
        }
    }

    pub fn tick(&mut self, cycles: u32) {
        // 暫時不實現聲音功能
        // TODO: 實現聲音處理邏輯
    }
}

// ============================================================================
// 測試模組
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_apu_initialization() {
        let mmu = Rc::new(RefCell::new(MMU::new(Vec::new())));
        let logger = RefCell::new(Logger::new());
        let apu = APU::new(mmu.clone(), logger);
        assert_eq!(apu.nr52 & 0x80, 0x80); // APU應該預設啟用
        assert_eq!(apu.nr50, 0x77); // 預設主音量
        assert_eq!(apu.nr51, 0xF3); // 預設混音設置
    }
    #[test]
    fn test_channel1_square_wave() {
        let mmu = Rc::new(RefCell::new(MMU::new(Vec::new())));
        let logger = RefCell::new(Logger::new());
        let mut apu = APU::new(mmu.clone(), logger);

        // 設置聲道1為方波
        apu.write_reg(0xFF12, 0xF0); // 最大音量
        apu.write_reg(0xFF11, 0x80); // 50%工作週期
        apu.write_reg(0xFF13, 0x00); // 頻率低位
        apu.write_reg(0xFF14, 0x87); // 頻率高位 + 觸發

        // 運行幾個步驟
        for _ in 0..1000 {
            apu.step().unwrap();
        }

        // 檢查是否生成音頻樣本
        let sample = apu.ch1.get_sample();
        assert!(sample >= -1.0 && sample <= 1.0);
    }

    #[test]
    fn test_apu_enable_disable() {
        let mmu = Rc::new(RefCell::new(MMU::new(Vec::new())));
        let logger = RefCell::new(Logger::new());
        let mut apu = APU::new(mmu.clone(), logger);

        // 測試APU關閉
        apu.write_reg(0xFF26, 0x00);
        assert_eq!(apu.nr52 & 0x80, 0x00);
    }

    #[test]
    fn test_channel1_frequency() {
        let mmu = Rc::new(RefCell::new(MMU::new(Vec::new())));
        let logger = RefCell::new(Logger::new());
        let mut apu = APU::new(mmu.clone(), logger);

        // 設置頻率
        apu.write_reg(0xFF13, 0x34); // 頻率低位
        apu.write_reg(0xFF14, 0x86); // 頻率高位 + 觸發

        // 檢查頻率設置
        assert_eq!(apu.ch1.frequency & 0xFF, 0x34);
        assert_eq!((apu.ch1.frequency >> 8) & 0x07, 0x06);
    }

    #[test]
    fn test_channel2_volume() {
        let mmu = Rc::new(RefCell::new(MMU::new(Vec::new())));
        let logger = RefCell::new(Logger::new());
        let mut apu = APU::new(mmu.clone(), logger);

        // 設置音量
        apu.write_reg(0xFF17, 0xF0); // 最大音量
        assert_eq!(apu.ch2.volume, 15);
    }

    #[test]
    fn test_channel3_wave_pattern() {
        let mmu = Rc::new(RefCell::new(MMU::new(Vec::new())));
        let logger = RefCell::new(Logger::new());
        let mut apu = APU::new(mmu.clone(), logger);

        // 測試波形表設定
        for i in 0..16 {
            apu.ch3.wave_ram[i] = i as u8;
        }
        assert_eq!(apu.ch3.wave_ram[0], 0);
        assert_eq!(apu.ch3.wave_ram[15], 15);
    }
}
