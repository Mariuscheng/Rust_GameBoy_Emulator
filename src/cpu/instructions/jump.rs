#![allow(unused_variables)]
#![allow(dead_code)]

use super::common::{
    Condition, InstructionError, CYCLES_1, CYCLES_2, CYCLES_3, CYCLES_4, CYCLES_5, CYCLES_6,
};
use super::CPU;
use crate::cpu::registers::{CARRY_FLAG, ZERO_FLAG};

pub fn dispatch(cpu: &mut CPU, opcode: u8) -> Result<u8, InstructionError> {
    match opcode {
        // 無條件跳轉
        0x18 => jr_n(cpu),  // JR n
        0xC3 => jp_nn(cpu), // JP nn
        0xE9 => jp_hl(cpu), // JP (HL)

        // 條件相對跳轉
        0x20 => jr_cc_n(cpu, Condition::NZ), // JR NZ,n
        0x28 => jr_cc_n(cpu, Condition::Z),  // JR Z,n
        0x30 => jr_cc_n(cpu, Condition::NC), // JR NC,n
        0x38 => jr_cc_n(cpu, Condition::C),  // JR C,n

        // 條件絕對跳轉
        0xC2 => jp_cc_nn(cpu, Condition::NZ), // JP NZ,nn
        0xCA => jp_cc_nn(cpu, Condition::Z),  // JP Z,nn
        0xD2 => jp_cc_nn(cpu, Condition::NC), // JP NC,nn
        0xDA => jp_cc_nn(cpu, Condition::C),  // JP C,nn

        // 呼叫和返回
        0xC4 => call_cc_nn(cpu, Condition::NZ), // CALL NZ,nn
        0xCC => call_cc_nn(cpu, Condition::Z),  // CALL Z,nn
        0xD4 => call_cc_nn(cpu, Condition::NC), // CALL NC,nn
        0xDC => call_cc_nn(cpu, Condition::C),  // CALL C,nn
        0xCD => call_nn(cpu),                   // CALL nn
        0xC0 => ret_cc(cpu, Condition::NZ),     // RET NZ
        0xC8 => ret_cc(cpu, Condition::Z),      // RET Z
        0xD0 => ret_cc(cpu, Condition::NC),     // RET NC
        0xD8 => ret_cc(cpu, Condition::C),      // RET C
        0xC9 => ret(cpu),                       // RET
        0xD9 => reti(cpu),                      // RETI

        // RST 指令
        0xC7 => rst(cpu, 0x00), // RST 00H
        0xCF => rst(cpu, 0x08), // RST 08H
        0xD7 => rst(cpu, 0x10), // RST 10H
        0xDF => rst(cpu, 0x18), // RST 18H
        0xE7 => rst(cpu, 0x20), // RST 20H
        0xEF => rst(cpu, 0x28), // RST 28H
        0xF7 => rst(cpu, 0x30), // RST 30H
        0xFF => rst(cpu, 0x38), // RST 38H

        _ => Err(InstructionError::InvalidOpcode(opcode)),
    }
}

fn check_condition(cpu: &CPU, condition: Condition) -> bool {
    match condition {
        Condition::Z => (cpu.registers.f & ZERO_FLAG) != 0,
        Condition::NZ => (cpu.registers.f & ZERO_FLAG) == 0,
        Condition::C => (cpu.registers.f & CARRY_FLAG) != 0,
        Condition::NC => (cpu.registers.f & CARRY_FLAG) == 0,
    }
}

// 相對跳轉指令
fn jr_n(cpu: &mut CPU) -> Result<u8, InstructionError> {
    let offset = cpu.fetch_byte()? as i8;
    let pc = cpu.registers.pc;
    cpu.registers.pc = pc.wrapping_add(offset as u16);
    Ok(CYCLES_3)
}

fn jr_cc_n(cpu: &mut CPU, condition: Condition) -> Result<u8, InstructionError> {
    let offset = cpu.fetch_byte()? as i8;
    if check_condition(cpu, condition) {
        let pc = cpu.registers.pc;
        cpu.registers.pc = pc.wrapping_add(offset as u16);
        Ok(CYCLES_3)
    } else {
        Ok(CYCLES_2)
    }
}

// 絕對跳轉指令
fn jp_nn(cpu: &mut CPU) -> Result<u8, InstructionError> {
    let low = cpu.fetch_byte()?;
    let high = cpu.fetch_byte()?;
    let address = ((high as u16) << 8) | (low as u16);
    cpu.registers.pc = address;
    Ok(CYCLES_3) // JP nn 指令需要 16 個機器週期 = 3 個 M-cycle
}

fn jp_hl(cpu: &mut CPU) -> Result<u8, InstructionError> {
    cpu.registers.pc = cpu.get_hl();
    Ok(CYCLES_1)
}

fn jp_cc_nn(cpu: &mut CPU, condition: Condition) -> Result<u8, InstructionError> {
    let addr = cpu.fetch_word()?;
    if check_condition(cpu, condition) {
        cpu.registers.pc = addr;
        Ok(CYCLES_4)
    } else {
        Ok(CYCLES_3)
    }
}

// 呼叫指令
fn call_nn(cpu: &mut CPU) -> Result<u8, InstructionError> {
    let addr = cpu.fetch_word()?;
    let pc = cpu.registers.pc;
    cpu.registers.sp = cpu.registers.sp.wrapping_sub(2);
    cpu.write_word(cpu.registers.sp, pc)?;
    cpu.registers.pc = addr;
    Ok(CYCLES_6)
}

fn call_cc_nn(cpu: &mut CPU, condition: Condition) -> Result<u8, InstructionError> {
    let addr = cpu.fetch_word()?;
    if check_condition(cpu, condition) {
        let pc = cpu.registers.pc;
        cpu.registers.sp = cpu.registers.sp.wrapping_sub(2);
        cpu.write_word(cpu.registers.sp, pc)?;
        cpu.registers.pc = addr;
        Ok(CYCLES_6)
    } else {
        Ok(CYCLES_3)
    }
}

// 返回指令
fn ret(cpu: &mut CPU) -> Result<u8, InstructionError> {
    let addr = cpu.read_word(cpu.registers.sp)?;
    cpu.registers.sp = cpu.registers.sp.wrapping_add(2);
    cpu.registers.pc = addr;
    Ok(CYCLES_4)
}

fn ret_cc(cpu: &mut CPU, condition: Condition) -> Result<u8, InstructionError> {
    if check_condition(cpu, condition) {
        let addr = cpu.read_word(cpu.registers.sp)?;
        cpu.registers.sp = cpu.registers.sp.wrapping_add(2);
        cpu.registers.pc = addr;
        Ok(CYCLES_5)
    } else {
        Ok(CYCLES_2)
    }
}

fn reti(cpu: &mut CPU) -> Result<u8, InstructionError> {
    let addr = cpu.read_word(cpu.registers.sp)?;
    cpu.registers.sp = cpu.registers.sp.wrapping_add(2);
    cpu.registers.pc = addr;
    cpu.enable_interrupts();
    Ok(CYCLES_4)
}

// RST n - 呼叫特定的向量位址
fn rst(cpu: &mut CPU, vector: u8) -> Result<u8, InstructionError> {
    let pc = cpu.registers.pc;
    cpu.registers.sp = cpu.registers.sp.wrapping_sub(2);
    cpu.write_word(cpu.registers.sp, pc)?;
    cpu.registers.pc = vector as u16;
    Ok(CYCLES_4) // RST 指令需要 16 個機器週期 = 4 個 M-cycle
}
