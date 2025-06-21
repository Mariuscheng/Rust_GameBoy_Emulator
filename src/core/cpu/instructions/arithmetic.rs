// arithmetic.rs - CPU 算術運算指令
// 2025.06.21

use crate::core::{cpu::CPU, cycles::CyclesType};
use crate::error::{Error, InstructionError, RegTarget, Result};

/// 算術運算指令分派
pub fn dispatch(cpu: &mut CPU, opcode: u8) -> Result<CyclesType> {
    match opcode {
        // DEC r 指令族 (05, 0D, 15, 1D, 25, 2D, 35, 3D)
        0x05 | 0x0D | 0x15 | 0x1D | 0x25 | 0x2D | 0x35 | 0x3D => {
            let reg = (opcode >> 3) & 0x07;
            let target = RegTarget::from_bits(reg)?;
            cpu.dec_r(target)
        }

        // ADD/ADC/SUB/SBC 指令族 (0x80-0x9F)
        0x80..=0x9F => {
            let src = (opcode & 0x07) as u8;
            let source = RegTarget::from_bits(src)?;
            let use_carry = (opcode & 0x18) == 0x18 || (opcode & 0x18) == 0x08;
            match opcode & 0xF0 {
                0x80 => cpu.add_a_r(source, false),
                0x90 => cpu.sub_a_r(source, false),
                _ => {
                    if use_carry {
                        if (opcode & 0xF0) == 0x80 {
                            cpu.add_a_r(source, true)
                        } else {
                            cpu.sub_a_r(source, true)
                        }
                    } else {
                        Err(Error::Instruction(InstructionError::InvalidOpcode(opcode)))
                    }
                }
            }
        }

        // 立即數運算指令 (0xC6, 0xCE, 0xD6, 0xDE)
        0xC6 | 0xCE | 0xD6 | 0xDE => {
            let use_carry = opcode == 0xCE || opcode == 0xDE;
            match opcode {
                0xC6 | 0xCE => cpu.add_a_n(use_carry),
                0xD6 | 0xDE => cpu.sub_a_n(use_carry),
                _ => unreachable!(),
            }
        }
        _ => Err(Error::Instruction(InstructionError::InvalidOpcode(opcode))),
    }
}
