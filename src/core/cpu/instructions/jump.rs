#![allow(unused_variables)]
#![allow(dead_code)]

use crate::core::cpu::instructions::register_utils::FlagOperations;
use crate::core::cpu::CPU;
use crate::core::cycles::{CyclesType, CYCLES_1, CYCLES_2, CYCLES_3};
use crate::error::{Error, InstructionError, Result};
use std::io::Write;

pub fn dispatch(cpu: &mut CPU, opcode: u8) -> Result<CyclesType> {
    match opcode {
        // JP nn
        0xC3 => cpu.jp_nn(),

        // JP cc, nn
        0xC2 | 0xCA | 0xD2 | 0xDA => {
            let condition = (opcode >> 3) & 0x03;
            cpu.jp_cc_nn(condition)
        }

        // JP (HL)
        0xE9 => cpu.jp_hl(),

        // JR n
        0x18 => cpu.jr_n(),

        // JR cc, n
        0x20 | 0x28 | 0x30 | 0x38 => {
            let condition = (opcode >> 3) & 0x03;
            cpu.jr_cc_n(condition)
        }

        // CALL nn
        0xCD => cpu.call_nn(),

        // CALL cc,nn
        0xC4 | 0xCC | 0xD4 | 0xDC => {
            let condition = (opcode >> 3) & 0x03;
            cpu.call_cc_nn(condition)
        }

        // RET
        0xC9 => cpu.return_no_condition(),

        // RET cc
        0xC0 | 0xC8 | 0xD0 | 0xD8 => {
            let condition = (opcode >> 3) & 0x03;
            cpu.return_if_condition(condition)
        }

        // RETI
        0xD9 => cpu.return_and_enable_interrupts(),

        _ => Err(Error::Instruction(InstructionError::InvalidOpcode(opcode))),
    }
}

impl CPU {
    fn log_instruction(&mut self, instruction_name: &str, details: &str) {
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("logs/cpu_exec.log")
        {
            writeln!(
                file,
                "PC={:04X} | {} | {} | AF={:04X} BC={:04X} DE={:04X} HL={:04X}",
                self.registers.get_pc(),
                instruction_name,
                details,
                self.registers.get_af(),
                self.registers.get_bc(),
                self.registers.get_de(),
                self.registers.get_hl()
            )
            .ok();
        }
    }

    pub fn jp_nn(&mut self) -> Result<CyclesType> {
        let address = self.fetch_word()?;
        self.log_instruction("JP nn", &format!("跳轉到 0x{:04X}", address));
        self.registers.pc = address;
        Ok(CYCLES_3)
    }
    pub fn jp_cc_nn(&mut self, condition: u8) -> Result<CyclesType> {
        let address = self.fetch_word()?;
        let condition_name = match condition {
            0 => "NZ",
            1 => "Z",
            2 => "NC",
            3 => "C",
            _ => {
                return Err(Error::Instruction(InstructionError::InvalidCondition(
                    condition,
                )))
            }
        };

        let jump = match condition {
            0 => !self.registers.get_zero(),  // NZ
            1 => self.registers.get_zero(),   // Z
            2 => !self.registers.get_carry(), // NC
            3 => self.registers.get_carry(),  // C
            _ => unreachable!(),
        };

        self.log_instruction(
            &format!("JP {}, nn", condition_name),
            &format!(
                "目標=0x{:04X} ({}執行)",
                address,
                if jump { "已" } else { "未" }
            ),
        );

        if jump {
            self.registers.pc = address;
        }
        Ok(CYCLES_3)
    }

    pub fn jp_hl(&mut self) -> Result<CyclesType> {
        let target = self.registers.get_hl();
        self.log_instruction("JP (HL)", &format!("跳轉到 HL=0x{:04X}", target));
        self.registers.pc = target;
        Ok(CYCLES_1)
    }

    pub fn jr_n(&mut self) -> Result<CyclesType> {
        let offset = self.fetch_byte()? as i8;
        let target = ((self.registers.pc as i32) + (offset as i32)) as u16;
        self.log_instruction("JR n", &format!("相對跳轉 {} 到 0x{:04X}", offset, target));
        self.registers.pc = target;
        Ok(CYCLES_2)
    }

    pub fn jr_cc_n(&mut self, condition: u8) -> Result<CyclesType> {
        let offset = self.fetch_byte()? as i8;
        let condition_name = match condition {
            0 => "NZ",
            1 => "Z",
            2 => "NC",
            3 => "C",
            _ => {
                return Err(Error::Instruction(InstructionError::Custom(
                    "無效的條件碼".to_string(),
                )))
            }
        };

        let jump = match condition {
            0 => !self.registers.get_zero(),  // NZ
            1 => self.registers.get_zero(),   // Z
            2 => !self.registers.get_carry(), // NC
            3 => self.registers.get_carry(),  // C
            _ => unreachable!(),
        };

        let target = ((self.registers.pc as i32) + (offset as i32)) as u16;
        self.log_instruction(
            &format!("JR {}, n", condition_name),
            &format!(
                "目標=0x{:04X} ({}執行)",
                target,
                if jump { "已" } else { "未" }
            ),
        );

        if jump {
            self.registers.pc = target;
            Ok(CYCLES_2)
        } else {
            Ok(CYCLES_1)
        }
    }

    pub fn call_nn(&mut self) -> Result<CyclesType> {
        let address = self.fetch_word()?;
        let return_addr = self.registers.pc;
        self.push_word(return_addr)?;
        self.log_instruction(
            "CALL nn",
            &format!("調用 0x{:04X}, 返回地址=0x{:04X}", address, return_addr),
        );
        self.registers.pc = address;
        Ok(CYCLES_3 + CYCLES_3) // 6 cycles total
    }

    pub fn call_cc_nn(&mut self, condition: u8) -> Result<CyclesType> {
        let address = self.fetch_word()?;
        let condition_name = match condition {
            0 => "NZ",
            1 => "Z",
            2 => "NC",
            3 => "C",
            _ => {
                return Err(Error::Instruction(InstructionError::Custom(
                    "無效的條件碼".to_string(),
                )))
            }
        };

        let call = match condition {
            0 => !self.registers.get_zero(),  // NZ
            1 => self.registers.get_zero(),   // Z
            2 => !self.registers.get_carry(), // NC
            3 => self.registers.get_carry(),  // C
            _ => unreachable!(),
        };

        self.log_instruction(
            &format!("CALL {}, nn", condition_name),
            &format!(
                "目標=0x{:04X} ({}執行)",
                address,
                if call { "已" } else { "未" }
            ),
        );

        if call {
            let return_addr = self.registers.pc;
            self.push_word(return_addr)?;
            self.registers.pc = address;
            Ok(CYCLES_3 + CYCLES_3) // 6 cycles if taken
        } else {
            Ok(CYCLES_3) // 3 cycles if not taken
        }
    }

    pub fn return_no_condition(&mut self) -> Result<CyclesType> {
        let return_addr = self.pop_word()?;
        self.log_instruction("RET", &format!("返回到 0x{:04X}", return_addr));
        self.registers.pc = return_addr;
        Ok(CYCLES_2 + CYCLES_2) // 4 cycles total
    }

    pub fn return_if_condition(&mut self, condition: u8) -> Result<CyclesType> {
        let condition_name = match condition {
            0 => "NZ",
            1 => "Z",
            2 => "NC",
            3 => "C",
            _ => {
                return Err(Error::Instruction(InstructionError::Custom(
                    "無效的條件碼".to_string(),
                )))
            }
        };

        let ret = match condition {
            0 => !self.registers.get_zero(),  // NZ
            1 => self.registers.get_zero(),   // Z
            2 => !self.registers.get_carry(), // NC
            3 => self.registers.get_carry(),  // C
            _ => unreachable!(),
        };

        if ret {
            let return_addr = self.pop_word()?;
            self.log_instruction(
                &format!("RET {}", condition_name),
                &format!("返回到 0x{:04X}", return_addr),
            );
            self.registers.pc = return_addr;
            Ok(CYCLES_2 + CYCLES_2) // 4 cycles if taken
        } else {
            self.log_instruction(&format!("RET {}", condition_name), "條件不成立，繼續執行");
            Ok(CYCLES_2) // 2 cycles if not taken
        }
    }

    pub fn return_and_enable_interrupts(&mut self) -> Result<CyclesType> {
        let return_addr = self.pop_word()?;
        self.log_instruction("RETI", &format!("返回到 0x{:04X} 並啟用中斷", return_addr));
        self.registers.pc = return_addr;
        self.ime = true;
        Ok(CYCLES_2 + CYCLES_2) // 4 cycles total
    }
}
