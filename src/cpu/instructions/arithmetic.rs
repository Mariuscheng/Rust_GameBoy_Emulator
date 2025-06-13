use super::{CPU, CYCLES_1, CYCLES_2, CYCLES_3, CYCLES_4};

impl CPU {
    fn add_a_r(&mut self, reg: u8) -> u8 {
        let a = self.registers.a;
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

        let result = a.wrapping_add(value);

        // 設置標誌位
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers
            .set_h_flag((a & 0x0F) + (value & 0x0F) > 0x0F);
        self.registers
            .set_c_flag((a as u16) + (value as u16) > 0xFF);

        self.registers.a = result;

        if reg == 6 { CYCLES_2 } else { CYCLES_1 }
    }

    fn adc_a_r(&mut self, reg: u8) -> u8 {
        let a = self.registers.a;
        let carry = if self.registers.get_c_flag() { 1 } else { 0 };
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

        let result = a.wrapping_add(value).wrapping_add(carry);
        let half_carry = (a & 0x0F) + (value & 0x0F) + carry > 0x0F;
        let carry_out = (a as u16) + (value as u16) + (carry as u16) > 0xFF;

        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(half_carry);
        self.registers.set_c_flag(carry_out);

        self.registers.a = result;

        if reg == 6 { CYCLES_2 } else { CYCLES_1 }
    }

    fn sub_a_r(&mut self, reg: u8) -> u8 {
        let a = self.registers.a;
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

        let result = a.wrapping_sub(value);

        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(true);
        self.registers.set_h_flag((a & 0x0F) < (value & 0x0F));
        self.registers.set_c_flag(a < value);

        self.registers.a = result;

        if reg == 6 { CYCLES_2 } else { CYCLES_1 }
    }

    fn sbc_a_r(&mut self, reg: u8) -> u8 {
        let a = self.registers.a;
        let carry = if self.registers.get_c_flag() { 1 } else { 0 };
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

        let result = a.wrapping_sub(value).wrapping_sub(carry);
        let half_carry = (a & 0x0F) < ((value & 0x0F) + carry);
        let carry_out = (a as u16) < (value as u16) + (carry as u16);

        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(true);
        self.registers.set_h_flag(half_carry);
        self.registers.set_c_flag(carry_out);

        self.registers.a = result;

        if reg == 6 { CYCLES_2 } else { CYCLES_1 }
    }

    /// INC r - Increment register r
    pub fn inc_r(&mut self, reg: u8) -> u8 {
        let (result, is_memory) = match reg {
            0 => (self.registers.b.wrapping_add(1), false),
            1 => (self.registers.c.wrapping_add(1), false),
            2 => (self.registers.d.wrapping_add(1), false),
            3 => (self.registers.e.wrapping_add(1), false),
            4 => (self.registers.h.wrapping_add(1), false),
            5 => (self.registers.l.wrapping_add(1), false),
            6 => {
                let addr = self.registers.get_hl();
                let value = self.mmu.read_byte(addr);
                let result = value.wrapping_add(1);
                self.mmu.write_byte(addr, result);
                (result, true)
            }
            7 => (self.registers.a.wrapping_add(1), false),
            _ => unreachable!(),
        };

        // 設置標誌位
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag((result & 0x0F) == 0);

        // 更新寄存器值（如果不是記憶體操作）
        if !is_memory {
            match reg {
                0 => self.registers.b = result,
                1 => self.registers.c = result,
                2 => self.registers.d = result,
                3 => self.registers.e = result,
                4 => self.registers.h = result,
                5 => self.registers.l = result,
                7 => self.registers.a = result,
                _ => unreachable!(),
            }
        }

        if is_memory { CYCLES_3 } else { CYCLES_1 }
    }

    /// DEC r - Decrement register r
    pub fn dec_r(&mut self, reg: u8) -> u8 {
        let (result, is_memory) = match reg {
            0 => (self.registers.b.wrapping_sub(1), false),
            1 => (self.registers.c.wrapping_sub(1), false),
            2 => (self.registers.d.wrapping_sub(1), false),
            3 => (self.registers.e.wrapping_sub(1), false),
            4 => (self.registers.h.wrapping_sub(1), false),
            5 => (self.registers.l.wrapping_sub(1), false),
            6 => {
                let addr = self.registers.get_hl();
                let value = self.mmu.read_byte(addr);
                let result = value.wrapping_sub(1);
                self.mmu.write_byte(addr, result);
                (result, true)
            }
            7 => (self.registers.a.wrapping_sub(1), false),
            _ => unreachable!(),
        };

        // 設置標誌位
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(true);
        self.registers.set_h_flag((result & 0x0F) == 0x0F);

        // 更新寄存器值（如果不是記憶體操作）
        if !is_memory {
            match reg {
                0 => self.registers.b = result,
                1 => self.registers.c = result,
                2 => self.registers.d = result,
                3 => self.registers.e = result,
                4 => self.registers.h = result,
                5 => self.registers.l = result,
                7 => self.registers.a = result,
                _ => unreachable!(),
            }
        }

        if is_memory { CYCLES_3 } else { CYCLES_1 }
    }

    /// INC rr - Increment 16-bit register
    pub fn inc_rr(&mut self, reg_pair: u8) -> u8 {
        match reg_pair {
            0 => {
                let bc = self.registers.get_bc().wrapping_add(1);
                self.registers.set_bc(bc);
            }
            1 => {
                let de = self.registers.get_de().wrapping_add(1);
                self.registers.set_de(de);
            }
            2 => {
                let hl = self.registers.get_hl().wrapping_add(1);
                self.registers.set_hl(hl);
            }
            3 => {
                self.registers.sp = self.registers.sp.wrapping_add(1);
            }
            _ => unreachable!(),
        }
        CYCLES_2
    }

    /// DEC rr - Decrement 16-bit register
    pub fn dec_rr(&mut self, reg_pair: u8) -> u8 {
        match reg_pair {
            0 => {
                let bc = self.registers.get_bc().wrapping_sub(1);
                self.registers.set_bc(bc);
            }
            1 => {
                let de = self.registers.get_de().wrapping_sub(1);
                self.registers.set_de(de);
            }
            2 => {
                let hl = self.registers.get_hl().wrapping_sub(1);
                self.registers.set_hl(hl);
            }
            3 => {
                self.registers.sp = self.registers.sp.wrapping_sub(1);
            }
            _ => unreachable!(),
        }
        CYCLES_2
    }

    /// ADD HL,rr - Add 16-bit register to HL
    pub fn add_hl_rr(&mut self, reg_pair: u8) -> u8 {
        let hl = self.registers.get_hl();
        let value = match reg_pair {
            0 => self.registers.get_bc(),
            1 => self.registers.get_de(),
            2 => self.registers.get_hl(),
            3 => self.registers.sp,
            _ => unreachable!(),
        };

        let result = hl.wrapping_add(value);

        self.registers.set_n_flag(false);
        self.registers
            .set_h_flag((hl & 0x0FFF) + (value & 0x0FFF) > 0x0FFF);
        self.registers
            .set_c_flag((hl as u32) + (value as u32) > 0xFFFF);

        self.registers.set_hl(result);

        CYCLES_2
    }

    /// ADD SP,n - Add signed immediate to SP
    pub fn add_sp_n(&mut self) -> u8 {
        let n = self.mmu.read_byte(self.registers.pc) as i8 as i16 as u16;
        self.registers.pc = self.registers.pc.wrapping_add(1);

        let sp = self.registers.sp;
        let result = sp.wrapping_add(n);

        self.registers.set_z_flag(false);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag((sp & 0x0F) + (n & 0x0F) > 0x0F);
        self.registers.set_c_flag((sp & 0xFF) + (n & 0xFF) > 0xFF);

        self.registers.sp = result;

        CYCLES_4
    }


    /// 執行算術指令
    pub fn execute_arithmetic_instruction(&mut self, opcode: u8) -> u8 {
        match opcode {
            // INC r
            0x04 => self.inc_r(0), // INC B
            0x0C => self.inc_r(1), // INC C
            0x14 => self.inc_r(2), // INC D
            0x1C => self.inc_r(3), // INC E
            0x24 => self.inc_r(4), // INC H
            0x2C => self.inc_r(5), // INC L
            0x34 => self.inc_r(6), // INC (HL)
            0x3C => self.inc_r(7), // INC A

            // DEC r
            0x05 => self.dec_r(0), // DEC B
            0x0D => self.dec_r(1), // DEC C
            0x15 => self.dec_r(2), // DEC D
            0x1D => self.dec_r(3), // DEC E
            0x25 => self.dec_r(4), // DEC H
            0x2D => self.dec_r(5), // DEC L
            0x35 => self.dec_r(6), // DEC (HL)
            0x3D => self.dec_r(7), // DEC A

            // INC rr
            0x03 => self.inc_rr(0), // INC BC
            0x13 => self.inc_rr(1), // INC DE
            0x23 => self.inc_rr(2), // INC HL
            0x33 => self.inc_rr(3), // INC SP

            // DEC rr
            0x0B => self.dec_rr(0), // DEC BC
            0x1B => self.dec_rr(1), // DEC DE
            0x2B => self.dec_rr(2), // DEC HL
            0x3B => self.dec_rr(3), // DEC SP

            // ADD HL,rr
            0x09 => self.add_hl_rr(0), // ADD HL,BC
            0x19 => self.add_hl_rr(1), // ADD HL,DE
            0x29 => self.add_hl_rr(2), // ADD HL,HL
            0x39 => self.add_hl_rr(3), // ADD HL,SP

            // ADD A,r
            0x80..=0x87 => self.add_a_r(opcode - 0x80),

            // ADC A,r
            0x88..=0x8F => self.adc_a_r(opcode - 0x88),

            // SUB A,r
            0x90..=0x97 => self.sub_a_r(opcode - 0x90),

            // SBC A,r
            0x98..=0x9F => self.sbc_a_r(opcode - 0x98),

            // ADD SP,n
            0xE8 => self.add_sp_n(),

            // DAA
            0x27 => self.daa(),

            _ => {
                println!("未實作的算術指令: 0x{:02X}", opcode);
                CYCLES_1
            }
        }
    }
}
