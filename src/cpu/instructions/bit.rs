use super::common::{InstructionError, RegTarget, CYCLES_2, CYCLES_4};
use super::CPU;

pub fn dispatch(cpu: &mut CPU, cb_opcode: u8) -> Result<u8, InstructionError> {
    match cb_opcode {
        // 位元操作
        0x40..=0x7F => bit_operations(cpu, cb_opcode), // BIT
        0xC0..=0xFF => set_operations(cpu, cb_opcode), // SET
        0x80..=0xBF => res_operations(cpu, cb_opcode), // RES
        // 旋轉與位移操作
        0x00..=0x3F => rotate_shift_operations(cpu, cb_opcode),
    }
}

/// 獲取寄存器值的輔助函數
fn get_register_value(cpu: &mut CPU, reg_index: u8) -> Result<u8, InstructionError> {
    match reg_index {
        0 => Ok(cpu.registers.b),
        1 => Ok(cpu.registers.c),
        2 => Ok(cpu.registers.d),
        3 => Ok(cpu.registers.e),
        4 => Ok(cpu.registers.h),
        5 => Ok(cpu.registers.l),
        6 => cpu.read_byte(cpu.registers.get_hl()).map_err(Into::into), // 正確處理 Result
        7 => Ok(cpu.registers.a),
        _ => Err(InstructionError::InvalidRegister(RegTarget::A)),
    }
}

/// 設置寄存器值的輔助函數
fn set_register_value(cpu: &mut CPU, reg_index: u8, value: u8) -> Result<(), InstructionError> {
    match reg_index {
        0 => cpu.registers.b = value,
        1 => cpu.registers.c = value,
        2 => cpu.registers.d = value,
        3 => cpu.registers.e = value,
        4 => cpu.registers.h = value,
        5 => cpu.registers.l = value,
        6 => cpu
            .write_byte(cpu.registers.get_hl(), value)
            .map_err(|e| InstructionError::MemoryError(e.to_string()))?,
        7 => cpu.registers.a = value,
        _ => return Err(InstructionError::InvalidRegister(RegTarget::A)),
    }
    Ok(())
}

// BIT b,r 指令的實作
fn bit_operations(cpu: &mut CPU, opcode: u8) -> Result<u8, InstructionError> {
    let bit = (opcode >> 3) & 0x07;
    let reg = opcode & 0x07;
    let reg_val = get_register_value(cpu, reg)?;

    let result = reg_val & (1 << bit);
    cpu.registers.f = (cpu.registers.f & 0x10) | 0x20; // Set H, preserve C
    if result == 0 {
        cpu.registers.f |= 0x80; // Set Z
    }

    Ok(if reg == 6 { CYCLES_4 } else { CYCLES_2 })
}

// SET b,r 指令的實作
fn set_operations(cpu: &mut CPU, opcode: u8) -> Result<u8, InstructionError> {
    let bit = (opcode >> 3) & 0x07;
    let reg = opcode & 0x07;
    let reg_val = get_register_value(cpu, reg)?;
    let new_val = reg_val | (1 << bit);
    set_register_value(cpu, reg, new_val)?;

    Ok(if reg == 6 { CYCLES_4 } else { CYCLES_2 })
}

// RES b,r 指令的實作
fn res_operations(cpu: &mut CPU, opcode: u8) -> Result<u8, InstructionError> {
    let bit = (opcode >> 3) & 0x07;
    let reg = opcode & 0x07;
    let reg_val = get_register_value(cpu, reg)?;
    let new_val = reg_val & !(1 << bit);
    set_register_value(cpu, reg, new_val)?;

    Ok(if reg == 6 { CYCLES_4 } else { CYCLES_2 })
}

// 旋轉與位移操作的實作
fn rotate_shift_operations(cpu: &mut CPU, opcode: u8) -> Result<u8, InstructionError> {
    let reg = opcode & 0x07;
    let reg_val = get_register_value(cpu, reg)?;
    let new_val = match opcode & 0xF8 {
        0x00 => rotate_left(reg_val, cpu, true),   // RLC
        0x08 => rotate_right(reg_val, cpu, true),  // RRC
        0x10 => rotate_left(reg_val, cpu, false),  // RL
        0x18 => rotate_right(reg_val, cpu, false), // RR
        0x20 => shift_left(reg_val, cpu),          // SLA
        0x28 => shift_right(reg_val, cpu, false),  // SRA
        0x30 => swap_nibbles(reg_val, cpu),        // SWAP
        0x38 => shift_right(reg_val, cpu, true),   // SRL
        _ => return Err(InstructionError::InvalidOpcode(opcode)),
    };

    set_register_value(cpu, reg, new_val)?;
    Ok(if reg == 6 { CYCLES_4 } else { CYCLES_2 })
}

// 輔助函數：向左旋轉
fn rotate_left(value: u8, cpu: &mut CPU, through_carry: bool) -> u8 {
    let old_carry = (cpu.registers.f & 0x10) != 0;
    let new_carry = (value & 0x80) != 0;

    let result = if through_carry {
        (value << 1) | (if new_carry { 1 } else { 0 })
    } else {
        (value << 1) | (if old_carry { 1 } else { 0 })
    };

    cpu.registers.f = 0;
    if new_carry {
        cpu.registers.f |= 0x10;
    }
    if result == 0 {
        cpu.registers.f |= 0x80;
    }

    result
}

// 輔助函數：向右旋轉
fn rotate_right(value: u8, cpu: &mut CPU, through_carry: bool) -> u8 {
    let old_carry = (cpu.registers.f & 0x10) != 0;
    let new_carry = (value & 0x01) != 0;

    let result = if through_carry {
        (value >> 1) | (if new_carry { 0x80 } else { 0 })
    } else {
        (value >> 1) | (if old_carry { 0x80 } else { 0 })
    };

    cpu.registers.f = 0;
    if new_carry {
        cpu.registers.f |= 0x10;
    }
    if result == 0 {
        cpu.registers.f |= 0x80;
    }

    result
}

// 輔助函數：算術左移
fn shift_left(value: u8, cpu: &mut CPU) -> u8 {
    let new_carry = (value & 0x80) != 0;
    let result = value << 1;

    cpu.registers.f = 0;
    if new_carry {
        cpu.registers.f |= 0x10;
    }
    if result == 0 {
        cpu.registers.f |= 0x80;
    }

    result
}

// 輔助函數：算術/邏輯右移
fn shift_right(value: u8, cpu: &mut CPU, logical: bool) -> u8 {
    let msb = value & 0x80;
    let new_carry = (value & 0x01) != 0;
    let result = if logical {
        value >> 1
    } else {
        (value >> 1) | msb
    };

    cpu.registers.f = 0;
    if new_carry {
        cpu.registers.f |= 0x10;
    }
    if result == 0 {
        cpu.registers.f |= 0x80;
    }

    result
}

// 輔助函數：交換半位元組
fn swap_nibbles(value: u8, cpu: &mut CPU) -> u8 {
    let result = ((value & 0x0F) << 4) | ((value & 0xF0) >> 4);

    cpu.registers.f = 0;
    if result == 0 {
        cpu.registers.f |= 0x80;
    }

    result
}
