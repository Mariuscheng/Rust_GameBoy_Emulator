pub mod flags;
pub mod instructions;
pub mod interrupts;
pub mod opcodes;
pub mod registers;

use self::instructions::*;
use self::registers::Registers;
use crate::mmu::MMU;

pub const CPU_CLOCK_SPEED: u32 = 4_194_304; // 4.194304 MHz
pub const MACHINE_CYCLES_PER_SECOND: u32 = CPU_CLOCK_SPEED / 4;

pub struct CPU {
    pub registers: Registers,
    pub mmu: MMU,
    pub ime: bool,           // 中斷主使能標誌
    pub ime_scheduled: bool, // 預定啟用中斷標誌
    pub halted: bool,        // CPU 停止標誌
    pub stopped: bool,       // CPU STOP 指令狀態
    instruction_count: u64,
    clock_cycles: u64,   // 追蹤時脈週期
    machine_cycles: u64, // 追蹤機器週期
}

impl CPU {
    pub fn new(mmu: MMU) -> CPU {
        CPU {
            registers: Registers::new(),
            mmu,
            ime: false,
            ime_scheduled: false,
            halted: false,
            stopped: false,
            instruction_count: 0,
            clock_cycles: 0,
            machine_cycles: 0,
        }
    }
    /// 執行一條指令並返回消耗的時脈週期數
    pub fn step(&mut self) -> u8 {
        let current_pc = self.registers.pc;

        if self.halted {
            // 在停止狀態下，仍然需要更新定時器
            self.update_timer(4); // 一個 NOP 指令的週期
            return 4;
        }

        let opcode = self.fetch();

        // 輸出指令執行信息
        if current_pc <= 0x100 || (current_pc >= 0x100 && current_pc <= 0x150) {
            println!("\n╔═══ CPU 執行 ═══════════════════");
            println!("║ PC: 0x{:04X}", current_pc);
            println!(
                "║ OP: 0x{:02X} ({})",
                opcode,
                self.get_instruction_name(opcode)
            );
            println!("╟───────────────────────────────");
            println!(
                "║ AF: 0x{:04X}  BC: 0x{:04X}",
                self.registers.get_af(),
                self.registers.get_bc()
            );
            println!(
                "║ DE: 0x{:04X}  HL: 0x{:04X}",
                self.registers.get_de(),
                self.registers.get_hl()
            );
            println!("║ SP: 0x{:04X}", self.registers.sp);
            println!(
                "║ Flags: Z={} N={} H={} C={}",
                (self.registers.f >> 7) & 1,
                (self.registers.f >> 6) & 1,
                (self.registers.f >> 5) & 1,
                (self.registers.f >> 4) & 1
            );
            println!("╚═════════════════════════════════");
        }

        let cycles = self.execute_instruction(opcode);

        // 更新定時器
        self.update_timer(cycles);

        // 更新週期計數
        self.clock_cycles += cycles as u64;
        self.machine_cycles += (cycles >> 2) as u64; // 4 個時脈週期 = 1 個機器週期
        self.instruction_count += 1;

        cycles
    }

    pub fn fetch(&mut self) -> u8 {
        let opcode = self.mmu.read_byte(self.registers.pc);
        self.registers.pc = self.registers.pc.wrapping_add(1);
        opcode
    }

    pub fn execute_instruction(&mut self, opcode: u8) -> u8 {
        match opcode {
            // 調用 instructions 子模組中的執行器
            // Load Instructions
            0x01
            | 0x02
            | 0x06
            | 0x0A
            | 0x0E
            | 0x11
            | 0x16
            | 0x1A
            | 0x1E
            | 0x21
            | 0x26
            | 0x2E
            | 0x31
            | 0x32  // 新增 LD (HL-),A
            | 0x36
            | 0x3E
            | 0x40..=0x7F
            | 0xE0
            | 0xE2
            | 0xEA
            | 0xF0
            | 0xF2
            | 0xFA => self.execute_load_instruction(opcode),

            // Arithmetic Instructions
            0x03
            | 0x04
            | 0x05
            | 0x0B
            | 0x0C
            | 0x0D
            | 0x13
            | 0x14
            | 0x15
            | 0x1B
            | 0x1C
            | 0x1D
            | 0x23
            | 0x24
            | 0x25
            | 0x2B
            | 0x2C
            | 0x2D
            | 0x33
            | 0x34
            | 0x35
            | 0x3B
            | 0x3C
            | 0x3D
            | 0x80..=0x8F
            | 0x90..=0x9F => self.execute_arithmetic_instruction(opcode),

            // Logic Instructions
            0xA0..=0xAF | 0xB0..=0xBF | 0xE6 | 0xEE | 0xF6 | 0xFE => {
                self.execute_logic_instruction(opcode)
            }

            // Control Instructions
            0x00 => self.execute_nop(),  // NOP
            0x10 => self.execute_stop(), // STOP
            0x76 => self.execute_halt(), // HALT
            0xF3 => self.execute_di(),   // DI
            0xFB => self.execute_ei(),   // EI

            // Jump Instructions
            0x18 | 0x20 | 0x28 | 0x30 | 0x38 | 0xC2 | 0xC3 | 0xC4 | 0xCA | 0xCC | 0xCD | 0xD2
            | 0xD4 | 0xDA | 0xDC => self.execute_jump_instruction(opcode),

            // Return Instructions
            0xC0 | 0xC8 | 0xC9 | 0xD0 | 0xD8 | 0xD9 => self.execute_return_instruction(opcode),

            // 0xCB 前綴指令（位操作）
            0xCB => {
                let cb_opcode = self.fetch();
                match cb_opcode {
                    // BIT 操作 (0x40-0x7F)
                    0x40..=0x7F => {
                        let bit = (cb_opcode - 0x40) / 8;
                        let reg = (cb_opcode - 0x40) % 8;
                        self.execute_bit_instruction(bit, reg)
                    }

                    // RES 操作 (0x80-0xBF)
                    0x80..=0xBF => {
                        let bit = (cb_opcode - 0x80) / 8;
                        let reg = (cb_opcode - 0x80) % 8;
                        self.execute_res_instruction(bit, reg)
                    }

                    // SET 操作 (0xC0-0xFF)
                    0xC0..=0xFF => {
                        let bit = (cb_opcode - 0xC0) / 8;
                        let reg = (cb_opcode - 0xC0) % 8;
                        self.execute_set_instruction(bit, reg)
                    }

                    _ => {
                        println!("未實作的 CB 指令: 0x{:02X}", cb_opcode);
                        8 // 2個機器週期
                    }
                }
            }

            _ => {
                println!(
                    "未實現的指令: 0x{:02X} at PC: 0x{:04X}",
                    opcode,
                    self.registers.pc.wrapping_sub(1)
                );
                4 // 1個機器週期
            }
        }
    }

    fn get_instruction_name(&self, opcode: u8) -> &'static str {
        match opcode {
            // Load Instructions
            0x01 => "LD BC,nn",
            0x02 => "LD (BC),A",
            0x06 => "LD B,n",
            0x0A => "LD A,(BC)",
            0x0E => "LD C,n",
            0x11 => "LD DE,nn",
            0x16 => "LD D,n",
            0x1A => "LD A,(DE)",
            0x1E => "LD E,n",
            0x21 => "LD HL,nn",
            0x26 => "LD H,n",
            0x2E => "LD L,n",
            0x31 => "LD SP,nn",
            0x32 => "LD (HL-),A",
            0x36 => "LD (HL),n",
            0x3E => "LD A,n",
            0x40..=0x7F => {
                let dst = (opcode >> 3) & 0x07;
                let src = opcode & 0x07;
                match (dst, src) {
                    (6, _) => "LD (HL),r",
                    (_, 6) => "LD r,(HL)",
                    _ => "LD r,r",
                }
            }
            0xE0 => "LDH (n),A",
            0xE2 => "LD (C),A",
            0xEA => "LD (nn),A",
            0xF0 => "LDH A,(n)",
            0xF2 => "LD A,(C)",
            0xFA => "LD A,(nn)",

            // Arithmetic Instructions
            0x03 => "INC BC",
            0x04 => "INC B",
            0x05 => "DEC B",
            0x0B => "DEC BC",
            0x0C => "INC C",
            0x0D => "DEC C",
            0x13 => "INC DE",
            0x14 => "INC D",
            0x15 => "DEC D",
            0x1B => "DEC DE",
            0x1C => "INC E",
            0x1D => "DEC E",
            0x23 => "INC HL",
            0x24 => "INC H",
            0x25 => "DEC H",
            0x2B => "DEC HL",
            0x2C => "INC L",
            0x2D => "DEC L",
            0x33 => "INC SP",
            0x34 => "INC (HL)",
            0x35 => "DEC (HL)",
            0x3B => "DEC SP",
            0x3C => "INC A",
            0x3D => "DEC A",

            // Jump Instructions
            0x18 => "JR n",
            0x20 => "JR NZ,n",
            0x28 => "JR Z,n",
            0x30 => "JR NC,n",
            0x38 => "JR C,n",
            0xC2 => "JP NZ,nn",
            0xC3 => "JP nn",
            0xCA => "JP Z,nn",
            0xD2 => "JP NC,nn",
            0xDA => "JP C,nn",

            // Call/Return Instructions
            0xC0 => "RET NZ",
            0xC1 => "POP BC",
            0xC4 => "CALL NZ,nn",
            0xC5 => "PUSH BC",
            0xC8 => "RET Z",
            0xC9 => "RET",
            0xCC => "CALL Z,nn",
            0xCD => "CALL nn",
            0xD0 => "RET NC",
            0xD1 => "POP DE",
            0xD4 => "CALL NC,nn",
            0xD5 => "PUSH DE",
            0xD8 => "RET C",
            0xD9 => "RETI",
            0xDC => "CALL C,nn",

            // Control Instructions
            0x00 => "NOP",
            0x27 => "DAA",
            0x2F => "CPL",
            0x37 => "SCF",
            0x3F => "CCF",
            0x76 => "HALT",
            0xF3 => "DI",
            0xFB => "EI",

            // 未實現的指令
            _ => "Unknown",
        }
    }

    /// 更新定時器並處理中斷
    fn update_timer(&mut self, cycles: u8) {
        // 如果定時器觸發了中斷
        if self.mmu.timer.update(cycles) {
            // 設置定時器中斷標誌
            let if_value = self.mmu.read_byte(0xFF0F);
            self.mmu.write_byte(0xFF0F, if_value | 0x04);
        }
    }

    /// 處理中斷
    pub fn handle_interrupts(&mut self) -> bool {
        if !self.ime {
            // 即使中斷被禁用，HALT 模式下的中斷也能喚醒 CPU
            if self.halted {
                let ie = self.mmu.read_byte(0xFFFF);
                let if_flags = self.mmu.read_byte(0xFF0F);
                if (ie & if_flags & 0x1F) != 0 {
                    self.halted = false;
                }
            }
            return false;
        }

        // 讀取中斷使能寄存器 (IE) 和中斷標誌寄存器 (IF)
        let ie = self.mmu.read_byte(0xFFFF);
        let mut if_flags = self.mmu.read_byte(0xFF0F);

        // 檢查所有中斷，按優先級順序處理
        // V-Blank (0x01, 最高優先級)
        if (ie & if_flags & 0x01) != 0 {
            self.service_interrupt(0x40, 0x01, &mut if_flags);
            return true;
        }

        // LCD STAT (0x02)
        if (ie & if_flags & 0x02) != 0 {
            self.service_interrupt(0x48, 0x02, &mut if_flags);
            return true;
        }

        // Timer (0x04)
        if (ie & if_flags & 0x04) != 0 {
            self.service_interrupt(0x50, 0x04, &mut if_flags);
            return true;
        }

        // Serial (0x08)
        if (ie & if_flags & 0x08) != 0 {
            self.service_interrupt(0x58, 0x08, &mut if_flags);
            return true;
        }

        // Joypad (0x10, 最低優先級)
        if (ie & if_flags & 0x10) != 0 {
            self.service_interrupt(0x60, 0x10, &mut if_flags);
            return true;
        }

        false
    }

    /// 服務特定中斷
    fn service_interrupt(&mut self, vector: u16, interrupt_bit: u8, if_flags: &mut u8) {
        // 禁用中斷
        self.ime = false;
        self.halted = false;

        // 清除中斷標誌
        *if_flags &= !interrupt_bit;
        self.mmu.write_byte(0xFF0F, *if_flags);

        // 保存當前 PC 到堆疊
        self.push_word(self.registers.pc);

        // 跳轉到中斷向量
        self.registers.pc = vector;
    }

    /// 將 16 位元值壓入堆疊
    fn push_word(&mut self, value: u16) {
        self.registers.sp = self.registers.sp.wrapping_sub(2);
        self.mmu.write_word(self.registers.sp, value);
    }

    /// 從堆疊彈出 16 位元值
    fn pop_word(&mut self) -> u16 {
        let value = self.mmu.read_word(self.registers.sp);
        self.registers.sp = self.registers.sp.wrapping_add(2);
        value
    }
}

pub const CYCLES_1: u8 = 4; // 1個機器週期 = 4個時脈週期
pub const CYCLES_2: u8 = 8; // 2個機器週期 = 8個時脈週期
pub const CYCLES_3: u8 = 12; // 3個機器週期 = 12個時脈週期
pub const CYCLES_4: u8 = 16; // 4個機器週期 = 16個時脈週期
pub const CYCLES_5: u8 = 20; // 5個機器週期 = 20個時脈週期
pub const CYCLES_6: u8 = 24; // 6個機器週期 = 24個時脈週期
