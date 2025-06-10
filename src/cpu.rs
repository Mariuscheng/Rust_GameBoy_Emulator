// Game Boy CPU 模擬器 - 修復版本
use crate::mmu::MMU;

#[derive(Default)]
pub struct Registers {
    pub a: u8,
    pub b: u8,
    pub c: u8,
    pub d: u8,
    pub e: u8,
    pub h: u8,
    pub l: u8,
    pub pc: u16,
    pub sp: u16,
    pub f: u8, // 標誌位暫存器
}

impl Registers {
    // 標誌位操作
    pub fn set_z_flag(&mut self, value: bool) {
        if value {
            self.f |= 0x80;
        } else {
            self.f &= !0x80;
        }
    }
    pub fn set_n_flag(&mut self, value: bool) {
        if value {
            self.f |= 0x40;
        } else {
            self.f &= !0x40;
        }
    }
    pub fn set_h_flag(&mut self, value: bool) {
        if value {
            self.f |= 0x20;
        } else {
            self.f &= !0x20;
        }
    }
    pub fn set_c_flag(&mut self, value: bool) {
        if value {
            self.f |= 0x10;
        } else {
            self.f &= !0x10;
        }
    }
}

pub struct CPU {
    pub registers: Registers,
    pub mmu: MMU,
    instruction_count: u64,
}

impl CPU {
    pub fn new(mmu: MMU) -> Self {
        let mut registers = Registers::default();
        registers.pc = 0x0100; // Game Boy CPU 应该从 0x0100 开始执行
        registers.sp = 0xFFFE; // 初始化堆栈指针

        CPU {
            registers,
            mmu,
            instruction_count: 0,
        }
    }
    pub fn step(&mut self) {
        self.execute();
        // 移除了不合理的 VRAM 寫入，因為這會破壞 VRAM 數據
        // let pos = (self.registers.pc as usize) % 0x2000;
        // self.mmu.vram.borrow_mut()[pos] = self.registers.pc as u8;
    }
    pub fn load_rom(&mut self, rom: &[u8]) {
        self.mmu.load_rom(rom.to_vec());
    }

    pub fn execute(&mut self) {
        let opcode = self.fetch();
        self.decode_and_execute(opcode);
        self.instruction_count += 1;
    }

    fn fetch(&mut self) -> u8 {
        let opcode = self.mmu.read_byte(self.registers.pc);
        self.registers.pc += 1;
        opcode
    }

    fn decode_and_execute(&mut self, opcode: u8) {
        // 添加調試輸出來追蹤指令執行
        if self.instruction_count < 20 {
            println!(
                "執行指令: PC=0x{:04X}, opcode=0x{:02X}",
                self.registers.pc.wrapping_sub(1),
                opcode
            );
        }

        match opcode {
            // 移除重複分支：0x44、0x76、0x78~0x7F、0x20（保留 0x40~0x7F 區塊與靠近 0x18 的 0x20）
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
            0x3E => {
                let n = self.fetch();
                self.registers.a = n;
            } // LD A, n
            0xAF => {
                self.registers.a ^= self.registers.a;
                self.registers.set_z_flag(self.registers.a == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(false);
            }
            0xC3 => {
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0x18 => {
                let offset = self.fetch() as i8;
                self.registers.pc = ((self.registers.pc as i32) + (offset as i32)) as u16;
            }
            // 新增的指令實現
            0xA7 => {
                self.registers.a &= self.registers.a;
                self.registers.set_z_flag(self.registers.a == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(true);
                self.registers.set_c_flag(false);
            }
            0x20 => {
                // JR NZ, n (如果 Z 標誌未設置則相對跳轉)
                let offset = self.fetch() as i8;
                let z_flag = (self.registers.f & 0x80) != 0;
                if !z_flag {
                    self.registers.pc = ((self.registers.pc as i32) + (offset as i32)) as u16;
                }
            }
            0x28 => {
                // JR Z, n (如果 Z 標誌設置則相對跳轉)
                let offset = self.fetch() as i8;
                let z_flag = (self.registers.f & 0x80) != 0;
                if z_flag {
                    self.registers.pc = ((self.registers.pc as i32) + (offset as i32)) as u16;
                }
            }
            0xFA => {
                // LD A, (nn) (從記憶體地址 nn 載入到 A)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                self.registers.a = self.mmu.read_byte(addr);
            }
            0xE0 => {
                // LDH (n), A (將 A 儲存到 0xFF00+n)
                let n = self.fetch();
                let addr = 0xFF00 + n as u16;
                self.mmu.write_byte(addr, self.registers.a);
            }
            0xE2 => {
                // LD (0xFF00+C), A
                let addr = 0xFF00 + self.registers.c as u16;
                self.mmu.write_byte(addr, self.registers.a);
            }
            0x85 => {
                // ADD A, L (A = A + L)
                let (result, carry) = self.registers.a.overflowing_add(self.registers.l);
                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(false);
                self.registers
                    .set_h_flag(((self.registers.a & 0xF) + (self.registers.l & 0xF)) > 0xF);
                self.registers.set_c_flag(carry);
                self.registers.a = result;
            }
            0x1D => {
                // DEC E (E 暫存器減一)
                self.registers.e = self.registers.e.wrapping_sub(1);
            }
            0xDA => {
                // JP C, nn (如果 C 標誌設置則跳轉)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                let c_flag = (self.registers.f & 0x10) != 0;
                if c_flag {
                    self.registers.pc = addr;
                }
            }
            0xFE => {
                // CP n (比較 A 和立即數 n)
                let n = self.fetch();
                let result = self.registers.a.wrapping_sub(n);
                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(true);
                self.registers
                    .set_h_flag((self.registers.a & 0xF) < (n & 0xF));
                self.registers.set_c_flag(self.registers.a < n);
            }
            0x03 => {
                // INC BC (BC 暫存器對增一)
                let bc = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                let result = bc.wrapping_add(1);
                self.registers.b = (result >> 8) as u8;
                self.registers.c = result as u8;
            }
            0x0B => {
                // DEC BC (BC 暫存器對減一)
                let bc = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                let result = bc.wrapping_sub(1);
                self.registers.b = (result >> 8) as u8;
                self.registers.c = result as u8;
            }
            0x0C => {
                // INC C (C 暫存器加一)
                self.registers.c = self.registers.c.wrapping_add(1);
            }
            0xEA => {
                // LD (nn), A (將 A 儲存到記憶體地址 nn)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                self.mmu.write_byte(addr, self.registers.a);
            }
            0xCD => {
                // CALL nn (呼叫子程式)
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;

                // 將返回地址推入堆疊
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, self.registers.pc as u8);

                // 跳轉到目標地址
                self.registers.pc = addr;
            }
            0xF0 => {
                // LDH A, (n) (從 0xFF00+n 載入到 A)
                let n = self.fetch();
                let addr = 0xFF00 + n as u16;
                self.registers.a = self.mmu.read_byte(addr);
            }
            // 新增 fallback ROM 需要的指令
            0x21 => {
                // LD HL, nn (載入 16 位立即值到 HL)
                let lo = self.fetch();
                let hi = self.fetch();
                self.registers.h = hi;
                self.registers.l = lo;
            }
            0x01 => {
                // LD BC, nn (載入 16 位立即值到 BC)
                let lo = self.fetch();
                let hi = self.fetch();
                self.registers.b = hi;
                self.registers.c = lo;
            }
            0x02 => {
                // LD (BC), A
                let addr = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                self.mmu.write_byte(addr, self.registers.a);
            }
            0x0A => {
                // LD A, (BC)
                let addr = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                self.registers.a = self.mmu.read_byte(addr);
            }
            0x12 => {
                // LD (DE), A
                let addr = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                self.mmu.write_byte(addr, self.registers.a);
            }
            0x1A => {
                // LD A, (DE)
                let addr = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                self.registers.a = self.mmu.read_byte(addr);
            }
            0x22 => {
                // LD (HL+), A
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.a);
                let hl = addr.wrapping_add(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = hl as u8;
            }
            0x2A => {
                // LD A, (HL+)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.a = self.mmu.read_byte(addr);
                let hl = addr.wrapping_add(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = hl as u8;
            }
            0x32 => {
                // LD (HL-), A
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.a);
                let hl = addr.wrapping_sub(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = hl as u8;
            }
            0x3A => {
                // LD A, (HL-)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.a = self.mmu.read_byte(addr);
                let hl = addr.wrapping_sub(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = hl as u8;
            }
            // 8-bit LD r,r' (0x40~0x7F) 合併去重，確保只出現一次
            0x40 => { /* LD B,B */ }
            0x41 => {
                self.registers.b = self.registers.c;
            }
            0x42 => {
                self.registers.b = self.registers.d;
            }
            0x43 => {
                self.registers.b = self.registers.e;
            }
            0x44 => {
                self.registers.b = self.registers.h;
            }
            0x45 => {
                self.registers.b = self.registers.l;
            }
            0x46 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.b = self.mmu.read_byte(addr);
            }
            0x47 => {
                self.registers.b = self.registers.a;
            }
            0x48 => {
                self.registers.c = self.registers.b;
            }
            0x49 => { /* LD C,C */ }
            0x4A => {
                self.registers.c = self.registers.d;
            }
            0x4B => {
                self.registers.c = self.registers.e;
            }
            0x4C => {
                self.registers.c = self.registers.h;
            }
            0x4D => {
                self.registers.c = self.registers.l;
            }
            0x4E => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.c = self.mmu.read_byte(addr);
            }
            0x4F => {
                self.registers.c = self.registers.a;
            }
            0x50 => {
                self.registers.d = self.registers.b;
            }
            0x51 => {
                self.registers.d = self.registers.c;
            }
            0x52 => { /* LD D,D */ }
            0x53 => {
                self.registers.d = self.registers.e;
            }
            0x54 => {
                self.registers.d = self.registers.h;
            }
            0x55 => {
                self.registers.d = self.registers.l;
            }
            0x56 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.d = self.mmu.read_byte(addr);
            }
            0x57 => {
                self.registers.d = self.registers.a;
            }
            0x58 => {
                self.registers.e = self.registers.b;
            }
            0x59 => {
                self.registers.e = self.registers.c;
            }
            0x5A => {
                self.registers.e = self.registers.d;
            }
            0x5B => { /* LD E,E */ }
            0x5C => {
                self.registers.e = self.registers.h;
            }
            0x5D => {
                self.registers.e = self.registers.l;
            }
            0x5E => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.e = self.mmu.read_byte(addr);
            }
            0x5F => {
                self.registers.e = self.registers.a;
            }
            0x60 => {
                self.registers.h = self.registers.b;
            }
            0x61 => {
                self.registers.h = self.registers.c;
            }
            0x62 => {
                self.registers.h = self.registers.d;
            }
            0x63 => {
                self.registers.h = self.registers.e;
            }
            0x64 => { /* LD H,H */ }
            0x65 => {
                self.registers.h = self.registers.l;
            }
            0x66 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.h = self.mmu.read_byte(addr);
            }
            0x67 => {
                self.registers.h = self.registers.a;
            }
            0x68 => {
                self.registers.l = self.registers.b;
            }
            0x69 => {
                self.registers.l = self.registers.c;
            }
            0x6A => {
                self.registers.l = self.registers.d;
            }
            0x6B => {
                self.registers.l = self.registers.e;
            }
            0x6C => {
                self.registers.l = self.registers.h;
            }
            0x6D => { /* LD L,L */ }
            0x6E => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.l = self.mmu.read_byte(addr);
            }
            0x6F => {
                self.registers.l = self.registers.a;
            }
            0x70 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.b);
            }
            0x71 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.c);
            }
            0x72 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.d);
            }
            0x73 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.e);
            }
            0x74 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.h);
            }
            0x75 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.l);
            }
            0x77 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.a);
            }
            // 0x78~0x7F 的 LD 指令
            0x78 => {
                self.registers.a = self.registers.b;
            }
            0x79 => {
                self.registers.a = self.registers.c;
            }
            0x7A => {
                self.registers.a = self.registers.d;
            }
            0x7B => {
                self.registers.a = self.registers.e;
            }
            0x7C => {
                self.registers.a = self.registers.h;
            }
            0x7D => {
                self.registers.a = self.registers.l;
            }
            0x7E => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.a = self.mmu.read_byte(addr);
            }
            0x7F => { /* LD A, A */ }
            // ALU 指令
            0x87 => {
                // ADD A, A
                let (result, carry) = self.registers.a.overflowing_add(self.registers.a);
                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(false);
                self.registers
                    .set_h_flag(((self.registers.a & 0xF) + (self.registers.a & 0xF)) > 0xF);
                self.registers.set_c_flag(carry);
                self.registers.a = result;
            }
            0xB0 => {
                // OR B
                self.registers.a |= self.registers.b;
                self.registers.set_z_flag(self.registers.a == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(false);
            }
            0xB1 => {
                // OR C
                self.registers.a |= self.registers.c;
                self.registers.set_z_flag(self.registers.a == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(false);
            }
            0xE6 => {
                // AND n
                let n = self.fetch();
                self.registers.a &= n;
                self.registers.set_z_flag(self.registers.a == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(true);
                self.registers.set_c_flag(false);
            }
            0x13 => {
                // INC DE
                let de = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                let result = de.wrapping_add(1);
                self.registers.d = (result >> 8) as u8;
                self.registers.e = result as u8;
            }
            0x09 => {
                // ADD HL, BC
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let bc = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                let (result, carry) = hl.overflowing_add(bc);
                self.registers.h = (result >> 8) as u8;
                self.registers.l = result as u8;
                self.registers.set_n_flag(false);
                self.registers
                    .set_h_flag(((hl & 0xFFF) + (bc & 0xFFF)) > 0xFFF);
                self.registers.set_c_flag(carry);
            }
            0xF5 => {
                // PUSH AF
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu.write_byte(self.registers.sp, self.registers.a);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu.write_byte(self.registers.sp, self.registers.f);
            }
            0xF1 => {
                // POP AF
                self.registers.f = self.mmu.read_byte(self.registers.sp);
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.registers.a = self.mmu.read_byte(self.registers.sp);
                self.registers.sp = self.registers.sp.wrapping_add(1);
            }
            0x3F => {
                // CCF
                let c = (self.registers.f & 0x10) != 0;
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(!c);
            }
            0x07 => {
                // RLCA
                let c = (self.registers.a & 0x80) != 0;
                self.registers.a = self.registers.a.rotate_left(1);
                self.registers.set_z_flag(false);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(c);
            }
            0xC9 => {
                // RET
                let lo = self.mmu.read_byte(self.registers.sp) as u16;
                self.registers.sp = self.registers.sp.wrapping_add(1);
                let hi = self.mmu.read_byte(self.registers.sp) as u16;
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.registers.pc = (hi << 8) | lo;
            }
            0xCB => {
                // CB 前綴指令
                let cb_opcode = self.fetch();
                match cb_opcode {
                    0x11 => {
                        // RL C
                        let c = self.registers.c;
                        let carry = (self.registers.f & 0x10) != 0;
                        let new_c = (c << 1) | if carry { 1 } else { 0 };
                        self.registers.c = new_c;
                        self.registers.set_z_flag(new_c == 0);
                        self.registers.set_n_flag(false);
                        self.registers.set_h_flag(false);
                        self.registers.set_c_flag((c & 0x80) != 0);
                    }
                    0x87 => {
                        // RES 0, A
                        self.registers.a &= !(1 << 0);
                    }
                    _ => {
                        println!("未處理的 CB 指令: 0x{:02X}", cb_opcode);
                    }
                }
            }
            0x31 => {
                // LD SP, nn
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.sp = (hi << 8) | lo;
            }
            0xCF => {
                // RST 08H
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, self.registers.pc as u8);
                self.registers.pc = 0x0008;
            }
            0xFF => {
                // RST 38H
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, self.registers.pc as u8);
                self.registers.pc = 0x0038;
            }
            _ => {
                println!("未處理的指令: 0x{:02X}", opcode);
            }
        } // match 結尾
    } // decode_and_execute 結尾
}
