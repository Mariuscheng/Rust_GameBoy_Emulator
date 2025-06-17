use super::common::FlagOperations;
use super::common::{InstructionError, RegTarget};
use super::CPU;

pub type Result<T> = std::result::Result<T, InstructionError>;

/// 處理所有算術指令的分派
pub fn dispatch(cpu: &mut CPU, opcode: u8) -> Result<u8> {
    match opcode {
        // 8-bit 遞增/遞減
        0x04 => inc_r(cpu, RegTarget::B), // INC B
        0x05 => dec_r(cpu, RegTarget::B), // DEC B
        0x0C => inc_r(cpu, RegTarget::C), // INC C
        0x0D => dec_r(cpu, RegTarget::C), // DEC C
        0x14 => inc_r(cpu, RegTarget::D), // INC D
        0x15 => dec_r(cpu, RegTarget::D), // DEC D
        0x1C => inc_r(cpu, RegTarget::E), // INC E
        0x1D => dec_r(cpu, RegTarget::E), // DEC E
        0x24 => inc_r(cpu, RegTarget::H), // INC H
        0x25 => dec_r(cpu, RegTarget::H), // DEC H
        0x2C => inc_r(cpu, RegTarget::L), // INC L
        0x2D => dec_r(cpu, RegTarget::L), // DEC L
        0x3C => inc_r(cpu, RegTarget::A), // INC A
        0x3D => dec_r(cpu, RegTarget::A), // DEC A
        0x34 => inc_hl_mem(cpu),          // INC (HL)
        0x35 => dec_hl_mem(cpu),          // DEC (HL)

        // 加法操作
        0x80..=0x87 => add_a_r(cpu, get_reg_from_opcode(opcode & 0x07)?), // ADD A,r
        0x88..=0x8F => adc_a_r(cpu, get_reg_from_opcode(opcode & 0x07)?), // ADC A,r
        0xC6 => add_a_n(cpu),                                             // ADD A,n
        0xCE => adc_a_n(cpu),                                             // ADC A,n

        // 減法操作
        0x90..=0x97 => sub_a_r(cpu, get_reg_from_opcode(opcode & 0x07)?), // SUB A,r
        0x98..=0x9F => sbc_a_r(cpu, get_reg_from_opcode(opcode & 0x07)?), // SBC A,r
        0xD6 => sub_a_n(cpu),                                             // SUB A,n
        0xDE => sbc_a_n(cpu),                                             // SBC A,n

        // 比較操作
        0xB8..=0xBF => cp_r(cpu, get_reg_from_opcode(opcode & 0x07)?), // CP A,r
        0xFE => cp_n(cpu),                                             // CP n

        // 16-bit 操作
        0x03 | 0x13 | 0x23 | 0x33 => inc_rr(cpu, opcode), // INC rr
        0x0B | 0x1B | 0x2B | 0x3B => dec_rr(cpu, opcode), // DEC rr
        0x09 | 0x19 | 0x29 | 0x39 => add_hl_rr(cpu, opcode), // ADD HL,rr
        0xE8 => add_sp_n(cpu),                            // ADD SP,n

        // 16-bit 載入
        0x27 => daa(cpu), // DAA

        _ => Err(InstructionError::InvalidOpcode(opcode)),
    }
}

// -- 實用函數 --

/// 從操作碼獲取寄存器目標
fn get_reg_from_opcode(reg_code: u8) -> Result<RegTarget> {
    match reg_code {
        0 => Ok(RegTarget::B),
        1 => Ok(RegTarget::C),
        2 => Ok(RegTarget::D),
        3 => Ok(RegTarget::E),
        4 => Ok(RegTarget::H),
        5 => Ok(RegTarget::L),
        6 => Ok(RegTarget::HL),
        7 => Ok(RegTarget::A),
        _ => Err(InstructionError::Custom("無效的寄存器代碼".to_string())),
    }
}

/// 獲取寄存器值
pub fn get_reg_value(cpu: &CPU, target: RegTarget) -> Result<u8> {
    match target {
        RegTarget::A => Ok(cpu.registers.a),
        RegTarget::B => Ok(cpu.registers.b),
        RegTarget::C => Ok(cpu.registers.c),
        RegTarget::D => Ok(cpu.registers.d),
        RegTarget::E => Ok(cpu.registers.e),
        RegTarget::H => Ok(cpu.registers.h),
        RegTarget::L => Ok(cpu.registers.l),
        RegTarget::HL => Ok(cpu.read_byte(cpu.registers.get_hl())?),
    }
}

/// 設置寄存器值
pub fn set_reg_value(cpu: &mut CPU, target: RegTarget, value: u8) -> Result<()> {
    match target {
        RegTarget::A => {
            cpu.registers.a = value;
            Ok(())
        }
        RegTarget::B => {
            cpu.registers.b = value;
            Ok(())
        }
        RegTarget::C => {
            cpu.registers.c = value;
            Ok(())
        }
        RegTarget::D => {
            cpu.registers.d = value;
            Ok(())
        }
        RegTarget::E => {
            cpu.registers.e = value;
            Ok(())
        }
        RegTarget::H => {
            cpu.registers.h = value;
            Ok(())
        }
        RegTarget::L => {
            cpu.registers.l = value;
            Ok(())
        }
        RegTarget::HL => Ok(cpu.write_byte(cpu.registers.get_hl(), value)?),
    }
}

// -- 基本算術函數 --

/// 加法核心邏輯
fn add_core(cpu: &mut CPU, value: u8, with_carry: bool) -> u8 {
    let a = cpu.registers.a;
    let carry = if with_carry && cpu.registers.get_carry_flag() {
        1
    } else {
        0
    };
    let result = a.wrapping_add(value).wrapping_add(carry);

    cpu.registers.update_flags(
        Some(result == 0),                                           // Zero flag
        Some(false),                                                 // Subtract flag
        Some((a & 0x0F) + (value & 0x0F) + carry > 0x0F),            // Half carry flag
        Some(((a as u16) + (value as u16) + (carry as u16)) > 0xFF), // Carry flag
    );

    result
}

/// 減法核心邏輯
fn sub_core(cpu: &mut CPU, value: u8, with_carry: bool) -> u8 {
    let a = cpu.registers.a;
    let carry = if with_carry && cpu.registers.get_carry_flag() {
        1
    } else {
        0
    };
    let result = a.wrapping_sub(value).wrapping_sub(carry);

    cpu.registers.update_flags(
        Some(result == 0),                                  // Zero flag
        Some(true),                                         // Subtract flag
        Some((a & 0x0F) < (value & 0x0F) + carry),          // Half carry flag
        Some((a as u16) < (value as u16) + (carry as u16)), // Carry flag
    );

    result
}

/// 比較核心邏輯
fn cp_core(cpu: &mut CPU, value: u8) {
    let a = cpu.registers.a;
    let result = a.wrapping_sub(value);

    cpu.registers.update_flags(
        Some(result == 0),                 // Zero flag
        Some(true),                        // Subtract flag
        Some((a & 0x0F) < (value & 0x0F)), // Half carry flag
        Some(a < value),                   // Carry flag
    );
}

// -- 8-bit 算術指令 --

/// 加法 A += r
fn add_a_r(cpu: &mut CPU, target: RegTarget) -> Result<u8> {
    let value = get_reg_value(cpu, target)?;
    cpu.registers.a = add_core(cpu, value, false);
    Ok(if target == RegTarget::HL { 8 } else { 4 })
}

/// 帶進位加法 A += r + carry
fn adc_a_r(cpu: &mut CPU, target: RegTarget) -> Result<u8> {
    let value = get_reg_value(cpu, target)?;
    cpu.registers.a = add_core(cpu, value, true);
    Ok(if target == RegTarget::HL { 8 } else { 4 })
}

/// 加法 A += n
fn add_a_n(cpu: &mut CPU) -> Result<u8> {
    let value = cpu.fetch_byte()?;
    cpu.registers.a = add_core(cpu, value, false);
    Ok(8)
}

/// 帶進位加法 A += n + carry
fn adc_a_n(cpu: &mut CPU) -> Result<u8> {
    let value = cpu.fetch_byte()?;
    cpu.registers.a = add_core(cpu, value, true);
    Ok(8)
}

/// 減法 A -= r
fn sub_a_r(cpu: &mut CPU, target: RegTarget) -> Result<u8> {
    let value = get_reg_value(cpu, target)?;
    cpu.registers.a = sub_core(cpu, value, false);
    Ok(if target == RegTarget::HL { 8 } else { 4 })
}

/// 帶借位減法 A -= r + carry
fn sbc_a_r(cpu: &mut CPU, target: RegTarget) -> Result<u8> {
    let value = get_reg_value(cpu, target)?;
    cpu.registers.a = sub_core(cpu, value, true);
    Ok(if target == RegTarget::HL { 8 } else { 4 })
}

/// 減法 A -= n
fn sub_a_n(cpu: &mut CPU) -> Result<u8> {
    let value = cpu.fetch_byte()?;
    cpu.registers.a = sub_core(cpu, value, false);
    Ok(8)
}

/// 帶借位減法 A -= n + carry
fn sbc_a_n(cpu: &mut CPU) -> Result<u8> {
    let value = cpu.fetch_byte()?;
    cpu.registers.a = sub_core(cpu, value, true);
    Ok(8)
}

/// 比較 A - r
fn cp_r(cpu: &mut CPU, target: RegTarget) -> Result<u8> {
    let value = get_reg_value(cpu, target)?;
    cp_core(cpu, value);
    Ok(if target == RegTarget::HL { 8 } else { 4 })
}

/// 比較 A - n
fn cp_n(cpu: &mut CPU) -> Result<u8> {
    let value = cpu.fetch_byte()?;
    cp_core(cpu, value);
    Ok(8)
}

/// 遞增一個寄存器的值
fn inc_r(cpu: &mut CPU, target: RegTarget) -> Result<u8> {
    let value = get_reg_value(cpu, target)?;
    let result = value.wrapping_add(1);

    cpu.registers.update_flags(
        Some(result == 0),          // Zero flag
        Some(false),                // Subtract flag
        Some(value & 0x0F == 0x0F), // Half carry flag
        None,                       // Carry flag unchanged
    );

    set_reg_value(cpu, target, result)?;
    Ok(if target == RegTarget::HL { 12 } else { 4 })
}

/// 遞減一個寄存器的值
fn dec_r(cpu: &mut CPU, target: RegTarget) -> Result<u8> {
    let value = get_reg_value(cpu, target)?;
    let result = value.wrapping_sub(1);

    cpu.registers.update_flags(
        Some(result == 0),          // Zero flag
        Some(true),                 // Subtract flag
        Some(value & 0x0F == 0x00), // Half carry flag
        None,                       // Carry flag unchanged
    );

    set_reg_value(cpu, target, result)?;
    Ok(if target == RegTarget::HL { 12 } else { 4 })
}

/// 遞增 (HL) 位址的值
fn inc_hl_mem(cpu: &mut CPU) -> Result<u8> {
    let addr = cpu.registers.get_hl();
    let value = cpu.read_byte(addr)?;
    let result = value.wrapping_add(1);

    cpu.registers.update_flags(
        Some(result == 0),            // Zero flag
        Some(false),                  // Subtract flag
        Some((value & 0x0F) == 0x0F), // Half carry flag
        None,                         // Carry flag unchanged
    );

    cpu.write_byte(addr, result)?;
    Ok(12)
}

/// 遞減 (HL) 位址的值
fn dec_hl_mem(cpu: &mut CPU) -> Result<u8> {
    let addr = cpu.registers.get_hl();
    let value = cpu.read_byte(addr)?;
    let result = value.wrapping_sub(1);

    cpu.registers.update_flags(
        Some(result == 0),         // Zero flag
        Some(true),                // Subtract flag
        Some((value & 0x0F) == 0), // Half carry flag
        None,                      // Carry flag unchanged
    );

    cpu.write_byte(addr, result)?;
    Ok(12)
}

// -- 16-bit 算術指令 --

/// 16-bit 加法 HL += rr
fn add_hl_rr(cpu: &mut CPU, opcode: u8) -> Result<u8> {
    let hl = cpu.registers.get_hl();
    let rr = match opcode {
        0x09 => cpu.registers.get_bc(),
        0x19 => cpu.registers.get_de(),
        0x29 => cpu.registers.get_hl(),
        0x39 => cpu.registers.get_sp(),
        _ => {
            return Err(InstructionError::Custom(
                "無效的 ADD HL,rr 操作碼".to_string(),
            ))
        }
    };

    let result = hl.wrapping_add(rr);
    let half_carry = (hl & 0x0FFF) + (rr & 0x0FFF) > 0x0FFF;
    let carry = (hl as u32) + (rr as u32) > 0xFFFF;

    cpu.registers.update_flags(
        None,             // Zero flag unchanged
        Some(false),      // Subtract flag
        Some(half_carry), // Half carry flag
        Some(carry),      // Carry flag
    );

    cpu.registers.set_hl(result);
    Ok(8)
}

/// 16-bit 遞增 INC rr
fn inc_rr(cpu: &mut CPU, opcode: u8) -> Result<u8> {
    match opcode {
        0x03 => cpu.registers.set_bc(cpu.registers.get_bc().wrapping_add(1)),
        0x13 => cpu.registers.set_de(cpu.registers.get_de().wrapping_add(1)),
        0x23 => cpu.registers.set_hl(cpu.registers.get_hl().wrapping_add(1)),
        0x33 => cpu.registers.sp = cpu.registers.sp.wrapping_add(1),
        _ => return Err(InstructionError::Custom("無效的 INC rr 操作碼".to_string())),
    }
    Ok(8)
}

/// 16-bit 遞減 DEC rr
fn dec_rr(cpu: &mut CPU, opcode: u8) -> Result<u8> {
    match opcode {
        0x0B => cpu.registers.set_bc(cpu.registers.get_bc().wrapping_sub(1)),
        0x1B => cpu.registers.set_de(cpu.registers.get_de().wrapping_sub(1)),
        0x2B => cpu.registers.set_hl(cpu.registers.get_hl().wrapping_sub(1)),
        0x3B => cpu.registers.sp = cpu.registers.sp.wrapping_sub(1),
        _ => return Err(InstructionError::Custom("無效的 DEC rr 操作碼".to_string())),
    }
    Ok(8)
}

/// ADD SP,n
fn add_sp_n(cpu: &mut CPU) -> Result<u8> {
    let n = cpu.fetch_byte()? as i8 as i16 as u16;
    let sp = cpu.registers.sp;
    let result = sp.wrapping_add(n);

    cpu.registers.update_flags(
        Some(false),                           // Zero flag
        Some(false),                           // Subtract flag
        Some((sp & 0x0F) + (n & 0x0F) > 0x0F), // Half carry flag
        Some((sp & 0xFF) + (n & 0xFF) > 0xFF), // Carry flag
    );

    cpu.registers.sp = result;
    Ok(16)
}

/// DAA (十進制調整 A 寄存器)
fn daa(cpu: &mut CPU) -> Result<u8> {
    let mut a = cpu.registers.a;
    let mut adjust = 0;
    let n_flag = cpu.registers.get_subtract_flag();
    let h_flag = cpu.registers.get_half_carry_flag();
    let c_flag = cpu.registers.get_carry_flag();

    if !n_flag {
        // 加法調整
        if c_flag || a > 0x99 {
            adjust |= 0x60;
            cpu.registers.set_carry_flag(true);
        }
        if h_flag || (a & 0x0F) > 0x09 {
            adjust |= 0x06;
        }
        a = a.wrapping_add(adjust);
    } else {
        // 減法調整
        if c_flag {
            adjust |= 0x60;
        }
        if h_flag {
            adjust |= 0x06;
        }
        a = a.wrapping_sub(adjust);
    }

    cpu.registers.update_flags(
        Some(a == 0), // Zero flag
        None,         // Subtract flag unchanged
        Some(false),  // Half carry flag
        None,         // Carry flag unchanged
    );

    cpu.registers.a = a;
    Ok(4)
}
