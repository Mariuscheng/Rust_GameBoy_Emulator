use crate::cpu::CPU;

mod arithmetic;
pub mod bit;
pub mod common;
mod control;
mod jump;
mod load;
mod logic;

use common::InstructionError;
pub type Result<T> = std::result::Result<T, InstructionError>;

pub fn dispatch(cpu: &mut CPU, opcode: u8) -> Result<u8> {
    match opcode {
        // 控制指令
        0x00 => control::dispatch(cpu, opcode), // NOP
        0x10 => control::dispatch(cpu, opcode), // STOP
        0x76 => control::dispatch(cpu, opcode), // HALT
        0xF3 => control::dispatch(cpu, opcode), // DI
        0xFB => control::dispatch(cpu, opcode), // EI

        // 條件分支指令
        0x20 => jump::dispatch(cpu, opcode), // JR NZ,n
        0x28 => jump::dispatch(cpu, opcode), // JR Z,n
        0x30 => jump::dispatch(cpu, opcode), // JR NC,n
        0x38 => jump::dispatch(cpu, opcode), // JR C,n
        0xC2 | 0xCA | 0xD2 | 0xDA => jump::dispatch(cpu, opcode), // JP cc,nn
        0xC3 => jump::dispatch(cpu, opcode), // JP nn
        0xE9 => jump::dispatch(cpu, opcode), // JP HL

        // 子程式呼叫與返回
        0xC4 | 0xCC | 0xD4 | 0xDC => jump::dispatch(cpu, opcode), // CALL cc,nn
        0xCD => jump::dispatch(cpu, opcode),                      // CALL nn
        0xC0 | 0xC8 | 0xD0 | 0xD8 => jump::dispatch(cpu, opcode), // RET cc
        0xC9 => jump::dispatch(cpu, opcode),                      // RET
        0xD9 => jump::dispatch(cpu, opcode),                      // RETI

        // 8-bit 載入指令
        0x40..=0x7F => load::dispatch(cpu, opcode), // LD r,r 指令
        0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x36 | 0x3E => load::dispatch(cpu, opcode), // LD r,n

        // 間接載入指令
        0x02 | 0x12 | 0x22 | 0x32 => load::dispatch(cpu, opcode), // LD (rr),A
        0x0A | 0x1A | 0x2A | 0x3A => load::dispatch(cpu, opcode), // LD A,(rr)
        0xE0 | 0xF0 | 0xE2 | 0xF2 => load::dispatch(cpu, opcode), // LDH (n)/A,A/(C)
        0xEA | 0xFA => load::dispatch(cpu, opcode),               // LD (nn)/A,A

        // 16-bit 載入指令
        0x01 | 0x11 | 0x21 | 0x31 => load::dispatch(cpu, opcode), // LD rp,nn
        0xC1 | 0xD1 | 0xE1 | 0xF1 => load::dispatch(cpu, opcode), // POP rp
        0xC5 | 0xD5 | 0xE5 | 0xF5 => load::dispatch(cpu, opcode), // PUSH rp
        0x08 => load::dispatch(cpu, opcode),                      // LD (nn),SP
        0xF8 | 0xF9 => load::dispatch(cpu, opcode),               // LD HL,SP+r8 / LD SP,HL

        // 8-bit 算術與邏輯指令
        0x04 | 0x0C | 0x14 | 0x1C | 0x24 | 0x2C | 0x34 | 0x3C => arithmetic::dispatch(cpu, opcode), // INC r8/(HL)
        0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x35 | 0x3D => arithmetic::dispatch(cpu, opcode), // DEC r8/(HL)
        0x80..=0x87 => arithmetic::dispatch(cpu, opcode), // ADD A,r
        0x88..=0x8F => arithmetic::dispatch(cpu, opcode), // ADC A,r
        0x90..=0x97 => arithmetic::dispatch(cpu, opcode), // SUB A,r
        0x98..=0x9F => arithmetic::dispatch(cpu, opcode), // SBC A,r
        0xA0..=0xA7 => logic::dispatch(cpu, opcode),      // AND A,r
        0xA8..=0xAF => logic::dispatch(cpu, opcode),      // XOR A,r
        0xB0..=0xB7 => logic::dispatch(cpu, opcode),      // OR A,r
        0xB8..=0xBF => logic::dispatch(cpu, opcode),      // CP A,r
        0xE6 => logic::dispatch(cpu, opcode),             // AND n
        0xEE => logic::dispatch(cpu, opcode),             // XOR n
        0xF6 => logic::dispatch(cpu, opcode),             // OR n
        0xFE => logic::dispatch(cpu, opcode),             // CP n

        // 位操作指令
        0x27 => arithmetic::dispatch(cpu, opcode), // DAA
        0x2F => logic::dispatch(cpu, opcode),      // CPL
        0x37 => logic::dispatch(cpu, opcode),      // SCF
        0x3F => logic::dispatch(cpu, opcode),      // CCF
        0xCB => {
            let cb_opcode = cpu.fetch_byte()?;
            bit::dispatch(cpu, cb_opcode)
        }

        // 未知的操作碼
        _ => {
            cpu.logger.borrow_mut().error(&format!(
                "未知的操作碼: 0x{:02X} at PC={:04X}",
                opcode, cpu.registers.pc
            ));
            Err(InstructionError::InvalidOpcode(opcode))
        }
    }
}
