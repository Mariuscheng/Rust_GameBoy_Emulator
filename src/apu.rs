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

use chrono::Local;
use std::fs::File;
use std::io::Write;

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

        let freq_hz = 65536.0 / (2048.0 - self.frequency as f32);
        self.phase += freq_hz / 4194304.0;

        if self.phase >= 1.0 {
            self.phase -= 1.0;
        }

        // 從波形表獲取樣本
        let sample_index = (self.phase * 32.0) as usize % 32;
        let wave_sample = self.wave_ram[sample_index];

        // 應用音量設置
        let volume_shift = match self.volume {
            0 => 4, // 靜音
            1 => 0, // 100%
            2 => 1, // 50%
            3 => 2, // 25%
            _ => 4,
        };

        let sample_value = (wave_sample >> volume_shift) as f32 / 15.0;
        self.sample = (sample_value - 0.5) * 0.5; // 轉換到 -0.25 到 0.25 範圍
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
    pub sample: f32,
    pub lfsr: u16, // 線性反饋移位寄存器
    pub clock_divider: u32,
    pub clock_counter: u32,
}

impl Channel4 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            length_counter: 0,
            volume: 0,
            sample: 0.0,
            lfsr: 0x7FFF,
            clock_divider: 1,
            clock_counter: 0,
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

        self.clock_counter += 1;
        if self.clock_counter >= self.clock_divider {
            self.clock_counter = 0;

            // 更新LFSR
            let bit = ((self.lfsr >> 1) ^ self.lfsr) & 1;
            self.lfsr >>= 1;
            self.lfsr |= bit << 14;

            // 生成噪音樣本
            let noise_bit = (self.lfsr & 1) as f32;
            self.sample = if noise_bit != 0.0 {
                (self.volume as f32 / 15.0) * 0.5
            } else {
                -(self.volume as f32 / 15.0) * 0.5
            };
        }
    }

    pub fn trigger(&mut self) {
        self.enabled = true;
        self.lfsr = 0x7FFF;
        self.clock_counter = 0;
    }
}

// ============================================================================
// APU 主結構體
// ============================================================================

pub struct APU {
    // 音頻通道
    pub ch1: Channel1,
    pub ch2: Channel2,
    pub ch3: Channel3,
    pub ch4: Channel4,

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
}

impl APU {
    pub fn new() -> Self {
        let debug_file = if cfg!(debug_assertions) {
            File::create("c:\\Users\\mariu\\Desktop\\Rust\\gameboy_emulator\\gameboy_emulator\\apu_debug.txt").ok()
        } else {
            None
        };

        Self {
            ch1: Channel1::new(),
            ch2: Channel2::new(),
            ch3: Channel3::new(),
            ch4: Channel4::new(),

            nr10: 0x80,
            nr11: 0xBF,
            nr12: 0xF3,
            nr13: 0xFF,
            nr14: 0xBF,

            nr21: 0x3F,
            nr22: 0x00,
            nr23: 0xFF,
            nr24: 0xBF,

            nr30: 0x7F,
            nr31: 0xFF,
            nr32: 0x9F,
            nr33: 0xFF,
            nr34: 0xBF,

            nr41: 0xFF,
            nr42: 0x00,
            nr43: 0x00,
            nr44: 0xBF,

            nr50: 0x77,
            nr51: 0xF3,
            nr52: 0xF1, // APU啟用，所有通道啟用

            debug_enabled: cfg!(debug_assertions),
            debug_file,
            register_write_count: 0,
            audio_step_count: 0,
        }
    }

    pub fn step(&mut self) {
        if (self.nr52 & 0x80) == 0 {
            // APU關閉時不處理音頻
            return;
        }

        self.audio_step_count += 1;

        // 更新所有通道
        self.ch1.step();
        self.ch2.step();
        self.ch3.step();
        self.ch4.step();

        // 每10000步記錄一次調試信息
        if self.debug_enabled && self.audio_step_count % 10000 == 0 {
            self.log_audio_state();
        }
    }

    pub fn read_reg(&self, addr: u16) -> u8 {
        match addr {
            0xFF10 => self.nr10,
            0xFF11 => self.nr11,
            0xFF12 => self.nr12,
            0xFF13 => self.nr13,
            0xFF14 => self.nr14,

            0xFF16 => self.nr21,
            0xFF17 => self.nr22,
            0xFF18 => self.nr23,
            0xFF19 => self.nr24,

            0xFF1A => self.nr30,
            0xFF1B => self.nr31,
            0xFF1C => self.nr32,
            0xFF1D => self.nr33,
            0xFF1E => self.nr34,

            0xFF20 => self.nr41,
            0xFF21 => self.nr42,
            0xFF22 => self.nr43,
            0xFF23 => self.nr44,

            0xFF24 => self.nr50,
            0xFF25 => self.nr51,
            0xFF26 => self.nr52,

            // 波形表 RAM (0xFF30-0xFF3F)
            0xFF30..=0xFF3F => {
                let index = ((addr - 0xFF30) * 2) as usize;
                if index < self.ch3.wave_ram.len() {
                    self.ch3.wave_ram[index]
                } else {
                    0xFF
                }
            }

            _ => 0xFF,
        }
    }

    pub fn write_reg(&mut self, addr: u16, value: u8) {
        if self.debug_enabled {
            self.register_write_count += 1;
            self.log_register_write(addr, value);
        }

        // 如果APU關閉，只允許寫入NR52
        if (self.nr52 & 0x80) == 0 && addr != 0xFF26 {
            return;
        }

        match addr {
            // 聲道1寄存器
            0xFF10 => {
                self.nr10 = value;
                self.ch1.sweep_period = (value >> 4) & 0x07;
                self.ch1.sweep_direction = (value & 0x08) != 0;
                self.ch1.sweep_shift = value & 0x07;
                self.ch1.sweep_enabled = self.ch1.sweep_period > 0 || self.ch1.sweep_shift > 0;
            }
            0xFF11 => {
                self.nr11 = value;
                self.ch1.duty_cycle = (value >> 6) & 0x03;
                self.ch1.length_counter = 64 - (value & 0x3F);
            }
            0xFF12 => {
                self.nr12 = value;
                self.ch1.volume = (value >> 4) & 0x0F;
            }
            0xFF13 => {
                self.nr13 = value;
                self.ch1.frequency = (self.ch1.frequency & 0xFF00) | value as u16;
            }
            0xFF14 => {
                self.nr14 = value;
                self.ch1.frequency = (self.ch1.frequency & 0x00FF) | ((value as u16 & 0x07) << 8);
                if (value & 0x80) != 0 {
                    self.ch1.trigger();
                }
            }

            // 聲道2寄存器
            0xFF16 => {
                self.nr21 = value;
                self.ch2.duty_cycle = (value >> 6) & 0x03;
                self.ch2.length_counter = 64 - (value & 0x3F);
            }
            0xFF17 => {
                self.nr22 = value;
                self.ch2.volume = (value >> 4) & 0x0F;
            }
            0xFF18 => {
                self.nr23 = value;
                self.ch2.frequency = (self.ch2.frequency & 0xFF00) | value as u16;
            }
            0xFF19 => {
                self.nr24 = value;
                self.ch2.frequency = (self.ch2.frequency & 0x00FF) | ((value as u16 & 0x07) << 8);
                if (value & 0x80) != 0 {
                    self.ch2.trigger();
                }
            }

            // 聲道3寄存器
            0xFF1A => {
                self.nr30 = value;
                self.ch3.enabled = (value & 0x80) != 0;
            }
            0xFF1B => {
                self.nr31 = value;
                self.ch3.length_counter = 256 - value as u16;
            }
            0xFF1C => {
                self.nr32 = value;
                self.ch3.volume = (value >> 5) & 0x03;
            }
            0xFF1D => {
                self.nr33 = value;
                self.ch3.frequency = (self.ch3.frequency & 0xFF00) | value as u16;
            }
            0xFF1E => {
                self.nr34 = value;
                self.ch3.frequency = (self.ch3.frequency & 0x00FF) | ((value as u16 & 0x07) << 8);
                if (value & 0x80) != 0 {
                    self.ch3.trigger();
                }
            }

            // 聲道4寄存器
            0xFF20 => {
                self.nr41 = value;
                self.ch4.length_counter = 64 - (value & 0x3F);
            }
            0xFF21 => {
                self.nr42 = value;
                self.ch4.volume = (value >> 4) & 0x0F;
            }
            0xFF22 => {
                self.nr43 = value;
                let divisor = match value & 0x07 {
                    0 => 8,
                    n => (n as u32) * 16,
                };
                let shift = (value >> 4) & 0x0F;
                self.ch4.clock_divider = divisor << shift;
            }
            0xFF23 => {
                self.nr44 = value;
                if (value & 0x80) != 0 {
                    self.ch4.trigger();
                }
            }

            // 主控制寄存器
            0xFF24 => {
                self.nr50 = value;
            }
            0xFF25 => {
                self.nr51 = value;
            }
            0xFF26 => {
                let was_enabled = (self.nr52 & 0x80) != 0;
                self.nr52 = (value & 0x80) | (self.nr52 & 0x7F);

                if !was_enabled && (value & 0x80) != 0 {
                    // APU啟用 - 重置狀態
                    self.reset_apu_state();
                } else if was_enabled && (value & 0x80) == 0 {
                    // APU關閉 - 清除所有通道
                    self.ch1.enabled = false;
                    self.ch2.enabled = false;
                    self.ch3.enabled = false;
                    self.ch4.enabled = false;
                }
            }

            // 波形表 RAM (0xFF30-0xFF3F)
            0xFF30..=0xFF3F => {
                let index = ((addr - 0xFF30) * 2) as usize;
                if index < self.ch3.wave_ram.len() {
                    // 每個寄存器包含2個4位樣本
                    self.ch3.wave_ram[index] = (value >> 4) & 0x0F;
                    if index + 1 < self.ch3.wave_ram.len() {
                        self.ch3.wave_ram[index + 1] = value & 0x0F;
                    }
                }
            }

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
            self.log_debug_message("APU狀態重置");
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
            self.debug_file = File::create("c:\\Users\\mariu\\Desktop\\Rust\\gameboy_emulator\\gameboy_emulator\\apu_debug.txt").ok();
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
        let report_path = "c:\\Users\\mariu\\Desktop\\Rust\\gameboy_emulator\\gameboy_emulator\\apu_final_report.txt";
        if let Ok(mut file) = File::create(report_path) {
            let report = self.generate_status_report();
            let _ = file.write_all(report.as_bytes());
            let _ = file.flush();
            println!("APU最終報告已生成: {}", report_path);
        }
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
        let apu = APU::new();
        assert_eq!(apu.nr52 & 0x80, 0x80); // APU應該預設啟用
        assert_eq!(apu.nr50, 0x77); // 預設主音量
        assert_eq!(apu.nr51, 0xF3); // 預設混音設置
    }

    #[test]
    fn test_channel1_square_wave() {
        let mut apu = APU::new();

        // 設置聲道1為方波
        apu.write_reg(0xFF12, 0xF0); // 最大音量
        apu.write_reg(0xFF11, 0x80); // 50%工作週期
        apu.write_reg(0xFF13, 0x00); // 頻率低位
        apu.write_reg(0xFF14, 0x87); // 頻率高位 + 觸發

        // 運行幾個步驟
        for _ in 0..1000 {
            apu.step();
        }

        // 檢查是否生成音頻樣本
        let sample = apu.ch1.get_sample();
        assert!(sample.abs() > 0.0, "聲道1應該生成音頻樣本");
    }

    #[test]
    fn test_apu_enable_disable() {
        let mut apu = APU::new();

        // 測試APU關閉
        apu.write_reg(0xFF26, 0x00);
        assert_eq!(apu.nr52 & 0x80, 0x00);

        // 測試APU重新啟用
        apu.write_reg(0xFF26, 0x80);
        assert_eq!(apu.nr52 & 0x80, 0x80);
    }
}
