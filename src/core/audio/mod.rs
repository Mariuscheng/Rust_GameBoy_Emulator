// 音頻系統核心模組

pub mod apu;
pub mod channel;
pub mod channels;
pub mod registers;

pub use apu::APU;
pub use channel::Channel;
pub use registers::AudioRegisters;

use crate::core::cycles::CyclesType;
use crate::error::Result;
use crate::interface::audio::CpalAudioOutput;
use std::sync::{Arc, Mutex};

pub const SAMPLE_RATE: u32 = 44100;
pub const FRAME_SEQUENCER_RATE: u32 = 512;
pub const APU_CLOCK_RATE: u32 = 4194304;

/// 音頻系統實現
#[derive(Debug)]
#[allow(dead_code)]
pub struct AudioSystem {
    apu: APU,
    sample_buffer: Arc<Mutex<Vec<f32>>>,
    sample_rate: u32,
    enabled: bool,
}

impl AudioSystem {
    pub fn new() -> Result<Self> {
        let audio_output = Box::new(CpalAudioOutput::new(SAMPLE_RATE));
        Ok(Self {
            apu: APU::new(Some(audio_output)),
            sample_buffer: Arc::new(Mutex::new(Vec::new())),
            sample_rate: SAMPLE_RATE,
            enabled: true,
        })
    }

    pub fn step(&mut self, cycles: CyclesType) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }

        // 每 95 個CPU週期產生一個音頻樣本 (4194304 / 44100 ≈ 95)
        if self.apu.step(cycles)? {
            let sample = self.apu.get_sample();
            if let Ok(mut buffer) = self.sample_buffer.lock() {
                buffer.push(sample);
            }
        }

        Ok(())
    }

    pub fn toggle_channel(&mut self, channel: usize, enabled: bool) {
        self.apu
            .toggle_channel(channel.try_into().unwrap_or(0), enabled);
    }
    pub fn read_byte(&self, address: u16) -> Result<u8> {
        Ok(self.apu.read_byte(address))
    }

    pub fn write_byte(&mut self, address: u16, value: u8) -> Result<()> {
        self.apu.write_byte(address, value);
        Ok(())
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn set_enabled(&mut self, enabled: bool) {
        self.enabled = enabled;
        self.apu.set_enabled(enabled);
    }

    pub fn get_samples(&self) -> Vec<f32> {
        if let Ok(mut buffer) = self.sample_buffer.lock() {
            let samples = buffer.clone();
            buffer.clear();
            samples
        } else {
            Vec::new()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_audio_system_initialization() {
        let audio = AudioSystem::new().unwrap();
        assert!(audio.enabled);
    }
}
