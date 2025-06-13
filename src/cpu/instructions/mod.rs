pub mod arithmetic;
pub mod bit;
pub mod control;
pub mod jump;
pub mod load;
pub mod logic;

use super::CPU;

// 指令週期常量
pub const CYCLES_1: u8 = 4; // 1個機器週期 = 4個時脈週期
pub const CYCLES_2: u8 = 8; // 2個機器週期 = 8個時脈週期
pub const CYCLES_3: u8 = 12; // 3個機器週期 = 12個時脈週期
pub const CYCLES_4: u8 = 16; // 4個機器週期 = 16個時脈週期
pub const CYCLES_5: u8 = 20; // 5個機器週期 = 20個時脈週期
pub const CYCLES_6: u8 = 24; // 6個機器週期 = 24個時脈週期

impl CPU {
    pub fn decode_and_execute(&mut self, opcode: u8) -> u8 {
        match opcode {
            // Load Instructions
            0x01 | 0x02 | 0x06 | 0x0A | 0x0E |           // LD rr,nn; LD (BC),A; LD B,n; LD A,(BC); LD C,n
            0x11 | 0x12 | 0x16 | 0x1A | 0x1E |           // LD DE,nn; LD (DE),A; LD D,n; LD A,(DE); LD E,n
            0x21 | 0x22 | 0x26 | 0x2A | 0x2E |           // LD HL,nn; LD (HL+),A; LD H,n; LD A,(HL+); LD L,n
            0x31 | 0x32 | 0x36 | 0x3A | 0x3E |           // LD SP,nn; LD (HL-),A; LD (HL),n; LD A,(HL-); LD A,n
            0x40..=0x7F |                                 // LD r,r'
            0xE2 | 0xF0 | 0xF2 | 0xF8 | 0xF9 | 0xFA => self.execute_load_instruction(opcode),  // LD (C),A; LDH A,(n); LD A,(C); LD HL,SP+n; LD SP,HL; LD A,(nn)

            // Arithmetic Instructions
            0x03 | 0x04 | 0x0C | 0x0D |                  // INC BC; INC B; INC C; DEC C
            0x13 | 0x14 | 0x1C | 0x1D |                  // INC DE; INC D; INC E; DEC E
            0x23 | 0x24 | 0x2C | 0x2D |                  // INC HL; INC H; INC L; DEC L
            0x33 | 0x34 | 0x3C | 0x3D |                  // INC SP; INC (HL); INC A; DEC A
            0x80..=0x8F |                                 // ADD/ADC instructions
            0x90..=0x9F |                                 // SUB/SBC instructions
            0xC6 | 0xCE | 0xD6 | 0xDE => self.execute_arithmetic_instruction(opcode),  // ADD A,n; ADC A,n; SUB n; SBC A,n

            // Jump Instructions
            0x18 | 0x20 | 0x28 | 0x30 | 0x38 |           // JR conditions
            0xC2 | 0xC3 | 0xC4 | 0xCA | 0xCC | 0xCD |    // JP and CALL conditions
            0xD2 | 0xD4 | 0xDA | 0xDC |                  // More JP and CALL conditions
            0xC9 | 0xC0 | 0xC8 | 0xD0 | 0xD8 |          // RET conditions
            0xD9 | 0xE9 => self.execute_jump_instruction(opcode),  // RETI and JP (HL)

            // Control Instructions
            0x00 | 0x10 | 0x27 | 0x2F | 0x37 | 0x3F |    // NOP, STOP, DAA, CPL, SCF, CCF
            0x76 | 0xF3 | 0xFB |                          // HALT, DI, EI
            0xC7 | 0xCF | 0xD7 | 0xDF |                   // RST instructions
            0xE7 | 0xEF | 0xF7 | 0xFF => self.execute_control_instruction(opcode),

            // Logic & Bit Instructions
            0xA0..=0xAF |                                 // AND/XOR instructions
            0xB0..=0xBF |                                 // OR/CP instructions
            0xE6 | 0xEE | 0xF6 | 0xFE |                  // AND n, XOR n, OR n, CP n
            0xCB => self.execute_logic_instruction(opcode),// Prefix CB bit instructions

            _ => {
                println!(
                    "未實現的指令: 0x{:02X} at PC: 0x{:04X}",
                    opcode,
                    self.registers.pc.wrapping_sub(1)
                );
                CYCLES_1
            }
        }
    }
}
