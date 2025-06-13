use super::{CPU, CYCLES_2, CYCLES_3, CYCLES_4};
use crate::cpu::flags::*;

impl CPU {
    pub(crate) fn execute_jump_instruction(&mut self, opcode: u8) -> u8 {
        match opcode {
            // 無條件跳躍
            0xC3 => self.jp_nn(), // JP nn

            // 條件跳躍
            0xC2 => self.jp_nz_nn(), // JP NZ,nn
            0xCA => self.jp_z_nn(),  // JP Z,nn
            0xD2 => self.jp_nc_nn(), // JP NC,nn
            0xDA => self.jp_c_nn(),  // JP C,nn

            // 相對跳躍
            0x18 => self.jr_n(),    // JR n
            0x20 => self.jr_nz_n(), // JR NZ,n
            0x28 => self.jr_z_n(),  // JR Z,n
            0x30 => self.jr_nc_n(), // JR NC,n
            0x38 => self.jr_c_n(),  // JR C,n

            // 呼叫子程式
            0xCD => self.call_nn(),    // CALL nn
            0xC4 => self.call_nz_nn(), // CALL NZ,nn
            0xCC => self.call_z_nn(),  // CALL Z,nn
            0xD4 => self.call_nc_nn(), // CALL NC,nn
            0xDC => self.call_c_nn(),  // CALL C,nn

            // 返回
            0xC9 => self.ret(),    // RET
            0xC0 => self.ret_nz(), // RET NZ
            0xC8 => self.ret_z(),  // RET Z
            0xD0 => self.ret_nc(), // RET NC
            0xD8 => self.ret_c(),  // RET C

            _ => {
                println!(
                    "未實作的跳躍指令: 0x{:02X} at PC: 0x{:04X}",
                    opcode,
                    self.registers.pc.wrapping_sub(1)
                );
                CYCLES_3
            }
        }
    } // 無條件跳躍實作
    fn jp_nn(&mut self) -> u8 {
        let low = self.fetch() as u16;
        let high = self.fetch() as u16;
        let addr = (high << 8) | low;
        self.registers.pc = addr;
        CYCLES_4
    }

    // 條件跳躍實作
    fn jp_nz_nn(&mut self) -> u8 {
        let low = self.fetch() as u16;
        let high = self.fetch() as u16;
        let addr = (high << 8) | low;
        if !self.registers.get_z_flag() {
            self.registers.pc = addr;
            return CYCLES_4;
        }
        CYCLES_3
    }

    fn jp_z_nn(&mut self) -> u8 {
        let low = self.fetch() as u16;
        let high = self.fetch() as u16;
        let addr = (high << 8) | low;
        if self.registers.get_z_flag() {
            self.registers.pc = addr;
            return CYCLES_4;
        }
        CYCLES_3
    }

    fn jp_nc_nn(&mut self) -> u8 {
        let low = self.fetch() as u16;
        let high = self.fetch() as u16;
        let addr = (high << 8) | low;
        if !self.registers.get_c_flag() {
            self.registers.pc = addr;
            return CYCLES_4;
        }
        CYCLES_3
    }

    fn jp_c_nn(&mut self) -> u8 {
        let low = self.fetch() as u16;
        let high = self.fetch() as u16;
        let addr = (high << 8) | low;
        if self.registers.get_c_flag() {
            self.registers.pc = addr;
            return CYCLES_4;
        }
        CYCLES_3
    }

    // 相對跳躍實作
    fn jr_n(&mut self) -> u8 {
        let offset = self.fetch() as i8;
        self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
        CYCLES_3
    }

    fn jr_nz_n(&mut self) -> u8 {
        let offset = self.fetch() as i8;
        if !self.registers.get_z_flag() {
            self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
            return CYCLES_3;
        }
        CYCLES_2
    }

    fn jr_z_n(&mut self) -> u8 {
        let offset = self.fetch() as i8;
        if self.registers.get_z_flag() {
            self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
            return CYCLES_3;
        }
        CYCLES_2
    }

    fn jr_nc_n(&mut self) -> u8 {
        let offset = self.fetch() as i8;
        if !self.registers.get_c_flag() {
            self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
            return CYCLES_3;
        }
        CYCLES_2
    }

    fn jr_c_n(&mut self) -> u8 {
        let offset = self.fetch() as i8;
        if self.registers.get_c_flag() {
            self.registers.pc = self.registers.pc.wrapping_add(offset as u16);
            return CYCLES_3;
        }
        CYCLES_2
    }

    // 呼叫子程式實作
    fn call_nn(&mut self) -> u8 {
        let low = self.fetch() as u16;
        let high = self.fetch() as u16;
        let addr = (high << 8) | low;

        // 保存返回地址到堆疊
        let pc = self.registers.pc;
        self.push_word(pc);

        // 跳轉到目標地址
        self.registers.pc = addr;
        CYCLES_4
    }

    fn call_nz_nn(&mut self) -> u8 {
        let low = self.fetch() as u16;
        let high = self.fetch() as u16;
        let addr = (high << 8) | low;

        if !self.registers.get_z_flag() {
            let pc = self.registers.pc;
            self.push_word(pc);
            self.registers.pc = addr;
            return CYCLES_4;
        }
        CYCLES_3
    }

    fn call_z_nn(&mut self) -> u8 {
        let low = self.fetch() as u16;
        let high = self.fetch() as u16;
        let addr = (high << 8) | low;

        if self.registers.get_z_flag() {
            let pc = self.registers.pc;
            self.push_word(pc);
            self.registers.pc = addr;
            return CYCLES_4;
        }
        CYCLES_3
    }

    fn call_nc_nn(&mut self) -> u8 {
        let low = self.fetch() as u16;
        let high = self.fetch() as u16;
        let addr = (high << 8) | low;

        if !self.registers.get_c_flag() {
            let pc = self.registers.pc;
            self.push_word(pc);
            self.registers.pc = addr;
            return CYCLES_4;
        }
        CYCLES_3
    }

    fn call_c_nn(&mut self) -> u8 {
        let low = self.fetch() as u16;
        let high = self.fetch() as u16;
        let addr = (high << 8) | low;

        if self.registers.get_c_flag() {
            let pc = self.registers.pc;
            self.push_word(pc);
            self.registers.pc = addr;
            return CYCLES_4;
        }
        CYCLES_3
    }

    // 特殊跳轉指令
    fn jp_hl(&mut self) -> u8 {
        self.registers.pc = self.registers.get_hl();
        CYCLES_2
    }

    // RST 指令實作
    fn rst(&mut self, addr: u16) -> u8 {
        self.push_word(self.registers.pc);
        self.registers.pc = addr;
        CYCLES_4
    }

    /// Execute a return instruction
    pub fn execute_return_instruction(&mut self, opcode: u8) -> u8 {
        match opcode {
            0xC0 => self.ret_nz(), // RET NZ
            0xC8 => self.ret_z(),  // RET Z
            0xC9 => self.ret(),    // RET
            0xD0 => self.ret_nc(), // RET NC
            0xD8 => self.ret_c(),  // RET C
            0xD9 => self.reti(),   // RETI
            _ => CYCLES_2,         // Should never happen
        }
    }

    /// RET - Return from subroutine
    pub fn ret(&mut self) -> u8 {
        self.registers.pc = self.pop_word();
        CYCLES_4
    }

    /// RET NZ - Return if not zero
    pub fn ret_nz(&mut self) -> u8 {
        if !self.registers.get_z_flag() {
            self.registers.pc = self.pop_word();
            CYCLES_4
        } else {
            CYCLES_2
        }
    }

    /// RET Z - Return if zero
    pub fn ret_z(&mut self) -> u8 {
        if self.registers.get_z_flag() {
            self.registers.pc = self.pop_word();
            CYCLES_4
        } else {
            CYCLES_2
        }
    }

    /// RET NC - Return if not carry
    pub fn ret_nc(&mut self) -> u8 {
        if !self.registers.get_c_flag() {
            self.registers.pc = self.pop_word();
            CYCLES_4
        } else {
            CYCLES_2
        }
    }

    /// RET C - Return if carry
    pub fn ret_c(&mut self) -> u8 {
        if self.registers.get_c_flag() {
            self.registers.pc = self.pop_word();
            CYCLES_4
        } else {
            CYCLES_2
        }
    }

    /// RETI - Return and enable interrupts
    pub fn reti(&mut self) -> u8 {
        self.registers.pc = self.pop_word();
        self.ime = true;
        CYCLES_4
    }
}
