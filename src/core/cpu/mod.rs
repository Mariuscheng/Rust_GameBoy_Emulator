use self::flags::Flag;
use crate::core::cycles::*;
use crate::core::mmu::MMU;
use crate::error::{Error, InstructionError, RegTarget, Result};
use std::io::Write;
use std::{cell::RefCell, rc::Rc};

pub mod flags;
pub mod instructions;
pub mod interrupts;
pub mod registers;

use self::registers::Registers;

#[derive(Debug)]
pub struct CPU {
    registers: Registers,
    mmu: Rc<RefCell<MMU>>,
    halted: bool,
    ime: bool,
    ime_scheduled: bool,
    instruction_count: u64,
}

impl CPU {    pub fn new(mmu: Rc<RefCell<MMU>>) -> Self {
        // Create log directory
        std::fs::create_dir_all("logs").ok();

        // Create basic CPU instance
        let mut cpu = Self {
            registers: Registers::new(),
            mmu,
            halted: false,
            ime: false,
            ime_scheduled: false,
            instruction_count: 0,
        };

        // Set standard register initial values according to Game Boy CPU Manual
        cpu.reset().unwrap_or_default();
        cpu
    }

    // Arithmetic instruction implementations
    pub fn add_a_r(&mut self, source: RegTarget, use_carry: bool) -> Result<CyclesType> {
        let src_val = match source {
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
            _ => {
                return Err(Error::Instruction(InstructionError::InvalidRegister(
                    source,
                )))
            }
        };

        self.add_a(src_val, use_carry);
        Ok(if matches!(source, RegTarget::HL) {
            CYCLES_2
        } else {
            CYCLES_1
        })
    }

    pub fn add_a_n(&mut self, use_carry: bool) -> Result<CyclesType> {
        let value = self.fetch_byte()?;
        self.add_a(value, use_carry);
        Ok(CYCLES_2)
    }

    fn add_a(&mut self, value: u8, use_carry: bool) {
        let carry = if use_carry && self.registers.get_flag(Flag::C) {
            1u8
        } else {
            0u8
        };
        let result = self.registers.a as u16 + value as u16 + carry as u16;
        let half_carry = ((self.registers.a & 0x0F) + (value & 0x0F) + carry) > 0x0F;

        self.registers.set_flag(Flag::Z, (result & 0xFF) == 0);
        self.registers.set_flag(Flag::N, false);
        self.registers.set_flag(Flag::H, half_carry);
        self.registers.set_flag(Flag::C, result > 0xFF);

        self.registers.a = result as u8;
    }

    pub fn sub_a_r(&mut self, source: RegTarget, use_carry: bool) -> Result<CyclesType> {
        let src_val = match source {
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
            _ => {
                return Err(Error::Instruction(InstructionError::InvalidRegister(
                    source,
                )))
            }
        };

        self.sub_a(src_val, use_carry);
        Ok(if matches!(source, RegTarget::HL) {
            CYCLES_2
        } else {
            CYCLES_1
        })
    }

    pub fn sub_a_n(&mut self, use_carry: bool) -> Result<CyclesType> {
        let value = self.fetch_byte()?;
        self.sub_a(value, use_carry);
        Ok(CYCLES_2)
    }

    fn sub_a(&mut self, value: u8, use_carry: bool) {
        let carry = if use_carry && self.registers.get_flag(Flag::C) {
            1
        } else {
            0
        };
        let result = self.registers.a.wrapping_sub(value).wrapping_sub(carry);
        let half_carry = (self.registers.a & 0x0F) < ((value & 0x0F) + carry);
        let carry = (self.registers.a as i16 - value as i16 - carry as i16) < 0;

        self.registers.set_flag(Flag::Z, result == 0);
        self.registers.set_flag(Flag::N, true);
        self.registers.set_flag(Flag::H, half_carry);
        self.registers.set_flag(Flag::C, carry);

        self.registers.a = result;
    }

    pub fn dec_r(&mut self, target: RegTarget) -> Result<CyclesType> {
        let value = match target {
            RegTarget::A => {
                let result = self.registers.a.wrapping_sub(1);
                self.set_dec_flags(result);
                self.registers.a = result;
                result
            }
            RegTarget::B => {
                let result = self.registers.b.wrapping_sub(1);
                self.set_dec_flags(result);
                self.registers.b = result;
                result
            }
            RegTarget::C => {
                let result = self.registers.c.wrapping_sub(1);
                self.set_dec_flags(result);
                self.registers.c = result;
                result
            }
            RegTarget::D => {
                let result = self.registers.d.wrapping_sub(1);
                self.set_dec_flags(result);
                self.registers.d = result;
                result
            }
            RegTarget::E => {
                let result = self.registers.e.wrapping_sub(1);
                self.set_dec_flags(result);
                self.registers.e = result;
                result
            }
            RegTarget::H => {
                let result = self.registers.h.wrapping_sub(1);
                self.set_dec_flags(result);
                self.registers.h = result;
                result
            }
            RegTarget::L => {
                let result = self.registers.l.wrapping_sub(1);
                self.set_dec_flags(result);
                self.registers.l = result;
                result
            }
            RegTarget::HL => {
                let addr = self.registers.get_hl();
                let value = self.read_byte(addr)?;
                let result = value.wrapping_sub(1);
                self.set_dec_flags(result);
                self.write_byte(addr, result)?;
                result
            }
            _ => {
                return Err(Error::Instruction(InstructionError::InvalidRegister(
                    target,
                )))
            }
        };

        if let Ok(mut f) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/cpu_exec.log")
        {
            writeln!(
                f,
                "DEC {:?}: {} -> {}",
                target,
                value.wrapping_add(1),
                value
            )?;
        }

        Ok(if matches!(target, RegTarget::HL) {
            CYCLES_3
        } else {
            CYCLES_1
        })
    }

    fn set_dec_flags(&mut self, result: u8) {
        self.registers.set_flag(Flag::Z, result == 0);
        self.registers.set_flag(Flag::N, true);
        self.registers.set_flag(Flag::H, (result & 0x0F) == 0x0F);
    }

    // 其他輔助方法
    pub fn fetch_byte(&mut self) -> Result<u8> {
        let byte = self.mmu.borrow().read_byte(self.registers.pc)?;
        self.registers.pc = self.registers.pc.wrapping_add(1);
        Ok(byte)
    }

    pub fn fetch_word(&mut self) -> Result<u16> {
        let low = self.fetch_byte()? as u16;
        let high = self.fetch_byte()? as u16;
        Ok((high << 8) | low)
    }

    pub fn read_byte(&mut self, addr: u16) -> Result<u8> {
        self.mmu.borrow().read_byte(addr)
    }

    pub fn write_byte(&mut self, addr: u16, value: u8) -> Result<()> {
        self.mmu.borrow_mut().write_byte(addr, value)
    }

    pub fn reset(&mut self) -> Result<()> {
        self.registers.set_af(0x01B0);
        self.registers.set_bc(0x0013);
        self.registers.set_de(0x00D8);
        self.registers.set_hl(0x014D);
        self.registers.set_sp(0xFFFE);
        self.registers.set_pc(0x0100);        self.halted = false;
        self.ime = false;
        self.ime_scheduled = false;
        self.instruction_count = 0;
        Ok(())
    }

    // Stack 操作
    pub fn push_word(&mut self, value: u16) -> Result<()> {
        self.registers.sp = self.registers.sp.wrapping_sub(2);
        let sp = self.registers.sp;
        self.write_byte(sp, (value & 0xFF) as u8)?;
        self.write_byte(sp + 1, (value >> 8) as u8)?;
        Ok(())
    }

    pub fn pop_word(&mut self) -> Result<u16> {
        let sp = self.registers.sp;
        let low = self.read_byte(sp)? as u16;
        let high = self.read_byte(sp + 1)? as u16;
        self.registers.sp = self.registers.sp.wrapping_add(2);
        Ok((high << 8) | low)
    }

    fn decode_opcode_name(opcode: u8) -> &'static str {
        match opcode {
            // NOP
            0x00 => "NOP",            // LD instruction family
            0x21 => "LD HL,nn",   // Load immediate 16-bit value to HL
            0x01 => "LD BC,nn",   // Load immediate 16-bit value to BC
            0x11 => "LD DE,nn",   // Load immediate 16-bit value to DE
            0x31 => "LD SP,nn",   // Load immediate 16-bit value to SP
            0x06 => "LD B,n",     // Load immediate 8-bit value to B
            0x0E => "LD C,n",     // Load immediate 8-bit value to C
            0x32 => "LD (HL-),A", // Load A to (HL) and decrement HL

            // Decrement instructions
            0x05 => "DEC B", // Decrement B

            // Jump instructions
            0xC3 => "JP nn", // Jump to immediate address

            // Conditional jump instructions
            0xC2 => "JP NZ,nn", // Jump if not zero
            0xCA => "JP Z,nn",  // Jump if zero
            0xD2 => "JP NC,nn", // Jump if no carry
            0xDA => "JP C,nn",  // Jump if carry
            0xE9 => "JP HL",    // Jump to HL

            // CALL instructions
            0xCD => "CALL nn",    // Unconditional call
            0xC4 => "CALL NZ,nn", // Call if not zero
            0xCC => "CALL Z,nn",  // Call if zero
            0xD4 => "CALL NC,nn", // Call if no carry
            0xDC => "CALL C,nn",  // Call if carry

            // RET instructions
            0xC9 => "RET",    // Unconditional return
            0xC0 => "RET NZ", // Return if not zero
            0xC8 => "RET Z",  // Return if zero
            0xD0 => "RET NC", // Return if no carry
            0xD8 => "RET C",  // Return if carry
            0xD9 => "RETI",   // Return from interrupt

            // Logic instructions
            0xA8..=0xAE => "XOR r", // XOR with register
            0xAF => "XOR A",        // XOR A with itself (sets A to 0)
            0xEE => "XOR n",        // XOR with immediate value
            0xB0..=0xB7 => "OR r",  // OR with register
            0xF6 => "OR n",         // OR with immediate value
            0xA0..=0xA7 => "AND r", // AND with register
            0xE6 => "AND n",        // AND with immediate value
            0xB8..=0xBF => "CP r",  // Compare with register
            0xFE => "CP n",         // Compare with immediate value

            // Stack operations
            0xC5 => "PUSH BC", // Push BC onto stack
            0xD5 => "PUSH DE", // Push DE onto stack
            0xE5 => "PUSH HL", // Push HL onto stack
            0xF5 => "PUSH AF", // Push AF onto stack
            0xC1 => "POP BC",  // Pop BC from stack
            0xD1 => "POP DE",  // Pop DE from stack
            0xE1 => "POP HL",  // Pop HL from stack
            0xF1 => "POP AF",  // Pop AF from stack

            // JR instructions
            0x20 => "JR NZ,n", // Relative jump if not zero
            0x28 => "JR Z,n",  // Relative jump if zero
            0x30 => "JR NC,n", // Relative jump if no carry
            0x38 => "JR C,n",  // Relative jump if carry
            0x18 => "JR n",    // Unconditional relative jump

            _ => "UNKNOWN",
        }
    }    pub fn step(&mut self) -> Result<CyclesType> {
        let pc = self.registers.get_pc();
        let opcode = self.fetch_byte()?;        // Basic CPU state logging for debugging
        self.instruction_count += 1;
        if self.instruction_count % 10000 == 0 {
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("logs/cpu_status.log")
            {
                writeln!(file, "CPU Status: PC=0x{:04X}, Instruction Count: {}", pc, self.instruction_count).ok();
            }
        }

        let cycles = match opcode {
            // NOP
            0x00 => Ok(CYCLES_1),            // DEC r
            0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x35 | 0x3D => {
                // Disabled logging for performance during long loops
                /*
                writeln!(
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("logs/cpu_exec.log")
                        .unwrap(),
                    "Executing DEC instruction: 0x{:02X}",
                    opcode
                ).ok();
                */
                self::instructions::arithmetic::dispatch(self, opcode)
            },            // LD instruction family
            0x01 | 0x11 | 0x21 | 0x31 | // LD rr,nn
            0x40..=0x7F |               // LD r,r'
            0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x3E | // LD r,n
            0x02 | 0x12 | 0x22 | 0x32 | // LD (rr),A
            0x0A | 0x1A | 0x2A | 0x3A | // LD A,(rr)
            0x36 |                      // LD (HL),n
            0x08 | 0xE0 | 0xE2 | 0xEA | 0xF0 | 0xF2 | 0xF8 | 0xF9 | 0xFA => {
                // Disabled logging for performance during long loops
                /*
                writeln!(
                    std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("logs/cpu_exec.log")
                        .unwrap(),
                    "Executing LD instruction: 0x{:02X}",
                    opcode
                ).ok();
                */
                self::instructions::load::dispatch(self, opcode)
            },

            // 邏輯運算指令（AND、OR、XOR、CP）
            0xA0..=0xA7 | 0xA8..=0xAF | 0xB0..=0xB7 | 0xB8..=0xBF | 0xE6 | 0xEE | 0xF6 | 0xFE => {
                self::instructions::logic::dispatch(self, opcode)
            },

            // JP nn（無條件跳轉）
            0xC3 => {
                let address = self.fetch_word()?;
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("logs/cpu_exec.log")
                {
                    writeln!(file, "無條件跳轉到 0x{:04X}", address).ok();
                }
                self.registers.pc = address;
                Ok(CYCLES_4)
            }

            // JP cc,nn（條件跳轉）
            0xC2 | 0xCA | 0xD2 | 0xDA => {
                let address = self.fetch_word()?;
                let condition = match opcode {
                    0xC2 => !self.registers.get_flag(Flag::Z),  // JP NZ,nn
                    0xCA => self.registers.get_flag(Flag::Z),   // JP Z,nn
                    0xD2 => !self.registers.get_flag(Flag::C),  // JP NC,nn
                    0xDA => self.registers.get_flag(Flag::C),   // JP C,nn
                    _ => unreachable!()
                };

                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("logs/cpu_exec.log")
                {
                    writeln!(
                        file,
                        "條件跳轉檢查: {}={}，目標地址=0x{:04X}",
                        Self::decode_opcode_name(opcode),
                        condition,
                        address
                    ).ok();
                }

                if condition {
                    self.registers.pc = address;
                    Ok(CYCLES_4)
                } else {
                    Ok(CYCLES_3)
                }
            }

            // JP HL
            0xE9 => {
                let address = self.registers.get_hl();
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("logs/cpu_exec.log")
                {
                    writeln!(file, "跳轉到 HL=0x{:04X}", address).ok();
                }
                self.registers.pc = address;
                Ok(CYCLES_1)
            }

            // CALL nn（無條件調用）
            0xCD => {
                let address = self.fetch_word()?;
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("logs/cpu_exec.log")
                {
                    writeln!(
                        file,
                        "調用子程序: PC=0x{:04X} -> 0x{:04X}",
                        self.registers.pc,
                        address
                    ).ok();
                }
                self.push_word(self.registers.pc)?;
                self.registers.pc = address;
                Ok(CYCLES_6)
            }

            // CALL cc,nn（條件調用）
            0xC4 | 0xCC | 0xD4 | 0xDC => {
                let address = self.fetch_word()?;
                let condition = match opcode {
                    0xC4 => !self.registers.get_flag(Flag::Z),  // CALL NZ,nn
                    0xCC => self.registers.get_flag(Flag::Z),   // CALL Z,nn
                    0xD4 => !self.registers.get_flag(Flag::C),  // CALL NC,nn
                    0xDC => self.registers.get_flag(Flag::C),   // CALL C,nn
                    _ => unreachable!()
                };

                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("logs/cpu_exec.log")
                {
                    writeln!(
                        file,
                        "條件調用檢查: {}={}，目標地址=0x{:04X}",
                        Self::decode_opcode_name(opcode),
                        condition,
                        address
                    ).ok();
                }

                if condition {
                    self.push_word(self.registers.pc)?;
                    self.registers.pc = address;
                    Ok(CYCLES_6)
                } else {
                    Ok(CYCLES_3)
                }
            }

            // RET（無條件返回）
            0xC9 => {
                let address = self.pop_word()?;
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("logs/cpu_exec.log")
                {
                    writeln!(file, "返回到 0x{:04X}", address).ok();
                }
                self.registers.pc = address;
                Ok(CYCLES_4)
            }

            // RET cc（條件返回）
            0xC0 | 0xC8 | 0xD0 | 0xD8 => {
                let condition = match opcode {
                    0xC0 => !self.registers.get_flag(Flag::Z),  // RET NZ
                    0xC8 => self.registers.get_flag(Flag::Z),   // RET Z
                    0xD0 => !self.registers.get_flag(Flag::C),  // RET NC
                    0xD8 => self.registers.get_flag(Flag::C),   // RET C
                    _ => unreachable!()
                };

                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("logs/cpu_exec.log")
                {
                    writeln!(
                        file,
                        "條件返回檢查: {}={}",
                        Self::decode_opcode_name(opcode),
                        condition
                    ).ok();
                }

                if condition {
                    let address = self.pop_word()?;
                    self.registers.pc = address;
                    Ok(CYCLES_5)
                } else {
                    Ok(CYCLES_2)
                }
            }            // RETI (Return from interrupt)
            0xD9 => {
                let address = self.pop_word()?;
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("logs/cpu_exec.log")
                {
                    writeln!(file, "Return from interrupt to 0x{:04X}", address).ok();
                }
                self.registers.pc = address;
                self.ime = true; // Enable interrupts
                Ok(CYCLES_4)
            }            // JR 指令族
            0x20 | 0x28 | 0x30 | 0x38 => {
                let offset = self.fetch_byte()? as i8;
                let condition = match opcode {
                    0x20 => !self.registers.get_flag(Flag::Z),     // JR NZ,n
                    0x28 => self.registers.get_flag(Flag::Z),      // JR Z,n
                    0x30 => !self.registers.get_flag(Flag::C),     // JR NC,n
                    0x38 => self.registers.get_flag(Flag::C),      // JR C,n
                    _ => unreachable!()
                };
                
                // Only log when condition changes or every 100 jumps to reduce log spam
                if !condition || self.instruction_count % 100 == 0 {
                    if let Ok(mut file) = std::fs::OpenOptions::new()
                        .create(true)
                        .append(true)
                        .open("logs/cpu_exec.log")
                    {
                        let target = ((self.registers.pc as i32) + (offset as i32)) as u16;
                        writeln!(
                            file,
                            "JR check: {}={}, offset={:+}, target=0x{:04X}, inst={}",
                            Self::decode_opcode_name(opcode),
                            condition,
                            offset,
                            target,
                            self.instruction_count
                        ).ok();
                    }
                }

                if condition {
                    self.registers.pc = ((self.registers.pc as i32) + (offset as i32)) as u16;
                    Ok(CYCLES_3)
                } else {
                    Ok(CYCLES_2)
                }
            }// JR n（無條件相對跳轉）
            0x18 => {
                let offset = self.fetch_byte()? as i8;
                let target = ((self.registers.pc as i32) + (offset as i32)) as u16;
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("logs/cpu_exec.log")
                {
                    writeln!(
                        file,
                        "無條件相對跳轉：偏移量={:+}，從 0x{:04X} 跳轉到 0x{:04X}",
                        offset,
                        self.registers.pc,
                        target
                    ).ok();
                }
                self.registers.pc = target;
                Ok(CYCLES_3)
            }            // DI (Disable Interrupts)
            0xF3 => {
                // Disabled logging for performance
                /*
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("logs/cpu_exec.log")
                {
                    writeln!(file, "Disable interrupts (DI)").ok();
                }
                */
                self.ime = false;
                Ok(CYCLES_1)
            }            // EI (Enable Interrupts)  
            0xFB => {
                // Disabled logging for performance
                /*
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("logs/cpu_exec.log")
                {
                    writeln!(file, "Enable interrupts (EI)").ok();
                }
                */
                self.ime_scheduled = true; // Enable after next instruction
                Ok(CYCLES_1)
            }

            // 其他指令將在後續添加
            _ => {
                if let Ok(mut file) = std::fs::OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open("logs/cpu_exec.log")
                {
                    writeln!(file, "未知指令: 0x{:02X} at PC=0x{:04X}", opcode, pc).ok();
                }
                Err(Error::Instruction(InstructionError::InvalidOpcode(opcode)))
            }        }?;

        // Handle interrupts after instruction execution
        if self.ime && !self.halted {
            if let Ok(interrupt_cycles) = self.handle_interrupts() {
                return Ok(cycles + interrupt_cycles);
            }
        }

        // Handle scheduled interrupt enable
        if self.ime_scheduled {
            self.ime = true;
            self.ime_scheduled = false;
        }

        Ok(cycles)
    }

    // Handle interrupts
    fn handle_interrupts(&mut self) -> Result<CyclesType> {
        let ie = self.read_byte(0xFFFF)?; // Interrupt Enable
        let if_reg = self.read_byte(0xFF0F)?; // Interrupt Flag
        
        let pending = ie & if_reg;
        if pending == 0 {
            return Ok(CYCLES_1);
        }

        // Check for V-Blank interrupt (bit 0)
        if pending & 0x01 != 0 {
            // Clear the interrupt flag
            self.write_byte(0xFF0F, if_reg & !0x01)?;
            
            // Disable interrupts
            self.ime = false;
            
            // Push PC onto stack and jump to interrupt vector
            self.push_word(self.registers.get_pc())?;
            self.registers.set_pc(0x0040); // V-Blank interrupt vector
            
            if let Ok(mut file) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("logs/interrupt.log")
            {
                writeln!(file, "V-Blank interrupt handled at PC=0x{:04X}", self.registers.get_pc()).ok();
            }
            
            return Ok(CYCLES_4);
        }
        
        // Handle other interrupts similarly...
        Ok(CYCLES_1)
    }
}
