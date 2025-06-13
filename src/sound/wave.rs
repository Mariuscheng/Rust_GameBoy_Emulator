use super::registers::{NR30, NR31, NR32, NR33, NR34};

pub struct WaveChannel {
    enabled: bool,
    dac_enabled: bool,
    output_level: u8,
    frequency: u16,
    length_counter: u8,
    length_enable: bool,
    position: u8,
    timer: u32,
    wave_ram: [u8; 32],
}

impl WaveChannel {
    pub fn new() -> Self {
        Self {
            enabled: false,
            dac_enabled: false,
            output_level: 0,
            frequency: 0,
            length_counter: 0,
            length_enable: false,
            position: 0,
            timer: 0,
            wave_ram: [0; 32],
        }
    }

    // 讀取相關方法
    pub fn read_enable(&self) -> u8 {
        (self.dac_enabled as u8) << 7
    }

    pub fn read_length(&self) -> u8 {
        self.length_counter
    }

    pub fn read_volume(&self) -> u8 {
        self.output_level << 5
    }

    pub fn read_frequency_lo(&self) -> u8 {
        self.frequency as u8
    }

    pub fn read_frequency_hi(&self) -> u8 {
        ((self.length_enable as u8) << 6) | ((self.frequency >> 8) as u8)
    }

    pub fn read_wave_ram(&self, offset: u16) -> u8 {
        self.wave_ram[offset as usize]
    }

    // 寫入相關方法
    pub fn write_enable(&mut self, value: u8) {
        self.dac_enabled = (value & 0x80) != 0;
        if !self.dac_enabled {
            self.enabled = false;
        }
    }

    pub fn write_length(&mut self, value: u8) {
        self.length_counter = value;
    }

    pub fn write_volume(&mut self, value: u8) {
        self.output_level = (value >> 5) & 0x03;
    }

    pub fn write_frequency_lo(&mut self, value: u8) {
        self.frequency = (self.frequency & 0x700) | value as u16;
    }

    pub fn write_frequency_hi(&mut self, value: u8) {
        self.frequency = (self.frequency & 0xFF) | (((value & 0x07) as u16) << 8);
        self.length_enable = (value & 0x40) != 0;

        if value & 0x80 != 0 {
            self.trigger();
        }
    }

    pub fn write_wave_ram(&mut self, offset: u16, value: u8) {
        self.wave_ram[offset as usize] = value;
    }

    // 觸發通道
    fn trigger(&mut self) {
        self.enabled = self.dac_enabled;
        if self.length_counter == 0 {
            self.length_counter = 255;
        }
        self.position = 0;
        self.timer = self.get_period();
    }

    // 更新通道狀態
    pub fn step(&mut self) {
        if !self.enabled {
            return;
        }

        if self.timer > 0 {
            self.timer -= 1;
            return;
        }

        // 重置定時器並更新位置
        self.timer = self.get_period();
        self.position = (self.position + 1) & 31;
    }

    // 更新長度計數器
    pub fn step_length(&mut self) {
        if self.length_enable && self.length_counter > 0 {
            self.length_counter -= 1;
            if self.length_counter == 0 {
                self.enabled = false;
            }
        }
    }

    // 取得輸出值
    pub fn get_output(&self) -> i8 {
        if !self.enabled || !self.dac_enabled {
            return 0;
        }

        let sample = self.wave_ram[self.position as usize];
        let amplitude = match self.output_level {
            0 => 0,
            1 => sample,      // 100%
            2 => sample >> 1, // 50%
            3 => sample >> 2, // 25%
            _ => unreachable!(),
        };

        (amplitude as i8).saturating_sub(8) // 將範圍調整為 -8 到 7
    }

    // 檢查通道是否啟用
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    // 重置通道
    pub fn reset(&mut self) {
        self.enabled = false;
        self.dac_enabled = false;
        self.output_level = 0;
        self.frequency = 0;
        self.length_counter = 0;
        self.length_enable = false;
        self.position = 0;
        self.timer = 0;
        self.wave_ram = [0; 32];
    }

    // 開機初始化
    pub fn power_on(&mut self) {
        self.reset();
    }

    // 關機處理
    pub fn power_off(&mut self) {
        self.enabled = false;
    }

    // 輔助方法：計算波形週期
    fn get_period(&self) -> u32 {
        (2048 - self.frequency as u32) * 2
    }
}
