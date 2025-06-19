use crate::cpu::interrupts::InterruptRegisters;
use crate::cpu::interrupts::TIMER_BIT;
use crate::error::Result;
use crate::mmu::MMU;
use std::cell::RefCell;
use std::rc::Rc;

const DIV: u16 = 0xFF04; // Divider Register
const TIMA: u16 = 0xFF05; // Timer Counter
const TMA: u16 = 0xFF06; // Timer Modulo
const TAC: u16 = 0xFF07; // Timer Control

// #[derive(Debug)] // ⚠️ 移除 Debug 衍生，MMU 不支援 Debug
pub struct Timer {
    div: u8,                                                      // DIV: Divider Register (8-bit)
    tima: u8,                                                     // TIMA: Timer Counter (8-bit)
    tma: u8,                                                      // TMA: Timer Modulo (8-bit)
    tac: u8,                                                      // TAC: Timer Control (8-bit)
    div_counter: u16,                                             // Internal div counter (16-bit)
    tima_counter: u16,                                            // Internal timer counter (16-bit)
    mmu: Option<Rc<RefCell<MMU>>>,                                // MMU reference
    interrupt_registers: Option<Rc<RefCell<InterruptRegisters>>>, // Interrupt registers reference
}

impl Timer {
    pub fn new() -> Self {
        Timer {
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            div_counter: 0,
            tima_counter: 0,
            mmu: None,
            interrupt_registers: None,
        }
    }

    pub fn init_mmu(&mut self, mmu: Rc<RefCell<MMU>>) {
        self.mmu = Some(mmu);
    }

    pub fn init_interrupt_registers(&mut self, interrupts: Rc<RefCell<InterruptRegisters>>) {
        self.interrupt_registers = Some(interrupts);
    }

    pub fn read_register(&self, addr: u16) -> Result<u8> {
        Ok(match addr {
            DIV => self.div,
            TIMA => self.tima,
            TMA => self.tma,
            TAC => self.tac,
            _ => 0xFF,
        })
    }

    pub fn write_register(&mut self, addr: u16, value: u8) -> Result<()> {
        match addr {
            DIV => self.div = 0, // Writing to DIV resets it to 0
            TIMA => self.tima = value,
            TMA => self.tma = value,
            TAC => self.tac = value & 0x07,
            _ => {}
        }
        Ok(())
    }
    /// Step the timer forward by one M-cycle (4 clock cycles)
    pub fn step(&mut self) -> Result<()> {
        // 更新 DIV
        self.div_counter = self.div_counter.wrapping_add(4);
        if self.div_counter >= 256 {
            self.div_counter -= 256;
            self.div = self.div.wrapping_add(1);
        }

        // 檢查計時器是否啟用 (TAC 的第 2 位)
        if self.tac & 0x04 != 0 {
            // 獲取計時器頻率 (TAC 的第 0-1 位)
            let freq = match self.tac & 0x03 {
                0 => 1024, // 4096Hz  (每 1024 個 CPU 時鐘週期)
                1 => 16,   // 262144Hz (每 16 個 CPU 時鐘週期)
                2 => 64,   // 65536Hz  (每 64 個 CPU 時鐘週期)
                3 => 256,  // 16384Hz  (每 256 個 CPU 時鐘週期)
                _ => 1024, // 預設使用最慢的頻率
            };

            // 更新計時器
            self.tima_counter = self.tima_counter.wrapping_add(4);
            if self.tima_counter >= freq {
                self.tima_counter -= freq;

                // 增加 TIMA，檢查溢位
                let (new_tima, overflowed) = self.tima.overflowing_add(1);
                if overflowed {
                    // TIMA 溢位時:
                    // 1. 設置為 TMA 的值
                    self.tima = self.tma;

                    // 2. 請求計時器中斷
                    if let Some(registers) = &self.interrupt_registers {
                        registers
                            .borrow_mut()
                            .request_interrupt(TIMER_BIT);
                    }
                } else {
                    self.tima = new_tima;
                }
            }
        }

        Ok(())
    }

    pub fn update(&mut self) -> Result<()> {
        self.step()?;
        Ok(())
    }

    pub fn reset(&mut self) {
        self.div = 0;
        self.tima = 0;
        self.tma = 0;
        self.tac = 0;
        self.div_counter = 0;
        self.tima_counter = 0;
    }
}
