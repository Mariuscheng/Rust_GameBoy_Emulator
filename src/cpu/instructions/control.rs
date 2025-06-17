use super::common::InstructionError;
use super::common::CYCLES_1;
use super::CPU;
use crate::cpu::CpuState;

type Result<T> = std::result::Result<T, InstructionError>;

pub fn dispatch(cpu: &mut CPU, opcode: u8) -> Result<u8> {
    match opcode {
        0x00 => nop(),     // NOP
        0x10 => stop(cpu), // STOP
        0x76 => halt(cpu), // HALT
        0xF3 => di(cpu),   // DI (Disable Interrupts)
        0xFB => ei(cpu),   // EI (Enable Interrupts)
        _ => Err(InstructionError::InvalidOpcode(opcode)),
    }
}

// -- 控制指令實作 --

/// NOP - 無操作
fn nop() -> Result<u8> {
    Ok(CYCLES_1)
}

/// STOP - 停止 CPU 和螢幕顯示直到按鍵輸入
fn stop(cpu: &mut CPU) -> Result<u8> {
    cpu.state = CpuState::Stopped;
    Ok(CYCLES_1)
}

/// HALT - 暫停 CPU 直到發生中斷
fn halt(cpu: &mut CPU) -> Result<u8> {
    cpu.state = CpuState::Halted;
    Ok(CYCLES_1)
}

/// DI - 禁用中斷
fn di(cpu: &mut CPU) -> Result<u8> {
    cpu.ime = false;
    Ok(CYCLES_1)
}

/// EI - 啟用中斷
fn ei(cpu: &mut CPU) -> Result<u8> {
    cpu.ime = true;
    Ok(CYCLES_1)
}
