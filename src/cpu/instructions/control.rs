use super::{CPU, CYCLES_1, CYCLES_2};

impl CPU {
    /// NOP - No Operation
    pub fn execute_nop(&self) -> u8 {
        CYCLES_1
    }

    /// STOP - Stop processor
    pub fn execute_stop(&mut self) -> u8 {
        // TODO: Implement proper STOP behavior
        CYCLES_1
    }

    /// HALT - Halt processor
    pub fn execute_halt(&mut self) -> u8 {
        self.halted = true;
        CYCLES_1
    }

    /// DI - Disable Interrupts
    pub fn execute_di(&mut self) -> u8 {
        self.ime = false;
        CYCLES_1
    }

    /// EI - Enable Interrupts
    pub fn execute_ei(&mut self) -> u8 {
        self.ime = true;
        CYCLES_1
    }

    pub(crate) fn execute_control_instruction(&mut self, opcode: u8) -> u8 {
        match opcode {
            // 基本控制指令
            0x00 => CYCLES_1,    // NOP
            0x10 => self.stop(), // STOP
            0x27 => self.daa(),  // DAA
            0x2F => self.cpl(),  // CPL
            0x37 => self.scf(),  // SCF
            0x3F => self.ccf(),  // CCF
            0x76 => self.halt(), // HALT
            0xF3 => self.di(),   // DI
            0xFB => self.ei(),   // EI            // RST 指令 - 轉發到 jump instructions 處理
            0xC7 | 0xCF | 0xD7 | 0xDF | 0xE7 | 0xEF | 0xF7 | 0xFF => {
                self.execute_jump_instruction(opcode)
            }

            _ => {
                println!(
                    "未實作的控制指令: 0x{:02X} at PC: 0x{:04X}",
                    opcode,
                    self.registers.pc.wrapping_sub(1)
                );
                CYCLES_1
            }
        }
    } // 停止執行
    pub(crate) fn stop(&mut self) -> u8 {
        self.stopped = true;
        CYCLES_2
    } // 十進制調整
    pub(crate) fn daa(&mut self) -> u8 {
        let mut adjust = 0;
        let mut carry = false;

        if self.registers.get_h_flag()
            || (!self.registers.get_n_flag() && (self.registers.a & 0x0F) > 9)
        {
            adjust |= 0x06;
        }

        if self.registers.get_c_flag() || (!self.registers.get_n_flag() && self.registers.a > 0x99)
        {
            adjust |= 0x60;
            carry = true;
        }

        self.registers.a = if self.registers.get_n_flag() {
            self.registers.a.wrapping_sub(adjust)
        } else {
            self.registers.a.wrapping_add(adjust)
        };

        self.registers.set_z_flag(self.registers.a == 0);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(carry);

        CYCLES_1
    } // 取反 A 寄存器
    pub(crate) fn cpl(&mut self) -> u8 {
        self.registers.a = !self.registers.a;
        self.registers.set_n_flag(true);
        self.registers.set_h_flag(true);
        CYCLES_1
    } // 設定進位標誌
    pub(crate) fn scf(&mut self) -> u8 {
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(true);
        CYCLES_1
    } // 反轉進位標誌
    pub(crate) fn ccf(&mut self) -> u8 {
        let current_c = self.registers.get_c_flag();
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(!current_c);
        CYCLES_1
    } // 停止 CPU 直到中斷發生
    pub(crate) fn halt(&mut self) -> u8 {
        self.halted = true;
        CYCLES_1
    } // 禁用中斷
    pub(crate) fn di(&mut self) -> u8 {
        self.ime = false;
        CYCLES_1
    } // 啟用中斷
    pub(crate) fn ei(&mut self) -> u8 {
        self.ime_scheduled = true;
        CYCLES_1
    } // RST 指令已經在 jump.rs 中實作
}
