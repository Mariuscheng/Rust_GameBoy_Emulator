use super::common::{InstructionError, CYCLES_1};
use super::CPU;
use crate::cpu::CpuState;

type Result<T> = std::result::Result<T, InstructionError>;

pub fn dispatch(cpu: &mut CPU, opcode: u8) -> Result<u8> {
    match opcode {
        0x00 => nop(),          // NOP
        0x10 => stop(),         // STOP
        0x76 => halt(cpu),      // HALT
        0xF3 => di(cpu),        // DI (Disable Interrupts)
        0xFB => ei(cpu),        // EI (Enable Interrupts)
        0xCB => handle_cb(cpu), // CB prefix
        0xD3 | 0xDB | 0xDD | 0xE3 | 0xE4 | 0xF4 | 0xFC | 0xFD => {
            Err(InstructionError::InvalidOpcode(opcode))
        }
        _ => Err(InstructionError::InvalidOpcode(opcode)),
    }
}

// 空操作
fn nop() -> Result<u8> {
    Ok(CYCLES_1)
}

// 停止操作
fn stop() -> Result<u8> {
    Ok(CYCLES_1)
}

// 暫停操作
fn halt(cpu: &mut CPU) -> Result<u8> {
    cpu.state = CpuState::Halted;
    Ok(CYCLES_1)
}

// 禁用中斷
fn di(cpu: &mut CPU) -> Result<u8> {
    cpu.disable_interrupts();
    Ok(CYCLES_1)
}

// 啟用中斷
fn ei(cpu: &mut CPU) -> Result<u8> {
    cpu.enable_interrupts();
    Ok(CYCLES_1)
}

// 處理 CB 前綴指令
fn handle_cb(cpu: &mut CPU) -> Result<u8> {
    let cb_opcode = cpu.read_next_byte()?;
    super::bit::dispatch(cpu, cb_opcode)
}
