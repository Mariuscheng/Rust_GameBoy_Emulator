use crate::core::cpu::instructions::register_utils::FlagOperations;
use crate::core::cpu::CPU;
use crate::core::cycles::{CyclesType, CYCLES_1, CYCLES_2, CYCLES_3, CYCLES_4};
use crate::error::{Error, InstructionError, Result};

pub fn dispatch(cpu: &mut CPU, opcode: u8) -> Result<CyclesType> {
    match opcode {
        // HALT
        0x76 => cpu.halt(),

        // STOP
        0x10 => cpu.stop(),

        // EI
        0xFB => cpu.enable_interrupts(),

        // DI
        0xF3 => cpu.disable_interrupts(), // CALL nn
        0xCD => cpu.call(),

        // JP nn
        0xC3 => cpu.jump(),

        // CALL cc, nn
        0xC4 | 0xCC | 0xD4 | 0xDC => {
            let condition = (opcode >> 3) & 0x03;
            cpu.call_conditional(condition)
        }

        // RET
        0xC9 => cpu.ret(),

        // RET cc
        0xC0 | 0xC8 | 0xD0 | 0xD8 => {
            let condition = (opcode >> 3) & 0x03;
            cpu.ret_conditional(condition)
        }

        // RETI
        0xD9 => cpu.reti(),

        // RST n
        0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => {
            let address = (opcode & 0x38) as u16;
            cpu.rst(address)
        } // NOP
        0x00 => Ok(CYCLES_1),

        // POP rr
        0xC1 => cpu.pop_bc(),
        0xD1 => cpu.pop_de(),
        0xE1 => cpu.pop_hl(),
        0xF1 => cpu.pop_af(),

        // PUSH rr
        0xC5 => cpu.push_bc(),
        0xD5 => cpu.push_de(),
        0xE5 => cpu.push_hl(),
        0xF5 => cpu.push_af(),

        _ => Err(Error::Instruction(InstructionError::InvalidOpcode(opcode))),
    }
}

impl CPU {
    pub fn halt(&mut self) -> Result<CyclesType> {
        self.halted = true;
        Ok(CYCLES_1)
    }

    pub fn stop(&mut self) -> Result<CyclesType> {
        self.halted = true;
        Ok(CYCLES_1)
    }

    pub fn enable_interrupts(&mut self) -> Result<CyclesType> {
        self.ime = true;
        Ok(CYCLES_1)
    }

    pub fn disable_interrupts(&mut self) -> Result<CyclesType> {
        self.ime = false;
        Ok(CYCLES_1)
    }

    pub fn call(&mut self) -> Result<CyclesType> {
        let address = self.fetch_word()?;
        self.push_word(self.registers.pc)?;
        self.registers.pc = address;
        Ok(CYCLES_3)
    }

    pub fn call_conditional(&mut self, condition: u8) -> Result<CyclesType> {
        let address = self.fetch_word()?;
        let jump = match condition {
            0 => !self.registers.get_zero(),  // NZ
            1 => self.registers.get_zero(),   // Z
            2 => !self.registers.get_carry(), // NC
            3 => self.registers.get_carry(),  // C
            _ => {
                return Err(Error::Instruction(InstructionError::Custom(
                    "無效的條件碼".to_string(),
                )))
            }
        };

        if jump {
            self.push_word(self.registers.pc)?;
            self.registers.pc = address;
            Ok(CYCLES_4)
        } else {
            Ok(CYCLES_3)
        }
    }

    pub fn ret(&mut self) -> Result<CyclesType> {
        self.registers.pc = self.pop_word()?;
        Ok(CYCLES_2)
    }

    pub fn ret_conditional(&mut self, condition: u8) -> Result<CyclesType> {
        let jump = match condition {
            0 => !self.registers.get_zero(),  // NZ
            1 => self.registers.get_zero(),   // Z
            2 => !self.registers.get_carry(), // NC
            3 => self.registers.get_carry(),  // C
            _ => {
                return Err(Error::Instruction(InstructionError::Custom(
                    "無效的條件碼".to_string(),
                )))
            }
        };

        if jump {
            self.registers.pc = self.pop_word()?;
            Ok(CYCLES_3)
        } else {
            Ok(CYCLES_2)
        }
    }

    pub fn reti(&mut self) -> Result<CyclesType> {
        self.registers.pc = self.pop_word()?;
        self.ime = true;
        Ok(CYCLES_2)
    }

    pub fn rst(&mut self, address: u16) -> Result<CyclesType> {
        self.push_word(self.registers.pc)?;
        self.registers.pc = address;
        Ok(CYCLES_2)
    }

    // 堆疊操作 - POP
    pub fn pop_bc(&mut self) -> Result<CyclesType> {
        let value = self.pop_word()?;
        self.registers.set_bc(value);
        Ok(CYCLES_3)
    }

    pub fn pop_de(&mut self) -> Result<CyclesType> {
        let value = self.pop_word()?;
        self.registers.set_de(value);
        Ok(CYCLES_3)
    }

    pub fn pop_hl(&mut self) -> Result<CyclesType> {
        let value = self.pop_word()?;
        self.registers.set_hl(value);
        Ok(CYCLES_3)
    }

    pub fn pop_af(&mut self) -> Result<CyclesType> {
        let value = self.pop_word()?;
        self.registers.set_af(value);
        Ok(CYCLES_3)
    }

    // 堆疊操作 - PUSH
    pub fn push_bc(&mut self) -> Result<CyclesType> {
        self.push_word(self.registers.get_bc())?;
        Ok(CYCLES_4)
    }

    pub fn push_de(&mut self) -> Result<CyclesType> {
        self.push_word(self.registers.get_de())?;
        Ok(CYCLES_4)
    }

    pub fn push_hl(&mut self) -> Result<CyclesType> {
        self.push_word(self.registers.get_hl())?;
        Ok(CYCLES_4)
    }

    pub fn push_af(&mut self) -> Result<CyclesType> {
        self.push_word(self.registers.get_af())?;
        Ok(CYCLES_4)
    }
}
