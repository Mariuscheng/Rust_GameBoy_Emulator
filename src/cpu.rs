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
pub struct CPU {
    pub registers: Registers,
    pub mmu: MMU,
}

impl CPU {
    pub fn step(&mut self) {
        self.execute();
        let pos = (self.registers.pc as usize) % 160;
        self.mmu.vram[pos] = self.registers.pc as u8;
    }

    pub fn new(mmu: MMU) -> Self {
        CPU {
            registers: Registers::default(),
            mmu,
        }
    }

    pub fn load_rom(&mut self, rom: &[u8]) {
        self.mmu.load_rom(rom);
    }

    pub fn execute(&mut self) {
        let opcode = self.fetch();
        self.decode_and_execute(opcode);
    }

    fn fetch(&mut self) -> u8 {
        let opcode = self.mmu.read_byte(self.registers.pc);
        self.registers.pc += 1;
        opcode
    }

    fn decode_and_execute(&mut self, opcode: u8) {
        match opcode {
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
            0xAF => { self.registers.a = 0; } // XOR A
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
            0x80 => { self.registers.a = self.registers.a.wrapping_add(self.registers.b); } // ADD A, B
            0x81 => { self.registers.a = self.registers.a.wrapping_add(self.registers.c); } // ADD A, C
            0x82 => { self.registers.a = self.registers.a.wrapping_add(self.registers.d); } // ADD A, D
            0x83 => { self.registers.a = self.registers.a.wrapping_add(self.registers.e); } // ADD A, E
            0x84 => { self.registers.a = self.registers.a.wrapping_add(self.registers.h); } // ADD A, H
            0x85 => { self.registers.a = self.registers.a.wrapping_add(self.registers.l); } // ADD A, L
            0x86 => { // ADD A, (HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.a = self.registers.a.wrapping_add(self.mmu.read_byte(hl));
            }
            0x87 => { self.registers.a = self.registers.a.wrapping_add(self.registers.a); } // ADD A, A
                        // SUB A, r
            0x90 => { self.registers.a = self.registers.a.wrapping_sub(self.registers.b); }
            0x91 => { self.registers.a = self.registers.a.wrapping_sub(self.registers.c); }
            0x92 => { self.registers.a = self.registers.a.wrapping_sub(self.registers.d); }
            0x93 => { self.registers.a = self.registers.a.wrapping_sub(self.registers.e); }
            0x94 => { self.registers.a = self.registers.a.wrapping_sub(self.registers.h); }
            0x95 => { self.registers.a = self.registers.a.wrapping_sub(self.registers.l); }
            0x96 => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.a = self.registers.a.wrapping_sub(self.mmu.read_byte(hl));
            }
            0x97 => { self.registers.a = self.registers.a.wrapping_sub(self.registers.a); }
            
            // AND A, r
            0xA0 => { self.registers.a &= self.registers.b; }
            0xA1 => { self.registers.a &= self.registers.c; }
            0xA2 => { self.registers.a &= self.registers.d; }
            0xA3 => { self.registers.a &= self.registers.e; }
            0xA4 => { self.registers.a &= self.registers.h; }
            0xA5 => { self.registers.a &= self.registers.l; }
            0xA6 => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.a &= self.mmu.read_byte(hl);
            }
            0xA7 => { self.registers.a &= self.registers.a; }
            
            // OR A, r
            0xB0 => { self.registers.a |= self.registers.b; }
            0xB1 => { self.registers.a |= self.registers.c; }
            0xB2 => { self.registers.a |= self.registers.d; }
            0xB3 => { self.registers.a |= self.registers.e; }
            0xB4 => { self.registers.a |= self.registers.h; }
            0xB5 => { self.registers.a |= self.registers.l; }
            0xB6 => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.a |= self.mmu.read_byte(hl);
            }
            0xB7 => { self.registers.a |= self.registers.a; }
            
            // XOR A, r
            0xA8 => { self.registers.a ^= self.registers.b; }
            0xA9 => { self.registers.a ^= self.registers.c; }
            0xAA => { self.registers.a ^= self.registers.d; }
            0xAB => { self.registers.a ^= self.registers.e; }
            0xAC => { self.registers.a ^= self.registers.h; }
            0xAD => { self.registers.a ^= self.registers.l; }
            0xAE => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.a ^= self.mmu.read_byte(hl);
            }
            
            // CP A, r（僅作減法，不改變A，這裡僅示意）
            0xB8 => { let _ = self.registers.a.wrapping_sub(self.registers.b); }
            0xB9 => { let _ = self.registers.a.wrapping_sub(self.registers.c); }
            0xBA => { let _ = self.registers.a.wrapping_sub(self.registers.d); }
            0xBB => { let _ = self.registers.a.wrapping_sub(self.registers.e); }
            0xBC => { let _ = self.registers.a.wrapping_sub(self.registers.h); }
            0xBD => { let _ = self.registers.a.wrapping_sub(self.registers.l); }
            0xBE => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let _ = self.registers.a.wrapping_sub(self.mmu.read_byte(hl));
            }
            0xBF => { let _ = self.registers.a.wrapping_sub(self.registers.a); }
            // ...可依此模式繼續補齊 SUB/AND/OR/XOR/CP 等...
            0x47 => { self.registers.b = self.registers.a; } // LD B,A
            0x4F => { self.registers.c = self.registers.a; } // LD C, A
            0x57 => { self.registers.d = self.registers.a; } // LD D, A
            0x5F => { self.registers.e = self.registers.a; } // LD E, A
            0x67 => { self.registers.h = self.registers.a; } // LD H, A
            0x6F => { self.registers.l = self.registers.a; } // LD L, A
            0x01 => { // LD BC, nn
                let lo = self.fetch();
                let hi = self.fetch();
                self.registers.b = hi;
                self.registers.c = lo;
            }
            0x0C => { self.registers.c = self.registers.c.wrapping_add(1); } // INC C
            0x0D => { self.registers.c = self.registers.c.wrapping_sub(1); } // DEC C
            0x11 => { // LD DE, nn
                let lo = self.fetch();
                let hi = self.fetch();
                self.registers.d = hi;
                self.registers.e = lo;
            }
            0x1C => { self.registers.e = self.registers.e.wrapping_add(1); } // INC E
            0x1D => { self.registers.e = self.registers.e.wrapping_sub(1); } // DEC E
            0x21 => { // LD HL, nn
                let lo = self.fetch();
                let hi = self.fetch();
                self.registers.h = hi;
                self.registers.l = lo;
            }
            0x23 => { // INC HL
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let hl = hl.wrapping_add(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = hl as u8;
            }
            0x31 => { // LD SP, nn
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.sp = (hi << 8) | lo;
            }
            0x32 => { // LD (HL-),A
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.mmu.write_byte(hl, self.registers.a);
                let hl = hl.wrapping_sub(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = hl as u8;
            }
            0x77 => { // LD (HL),A
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.mmu.write_byte(hl, self.registers.a);
            }
            0x7E => { // LD A,(HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.a = self.mmu.read_byte(hl);
            }
            0xC9 => { /* RET (暫不實作堆疊) */ }
            0x76 => { /* HALT (暫不處理) */ }
            0x02 => { // LD (BC),A
                let addr = ((self.registers.b as u16) << 8) | self.registers.c as u16;
                self.mmu.write_byte(addr, self.registers.a);
            }
            0x0A => { // LD A,(BC)
                let addr = ((self.registers.b as u16) << 8) | self.registers.c as u16;
                self.registers.a = self.mmu.read_byte(addr);
            }
            0x12 => { // LD (DE),A
                let addr = ((self.registers.d as u16) << 8) | self.registers.e as u16;
                self.mmu.write_byte(addr, self.registers.a);
            }
            0x1A => { // LD A,(DE)
                let addr = ((self.registers.d as u16) << 8) | self.registers.e as u16;
                self.registers.a = self.mmu.read_byte(addr);
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
            0x03 => { // INC BC
                let bc = (((self.registers.b as u16) << 8) | self.registers.c as u16).wrapping_add(1);
                self.registers.b = (bc >> 8) as u8;
                self.registers.c = bc as u8;
            }
            0x13 => { // INC DE
                let de = (((self.registers.d as u16) << 8) | self.registers.e as u16).wrapping_add(1);
                self.registers.d = (de >> 8) as u8;
                self.registers.e = de as u8;
            }
            0x0B => { // DEC BC
                let bc = (((self.registers.b as u16) << 8) | self.registers.c as u16).wrapping_sub(1);
                self.registers.b = (bc >> 8) as u8;
                self.registers.c = bc as u8;
            }
            0x1B => { // DEC DE
                let de = (((self.registers.d as u16) << 8) | self.registers.e as u16).wrapping_sub(1);
                self.registers.d = (de >> 8) as u8;
                self.registers.e = de as u8;
            }
            0x0F => { self.registers.a = self.registers.a.rotate_left(1); } // RRCA (簡化)
            0x17 => { self.registers.a = self.registers.a.rotate_left(1); } // RLA (簡化)
            0x1F => { self.registers.a = self.registers.a.rotate_right(1); } // RRCA (簡化)
            0x18 => { // JR n
                let offset = self.fetch() as i8;
                self.registers.pc = (self.registers.pc as i16 + offset as i16) as u16;
            }
            0x20 => { // JR NZ,n (簡化:永遠跳)
                let offset = self.fetch() as i8;
                self.registers.pc = (self.registers.pc as i16 + offset as i16) as u16;
            }
            0x28 => { // JR Z,n (簡化:永遠跳)
                let offset = self.fetch() as i8;
                self.registers.pc = (self.registers.pc as i16 + offset as i16) as u16;
            }
            0x24 => { self.registers.h = self.registers.h.wrapping_add(1); } // INC H
            0x25 => { self.registers.h = self.registers.h.wrapping_sub(1); } // DEC H
            0x2C => { self.registers.l = self.registers.l.wrapping_add(1); } // INC L
            0x2D => { self.registers.l = self.registers.l.wrapping_sub(1); } // DEC L
            0x3E => { let n = self.fetch(); self.registers.a = n; } // LD A, n
            // ...繼續補齊其他常用指令...
            0x14 => { self.registers.d = self.registers.d.wrapping_add(1); } // INC D
            0x15 => { self.registers.d = self.registers.d.wrapping_sub(1); } // DEC D
            0x19 => { // ADD HL,DE
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let de = ((self.registers.d as u16) << 8) | self.registers.e as u16;
                let result = hl.wrapping_add(de);
                self.registers.h = (result >> 8) as u8;
                self.registers.l = result as u8;
            }
            0x29 => { // ADD HL,HL
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let result = hl.wrapping_add(hl);
                self.registers.h = (result >> 8) as u8;
                self.registers.l = result as u8;
            }
            0x2B => { // DEC HL
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let hl = hl.wrapping_sub(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = hl as u8;
            }
            0x33 => { self.registers.sp = self.registers.sp.wrapping_add(1); } // INC SP
            0x3B => { self.registers.sp = self.registers.sp.wrapping_sub(1); } // DEC SP
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
            0x36 => { // LD (HL),n
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let n = self.fetch();
                self.mmu.write_byte(hl, n);
            }
            // 0x2F: CPL (A 取反)
            0x2F => { self.registers.a = !self.registers.a; }
            // 0x30: JR NC, n (這裡簡化為無條件跳轉)
            0x30 => {
                let offset = self.fetch() as i8;
                self.registers.pc = (self.registers.pc as i16 + offset as i16) as u16;
            }
            // 0x37: SCF (設置進位旗標，這裡僅示意)
            0x37 => { /* self.registers.f |= 0x10; */ }
            // 0x38: JR C, n (這裡簡化為無條件跳轉)
            0x38 => {
                let offset = self.fetch() as i8;
                self.registers.pc = (self.registers.pc as i16 + offset as i16) as u16;
            }
            // 0x39: ADD HL, SP
            0x39 => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let result = hl.wrapping_add(self.registers.sp);
                self.registers.h = (result >> 8) as u8;
                self.registers.l = result as u8;
            }
            // 0x3A: LD A, (HL-)
            0x3A => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.a = self.mmu.read_byte(hl);
                let hl = hl.wrapping_sub(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = hl as u8;
            }
            // 0x3F: CCF (反轉進位旗標，這裡僅示意)
            0x3F => { /* self.registers.f ^= 0x10; */ }
            // 0x88~0x8F: ADC A, r（這裡暫以普通加法代替，未處理進位）
            0x88 => { self.registers.a = self.registers.a.wrapping_add(self.registers.b); }
            0x89 => { self.registers.a = self.registers.a.wrapping_add(self.registers.c); }
            0x8A => { self.registers.a = self.registers.a.wrapping_add(self.registers.d); }
            0x8B => { self.registers.a = self.registers.a.wrapping_add(self.registers.e); }
            0x8C => { self.registers.a = self.registers.a.wrapping_add(self.registers.h); }
            0x8D => { self.registers.a = self.registers.a.wrapping_add(self.registers.l); }
            0x8E => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.a = self.registers.a.wrapping_add(self.mmu.read_byte(hl));
            }
            0x8F => { self.registers.a = self.registers.a.wrapping_add(self.registers.a); } // ADD A, A
                        // 0x98~0x9F: SBC A, r（這裡暫以普通減法代替，未處理進位）
            0x98 => { self.registers.a = self.registers.a.wrapping_sub(self.registers.b); }
            0x99 => { self.registers.a = self.registers.a.wrapping_sub(self.registers.c); }
            0x9A => { self.registers.a = self.registers.a.wrapping_sub(self.registers.d); }
            0x9B => { self.registers.a = self.registers.a.wrapping_sub(self.registers.e); }
            0x9C => { self.registers.a = self.registers.a.wrapping_sub(self.registers.h); }
            0x9D => { self.registers.a = self.registers.a.wrapping_sub(self.registers.l); }
            0x9E => {
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.a = self.registers.a.wrapping_sub(self.mmu.read_byte(hl));
            }
            0x9F => { self.registers.a = self.registers.a.wrapping_sub(self.registers.a); }
            // 0xC3: JP nn 已補
            // 0xC9: RET 已補
            // 0xCB: CB 前綴指令
            0xCB => {
                let cb_opcode = self.fetch();
                self.decode_cb(cb_opcode);
            }
            0x10 => { /* STOP (暫不處理) */ }
            0x27 => { /* DAA (十進制調整，暫不處理) */ }
            0xC0 => { /* RET NZ (暫不處理條件與堆疊) */ }
            0xC1 => { /* POP BC (暫不處理堆疊) */ }
            0xC5 => { /* PUSH BC (暫不處理堆疊) */ }
            0xD1 => { /* POP DE (暫不處理堆疊) */ }
            0xD5 => { /* PUSH DE (暫不處理堆疊) */ }
            0xE1 => { /* POP HL (暫不處理堆疊) */ }
            0xE5 => { /* PUSH HL (暫不處理堆疊) */ }
            0xF1 => { /* POP AF (暫不處理堆疊) */ }
            0xF5 => { /* PUSH AF (暫不處理堆疊) */ }
            // 0xC9 => { /* RET (暫不處理堆疊) */ } // Removed duplicate to fix unreachable pattern
            0xD9 => { /* RETI (暫不處理堆疊) */ }
            0xC7 => { /* RST 00H (暫不處理) */ }
            0xCF => { /* RST 08H (暫不處理) */ }
            0xD7 => { /* RST 10H (暫不處理) */ }
            0xDF => { /* RST 18H (暫不處理) */ }
            0xE7 => { /* RST 20H (暫不處理) */ }
            0xEF => { /* RST 28H (暫不處理) */ }
            0xF7 => { /* RST 30H (暫不處理) */ }
            0xFF => { /* RST 38H (暫不處理) */ }
            0xC8 => { /* RET Z (暫不處理條件與堆疊) */ }
            0xD0 => { /* RET NC (暫不處理條件與堆疊) */ }
            0xD8 => { /* RET C (暫不處理條件與堆疊) */ }
            0xC4 => { // CALL NZ, nn (簡化:無條件跳轉)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0xCC => { // CALL Z, nn (簡化:無條件跳轉)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0xCD => { // CALL nn (簡化:無條件跳轉)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0xD4 => { // CALL NC, nn (簡化:無條件跳轉)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0xDC => { // CALL C, nn (簡化:無條件跳轉)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0xC2 => { // JP NZ, nn (簡化:無條件跳轉)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0xCA => { // JP Z, nn (簡化:無條件跳轉)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0xD2 => { // JP NC, nn (簡化:無條件跳轉)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0xDA => { // JP C, nn (簡化:無條件跳轉)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0xF3 => { /* DI (Disable Interrupts)，暫不處理可留空 */ }
            0xE0 => { // LDH (n),A
                let n = self.fetch() as u16;
                self.mmu.write_byte(0xFF00 + n, self.registers.a);
            }
            0xF0 => { // LDH A,(n)
                let n = self.fetch() as u16;
                self.registers.a = self.mmu.read_byte(0xFF00 + n);
            }
            0xFE => { // CP n
                let n = self.fetch();
                let _ = self.registers.a.wrapping_sub(n);
                // 這裡暫不處理旗標
            }
            // ...existing code...
        _ => {
            
            let mut set = UNIMPL_OPCODES.lock().unwrap();
            if set.insert(opcode) {
                println!("未實作的指令: 0x{:02X} 在 PC: 0x{:04X}", opcode, self.registers.pc);
            }
            
        }
    }
}

    fn decode_cb(&mut self, cb_opcode: u8) {
        match cb_opcode {
            0x7C => { // BIT 7, H
                // 測試 H 的 bit 7，這裡僅示意，不處理旗標
                let _ = (self.registers.h & 0x80) != 0;
            }
            0x00 => { // RLC B
                self.registers.b = self.registers.b.rotate_left(1);
            }
            0x01 => { // RLC C
                self.registers.c = self.registers.c.rotate_left(1);
            }
            0x02 => { // RLC D
                self.registers.d = self.registers.d.rotate_left(1);
            }
            0x03 => { // RLC E
                self.registers.e = self.registers.e.rotate_left(1);
            }
            0x04 => { // RLC H
                self.registers.h = self.registers.h.rotate_left(1);
            }
            0x05 => { // RLC L
                self.registers.l = self.registers.l.rotate_left(1);
            }
            0x07 => { // RLC A
                self.registers.a = self.registers.a.rotate_left(1);
            }
            0x0F => { // RRC A
                self.registers.a = self.registers.a.rotate_right(1);
            }
            0x17 => { // RL A
                self.registers.a = self.registers.a.rotate_left(1);
            }
            0x1F => { // RR A
                self.registers.a = self.registers.a.rotate_right(1);
            }
            0x3F => { // SRL A
                // 右移 A，最低位進 Carry，最高位補 0（這裡不處理旗標，只右移）
                self.registers.a >>= 1;
            }
            0x08 => { // RRC B
                self.registers.b = self.registers.b.rotate_right(1);
            }
            0x10 => { // RL B
                self.registers.b = self.registers.b.rotate_left(1);
            }
            0x18 => { // RR B
                self.registers.b = self.registers.b.rotate_right(1);
            }
            0x20 => { // SLA B
                self.registers.b <<= 1;
            }
            0x28 => { // SRA B
                // 算術右移，最高位不變
                let msb = self.registers.b & 0x80;
                self.registers.b = (self.registers.b >> 1) | msb;
            }
            0x30 => { // SWAP B
                let b = self.registers.b;
                self.registers.b = (b >> 4) | (b << 4);
            }
            0x38 => { // SRL B
                self.registers.b >>= 1;
            }
            0x40 => { // BIT 0, B
                let _ = (self.registers.b & 0x01) != 0;
            }
            0x80 => { // RES 0, B
                self.registers.b &= !0x01;
            }
            0xC0 => { // SET 0, B
                self.registers.b |= 0x01;
            }
            0x09 => { // RRC C
                self.registers.c = self.registers.c.rotate_right(1);
            }
            0x11 => { // RL C
                self.registers.c = self.registers.c.rotate_left(1);
            }
            0x19 => { // RR C
                self.registers.c = self.registers.c.rotate_right(1);
            }
            0x21 => { // SLA C
                self.registers.c <<= 1;
            }
            0x29 => { // SRA C
                let msb = self.registers.c & 0x80;
                self.registers.c = (self.registers.c >> 1) | msb;
            }
            0x31 => { // SWAP C
                let c = self.registers.c;
                self.registers.c = (c >> 4) | (c << 4);
            }
            0x39 => { // SRL C
                self.registers.c >>= 1;
            }
            0x41 => { // BIT 0, C
                let _ = (self.registers.c & 0x01) != 0;
            }
            0x49 => { // BIT 1, C
                let _ = (self.registers.c & 0x02) != 0;
            }
            0x51 => { // BIT 2, C
                let _ = (self.registers.c & 0x04) != 0;
            }
            0x59 => { // BIT 3, C
                let _ = (self.registers.c & 0x08) != 0;
            }
            0x61 => { // BIT 4, C
                let _ = (self.registers.c & 0x10) != 0;
            }
            0x69 => { // BIT 5, C
                let _ = (self.registers.c & 0x20) != 0;
            }
            0x71 => { // BIT 6, C
                let _ = (self.registers.c & 0x40) != 0;
            }
            0x79 => { // BIT 7, C
                let _ = (self.registers.c & 0x80) != 0;
            }
            0x81 => { // RES 0, C
                self.registers.c &= !0x01;
            }
            0x89 => { // RES 1, C
                self.registers.c &= !0x02;
            }
            0x91 => { // RES 2, C
                self.registers.c &= !0x04;
            }
            0x99 => { // RES 3, C
                self.registers.c &= !0x08;
            }
            0xA1 => { // RES 4, C
                self.registers.c &= !0x10;
            }
            0xA9 => { // RES 5, C
                self.registers.c &= !0x20;
            }
            0xB1 => { // RES 6, C
                self.registers.c &= !0x40;
            }
            0xB9 => { // RES 7, C
                self.registers.c &= !0x80;
            }
            0xC1 => { // SET 0, C
                self.registers.c |= 0x01;
            }
            0xC9 => { // SET 1, C
                self.registers.c |= 0x02;
            }
            0xD1 => { // SET 2, C
                self.registers.c |= 0x04;
            }
            0xD9 => { // SET 3, C
                self.registers.c |= 0x08;
            }
            0xE1 => { // SET 4, C
                self.registers.c |= 0x10;
            }
            0xE9 => { // SET 5, C
                self.registers.c |= 0x20;
            }
            0xF1 => { // SET 6, C
                self.registers.c |= 0x40;
            }
            0xF9 => { // SET 7, C
                self.registers.c |= 0x80;
            }
            
        // ...你可以依需求繼續補齊其他 CB 指令...
        _ => {
            let mut set = UNIMPL_OPCODES.lock().unwrap();
            if set.insert(cb_opcode) {
                println!("未實作的 CB 指令: 0xCB{:02X} 在 PC: 0x{:04X}", cb_opcode, self.registers.pc);
            }
        }
    }
}
}

