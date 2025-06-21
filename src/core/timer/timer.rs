use crate::error::{Error, Result};

#[derive(Clone, Debug)]
pub struct Timer {
    pub div: u8,
    pub tima: u8,
    pub tma: u8,
    pub tac: u8,
    cycles: u32,
}

impl Timer {
    pub fn new() -> Self {
        Self {
            div: 0,
            tima: 0,
            tma: 0,
            tac: 0,
            cycles: 0,
        }
    }

    pub fn read_byte(&self, addr: u16) -> Result<u8> {
        match addr {
            0xFF04 => Ok(self.div),
            0xFF05 => Ok(self.tima),
            0xFF06 => Ok(self.tma),
            0xFF07 => Ok(self.tac),
            _ => Err(Error::Hardware(crate::error::HardwareError::Timer(
                format!("Invalid timer register address: {:#04X}", addr),
            ))),
        }
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) -> Result<()> {
        match addr {
            0xFF04 => self.div = 0,
            0xFF05 => self.tima = value,
            0xFF06 => self.tma = value,
            0xFF07 => self.tac = value & 0x07,
            _ => {
                return Err(Error::Hardware(crate::error::HardwareError::Timer(
                    format!("Invalid timer register address: {:#04X}", addr),
                )))
            }
        }
        Ok(())
    }

    pub fn step(&mut self, cycles: u32) -> Result<()> {
        self.cycles += cycles;

        // DIV 寄存器每 256 個時脈週期遞增
        self.div = self.div.wrapping_add((self.cycles / 256) as u8);
        self.cycles %= 256;

        // 只有在 TAC 的最高位設定為 1 時才會更新 TIMA
        if self.tac & 0x04 != 0 {
            let ticks = match self.tac & 0x03 {
                0 => 1024, // 4096 Hz
                1 => 16,   // 262144 Hz
                2 => 64,   // 65536 Hz
                3 => 256,  // 16384 Hz
                _ => unreachable!(),
            };

            while self.cycles >= ticks {
                self.tima = self.tima.wrapping_add(1);
                if self.tima == 0 {
                    self.tima = self.tma;
                }
                self.cycles -= ticks;
            }
        }

        Ok(())
    }

    pub fn reset(&mut self) -> Result<()> {
        self.div = 0;
        self.tima = 0;
        self.tma = 0;
        self.tac = 0;
        self.cycles = 0;
        Ok(())
    }

    pub fn update(&mut self, cycles: u32) -> Result<()> {
        self.step(cycles)
    }
}
