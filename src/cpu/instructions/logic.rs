use super::{CPU, CYCLES_1, CYCLES_2};
use crate::cpu::flags::*;

impl CPU {
    // 左旋轉 A 暫存器（帶進位）
    fn rla(&mut self) -> u8 {
        let old_c = self.registers.get_c_flag();
        let new_c = (self.registers.a & 0x80) != 0;

        self.registers.a = (self.registers.a << 1) | (if old_c { 1 } else { 0 });

        self.registers.set_z_flag(false);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(new_c);

        CYCLES_1
    }

    // 左旋轉 A 暫存器（不帶進位）
    fn rlca(&mut self) -> u8 {
        let carry = (self.registers.a & 0x80) != 0;
        self.registers.a = (self.registers.a << 1) | (if carry { 1 } else { 0 });

        self.registers.set_z_flag(false);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(carry);

        CYCLES_1
    }

    // 右旋轉 A 暫存器（帶進位）
    fn rra(&mut self) -> u8 {
        let old_c = self.registers.get_c_flag();
        let new_c = (self.registers.a & 0x01) != 0;

        self.registers.a = (self.registers.a >> 1) | (if old_c { 0x80 } else { 0 });

        self.registers.set_z_flag(false);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(new_c);

        CYCLES_1
    }

    // 右旋轉 A 暫存器（不帶進位）
    fn rrca(&mut self) -> u8 {
        let carry = (self.registers.a & 0x01) != 0;
        self.registers.a = (self.registers.a >> 1) | (if carry { 0x80 } else { 0 });

        self.registers.set_z_flag(false);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(carry);

        CYCLES_1
    }

    /// AND r - Logical AND register with A
    fn and_r(&mut self, reg: u8) -> u8 {
        let value = match reg {
            0 => self.registers.b,
            1 => self.registers.c,
            2 => self.registers.d,
            3 => self.registers.e,
            4 => self.registers.h,
            5 => self.registers.l,
            6 => self.mmu.read_byte(self.registers.get_hl()),
            7 => self.registers.a,
            _ => unreachable!(),
        };

        self.registers.a &= value;

        self.registers.set_z_flag(self.registers.a == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(true);
        self.registers.set_c_flag(false);

        if reg == 6 { CYCLES_2 } else { CYCLES_1 }
    }

    /// AND n - Logical AND immediate with A
    fn and_n(&mut self) -> u8 {
        let value = self.mmu.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);

        self.registers.a &= value;

        self.registers.set_z_flag(self.registers.a == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(true);
        self.registers.set_c_flag(false);

        CYCLES_2
    }

    /// OR r - Logical OR register with A
    fn or_r(&mut self, reg: u8) -> u8 {
        let value = match reg {
            0 => self.registers.b,
            1 => self.registers.c,
            2 => self.registers.d,
            3 => self.registers.e,
            4 => self.registers.h,
            5 => self.registers.l,
            6 => self.mmu.read_byte(self.registers.get_hl()),
            7 => self.registers.a,
            _ => unreachable!(),
        };

        self.registers.a |= value;

        self.registers.set_z_flag(self.registers.a == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(false);

        if reg == 6 { CYCLES_2 } else { CYCLES_1 }
    }

    /// OR n - Logical OR immediate with A
    fn or_n(&mut self) -> u8 {
        let value = self.mmu.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);

        self.registers.a |= value;

        self.registers.set_z_flag(self.registers.a == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(false);

        CYCLES_2
    }

    /// XOR r - Logical XOR register with A
    fn xor_r(&mut self, reg: u8) -> u8 {
        let value = match reg {
            0 => self.registers.b,
            1 => self.registers.c,
            2 => self.registers.d,
            3 => self.registers.e,
            4 => self.registers.h,
            5 => self.registers.l,
            6 => self.mmu.read_byte(self.registers.get_hl()),
            7 => self.registers.a,
            _ => unreachable!(),
        };

        self.registers.a ^= value;

        self.registers.set_z_flag(self.registers.a == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(false);

        if reg == 6 { CYCLES_2 } else { CYCLES_1 }
    }

    /// XOR n - Logical XOR immediate with A
    fn xor_n(&mut self) -> u8 {
        let value = self.mmu.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);

        self.registers.a ^= value;

        self.registers.set_z_flag(self.registers.a == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(false);

        CYCLES_2
    }

    /// CP r - Compare register with A
    fn cp_r(&mut self, reg: u8) -> u8 {
        let value = match reg {
            0 => self.registers.b,
            1 => self.registers.c,
            2 => self.registers.d,
            3 => self.registers.e,
            4 => self.registers.h,
            5 => self.registers.l,
            6 => self.mmu.read_byte(self.registers.get_hl()),
            7 => self.registers.a,
            _ => unreachable!(),
        };

        let result = self.registers.a.wrapping_sub(value);
        let h_carry = (self.registers.a & 0x0F) < (value & 0x0F);

        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(true);
        self.registers.set_h_flag(h_carry);
        self.registers.set_c_flag(self.registers.a < value);

        if reg == 6 { CYCLES_2 } else { CYCLES_1 }
    }

    /// CP n - Compare immediate with A
    fn cp_n(&mut self) -> u8 {
        let value = self.mmu.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);

        let result = self.registers.a.wrapping_sub(value);
        let h_carry = (self.registers.a & 0x0F) < (value & 0x0F);

        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(true);
        self.registers.set_h_flag(h_carry);
        self.registers.set_c_flag(self.registers.a < value);

        CYCLES_2
    }

    pub fn execute_logic_instruction(&mut self, opcode: u8) -> u8 {
        match opcode {
            // AND A,r
            0xA0..=0xA7 => self.and_r(opcode & 0x07),

            // XOR A,r
            0xA8..=0xAF => self.xor_r(opcode & 0x07),

            // OR A,r
            0xB0..=0xB7 => self.or_r(opcode & 0x07),

            // CP A,r (Compare)
            0xB8..=0xBF => self.cp_r(opcode & 0x07),

            // Rotate instructions
            0x07 => self.rlca(), // RLCA
            0x17 => self.rla(),  // RLA
            0x0F => self.rrca(), // RRCA
            0x1F => self.rra(),  // RRA

            // Immediate operations
            0xE6 => self.and_n(), // AND n
            0xF6 => self.or_n(),  // OR n
            0xEE => self.xor_n(), // XOR n
            0xFE => self.cp_n(),  // CP n

            _ => {
                println!("未實作的邏輯指令: 0x{:02X}", opcode);
                CYCLES_1
            }
        }
    }
}
