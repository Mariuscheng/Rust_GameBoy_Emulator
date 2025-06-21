use crate::core::cpu::instructions::register_utils::FlagOperations;
use crate::core::cpu::CPU;
use crate::core::cycles::{CyclesType, CYCLES_1, CYCLES_2};
use crate::error::{Error, InstructionError, RegTarget, Result};

pub fn dispatch(cpu: &mut CPU, opcode: u8) -> Result<CyclesType> {
    match opcode {
        // AND r
        0xA0..=0xA7 => {
            let reg = opcode & 0x07;
            let target = RegTarget::from_bits(reg)?;
            cpu.and_a_r(target)
        }

        // AND n
        0xE6 => cpu.and_a_n(),

        // OR r
        0xB0..=0xB7 => {
            let reg = opcode & 0x07;
            let target = RegTarget::from_bits(reg)?;
            cpu.or_a_r(target)
        }

        // OR n
        0xF6 => cpu.or_a_n(),

        // XOR r
        0xA8..=0xAF => {
            let reg = opcode & 0x07;
            let target = RegTarget::from_bits(reg)?;
            cpu.xor_a_r(target)
        }

        // XOR n
        0xEE => cpu.xor_a_n(),

        // CP r
        0xB8..=0xBF => {
            let reg = opcode & 0x07;
            let target = RegTarget::from_bits(reg)?;
            cpu.cp_a_r(target)
        }

        // CP n
        0xFE => cpu.cp_a_n(),

        _ => Err(Error::Instruction(InstructionError::InvalidOpcode(opcode))),
    }
}

impl CPU {
    pub fn and_a_r(&mut self, reg: RegTarget) -> Result<CyclesType> {
        let value = match reg {
            RegTarget::A => self.registers.a,
            RegTarget::B => self.registers.b,
            RegTarget::C => self.registers.c,
            RegTarget::D => self.registers.d,
            RegTarget::E => self.registers.e,
            RegTarget::H => self.registers.h,
            RegTarget::L => self.registers.l,
            RegTarget::HL => {
                let addr = self.registers.get_hl();
                self.read_byte(addr)?
            }
            _ => return Err(Error::Instruction(InstructionError::InvalidRegister(reg))),
        };

        self.registers.a &= value;
        self.update_logic_flags(self.registers.a, true);
        Ok(if matches!(reg, RegTarget::HL) {
            CYCLES_2
        } else {
            CYCLES_1
        })
    }

    pub fn and_a_n(&mut self) -> Result<CyclesType> {
        let value = self.fetch_byte()?;
        self.registers.a &= value;
        self.update_logic_flags(self.registers.a, true);
        Ok(CYCLES_2)
    }

    pub fn or_a_r(&mut self, reg: RegTarget) -> Result<CyclesType> {
        let value = match reg {
            RegTarget::A => self.registers.a,
            RegTarget::B => self.registers.b,
            RegTarget::C => self.registers.c,
            RegTarget::D => self.registers.d,
            RegTarget::E => self.registers.e,
            RegTarget::H => self.registers.h,
            RegTarget::L => self.registers.l,
            RegTarget::HL => {
                let addr = self.registers.get_hl();
                self.read_byte(addr)?
            }
            _ => return Err(Error::Instruction(InstructionError::InvalidRegister(reg))),
        };

        self.registers.a |= value;
        self.update_logic_flags(self.registers.a, false);
        Ok(if matches!(reg, RegTarget::HL) {
            CYCLES_2
        } else {
            CYCLES_1
        })
    }

    pub fn or_a_n(&mut self) -> Result<CyclesType> {
        let value = self.fetch_byte()?;
        self.registers.a |= value;
        self.update_logic_flags(self.registers.a, false);
        Ok(CYCLES_2)
    }

    pub fn xor_a_r(&mut self, reg: RegTarget) -> Result<CyclesType> {
        let value = match reg {
            RegTarget::A => self.registers.a,
            RegTarget::B => self.registers.b,
            RegTarget::C => self.registers.c,
            RegTarget::D => self.registers.d,
            RegTarget::E => self.registers.e,
            RegTarget::H => self.registers.h,
            RegTarget::L => self.registers.l,
            RegTarget::HL => {
                let addr = self.registers.get_hl();
                self.read_byte(addr)?
            }
            _ => return Err(Error::Instruction(InstructionError::InvalidRegister(reg))),
        };

        self.registers.a ^= value;
        self.update_logic_flags(self.registers.a, false);
        Ok(if matches!(reg, RegTarget::HL) {
            CYCLES_2
        } else {
            CYCLES_1
        })
    }

    pub fn xor_a_n(&mut self) -> Result<CyclesType> {
        let value = self.fetch_byte()?;
        self.registers.a ^= value;
        self.update_logic_flags(self.registers.a, false);
        Ok(CYCLES_2)
    }

    pub fn cp_a_r(&mut self, reg: RegTarget) -> Result<CyclesType> {
        let value = match reg {
            RegTarget::A => self.registers.a,
            RegTarget::B => self.registers.b,
            RegTarget::C => self.registers.c,
            RegTarget::D => self.registers.d,
            RegTarget::E => self.registers.e,
            RegTarget::H => self.registers.h,
            RegTarget::L => self.registers.l,
            RegTarget::HL => {
                let addr = self.registers.get_hl();
                self.read_byte(addr)?
            }
            _ => return Err(Error::Instruction(InstructionError::InvalidRegister(reg))),
        };

        self.cp_a(value);
        Ok(if matches!(reg, RegTarget::HL) {
            CYCLES_2
        } else {
            CYCLES_1
        })
    }

    pub fn cp_a_n(&mut self) -> Result<CyclesType> {
        let value = self.fetch_byte()?;
        self.cp_a(value);
        Ok(CYCLES_2)
    }

    fn cp_a(&mut self, value: u8) {
        let (result, borrow) = self.registers.a.overflowing_sub(value);
        let half_borrow = (self.registers.a & 0xF) < (value & 0xF);

        self.registers.set_zero(result == 0);
        self.registers.set_subtract(true);
        self.registers.set_half_carry(half_borrow);
        self.registers.set_carry(borrow);
    }

    fn update_logic_flags(&mut self, result: u8, half_carry: bool) {
        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(half_carry);
        self.registers.set_carry(false);
    }
}
