use super::common::{InstructionError, RegPair, RegTarget};
use super::CPU;
use crate::error::Result;

const CYCLES_2: u8 = 8;
const CYCLES_3: u8 = 12;

pub fn dispatch(cpu: &mut CPU, opcode: u8) -> Result<u8> {
    match opcode {
        // 8-bit 即時數載入
        0x06 => ld_r_n(cpu, 'B'), // LD B,n
        0x0E => ld_r_n(cpu, 'C'), // LD C,n
        0x16 => ld_r_n(cpu, 'D'), // LD D,n
        0x1E => ld_r_n(cpu, 'E'), // LD E,n
        0x26 => ld_r_n(cpu, 'H'), // LD H,n
        0x2E => ld_r_n(cpu, 'L'), // LD L,n
        0x36 => ld_hl_n(cpu),     // LD (HL),n
        0x3E => ld_r_n(cpu, 'A'), // LD A,n

        // 16-bit 即時數載入
        0x01 => ld_rr_nn(cpu, 'B'), // LD BC,nn
        0x11 => ld_rr_nn(cpu, 'D'), // LD DE,nn
        0x21 => ld_rr_nn(cpu, 'H'), // LD HL,nn
        0x31 => ld_sp_nn(cpu),      // LD SP,nn

        // 寄存器間移動和 HL 間接載入/儲存
        0x40..=0x7F => ld_r_r(cpu, opcode), // 包括 LD r,r 和 (HL) 相關指令

        // 間接載入
        0x22 => ld_hl_inc_a(cpu), // LD (HL+),A
        0x2A => ld_a_hl_inc(cpu), // LD A,(HL+)
        0x32 => ld_hl_dec_a(cpu), // LD (HL-),A
        0x3A => ld_a_hl_dec(cpu), // LD A,(HL-)

        // 位址載入
        0x02 => ld_bc_a(cpu), // LD (BC),A
        0x12 => ld_de_a(cpu), // LD (DE),A
        0x0A => ld_a_bc(cpu), // LD A,(BC)
        0x1A => ld_a_de(cpu), // LD A,(DE)
        0xE0 => ldh_n_a(cpu), // LDH (n),A
        0xF0 => ldh_a_n(cpu), // LDH A,(n)
        0xE2 => ldh_c_a(cpu), // LDH (C),A
        0xF2 => ldh_a_c(cpu), // LDH A,(C)
        0xEA => ld_nn_a(cpu), // LD (nn),A
        0xFA => ld_a_nn(cpu), // LD A,(nn)

        // PUSH/POP 指令
        0xC1 => pop_rr(cpu, RegPair::BC),  // POP BC
        0xD1 => pop_rr(cpu, RegPair::DE),  // POP DE
        0xE1 => pop_rr(cpu, RegPair::HL),  // POP HL
        0xF1 => pop_rr(cpu, RegPair::AF),  // POP AF
        0xC5 => push_rr(cpu, RegPair::BC), // PUSH BC
        0xD5 => push_rr(cpu, RegPair::DE), // PUSH DE
        0xE5 => push_rr(cpu, RegPair::HL), // PUSH HL
        0xF5 => push_rr(cpu, RegPair::AF), // PUSH AF

        _ => Err(InstructionError::InvalidOpcode(opcode).into()),
    }
}

fn ld_r_n(cpu: &mut CPU, reg: char) -> Result<u8> {
    let value = cpu.fetch_byte()?;
    let reg_target = RegTarget::from(reg);
    match reg_target {
        RegTarget::A => cpu.registers.a = value,
        RegTarget::B => cpu.registers.b = value,
        RegTarget::C => cpu.registers.c = value,
        RegTarget::D => cpu.registers.d = value,
        RegTarget::E => cpu.registers.e = value,
        RegTarget::H => cpu.registers.h = value,
        RegTarget::L => cpu.registers.l = value,
        _ => return Err(InstructionError::InvalidRegister(reg_target))?,
    }
    Ok(CYCLES_2)
}

fn ld_r_r(cpu: &mut CPU, opcode: u8) -> Result<u8> {
    let dst = (opcode >> 3) & 0x07;
    let src = opcode & 0x07;

    // 如果 src 或 dst 為 6，表示使用 (HL)
    let src_value = match src {
        0 => Ok(cpu.registers.b),
        1 => Ok(cpu.registers.c),
        2 => Ok(cpu.registers.d),
        3 => Ok(cpu.registers.e),
        4 => Ok(cpu.registers.h),
        5 => Ok(cpu.registers.l),
        6 => cpu.read_byte(cpu.registers.get_hl()), // 從 (HL) 讀取
        7 => Ok(cpu.registers.a),
        _ => return Err(InstructionError::InvalidSource(src).into()),
    }?;

    match dst {
        0 => cpu.registers.b = src_value,
        1 => cpu.registers.c = src_value,
        2 => cpu.registers.d = src_value,
        3 => cpu.registers.e = src_value,
        4 => cpu.registers.h = src_value,
        5 => cpu.registers.l = src_value,
        6 => cpu.write_byte(cpu.registers.get_hl(), src_value)?, // 寫入 (HL)
        7 => cpu.registers.a = src_value,
        _ => return Err(InstructionError::InvalidDestination(dst).into()),
    }

    Ok(CYCLES_2)
}

fn ld_rr_nn(cpu: &mut CPU, first_reg: char) -> Result<u8> {
    let low = cpu.fetch_byte()?;
    let high = cpu.fetch_byte()?;

    match first_reg {
        'B' => {
            cpu.registers.b = high;
            cpu.registers.c = low;
        }
        'D' => {
            cpu.registers.d = high;
            cpu.registers.e = low;
        }
        'H' => {
            cpu.registers.h = high;
            cpu.registers.l = low;
        }
        _ => {
            return Err(InstructionError::InvalidRegister(RegTarget::from(
                first_reg,
            )))?
        }
    }
    Ok(CYCLES_3)
}

fn ld_sp_nn(cpu: &mut CPU) -> Result<u8> {
    let low = cpu.fetch_byte()?;
    let high = cpu.fetch_byte()?;
    let value = ((high as u16) << 8) | (low as u16);
    cpu.registers.sp = value;
    Ok(CYCLES_3)
}

/// 載入 A 到 (HL) 並遞減 HL
fn ld_hl_dec_a(cpu: &mut CPU) -> Result<u8> {
    let addr = cpu.registers.get_hl();
    cpu.write_byte(addr, cpu.registers.a)?;
    cpu.registers.set_hl(addr.wrapping_sub(1));
    Ok(CYCLES_2)
}

/// 載入 (HL) 到 A 並遞減 HL
fn ld_a_hl_dec(cpu: &mut CPU) -> Result<u8> {
    let addr = cpu.registers.get_hl();
    cpu.registers.a = cpu.read_byte(addr)?;
    cpu.registers.set_hl(addr.wrapping_sub(1));
    Ok(CYCLES_2)
}

/// 載入 A 到 (HL) 並遞增 HL
fn ld_hl_inc_a(cpu: &mut CPU) -> Result<u8> {
    let addr = cpu.registers.get_hl();
    cpu.write_byte(addr, cpu.registers.a)?;
    cpu.registers.set_hl(addr.wrapping_add(1));
    Ok(CYCLES_2)
}

/// 載入 (HL) 到 A 並遞增 HL
fn ld_a_hl_inc(cpu: &mut CPU) -> Result<u8> {
    let addr = cpu.registers.get_hl();
    cpu.registers.a = cpu.read_byte(addr)?;
    cpu.registers.set_hl(addr.wrapping_add(1));
    Ok(CYCLES_2)
}

// 保存 A 到間接位址
fn ld_bc_a(cpu: &mut CPU) -> Result<u8> {
    cpu.write_byte(cpu.get_bc(), cpu.registers.a)?;
    Ok(CYCLES_2)
}

fn ld_de_a(cpu: &mut CPU) -> Result<u8> {
    cpu.write_byte(cpu.get_de(), cpu.registers.a)?;
    Ok(CYCLES_2)
}

// 從間接位址載入到 A
fn ld_a_bc(cpu: &mut CPU) -> Result<u8> {
    cpu.registers.a = cpu.read_byte(cpu.get_bc())?;
    Ok(CYCLES_2)
}

fn ld_a_de(cpu: &mut CPU) -> Result<u8> {
    cpu.registers.a = cpu.read_byte(cpu.get_de())?;
    Ok(CYCLES_2)
}

// 高速頁面區域相關指令
fn ldh_n_a(cpu: &mut CPU) -> Result<u8> {
    let offset = cpu.fetch_byte()?;
    let address = 0xFF00 | (offset as u16);
    cpu.write_byte(address, cpu.registers.a)?;
    Ok(CYCLES_3)
}

fn ldh_a_n(cpu: &mut CPU) -> Result<u8> {
    let offset = cpu.fetch_byte()?;
    let address = 0xFF00 | (offset as u16);
    cpu.registers.a = cpu.read_byte(address)?;
    Ok(CYCLES_3)
}

fn ldh_c_a(cpu: &mut CPU) -> Result<u8> {
    let address = 0xFF00 | (cpu.registers.c as u16);
    cpu.write_byte(address, cpu.registers.a)?;
    Ok(CYCLES_2)
}

fn ldh_a_c(cpu: &mut CPU) -> Result<u8> {
    let address = 0xFF00 | (cpu.registers.c as u16);
    cpu.registers.a = cpu.read_byte(address)?;
    Ok(CYCLES_2)
}

// 絕對位址載入
fn ld_nn_a(cpu: &mut CPU) -> Result<u8> {
    let low = cpu.fetch_byte()?;
    let high = cpu.fetch_byte()?;
    let address = ((high as u16) << 8) | (low as u16);
    cpu.write_byte(address, cpu.registers.a)?;
    Ok(CYCLES_3)
}

fn ld_a_nn(cpu: &mut CPU) -> Result<u8> {
    let low = cpu.fetch_byte()?;
    let high = cpu.fetch_byte()?;
    let address = ((high as u16) << 8) | (low as u16);
    cpu.registers.a = cpu.read_byte(address)?;
    Ok(CYCLES_3)
}

// PUSH rr - 將 16-bit 暫存器對推入堆疊
fn push_rr(cpu: &mut CPU, reg_pair: RegPair) -> Result<u8> {
    let value = match reg_pair {
        RegPair::BC => cpu.registers.get_bc(),
        RegPair::DE => cpu.registers.get_de(),
        RegPair::HL => cpu.registers.get_hl(),
        RegPair::AF => cpu.registers.get_af(),
        _ => return Err(InstructionError::InvalidRegister(RegTarget::BC))?,
    };

    cpu.push(value)?;
    Ok(CYCLES_3) // PUSH 指令需要 16 個機器週期 = 3 個 M-cycle
}

// POP rr - 從堆疊取出 16-bit 值到暫存器對
fn pop_rr(cpu: &mut CPU, reg_pair: RegPair) -> Result<u8> {
    let value = cpu.read_word(cpu.registers.sp)?;
    cpu.registers.sp = cpu.registers.sp.wrapping_add(2);

    match reg_pair {
        RegPair::BC => cpu.registers.set_bc(value),
        RegPair::DE => cpu.registers.set_de(value),
        RegPair::HL => cpu.registers.set_hl(value),
        RegPair::AF => {
            // AF 時需要保留最低位元組的低 4 位為 0
            let af = value & 0xFFF0;
            cpu.registers.set_af(af);
        }
        _ => return Err(InstructionError::InvalidRegister(RegTarget::BC))?,
    }

    Ok(CYCLES_2) // POP 指令需要 12 個機器週期 = 2 個 M-cycle
}

/// LD (HL),n - 將立即數值寫入 HL 指向的記憶體位置
fn ld_hl_n(cpu: &mut CPU) -> Result<u8> {
    let value = cpu.fetch_byte()?;
    cpu.write_byte(cpu.registers.get_hl(), value)?;
    Ok(CYCLES_3) // 這個指令需要 3 個 M-cycle
}
