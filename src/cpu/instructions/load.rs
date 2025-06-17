use super::common::{InstructionError, RegPair, RegTarget};
use super::CPU;

pub type Result<T> = std::result::Result<T, InstructionError>;

/// 處理所有載入指令的分派
pub fn dispatch(cpu: &mut CPU, opcode: u8) -> Result<u8> {
    match opcode {
        // 8-bit 載入指令
        0x06 => ld_n_to_r(cpu, RegTarget::B), // LD B,n
        0x0E => ld_n_to_r(cpu, RegTarget::C), // LD C,n
        0x16 => ld_n_to_r(cpu, RegTarget::D), // LD D,n
        0x1E => ld_n_to_r(cpu, RegTarget::E), // LD E,n
        0x26 => ld_n_to_r(cpu, RegTarget::H), // LD H,n
        0x2E => ld_n_to_r(cpu, RegTarget::L), // LD L,n
        0x3E => ld_n_to_r(cpu, RegTarget::A), // LD A,n

        // 16-bit 載入指令
        0x01 => ld_nn_to_rr(cpu, RegPair::BC), // LD BC,nn
        0x11 => ld_nn_to_rr(cpu, RegPair::DE), // LD DE,nn
        0x21 => ld_nn_to_rr(cpu, RegPair::HL), // LD HL,nn
        0x31 => ld_sp_nn(cpu),                 // LD SP,nn
        0xF9 => ld_sp_hl(cpu),                 // LD SP,HL

        // 8-bit 記憶體載入指令
        0x02 => ld_rr_addr_a(cpu, RegPair::BC), // LD (BC),A
        0x12 => ld_rr_addr_a(cpu, RegPair::DE), // LD (DE),A
        0x77 => ld_hl_addr_a(cpu),              // LD (HL),A
        0x36 => ld_hl_addr_n(cpu),              // LD (HL),n
        0x0A => ld_a_rr_addr(cpu, RegPair::BC), // LD A,(BC)
        0x1A => ld_a_rr_addr(cpu, RegPair::DE), // LD A,(DE)
        0x7E => ld_a_hl_addr(cpu),              // LD A,(HL)

        // HL 增減載入指令
        0x22 => ld_hl_inc_a(cpu), // LD (HL+),A
        0x2A => ld_a_hl_inc(cpu), // LD A,(HL+)
        0x32 => ld_hl_dec_a(cpu), // LD (HL-),A
        0x3A => ld_a_hl_dec(cpu), // LD A,(HL-)

        // 16-bit 記憶體載入指令
        0x08 => ld_nn_addr_sp(cpu), // LD (nn),SP
        0xEA => ld_nn_addr_a(cpu),  // LD (nn),A
        0xFA => ld_a_nn_addr(cpu),  // LD A,(nn)

        // 高記憶體區域載入指令
        0xE0 => ldh_n_a(cpu), // LDH (n),A
        0xF0 => ldh_a_n(cpu), // LDH A,(n)
        0xE2 => ldh_c_a(cpu), // LD (C),A
        0xF2 => ldh_a_c(cpu), // LD A,(C)

        // 堆疊操作指令
        0xC1 => pop_rr(cpu, RegPair::BC),  // POP BC
        0xD1 => pop_rr(cpu, RegPair::DE),  // POP DE
        0xE1 => pop_rr(cpu, RegPair::HL),  // POP HL
        0xF1 => pop_af(cpu),               // POP AF
        0xC5 => push_rr(cpu, RegPair::BC), // PUSH BC
        0xD5 => push_rr(cpu, RegPair::DE), // PUSH DE
        0xE5 => push_rr(cpu, RegPair::HL), // PUSH HL
        0xF5 => push_af(cpu),              // PUSH AF

        // 寄存器間載入指令
        0x40..=0x7F => {
            let src = get_reg_target(opcode & 0x07)?;
            let dst = get_reg_target((opcode >> 3) & 0x07)?;
            if src == RegTarget::HL && dst == RegTarget::HL {
                return Err(InstructionError::Custom(
                    "HALT 指令不應在此處理".to_string(),
                ));
            }
            ld_r_r(cpu, dst, src)
        }

        // 其他 LD 相關指令
        0xF8 => ld_hl_sp_n(cpu), // LD HL,SP+n

        _ => Err(InstructionError::InvalidOpcode(opcode)),
    }
}

/// 從操作碼獲取寄存器目標
fn get_reg_target(reg_code: u8) -> Result<RegTarget> {
    match reg_code {
        0 => Ok(RegTarget::B),
        1 => Ok(RegTarget::C),
        2 => Ok(RegTarget::D),
        3 => Ok(RegTarget::E),
        4 => Ok(RegTarget::H),
        5 => Ok(RegTarget::L),
        6 => Ok(RegTarget::HL),
        7 => Ok(RegTarget::A),
        _ => Err(InstructionError::InvalidRegister(RegTarget::B)), // 保持此處的錯誤處理
    }
}

// -- 8-bit 載入指令 --

/// LD r,n - 載入即時數到寄存器
fn ld_n_to_r(cpu: &mut CPU, target: RegTarget) -> Result<u8> {
    let n = cpu.fetch_byte()?;
    match target {
        RegTarget::A => cpu.registers.a = n,
        RegTarget::B => cpu.registers.b = n,
        RegTarget::C => cpu.registers.c = n,
        RegTarget::D => cpu.registers.d = n,
        RegTarget::E => cpu.registers.e = n,
        RegTarget::H => cpu.registers.h = n,
        RegTarget::L => cpu.registers.l = n,
        RegTarget::HL => return Err(InstructionError::InvalidRegister(target)), // HL 不適用於此指令
    }
    Ok(8)
}

/// LD r1,r2 - 在寄存器之間複製值
fn ld_r_r(cpu: &mut CPU, dst: RegTarget, src: RegTarget) -> Result<u8> {
    let value = match src {
        RegTarget::A => cpu.registers.a,
        RegTarget::B => cpu.registers.b,
        RegTarget::C => cpu.registers.c,
        RegTarget::D => cpu.registers.d,
        RegTarget::E => cpu.registers.e,
        RegTarget::H => cpu.registers.h,
        RegTarget::L => cpu.registers.l,
        RegTarget::HL => cpu.read_byte(cpu.registers.get_hl())?,
    };

    match dst {
        RegTarget::A => cpu.registers.a = value,
        RegTarget::B => cpu.registers.b = value,
        RegTarget::C => cpu.registers.c = value,
        RegTarget::D => cpu.registers.d = value,
        RegTarget::E => cpu.registers.e = value,
        RegTarget::H => cpu.registers.h = value,
        RegTarget::L => cpu.registers.l = value,
        RegTarget::HL => cpu.write_byte(cpu.registers.get_hl(), value)?,
    }

    Ok(if src == RegTarget::HL || dst == RegTarget::HL {
        8
    } else {
        4
    })
}

// -- 16-bit 載入指令 --

/// LD rr,nn - 載入 16-bit 即時數到寄存器對
fn ld_nn_to_rr(cpu: &mut CPU, pair: RegPair) -> Result<u8> {
    let nn = cpu.fetch_word()?;
    match pair {
        RegPair::BC => cpu.registers.set_bc(nn),
        RegPair::DE => cpu.registers.set_de(nn),
        RegPair::HL => cpu.registers.set_hl(nn),
        RegPair::AF => return Err(InstructionError::InvalidRegister(RegTarget::A)),
        RegPair::SP | RegPair::PC => {
            return Err(InstructionError::Custom("無效的寄存器對目標".to_string()))
        }
    }
    Ok(12)
}

/// LD SP,nn - 載入 16-bit 即時數到堆疊指標
fn ld_sp_nn(cpu: &mut CPU) -> Result<u8> {
    let nn = cpu.fetch_word()?;
    cpu.registers.sp = nn;
    Ok(12)
}

/// LD SP,HL - 載入 HL 到堆疊指標
fn ld_sp_hl(cpu: &mut CPU) -> Result<u8> {
    cpu.registers.sp = cpu.registers.get_hl();
    Ok(8)
}

// -- 記憶體載入指令 --

/// LD (rr),A - 載入 A 到指定寄存器對的地址
fn ld_rr_addr_a(cpu: &mut CPU, pair: RegPair) -> Result<u8> {
    let addr = match pair {
        RegPair::BC => cpu.registers.get_bc(),
        RegPair::DE => cpu.registers.get_de(),
        RegPair::HL => cpu.registers.get_hl(),
        RegPair::AF => return Err(InstructionError::InvalidRegister(RegTarget::A)),
        RegPair::SP | RegPair::PC => {
            return Err(InstructionError::Custom("無效的寄存器對目標".to_string()))
        }
    };
    cpu.write_byte(addr, cpu.registers.a)?;
    Ok(8)
}

/// LD (HL),A - 載入 A 到 HL 指向的地址
fn ld_hl_addr_a(cpu: &mut CPU) -> Result<u8> {
    let addr = cpu.registers.get_hl();
    cpu.write_byte(addr, cpu.registers.a)?;
    Ok(8)
}

/// LD (HL),n - 載入即時數到 HL 指向的地址
fn ld_hl_addr_n(cpu: &mut CPU) -> Result<u8> {
    let n = cpu.fetch_byte()?;
    let addr = cpu.registers.get_hl();
    cpu.write_byte(addr, n)?;
    Ok(12)
}

/// LD A,(rr) - 從指定寄存器對的地址載入到 A
fn ld_a_rr_addr(cpu: &mut CPU, pair: RegPair) -> Result<u8> {
    let addr = match pair {
        RegPair::BC => cpu.registers.get_bc(),
        RegPair::DE => cpu.registers.get_de(),
        RegPair::HL => cpu.registers.get_hl(),
        RegPair::AF => return Err(InstructionError::InvalidRegister(RegTarget::A)),
        RegPair::SP | RegPair::PC => {
            return Err(InstructionError::Custom("無效的寄存器對目標".to_string()))
        }
    };
    cpu.registers.a = cpu.read_byte(addr)?;
    Ok(8)
}

/// LD A,(HL) - 從 HL 指向的地址載入到 A
fn ld_a_hl_addr(cpu: &mut CPU) -> Result<u8> {
    let addr = cpu.registers.get_hl();
    cpu.registers.a = cpu.read_byte(addr)?;
    Ok(8)
}

// -- HL 增減載入指令 --

/// LD (HL+),A - 載入 A 到 HL 指向的地址並遞增 HL
fn ld_hl_inc_a(cpu: &mut CPU) -> Result<u8> {
    let addr = cpu.registers.get_hl();
    cpu.write_byte(addr, cpu.registers.a)?;
    cpu.registers.set_hl(addr.wrapping_add(1));
    Ok(8)
}

/// LD A,(HL+) - 從 HL 指向的地址載入到 A 並遞增 HL
fn ld_a_hl_inc(cpu: &mut CPU) -> Result<u8> {
    let addr = cpu.registers.get_hl();
    cpu.registers.a = cpu.read_byte(addr)?;
    cpu.registers.set_hl(addr.wrapping_add(1));
    Ok(8)
}

/// LD (HL-),A - 載入 A 到 HL 指向的地址並遞減 HL
fn ld_hl_dec_a(cpu: &mut CPU) -> Result<u8> {
    let addr = cpu.registers.get_hl();
    cpu.write_byte(addr, cpu.registers.a)?;
    cpu.registers.set_hl(addr.wrapping_sub(1));
    Ok(8)
}

/// LD A,(HL-) - 從 HL 指向的地址載入到 A 並遞減 HL
fn ld_a_hl_dec(cpu: &mut CPU) -> Result<u8> {
    let addr = cpu.registers.get_hl();
    cpu.registers.a = cpu.read_byte(addr)?;
    cpu.registers.set_hl(addr.wrapping_sub(1));
    Ok(8)
}

// -- 其他記憶體載入指令 --

/// LD (nn),SP - 載入 SP 到指定地址
fn ld_nn_addr_sp(cpu: &mut CPU) -> Result<u8> {
    let addr = cpu.fetch_word()?;
    let sp = cpu.registers.sp;
    cpu.write_byte(addr, (sp & 0xFF) as u8)?;
    cpu.write_byte(addr.wrapping_add(1), (sp >> 8) as u8)?;
    Ok(20)
}

/// LD (nn),A - 載入 A 到指定地址
fn ld_nn_addr_a(cpu: &mut CPU) -> Result<u8> {
    let addr = cpu.fetch_word()?;
    cpu.write_byte(addr, cpu.registers.a)?;
    Ok(16)
}

/// LD A,(nn) - 從指定地址載入到 A
fn ld_a_nn_addr(cpu: &mut CPU) -> Result<u8> {
    let addr = cpu.fetch_word()?;
    cpu.registers.a = cpu.read_byte(addr)?;
    Ok(16)
}

// -- 高記憶體區域載入指令 --

/// LDH (n),A - 載入 A 到高記憶體區域
fn ldh_n_a(cpu: &mut CPU) -> Result<u8> {
    let n = cpu.fetch_byte()?;
    let addr = 0xFF00 | (n as u16);
    cpu.write_byte(addr, cpu.registers.a)?;
    Ok(12)
}

/// LDH A,(n) - 從高記憶體區域載入到 A
fn ldh_a_n(cpu: &mut CPU) -> Result<u8> {
    let n = cpu.fetch_byte()?;
    let addr = 0xFF00 | (n as u16);
    cpu.registers.a = cpu.read_byte(addr)?;
    Ok(12)
}

/// LD (C),A - 載入 A 到 FF00+C 地址
fn ldh_c_a(cpu: &mut CPU) -> Result<u8> {
    let addr = 0xFF00 | (cpu.registers.c as u16);
    cpu.write_byte(addr, cpu.registers.a)?;
    Ok(8)
}

/// LD A,(C) - 從 FF00+C 地址載入到 A
fn ldh_a_c(cpu: &mut CPU) -> Result<u8> {
    let addr = 0xFF00 | (cpu.registers.c as u16);
    cpu.registers.a = cpu.read_byte(addr)?;
    Ok(8)
}

// -- 堆疊操作指令 --

/// PUSH rr - 將寄存器對推入堆疊
fn push_rr(cpu: &mut CPU, pair: RegPair) -> Result<u8> {
    let value = match pair {
        RegPair::BC => cpu.registers.get_bc(),
        RegPair::DE => cpu.registers.get_de(),
        RegPair::HL => cpu.registers.get_hl(),
        RegPair::AF => {
            let value = ((cpu.registers.a as u16) << 8) | (cpu.registers.get_flags() as u16);
            value
        }
        RegPair::SP | RegPair::PC => {
            return Err(InstructionError::Custom("無效的寄存器對目標".to_string()))
        }
    };
    cpu.registers.sp = cpu.registers.sp.wrapping_sub(2);
    let sp = cpu.registers.sp;
    cpu.write_byte(sp.wrapping_add(1), (value >> 8) as u8)?;
    cpu.write_byte(sp, (value & 0xFF) as u8)?;
    Ok(16)
}

/// PUSH AF - 將 AF 推入堆疊（特殊處理）
fn push_af(cpu: &mut CPU) -> Result<u8> {
    cpu.registers.sp = cpu.registers.sp.wrapping_sub(2);
    let sp = cpu.registers.sp;
    cpu.write_byte(sp.wrapping_add(1), cpu.registers.a)?;
    cpu.write_byte(sp, cpu.registers.get_flags())?;
    Ok(16)
}

/// POP rr - 從堆疊中彈出到寄存器對
fn pop_rr(cpu: &mut CPU, pair: RegPair) -> Result<u8> {
    let sp = cpu.registers.sp;
    let value_low = cpu.read_byte(sp)?;
    let value_high = cpu.read_byte(sp.wrapping_add(1))?;
    let value = ((value_high as u16) << 8) | (value_low as u16);

    match pair {
        RegPair::BC => cpu.registers.set_bc(value),
        RegPair::DE => cpu.registers.set_de(value),
        RegPair::HL => cpu.registers.set_hl(value),
        RegPair::AF => {
            cpu.registers.a = value_high;
            cpu.registers.set_flags(value_low & 0xF0); // 只使用高 4 位
        }
        RegPair::SP | RegPair::PC => {
            return Err(InstructionError::Custom("無效的寄存器對目標".to_string()))
        }
    }

    cpu.registers.sp = cpu.registers.sp.wrapping_add(2);
    Ok(12)
}

/// POP AF - 從堆疊中彈出到 AF（特殊處理）
fn pop_af(cpu: &mut CPU) -> Result<u8> {
    let sp = cpu.registers.sp;
    let flags = cpu.read_byte(sp)? & 0xF0; // 只保留高 4 位
    let a = cpu.read_byte(sp.wrapping_add(1))?;
    cpu.registers.a = a;
    cpu.registers.set_flags(flags);
    cpu.registers.sp = cpu.registers.sp.wrapping_add(2);
    Ok(12)
}

/// LD HL,SP+n - 將 SP 加一個有符號數載入到 HL
fn ld_hl_sp_n(cpu: &mut CPU) -> Result<u8> {
    let n = cpu.fetch_byte()? as i8 as i16 as u16;
    let sp = cpu.registers.sp;
    let result = sp.wrapping_add(n);

    // 設置標誌位
    cpu.registers.update_flags(
        Some(false),                           // Zero: reset
        Some(false),                           // Subtract: reset
        Some((sp & 0x0F) + (n & 0x0F) > 0x0F), // H: 設置如果第 3 位有進位
        Some((sp & 0xFF) + (n & 0xFF) > 0xFF), // C: 設置如果第 7 位有進位
    );

    cpu.registers.set_hl(result);
    Ok(12)
}
