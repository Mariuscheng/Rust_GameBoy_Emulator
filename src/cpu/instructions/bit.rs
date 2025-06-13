use super::{CPU, CYCLES_2, CYCLES_4};
use crate::cpu::flags::*;

impl CPU {
    pub(crate) fn execute_cb_instruction(&mut self, opcode: u8) -> u8 {
        match opcode {
            // RLC r (Rotate Left)
            0x00..=0x07 => self.rlc_r(opcode & 0x07),

            // RRC r (Rotate Right)
            0x08..=0x0F => self.rrc_r(opcode & 0x07),

            // RL r (Rotate Left through Carry)
            0x10..=0x17 => self.rl_r(opcode & 0x07),

            // RR r (Rotate Right through Carry)
            0x18..=0x1F => self.rr_r(opcode & 0x07),

            // SLA r (Shift Left Arithmetic)
            0x20..=0x27 => self.sla_r(opcode & 0x07),

            // SRA r (Shift Right Arithmetic)
            0x28..=0x2F => self.sra_r(opcode & 0x07),

            // SWAP r
            0x30..=0x37 => self.swap_r(opcode & 0x07),

            // SRL r (Shift Right Logic)
            0x38..=0x3F => self.srl_r(opcode & 0x07),

            // BIT b,r
            0x40..=0x7F => {
                let bit = (opcode - 0x40) >> 3;
                let reg = opcode & 0x07;
                self.bit_b_r(bit, reg)
            }

            // RES b,r
            0x80..=0xBF => {
                let bit = (opcode - 0x80) >> 3;
                let reg = opcode & 0x07;
                self.res_b_r(bit, reg)
            }

            // SET b,r
            0xC0..=0xFF => {
                let bit = (opcode - 0xC0) >> 3;
                let reg = opcode & 0x07;
                self.set_b_r(bit, reg)
            }
        }
    }

    // 輔助函數：根據寄存器索引獲取值
    fn get_reg_value(&self, reg: u8) -> u8 {
        match reg {
            0 => self.registers.b,
            1 => self.registers.c,
            2 => self.registers.d,
            3 => self.registers.e,
            4 => self.registers.h,
            5 => self.registers.l,
            6 => self.mmu.read_byte(self.registers.get_hl()),
            7 => self.registers.a,
            _ => unreachable!(),
        }
    }

    // 輔助函數：根據寄存器索引設置值
    fn set_reg_value(&mut self, reg: u8, value: u8) {
        match reg {
            0 => self.registers.b = value,
            1 => self.registers.c = value,
            2 => self.registers.d = value,
            3 => self.registers.e = value,
            4 => self.registers.h = value,
            5 => self.registers.l = value,
            6 => self.mmu.write_byte(self.registers.get_hl(), value),
            7 => self.registers.a = value,
            _ => unreachable!(),
        }
    }

    // CB前綴指令實作
    fn rlc_r(&mut self, reg: u8) -> u8 {
        let value = self.get_reg_value(reg);
        let carry = (value & 0x80) != 0;
        let result = (value << 1) | (if carry { 1 } else { 0 });

        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(carry);

        self.set_reg_value(reg, result);
        if reg == 6 { CYCLES_4 } else { CYCLES_2 }
    }

    fn rrc_r(&mut self, reg: u8) -> u8 {
        let value = self.get_reg_value(reg);
        let carry = (value & 0x01) != 0;
        let result = (value >> 1) | (if carry { 0x80 } else { 0 });

        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(carry);

        self.set_reg_value(reg, result);
        if reg == 6 { CYCLES_4 } else { CYCLES_2 }
    }

    fn rl_r(&mut self, reg: u8) -> u8 {
        let value = self.get_reg_value(reg);
        let old_carry = self.registers.get_c_flag();
        let new_carry = (value & 0x80) != 0;
        let result = (value << 1) | (if old_carry { 1 } else { 0 });

        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(new_carry);

        self.set_reg_value(reg, result);
        if reg == 6 { CYCLES_4 } else { CYCLES_2 }
    }

    fn rr_r(&mut self, reg: u8) -> u8 {
        let value = self.get_reg_value(reg);
        let old_carry = self.registers.get_c_flag();
        let new_carry = (value & 0x01) != 0;
        let result = (value >> 1) | (if old_carry { 0x80 } else { 0 });

        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(new_carry);

        self.set_reg_value(reg, result);
        if reg == 6 { CYCLES_4 } else { CYCLES_2 }
    }

    fn sla_r(&mut self, reg: u8) -> u8 {
        let value = self.get_reg_value(reg);
        let carry = (value & 0x80) != 0;
        let result = value << 1;

        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(carry);

        self.set_reg_value(reg, result);
        if reg == 6 { CYCLES_4 } else { CYCLES_2 }
    }

    fn sra_r(&mut self, reg: u8) -> u8 {
        let value = self.get_reg_value(reg);
        let carry = (value & 0x01) != 0;
        let result = (value >> 1) | (value & 0x80);

        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(carry);

        self.set_reg_value(reg, result);
        if reg == 6 { CYCLES_4 } else { CYCLES_2 }
    }

    fn swap_r(&mut self, reg: u8) -> u8 {
        let value = self.get_reg_value(reg);
        let result = ((value & 0x0F) << 4) | ((value & 0xF0) >> 4);

        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(false);

        self.set_reg_value(reg, result);
        if reg == 6 { CYCLES_4 } else { CYCLES_2 }
    }

    fn srl_r(&mut self, reg: u8) -> u8 {
        let value = self.get_reg_value(reg);
        let carry = (value & 0x01) != 0;
        let result = value >> 1;

        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(carry);

        self.set_reg_value(reg, result);
        if reg == 6 { CYCLES_4 } else { CYCLES_2 }
    }

    fn bit_b_r(&mut self, bit: u8, reg: u8) -> u8 {
        let value = self.get_reg_value(reg);
        let bit_value = (value >> bit) & 0x01;

        self.registers.set_z_flag(bit_value == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(true);

        if reg == 6 { CYCLES_4 } else { CYCLES_2 }
    }

    fn res_b_r(&mut self, bit: u8, reg: u8) -> u8 {
        let value = self.get_reg_value(reg);
        let result = value & !(1 << bit);

        self.set_reg_value(reg, result);
        if reg == 6 { CYCLES_4 } else { CYCLES_2 }
    }

    fn set_b_r(&mut self, bit: u8, reg: u8) -> u8 {
        let value = self.get_reg_value(reg);
        let result = value | (1 << bit);

        self.set_reg_value(reg, result);
        if reg == 6 { CYCLES_4 } else { CYCLES_2 }
    }

    /// BIT b,r - Test bit b in register r
    pub fn execute_bit_instruction(&mut self, bit: u8, reg: u8) -> u8 {
        let value = match reg {
            0 => self.registers.b,
            1 => self.registers.c,
            2 => self.registers.d,
            3 => self.registers.e,
            4 => self.registers.h,
            5 => self.registers.l,
            6 => self.mmu.read_byte(self.registers.get_hl()),
            7 => self.registers.a,
            _ => panic!("Invalid register index for BIT instruction"),
        };

        let mask = 1 << bit;
        let result = value & mask == 0;

        self.registers.set_z_flag(result);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(true);

        CYCLES_2
    }

    /// RES b,r - Reset bit b in register r
    pub fn execute_res_instruction(&mut self, bit: u8, reg: u8) -> u8 {
        let mask = !(1 << bit);
        match reg {
            0 => self.registers.b &= mask,
            1 => self.registers.c &= mask,
            2 => self.registers.d &= mask,
            3 => self.registers.e &= mask,
            4 => self.registers.h &= mask,
            5 => self.registers.l &= mask,
            6 => {
                let addr = self.registers.get_hl();
                let value = self.mmu.read_byte(addr) & mask;
                self.mmu.write_byte(addr, value);
            }
            7 => self.registers.a &= mask,
            _ => panic!("Invalid register index for RES instruction"),
        }

        CYCLES_2
    }

    /// SET b,r - Set bit b in register r
    pub fn execute_set_instruction(&mut self, bit: u8, reg: u8) -> u8 {
        let mask = 1 << bit;
        match reg {
            0 => self.registers.b |= mask,
            1 => self.registers.c |= mask,
            2 => self.registers.d |= mask,
            3 => self.registers.e |= mask,
            4 => self.registers.h |= mask,
            5 => self.registers.l |= mask,
            6 => {
                let addr = self.registers.get_hl();
                let value = self.mmu.read_byte(addr) | mask;
                self.mmu.write_byte(addr, value);
            }
            7 => self.registers.a |= mask,
            _ => panic!("Invalid register index for SET instruction"),
        }

        CYCLES_2
    }
}
