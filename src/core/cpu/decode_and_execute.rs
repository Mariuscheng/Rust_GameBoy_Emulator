// Decode and execute instructions

use crate::core::cpu::registers::Registers;
use crate::core::cpu::registers::{CARRY_FLAG, HALF_CARRY_FLAG, SUBTRACT_FLAG, ZERO_FLAG};
use crate::core::mmu::MMU;

pub fn decode_and_execute(registers: &mut Registers, mmu: &mut MMU, opcode: u8) {
    match opcode {
        0x00 => {} // NOP
        0x3C => {
            registers.a = registers.a.wrapping_add(1);
        } // INC A
        0x3D => {
            registers.a = registers.a.wrapping_sub(1);
        } // DEC A
        0x36 => {
            let hl = registers.get_hl();
            let value = match mmu.read_byte(registers.pc) {
                Ok(v) => v,
                Err(e) => {
                    println!(
                        "Error reading byte at PC: 0x{:04X}, error: {:?}",
                        registers.pc, e
                    );
                    return;
                }
            };
            registers.pc += 1;
            println!(
                "LD (HL), n: Writing value 0x{:02X} to address 0x{:04X}",
                value, hl
            );
            mmu.write_byte(hl, value).unwrap_or_else(|e| {
                println!("Error writing byte to address 0x{:04X}, error: {:?}", hl, e);
            });
        } // LD (HL), n
        0x77 => {
            let hl = registers.get_hl();
            println!(
                "LD (HL), A: Writing value 0x{:02X} to address 0x{:04X}",
                registers.a, hl
            );
            mmu.write_byte(hl, registers.a).unwrap_or_else(|e| {
                println!("Error writing byte to address 0x{:04X}, error: {:?}", hl, e);
            });
        } // LD (HL), A
        0x0F => {
            // RRCA: Rotate A right. Old bit 0 to Carry and bit 7
            let carry = registers.a & 0x01;
            registers.a = (registers.a >> 1) | (carry << 7);
            registers.f = (registers.f & 0xEF) | if carry != 0 { 0x10 } else { 0 };
            registers.f &= !ZERO_FLAG & !SUBTRACT_FLAG & !HALF_CARRY_FLAG;
        }
        0x35 => {
            // DEC (HL)
            let hl = registers.get_hl();
            let mut value = mmu.read_byte(hl).unwrap_or(0);
            value = value.wrapping_sub(1);
            mmu.write_byte(hl, value).unwrap_or(());
            registers.update_flags(
                Some(value == 0),
                Some(true),
                Some((value & 0x0F) == 0x0F),
                None,
            );
        }
        0x37 => {
            // SCF: Set carry flag
            registers.f = (registers.f & !(SUBTRACT_FLAG | HALF_CARRY_FLAG)) | CARRY_FLAG;
        }
        0x3F => {
            // CCF: Complement carry flag
            let c = (registers.f & CARRY_FLAG) != 0;
            registers.f = (registers.f & !(SUBTRACT_FLAG | HALF_CARRY_FLAG | CARRY_FLAG))
                | if !c { CARRY_FLAG } else { 0 };
        }
        0x9F => {
            // SBC A, A
            let _carry = if (registers.f & CARRY_FLAG) != 0 {
                1
            } else {
                0
            };
            registers.update_flags(Some(true), Some(true), Some(false), Some(false));
            registers.a = 0;
        }
        0xBF => {
            // CP A
            registers.update_flags(Some(true), Some(true), Some(false), Some(false));
        }
        0xF1 => {
            // POP AF
            let lo = mmu.read_byte(registers.sp).unwrap_or(0);
            registers.sp = registers.sp.wrapping_add(1);
            let hi = mmu.read_byte(registers.sp).unwrap_or(0);
            registers.sp = registers.sp.wrapping_add(1);
            registers.f = lo & 0xF0;
            registers.a = hi;
        }
        0xC4 => {
            // CALL NZ, nn (略過，僅遞增PC)
            registers.pc = registers.pc.wrapping_add(2);
        }
        0xE4 => {
            // CALL PO, nn (略過，僅遞增PC)
            registers.pc = registers.pc.wrapping_add(2);
        }
        0xFF => {
            // RST 38H
            registers.sp = registers.sp.wrapping_sub(2);
            mmu.write_byte(registers.sp, (registers.pc & 0xFF) as u8)
                .unwrap_or(());
            mmu.write_byte(registers.sp + 1, (registers.pc >> 8) as u8)
                .unwrap_or(());
            registers.pc = 0x38;
        }
        0x01 => {
            // LD BC, nn
            let lo = mmu.read_byte(registers.pc).unwrap_or(0);
            registers.pc += 1;
            let hi = mmu.read_byte(registers.pc).unwrap_or(0);
            registers.pc += 1;
            registers.b = hi;
            registers.c = lo;
        }
        0x02 => {
            // LD (BC), A
            let addr = ((registers.b as u16) << 8) | (registers.c as u16);
            mmu.write_byte(addr, registers.a).unwrap_or(());
        }
        0x0A => {
            // LD A, (BC)
            let addr = ((registers.b as u16) << 8) | (registers.c as u16);
            registers.a = mmu.read_byte(addr).unwrap_or(0);
        }
        0x11 => {
            // LD DE, nn
            let lo = mmu.read_byte(registers.pc).unwrap_or(0);
            registers.pc += 1;
            let hi = mmu.read_byte(registers.pc).unwrap_or(0);
            registers.pc += 1;
            registers.d = hi;
            registers.e = lo;
        }
        0x12 => {
            // LD (DE), A
            let addr = ((registers.d as u16) << 8) | (registers.e as u16);
            mmu.write_byte(addr, registers.a).unwrap_or(());
        }
        0x1A => {
            // LD A, (DE)
            let addr = ((registers.d as u16) << 8) | (registers.e as u16);
            registers.a = mmu.read_byte(addr).unwrap_or(0);
        }
        0x21 => {
            // LD HL, nn
            let lo = mmu.read_byte(registers.pc).unwrap_or(0);
            registers.pc += 1;
            let hi = mmu.read_byte(registers.pc).unwrap_or(0);
            registers.pc += 1;
            registers.h = hi;
            registers.l = lo;
        }
        0x31 => {
            // LD SP, nn
            let lo = mmu.read_byte(registers.pc).unwrap_or(0) as u16;
            registers.pc += 1;
            let hi = mmu.read_byte(registers.pc).unwrap_or(0) as u16;
            registers.pc += 1;
            registers.sp = (hi << 8) | lo;
        }
        0xAF => {
            // XOR A
            registers.a = 0;
            registers.update_flags(Some(true), Some(false), Some(false), Some(false));
        }
        0xC3 => {
            // JP nn
            let lo = mmu.read_byte(registers.pc).unwrap_or(0) as u16;
            registers.pc += 1;
            let hi = mmu.read_byte(registers.pc).unwrap_or(0) as u16;
            registers.pc += 1;
            registers.pc = (hi << 8) | lo;
        }
        // ... Add other instructions here ...
        _ => {
            println!(
                "未實現的指令: 0x{:02X} at PC: 0x{:04X}",
                opcode,
                registers.pc - 1
            );
        }
    }
}
