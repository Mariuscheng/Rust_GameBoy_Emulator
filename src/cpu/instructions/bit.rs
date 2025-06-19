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
    let value = get_register_value(cpu, reg)?;
    let is_set = (value & (1 << bit)) != 0;

    cpu.registers.set_flag_z(!is_set);
    cpu.registers.set_flag_n(false);
    cpu.registers.set_flag_h(true);

    Ok(if reg == 6 { CYCLES_4 } else { CYCLES_2 })
}

// SET b,r 指令的實作
fn set_operations(cpu: &mut CPU, opcode: u8) -> Result<u8, InstructionError> {
    let bit = (opcode >> 3) & 0x07;
    let reg = opcode & 0x07;
    let value = get_register_value(cpu, reg)?;
    let new_value = value | (1 << bit);
    set_register_value(cpu, reg, new_value)?;

    Ok(if reg == 6 { CYCLES_4 } else { CYCLES_2 })
}

// RES b,r 指令的實作
fn res_operations(cpu: &mut CPU, opcode: u8) -> Result<u8, InstructionError> {
    let bit = (opcode >> 3) & 0x07;
    let reg = opcode & 0x07;
    let value = get_register_value(cpu, reg)?;
    let new_value = value & !(1 << bit);
    set_register_value(cpu, reg, new_value)?;

    Ok(if reg == 6 { CYCLES_4 } else { CYCLES_2 })
}

// 旋轉與位移操作
fn rotate_shift_operations(cpu: &mut CPU, opcode: u8) -> Result<u8, InstructionError> {
    let reg = opcode & 0x07;
    let value = get_register_value(cpu, reg)?;
    let new_value = match opcode & 0xF8 {
        0x00 => rlc(cpu, value),  // RLC r
        0x08 => rrc(cpu, value),  // RRC r
        0x10 => rl(cpu, value),   // RL r
        0x18 => rr(cpu, value),   // RR r
        0x20 => sla(cpu, value),  // SLA r
        0x28 => sra(cpu, value),  // SRA r
        0x30 => swap(cpu, value), // SWAP r
        0x38 => srl(cpu, value),  // SRL r
        _ => return Err(InstructionError::InvalidOpcode(opcode)),
    };
    set_register_value(cpu, reg, new_value)?;

    Ok(if reg == 6 { CYCLES_4 } else { CYCLES_2 })
}

// 向左循環旋轉
fn rlc(cpu: &mut CPU, value: u8) -> u8 {
    let carry = value & 0x80 != 0;
    let result = value.rotate_left(1);

    cpu.registers.set_flag_z(result == 0);
    cpu.registers.set_flag_n(false);
    cpu.registers.set_flag_h(false);
    cpu.registers.set_flag_c(carry);

    result
}

// 向右循環旋轉
fn rrc(cpu: &mut CPU, value: u8) -> u8 {
    let carry = value & 0x01 != 0;
    let result = value.rotate_right(1);

    cpu.registers.set_flag_z(result == 0);
    cpu.registers.set_flag_n(false);
    cpu.registers.set_flag_h(false);
    cpu.registers.set_flag_c(carry);

    result
}

// 向左旋轉（通過進位）
fn rl(cpu: &mut CPU, value: u8) -> u8 {
    let old_carry = cpu.registers.get_flag_c();
    let new_carry = value & 0x80 != 0;
    let result = (value << 1) | (if old_carry { 1 } else { 0 });

    cpu.registers.set_flag_z(result == 0);
    cpu.registers.set_flag_n(false);
    cpu.registers.set_flag_h(false);
    cpu.registers.set_flag_c(new_carry);

    result
}

// 向右旋轉（通過進位）
fn rr(cpu: &mut CPU, value: u8) -> u8 {
    let old_carry = cpu.registers.get_flag_c();
    let new_carry = value & 0x01 != 0;
    let result = (value >> 1) | (if old_carry { 0x80 } else { 0 });

    cpu.registers.set_flag_z(result == 0);
    cpu.registers.set_flag_n(false);
    cpu.registers.set_flag_h(false);
    cpu.registers.set_flag_c(new_carry);

    result
}

// 算術左移
fn sla(cpu: &mut CPU, value: u8) -> u8 {
    let carry = value & 0x80 != 0;
    let result = value << 1;

    cpu.registers.set_flag_z(result == 0);
    cpu.registers.set_flag_n(false);
    cpu.registers.set_flag_h(false);
    cpu.registers.set_flag_c(carry);

    result
}

// 算術右移
fn sra(cpu: &mut CPU, value: u8) -> u8 {
    let carry = value & 0x01 != 0;
    let result = (value >> 1) | (value & 0x80);

    cpu.registers.set_flag_z(result == 0);
    cpu.registers.set_flag_n(false);
    cpu.registers.set_flag_h(false);
    cpu.registers.set_flag_c(carry);

    result
}

// 交換高低四位
fn swap(cpu: &mut CPU, value: u8) -> u8 {
    let result = ((value & 0x0F) << 4) | ((value & 0xF0) >> 4);

    cpu.registers.set_flag_z(result == 0);
    cpu.registers.set_flag_n(false);
    cpu.registers.set_flag_h(false);
    cpu.registers.set_flag_c(false);

    result
}

// 邏輯右移
fn srl(cpu: &mut CPU, value: u8) -> u8 {
    let carry = value & 0x01 != 0;
    let result = value >> 1;

    cpu.registers.set_flag_z(result == 0);
    cpu.registers.set_flag_n(false);
    cpu.registers.set_flag_h(false);
    cpu.registers.set_flag_c(carry);

    result
}

#[cfg(test)]
mod tests {
    
    use crate::cpu::interrupts::InterruptRegisters;
    use crate::cpu::registers::Registers;
    use crate::cpu::CPU;
    use crate::mmu::MMU;
    use std::cell::RefCell;
    use std::rc::Rc;

    fn test_cpu() -> CPU {
        CPU {
            registers: Registers::new(),
            mmu: Rc::new(RefCell::new(MMU::new())),
            interrupt_registers: Rc::new(RefCell::new(InterruptRegisters::default())),
            instruction_count: 0,
            ime: false,
            ei_pending: false,
            state: crate::cpu::CpuState::Running,
            total_cycles: 0,
            loop_detection: super::super::super::CpuLoopDetection::new(false),
        }
    }

    #[test]
    fn test_rlc_b() {
        let mut cpu = test_cpu();
        cpu.registers.b = 0b1000_0001;
        let _ = super::dispatch(&mut cpu, 0x00); // CB 00 = RLC B
        assert_eq!(cpu.registers.b, 0b0000_0011);
        assert!(cpu.registers.get_flag_c());
        assert!(!cpu.registers.get_flag_z());
    }

    #[test]
    fn test_bit_7_b() {
        let mut cpu = test_cpu();
        cpu.registers.b = 0b1000_0000;
        let _ = super::dispatch(&mut cpu, 0x78); // CB 78 = BIT 7,B
        assert!(!cpu.registers.get_flag_z());
        cpu.registers.b = 0b0000_0000;
        let _ = super::dispatch(&mut cpu, 0x78);
        assert!(cpu.registers.get_flag_z());
    }

    #[test]
    fn test_set_3_c() {
        let mut cpu = test_cpu();
        cpu.registers.c = 0b0000_0000;
        let _ = super::dispatch(&mut cpu, 0xD9); // CB D9 = SET 3,C
        assert_eq!(cpu.registers.c, 0b0000_1000);
    }

    #[test]
    fn test_res_0_d() {
        let mut cpu = test_cpu();
        cpu.registers.d = 0b0000_0001;
        let _ = super::dispatch(&mut cpu, 0x82); // CB 82 = RES 0,D
        assert_eq!(cpu.registers.d, 0b0000_0000);
    }
}
