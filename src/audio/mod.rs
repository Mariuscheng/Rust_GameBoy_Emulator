//! Game Boy 音效處理模組
//! 包含所有音效相關的功能：通道、混音、音頻處理等

mod channel; // 音效通道實現
pub mod common; // 通用特徵和類型
mod registers; // 內部寄存器定義

use crate::config::AudioConfig;

// 重新導出常用類型，讓使用者可以直接從 audio 模組中使用
pub use channel::{Noise, Square1, Square2, Wave};
pub use common::AudioChannel;

pub struct AudioProcessor {
    square1: Square1,
    square2: Square2,
    wave: Wave,
    noise: Noise,
    enabled: bool,
    frame_sequencer: u8,
    frame_cycles: u32,
    config: Option<AudioConfig>,
}

impl AudioProcessor {
    pub fn new() -> Self {
        AudioProcessor {
            square1: Square1::new(),
            square2: Square2::new(),
            wave: Wave::new(),
            noise: Noise::new(),
            enabled: false,
            frame_sequencer: 0,
            frame_cycles: 0,
            config: None,
        }
    }

    /// 重置音效處理器
    pub fn reset(&mut self) {
        self.enabled = false;
        self.frame_sequencer = 0;
        self.frame_cycles = 0;
        self.square1.reset();
        self.square2.reset();
        self.wave.reset();
        self.noise.reset();
    }

    /// 禁用音效處理器
    pub fn disable(&mut self) {
        self.enabled = false;
        self.reset();
    }

    /// 啟用音效處理器
    pub fn enable(&mut self) {
        self.enabled = true;
    }

    /// 更新音效處理器的狀態
    pub fn step(&mut self, cycles: u32) {
        if !self.enabled {
            return;
        }

        // 更新所有通道
        self.square1.step(cycles);
        self.square2.step(cycles);
        self.wave.step(cycles);
        self.noise.step(cycles);

        // 更新幀序列器
        self.frame_cycles += cycles;
        if self.frame_cycles >= 8192 {
            // 每 8192 個週期更新一次幀序列器（約 512Hz）
            self.frame_cycles -= 8192;
            self.frame_sequencer = (self.frame_sequencer + 1) % 8;
        }
    }

    /// 取得當前的音訊樣本
    pub fn get_sample(&self) -> (f32, f32) {
        if !self.enabled {
            return (0.0, 0.0);
        }

        // 混合所有通道的輸出
        let square1 = self.square1.get_sample();
        let square2 = self.square2.get_sample();
        let wave = self.wave.get_sample();
        let noise = self.noise.get_sample();

        // 簡單的均值混音
        let mixed = (square1 + square2 + wave + noise) / 4.0;
        (mixed, mixed) // 左右聲道相同
    }

    /// 讀取音效寄存器
    pub fn read_register(&self, addr: u16) -> u8 {
        match addr {
            0xFF10..=0xFF14 => self.square1.read_register(addr),
            0xFF15..=0xFF19 => self.square2.read_register(addr),
            0xFF1A..=0xFF1E => self.wave.read_register(addr),
            0xFF1F..=0xFF23 => self.noise.read_register(addr),
            0xFF24 => 0xFF, // NR50 - Master volume & VIN panning
            0xFF25 => 0xFF, // NR51 - Sound panning
            0xFF26 => {
                // NR52 - Sound on/off
                (self.enabled as u8) << 7
                    | (self.noise.is_enabled() as u8) << 3
                    | (self.wave.is_enabled() as u8) << 2
                    | (self.square2.is_enabled() as u8) << 1
                    | (self.square1.is_enabled() as u8)
            }
            _ => 0xFF,
        }
    }

    /// 寫入音效寄存器
    pub fn write_register(&mut self, addr: u16, value: u8) {
        if !self.enabled && addr != 0xFF26 {
            return;
        }

        match addr {
            0xFF10..=0xFF14 => self.square1.write_register(addr, value),
            0xFF15..=0xFF19 => self.square2.write_register(addr, value),
            0xFF1A..=0xFF1E => self.wave.write_register(addr, value),
            0xFF1F..=0xFF23 => self.noise.write_register(addr, value),
            0xFF24 => {} // NR50 - Master volume & VIN panning (未實作)
            0xFF25 => {} // NR51 - Sound panning (未實作)
            0xFF26 => {
                // NR52 - Sound on/off
                let enabled = (value & 0x80) != 0;
                if !enabled {
                    self.disable();
                } else if !self.enabled {
                    self.enable();
                }
            }
            _ => {}
        }
    }

    /// 設定音效組態
    pub fn init_config(&mut self, config: AudioConfig) {
        self.config = Some(config);
    }
}
