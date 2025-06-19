use super::common::FlagOperations;
use super::common::{InstructionError, RegTarget};
use super::CPU;

pub type Result<T> = std::result::Result<T, InstructionError>;

const CYCLES_1: u8 = 4; // 1 M-cycle = 4 t-cycles
const CYCLES_2: u8 = 8; // 2 M-cycles = 8 t-cycles
const CYCLES_3: u8 = 12; // 3 M-cycles = 12 t-cycles

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
        0x09 => add_hl_rr(cpu, RegPair::BC),              // ADD HL,BC
        0x19 => add_hl_rr(cpu, RegPair::DE),              // ADD HL,DE
        0x29 => add_hl_rr(cpu, RegPair::HL),              // ADD HL,HL
        0x39 => add_hl_rr(cpu, RegPair::SP),              // ADD HL,SP
        0xE8 => add_sp_n(cpu),                            // ADD SP,n        // 特殊操作
        0x27 => daa(cpu),                                 // DAA (十進制調整)
        // 0x2F CPL (取反) 已移至 logic 模組實現
        0x3F => ccf(cpu), // CCF (進位標誌取反)
        0x37 => scf(cpu), // SCF (設置進位標誌)

        _ => Err(InstructionError::InvalidOpcode(opcode)),
    }
}

/// 用於識別 16-bit 暫存器對
#[derive(Debug, Clone, Copy)]
pub enum RegPair {
    BC,
    DE,
    HL,
    SP,
}

/// 增加 16-bit 暫存器對的值
pub fn add_hl_rr(cpu: &mut CPU, rp: RegPair) -> Result<u8> {
    let hl = cpu.registers.get_hl();
    let rr = match rp {
        RegPair::BC => cpu.registers.get_bc(),
        RegPair::DE => cpu.registers.get_de(),
        RegPair::HL => cpu.registers.get_hl(),
        RegPair::SP => cpu.registers.sp,
    };

    let result = hl.wrapping_add(rr);
    let half_carry = (hl & 0x0FFF) + (rr & 0x0FFF) > 0x0FFF;
    let carry = ((hl as u32) + (rr as u32)) > 0xFFFF;

    cpu.registers.set_subtract_flag(false);
    cpu.registers.set_half_carry_flag(half_carry);
    cpu.registers.set_carry_flag(carry);
    cpu.registers.set_hl(result);

    // 記錄詳細的調試信息
    let _rp_name = match rp {
        RegPair::BC => "BC",
        RegPair::DE => "DE",
        RegPair::HL => "HL",
        RegPair::SP => "SP",
    };

    // 移除 cpu.logger 調用，保留原本 log 行為的註解
    // let _ = cpu.logger.borrow_mut().debug(&format!(
    //     "ADD HL,{}: HL={:04X}, {}={:04X}, 結果={:04X}, HF={}, CF={}",
    //     rp_name,
    //     hl,
    //     rp_name,
    //     rr,
    //     result,
    //     if half_carry { 1 } else { 0 },
    //     if carry { 1 } else { 0 }
    // ));

    Ok(CYCLES_2)
}

/// 將 8-bit 有符號數加到 SP
pub fn add_sp_n(cpu: &mut CPU) -> Result<u8> {
    let n = cpu.read_next_byte()? as i8;
    let sp = cpu.registers.sp;
    let n_u16 = n as i16 as u16;
    let result = sp.wrapping_add(n_u16);

    // 根據 Game Boy CPU 手冊設置標誌
    let half_carry = (sp & 0x000F) + (n_u16 & 0x000F) > 0x000F;
    let carry = (sp & 0x00FF) + (n_u16 & 0x00FF) > 0x00FF;

    cpu.registers.set_zero_flag(false);
    cpu.registers.set_subtract_flag(false);
    cpu.registers.set_half_carry_flag(half_carry);
    cpu.registers.set_carry_flag(carry);
    cpu.registers.sp = result;

    Ok(CYCLES_3)
}

/// 十進制調整累加器 (DAA)
pub fn daa(cpu: &mut CPU) -> Result<u8> {
    let mut a = cpu.registers.a;
    let mut adjust = 0;

    if cpu.registers.get_half_carry_flag() || (!cpu.registers.get_subtract_flag() && (a & 0x0F) > 9)
    {
        adjust |= 0x06;
    }

    if cpu.registers.get_carry_flag() || (!cpu.registers.get_subtract_flag() && a > 0x99) {
        adjust |= 0x60;
        cpu.registers.set_carry_flag(true);
    }

    a = if cpu.registers.get_subtract_flag() {
        a.wrapping_sub(adjust)
    } else {
        a.wrapping_add(adjust)
    };

    cpu.registers.set_zero_flag(a == 0);
    cpu.registers.set_half_carry_flag(false);
    cpu.registers.a = a;

    Ok(CYCLES_1)
}

#[allow(dead_code)]
/// 取反累加器 (CPL)
pub fn cpl(cpu: &mut CPU) -> Result<u8> {
    cpu.registers.a = !cpu.registers.a;
    cpu.registers.set_subtract_flag(true);
    cpu.registers.set_half_carry_flag(true);
    Ok(CYCLES_1)
}

/// 進位標誌取反 (CCF)
pub fn ccf(cpu: &mut CPU) -> Result<u8> {
    let current_carry = cpu.registers.get_carry_flag();
    cpu.registers.set_subtract_flag(false);
    cpu.registers.set_half_carry_flag(false);
    cpu.registers.set_carry_flag(!current_carry);
    Ok(CYCLES_1)
}

/// 設置進位標誌 (SCF)
pub fn scf(cpu: &mut CPU) -> Result<u8> {
    cpu.registers.set_subtract_flag(false);
    cpu.registers.set_half_carry_flag(false);
    cpu.registers.set_carry_flag(true);
    Ok(CYCLES_1)
}

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
        RegTarget::BC => Ok(cpu.read_byte(cpu.registers.get_bc())?),
        RegTarget::DE => Ok(cpu.read_byte(cpu.registers.get_de())?),
        RegTarget::SP => Ok(cpu.registers.sp as u8),
        RegTarget::AF => Ok(cpu.registers.get_af() as u8),
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
        RegTarget::BC => Ok(cpu.write_byte(cpu.registers.get_bc(), value)?),
        RegTarget::DE => Ok(cpu.write_byte(cpu.registers.get_de(), value)?),
        RegTarget::SP => {
            cpu.registers.sp = (cpu.registers.sp & 0xFF00) | (value as u16);
            Ok(())
        }
        RegTarget::AF => {
            cpu.registers
                .set_af((cpu.registers.get_af() & 0xFF00) | (value as u16));
            Ok(())
        }
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
    let a = cpu.registers.a;
    let carry = if cpu.registers.get_carry_flag() { 1 } else { 0 };

    let result = a.wrapping_add(value).wrapping_add(carry);
    let half_carry = (a & 0x0F) + (value & 0x0F) + carry > 0x0F;
    let carry = (a as u16) + (value as u16) + (carry as u16) > 0xFF;

    cpu.registers.a = result;
    cpu.registers.update_flags(
        Some(result == 0),
        Some(false),
        Some(half_carry),
        Some(carry),
    );

    Ok(8) // ADC A,n 指令需要 8 個機器週期
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
    let half_carry = (value & 0x0F) == 0;

    set_reg_value(cpu, target, result)?;
    cpu.registers.set_zero_flag(result == 0);
    cpu.registers.set_subtract_flag(true);
    cpu.registers.set_half_carry_flag(half_carry);

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

/// 16-bit 寄存器對遞增
fn inc_rr(cpu: &mut CPU, opcode: u8) -> Result<u8> {
    match opcode {
        0x03 => cpu.registers.set_bc(cpu.registers.get_bc().wrapping_add(1)),
        0x13 => cpu.registers.set_de(cpu.registers.get_de().wrapping_add(1)),
        0x23 => cpu.registers.set_hl(cpu.registers.get_hl().wrapping_add(1)),
        0x33 => cpu.registers.sp = cpu.registers.sp.wrapping_add(1),
        _ => return Err(InstructionError::InvalidOpcode(opcode)),
    }
    Ok(CYCLES_2)
}

/// 16-bit 寄存器對遞減
fn dec_rr(cpu: &mut CPU, opcode: u8) -> Result<u8> {
    match (opcode >> 4) & 0x3 {
        0 => cpu.registers.set_bc(cpu.registers.get_bc().wrapping_sub(1)),
        1 => cpu.registers.set_de(cpu.registers.get_de().wrapping_sub(1)),
        2 => cpu.registers.set_hl(cpu.registers.get_hl().wrapping_sub(1)),
        3 => cpu.registers.sp = cpu.registers.sp.wrapping_sub(1),
        _ => return Err(InstructionError::InvalidOpcode(opcode)),
    }
    Ok(CYCLES_2)
}
