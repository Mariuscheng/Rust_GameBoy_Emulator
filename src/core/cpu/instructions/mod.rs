pub mod arithmetic;
pub mod bit;
pub mod control;
pub mod cycles;
pub mod jump;
pub mod load;
pub mod logic;
pub mod prelude;
pub mod register_utils;

use crate::error::{Error, InstructionError, Result};

pub fn execute(
    cpu: &mut crate::core::cpu::CPU,
    opcode: u8,
) -> Result<crate::core::cycles::CyclesType> {
    // 非法操作碼判斷
    if [
        0xD3, 0xDB, 0xDD, 0xE3, 0xE4, 0xEB, 0xEC, 0xED, 0xF4, 0xFC, 0xFD,
    ]
    .contains(&opcode)
    {
        return Err(Error::Instruction(InstructionError::InvalidOpcode(opcode)));
    }

    match opcode {
        // NOP
        0x00 => Ok(4),

        // LD 指令
        0x08 => load::dispatch(cpu, opcode), // LD (nn),SP
        0x01 | 0x11 | 0x21 | 0x31 => load::dispatch(cpu, opcode), // LD rr,nn
        0x02 | 0x12 | 0x22 | 0x32 => load::dispatch(cpu, opcode), // LD (rr),A
        0x06 | 0x0E | 0x16 | 0x1E | 0x26 | 0x2E | 0x36 | 0x3E => load::dispatch(cpu, opcode), // LD r,n
        0x0A | 0x1A | 0x2A | 0x3A => load::dispatch(cpu, opcode), // LD A,(rr)
        0x40..=0x75 | 0x77..=0x7F => load::dispatch(cpu, opcode), // LD r,r

        // INC/DEC 指令
        0x03 | 0x13 | 0x23 | 0x33 => arithmetic::dispatch(cpu, opcode), // INC rr
        0x0B | 0x1B | 0x2B | 0x3B => arithmetic::dispatch(cpu, opcode), // DEC rr
        0x04 | 0x0C | 0x14 | 0x1C | 0x24 | 0x2C | 0x34 | 0x3C => arithmetic::dispatch(cpu, opcode), // INC r
        0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x35 | 0x3D => arithmetic::dispatch(cpu, opcode), // DEC r

        // 控制指令
        0x10 => control::dispatch(cpu, opcode), // STOP
        0x76 => control::dispatch(cpu, opcode), // HALT

        // 跳躍指令
        0x18 => jump::dispatch(cpu, opcode), // JR r8
        0x20 | 0x28 | 0x30 | 0x38 => jump::dispatch(cpu, opcode), // JR cc,r8
        0xC2 | 0xC3 | 0xCA | 0xD2 | 0xDA | 0xE9 => jump::dispatch(cpu, opcode), // JP

        // CALL 指令
        0xC4 | 0xCC | 0xCD | 0xD4 | 0xDC => jump::dispatch(cpu, opcode), // CALL

        // RET/RETI 指令
        0xC0 | 0xC8 | 0xC9 | 0xD0 | 0xD8 | 0xD9 => jump::dispatch(cpu, opcode), // RET/RETI

        // 邏輯運算指令
        0xA0..=0xA7 => logic::dispatch(cpu, opcode), // AND r
        0xE6 => logic::dispatch(cpu, opcode),        // AND n
        0xB0..=0xB7 => logic::dispatch(cpu, opcode), // OR r
        0xF6 => logic::dispatch(cpu, opcode),        // OR n
        0xA8..=0xAF => logic::dispatch(cpu, opcode), // XOR r
        0xEE => logic::dispatch(cpu, opcode),        // XOR n
        0xB8..=0xBF => logic::dispatch(cpu, opcode), // CP r
        0xFE => logic::dispatch(cpu, opcode),        // CP n

        // 未實現的指令
        _ => Err(Error::Instruction(InstructionError::InvalidOpcode(opcode))),
    }
}

// Re-export important components
pub use prelude::{FlagOperations, FlagUtils};
