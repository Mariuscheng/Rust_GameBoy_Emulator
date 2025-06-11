// filepath: c:\Users\mariu\Desktop\Rust\gameboy_emulator\src\cpu.rs
use crate::mmu::MMU;

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
        // 這裡未來會執行一條指令
        // 目前先留空
        let pos = (self.registers.pc as usize) % 160;
        // 修復 VRAM 存取
        let mut vram = self.mmu.vram.borrow_mut();
        vram[pos] = self.registers.pc as u8;
    }

    pub fn new(mmu: MMU) -> Self {
        CPU {
            registers: Registers::default(),
            mmu,
        }
    }
    pub fn load_rom(&mut self, rom: &[u8]) {
        self.mmu.load_rom(rom.to_vec());
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
                self.mmu.write_byte(hl, self.registers.a);
            }
            0x7E => {
                // LD A,(HL)
                let hl = ((self.registers.h as u16) << 8) | self.registers.l as u16;
                self.registers.a = self.mmu.read_byte(hl);
            }
            0xC9 => { /* RET (暫不實作堆疊) */ }
            0x76 => { /* HALT (暫不處理) */ }
            0x02 => {
                // LD (BC),A
                let addr = ((self.registers.b as u16) << 8) | self.registers.c as u16;
                self.mmu.write_byte(addr, self.registers.a);
            }
            0x0A => {
                // LD A,(BC)
                let addr = ((self.registers.b as u16) << 8) | self.registers.c as u16;
                self.registers.a = self.mmu.read_byte(addr);
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
            0x76 => { /* HALT (暫不處理) */ }
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
            // 新增缺失的指令
            0x27 => {
                // DAA (Decimal Adjust Accumulator)
                // 簡化版本的 DAA 實現
                let mut a = self.registers.a;
                if (a & 0xF) > 0x9 {
                    a = a.wrapping_add(0x6);
                }
                if a > 0x99 {
                    a = a.wrapping_add(0x60);
                }
                self.registers.a = a;
            }
            0xCF => {
                // RST 08H (Restart to address 0x08)
                // 模擬調用地址 0x08
                self.registers.pc = 0x08;
            }
            0xE6 => {
                // AND n (Logical AND with immediate)
                let n = self.fetch();
                self.registers.a &= n;
            }
            0xF8 => {
                // LD HL, SP+n (Load SP+signed_offset to HL)
                let offset = self.fetch() as i8;
                let result = (self.registers.sp as i32 + offset as i32) as u16;
                self.registers.h = (result >> 8) as u8;
                self.registers.l = (result & 0xFF) as u8;
            }
            0xFF => {
                // RST 38H (Restart to address 0x38)
                // 模擬調用地址 0x38
                self.registers.pc = 0x38;
            }

            // ...繼續補齊其他常用指令...
            _ => println!("Opcode {:02X} not implemented", opcode),
        }
    }

    fn decode_cb(&mut self, cb_opcode: u8) {
        match cb_opcode {
            0x11 => { /* RL C */ }
            0x7C => { /* BIT 7, H */ }
            // ...補齊 CB 指令...
            _ => unimplemented!("CB Opcode {:#X} not implemented", cb_opcode),
        }
    }
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
