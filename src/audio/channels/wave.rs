use super::super::common::AudioChannel;

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

    /// 取得波形 RAM 的內容
    pub fn read_wave_ram(&self, offset: u16) -> u8 {
        self.wave_ram[offset as usize]
    }

    /// 寫入波形 RAM
    pub fn write_wave_ram(&mut self, offset: u16, value: u8) {
        self.wave_ram[offset as usize] = value;
    } // 更新通道狀態
    fn step(&mut self, cycles: u32) {
        if !self.enabled {
            return;
        }

        // 更新定時器
        self.frame_cycles += cycles;
        if self.frame_cycles >= self.get_period() {
            self.frame_cycles = 0;
            self.update_sample();
        }

        // 重置定時器並更新位置
        self.timer = self.get_period();
        self.position = (self.position + 1) & 31;
    }

    // 更新長度計數器
    #[allow(dead_code)]
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

    /// 計算波形輸出的週期
    fn get_period(&self) -> u32 {
        let freq = 2048 - self.frequency as u32;
        freq * 2 // 除以 2MHz 的週期
    }

    /// 更新波形樣本
    fn update_sample(&mut self) {
        if !self.enabled || !self.dac_enabled {
            return;
        }

        // 更新位置
        self.position = (self.position + 1) % 32;

        // 如果長度計數器啟用且不為零，則遞減
        if self.length_enabled && self.length_counter > 0 {
            self.length_counter -= 1;
            if self.length_counter == 0 {
                self.enabled = false;
            }
        }
    }
}

impl AudioChannel for Wave {
    fn init(&mut self) {
        self.reset();
    }

    fn reset(&mut self) {
        self.enabled = false;
        self.length_enabled = false;
        self.length_counter = 0;
        self.output_level = 0;
        self.frequency = 0;
        self.position = 0;
        self.dac_enabled = false;
        self.frame_cycles = 0;
        self.wave_ram = [0; 32];
    }

    fn step(&mut self, cycles: u32) {
        if !self.enabled {
            return;
        }

        // 更新定時器
        self.frame_cycles += cycles;
        if self.frame_cycles >= self.get_period() {
            self.frame_cycles = 0;
            self.update_sample();
        }
    }

    fn get_sample(&self) -> f32 {
        if !self.enabled || !self.dac_enabled {
            return 0.0;
        }

        // 從波形 RAM 讀取當前樣本
        let sample = (self.wave_ram[self.position as usize]
            >> match self.output_level {
                0 => 4, // 0%
                1 => 0, // 100%
                2 => 1, // 50%
                3 => 2, // 25%
                _ => 4, // 靜音
            })
            & 0xF;

        // 轉換為 -1.0 到 1.0 之間的浮點數
        (sample as f32 / 7.5) - 1.0
    }

    fn is_enabled(&self) -> bool {
        self.enabled && self.dac_enabled
    }

    fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        if !enabled {
            self.length_counter = 0;
        }
    }

    fn write_register(&mut self, addr: u16, value: u8) {
        match addr & 0xFF {
            0x1A => {
                // NR30: Channel enable & DAC power
                self.dac_enabled = (value & 0x80) != 0;
                if !self.dac_enabled {
                    self.enabled = false;
                }
            }
            0x1B => {
                // NR31: Length timer
                self.length_counter = value;
            }
            0x1C => {
                // NR32: Output level
                self.output_level = (value >> 5) & 0x03;
            }
            0x1D => {
                // NR33: Frequency LSB
                self.frequency = (self.frequency & 0x700) | value as u16;
            }
            0x1E => {
                // NR34: Frequency MSB & control
                self.frequency = (self.frequency & 0xFF) | ((value as u16 & 0x07) << 8);
                self.length_enabled = (value & 0x40) != 0;

                if value & 0x80 != 0 {
                    // 觸發位
                    self.enabled = self.dac_enabled;
                    if self.length_counter == 0 {
                        self.length_counter = 255; // Wave 通道使用 256 步的長度計數器
                    }
                    self.position = 0; // 重置波形位置
                }
            }
            0x30..=0x3F => {
                // Wave Pattern RAM
                self.write_wave_ram(addr - 0x30, value);
            }
            _ => {}
        }
    }

    fn read_register(&self, addr: u16) -> u8 {
        match addr & 0xFF {
            0x1A => {
                // NR30
                (self.dac_enabled as u8) << 7
            }
            0x1B => {
                // NR31
                self.length_counter
            }
            0x1C => {
                // NR32
                self.output_level << 5
            }
            0x1D => {
                // NR33
                self.frequency as u8
            }
            0x1E => {
                // NR34
                ((self.length_enabled as u8) << 6) | ((self.frequency >> 8) as u8 & 0x07)
            }
            0x30..=0x3F => {
                // Wave Pattern RAM
                self.read_wave_ram(addr - 0x30)
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

    fn set_envelope(&mut self, _initial_volume: u8, _direction: bool, _period: u8) {
        // Wave 通道不使用音量包絡
    }

    fn get_volume(&self) -> u8 {
        match self.output_level {
            0 => 0,
            1 => 15, // 100%
            2 => 7,  // 50%
            3 => 3,  // 25%
            _ => 0,
        }
    }
}
