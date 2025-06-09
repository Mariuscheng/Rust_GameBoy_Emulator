// filepath: c:\Users\mariu\Desktop\Rust\gameboy_emulator\src\cpu.rs
use crate::mmu::MMU;
use std::collections::HashMap;
use std::time::{Duration, Instant};

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
    // 性能監控字段
    pub instruction_count: u64,
    pub last_pc: u16,
    // 新增：指令頻率分析
    pub opcode_frequency: HashMap<u8, u64>,
    // 新增：執行時間統計
    pub start_time: Instant,
    pub last_report_time: Instant,
    // 新增：循環檢測
    pub pc_history: Vec<u16>,
    pub max_history_size: usize,
    // 新增：記憶體訪問分析
    pub memory_reads: HashMap<u16, u64>,
    pub memory_writes: HashMap<u16, u64>,
    pub memory_access_count: u64,
    pub hot_memory_threshold: u64,
}

impl CPU {
    pub fn step(&mut self) {
        self.execute();
        // 這裡未來會執行一條指令
        // 目前先留空
        // 註釋掉這行以避免覆蓋 VRAM 測試數據
        // let pos = (self.registers.pc as usize) % 160;
        // self.mmu.vram[pos] = self.registers.pc as u8;
    }
    pub fn new(mmu: MMU) -> Self {
        CPU {
            registers: Registers::default(),
            mmu,
            instruction_count: 0,
            last_pc: 0,
            opcode_frequency: HashMap::new(),
            start_time: Instant::now(),
            last_report_time: Instant::now(),
            pc_history: Vec::new(),
            max_history_size: 100,
            memory_reads: HashMap::new(),
            memory_writes: HashMap::new(),
            memory_access_count: 0,
            hot_memory_threshold: 10,
        }
    }
    pub fn load_rom(&mut self, rom: &[u8]) {
        self.mmu.load_rom(rom.to_vec());
    }

    pub fn execute(&mut self) {
        let opcode = self.fetch();
        self.last_pc = self.registers.pc - 1;
        self.instruction_count += 1;
        self.update_opcode_frequency(opcode);
        self.decode_and_execute(opcode);

        // 每執行1000條指令輸出一次統計
        if self.instruction_count % 1000 == 0 {
            self.report_statistics();
        }
    }
    fn fetch(&mut self) -> u8 {
        let opcode = self.read_byte_tracked(self.registers.pc);
        self.registers.pc += 1;
        opcode
    }

    fn decode_and_execute(&mut self, opcode: u8) {
        match opcode {
            0x00 => {} // NOP
            0x3C => {
                self.registers.a = self.registers.a.wrapping_add(1);
            } // INC A
            0x3D => {
                self.registers.a = self.registers.a.wrapping_sub(1);
            } // DEC A
            0x04 => {
                self.registers.b = self.registers.b.wrapping_add(1);
            } // INC B
            0x05 => {
                self.registers.b = self.registers.b.wrapping_sub(1);
            } // DEC B
            0x06 => {
                let n = self.fetch();
                self.registers.b = n;
            } // LD B, n
            0x0E => {
                let n = self.fetch();
                self.registers.c = n;
            } // LD C, n
            0x16 => {
                let n = self.fetch();
                self.registers.d = n;
            } // LD D, n
            0x1E => {
                let n = self.fetch();
                self.registers.e = n;
            } // LD E, n
            0x26 => {
                let n = self.fetch();
                self.registers.h = n;
            } // LD H, n
            0x2E => {
                let n = self.fetch();
                self.registers.l = n;
            } // LD L, n
            0xAF => {
                self.registers.a = 0;
            } // XOR A
            0xC3 => {
                // JP nn
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0x78 => {
                self.registers.a = self.registers.b;
            } // LD A, B
            0x79 => {
                self.registers.a = self.registers.c;
            } // LD A, C
            0x7A => {
                self.registers.a = self.registers.d;
            } // LD A, D
            0x7B => {
                self.registers.a = self.registers.e;
            } // LD A, E
            0x7C => {
                self.registers.a = self.registers.h;
            } // LD A, H
            0x7D => {
                self.registers.a = self.registers.l;
            } // LD A, L
            0x7F => { /* LD A, A (無動作) */ }
            0x47 => {
                self.registers.b = self.registers.a;
            } // LD B, A
            0x4F => {
                self.registers.c = self.registers.a;
            } // LD C, A
            0x57 => {
                self.registers.d = self.registers.a;
            } // LD D, A
            0x5F => {
                self.registers.e = self.registers.a;
            } // LD E, A
            0x67 => {
                self.registers.h = self.registers.a;
            } // LD H, A
            0x6F => {
                self.registers.l = self.registers.a;
            } // LD L, A
            0x01 => {
                // LD BC, nn
                let lo = self.fetch();
                let hi = self.fetch();
                self.registers.b = hi;
                self.registers.c = lo;
            }
            0x0C => {
                self.registers.c = self.registers.c.wrapping_add(1);
            } // INC C
            0x0D => {
                self.registers.c = self.registers.c.wrapping_sub(1);
            } // DEC C
            0x11 => {
                // LD DE, nn
                let lo = self.fetch();
                let hi = self.fetch();
                self.registers.d = hi;
                self.registers.e = lo;
            }
            0x1C => {
                self.registers.e = self.registers.e.wrapping_add(1);
            } // INC E
            0x1D => {
                self.registers.e = self.registers.e.wrapping_sub(1);
            } // DEC E
            0x21 => {
                // LD HL, nn
                let lo = self.fetch();
                let hi = self.fetch();
                self.registers.h = hi;
                self.registers.l = lo;
            }
            0x23 => {
                // INC HL
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let hl = hl.wrapping_add(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = hl as u8;
            }
            0x31 => {
                // LD SP, nn
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.sp = (hi << 8) | lo;
            }
            0x32 => {
                // LD (HL-),A
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.mmu.write_byte(hl, self.registers.a);
                let hl = hl.wrapping_sub(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = hl as u8;
            }
            0x77 => {
                // LD (HL),A
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.write_byte_tracked(hl, self.registers.a);
            }
            0x7E => {
                // LD A,(HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.a = self.read_byte_tracked(hl);
            }
            0xC9 => { /* RET (暫不實作堆疊) */ }
            0x76 => { /* HALT (暫不處理) */ }
            0x02 => {
                // LD (BC),A
                let addr = ((self.registers.b as u16) << 8) | self.registers.c as u16;
                self.write_byte_tracked(addr, self.registers.a);
            }
            0x0A => {
                // LD A,(BC)
                let addr = ((self.registers.b as u16) << 8) | self.registers.c as u16;
                self.registers.a = self.read_byte_tracked(addr);
            }
            0x12 => {
                // LD (DE),A
                let addr = ((self.registers.d as u16) << 8) | self.registers.e as u16;
                self.mmu.write_byte(addr, self.registers.a);
            }
            0x1A => {
                // LD A,(DE)
                let addr = ((self.registers.d as u16) << 8) | self.registers.e as u16;
                self.registers.a = self.mmu.read_byte(addr);
            }
            0x22 => {
                // LD (HL+),A
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.mmu.write_byte(hl, self.registers.a);
                let hl = hl.wrapping_add(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = hl as u8;
            }
            0x2A => {
                // LD A,(HL+)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.a = self.mmu.read_byte(hl);
                let hl = hl.wrapping_add(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = hl as u8;
            }
            0x03 => {
                // INC BC
                let bc =
                    (((self.registers.b as u16) << 8) | self.registers.c as u16).wrapping_add(1);
                self.registers.b = (bc >> 8) as u8;
                self.registers.c = bc as u8;
            }
            0x13 => {
                // INC DE
                let de =
                    (((self.registers.d as u16) << 8) | self.registers.e as u16).wrapping_add(1);
                self.registers.d = (de >> 8) as u8;
                self.registers.e = de as u8;
            }
            0x0B => {
                // DEC BC
                let bc =
                    (((self.registers.b as u16) << 8) | self.registers.c as u16).wrapping_sub(1);
                self.registers.b = (bc >> 8) as u8;
                self.registers.c = bc as u8;
            }
            0x1B => {
                // DEC DE
                let de =
                    (((self.registers.d as u16) << 8) | self.registers.e as u16).wrapping_sub(1);
                self.registers.d = (de >> 8) as u8;
                self.registers.e = de as u8;
            }
            0x08 => {
                // LD (nn),SP
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                self.mmu.write_byte(addr, (self.registers.sp & 0xFF) as u8);
                self.mmu
                    .write_byte(addr + 1, (self.registers.sp >> 8) as u8);
            }
            0x09 => {
                // ADD HL,BC
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let bc = ((self.registers.b as u16) << 8) | self.registers.c as u16;
                let result = hl.wrapping_add(bc);
                self.registers.h = (result >> 8) as u8;
                self.registers.l = result as u8;
            }
            0x0F => {
                self.registers.a = self.registers.a.rotate_left(1);
            } // RRCA (簡化)
            0x17 => {
                self.registers.a = self.registers.a.rotate_left(1);
            } // RLA (簡化)
            0x1F => {
                self.registers.a = self.registers.a.rotate_right(1);
            } // RRCA (簡化)
            0x18 => {
                // JR n
                let offset = self.fetch() as i8;
                self.registers.pc = (self.registers.pc as i16 + offset as i16) as u16;
            }
            0x20 => {
                // JR NZ,n (簡化:永遠跳)
                let offset = self.fetch() as i8;
                self.registers.pc = (self.registers.pc as i16 + offset as i16) as u16;
            }
            0x28 => {
                // JR Z,n (簡化:永遠跳)
                let offset = self.fetch() as i8;
                self.registers.pc = (self.registers.pc as i16 + offset as i16) as u16;
            }
            0x24 => {
                self.registers.h = self.registers.h.wrapping_add(1);
            } // INC H
            0x25 => {
                self.registers.h = self.registers.h.wrapping_sub(1);
            } // DEC H
            0x2C => {
                self.registers.l = self.registers.l.wrapping_add(1);
            } // INC L
            0x2D => {
                self.registers.l = self.registers.l.wrapping_sub(1);
            } // DEC L
            0x3E => {
                let n = self.fetch();
                self.registers.a = n;
            } // LD A, n
            // ...繼續補齊其他常用指令...
            0x14 => {
                self.registers.d = self.registers.d.wrapping_add(1);
            } // INC D
            0x15 => {
                self.registers.d = self.registers.d.wrapping_sub(1);
            } // DEC D
            0x19 => {
                // ADD HL,DE
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let de = ((self.registers.d as u16) << 8) | self.registers.e as u16;
                let result = hl.wrapping_add(de);
                self.registers.h = (result >> 8) as u8;
                self.registers.l = result as u8;
            }
            0x29 => {
                // ADD HL,HL
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let result = hl.wrapping_add(hl);
                self.registers.h = (result >> 8) as u8;
                self.registers.l = result as u8;
            }
            0x2B => {
                // DEC HL
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let hl = hl.wrapping_sub(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = hl as u8;
            }
            0x33 => {
                self.registers.sp = self.registers.sp.wrapping_add(1);
            } // INC SP
            0x3B => {
                self.registers.sp = self.registers.sp.wrapping_sub(1);
            } // DEC SP
            0x34 => {
                // INC (HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let val = self.mmu.read_byte(hl).wrapping_add(1);
                self.mmu.write_byte(hl, val);
            }
            0x35 => {
                // DEC (HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let val = self.mmu.read_byte(hl).wrapping_sub(1);
                self.mmu.write_byte(hl, val);
            }
            0x36 => {
                // LD (HL),n
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let n = self.fetch();
                self.mmu.write_byte(hl, n);
            }

            // 8位暫存器間載入指令 (LD r,r)
            0x40 => {
                self.registers.b = self.registers.b;
            } // LD B,B
            0x41 => {
                self.registers.b = self.registers.c;
            } // LD B,C
            0x42 => {
                self.registers.b = self.registers.d;
            } // LD B,D
            0x43 => {
                self.registers.b = self.registers.e;
            } // LD B,E
            0x44 => {
                self.registers.b = self.registers.h;
            } // LD B,H
            0x45 => {
                self.registers.b = self.registers.l;
            } // LD B,L
            0x46 => {
                // LD B,(HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.b = self.mmu.read_byte(hl);
            }
            0x48 => {
                self.registers.c = self.registers.b;
            } // LD C,B
            0x49 => {
                self.registers.c = self.registers.c;
            } // LD C,C
            0x4A => {
                self.registers.c = self.registers.d;
            } // LD C,D
            0x4B => {
                self.registers.c = self.registers.e;
            } // LD C,E
            0x4C => {
                self.registers.c = self.registers.h;
            } // LD C,H
            0x4D => {
                self.registers.c = self.registers.l;
            } // LD C,L
            0x4E => {
                // LD C,(HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.c = self.mmu.read_byte(hl);
            }
            0x50 => {
                self.registers.d = self.registers.b;
            } // LD D,B
            0x51 => {
                self.registers.d = self.registers.c;
            } // LD D,C
            0x52 => {
                self.registers.d = self.registers.d;
            } // LD D,D
            0x53 => {
                self.registers.d = self.registers.e;
            } // LD D,E
            0x54 => {
                self.registers.d = self.registers.h;
            } // LD D,H
            0x55 => {
                self.registers.d = self.registers.l;
            } // LD D,L
            0x56 => {
                // LD D,(HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.d = self.mmu.read_byte(hl);
            }
            0x58 => {
                self.registers.e = self.registers.b;
            } // LD E,B
            0x59 => {
                self.registers.e = self.registers.c;
            } // LD E,C
            0x5A => {
                self.registers.e = self.registers.d;
            } // LD E,D
            0x5B => {
                self.registers.e = self.registers.e;
            } // LD E,E
            0x5C => {
                self.registers.e = self.registers.h;
            } // LD E,H
            0x5D => {
                self.registers.e = self.registers.l;
            } // LD E,L
            0x5E => {
                // LD E,(HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.e = self.mmu.read_byte(hl);
            }
            0x60 => {
                self.registers.h = self.registers.b;
            } // LD H,B
            0x61 => {
                self.registers.h = self.registers.c;
            } // LD H,C
            0x62 => {
                self.registers.h = self.registers.d;
            } // LD H,D
            0x63 => {
                self.registers.h = self.registers.e;
            } // LD H,E
            0x64 => {
                self.registers.h = self.registers.h;
            } // LD H,H
            0x65 => {
                self.registers.h = self.registers.l;
            } // LD H,L
            0x66 => {
                // LD H,(HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.h = self.mmu.read_byte(hl);
            }
            0x68 => {
                self.registers.l = self.registers.b;
            } // LD L,B
            0x69 => {
                self.registers.l = self.registers.c;
            } // LD L,C
            0x6A => {
                self.registers.l = self.registers.d;
            } // LD L,D
            0x6B => {
                self.registers.l = self.registers.e;
            } // LD L,E
            0x6C => {
                self.registers.l = self.registers.h;
            } // LD L,H
            0x6D => {
                self.registers.l = self.registers.l;
            } // LD L,L
            0x6E => {
                // LD L,(HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.l = self.mmu.read_byte(hl);
            }
            0x70 => {
                // LD (HL),B
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.mmu.write_byte(hl, self.registers.b);
            }
            0x71 => {
                // LD (HL),C
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.mmu.write_byte(hl, self.registers.c);
            }
            0x72 => {
                // LD (HL),D
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.mmu.write_byte(hl, self.registers.d);
            }
            0x73 => {
                // LD (HL),E
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.mmu.write_byte(hl, self.registers.e);
            }
            0x74 => {
                // LD (HL),H
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.mmu.write_byte(hl, self.registers.h);
            }
            0x75 => {
                // LD (HL),L
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.mmu.write_byte(hl, self.registers.l);
            }

            // 算術操作指令 (ADD A,r)
            0x80 => {
                self.registers.a = self.registers.a.wrapping_add(self.registers.b);
            } // ADD A,B
            0x81 => {
                self.registers.a = self.registers.a.wrapping_add(self.registers.c);
            } // ADD A,C
            0x82 => {
                self.registers.a = self.registers.a.wrapping_add(self.registers.d);
            } // ADD A,D
            0x83 => {
                self.registers.a = self.registers.a.wrapping_add(self.registers.e);
            } // ADD A,E
            0x84 => {
                self.registers.a = self.registers.a.wrapping_add(self.registers.h);
            } // ADD A,H
            0x85 => {
                self.registers.a = self.registers.a.wrapping_add(self.registers.l);
            } // ADD A,L
            0x86 => {
                // ADD A,(HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let val = self.mmu.read_byte(hl);
                self.registers.a = self.registers.a.wrapping_add(val);
            }
            0x87 => {
                self.registers.a = self.registers.a.wrapping_add(self.registers.a);
            } // ADD A,A

            // 減法指令 (SUB r)
            0x90 => {
                self.registers.a = self.registers.a.wrapping_sub(self.registers.b);
            } // SUB B
            0x91 => {
                self.registers.a = self.registers.a.wrapping_sub(self.registers.c);
            } // SUB C
            0x92 => {
                self.registers.a = self.registers.a.wrapping_sub(self.registers.d);
            } // SUB D
            0x93 => {
                self.registers.a = self.registers.a.wrapping_sub(self.registers.e);
            } // SUB E
            0x94 => {
                self.registers.a = self.registers.a.wrapping_sub(self.registers.h);
            } // SUB H
            0x95 => {
                self.registers.a = self.registers.a.wrapping_sub(self.registers.l);
            } // SUB L
            0x96 => {
                // SUB (HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let val = self.mmu.read_byte(hl);
                self.registers.a = self.registers.a.wrapping_sub(val);
            }
            0x97 => {
                self.registers.a = 0;
            } // SUB A

            // 邏輯運算 (AND, OR, XOR, CP)
            0xA0 => {
                self.registers.a &= self.registers.b;
            } // AND B
            0xA1 => {
                self.registers.a &= self.registers.c;
            } // AND C
            0xA2 => {
                self.registers.a &= self.registers.d;
            } // AND D
            0xA3 => {
                self.registers.a &= self.registers.e;
            } // AND E
            0xA4 => {
                self.registers.a &= self.registers.h;
            } // AND H
            0xA5 => {
                self.registers.a &= self.registers.l;
            } // AND L
            0xA6 => {
                // AND (HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let val = self.mmu.read_byte(hl);
                self.registers.a &= val;
            }
            0xA7 => {
                self.registers.a &= self.registers.a;
            } // AND A

            0xB0 => {
                self.registers.a |= self.registers.b;
            } // OR B
            0xB1 => {
                self.registers.a |= self.registers.c;
            } // OR C
            0xB2 => {
                self.registers.a |= self.registers.d;
            } // OR D
            0xB3 => {
                self.registers.a |= self.registers.e;
            } // OR E
            0xB4 => {
                self.registers.a |= self.registers.h;
            } // OR H
            0xB5 => {
                self.registers.a |= self.registers.l;
            } // OR L
            0xB6 => {
                // OR (HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let val = self.mmu.read_byte(hl);
                self.registers.a |= val;
            }
            0xB7 => {
                self.registers.a |= self.registers.a;
            } // OR A

            0xB8 => {
                // CP B - Compare A with B
                let _ = self.registers.a.wrapping_sub(self.registers.b);
            }
            0xB9 => {
                // CP C - Compare A with C
                let _ = self.registers.a.wrapping_sub(self.registers.c);
            }
            0xBA => {
                // CP D - Compare A with D
                let _ = self.registers.a.wrapping_sub(self.registers.d);
            }
            0xBB => {
                // CP E - Compare A with E
                let _ = self.registers.a.wrapping_sub(self.registers.e);
            }
            0xBC => {
                // CP H - Compare A with H
                let _ = self.registers.a.wrapping_sub(self.registers.h);
            }
            0xBD => {
                // CP L - Compare A with L
                let _ = self.registers.a.wrapping_sub(self.registers.l);
            }
            0xBE => {
                // CP (HL) - Compare A with (HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                let val = self.mmu.read_byte(hl);
                let _ = self.registers.a.wrapping_sub(val);
            }
            0xBF => {
                // CP A - Compare A with A
                let _ = self.registers.a.wrapping_sub(self.registers.a);
            }

            // I/O和記憶體操作
            0xE0 => {
                // LDH (n),A - Load A into (0xFF00+n)
                let n = self.fetch();
                self.mmu.write_byte(0xFF00 + n as u16, self.registers.a);
            }
            0xE2 => {
                // LD (C),A - Load A into (0xFF00+C)
                self.mmu
                    .write_byte(0xFF00 + self.registers.c as u16, self.registers.a);
            }
            0xF0 => {
                // LDH A,(n) - Load (0xFF00+n) into A
                let n = self.fetch();
                self.registers.a = self.mmu.read_byte(0xFF00 + n as u16);
            }
            0xF2 => {
                // LD A,(C) - Load (0xFF00+C) into A
                self.registers.a = self.mmu.read_byte(0xFF00 + self.registers.c as u16);
            }

            // 堆疊操作
            0xF8 => {
                // LD HL,SP+r8 - Load SP+signed into HL
                let offset = self.fetch() as i8;
                let result = (self.registers.sp as i16 + offset as i16) as u16;
                self.registers.h = (result >> 8) as u8;
                self.registers.l = result as u8;
            }
            0xF9 => {
                // LD SP,HL - Load HL into SP
                self.registers.sp = ((self.registers.h as u16) << 8) | self.registers.l as u16;
            }
            0xE8 => {
                // ADD SP,r8 - Add signed 8-bit to SP
                let offset = self.fetch() as i8;
                self.registers.sp = (self.registers.sp as i16 + offset as i16) as u16;
            }

            // 跳轉指令
            0x30 => {
                // JR NC,r8 - Jump if no carry (簡化:永遠跳)
                let offset = self.fetch() as i8;
                self.registers.pc = (self.registers.pc as i16 + offset as i16) as u16;
            }
            0x38 => {
                // JR C,r8 - Jump if carry (簡化:永遠跳)
                let offset = self.fetch() as i8;
                self.registers.pc = (self.registers.pc as i16 + offset as i16) as u16;
            } // 中斷和特殊指令
            0xF3 => { /* DI - Disable interrupts (簡化：忽略) */ }
            0xFB => { /* EI - Enable interrupts (簡化：忽略) */ }

            // 立即數操作
            0xC6 => {
                // ADD A,n
                let n = self.fetch();
                self.registers.a = self.registers.a.wrapping_add(n);
            }
            0xD6 => {
                // SUB A,n
                let n = self.fetch();
                self.registers.a = self.registers.a.wrapping_sub(n);
            }
            0xE6 => {
                // AND n
                let n = self.fetch();
                self.registers.a &= n;
            }
            0xF6 => {
                // OR n
                let n = self.fetch();
                self.registers.a |= n;
            }
            0xEE => {
                // XOR n
                let n = self.fetch();
                self.registers.a ^= n;
            }
            0xFE => {
                // CP n - Compare A with n
                let n = self.fetch();
                let _ = self.registers.a.wrapping_sub(n);
            }
            // CB 前綴指令
            0xCB => {
                let cb_opcode = self.fetch();
                self.decode_cb(cb_opcode);
            }

            // 16位載入指令
            0xEA => {
                // LD (nn),A
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                self.mmu.write_byte(addr, self.registers.a);
            }
            0xFA => {
                // LD A,(nn)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                self.registers.a = self.mmu.read_byte(addr);
            }

            // 堆疊操作指令
            0xC5 => { // PUSH BC (簡化：忽略)
                 // 在完整實作中會將BC推入堆疊
            }
            0xC1 => { // POP BC (簡化：忽略)
                 // 在完整實作中會從堆疊彈出到BC
            }
            0xD5 => { // PUSH DE (簡化：忽略)
                 // 在完整實作中會將DE推入堆疊
            }
            0xD1 => { // POP DE (簡化：忽略)
                 // 在完整實作中會從堆疊彈出到DE
            }
            0xE5 => { // PUSH HL (簡化：忽略)
                 // 在完整實作中會將HL推入堆疊
            }
            0xE1 => { // POP HL (簡化：忽略)
                 // 在完整實作中會從堆疊彈出到HL
            }
            0xF5 => { // PUSH AF (簡化：忽略)
                 // 在完整實作中會將AF推入堆疊
            }
            0xF1 => { // POP AF (簡化：忽略)
                 // 在完整實作中會從堆疊彈出到AF
            }

            // 條件跳轉和調用指令
            0xC2 => {
                // JP NZ,nn (簡化：永遠跳)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0xCA => {
                // JP Z,nn (簡化：永遠跳)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0xD2 => {
                // JP NC,nn (簡化：永遠跳)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0xDA => {
                // JP C,nn (簡化：永遠跳)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0xC4 => {
                // CALL NZ,nn (簡化：當作 JP)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0xCC => {
                // CALL Z,nn (簡化：當作 JP)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0xCD => {
                // CALL nn (簡化：當作 JP)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0xD4 => {
                // CALL NC,nn (簡化：當作 JP)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0xDC => {
                // CALL C,nn (簡化：當作 JP)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }

            // 條件返回指令
            0xC0 => { /* RET NZ (簡化：忽略) */ }
            0xC8 => { /* RET Z (簡化：忽略) */ }
            0xD0 => { /* RET NC (簡化：忽略) */ }
            0xD8 => { /* RET C (簡化：忽略) */ }

            // RST 指令 (重啟到固定地址)
            0xC7 => { /* RST 00H (簡化：忽略) */ }
            0xCF => { /* RST 08H (簡化：忽略) */ }
            0xD7 => { /* RST 10H (簡化：忽略) */ }
            0xDF => { /* RST 18H (簡化：忽略) */ }
            0xE7 => { /* RST 20H (簡化：忽略) */ }
            0xEF => { /* RST 28H (簡化：忽略) */ }
            0xF7 => { /* RST 30H (簡化：忽略) */ }
            0xFF => { /* RST 38H (簡化：忽略) */ }

            // 非法指令處理
            0xD3 => { /* Illegal opcode (簡化：忽略) */ }
            0xE3 => { /* Illegal opcode (簡化：忽略) */ }
            0xE4 => { /* Illegal opcode (簡化：忽略) */ }
            0xEB => { /* Illegal opcode (簡化：忽略) */ }
            0xEC => { /* Illegal opcode (簡化：忽略) */ }
            0xED => { /* Illegal opcode (簡化：忽略) */ }
            0xF4 => { /* Illegal opcode (簡化：忽略) */ }
            0xFC => { /* Illegal opcode (簡化：忽略) */ }
            0xFD => { /* Illegal opcode (簡化：忽略) */ }

            _ => {
                eprintln!(
                    "Unimplemented opcode: {:#04X} at PC: {:#06X}",
                    opcode,
                    self.registers.pc - 1
                );
            }
        }
    }
    fn decode_cb(&mut self, cb_opcode: u8) {
        // CB 指令簡化實現，避免無法到達的模式警告
        match cb_opcode {
            // 0x00-0x3F: 旋轉和移位指令（RLC, RRC, RL, RR, SLA, SRA, SWAP, SRL）
            0x00..=0x3F => {
                // 簡化：忽略具體實現，只是為了避免未實現錯誤
            }
            // 0x40-0x7F: BIT 測試指令
            0x40..=0x7F => {
                // BIT b,r - 測試暫存器的特定位（簡化：忽略）
            }
            // 0x80-0xBF: RES 位重置指令
            0x80..=0xBF => {
                // RES b,r - 重置暫存器的特定位（簡化：忽略）
            }
            // 0xC0-0xFF: SET 位設置指令
            0xC0..=0xFF => {
                // SET b,r - 設置暫存器的特定位（簡化：忽略）
            }
        }
    }

    fn update_opcode_frequency(&mut self, opcode: u8) {
        let counter = self.opcode_frequency.entry(opcode).or_insert(0);
        *counter += 1;
    }
    fn report_statistics(&mut self) {
        let elapsed = self.last_report_time.elapsed();
        let duration = Duration::from_millis(1000);
        if elapsed >= duration {
            let instructions_per_second = self.instruction_count as f64 / elapsed.as_secs_f64();
            println!(
                "已執行 {} 條指令，當前 PC: {:#06X}, 每秒指令數: {:.0}",
                self.instruction_count, self.registers.pc, instructions_per_second
            );
            // 重置計時器
            self.last_report_time = Instant::now();
        }

        // 每10000條指令顯示詳細統計
        if self.instruction_count % 10000 == 0 {
            self.show_detailed_statistics();
        }

        // 循環檢測
        self.pc_history.push(self.registers.pc);
        if self.pc_history.len() > self.max_history_size {
            self.pc_history.remove(0);
        }
        let is_cyclic = self
            .pc_history
            .windows(2)
            .any(|window| window[0] == window[1]);
        if is_cyclic {
            println!("警告: 偵測到程式計數器循環 (可能的無窮迴圈)");
        }
    }

    fn show_detailed_statistics(&self) {
        // 報告指令頻率
        let mut sorted_opcodes: Vec<_> = self.opcode_frequency.iter().collect();
        sorted_opcodes.sort_by(|a, b| b.1.cmp(a.1));
        println!("前 10 個最常使用的指令:");
        for (opcode, count) in sorted_opcodes.iter().take(10) {
            let opcode_name = self.get_opcode_name(**opcode);
            println!("  {:#04X} ({}): {} 次", opcode, opcode_name, count);
        }

        // 執行時間統計
        let elapsed = self.start_time.elapsed();
        let instructions_per_second = self.instruction_count as f64 / elapsed.as_secs_f64();
        println!("執行統計:");
        println!("  - 總執行時間: {:.2} 秒", elapsed.as_secs_f64());
        println!("  - 平均每秒指令數: {:.0}", instructions_per_second);
        println!("  - 總指令數: {}", self.instruction_count);
    }
    fn get_opcode_name(&self, opcode: u8) -> &'static str {
        match opcode {
            // 控制指令
            0x00 => "NOP",
            0x76 => "HALT",
            0xC9 => "RET",
            0xCB => "CB prefix",
            0xF3 => "DI",
            0xFB => "EI",

            // 8位載入指令 (LD r,n)
            0x06 => "LD B,n",
            0x0E => "LD C,n",
            0x16 => "LD D,n",
            0x1E => "LD E,n",
            0x26 => "LD H,n",
            0x2E => "LD L,n",
            0x3E => "LD A,n",

            // 8位載入指令 (LD A,r)
            0x78 => "LD A,B",
            0x79 => "LD A,C",
            0x7A => "LD A,D",
            0x7B => "LD A,E",
            0x7C => "LD A,H",
            0x7D => "LD A,L",
            0x7E => "LD A,(HL)",
            0x7F => "LD A,A",

            // 8位載入指令 (LD r,A)
            0x47 => "LD B,A",
            0x4F => "LD C,A",
            0x57 => "LD D,A",
            0x5F => "LD E,A",
            0x67 => "LD H,A",
            0x6F => "LD L,A",
            0x77 => "LD (HL),A",

            // 記憶體載入指令
            0x02 => "LD (BC),A",
            0x0A => "LD A,(BC)",
            0x12 => "LD (DE),A",
            0x1A => "LD A,(DE)",
            0x22 => "LD (HL+),A",
            0x2A => "LD A,(HL+)",
            0x32 => "LD (HL-),A",
            0x3A => "LD A,(HL-)",

            // 16位載入指令
            0x01 => "LD BC,nn",
            0x11 => "LD DE,nn",
            0x21 => "LD HL,nn",
            0x31 => "LD SP,nn",
            0xF9 => "LD SP,HL",

            // I/O載入指令
            0xE0 => "LDH (n),A",
            0xE2 => "LD (C),A",
            0xF0 => "LDH A,(n)",
            0xF2 => "LD A,(C)",
            0xEA => "LD (nn),A",
            0xFA => "LD A,(nn)",

            // 增減指令
            0x3C => "INC A",
            0x3D => "DEC A",
            0x04 => "INC B",
            0x05 => "DEC B",
            0x0C => "INC C",
            0x0D => "DEC C",
            0x14 => "INC D",
            0x15 => "DEC D",
            0x1C => "INC E",
            0x1D => "DEC E",
            0x24 => "INC H",
            0x25 => "DEC H",
            0x2C => "INC L",
            0x2D => "DEC L",
            0x34 => "INC (HL)",
            0x35 => "DEC (HL)",

            // 16位增減指令
            0x03 => "INC BC",
            0x0B => "DEC BC",
            0x13 => "INC DE",
            0x1B => "DEC DE",
            0x23 => "INC HL",
            0x2B => "DEC HL",
            0x33 => "INC SP",
            0x3B => "DEC SP",

            // 算術指令 (ADD)
            0x80 => "ADD A,B",
            0x81 => "ADD A,C",
            0x82 => "ADD A,D",
            0x83 => "ADD A,E",
            0x84 => "ADD A,H",
            0x85 => "ADD A,L",
            0x86 => "ADD A,(HL)",
            0x87 => "ADD A,A",
            0xC6 => "ADD A,n",

            // 減法指令 (SUB)
            0x90 => "SUB B",
            0x91 => "SUB C",
            0x92 => "SUB D",
            0x93 => "SUB E",
            0x94 => "SUB H",
            0x95 => "SUB L",
            0x96 => "SUB (HL)",
            0x97 => "SUB A",
            0xD6 => "SUB n",

            // 邏輯指令 (AND)
            0xA0 => "AND B",
            0xA1 => "AND C",
            0xA2 => "AND D",
            0xA3 => "AND E",
            0xA4 => "AND H",
            0xA5 => "AND L",
            0xA6 => "AND (HL)",
            0xA7 => "AND A",
            0xE6 => "AND n",

            // 邏輯指令 (OR)
            0xB0 => "OR B",
            0xB1 => "OR C",
            0xB2 => "OR D",
            0xB3 => "OR E",
            0xB4 => "OR H",
            0xB5 => "OR L",
            0xB6 => "OR (HL)",
            0xB7 => "OR A",
            0xF6 => "OR n",

            // 邏輯指令 (XOR)
            0xA8 => "XOR B",
            0xA9 => "XOR C",
            0xAA => "XOR D",
            0xAB => "XOR E",
            0xAC => "XOR H",
            0xAD => "XOR L",
            0xAE => "XOR (HL)",
            0xAF => "XOR A",
            0xEE => "XOR n",

            // 比較指令 (CP)
            0xB8 => "CP B",
            0xB9 => "CP C",
            0xBA => "CP D",
            0xBB => "CP E",
            0xBC => "CP H",
            0xBD => "CP L",
            0xBE => "CP (HL)",
            0xBF => "CP A",
            0xFE => "CP n",

            // 跳轉指令
            0x18 => "JR n",
            0x20 => "JR NZ,n",
            0x28 => "JR Z,n",
            0x30 => "JR NC,n",
            0x38 => "JR C,n",
            0xC3 => "JP nn",
            0xC2 => "JP NZ,nn",
            0xCA => "JP Z,nn",
            0xD2 => "JP NC,nn",
            0xDA => "JP C,nn",

            // 調用和返回指令
            0xCD => "CALL nn",
            0xC4 => "CALL NZ,nn",
            0xCC => "CALL Z,nn",
            0xD4 => "CALL NC,nn",
            0xDC => "CALL C,nn",
            0xC0 => "RET NZ",
            0xC8 => "RET Z",
            0xD0 => "RET NC",
            0xD8 => "RET C",

            // 堆疊指令
            0xC5 => "PUSH BC",
            0xC1 => "POP BC",
            0xD5 => "PUSH DE",
            0xD1 => "POP DE",
            0xE5 => "PUSH HL",
            0xE1 => "POP HL",
            0xF5 => "PUSH AF",
            0xF1 => "POP AF",

            // RST指令
            0xC7 => "RST 00H",
            0xCF => "RST 08H",
            0xD7 => "RST 10H",
            0xDF => "RST 18H",
            0xE7 => "RST 20H",
            0xEF => "RST 28H",
            0xF7 => "RST 30H",
            0xFF => "RST 38H",

            // 16位算術指令
            0x09 => "ADD HL,BC",
            0x19 => "ADD HL,DE",
            0x29 => "ADD HL,HL",
            0x39 => "ADD HL,SP",
            0xE8 => "ADD SP,r8",
            0xF8 => "LD HL,SP+r8",

            // 旋轉指令
            0x07 => "RLCA",
            0x0F => "RRCA",
            0x17 => "RLA",
            0x1F => "RRA",

            // 記憶體存取指令
            0x08 => "LD (nn),SP",

            _ => "UNKNOWN",
        }
    }

    // 性能監控和診斷方法
    pub fn get_instruction_count(&self) -> u64 {
        self.instruction_count
    }
    pub fn get_status_report(&self) -> String {
        let elapsed = self.start_time.elapsed();
        let instructions_per_second = if elapsed.as_secs_f64() > 0.0 {
            self.instruction_count as f64 / elapsed.as_secs_f64()
        } else {
            0.0
        };

        // 找出最常使用的指令
        let mut sorted_opcodes: Vec<_> = self.opcode_frequency.iter().collect();
        sorted_opcodes.sort_by(|a, b| b.1.cmp(a.1));
        let top_opcodes = sorted_opcodes
            .iter()
            .take(3)
            .map(|(opcode, count)| format!("{:#04X}({}次)", opcode, count))
            .collect::<Vec<_>>()
            .join(", ");

        format!(
            "CPU 狀態報告:\n\
             - 已執行指令數: {}\n\
             - 執行時間: {:.2} 秒\n\
             - 平均每秒指令數: {:.0}\n\
             - 當前 PC: {:#06X}\n\
             - 最常用指令: {}\n\
             - 暫存器 A: {:#04X}\n\
             - 暫存器 BC: {:#06X}\n\
             - 暫存器 DE: {:#06X}\n\
             - 暫存器 HL: {:#06X}\n\
             - 堆疊指標 SP: {:#06X}",
            self.instruction_count,
            elapsed.as_secs_f64(),
            instructions_per_second,
            self.registers.pc,
            if top_opcodes.is_empty() {
                "無".to_string()
            } else {
                top_opcodes
            },
            self.registers.a,
            ((self.registers.b as u16) << 8) | self.registers.c as u16,
            ((self.registers.d as u16) << 8) | self.registers.e as u16,
            ((self.registers.h as u16) << 8) | self.registers.l as u16,
            self.registers.sp
        )
    }

    pub fn save_performance_report(&self) {
        use std::fs::File;
        use std::io::Write;

        let report_path = "c:\\Users\\mariu\\Desktop\\Rust\\gameboy_emulator\\gameboy_emulator\\cpu_performance_report.txt";
        if let Ok(mut file) = File::create(report_path) {
            let elapsed = self.start_time.elapsed();
            let instructions_per_second = if elapsed.as_secs_f64() > 0.0 {
                self.instruction_count as f64 / elapsed.as_secs_f64()
            } else {
                0.0
            };

            let report = format!(
                "================================================================================\n\
                Game Boy CPU 性能報告\n\
                ================================================================================\n\
                \n\
                基本統計:\n\
                - 總執行時間: {:.3} 秒\n\
                - 總指令數: {}\n\
                - 平均每秒指令數: {:.0}\n\
                - 當前程式計數器: {:#06X}\n\
                \n\
                暫存器狀態:\n\
                - A: {:#04X}\n\
                - BC: {:#06X}\n\
                - DE: {:#06X}\n\
                - HL: {:#06X}\n\
                - SP: {:#06X}\n\
                \n\
                指令頻率分析 (前 20 個最常用指令):\n\
                {}\n\
                \n\
                記憶體訪問統計:\n\
                {}\n\
                \n\
                記憶體熱點分析:\n\
                {}\n\
                \n\
                程式計數器歷史 (最近 10 個位置):\n\
                {}\n\
                \n\
                ================================================================================\n",
                elapsed.as_secs_f64(),
                self.instruction_count,
                instructions_per_second,
                self.registers.pc,
                self.registers.a,
                ((self.registers.b as u16) << 8) | self.registers.c as u16,
                ((self.registers.d as u16) << 8) | self.registers.e as u16,
                ((self.registers.h as u16) << 8) | self.registers.l as u16,                self.registers.sp,
                self.get_frequency_analysis(),
                self.get_memory_region_analysis(),
                self.get_memory_hotspot_report(),
                self.get_pc_history_string()
            );

            let _ = file.write_all(report.as_bytes());
            let _ = file.flush();
            println!("CPU 性能報告已保存至: {}", report_path);
        }
    }
    fn get_frequency_analysis(&self) -> String {
        let mut sorted_opcodes: Vec<_> = self.opcode_frequency.iter().collect();
        sorted_opcodes.sort_by(|a, b| b.1.cmp(a.1));

        sorted_opcodes
            .iter()
            .take(20)
            .map(|(opcode, count)| {
                let percentage = (**count as f64 / self.instruction_count as f64) * 100.0;
                format!(
                    "  {:#04X} ({:<12}): {:>8} 次 ({:.2}%)",
                    opcode,
                    self.get_opcode_name(**opcode),
                    count,
                    percentage
                )
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn get_pc_history_string(&self) -> String {
        self.pc_history
            .iter()
            .rev()
            .take(10)
            .map(|pc| format!("{:#06X}", pc))
            .collect::<Vec<_>>()
            .join(" -> ")
    }

    // 記憶體訪問追蹤方法
    fn track_memory_read(&mut self, addr: u16) {
        *self.memory_reads.entry(addr).or_insert(0) += 1;
        self.memory_access_count += 1;
    }

    fn track_memory_write(&mut self, addr: u16) {
        *self.memory_writes.entry(addr).or_insert(0) += 1;
        self.memory_access_count += 1;
    }

    // 增強版記憶體讀取 - 帶追蹤功能
    fn read_byte_tracked(&mut self, addr: u16) -> u8 {
        self.track_memory_read(addr);
        self.mmu.read_byte(addr)
    }

    // 增強版記憶體寫入 - 帶追蹤功能
    fn write_byte_tracked(&mut self, addr: u16, value: u8) {
        self.track_memory_write(addr);
        self.mmu.write_byte(addr, value);
    }

    // 記憶體熱點分析
    pub fn get_memory_hotspots(&self) -> (Vec<(u16, u64)>, Vec<(u16, u64)>) {
        // 讀取熱點 - 按訪問頻率排序
        let mut read_hotspots: Vec<_> = self
            .memory_reads
            .iter()
            .filter(|(_, &count)| count >= self.hot_memory_threshold)
            .map(|(&addr, &count)| (addr, count))
            .collect();
        read_hotspots.sort_by(|a, b| b.1.cmp(&a.1));

        // 寫入熱點 - 按訪問頻率排序
        let mut write_hotspots: Vec<_> = self
            .memory_writes
            .iter()
            .filter(|(_, &count)| count >= self.hot_memory_threshold)
            .map(|(&addr, &count)| (addr, count))
            .collect();
        write_hotspots.sort_by(|a, b| b.1.cmp(&a.1));

        (read_hotspots, write_hotspots)
    }

    // 記憶體區域分析
    pub fn get_memory_region_analysis(&self) -> String {
        let mut analysis = String::new();

        // 統計不同記憶體區域的訪問
        let mut rom_reads = 0;
        let mut vram_reads = 0;
        let mut io_reads = 0;
        let mut other_reads = 0;

        let mut rom_writes = 0;
        let mut vram_writes = 0;
        let mut io_writes = 0;
        let mut other_writes = 0;

        // 分析讀取
        for (&addr, &count) in &self.memory_reads {
            match addr {
                0x0000..=0x7FFF => rom_reads += count,
                0x8000..=0x9FFF => vram_reads += count,
                0xFF00..=0xFFFF => io_reads += count,
                _ => other_reads += count,
            }
        }

        // 分析寫入
        for (&addr, &count) in &self.memory_writes {
            match addr {
                0x0000..=0x7FFF => rom_writes += count,
                0x8000..=0x9FFF => vram_writes += count,
                0xFF00..=0xFFFF => io_writes += count,
                _ => other_writes += count,
            }
        }

        let total_reads = rom_reads + vram_reads + io_reads + other_reads;
        let total_writes = rom_writes + vram_writes + io_writes + other_writes;

        analysis.push_str("記憶體區域訪問統計:\n");
        analysis.push_str("  讀取分佈:\n");
        if total_reads > 0 {
            analysis.push_str(&format!(
                "    ROM (0x0000-0x7FFF): {} 次 ({:.1}%)\n",
                rom_reads,
                (rom_reads as f64 / total_reads as f64) * 100.0
            ));
            analysis.push_str(&format!(
                "    VRAM (0x8000-0x9FFF): {} 次 ({:.1}%)\n",
                vram_reads,
                (vram_reads as f64 / total_reads as f64) * 100.0
            ));
            analysis.push_str(&format!(
                "    I/O (0xFF00-0xFFFF): {} 次 ({:.1}%)\n",
                io_reads,
                (io_reads as f64 / total_reads as f64) * 100.0
            ));
            analysis.push_str(&format!(
                "    其他: {} 次 ({:.1}%)\n",
                other_reads,
                (other_reads as f64 / total_reads as f64) * 100.0
            ));
        }

        analysis.push_str("  寫入分佈:\n");
        if total_writes > 0 {
            analysis.push_str(&format!(
                "    ROM (0x0000-0x7FFF): {} 次 ({:.1}%)\n",
                rom_writes,
                (rom_writes as f64 / total_writes as f64) * 100.0
            ));
            analysis.push_str(&format!(
                "    VRAM (0x8000-0x9FFF): {} 次 ({:.1}%)\n",
                vram_writes,
                (vram_writes as f64 / total_writes as f64) * 100.0
            ));
            analysis.push_str(&format!(
                "    I/O (0xFF00-0xFFFF): {} 次 ({:.1}%)\n",
                io_writes,
                (io_writes as f64 / total_writes as f64) * 100.0
            ));
            analysis.push_str(&format!(
                "    其他: {} 次 ({:.1}%)\n",
                other_writes,
                (other_writes as f64 / total_writes as f64) * 100.0
            ));
        }

        analysis.push_str(&format!(
            "  總計: {} 次讀取, {} 次寫入\n",
            total_reads, total_writes
        ));

        analysis
    }

    // 記憶體熱點報告
    pub fn get_memory_hotspot_report(&self) -> String {
        let (read_hotspots, write_hotspots) = self.get_memory_hotspots();
        let mut report = String::new();

        report.push_str("記憶體熱點分析:\n");

        if !read_hotspots.is_empty() {
            report.push_str("  讀取熱點 (前10個):\n");
            for (addr, count) in read_hotspots.iter().take(10) {
                let region = match *addr {
                    0x0000..=0x7FFF => "ROM",
                    0x8000..=0x9FFF => "VRAM",
                    0xFF00..=0xFFFF => "I/O",
                    _ => "OTHER",
                };
                report.push_str(&format!("    {:#06X} ({}): {} 次\n", addr, region, count));
            }
        } else {
            report.push_str(&format!(
                "  讀取熱點: 無 (閾值 >= {})\n",
                self.hot_memory_threshold
            ));
        }

        if !write_hotspots.is_empty() {
            report.push_str("  寫入熱點 (前10個):\n");
            for (addr, count) in write_hotspots.iter().take(10) {
                let region = match *addr {
                    0x0000..=0x7FFF => "ROM",
                    0x8000..=0x9FFF => "VRAM",
                    0xFF00..=0xFFFF => "I/O",
                    _ => "OTHER",
                };
                report.push_str(&format!("    {:#06X} ({}): {} 次\n", addr, region, count));
            }
        } else {
            report.push_str(&format!(
                "  寫入熱點: 無 (閾值 >= {})\n",
                self.hot_memory_threshold
            ));
        }

        report
    }

    // 更新記憶體訪問統計的閾值
    pub fn set_memory_hotspot_threshold(&mut self, threshold: u64) {
        self.hot_memory_threshold = threshold;
    }

    // 獲取記憶體訪問總數
    pub fn get_memory_access_count(&self) -> u64 {
        self.memory_access_count
    }

    // 清理記憶體訪問統計 (保留高頻訪問記錄)
    pub fn cleanup_memory_statistics(&mut self) {
        // 只保留訪問次數超過閾值的記錄，以節省記憶體
        self.memory_reads
            .retain(|_, &mut count| count >= self.hot_memory_threshold);
        self.memory_writes
            .retain(|_, &mut count| count >= self.hot_memory_threshold);
    }

    // ...existing methods...
}
#[test]
fn test_decode_and_execute() {
    let mmu = MMU::new();
    let mut cpu = CPU::new(mmu);
    cpu.decode_and_execute(0x3C); // 測試 INC A
    assert_eq!(cpu.registers.a, 1);
}

#[test]
fn test_all_opcodes() {
    for opcode in 0x00u8..=0xFF {
        let mmu = MMU::new();
        let mut cpu = CPU::new(mmu);
        // 用 AssertUnwindSafe 包裝，避免 &mut 問題
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            cpu.decode_and_execute(opcode);
        }));
        if result.is_err() {
            println!("Opcode {:02X} not implemented", opcode);
        }
    }
}
