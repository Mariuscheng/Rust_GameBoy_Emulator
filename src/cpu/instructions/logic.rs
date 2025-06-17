use super::common::{InstructionError, RegTarget, CYCLES_1, CYCLES_2};
use super::CPU;

pub fn dispatch(cpu: &mut CPU, opcode: u8) -> Result<u8, InstructionError> {
    match opcode {
        0xA0..=0xA7 => and_a_r(cpu, opcode & 0x07),
        0xE6 => and_a_n(cpu),
        0xA8..=0xAF => xor_a_r(cpu, opcode & 0x07),
        0xEE => xor_a_n(cpu),
        0xB0..=0xB7 => or_a_r(cpu, opcode & 0x07),
        0xF6 => or_a_n(cpu),
        0xB8..=0xBF => cp_a_r(cpu, opcode & 0x07),  // 新增: CP A,r 指令
        0xFE => cp_a_n(cpu),                         // 新增: CP A,n 指令
        _ => Err(InstructionError::InvalidOpcode(opcode)),
    }
}

fn and_a_r(cpu: &mut CPU, reg: u8) -> Result<u8, InstructionError> {
    let value = match reg {
        0 => cpu.registers.b,
        1 => cpu.registers.c,
        2 => cpu.registers.d,
        3 => cpu.registers.e,
        4 => cpu.registers.h,
        5 => cpu.registers.l,
        6 => cpu.read_byte(cpu.registers.get_hl())?,
        7 => cpu.registers.a,
        _ => return Err(InstructionError::InvalidRegister(RegTarget::A)),
    };

    let result = cpu.registers.a & value;

    cpu.registers.set_z_flag(result == 0);
    cpu.registers.set_n_flag(false);
    cpu.registers.set_h_flag(true);
    cpu.registers.set_c_flag(false);

    cpu.registers.a = result;
    Ok(if reg == 6 { CYCLES_2 } else { CYCLES_1 })
}

fn and_a_n(cpu: &mut CPU) -> Result<u8, InstructionError> {
    let value = cpu.read_byte(cpu.registers.pc)?;
    cpu.registers.pc = cpu.registers.pc.wrapping_add(1);

    let result = cpu.registers.a & value;

    cpu.registers.set_z_flag(result == 0);
    cpu.registers.set_n_flag(false);
    cpu.registers.set_h_flag(true);
    cpu.registers.set_c_flag(false);

    cpu.registers.a = result;
    Ok(CYCLES_2)
}

fn xor_a_r(cpu: &mut CPU, reg: u8) -> Result<u8, InstructionError> {
    let value = match reg {
        0 => cpu.registers.b,
        1 => cpu.registers.c,
        2 => cpu.registers.d,
        3 => cpu.registers.e,
        4 => cpu.registers.h,
        5 => cpu.registers.l,
        6 => cpu.read_byte(cpu.registers.get_hl())?,
        7 => cpu.registers.a,
        _ => return Err(InstructionError::InvalidRegister(RegTarget::A)),
    };

    let result = cpu.registers.a ^ value;

    cpu.registers.set_z_flag(result == 0);
    cpu.registers.set_n_flag(false);
    cpu.registers.set_h_flag(false);
    cpu.registers.set_c_flag(false);

    cpu.registers.a = result;
    Ok(if reg == 6 { CYCLES_2 } else { CYCLES_1 })
}

fn xor_a_n(cpu: &mut CPU) -> Result<u8, InstructionError> {
    let value = cpu.read_byte(cpu.registers.pc)?;
    cpu.registers.pc = cpu.registers.pc.wrapping_add(1);

    let result = cpu.registers.a ^ value;

    cpu.registers.set_z_flag(result == 0);
    cpu.registers.set_n_flag(false);
    cpu.registers.set_h_flag(false);
    cpu.registers.set_c_flag(false);

    cpu.registers.a = result;
    Ok(CYCLES_2)
}

fn or_a_r(cpu: &mut CPU, reg: u8) -> Result<u8, InstructionError> {
    let value = match reg {
        0 => cpu.registers.b,
        1 => cpu.registers.c,
        2 => cpu.registers.d,
        3 => cpu.registers.e,
        4 => cpu.registers.h,
        5 => cpu.registers.l,
        6 => cpu.read_byte(cpu.registers.get_hl())?,
        7 => cpu.registers.a,
        _ => return Err(InstructionError::InvalidRegister(RegTarget::A)),
    };

    let result = cpu.registers.a | value;

    cpu.registers.set_z_flag(result == 0);
    cpu.registers.set_n_flag(false);
    cpu.registers.set_h_flag(false);
    cpu.registers.set_c_flag(false);

    cpu.registers.a = result;
    Ok(if reg == 6 { CYCLES_2 } else { CYCLES_1 })
}

fn or_a_n(cpu: &mut CPU) -> Result<u8, InstructionError> {
    let value = cpu.read_byte(cpu.registers.pc)?;
    cpu.registers.pc = cpu.registers.pc.wrapping_add(1);

    let result = cpu.registers.a | value;

    cpu.registers.set_z_flag(result == 0);
    cpu.registers.set_n_flag(false);
    cpu.registers.set_h_flag(false);
    cpu.registers.set_c_flag(false);

    cpu.registers.a = result;
    Ok(CYCLES_2)
}

fn cp_a_r(cpu: &mut CPU, reg: u8) -> Result<u8, InstructionError> {
    let value = match reg {
        0 => cpu.registers.b,
        1 => cpu.registers.c,
        2 => cpu.registers.d,
        3 => cpu.registers.e,
        4 => cpu.registers.h,
        5 => cpu.registers.l,
        6 => cpu.read_byte(cpu.registers.get_hl())?,
        7 => cpu.registers.a,
        _ => return Err(InstructionError::InvalidRegister(RegTarget::A)),
    };

    let result = cpu.registers.a.wrapping_sub(value);
    let half_carry = (cpu.registers.a & 0xF).wrapping_sub(value & 0xF) > 0xF;

    cpu.registers.set_z_flag(result == 0);
    cpu.registers.set_n_flag(true);
    cpu.registers.set_h_flag(half_carry);
    cpu.registers.set_c_flag(cpu.registers.a < value);

    Ok(if reg == 6 { CYCLES_2 } else { CYCLES_1 })
}

fn cp_a_n(cpu: &mut CPU) -> Result<u8, InstructionError> {
    let value = cpu.read_byte(cpu.registers.pc)?;
    cpu.registers.pc = cpu.registers.pc.wrapping_add(1);

    let result = cpu.registers.a.wrapping_sub(value);
    let half_carry = (cpu.registers.a & 0xF).wrapping_sub(value & 0xF) > 0xF;

    cpu.registers.set_z_flag(result == 0);
    cpu.registers.set_n_flag(true);
    cpu.registers.set_h_flag(half_carry);
    cpu.registers.set_c_flag(cpu.registers.a < value);

    Ok(CYCLES_2)
}
