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
            0x76 => { /* HALT (暫不處理) */ }
            0xAF => {
                // XOR A (A = A ^ A)
                self.registers.a ^= self.registers.a;
                self.registers.set_z_flag(self.registers.a == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(false);
            }
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
            0x18 => {
                // JR n
                let offset = self.fetch() as i8;
                self.registers.pc = ((self.registers.pc as i32) + (offset as i32)) as u16;
            }
            // 新增的指令實現
            0xA7 => {
                // AND A (A = A & A)
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
            0x20 => {
                // JR NZ, n (如果 Z 標誌未設置則相對跳轉)
                let offset = self.fetch() as i8;
                // 為了使 fallback ROM 的循環能正常工作，我們需要檢查 Z 標誌
                // 暫時實現簡單的邏輯：當 A 為 0 時設置 Z 標誌，否則清除
                let zero_flag =
                    self.registers.a == 0 && self.registers.b == 0 && self.registers.c == 0;
                if !zero_flag {
                    self.registers.pc = ((self.registers.pc as i32) + (offset as i32)) as u16;
                }
            }
            0x0B => {
                // DEC BC (BC 暫存器對減一)
                let bc = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                let result = bc.wrapping_sub(1);
                self.registers.b = (result >> 8) as u8;
                self.registers.c = result as u8;
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
            0x44 => {
                // LD B, H (將 H 暫存器的值載入 B)
                self.registers.b = self.registers.h;
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
            0x22 => {
                // LD (HL+), A (將 A 載入到 HL 指向的地址，然後 HL 增一)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.a);

                // HL 增一
                let hl = addr.wrapping_add(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = hl as u8;
            }
            0x32 => {
                // LD (HL-), A (將 A 載入到 HL 指向的地址，然後 HL 減一)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.a);

                // HL 減一
                let hl = addr.wrapping_sub(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = hl as u8;
            }
            0xB1 => {
                // OR C
                self.registers.a |= self.registers.c;
                self.registers.set_z_flag(self.registers.a == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(false);
            }
            0xF3 => {
                // DI (禁用中斷)
                // TODO: 實現中斷禁用
            }
            0x24 => {
                // INC H (H 暫存器加一)
                let old = self.registers.h;
                self.registers.h = self.registers.h.wrapping_add(1);
                self.registers.set_z_flag(self.registers.h == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag((old & 0xF) + 1 > 0xF);
            }
            0x4D => {
                // LD C, L (將 L 暫存器的值載入 C)
                self.registers.c = self.registers.l;
            }
            0x1F => {
                // RRA (Rotate Right Accumulator through Carry)
                let _carry = self.registers.a & 0x01; // 獲取最低位
                self.registers.a = self.registers.a >> 1;
                // TODO: 實現進位標誌邏輯
                // 如果之前有進位標誌，將其設置為最高位
                // self.registers.a |= (old_carry << 7);
                // 設置新的進位標誌
                // self.registers.set_c_flag(carry != 0);
                // self.registers.set_z_flag(false);
                // self.registers.set_n_flag(false);
                // self.registers.set_h_flag(false);
            }
            0xFF => {
                // RST 38H (重置到地址 0x38)
                // 將當前 PC 推入堆疊
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, self.registers.pc as u8);
                // 跳轉到 0x38
                self.registers.pc = 0x38;
            }
            0xCF => {
                // RST 08H (重置到 0x08)
                // 將當前 PC 推入堆疊
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, self.registers.pc as u8);
                // 跳轉到 0x08
                self.registers.pc = 0x08;
            }
            0x11 => {
                // LD DE, nn
                let lo = self.fetch();
                let hi = self.fetch();
                self.registers.d = hi;
                self.registers.e = lo;
            }
            0x19 => {
                // ADD HL, DE
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let de = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                let result = hl.wrapping_add(de);
                self.registers.h = (result >> 8) as u8;
                self.registers.l = result as u8;
                // TODO: 設置標誌位 N=0, H, C
            }
            0xD1 => {
                // POP DE
                let lo = self.mmu.read_byte(self.registers.sp) as u8;
                self.registers.sp = self.registers.sp.wrapping_add(1);
                let hi = self.mmu.read_byte(self.registers.sp) as u8;
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.registers.e = lo;
                self.registers.d = hi;
            }
            0xE5 => {
                // PUSH HL
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu.write_byte(self.registers.sp, (hl >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu.write_byte(self.registers.sp, hl as u8);
            }
            0xC6 => {
                // ADD A, n
                let n = self.fetch();
                let (result, carry) = self.registers.a.overflowing_add(n);
                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(false);
                self.registers
                    .set_h_flag(((self.registers.a & 0xF) + (n & 0xF)) > 0xF);
                self.registers.set_c_flag(carry);
                self.registers.a = result;
            }
            0x30 => {
                // JR NC, n
                let offset = self.fetch() as i8;
                // TODO: 真正檢查 C 標誌
                let c_flag = false;
                if !c_flag {
                    self.registers.pc = ((self.registers.pc as i32) + (offset as i32)) as u16;
                }
            }
            0x67 => {
                // LD H, A
                self.registers.h = self.registers.a;
            }
            0x36 => {
                // LD (HL), n
                let n = self.fetch();
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, n);
            }
            0xE1 => {
                // POP HL
                let lo = self.mmu.read_byte(self.registers.sp) as u8;
                self.registers.sp = self.registers.sp.wrapping_add(1);
                let hi = self.mmu.read_byte(self.registers.sp) as u8;
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.registers.l = lo;
                self.registers.h = hi;
            }
            0x1A => {
                // LD A, (DE)
                let addr = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                self.registers.a = self.mmu.read_byte(addr);
            }
            0x77 => {
                // LD (HL), A
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.a);
            }
            0xCC => {
                // CALL Z, nn
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                let z_flag = (self.registers.f & 0x80) != 0;
                if z_flag {
                    self.registers.sp = self.registers.sp.wrapping_sub(1);
                    self.mmu
                        .write_byte(self.registers.sp, (self.registers.pc >> 8) as u8);
                    self.registers.sp = self.registers.sp.wrapping_sub(1);
                    self.mmu
                        .write_byte(self.registers.sp, self.registers.pc as u8);
                    self.registers.pc = addr;
                }
            }
            0x63 => {
                // LD H, E
                self.registers.h = self.registers.e;
            }
            0x23 => {
                // INC HL
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let result = hl.wrapping_add(1);
                self.registers.h = (result >> 8) as u8;
                self.registers.l = result as u8;
            }
            0x1C => {
                // INC E
                let old = self.registers.e;
                self.registers.e = self.registers.e.wrapping_add(1);
                self.registers.set_z_flag(self.registers.e == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag((old & 0xF) + 1 > 0xF);
            }
            0xD5 => {
                // PUSH DE
                let de = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu.write_byte(self.registers.sp, (de >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu.write_byte(self.registers.sp, de as u8);
            }
            0x7E => {
                // LD A, (HL)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.a = self.mmu.read_byte(addr);
            }
            0xBE => {
                // CP (HL) (比較 A 和 (HL) 的值，設置標誌位)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let value = self.mmu.read_byte(addr);
                let result = self.registers.a.wrapping_sub(value);
                // 設置標誌位 Z, N=1, H, C
                self.registers.set_z_flag(result == 0);
                self.registers.set_n_flag(true);
                self.registers
                    .set_h_flag((self.registers.a & 0x0F) < (value & 0x0F));
                self.registers.set_c_flag(self.registers.a < value);
            }
            0x31 => {
                // LD SP, nn
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.sp = (hi << 8) | lo;
            }
            0xF5 => {
                // PUSH AF
                let af = ((self.registers.a as u16) << 8) | (self.registers.f as u16);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu.write_byte(self.registers.sp, (af >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu.write_byte(self.registers.sp, af as u8);
            }
            0xC5 => {
                // PUSH BC
                let bc = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu.write_byte(self.registers.sp, (bc >> 8) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu.write_byte(self.registers.sp, bc as u8);
            }
            0xC0 => {
                // RET NZ
                // TODO: 真正檢查 Z 標誌
                let z_flag = false;
                if !z_flag {
                    let lo = self.mmu.read_byte(self.registers.sp) as u16;
                    self.registers.sp = self.registers.sp.wrapping_add(1);
                    let hi = self.mmu.read_byte(self.registers.sp) as u16;
                    self.registers.sp = self.registers.sp.wrapping_add(1);
                    self.registers.pc = (hi << 8) | lo;
                }
            }
            0x6F => {
                // LD L, A
                self.registers.l = self.registers.a;
            }
            0xC9 => {
                // RET
                let lo = self.mmu.read_byte(self.registers.sp) as u16;
                self.registers.sp = self.registers.sp.wrapping_add(1);
                let hi = self.mmu.read_byte(self.registers.sp) as u16;
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.registers.pc = (hi << 8) | lo;
            }
            0xC1 => {
                // POP BC
                let lo = self.mmu.read_byte(self.registers.sp) as u8;
                self.registers.sp = self.registers.sp.wrapping_add(1);
                let hi = self.mmu.read_byte(self.registers.sp) as u8;
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.registers.c = lo;
                self.registers.b = hi;
            }
            0xC8 => {
                // RET Z
                // TODO: 真正檢查 Z 標誌
                let z_flag = false;
                if z_flag {
                    let lo = self.mmu.read_byte(self.registers.sp) as u16;
                    self.registers.sp = self.registers.sp.wrapping_add(1);
                    let hi = self.mmu.read_byte(self.registers.sp) as u16;
                    self.registers.sp = self.registers.sp.wrapping_add(1);
                    self.registers.pc = (hi << 8) | lo;
                }
            }
            0xF1 => {
                // POP AF
                let lo = self.mmu.read_byte(self.registers.sp) as u8;
                self.registers.sp = self.registers.sp.wrapping_add(1);
                let hi = self.mmu.read_byte(self.registers.sp) as u8;
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.registers.f = lo & 0xF0; // F的低4位必須為0
                self.registers.a = hi;
            }
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
            0x5F => {
                // LD E, A
                self.registers.e = self.registers.a;
            }
            0x5E => {
                // LD E, (HL)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.e = self.mmu.read_byte(addr);
            }
            0x56 => {
                // LD D, (HL)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.d = self.mmu.read_byte(addr);
            }
            0xE2 => {
                // LD (C), A  (寫入A到(0xFF00+C))
                let addr = 0xFF00 + self.registers.c as u16;
                self.mmu.write_byte(addr, self.registers.a);
            }
            0x69 => {
                // LD L, C
                self.registers.l = self.registers.c;
            }
            0x5D => {
                // LD E, L
                self.registers.e = self.registers.l;
            }
            0x0C => {
                // INC C (C 暫存器加一)
                let old = self.registers.c;
                self.registers.c = self.registers.c.wrapping_add(1);
                self.registers.set_z_flag(self.registers.c == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag((old & 0xF) + 1 > 0xF);
            }
            0x2A => {
                // LD A, (HL+) (A = (HL), HL++)
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.a = self.mmu.read_byte(addr);
                let hl = addr.wrapping_add(1);
                self.registers.h = (hl >> 8) as u8;
                self.registers.l = hl as u8;
            }
            0xE6 => {
                // AND n (A = A & n)
                let n = self.fetch();
                self.registers.a &= n;
                self.registers.set_z_flag(self.registers.a == 0);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(true);
                self.registers.set_c_flag(false);
            }
            0xCB => {
                // CB 前綴指令集 (Bit操作/Shift/Rotate等)
                let cb_opcode = self.fetch();
                match cb_opcode {
                    0x11 => {
                        // RL C (C = (C << 1) | Carry)
                        let old_carry = (self.registers.f & 0x10) != 0;
                        let c = self.registers.c;
                        let new_c = (c << 1) | if old_carry { 1 } else { 0 };
                        self.registers.c = new_c;
                        self.registers.set_z_flag(self.registers.c == 0);
                        self.registers.set_n_flag(false);
                        self.registers.set_h_flag(false);
                        self.registers.set_c_flag((c & 0x80) != 0);
                    }
                    0x87 => {
                        // RES 0, A (將 A 的 bit 0 清 0)
                        self.registers.a &= !0x01;
                        // RES 不影響標誌位
                    }
                    _ => println!("CB Opcode {:02X} not implemented", cb_opcode),
                }
            }
            0x13 => {
                // INC DE (DE 暫存器對增一)
                let de = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                let result = de.wrapping_add(1);
                self.registers.d = (result >> 8) as u8;
                self.registers.e = result as u8;
            }
            0x47 => {
                // LD B, A
                self.registers.b = self.registers.a;
            }
            0x07 => {
                // RLCA (Rotate A left, old bit 7 to Carry and bit 0)
                let a = self.registers.a;
                let carry = (a & 0x80) != 0;
                self.registers.a = (a << 1) | if carry { 1 } else { 0 };
                self.registers.set_z_flag(false);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(carry);
            }
            0x09 => {
                // ADD HL, BC
                let hl = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let bc = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                let result = hl.wrapping_add(bc);
                // H 標誌：如果低 12 位溢出
                self.registers
                    .set_h_flag(((hl & 0x0FFF) + (bc & 0x0FFF)) > 0x0FFF);
                // C 標誌：如果 16 位溢出
                self.registers.set_c_flag(result < hl);
                self.registers.set_n_flag(false);
                // Z 標誌不變
                self.registers.h = (result >> 8) as u8;
                self.registers.l = result as u8;
            }
            0x12 => {
                // LD (DE), A (將 A 的值寫入 DE 指向的記憶體)
                let addr = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                self.mmu.write_byte(addr, self.registers.a);
            }
            0xFB => {
                // EI (啟用中斷，暫時可不實作副作用)
                // TODO: 實現中斷啟用
            }
            _ => println!("Opcode {:02X} not implemented", opcode),
        }
    }
}
