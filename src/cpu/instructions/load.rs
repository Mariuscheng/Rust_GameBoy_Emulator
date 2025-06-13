use super::{CPU, CYCLES_1, CYCLES_2, CYCLES_3};

impl CPU {
    pub(crate) fn execute_load_instruction(&mut self, opcode: u8) -> u8 {
        match opcode {
            // 8-bit Load Instructions
            0x06 => self.ld_b_n(), // LD B,n
            0x0E => self.ld_c_n(), // LD C,n
            0x16 => self.ld_d_n(), // LD D,n
            0x1E => self.ld_e_n(), // LD E,n
            0x26 => self.ld_h_n(), // LD H,n
            0x2E => self.ld_l_n(), // LD L,n
            0x3E => self.ld_a_n(), // LD A,n

            // 16-bit Load Instructions
            0x01 => self.ld_bc_nn(), // LD BC,nn
            0x11 => self.ld_de_nn(), // LD DE,nn
            0x21 => self.ld_hl_nn(), // LD HL,nn
            0x31 => self.ld_sp_nn(), // LD SP,nn

            // Register to Register loads
            0x40..=0x7F => self.execute_reg_to_reg_load(opcode), // Memory loads
            0x0A => self.ld_a_mem_bc(),                          // LD A,(BC)
            0x1A => self.ld_a_mem_de(),                          // LD A,(DE)
            0x02 => self.ld_mem_bc_a(),                          // LD (BC),A
            0x12 => self.ld_mem_de_a(),                          // LD (DE),A
            0x32 => self.ld_mem_hl_dec_a(),                      // LD (HL-),A
            0xF0 => self.ldh_a_mem_n(),                          // LDH A,(n)
            0xF2 => self.ldh_a_n(),                              // LDH A,(C)
            0xE0 => self.ldh_mem_n_a(),                          // LDH (n),A
            0xE2 => self.ld_mem_c_a(),                           // LD (C),A
            0xEA => self.ld_mem_nn_a(),                          // LD (nn),A
            0xFA => self.ld_a_mem_nn(),                          // LD A,(nn)

            _ => CYCLES_1,
        }
    }

    // 8-bit Load implementations
    fn ld_b_n(&mut self) -> u8 {
        self.registers.b = self.fetch();
        CYCLES_2
    }

    fn ld_c_n(&mut self) -> u8 {
        self.registers.c = self.fetch();
        CYCLES_2
    }

    fn ld_d_n(&mut self) -> u8 {
        self.registers.d = self.fetch();
        CYCLES_2
    }

    fn ld_e_n(&mut self) -> u8 {
        self.registers.e = self.fetch();
        CYCLES_2
    }

    fn ld_h_n(&mut self) -> u8 {
        self.registers.h = self.fetch();
        CYCLES_2
    }

    fn ld_l_n(&mut self) -> u8 {
        self.registers.l = self.fetch();
        CYCLES_2
    }

    fn ld_a_n(&mut self) -> u8 {
        self.registers.a = self.fetch();
        CYCLES_2
    }

    // 記憶體載入指令
    fn ld_a_mem_bc(&mut self) -> u8 {
        let addr = self.registers.get_bc();
        self.registers.a = self.mmu.read_byte(addr);
        CYCLES_2
    }

    fn ld_a_mem_de(&mut self) -> u8 {
        let addr = self.registers.get_de();
        self.registers.a = self.mmu.read_byte(addr);
        CYCLES_2
    }

    fn ld_mem_bc_a(&mut self) -> u8 {
        let addr = self.registers.get_bc();
        self.mmu.write_byte(addr, self.registers.a);
        CYCLES_2
    }

    fn ld_mem_de_a(&mut self) -> u8 {
        let addr = self.registers.get_de();
        self.mmu.write_byte(addr, self.registers.a);
        CYCLES_2
    }

    // LD (HL-),A 的實作
    fn ld_mem_hl_dec_a(&mut self) -> u8 {
        let addr = self.registers.get_hl();
        self.mmu.write_byte(addr, self.registers.a);
        self.registers.set_hl(addr.wrapping_sub(1));
        CYCLES_2
    }

    // 16-bit 載入指令的實作
    fn ld_bc_nn(&mut self) -> u8 {
        let low = self.fetch();
        let high = self.fetch();
        self.registers.set_bc((high as u16) << 8 | low as u16);
        CYCLES_3
    }

    fn ld_de_nn(&mut self) -> u8 {
        let low = self.fetch();
        let high = self.fetch();
        self.registers.set_de((high as u16) << 8 | low as u16);
        CYCLES_3
    }

    fn ld_hl_nn(&mut self) -> u8 {
        let low = self.fetch();
        let high = self.fetch();
        self.registers.set_hl((high as u16) << 8 | low as u16);
        CYCLES_3
    }

    fn ld_sp_nn(&mut self) -> u8 {
        let low = self.fetch();
        let high = self.fetch();
        self.registers.sp = (high as u16) << 8 | low as u16;
        CYCLES_3
    }

    // Register to Register loads
    fn execute_reg_to_reg_load(&mut self, opcode: u8) -> u8 {
        let source = opcode & 0x07;
        let dest = (opcode >> 3) & 0x07;

        let value = match source {
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

        match dest {
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

        if source == 6 || dest == 6 {
            CYCLES_2 // 涉及到記憶體操作需要 2 個週期
        } else {
            CYCLES_1 // 純暫存器操作只需要 1 個週期
        }
    }

    fn ld_b_c(&mut self) -> u8 {
        self.registers.b = self.registers.c;
        CYCLES_1
    }

    fn ldh_a_n(&mut self) -> u8 {
        let offset = self.fetch();
        let addr = 0xFF00 | (offset as u16);
        self.registers.a = self.mmu.read_byte(addr);
        CYCLES_2
    }

    fn ldh_a_mem_n(&mut self) -> u8 {
        let offset = self.fetch();
        let addr = 0xFF00 | (offset as u16);
        self.registers.a = self.mmu.read_byte(addr);
        CYCLES_2
    }

    fn ldh_mem_n_a(&mut self) -> u8 {
        let n = self.mmu.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        self.mmu.write_byte(0xFF00 | n as u16, self.registers.a);
        CYCLES_3 // 12 cycles
    }

    fn ld_mem_c_a(&mut self) -> u8 {
        self.mmu
            .write_byte(0xFF00 | self.registers.c as u16, self.registers.a);
        CYCLES_2 // 8 cycles
    }

    fn ld_mem_nn_a(&mut self) -> u8 {
        let addr_low = self.mmu.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        let addr_high = self.mmu.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        let addr = ((addr_high as u16) << 8) | (addr_low as u16);
        self.mmu.write_byte(addr, self.registers.a);
        CYCLES_3 // 16 cycles
    }

    fn ld_a_mem_nn(&mut self) -> u8 {
        let addr_low = self.mmu.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        let addr_high = self.mmu.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        let addr = ((addr_high as u16) << 8) | (addr_low as u16);
        self.registers.a = self.mmu.read_byte(addr);
        CYCLES_3 // 16 cycles
    }
}
