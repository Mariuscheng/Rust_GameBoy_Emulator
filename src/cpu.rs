// filepath: c:\Users\mariu\Desktop\Rust\gameboy_emulator\src\cpu.rs
use crate::mmu::MMU;
use std::collections::HashSet;
use std::sync::Mutex;

lazy_static::lazy_static! {
    pub static ref UNIMPL_OPCODES: Mutex<HashSet<u8>> = Mutex::new(HashSet::new());
}

#[allow(dead_code)]
pub struct Registers {
    pub a: u8,
    pub f: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub sp: u16,
    pub pc: u16,
}

impl Default for Registers {
    fn default() -> Self {
        Registers {
            a: 0,
            f: 0,
            b: 0,
            c: 0,
            d: 0,
            e: 0,
            h: 0,
            l: 0,
            sp: 0,
            pc: 0,
    }
}
}
impl Registers {
    // Zero Flag (Z)
    pub fn get_z(&self) -> bool { self.f & 0x80 != 0 }
    pub fn set_z(&mut self, val: bool) {
        if val { self.f |= 0x80; } else { self.f &= !0x80; }
    }
    // Subtract Flag (N)
    pub fn get_n(&self) -> bool { self.f & 0x40 != 0 }
    pub fn set_n(&mut self, val: bool) {
        if val { self.f |= 0x40; } else { self.f &= !0x40; }
    }
    // Half Carry Flag (H)
    pub fn get_h(&self) -> bool { self.f & 0x20 != 0 }
    pub fn set_h(&mut self, val: bool) {
        if val { self.f |= 0x20; } else { self.f &= !0x20; }
    }
    // Carry Flag (C)
    pub fn get_c(&self) -> bool { self.f & 0x10 != 0 }
    pub fn set_c(&mut self, val: bool) {
        if val { self.f |= 0x10; } else { self.f &= !0x10; }
    }
    // 清除所有旗標
    #[allow(dead_code)]
    pub fn clear_flags(&mut self) {
        self.f = 0;
    }
}


pub struct CPU {
    pub registers: Registers,
    pub mmu: MMU,
    pub ime: bool, // Interrupt Master Enable flag
    pub halted: bool, // CPU halted state
    pub stopped: bool, // CPU stopped state
}

impl CPU {

    #[allow(dead_code)]
    fn handle_interrupts(&mut self) {
        let fired = self.mmu.get_if() & self.mmu.get_ie();
        if self.ime {
            if fired != 0 {
                self.ime = false;
                // 依優先順序處理
                for i in 0..5 {
                    if fired & (1 << i) != 0 {
                        self.mmu.if_reg &= !(1 << i);
                        // push PC
                        self.registers.sp = self.registers.sp.wrapping_sub(2);
                        self.mmu.write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                        self.mmu.write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
                        // 跳到中斷向量
                        self.registers.pc = match i {
                            0 => 0x40,  // VBlank
                            1 => 0x48,  // LCD STAT
                            2 => 0x50,  // Timer
                            3 => 0x58,  // Serial
                            4 => 0x60,  // Joypad
                            _ => 0,
                        };
                        break;
                    }
                }
            }
        }
    }
    pub fn execute(&mut self) {
        let opcode = self.fetch();
        self.decode_and_execute(opcode);
    }

    pub fn step(&mut self) {
        // let opcode = self.mmu.read_byte(self.registers.pc);
        // println!("shade: {}, rgb: {:06X}", shade, rgb);
        if self.halted || self.stopped {
            return;
        }
        self.execute();
        // let _pos = (self.registers.pc as usize) % 160;
        //println!("PC={:04X} OPCODE={:02X}", self.registers.pc, opcode);
        //println!("VRAM[0..16]: {:?}", &self.mmu.vram[0..16]);
    }


    pub fn new(mmu: MMU) -> Self {
        CPU {
            registers: Registers::default(),
            mmu,
            ime: false, // default to disabled
            halted: false, // default to not halted
            stopped: false, // default to not stopped
        }
    }
    

    pub fn load_rom(&mut self, rom: &[u8]) {
        self.mmu.load_rom(rom);
    }

    fn fetch(&mut self) -> u8 {
        let opcode = self.mmu.read_byte(self.registers.pc);
        self.registers.pc += 1;
        opcode
    }

    fn decode_and_execute(&mut self, opcode: u8) {
        match opcode {
            0xCB => { // CB prefix for extended instructions
                let cb_opcode = self.fetch();
                self.decode_cb(cb_opcode);
            }
            0x00 => {} // NOP
            0x3C => { self.registers.a = self.registers.a.wrapping_add(1); } // INC A
            0x3D => { self.registers.a = self.registers.a.wrapping_sub(1); } // DEC A
            0x04 => { self.registers.b = self.registers.b.wrapping_add(1); } // INC B
            0x05 => { self.registers.b = self.registers.b.wrapping_sub(1); } // DEC B
            0x06 => { let n = self.fetch(); self.registers.b = n; } // LD B, n
            0x08 => { // LD (nn),SP
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                self.mmu.write_byte(addr, (self.registers.sp & 0xFF) as u8);
                self.mmu.write_byte(addr + 1, (self.registers.sp >> 8) as u8);
            }
            0x09 => { // ADD HL,BC
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let bc = ((self.registers.b as u16) << 8) | self.registers.c as u16;
                let result = hl.wrapping_add(bc);
                self.registers.h = (result >> 8) as u8;
                self.registers.l = result as u8;
            }
            0x0E => { let n = self.fetch(); self.registers.c = n; } // LD C, n
            0x16 => { let n = self.fetch(); self.registers.d = n; } // LD D, n
            0x1E => { let n = self.fetch(); self.registers.e = n; } // LD E, n
            0x26 => { let n = self.fetch(); self.registers.h = n; } // LD H, n
            0x2E => { let n = self.fetch(); self.registers.l = n; } // LD L, n
            0xAF => { // XOR A
                let result = self.registers.a ^ self.registers.a;
                self.registers.a = result;
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h(false);
                self.registers.set_c(false);
            }
            0xC3 => { // JP nn
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0x78 => { self.registers.a = self.registers.b; } // LD A, B
            0x79 => { self.registers.a = self.registers.c; } // LD A, C
            0x7A => { self.registers.a = self.registers.d; } // LD A, D
            0x7B => { self.registers.a = self.registers.e; } // LD A, E
            0x7C => { self.registers.a = self.registers.h; } // LD A, H
            0x7D => { self.registers.a = self.registers.l; } // LD A, L
            0x7F => { /* LD A, A (無動作) */ }
            0x40 => { self.registers.b = self.registers.b; } // LD B,B
            0x41 => { self.registers.b = self.registers.c; } // LD B,C
            0x42 => { self.registers.b = self.registers.d; } // LD B,D
            0x43 => { self.registers.b = self.registers.e; } // LD B,E
            0x44 => { self.registers.b = self.registers.h; } // LD B,H
            0x45 => { self.registers.b = self.registers.l; } // LD B,L
                        // LD r, r' 範例（0x40~0x7F，部分）
            0x46 => { // LD B, (HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.b = self.mmu.read_byte(hl);
            }
            0x48 => { self.registers.c = self.registers.b; } // LD C, B
            0x49 => { self.registers.c = self.registers.c; } // LD C, C
            0x4A => { self.registers.c = self.registers.d; } // LD C, D
            0x4B => { self.registers.c = self.registers.e; } // LD C, E
            0x4C => { self.registers.c = self.registers.h; } // LD C, H
            0x4D => { self.registers.c = self.registers.l; } // LD C, L
            0x4E => { // LD C, (HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.c = self.mmu.read_byte(hl);
            }
            0x50 => { self.registers.d = self.registers.b; } // LD D, B
            0x51 => { self.registers.d = self.registers.c; } // LD D, C
            0x52 => { self.registers.d = self.registers.d; } // LD D, D
            0x53 => { self.registers.d = self.registers.e; } // LD D, E
            0x54 => { self.registers.d = self.registers.h; } // LD D, H
            0x55 => { self.registers.d = self.registers.l; } // LD D, L
            0x56 => { // LD D, (HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.d = self.mmu.read_byte(hl);
            }
            0x58 => { self.registers.e = self.registers.b; } // LD E, B
            0x59 => { self.registers.e = self.registers.c; } // LD E, C
            0x5A => { self.registers.e = self.registers.d; } // LD E, D
            0x5B => { self.registers.e = self.registers.e; } // LD E, E
            0x5C => { self.registers.e = self.registers.h; } // LD E, H
            0x5D => { self.registers.e = self.registers.l; } // LD E, L
            0x5E => { // LD E, (HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.e = self.mmu.read_byte(hl);
            }
            0x60 => { self.registers.h = self.registers.b; } // LD H, B
            0x61 => { self.registers.h = self.registers.c; } // LD H, C
            0x62 => { self.registers.h = self.registers.d; } // LD H, D
            0x63 => { self.registers.h = self.registers.e; } // LD H, E
            0x64 => { self.registers.h = self.registers.h; } // LD H, H
            0x65 => { self.registers.h = self.registers.l; } // LD H, L
            0x66 => { // LD H, (HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.h = self.mmu.read_byte(hl);
            }
            0x68 => { self.registers.l = self.registers.b; } // LD L, B
            0x69 => { self.registers.l = self.registers.c; } // LD L, C
            0x6A => { self.registers.l = self.registers.d; } // LD L, D
            0x6B => { self.registers.l = self.registers.e; } // LD L, E
            0x6C => { self.registers.l = self.registers.h; } // LD L, H
            0x6D => { self.registers.l = self.registers.l; } // LD L, L
            0x6E => { // LD L, (HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.l = self.mmu.read_byte(hl);
            }
            0x70 => { // LD (HL), B
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.mmu.write_byte(hl, self.registers.b);
            }
            0x71 => { // LD (HL), C
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.mmu.write_byte(hl, self.registers.c);
            }
            0x72 => { // LD (HL), D
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.mmu.write_byte(hl, self.registers.d);
            }
            0x73 => { // LD (HL), E
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.mmu.write_byte(hl, self.registers.e);
            }
            0x74 => { // LD (HL), H
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.mmu.write_byte(hl, self.registers.h);
            }
            0x75 => { // LD (HL), L
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.mmu.write_byte(hl, self.registers.l);
            }
            
            // ADD A, r 範例（0x80~0x87）
            0x80 => { // ADD A, B
    let a = self.registers.a;
    let b = self.registers.b;
    let (result, carry) = a.overflowing_add(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(((a & 0xF) + (b & 0xF)) > 0xF);
    self.registers.set_c(carry);
    self.registers.a = result;
}
0x81 => { // ADD A, C
    let a = self.registers.a;
    let b = self.registers.c;
    let (result, carry) = a.overflowing_add(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(((a & 0xF) + (b & 0xF)) > 0xF);
    self.registers.set_c(carry);
    self.registers.a = result;
}
0x82 => { // ADD A, D
    let a = self.registers.a;
    let b = self.registers.d;
    let (result, carry) = a.overflowing_add(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(((a & 0xF) + (b & 0xF)) > 0xF);
    self.registers.set_c(carry);
    self.registers.a = result;
}
0x83 => { // ADD A, E
    let a = self.registers.a;
    let b = self.registers.e;
    let (result, carry) = a.overflowing_add(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(((a & 0xF) + (b & 0xF)) > 0xF);
    self.registers.set_c(carry);
    self.registers.a = result;
}
0x84 => { // ADD A, H
    let a = self.registers.a;
    let b = self.registers.h;
    let (result, carry) = a.overflowing_add(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(((a & 0xF) + (b & 0xF)) > 0xF);
    self.registers.set_c(carry);
    self.registers.a = result;
}
0x85 => { // ADD A, L
    let a = self.registers.a;
    let b = self.registers.l;
    let (result, carry) = a.overflowing_add(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(((a & 0xF) + (b & 0xF)) > 0xF);
    self.registers.set_c(carry);
    self.registers.a = result;
}
0x86 => { // ADD A, (HL)
    let a = self.registers.a;
    let b = self.mmu.read_byte(((self.registers.h as u16) << 8) | self.registers.l as u16);
    let (result, carry) = a.overflowing_add(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(((a & 0xF) + (b & 0xF)) > 0xF);
    self.registers.set_c(carry);
    self.registers.a = result;
}
0x87 => { // ADD A, A
    let a = self.registers.a;
    let b = self.registers.a;
    let (result, carry) = a.overflowing_add(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(((a & 0xF) + (b & 0xF)) > 0xF);
    self.registers.set_c(carry);
    self.registers.a = result;
}
                        // SUB A, r
            0x90 => { // SUB B
    let a = self.registers.a;
    let b = self.registers.b;
    let (result, borrow) = a.overflowing_sub(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h((a & 0xF) < (b & 0xF));
    self.registers.set_c(borrow);
    self.registers.a = result;
}
0x91 => { // SUB C
    let a = self.registers.a;
    let b = self.registers.c;
    let (result, borrow) = a.overflowing_sub(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h((a & 0xF) < (b & 0xF));
    self.registers.set_c(borrow);
}
0x92 => { // SUB D
    let a = self.registers.a;
    let b = self.registers.d;
    let (result, borrow) = a.overflowing_sub(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h((a & 0xF) < (b & 0xF));
    self.registers.set_c(borrow);
}
0x93 => { // SUB E
    let a = self.registers.a;
    let b = self.registers.e;
    let (result, borrow) = a.overflowing_sub(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h((a & 0xF) < (b & 0xF));
    self.registers.set_c(borrow);
}
0x94 => { // SUB H
    let a = self.registers.a;
    let b = self.registers.h;
    let (result, borrow) = a.overflowing_sub(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h((a & 0xF) < (b & 0xF));
    self.registers.set_c(borrow);
}
0x95 => { // SUB L
    let a = self.registers.a;
    let b = self.registers.l;
    let (result, borrow) = a.overflowing_sub(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h((a & 0xF) < (b & 0xF));
    self.registers.set_c(borrow);
}
0x96 => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let b = self.mmu.read_byte(hl);
                let a = self.registers.a;
                let (result, borrow) = a.overflowing_sub(b);
                self.registers.set_z(result == 0);
                self.registers.set_n(true);
                self.registers.set_h((a & 0xF) < (b & 0xF));
                self.registers.set_c(borrow);
                self.registers.a = result;
            }
0x97 => { self.registers.a = self.registers.a.wrapping_sub(self.registers.a); }
            
            // AND A, r
            0xA0 => { // AND B
                let result = self.registers.a & self.registers.b;
                self.registers.a = result;
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h(true);
                self.registers.set_c(false);
            }
0xA1 => { // AND C
    let result = self.registers.a & self.registers.c;
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(true);
    self.registers.set_c(false);
}
0xA2 => { // AND D
    let result = self.registers.a & self.registers.d;
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(true);
    self.registers.set_c(false);
}
0xA3 => { // AND E
    let result = self.registers.a & self.registers.e;
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(true);
    self.registers.set_c(false);
}
0xA4 => { // AND H
    let result = self.registers.a & self.registers.h;
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(true);
    self.registers.set_c(false);
}
0xA5 => { // AND L
    let result = self.registers.a & self.registers.l;
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(true);
    self.registers.set_c(false);
}
0xA6 => { // AND (HL)
    let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
    let result = self.registers.a & self.mmu.read_byte(hl);
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(true);
    self.registers.set_c(false);
}
0xA7 => { // AND A
    let result = self.registers.a & self.registers.a;
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(true);
    self.registers.set_c(false);
}

// OR A, r
0xB0 => { // OR B
    let result = self.registers.a | self.registers.b;
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(false);
    self.registers.set_c(false);
}
0xB1 => { // OR C
    let result = self.registers.a | self.registers.c;
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(false);
    self.registers.set_c(false);
}
0xB2 => { // OR D
    let result = self.registers.a | self.registers.d;
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(false);
    self.registers.set_c(false);
}
0xB3 => { // OR E
    let result = self.registers.a | self.registers.e;
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(false);
    self.registers.set_c(false);
}
0xB4 => { // OR H
    let result = self.registers.a | self.registers.h;
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(false);
    self.registers.set_c(false);
}
0xB5 => { // OR L
    let result = self.registers.a | self.registers.l;
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(false);
    self.registers.set_c(false);
}
0xB6 => { // OR (HL)
    let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
    let result = self.registers.a | self.mmu.read_byte(hl);
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(false);
    self.registers.set_c(false);
}
0xB7 => { // OR A
    let result = self.registers.a | self.registers.a;
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(false);
    self.registers.set_c(false);
}

// XOR A, r
0xA8 => { // XOR B
    let result = self.registers.a ^ self.registers.b;
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(false);
    self.registers.set_c(false);
}
0xA9 => { // XOR C
    let result = self.registers.a ^ self.registers.c;
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(false);
    self.registers.set_c(false);
}
0xAA => { // XOR D
    let result = self.registers.a ^ self.registers.d;
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(false);
    self.registers.set_c(false);
}
0xAB => { // XOR E
    let result = self.registers.a ^ self.registers.e;
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(false);
    self.registers.set_c(false);
}
0xAC => { // XOR H
    let result = self.registers.a ^ self.registers.h;
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(false);
    self.registers.set_c(false);
}
0xAD => { // XOR L
    let result = self.registers.a ^ self.registers.l;
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(false);
    self.registers.set_c(false);
}
0xAE => { // XOR (HL)
    let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
    let result = self.registers.a ^ self.mmu.read_byte(hl);
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(false);
    self.registers.set_c(false);
}
// CP A, r
0xB8 => { // CP B
    let a = self.registers.a;
    let b = self.registers.b;
    let (result, borrow) = a.overflowing_sub(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h((a & 0xF) < (b & 0xF));
    self.registers.set_c(borrow);
}
0xB9 => { // CP C
    let a = self.registers.a;
    let b = self.registers.c;
    let (result, borrow) = a.overflowing_sub(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h((a & 0xF) < (b & 0xF));
    self.registers.set_c(borrow);
}
0xBA => { // CP D
    let a = self.registers.a;
    let b = self.registers.d;
    let (result, borrow) = a.overflowing_sub(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h((a & 0xF) < (b & 0xF));
    self.registers.set_c(borrow);
}
0xBB => { // CP E
    let a = self.registers.a;
    let b = self.registers.e;
    let (result, borrow) = a.overflowing_sub(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h((a & 0xF) < (b & 0xF));
    self.registers.set_c(borrow);
}
0xBC => { // CP H
    let a = self.registers.a;
    let b = self.registers.h;
    let (result, borrow) = a.overflowing_sub(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h((a & 0xF) < (b & 0xF));
    self.registers.set_c(borrow);
}
0xBD => { // CP L
    let a = self.registers.a;
    let b = self.registers.l;
    let (result, borrow) = a.overflowing_sub(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h((a & 0xF) < (b & 0xF));
    self.registers.set_c(borrow);
}
0xBE => { // CP (HL)
    let a = self.registers.a;
    let b = self.mmu.read_byte(((self.registers.h as u16) << 8) | self.registers.l as u16);
    let (result, borrow) = a.overflowing_sub(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h((a & 0xF) < (b & 0xF));
    self.registers.set_c(borrow);
}
0xBF => { // CP A
    let a = self.registers.a;
    let b = self.registers.a;
    let (result, borrow) = a.overflowing_sub(b);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h((a & 0xF) < (b & 0xF));
    self.registers.set_c(borrow);
}
// ADC A, r
0x88 => { // ADC A, B
    let a = self.registers.a;
    let b = self.registers.b;
    let c = if self.registers.get_c() { 1 } else { 0 };
    let (sum1, carry1) = a.overflowing_add(b);
    let (result, carry2) = sum1.overflowing_add(c);
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(((a & 0xF) + (b & 0xF) + c) > 0xF);
    self.registers.set_c(carry1 || carry2);
    self.registers.a = result;
}
0x89 => { // ADC A, C
    let a = self.registers.a;
    let b = self.registers.c;
    let c = if self.registers.get_c() { 1 } else { 0 };
    let (sum1, carry1) = a.overflowing_add(b);
    let (result, carry2) = sum1.overflowing_add(c);
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(((a & 0xF) + (b & 0xF) + c) > 0xF);
    self.registers.set_c(carry1 || carry2);
    self.registers.a = result;
}
0x8A => { // ADC A, D
    let a = self.registers.a;
    let b = self.registers.d;
    let c = if self.registers.get_c() { 1 } else { 0 };
    let (sum1, carry1) = a.overflowing_add(b);
    let (result, carry2) = sum1.overflowing_add(c);
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(((a & 0xF) + (b & 0xF) + c) > 0xF);
    self.registers.set_c(carry1 || carry2);
    self.registers.a = result;
}
0x8B => { // ADC A, E
    let a = self.registers.a;
    let b = self.registers.e;
    let c = if self.registers.get_c() { 1 } else { 0 };
    let (sum1, carry1) = a.overflowing_add(b);
    let (result, carry2) = sum1.overflowing_add(c);
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(((a & 0xF) + (b & 0xF) + c) > 0xF);
    self.registers.set_c(carry1 || carry2);
    self.registers.a = result;
}
0x8C => { // ADC A, H
    let a = self.registers.a;
    let b = self.registers.h;
    let c = if self.registers.get_c() { 1 } else { 0 };
    let (sum1, carry1) = a.overflowing_add(b);
    let (result, carry2) = sum1.overflowing_add(c);
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(((a & 0xF) + (b & 0xF) + c) > 0xF);
    self.registers.set_c(carry1 || carry2);
    self.registers.a = result;
}
0x8D => { // ADC A, L
    let a = self.registers.a;
    let b = self.registers.l;
    let c = if self.registers.get_c() { 1 } else { 0 };
    let (sum1, carry1) = a.overflowing_add(b);
    let (result, carry2) = sum1.overflowing_add(c);
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(((a & 0xF) + (b & 0xF) + c) > 0xF);
    self.registers.set_c(carry1 || carry2);
    self.registers.a = result;
}
0x8E => { // ADC A, (HL)
    let a = self.registers.a;
    let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
    let b = self.mmu.read_byte(hl);
    let c = if self.registers.get_c() { 1 } else { 0 };
    let (sum1, carry1) = a.overflowing_add(b);
    let (result, carry2) = sum1.overflowing_add(c);
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(((a & 0xF) + (b & 0xF) + c) > 0xF);
    self.registers.set_c(carry1 || carry2);
    self.registers.a = result;
}
0x8F => { // ADC A, A
    let a = self.registers.a;
    let b = self.registers.a;
    let c = if self.registers.get_c() { 1 } else { 0 };
    let (sum1, carry1) = a.overflowing_add(b);
    let (result, carry2) = sum1.overflowing_add(c);
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(((a & 0xF) + (b & 0xF) + c) > 0xF);
    self.registers.set_c(carry1 || carry2);
    self.registers.a = result;
}

// SBC A, r
0x98 => { // SBC A, B
    let a = self.registers.a;
    let b = self.registers.b;
    let c = if self.registers.get_c() { 1 } else { 0 };
    let (sub1, borrow1) = a.overflowing_sub(b);
    let (result, borrow2) = sub1.overflowing_sub(c);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h(((a & 0xF).wrapping_sub(b & 0xF).wrapping_sub(c)) > 0xF);
    self.registers.set_c(borrow1 || borrow2);
    self.registers.a = result;
}
0x99 => { // SBC A, C
    let a = self.registers.a;
    let b = self.registers.c;
    let c = if self.registers.get_c() { 1 } else { 0 };
    let (sub1, borrow1) = a.overflowing_sub(b);
    let (result, borrow2) = sub1.overflowing_sub(c);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h(((a & 0xF).wrapping_sub(b & 0xF).wrapping_sub(c)) > 0xF);
    self.registers.set_c(borrow1 || borrow2);
    self.registers.a = result;
}
0x9A => { // SBC A, D
    let a = self.registers.a;
    let b = self.registers.d;
    let c = if self.registers.get_c() { 1 } else { 0 };
    let (sub1, borrow1) = a.overflowing_sub(b);
    let (result, borrow2) = sub1.overflowing_sub(c);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h(((a & 0xF).wrapping_sub(b & 0xF).wrapping_sub(c)) > 0xF);
    self.registers.set_c(borrow1 || borrow2);
    self.registers.a = result;
}
0x9B => { // SBC A, E
    let a = self.registers.a;
    let b = self.registers.e;
    let c = if self.registers.get_c() { 1 } else { 0 };
    let (sub1, borrow1) = a.overflowing_sub(b);
    let (result, borrow2) = sub1.overflowing_sub(c);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h(((a & 0xF).wrapping_sub(b & 0xF).wrapping_sub(c)) > 0xF);
    self.registers.set_c(borrow1 || borrow2);
    self.registers.a = result;
}
0x9C => { // SBC A, H
    let a = self.registers.a;
    let b = self.registers.h;
    let c = if self.registers.get_c() { 1 } else { 0 };
    let (sub1, borrow1) = a.overflowing_sub(b);
    let (result, borrow2) = sub1.overflowing_sub(c);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h(((a & 0xF).wrapping_sub(b & 0xF).wrapping_sub(c)) > 0xF);
    self.registers.set_c(borrow1 || borrow2);
    self.registers.a = result;
}
0x9D => { // SBC A, L
    let a = self.registers.a;
    let b = self.registers.l;
    let c = if self.registers.get_c() { 1 } else { 0 };
    let (sub1, borrow1) = a.overflowing_sub(b);
    let (result, borrow2) = sub1.overflowing_sub(c);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h(((a & 0xF).wrapping_sub(b & 0xF).wrapping_sub(c)) > 0xF);
    self.registers.set_c(borrow1 || borrow2);
    self.registers.a = result;
}
0x9E => { // SBC A, (HL)
    let a = self.registers.a;
    let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
    let b = self.mmu.read_byte(hl);
    let c = if self.registers.get_c() { 1 } else { 0 };
    let (sub1, borrow1) = a.overflowing_sub(b);
    let (result, borrow2) = sub1.overflowing_sub(c);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h(((a & 0xF).wrapping_sub(b & 0xF).wrapping_sub(c)) > 0xF);
    self.registers.set_c(borrow1 || borrow2);
    self.registers.a = result;
}
0x9F => { // SBC A, A
    let a = self.registers.a;
    let b = self.registers.a;
    let c = if self.registers.get_c() { 1 } else { 0 };
    let (sub1, borrow1) = a.overflowing_sub(b);
    let (result, borrow2) = sub1.overflowing_sub(c);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h(((a & 0xF).wrapping_sub(b & 0xF).wrapping_sub(c)) > 0xF);
    self.registers.set_c(borrow1 || borrow2);
    self.registers.a = result;
}
// PUSH/POP 指令
0xC5 => { // PUSH BC
    let bc = ((self.registers.b as u16) << 8) | self.registers.c as u16;
    self.registers.sp = self.registers.sp.wrapping_sub(2);
    self.mmu.write_byte(self.registers.sp, (bc & 0xFF) as u8);
    self.mmu.write_byte(self.registers.sp + 1, (bc >> 8) as u8);
}
0xD5 => { // PUSH DE
    let de = ((self.registers.d as u16) << 8) | self.registers.e as u16;
    self.registers.sp = self.registers.sp.wrapping_sub(2);
    self.mmu.write_byte(self.registers.sp, (de & 0xFF) as u8);
    self.mmu.write_byte(self.registers.sp + 1, (de >> 8) as u8);
}
0xE5 => { // PUSH HL
    let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
    self.registers.sp = self.registers.sp.wrapping_sub(2);
    self.mmu.write_byte(self.registers.sp, (hl & 0xFF) as u8);
    self.mmu.write_byte(self.registers.sp + 1, (hl >> 8) as u8);
}
0xF5 => { // PUSH AF
    let af = ((self.registers.a as u16) << 8) | self.registers.f as u16;
    self.registers.sp = self.registers.sp.wrapping_sub(2);
    self.mmu.write_byte(self.registers.sp, (af & 0xFF) as u8);
    self.mmu.write_byte(self.registers.sp + 1, (af >> 8) as u8);
}
0xC1 => { // POP BC
    let lo = self.mmu.read_byte(self.registers.sp) as u16;
    let hi = self.mmu.read_byte(self.registers.sp + 1) as u16;
    self.registers.b = (hi >> 8) as u8;
    self.registers.c = lo as u8;
    self.registers.sp = self.registers.sp.wrapping_add(2);
}
0xD1 => { // POP DE
    let lo = self.mmu.read_byte(self.registers.sp) as u16;
    let hi = self.mmu.read_byte(self.registers.sp + 1) as u16;
    self.registers.d = (hi >> 8) as u8;
    self.registers.e = lo as u8;
    self.registers.sp = self.registers.sp.wrapping_add(2);
}
0xE1 => { // POP HL
    let lo = self.mmu.read_byte(self.registers.sp) as u16;
    let hi = self.mmu.read_byte(self.registers.sp + 1) as u16;
    self.registers.h = (hi >> 8) as u8;
    self.registers.l = lo as u8;
    self.registers.sp = self.registers.sp.wrapping_add(2);
}
0xF1 => { // POP AF
    let lo = self.mmu.read_byte(self.registers.sp) as u16;
    let hi = self.mmu.read_byte(self.registers.sp + 1) as u16;
    self.registers.a = (hi >> 8) as u8;
    self.registers.f = lo as u8 & 0xF0; // F低4位永遠為0
    self.registers.sp = self.registers.sp.wrapping_add(2);
}
// CALL nn
0xCD => {
    let lo = self.fetch() as u16;
    let hi = self.fetch() as u16;
    let addr = (hi << 8) | lo;
    self.registers.sp = self.registers.sp.wrapping_sub(2);
    self.mmu.write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
    self.mmu.write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
    self.registers.pc = addr;
}

// RET
0xC9 => {
    let lo = self.mmu.read_byte(self.registers.sp) as u16;
    let hi = self.mmu.read_byte(self.registers.sp + 1) as u16;
    self.registers.pc = (hi << 8) | lo;
    self.registers.sp = self.registers.sp.wrapping_add(2);
}

// JP (HL)
0xE9 => {
    let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
    self.registers.pc = hl;
}

// JR n
0x18 => {
    let offset = self.fetch() as i8;
    self.registers.pc = (self.registers.pc as i16).wrapping_add(offset as i16) as u16;
}

// JR Z, n
0x28 => {
    let offset = self.fetch() as i8;
    if self.registers.get_z() {
        self.registers.pc = (self.registers.pc as i16).wrapping_add(offset as i16) as u16;
    }
}

// JR NZ, n
0x20 => {
    let offset = self.fetch() as i8;
    if !self.registers.get_z() {
        self.registers.pc = (self.registers.pc as i16).wrapping_add(offset as i16) as u16;
    }
}

// JR C, n
0x38 => {
    let offset = self.fetch() as i8;
    if self.registers.get_c() {
        self.registers.pc = (self.registers.pc as i16).wrapping_add(offset as i16) as u16;
    }
}

// JR NC, n
0x30 => {
    let offset = self.fetch() as i8;
    if !self.registers.get_c() {
        self.registers.pc = (self.registers.pc as i16).wrapping_add(offset as i16) as u16;
    }
}
// LD (nn),A
0xEA => { // LD (nn),A
    let lo = self.fetch();
    let hi = self.fetch();
    let addr = ((hi as u16) << 8) | (lo as u16);
    self.mmu.write_byte(addr, self.registers.a);
}

// LD A,(nn)
0xFA => { // LD A,(nn)
    let lo = self.fetch();
    let hi = self.fetch();
    let addr = ((hi as u16) << 8) | (lo as u16);
    self.registers.a = self.mmu.read_byte(addr);
}

// LD (C),A
0xE2 => {
    let addr = 0xFF00 + self.registers.c as u16;
    self.mmu.write_byte(addr, self.registers.a);
}

// LD A,(C)
0xF2 => {
    let addr = 0xFF00 + self.registers.c as u16;
    self.registers.a = self.mmu.read_byte(addr);
}

// INC 16-bit
0x03 => { // INC BC
    let bc = ((self.registers.b as u16) << 8) | self.registers.c as u16;
    let bc = bc.wrapping_add(1);
    self.registers.b = (bc >> 8) as u8;
    self.registers.c = bc as u8;
}
0x13 => { // INC DE
    let de = ((self.registers.d as u16) << 8) | self.registers.e as u16;
    let de = de.wrapping_add(1);
    self.registers.d = (de >> 8) as u8;
    self.registers.e = de as u8;
}
0x23 => { // INC HL
    let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
    let hl = hl.wrapping_add(1);
    self.registers.h = (hl >> 8) as u8;
    self.registers.l = hl as u8;
}
0x33 => { // INC SP
    self.registers.sp = self.registers.sp.wrapping_add(1);
}

// DEC 16-bit
0x0B => { // DEC BC
    let bc = ((self.registers.b as u16) << 8) | self.registers.c as u16;
    let bc = bc.wrapping_sub(1);
    self.registers.b = (bc >> 8) as u8;
    self.registers.c = bc as u8;
}
0x1B => { // DEC DE
    let de = ((self.registers.d as u16) << 8) | self.registers.e as u16;
    let de = de.wrapping_sub(1);
    self.registers.d = (de >> 8) as u8;
    self.registers.e = de as u8;
}
0x2B => { // DEC HL
    let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
    let hl = hl.wrapping_sub(1);
    self.registers.h = (hl >> 8) as u8;
    self.registers.l = hl as u8;
}
0x3B => { // DEC SP
    self.registers.sp = self.registers.sp.wrapping_sub(1);
}

// DI/EI
0xF3 => { self.ime = false; } // DI
0xFB => { self.ime = true; }  // EI

// HALT
0x76 => {self.halted = true;}

// STOP 指令通常會進入低功耗狀態，這裡可設一個 flag
0x10 => {self.stopped = true;}
0x3E => {
    let n = self.fetch();
    self.registers.a = n;
}
0x77 => {
    let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
    self.mmu.write_byte(hl, self.registers.a);
}
0x21 => { // LD HL,nn
    let lo = self.fetch();
    let hi = self.fetch();
    self.registers.h = hi;
    self.registers.l = lo;
}
0x01 => { // LD BC,nn
    let lo = self.fetch();
    let hi = self.fetch();
    self.registers.b = hi;
    self.registers.c = lo;
}
0x11 => { // LD DE,nn
    let lo = self.fetch();
    let hi = self.fetch();
    self.registers.d = hi;
    self.registers.e = lo;
}
0x19 => { // ADD HL,DE
    let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
    let de = ((self.registers.d as u16) << 8) | self.registers.e as u16;
    let result = hl.wrapping_add(de);
    self.registers.h = (result >> 8) as u8;
    self.registers.l = result as u8;
}
0x0A => { // LD A,(BC)
    let addr = ((self.registers.b as u16) << 8) | self.registers.c as u16;
    self.registers.a = self.mmu.read_byte(addr);
}
0x02 => { // LD (BC),A
    let addr = ((self.registers.b as u16) << 8) | self.registers.c as u16;
    self.mmu.write_byte(addr, self.registers.a);
}
0x12 => { // LD (DE),A
    let addr = ((self.registers.d as u16) << 8) | self.registers.e as u16;
    self.mmu.write_byte(addr, self.registers.a);
}
0x22 => { // LD (HL+),A
    let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
    self.mmu.write_byte(hl, self.registers.a);
    let hl = hl.wrapping_add(1);
    self.registers.h = (hl >> 8) as u8;
    self.registers.l = hl as u8;
}
0x2A => { // LD A,(HL+)
    let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
    self.registers.a = self.mmu.read_byte(hl);
    let hl = hl.wrapping_add(1);
    self.registers.h = (hl >> 8) as u8;
    self.registers.l = hl as u8;
}
0x32 => { // LD (HL-),A
    let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
    self.mmu.write_byte(hl, self.registers.a);
    let hl = hl.wrapping_sub(1);
    self.registers.h = (hl >> 8) as u8;
    self.registers.l = hl as u8;
}
0x36 => { // LD (HL),n
    let n = self.fetch();
    let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
    self.mmu.write_byte(hl, n);
}
0x24 => { // INC H
    self.registers.h = self.registers.h.wrapping_add(1);
}
0x25 => { // DEC H
    self.registers.h = self.registers.h.wrapping_sub(1);
}
0x2C => { // INC L
    self.registers.l = self.registers.l.wrapping_add(1);
}
0x2D => { // DEC L
    self.registers.l = self.registers.l.wrapping_sub(1);
}
0x34 => { // INC (HL)
    let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
    let val = self.mmu.read_byte(hl).wrapping_add(1);
    self.mmu.write_byte(hl, val);
}
0x35 => { // DEC (HL)
    let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
    let val = self.mmu.read_byte(hl).wrapping_sub(1);
    self.mmu.write_byte(hl, val);
}
0x31 => { // LD SP,nn
    let lo = self.fetch() as u16;
    let hi = self.fetch() as u16;
    self.registers.sp = (hi << 8) | lo;
}
0x5F => { self.registers.e = self.registers.a; } // LD E,A
0x67 => { self.registers.h = self.registers.a; } // LD H,A
0x47 => { self.registers.b = self.registers.a; } // LD B,A
0x7E => { // LD A,(HL)
    let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
    self.registers.a = self.mmu.read_byte(hl);
}
0x14 => { self.registers.d = self.registers.d.wrapping_add(1); } // INC D
0x0C => { self.registers.c = self.registers.c.wrapping_add(1); } // INC C
0x0D => { self.registers.c = self.registers.c.wrapping_sub(1); } // DEC C
0x2F => { self.registers.a ^= 0xFF; } // CPL
0x07 => { // RLCA
    let a = self.registers.a;
    let carry = (a & 0x80) != 0;
    let result = a.rotate_left(1);
    self.registers.a = result;
    self.registers.set_z(false);
    self.registers.set_n(false);
    self.registers.set_h(false);
    self.registers.set_c(carry);
}
0x37 => { self.registers.set_c(true); self.registers.set_n(false); self.registers.set_h(false); } // SCF
0xE0 => { // LDH (n),A
    let n = self.fetch();
    let addr = 0xFF00 | (n as u16);
    self.mmu.write_byte(addr, self.registers.a);
}
0xF0 => { // LDH A,(n)
    let n = self.fetch();
    let addr = 0xFF00 | (n as u16);
    self.registers.a = self.mmu.read_byte(addr);
}
0xE6 => { // AND n
    let n = self.fetch();
    let result = self.registers.a & n;
    self.registers.a = result;
    self.registers.set_z(result == 0);
    self.registers.set_n(false);
    self.registers.set_h(true);
    self.registers.set_c(false);
}
0xFE => { // CP n
    let n = self.fetch();
    let a = self.registers.a;
    let (result, borrow) = a.overflowing_sub(n);
    self.registers.set_z(result == 0);
    self.registers.set_n(true);
    self.registers.set_h((a & 0xF) < (n & 0xF));
    self.registers.set_c(borrow);
}
0xFF => { // RST 38H
    self.registers.sp = self.registers.sp.wrapping_sub(2);
    self.mmu.write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
    self.mmu.write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
    self.registers.pc = 0x0038;
}
0xC0 => { // RET NZ
    if !self.registers.get_z() {
        let lo = self.mmu.read_byte(self.registers.sp) as u16;
        let hi = self.mmu.read_byte(self.registers.sp + 1) as u16;
        self.registers.pc = (hi << 8) | lo;
        self.registers.sp = self.registers.sp.wrapping_add(2);
    }
},
                // JP NZ,nn
        0xC2 => {
            let lo = self.fetch() as u16;
            let hi = self.fetch() as u16;
            let addr = (hi << 8) | lo;
            if !self.registers.get_z() {
                self.registers.pc = addr;
            }
        },
        // JP NC,nn
        0xD2 => {
            let lo = self.fetch() as u16;
            let hi = self.fetch() as u16;
            let addr = (hi << 8) | lo;
            if !self.registers.get_c() {
                self.registers.pc = addr;
            }
        },
        // CALL Z,nn
        0xCC => {
            let lo = self.fetch() as u16;
            let hi = self.fetch() as u16;
            let addr = (hi << 8) | lo;
            if self.registers.get_z() {
                self.registers.sp = self.registers.sp.wrapping_sub(2);
                self.mmu.write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                self.mmu.write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
                self.registers.pc = addr;
            }
        },
        // RST 20H
        0xE7 => {
            self.registers.sp = self.registers.sp.wrapping_sub(2);
            self.mmu.write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
            self.mmu.write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
            self.registers.pc = 0x0020;
        },
        // DAA
        0x27 => {
            let mut a = self.registers.a;
            let mut adjust = 0;
            let mut carry = false;
            if !self.registers.get_n() {
                if self.registers.get_h() || (a & 0x0F) > 9 {
                    adjust |= 0x06;
                }
                if self.registers.get_c() || a > 0x99 {
                    adjust |= 0x60;
                    carry = true;
                }
                a = a.wrapping_add(adjust);
            } else {
                if self.registers.get_h() {
                    adjust |= 0x06;
                }
                if self.registers.get_c() {
                    adjust |= 0x60;
                }
                a = a.wrapping_sub(adjust);
            }
            self.registers.a = a;
            self.registers.set_z(a == 0);
            self.registers.set_h(false);
            self.registers.set_c(carry);
        }
                // RET Z
        0xC8 => {
            if self.registers.get_z() {
                let lo = self.mmu.read_byte(self.registers.sp) as u16;
                let hi = self.mmu.read_byte(self.registers.sp + 1) as u16;
                self.registers.pc = (hi << 8) | lo;
                self.registers.sp = self.registers.sp.wrapping_add(2);
            }
        },
        // RET C
        0xD8 => {
            if self.registers.get_c() {
                let lo = self.mmu.read_byte(self.registers.sp) as u16;
                let hi = self.mmu.read_byte(self.registers.sp + 1) as u16;
                self.registers.pc = (hi << 8) | lo;
                self.registers.sp = self.registers.sp.wrapping_add(2);
            }
        },
        // RET NC
        0xD0 => {
            if !self.registers.get_c() {
                let lo = self.mmu.read_byte(self.registers.sp) as u16;
                let hi = self.mmu.read_byte(self.registers.sp + 1) as u16;
                self.registers.pc = (hi << 8) | lo;
                self.registers.sp = self.registers.sp.wrapping_add(2);
            }
        },
        // CALL NZ,nn
        0xC4 => {
            let lo = self.fetch() as u16;
            let hi = self.fetch() as u16;
            let addr = (hi << 8) | lo;
            if !self.registers.get_z() {
                self.registers.sp = self.registers.sp.wrapping_sub(2);
                self.mmu.write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                self.mmu.write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
                self.registers.pc = addr;
            }
        },
        // CALL NC,nn
        0xD4 => {
            let lo = self.fetch() as u16;
            let hi = self.fetch() as u16;
            let addr = (hi << 8) | lo;
            if !self.registers.get_c() {
                self.registers.sp = self.registers.sp.wrapping_sub(2);
                self.mmu.write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                self.mmu.write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
                self.registers.pc = addr;
            }
        },
        // CALL C,nn
        0xDC => {
            let lo = self.fetch() as u16;
            let hi = self.fetch() as u16;
            let addr = (hi << 8) | lo;
            if self.registers.get_c() {
                self.registers.sp = self.registers.sp.wrapping_sub(2);
                self.mmu.write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                self.mmu.write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
                self.registers.pc = addr;
            }
        },
        // RST 00H
        0xC7 => {
            self.registers.sp = self.registers.sp.wrapping_sub(2);
            self.mmu.write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
            self.mmu.write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
            self.registers.pc = 0x0000;
        },
        // RST 08H
        0xCF => {
            self.registers.sp = self.registers.sp.wrapping_sub(2);
            self.mmu.write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
            self.mmu.write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
            self.registers.pc = 0x0008;
        },
        // RST 10H
        0xD7 => {
            self.registers.sp = self.registers.sp.wrapping_sub(2);
            self.mmu.write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
            self.mmu.write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
            self.registers.pc = 0x0010;
        },
        // RST 18H
        0xDF => {
            self.registers.sp = self.registers.sp.wrapping_sub(2);
            self.mmu.write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
            self.mmu.write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
            self.registers.pc = 0x0018;
        },
        // RST 28H
        0xEF => {
            self.registers.sp = self.registers.sp.wrapping_sub(2);
            self.mmu.write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
            self.mmu.write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
            self.registers.pc = 0x0028;
        },
        // RST 30H
        0xF7 => {
            self.registers.sp = self.registers.sp.wrapping_sub(2);
            self.mmu.write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
            self.mmu.write_byte(self.registers.sp + 1, (self.registers.pc >> 8) as u8);
            self.registers.pc = 0x0030;
        },
        // RST 38H 已有
        
        // RETI
        0xD9 => {
            let lo = self.mmu.read_byte(self.registers.sp) as u16;
            let hi = self.mmu.read_byte(self.registers.sp + 1) as u16;
            self.registers.pc = (hi << 8) | lo;
            self.registers.sp = self.registers.sp.wrapping_add(2);
            self.ime = true;
        },
        // CCF
        0x3F => {
            self.registers.set_c(!self.registers.get_c());
            self.registers.set_n(false);
            self.registers.set_h(false);
        },
                // LD HL,SP+n (signed offset)
        0xF8 => {
            let n = self.fetch() as i8 as i16;
            let sp = self.registers.sp as i16;
            let result = sp.wrapping_add(n) as u16;
            self.registers.h = (result >> 8) as u8;
            self.registers.l = result as u8;
            self.registers.set_z(false);
            self.registers.set_n(false);
            self.registers.set_h(((sp & 0xF) + (n & 0xF)) > 0xF);
            self.registers.set_c(((sp & 0xFF) + (n & 0xFF)) > 0xFF);
        },
        // LD SP,HL
        0xF9 => {
            let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
            self.registers.sp = hl;
        },
        // LD A,(HL-)
        0x3A => {
            let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
            self.registers.a = self.mmu.read_byte(hl);
            let hl = hl.wrapping_sub(1);
            self.registers.h = (hl >> 8) as u8;
            self.registers.l = hl as u8;
        }
        0xC6 => { // ADD A, n
            let n = self.fetch();
            let a = self.registers.a;
            let (result, carry) = a.overflowing_add(n);
            self.registers.set_z(result == 0);
            self.registers.set_n(false);
            self.registers.set_h(((a & 0xF) + (n & 0xF)) > 0xF);
            self.registers.set_c(carry);
            self.registers.a = result;
        }
        0xCE => { // ADC A, n
            let n = self.fetch();
            let a = self.registers.a;
            let c = if self.registers.get_c() { 1 } else { 0 };
            let (sum1, carry1) = a.overflowing_add(n);
            let (result, carry2) = sum1.overflowing_add(c);
            self.registers.set_z(result == 0);
            self.registers.set_n(false);
            self.registers.set_h(((a & 0xF) + (n & 0xF) + c) > 0xF);
            self.registers.set_c(carry1 || carry2);
            self.registers.a = result;
        }
        0xD6 => { // SUB n
            let n = self.fetch();
            let a = self.registers.a;
            let (result, borrow) = a.overflowing_sub(n);
            self.registers.set_z(result == 0);
            self.registers.set_n(true);
            self.registers.set_h((a & 0xF) < (n & 0xF));
            self.registers.set_c(borrow);
            self.registers.a = result;
        }
        0xDE => { // SBC A, n
            let n = self.fetch();
            let a = self.registers.a;
            let c = if self.registers.get_c() { 1 } else { 0 };
            let (sub1, borrow1) = a.overflowing_sub(n);
            let (result, borrow2) = sub1.overflowing_sub(c);
            self.registers.set_z(result == 0);
            self.registers.set_n(true);
            self.registers.set_h(((a & 0xF).wrapping_sub(n & 0xF).wrapping_sub(c)) > 0xF);
            self.registers.set_c(borrow1 || borrow2);
            self.registers.a = result;
        }
        0xEE => { // XOR n
            let n = self.fetch();
            let result = self.registers.a ^ n;
            self.registers.a = result;
            self.registers.set_z(result == 0);
            self.registers.set_n(false);
            self.registers.set_h(false);
            self.registers.set_c(false);
        }
        0xF6 => { // OR n
            let n = self.fetch();
            let result = self.registers.a | n;
            self.registers.a = result;
            self.registers.set_z(result == 0);
            self.registers.set_n(false);
            self.registers.set_h(false);
            self.registers.set_c(false);
        }
        // ...existing code...
        // DI/EI 已有
        // CPL 已有
        // RLCA 已有
        // SCF 已有
        _ => {
            // 只記錄一次未實作指令，避免大量輸出導致閃退
            let mut set = UNIMPL_OPCODES.lock().unwrap();
            if set.insert(opcode) {
                // eprintln! 可避免與 stdout 混淆，或直接註解掉
                eprintln!("未實作的指令: 0x{:02X} 在 PC: 0x{:04X}", opcode, self.registers.pc);
            }
            // 不要 panic，讓模擬器繼續執行
        }
    
}
    }


    fn decode_cb(&mut self, cb_opcode: u8) {
        match cb_opcode {
        // --- RLC r ---
            0x00..=0x07 => {
                let reg = cb_opcode & 0x07;
                let (val, mut set_fn): (u8, Box<dyn FnMut(&mut CPU, u8)>) = match reg {
                    0 => (self.registers.b, Box::new(|cpu: &mut CPU, v| cpu.registers.b = v)),
                    1 => (self.registers.c, Box::new(|cpu: &mut CPU, v| cpu.registers.c = v)),
                    2 => (self.registers.d, Box::new(|cpu: &mut CPU, v| cpu.registers.d = v)),
                    3 => (self.registers.e, Box::new(|cpu: &mut CPU, v| cpu.registers.e = v)),
                    4 => (self.registers.h, Box::new(|cpu: &mut CPU, v| cpu.registers.h = v)),
                    5 => (self.registers.l, Box::new(|cpu: &mut CPU, v| cpu.registers.l = v)),
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        (
                            self.mmu.read_byte(hl),
                            Box::new(|cpu: &mut CPU, v| {
                                let hl = ((cpu.registers.h as u16) << 8) | cpu.registers.l as u16;
                                cpu.mmu.write_byte(hl, v);
                            }),
                        )
                    },
                    7 => (self.registers.a, Box::new(|cpu: &mut CPU, v| cpu.registers.a = v)),
                    _ => unreachable!(),
                };
                let carry = (val & 0x80) != 0;
                let result = val.rotate_left(1);
                set_fn(self, result);
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h(false);
                self.registers.set_c(carry);
            }
            // --- RRC r ---
            0x08..=0x0F => {
                let reg = cb_opcode & 0x07;
                let (val, mut set_fn): (u8, Box<dyn FnMut(&mut CPU, u8)>) = match reg {
                    0 => (self.registers.b, Box::new(|cpu: &mut CPU, v| cpu.registers.b = v)),
                    1 => (self.registers.c, Box::new(|cpu: &mut CPU, v| cpu.registers.c = v)),
                    2 => (self.registers.d, Box::new(|cpu: &mut CPU, v| cpu.registers.d = v)),
                    3 => (self.registers.e, Box::new(|cpu: &mut CPU, v| cpu.registers.e = v)),
                    4 => (self.registers.h, Box::new(|cpu: &mut CPU, v| cpu.registers.h = v)),
                    5 => (self.registers.l, Box::new(|cpu: &mut CPU, v| cpu.registers.l = v)),
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        (
                            self.mmu.read_byte(hl),
                            Box::new(|cpu: &mut CPU, v| {
                                let hl = ((cpu.registers.h as u16) << 8) | cpu.registers.l as u16;
                                cpu.mmu.write_byte(hl, v);
                            }),
                        )
                    },
                    7 => (self.registers.a, Box::new(|cpu: &mut CPU, v| cpu.registers.a = v)),
                    _ => unreachable!(),
                };
                let carry = (val & 0x01) != 0;
                let result = val.rotate_right(1);
                set_fn(self, result);
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h(false);
                self.registers.set_c(carry);
            }
            // --- RL r ---
            0x10..=0x17 => {
                let reg = cb_opcode & 0x07;
                let (val, mut set_fn): (u8, Box<dyn FnMut(&mut CPU, u8)>) = match reg {
                    0 => (self.registers.b, Box::new(|cpu: &mut CPU, v| cpu.registers.b = v)),
                    1 => (self.registers.c, Box::new(|cpu: &mut CPU, v| cpu.registers.c = v)),
                    2 => (self.registers.d, Box::new(|cpu: &mut CPU, v| cpu.registers.d = v)),
                    3 => (self.registers.e, Box::new(|cpu: &mut CPU, v| cpu.registers.e = v)),
                    4 => (self.registers.h, Box::new(|cpu: &mut CPU, v| cpu.registers.h = v)),
                    5 => (self.registers.l, Box::new(|cpu: &mut CPU, v| cpu.registers.l = v)),
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        (
                            self.mmu.read_byte(hl),
                            Box::new(|cpu: &mut CPU, v| {
                                let hl = ((cpu.registers.h as u16) << 8) | cpu.registers.l as u16;
                                cpu.mmu.write_byte(hl, v);
                            }),
                        )
                    },
                    7 => (self.registers.a, Box::new(|cpu: &mut CPU, v| cpu.registers.a = v)),
                    _ => unreachable!(),
                };
                let carry_in = if self.registers.get_c() { 1 } else { 0 };
                let carry_out = (val & 0x80) != 0;
                let result = (val << 1) | carry_in;
                set_fn(self, result);
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h(false);
                self.registers.set_c(carry_out);
            }
            // --- RR r ---
            0x18..=0x1F => {
                let reg = cb_opcode & 0x07;
                let (val, mut set_fn): (u8, Box<dyn FnMut(&mut CPU, u8)>) = match reg {
                    0 => (self.registers.b, Box::new(|cpu: &mut CPU, v| cpu.registers.b = v)),
                    1 => (self.registers.c, Box::new(|cpu: &mut CPU, v| cpu.registers.c = v)),
                    2 => (self.registers.d, Box::new(|cpu: &mut CPU, v| cpu.registers.d = v)),
                    3 => (self.registers.e, Box::new(|cpu: &mut CPU, v| cpu.registers.e = v)),
                    4 => (self.registers.h, Box::new(|cpu: &mut CPU, v| cpu.registers.h = v)),
                    5 => (self.registers.l, Box::new(|cpu: &mut CPU, v| cpu.registers.l = v)),
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        (
                            self.mmu.read_byte(hl),
                            Box::new(|cpu: &mut CPU, v| {
                                let hl = ((cpu.registers.h as u16) << 8) | cpu.registers.l as u16;
                                cpu.mmu.write_byte(hl, v);
                            }),
                        )
                    },
                    7 => (self.registers.a, Box::new(|cpu: &mut CPU, v| cpu.registers.a = v)),
                    _ => unreachable!(),
                };
                let carry_in = if self.registers.get_c() { 0x80 } else { 0 };
                let carry_out = (val & 0x01) != 0;
                let result = (val >> 1) | carry_in;
                set_fn(self, result);
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h(false);
                self.registers.set_c(carry_out);
            }
            // --- SLA r ---
            0x20..=0x27 => {
                let reg = cb_opcode & 0x07;
                let (val, mut set_fn): (u8, Box<dyn FnMut(&mut CPU, u8)>) = match reg {
                    0 => (self.registers.b, Box::new(|cpu: &mut CPU, v| cpu.registers.b = v)),
                    1 => (self.registers.c, Box::new(|cpu: &mut CPU, v| cpu.registers.c = v)),
                    2 => (self.registers.d, Box::new(|cpu: &mut CPU, v| cpu.registers.d = v)),
                    3 => (self.registers.e, Box::new(|cpu: &mut CPU, v| cpu.registers.e = v)),
                    4 => (self.registers.h, Box::new(|cpu: &mut CPU, v| cpu.registers.h = v)),
                    5 => (self.registers.l, Box::new(|cpu: &mut CPU, v| cpu.registers.l = v)),
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        (
                            self.mmu.read_byte(hl),
                            Box::new(|cpu: &mut CPU, v| {
                                let hl = ((cpu.registers.h as u16) << 8) | cpu.registers.l as u16;
                                cpu.mmu.write_byte(hl, v);
                            }),
                        )
                    },
                    7 => (self.registers.a, Box::new(|cpu: &mut CPU, v| cpu.registers.a = v)),
                    _ => unreachable!(),
                };
                let carry = (val & 0x80) != 0;
                let result = val << 1;
                set_fn(self, result);
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h(false);
                self.registers.set_c(carry);
            }
            // --- SRL r ---
            0x38..=0x3F => {
                let reg = cb_opcode & 0x07;
                let (val, mut set_fn): (u8, Box<dyn FnMut(&mut CPU, u8)>) = match reg {
                    0 => (self.registers.b, Box::new(|cpu: &mut CPU, v| cpu.registers.b = v)),
                    1 => (self.registers.c, Box::new(|cpu: &mut CPU, v| cpu.registers.c = v)),
                    2 => (self.registers.d, Box::new(|cpu: &mut CPU, v| cpu.registers.d = v)),
                    3 => (self.registers.e, Box::new(|cpu: &mut CPU, v| cpu.registers.e = v)),
                    4 => (self.registers.h, Box::new(|cpu: &mut CPU, v| cpu.registers.h = v)),
                    5 => (self.registers.l, Box::new(|cpu: &mut CPU, v| cpu.registers.l = v)),
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        (
                            self.mmu.read_byte(hl),
                            Box::new(|cpu: &mut CPU, v| {
                                let hl = ((cpu.registers.h as u16) << 8) | cpu.registers.l as u16;
                                cpu.mmu.write_byte(hl, v);
                            }),
                        )
                    },
                    7 => (self.registers.a, Box::new(|cpu: &mut CPU, v| cpu.registers.a = v)),
                    _ => unreachable!(),
                };
                let carry = (val & 0x01) != 0;
                let result = val >> 1;
                set_fn(self, result);
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h(false);
                self.registers.set_c(carry);
            }
            // --- SWAP r ---
            0x30..=0x37 => {
                let reg = cb_opcode & 0x07;
                let (val, mut set_fn): (u8, Box<dyn FnMut(&mut CPU, u8)>) = match reg {
                    0 => (self.registers.b, Box::new(|cpu: &mut CPU, v| cpu.registers.b = v)),
                    1 => (self.registers.c, Box::new(|cpu: &mut CPU, v| cpu.registers.c = v)),
                    2 => (self.registers.d, Box::new(|cpu: &mut CPU, v| cpu.registers.d = v)),
                    3 => (self.registers.e, Box::new(|cpu: &mut CPU, v| cpu.registers.e = v)),
                    4 => (self.registers.h, Box::new(|cpu: &mut CPU, v| cpu.registers.h = v)),
                    5 => (self.registers.l, Box::new(|cpu: &mut CPU, v| cpu.registers.l = v)),
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        (
                            self.mmu.read_byte(hl),
                            Box::new(|cpu: &mut CPU, v| {
                                let hl = ((cpu.registers.h as u16) << 8) | cpu.registers.l as u16;
                                cpu.mmu.write_byte(hl, v);
                            }),
                        )
                    },
                    7 => (self.registers.a, Box::new(|cpu: &mut CPU, v| cpu.registers.a = v)),
                    _ => unreachable!(),
                };
                let result = (val >> 4) | (val << 4);
                set_fn(self, result);
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h(false);
                self.registers.set_c(false);
            }
            // --- SRA r ---
            0x28..=0x2F => {
                let reg = cb_opcode & 0x07;
                let (val, mut set_fn): (u8, Box<dyn FnMut(&mut CPU, u8)>) = match reg {
                    0 => (self.registers.b, Box::new(|cpu: &mut CPU, v| cpu.registers.b = v)),
                    1 => (self.registers.c, Box::new(|cpu: &mut CPU, v| cpu.registers.c = v)),
                    2 => (self.registers.d, Box::new(|cpu: &mut CPU, v| cpu.registers.d = v)),
                    3 => (self.registers.e, Box::new(|cpu: &mut CPU, v| cpu.registers.e = v)),
                    4 => (self.registers.h, Box::new(|cpu: &mut CPU, v| cpu.registers.h = v)),
                    5 => (self.registers.l, Box::new(|cpu: &mut CPU, v| cpu.registers.l = v)),
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        (
                            self.mmu.read_byte(hl),
                            Box::new(|cpu: &mut CPU, v| {
                                let hl = ((cpu.registers.h as u16) << 8) | cpu.registers.l as u16;
                                cpu.mmu.write_byte(hl, v);
                            }),
                        )
                    },
                    7 => (self.registers.a, Box::new(|cpu: &mut CPU, v| cpu.registers.a = v)),
                    _ => unreachable!(),
                };
                let carry = (val & 0x01) != 0;
                let msb = val & 0x80;
                let result = (val >> 1) | msb;
                set_fn(self, result);
                self.registers.set_z(result == 0);
                self.registers.set_n(false);
                self.registers.set_h(false);
                self.registers.set_c(carry);
            }
            // --- BIT/RES/SET ---
            0x40..=0x7F => {
                // --- BIT n, r ---
                let bit = (cb_opcode >> 3) & 0x07;
                let reg = cb_opcode & 0x07;
                let value = match reg {
                    0 => self.registers.b,
                    1 => self.registers.c,
                    2 => self.registers.d,
                    3 => self.registers.e,
                    4 => self.registers.h,
                    5 => self.registers.l,
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        self.mmu.read_byte(hl)
                    }
                    7 => self.registers.a,
                    _ => 0,
                };
                self.registers.set_z((value & (1 << bit)) == 0);
                self.registers.set_n(false);
                self.registers.set_h(true);
            }
            // --- RES n, r ---
            0x80..=0xBF => {
                let bit = (cb_opcode >> 3) & 0x07;
                let reg = cb_opcode & 0x07;
                match reg {
                    0 => self.registers.b &= !(1 << bit),
                    1 => self.registers.c &= !(1 << bit),
                    2 => self.registers.d &= !(1 << bit),
                    3 => self.registers.e &= !(1 << bit),
                    4 => self.registers.h &= !(1 << bit),
                    5 => self.registers.l &= !(1 << bit),
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        let mut v = self.mmu.read_byte(hl);
                        v &= !(1 << bit);
                        self.mmu.write_byte(hl, v);
                    }
                    7 => self.registers.a &= !(1 << bit),
                    _ => {}
                }
            }
            // --- SET n, r ---
            0xC0..=0xFF => {
                let bit = (cb_opcode >> 3) & 0x07;
                let reg = cb_opcode & 0x07;
                match reg {
                    0 => self.registers.b |= 1 << bit,
                    1 => self.registers.c |= 1 << bit,
                    2 => self.registers.d |= 1 << bit,
                    3 => self.registers.e |= 1 << bit,
                    4 => self.registers.h |= 1 << bit,
                    5 => self.registers.l |= 1 << bit,
                    6 => {
                        let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                        let mut v = self.mmu.read_byte(hl);
                        v |= 1 << bit;
                        self.mmu.write_byte(hl, v);
                    }
                    7 => self.registers.a |= 1 << bit,
                    _ => {}
                }
            }
        
        }
    }   
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flag_set_and_get() {
        let mut reg = Registers::default();
        reg.set_z(true);
        assert!(reg.get_z());
        reg.set_z(false);
        assert!(!reg.get_z());

        reg.set_n(true);
        assert!(reg.get_n());
        reg.set_n(false);
        assert!(!reg.get_n());

        reg.set_h(true);
        assert!(reg.get_h());
        reg.set_h(false);
        assert!(!reg.get_h());

        reg.set_c(true);
        assert!(reg.get_c());
        reg.set_c(false);
        assert!(!reg.get_c());
    }

    #[test]
    fn test_clear_flags() {
        let mut reg = Registers::default();
        reg.f = 0xFF;
        reg.clear_flags();
        assert_eq!(reg.f, 0);
    }

    #[test]
    fn test_inc_a() {
        let mut cpu = CPU::new(MMU::new());
        cpu.registers.a = 0x0F;
        cpu.decode_and_execute(0x3C); // INC A
        assert_eq!(cpu.registers.a, 0x10);
    }

    #[test]
    fn test_add_a_b() {
        let mut cpu = CPU::new(MMU::new());
        cpu.registers.a = 1;
        cpu.registers.b = 2;
        cpu.decode_and_execute(0x80); // ADD A, B
        assert_eq!(cpu.registers.a, 3);
        assert!(!cpu.registers.get_z());
    }

    #[test]
    fn test_xor_a() {
        let mut cpu = CPU::new(MMU::new());
        cpu.registers.a = 0xFF;
        cpu.decode_and_execute(0xAF); // XOR A
        assert_eq!(cpu.registers.a, 0);
        assert!(cpu.registers.get_z());
    }
}