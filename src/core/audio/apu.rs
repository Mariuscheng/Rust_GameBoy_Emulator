use crate::error::Result;
use crate::interface::audio::AudioInterface;

#[derive(Debug)]
#[allow(dead_code)]
pub struct APU {
    pub audio_output: Option<Box<dyn AudioInterface>>,
    pub enabled: bool,
    cycles: u32,
    enable_flags: u8,
}

impl APU {
    pub fn new(audio_output: Option<Box<dyn AudioInterface>>) -> Self {
        Self {
            audio_output,
            enabled: false,
            cycles: 0,
            enable_flags: 0,
        }
    }

    pub fn update(&mut self, cycles: u32) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        self.cycles += cycles;
        // 每 4194304/60 個週期（約 70224）產生一次音訊樣本
        while self.cycles >= 70224 {
            if let Some(output) = &mut self.audio_output {
                // 簡化版本：推送靜音樣本
                output.push_sample(0.0);
            }
            self.cycles -= 70224;
        }

        Ok(())
    }

    pub fn reset(&mut self) -> Result<()> {
        self.enabled = false;
        self.cycles = 0;
        Ok(())
    }

    // 簡化版本的寄存器讀寫
    pub fn read_register(&self, _address: u16) -> Result<u8> {
        Ok(0)
    }

    pub fn write_register(&mut self, _address: u16, _value: u8) {
        // 簡化版本：不做任何操作
    }

    pub fn step(&mut self, cycles: u32) -> Result<bool> {
        self.update(cycles)?;
        Ok(false) // 簡化版本，不產生中斷
    }

    pub fn get_sample(&self) -> f32 {
        0.0 // 簡化版本，返回靜音
    }

    pub fn toggle_channel(&mut self, _channel: u8, _enabled: bool) {
        // 簡化版本：不做任何操作
    }

    pub fn read_byte(&self, address: u16) -> u8 {
        self.read_register(address).unwrap_or(0)
    }

    pub fn write_byte(&mut self, address: u16, value: u8) {
        self.write_register(address, value);
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
    }
}
