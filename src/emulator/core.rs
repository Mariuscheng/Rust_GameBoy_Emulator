/*
================================================================================
Game Boy 模擬器 - 核心模擬器實現
================================================================================
整合所有硬體組件的主要模擬器結構體
================================================================================
*/
use crate::apu::APU;
use crate::cpu::CPU;
use crate::error::Result;
use crate::joypad::Joypad;
use crate::mmu::MMU;
use crate::timer::Timer;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Emulator {
    pub cpu: CPU,
    // pub ppu: PPU, // 已重構，請依新架構整合 display/lcd/background 等
    pub apu: APU,
    pub joypad: Joypad,
    pub timer: Timer,
    pub cycles: u64,
    pub frames: u64,
}

impl Emulator {
    pub fn new(_rom_data: &[u8]) -> Result<Self> {
        let mmu = Rc::new(RefCell::new(MMU::new()));
        let interrupt_registers = Rc::new(RefCell::new(
            crate::cpu::interrupts::InterruptRegisters::new(),
        ));
        // let logger = Rc::new(RefCell::new(crate::utils::Logger::new()));
        let cpu = CPU::new(mmu.clone(), interrupt_registers.clone());
        // CPU::load_rom 尚未實作，暫以 unimplemented!() 取代
        // cpu.load_rom(rom_data);
        let apu = APU::new();
        // Joypad::new() 無參數
        let joypad = Joypad::new();
        let timer = Timer::new();
        Ok(Self {
            cpu,
            // ppu,
            apu,
            joypad,
            timer,
            cycles: 0,
            frames: 0,
        })
    }
    pub fn update(&mut self) -> Result<()> {
        // CPU 執行一條指令
        let cycles = self.cpu.step()?;

        // 更新 PPU
        // self.ppu.update(cycles as u32);

        // 更新計時器
        self.timer.update()?;

        // 累計週期
        self.cycles += cycles as u64;

        // 每完成一幀就增加幀計數
        // if self.ppu.ly == 144 {
        //     self.frames += 1;
        // }

        Ok(())
    }
}
