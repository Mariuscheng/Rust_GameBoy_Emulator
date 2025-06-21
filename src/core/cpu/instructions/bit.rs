use super::register_utils::FlagOperations;
use crate::core::cpu::CPU;
use crate::core::cycles::{CyclesType, CYCLES_2, CYCLES_3};
use crate::error::{Error, InstructionError, RegTarget, Result};

pub fn dispatch(cpu: &mut CPU, opcode: u8) -> Result<CyclesType> {
    match opcode {
        // RLC r (旋轉左移，bit 7 進入 carry 和 bit 0)
        0x00..=0x07 => {
            let reg = opcode & 0x07;
            let target = RegTarget::from_bits(reg)?;
            cpu.rlc_r(target)
        }

        // RRC r (旋轉右移，bit 0 進入 carry 和 bit 7)
        0x08..=0x0F => {
            let reg = opcode & 0x07;
            let target = RegTarget::from_bits(reg)?;
            cpu.rrc_r(target)
        }

        // RL r (左移，carry 進入 bit 0)
        0x10..=0x17 => {
            let reg = opcode & 0x07;
            let target = RegTarget::from_bits(reg)?;
            cpu.rl_r(target)
        }

        // RR r (右移，carry 進入 bit 7)
        0x18..=0x1F => {
            let reg = opcode & 0x07;
            let target = RegTarget::from_bits(reg)?;
            cpu.rr_r(target)
        }

        // SLA r (算術左移)
        0x20..=0x27 => {
            let reg = opcode & 0x07;
            let target = RegTarget::from_bits(reg)?;
            cpu.sla_r(target)
        }

        // SRA r (算術右移)
        0x28..=0x2F => {
            let reg = opcode & 0x07;
            let target = RegTarget::from_bits(reg)?;
            cpu.sra_r(target)
        }

        // SWAP r (交換高低 4 位)
        0x30..=0x37 => {
            let reg = opcode & 0x07;
            let target = RegTarget::from_bits(reg)?;
            cpu.swap_r(target)
        }

        // SRL r (邏輯右移)
        0x38..=0x3F => {
            let reg = opcode & 0x07;
            let target = RegTarget::from_bits(reg)?;
            cpu.srl_r(target)
        }

        // BIT b, r
        0x40..=0x7F => {
            let bit = (opcode >> 3) & 0x07;
            let reg = opcode & 0x07;
            let target = RegTarget::from_bits(reg)?;
            cpu.bit_b_r(bit, target)
        }

        // RES b, r
        0x80..=0xBF => {
            let bit = (opcode >> 3) & 0x07;
            let reg = opcode & 0x07;
            let target = RegTarget::from_bits(reg)?;
            cpu.res_b_r(bit, target)
        } // SET b, r
        0xC0..=0xFF => {
            let bit = (opcode >> 3) & 0x07;
            let reg = opcode & 0x07;
            let target = RegTarget::from_bits(reg)?;
            cpu.set_b_r(bit, target)
        }
    }
}

impl CPU {
    fn get_reg_value(&mut self, reg: RegTarget) -> Result<u8> {
        Ok(match reg {
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
        })
    }

    fn set_reg_value(&mut self, reg: RegTarget, value: u8) -> Result<()> {
        match reg {
            RegTarget::A => self.registers.a = value,
            RegTarget::B => self.registers.b = value,
            RegTarget::C => self.registers.c = value,
            RegTarget::D => self.registers.d = value,
            RegTarget::E => self.registers.e = value,
            RegTarget::H => self.registers.h = value,
            RegTarget::L => self.registers.l = value,
            RegTarget::HL => {
                let addr = self.registers.get_hl();
                self.write_byte(addr, value)?;
            }
            _ => return Err(Error::Instruction(InstructionError::InvalidRegister(reg))),
        }
        Ok(())
    }

    pub fn bit_b_r(&mut self, bit: u8, reg: RegTarget) -> Result<CyclesType> {
        let value = self.get_reg_value(reg)?;

        let is_zero = (value & (1 << bit)) == 0;
        self.registers.set_zero(is_zero);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(true);

        Ok(if matches!(reg, RegTarget::HL) {
            CYCLES_3
        } else {
            CYCLES_2
        })
    }

    pub fn set_b_r(&mut self, bit: u8, reg: RegTarget) -> Result<CyclesType> {
        let value = self.get_reg_value(reg)?;
        let result = value | (1 << bit);
        self.set_reg_value(reg, result)?;

        Ok(if matches!(reg, RegTarget::HL) {
            CYCLES_3
        } else {
            CYCLES_2
        })
    }

    pub fn res_b_r(&mut self, bit: u8, reg: RegTarget) -> Result<CyclesType> {
        let value = self.get_reg_value(reg)?;
        let result = value & !(1 << bit);
        self.set_reg_value(reg, result)?;

        Ok(if matches!(reg, RegTarget::HL) {
            CYCLES_3
        } else {
            CYCLES_2
        })
    }

    pub fn rlc_r(&mut self, reg: RegTarget) -> Result<CyclesType> {
        let value = self.get_reg_value(reg)?;
        let carry = (value & 0x80) != 0;
        let result = (value << 1) | (if carry { 1 } else { 0 });

        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(carry);

        self.set_reg_value(reg, result)?;

        Ok(if matches!(reg, RegTarget::HL) {
            CYCLES_3
        } else {
            CYCLES_2
        })
    }

    pub fn rrc_r(&mut self, reg: RegTarget) -> Result<CyclesType> {
        let value = self.get_reg_value(reg)?;
        let carry = (value & 0x01) != 0;
        let result = if carry {
            0x80 | (value >> 1)
        } else {
            value >> 1
        };

        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(carry);

        self.set_reg_value(reg, result)?;

        Ok(if matches!(reg, RegTarget::HL) {
            CYCLES_3
        } else {
            CYCLES_2
        })
    }

    pub fn rl_r(&mut self, reg: RegTarget) -> Result<CyclesType> {
        let value = self.get_reg_value(reg)?;
        let old_carry = self.registers.get_carry();
        let new_carry = (value & 0x80) != 0;
        let result = (value << 1) | (if old_carry { 1 } else { 0 });

        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(new_carry);

        self.set_reg_value(reg, result)?;

        Ok(if matches!(reg, RegTarget::HL) {
            CYCLES_3
        } else {
            CYCLES_2
        })
    }

    pub fn rr_r(&mut self, reg: RegTarget) -> Result<CyclesType> {
        let value = self.get_reg_value(reg)?;
        let old_carry = self.registers.get_carry();
        let new_carry = (value & 0x01) != 0;
        let result = if old_carry {
            0x80 | (value >> 1)
        } else {
            value >> 1
        };

        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(new_carry);

        self.set_reg_value(reg, result)?;

        Ok(if matches!(reg, RegTarget::HL) {
            CYCLES_3
        } else {
            CYCLES_2
        })
    }

    pub fn sla_r(&mut self, reg: RegTarget) -> Result<CyclesType> {
        let value = self.get_reg_value(reg)?;
        let carry = (value & 0x80) != 0;
        let result = value << 1;

        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(carry);

        self.set_reg_value(reg, result)?;

        Ok(if matches!(reg, RegTarget::HL) {
            CYCLES_3
        } else {
            CYCLES_2
        })
    }

    pub fn sra_r(&mut self, reg: RegTarget) -> Result<CyclesType> {
        let value = self.get_reg_value(reg)?;
        let carry = (value & 0x01) != 0;
        let result = (value & 0x80) | (value >> 1);

        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(carry);

        self.set_reg_value(reg, result)?;

        Ok(if matches!(reg, RegTarget::HL) {
            CYCLES_3
        } else {
            CYCLES_2
        })
    }

    pub fn swap_r(&mut self, reg: RegTarget) -> Result<CyclesType> {
        let value = self.get_reg_value(reg)?;
        let result = ((value & 0x0F) << 4) | ((value & 0xF0) >> 4);

        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(false);

        self.set_reg_value(reg, result)?;

        Ok(if matches!(reg, RegTarget::HL) {
            CYCLES_3
        } else {
            CYCLES_2
        })
    }

    pub fn srl_r(&mut self, reg: RegTarget) -> Result<CyclesType> {
        let value = self.get_reg_value(reg)?;
        let carry = (value & 0x01) != 0;
        let result = value >> 1;

        self.registers.set_zero(result == 0);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(false);
        self.registers.set_carry(carry);

        self.set_reg_value(reg, result)?;

        Ok(if matches!(reg, RegTarget::HL) {
            CYCLES_3
        } else {
            CYCLES_2
        })
    }

    pub fn bit_r(&mut self, bit: u8, reg: RegTarget) -> Result<CyclesType> {
        let value = self.get_reg_value(reg)?;
        let result = (value & (1 << bit)) == 0;

        self.registers.set_zero(result);
        self.registers.set_subtract(false);
        self.registers.set_half_carry(true);

        Ok(if matches!(reg, RegTarget::HL) {
            CYCLES_3
        } else {
            CYCLES_2
        })
    }

    pub fn set_r(&mut self, bit: u8, reg: RegTarget) -> Result<CyclesType> {
        let value = self.get_reg_value(reg)?;
        let result = value | (1 << bit);
        self.set_reg_value(reg, result)?;

        Ok(if matches!(reg, RegTarget::HL) {
            CYCLES_3
        } else {
            CYCLES_2
        })
    }

    pub fn res_r(&mut self, bit: u8, reg: RegTarget) -> Result<CyclesType> {
        let value = self.get_reg_value(reg)?;
        let result = value & !(1 << bit);
        self.set_reg_value(reg, result)?;

        Ok(if matches!(reg, RegTarget::HL) {
            CYCLES_3
        } else {
            CYCLES_2
        })
    }
}

#[cfg(test)]
mod tests {
    use crate::core::cpu::CPU;
    use crate::core::mmu::MMU;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn test_cpu() -> CPU {
        let mmu = Rc::new(RefCell::new(MMU::new()));
        CPU::new(mmu)
    }

    #[test]
    fn test_rlc_b() {
        let mut cpu = test_cpu();
        cpu.registers_mut().b = 0b1000_0001;
        let _ = super::dispatch(&mut cpu, 0x00); // CB 00 = RLC B
        assert_eq!(cpu.registers().b, 0b0000_0011);
        assert!(cpu.registers().get_flag_c());
        assert!(!cpu.registers().get_flag_z());
    }

    #[test]
    fn test_bit_7_b() {
        let mut cpu = test_cpu();
        cpu.registers_mut().b = 0b1000_0000;
        let _ = super::dispatch(&mut cpu, 0x78); // CB 78 = BIT 7,B
        assert!(!cpu.registers().get_flag_z());
        cpu.registers_mut().b = 0b0000_0000;
        let _ = super::dispatch(&mut cpu, 0x78);
        assert!(cpu.registers().get_flag_z());
    }

    #[test]
    fn test_set_3_c() {
        let mut cpu = test_cpu();
        cpu.registers_mut().c = 0b0000_0000;
        let _ = super::dispatch(&mut cpu, 0xD9); // CB D9 = SET 3,C
        assert_eq!(cpu.registers().c, 0b0000_1000);
    }

    #[test]
    fn test_res_0_d() {
        let mut cpu = test_cpu();
        cpu.registers_mut().d = 0b0000_0001;
        let _ = super::dispatch(&mut cpu, 0x82); // CB 82 = RES 0,D
        assert_eq!(cpu.registers().d, 0b0000_0000);
    }
}
