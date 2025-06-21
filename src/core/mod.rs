// Game Boy Emulator Core Components
use std::cell::RefCell;
use std::rc::Rc;

use crate::error::Result;
use crate::interface::{audio::AudioInterface, input::joypad::Joypad, video::VideoInterface};

pub mod audio;
pub mod cpu;
pub mod cycles;
pub mod mmu;
pub mod ppu;
pub mod timer;

use audio::apu::APU;
use cpu::CPU;
use mmu::MMU;
use ppu::PPU;
use timer::Timer;

#[derive(Debug)]
pub struct Core {
    pub cpu: CPU,
    pub mmu: Rc<RefCell<MMU>>,
    pub ppu: PPU,
    pub apu: APU,
    pub timer: Timer,
    cycles: u32,
}

impl Core {
    /// Create a new emulator core instance
    pub fn new(
        video: Box<dyn VideoInterface>,
        audio: Option<Box<dyn AudioInterface>>,
    ) -> Result<Self> {
        let mmu = Rc::new(RefCell::new(MMU::new()));

        Ok(Self {
            cpu: CPU::new(mmu.clone()),
            mmu: mmu.clone(),
            ppu: PPU::new(mmu.clone(), video),
            apu: APU::new(audio),
            timer: Timer::new(),
            cycles: 0,
        })
    }

    /// Load ROM data
    pub fn load_rom(&mut self, rom_data: Vec<u8>) -> Result<()> {
        self.mmu.borrow_mut().load_rom(&rom_data)
    }
    /// Execute emulator cycle
    pub fn step(&mut self) -> Result<()> {
        // Execute one CPU instruction
        let cycles = self.cpu.step()?;

        // PPU step
        self.ppu.step(cycles)?;

        Ok(())
    }
    /// Reset all component states
    pub fn reset(&mut self) -> Result<()> {
        self.cpu.reset()?;
        self.mmu.borrow_mut().reset();
        self.ppu.reset()?;
        self.apu.reset()?;
        self.timer.reset()?;
        self.cycles = 0;
        Ok(())
    }

    /// Get mutable reference to video interface
    pub fn get_video_mut(&mut self) -> &mut dyn VideoInterface {
        self.ppu.get_video_mut()
    }

    /// Update input state
    pub fn update_joypad_state(&mut self, joypad: &dyn Joypad) -> Result<()> {
        self.mmu.borrow_mut().update_joypad_state(joypad);
        Ok(())
    }

    /// Render display
    pub fn render(&mut self) -> Result<()> {
        self.ppu.render()
    }
}
