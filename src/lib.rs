// Game Boy Emulator Main Module
#![forbid(unsafe_code)]

use std::cell::RefCell;
use std::rc::Rc;

// Core emulator modules
pub mod core;

// Error handling
pub mod error;

// Interface layer
pub mod interface;

// Configuration management
pub mod config;

// Utility functions
pub mod utils;

// Debug features
#[cfg(debug_assertions)]
pub mod debugger;

// Test modules
#[cfg(test)]
pub mod tests;

// Re-exports for public API
// pub use crate::emulator::core::Emulator as Core;
pub use error::{Error, Result};
pub use interface::{audio::AudioInterface, input::joypad::Joypad, video::VideoInterface};

// Re-export core modules for external use
pub use crate::core::cpu::CPU;
pub use crate::core::mmu::MMU;
pub use crate::core::ppu::PPU;

/// Main structure of the GameBoy emulator
#[derive(Debug)]
pub struct GameBoy {
    mmu: Rc<RefCell<MMU>>,
    ppu: PPU,
    cpu: CPU,
    render_count: u64,
}

impl GameBoy {
    pub fn new(
        video: Box<dyn VideoInterface>,
        _audio: Option<Box<dyn AudioInterface>>,
    ) -> Result<Self> {
        let mmu = Rc::new(RefCell::new(MMU::new())); // Don't initialize any LCD registers here - let ROM control everything
                                                     // Removed logging for better performance

        let ppu = PPU::new(Rc::clone(&mmu), video);
        let cpu = CPU::new(Rc::clone(&mmu));

        Ok(Self {
            mmu,
            ppu,
            cpu,
            render_count: 0,
        })
    }
    pub fn load_rom(&mut self, rom_data: Vec<u8>) -> Result<()> {
        let mut mmu = self.mmu.borrow_mut();
        mmu.load_rom(&rom_data)?;

        // Log ROM entry point for debugging
        if rom_data.len() > 0x100 {
            println!("ROM Entry Point Instructions:");
            for i in 0..8 {
                if 0x100 + i < rom_data.len() {
                    println!("  0x{:04X}: 0x{:02X}", 0x100 + i, rom_data[0x100 + i]);
                }
            }
        }

        drop(mmu);
        self.init_lcd_registers()?;

        Ok(())
    }
    /// Initialize only essential registers for ROM execution
    fn init_lcd_registers(&mut self) -> Result<()> {
        let mut mmu = self.mmu.borrow_mut();

        // Set only essential joypad register (pulled high, no buttons pressed)
        // This is critical for Tetris - prevents soft reset
        mmu.write_byte(0xFF00, 0xFF).ok(); // Joypad register - all buttons released        // Enable LCD and background for basic display
        mmu.write_byte(0xFF40, 0x91).ok(); // LCDC: LCD=1, BG=1, BG tile map=0, BG tile data=1
        mmu.write_byte(0xFF47, 0xE4).ok(); // BGP: Background palette

        // Enable V-Blank interrupts by default
        mmu.write_byte(0xFFFF, 0x01).ok(); // IE: Enable V-Blank interrupt
        mmu.write_byte(0xFF0F, 0x00).ok(); // IF: Clear all interrupt flags

        // Let ROM set all LCD registers itself
        println!("Essential registers initialized - ROM has full control with V-Blank enabled");

        // Removed system init logging for performance

        Ok(())
    }
    pub fn step(&mut self) -> Result<()> {
        // At 1 FPS, execute many more instructions per frame to complete ROM init loops
        // ROM has multiple initialization phases that need to complete
        for _ in 0..5000 {
            let cpu_cycles = self.cpu.step()?;
            self.ppu.step(cpu_cycles)?;
        }

        Ok(())
    }
    pub fn reset(&mut self) -> Result<()> {
        let mut mmu = self.mmu.borrow_mut();
        mmu.reset();
        drop(mmu); // Release the borrow        // Reset CPU to initial state
        self.cpu.reset()?; // Reset PPU
        self.ppu.reset()?;

        // Removed reset logging for performance

        Ok(())
    }

    pub fn get_video_mut(&mut self) -> &mut dyn VideoInterface {
        self.ppu.get_video_mut()
    }

    pub fn update_joypad_state(&mut self, joypad: &dyn Joypad) -> Result<()> {
        self.mmu.borrow_mut().update_joypad_state(joypad);
        Ok(())
    }
    pub fn render(&mut self) -> Result<()> {
        // Log VRAM status every few seconds for debugging
        self.render_count += 1;
        if self.render_count % 300 == 0 {
            // Every ~5 seconds at 60 FPS
            if let Ok(mmu) = self.mmu.try_borrow() {
                let mut non_zero_count = 0;
                for i in 0x8000..0x9000 {
                    if mmu.read_byte(i).unwrap_or(0) != 0 {
                        non_zero_count += 1;
                    }
                }
                println!(
                    "VRAM Status: {} non-zero bytes in tile data area",
                    non_zero_count
                );

                // Check background map
                let mut bg_map_count = 0;
                for i in 0x9800..0x9C00 {
                    if mmu.read_byte(i).unwrap_or(0) != 0 {
                        bg_map_count += 1;
                    }
                }
                println!("Background Map: {} non-zero entries", bg_map_count);
            }
        }

        self.ppu.render()
    }
}
