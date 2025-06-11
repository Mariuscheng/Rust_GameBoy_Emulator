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
    // Implements ADD A, value (8-bit) and sets flags accordingly
    fn alu_add(&mut self, value: u8) {
        let a = self.registers.a;
        let result = a.wrapping_add(value);
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(((a & 0xF) + (value & 0xF)) > 0xF);
        self.registers.set_c_flag((a as u16 + value as u16) > 0xFF);
        self.registers.a = result;
    }

    // Implements ADC A, value (8-bit) and sets flags accordingly
    fn alu_adc(&mut self, value: u8) {
        let a = self.registers.a;
        let carry = if (self.registers.f & 0x10) != 0 { 1 } else { 0 };
        let result = a.wrapping_add(value).wrapping_add(carry);
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers
            .set_h_flag(((a & 0xF) + (value & 0xF) + carry) > 0xF);
        self.registers
            .set_c_flag((a as u16 + value as u16 + carry as u16) > 0xFF);
        self.registers.a = result;
    }

    // Implements SUB A, value (8-bit) and sets flags accordingly
    fn alu_sub(&mut self, value: u8) {
        let a = self.registers.a;
        let result = a.wrapping_sub(value);
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(true);
        self.registers.set_h_flag((a & 0xF) < (value & 0xF));
        self.registers.set_c_flag(a < value);
        self.registers.a = result;
    }

    // Implements SBC A, value (8-bit) and sets flags accordingly
    fn alu_sbc(&mut self, value: u8) {
        let a = self.registers.a;
        let carry = if (self.registers.f & 0x10) != 0 { 1 } else { 0 };
        let result = a.wrapping_sub(value).wrapping_sub(carry);
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(true);
        self.registers
            .set_h_flag((a & 0xF) < ((value & 0xF) + carry));
        self.registers
            .set_c_flag((a as u16) < (value as u16 + carry as u16));
        self.registers.a = result;
    }

    // Implements AND A, value (8-bit) and sets flags accordingly
    fn alu_and(&mut self, value: u8) {
        let result = self.registers.a & value;
        self.registers.a = result;
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(true);
        self.registers.set_c_flag(false);
    }

    // Implements OR A, value (8-bit) and sets flags accordingly
    fn alu_or(&mut self, value: u8) {
        let result = self.registers.a | value;
        self.registers.a = result;
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(false);
    }

    // Implements XOR A, value (8-bit) and sets flags accordingly
    fn alu_xor(&mut self, value: u8) {
        let result = self.registers.a ^ value;
        self.registers.a = result;
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(false);
    }

    // Implements CP A, value (8-bit) and sets flags accordingly
    fn alu_cp(&mut self, value: u8) {
        let a = self.registers.a;
        let result = a.wrapping_sub(value);
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(true);
        self.registers.set_h_flag((a & 0xF) < (value & 0xF));
        self.registers.set_c_flag(a < value);
    }

    // RLC: Rotate Left Circular
    fn rlc(&mut self, value: u8) -> u8 {
        let result = (value << 1) | (value >> 7);
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag((value & 0x80) != 0);
        result
    }

    // RRC: Rotate Right Circular
    fn rrc(&mut self, value: u8) -> u8 {
        let result = (value >> 1) | ((value & 0x01) << 7);
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag((value & 0x01) != 0);
        result
    }

    // RL: Rotate Left through Carry
    fn rl(&mut self, value: u8) -> u8 {
        let c = if (self.registers.f & 0x10) != 0 { 1 } else { 0 };
        let result = (value << 1) | c;
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag((value & 0x80) != 0);
        result
    }

    // RR: Rotate Right through Carry
    fn rr(&mut self, value: u8) -> u8 {
        let c = if (self.registers.f & 0x10) != 0 {
            0x80
        } else {
            0
        };
        let result = (value >> 1) | c;
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag((value & 0x01) != 0);
        result
    }

    // SLA: Shift Left Arithmetic
    fn sla(&mut self, value: u8) -> u8 {
        let result = value << 1;
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag((value & 0x80) != 0);
        result
    }

    // SRA: Shift Right Arithmetic
    fn sra(&mut self, value: u8) -> u8 {
        let result = (value >> 1) | (value & 0x80);
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag((value & 0x01) != 0);
        result
    }

    // SRL: Shift Right Logical
    fn srl(&mut self, value: u8) -> u8 {
        let result = value >> 1;
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag((value & 0x01) != 0);
        result
    }

    // SWAP: Swap nibbles
    fn swap(&mut self, value: u8) -> u8 {
        let result = (value >> 4) | (value << 4);
        self.registers.set_z_flag(result == 0);
        self.registers.set_n_flag(false);
        self.registers.set_h_flag(false);
        self.registers.set_c_flag(false);
        result
    }

    // --- ALU 運算相關方法 ---

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
            // 新增指令: LD BC,d16
            0x01 => {
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.c = lo as u8;
                self.registers.b = hi as u8;
            } // LD BC,d16
            // 新增指令: LD (BC),A
            0x02 => {
                let addr = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                self.mmu.write_byte(addr, self.registers.a);
            } // LD (BC),A
            // 新增指令: DEC BC
            0x0B => {
                let bc = ((self.registers.b as u16) << 8) | (self.registers.c as u16);
                let result = bc.wrapping_sub(1);
                self.registers.b = (result >> 8) as u8;
                self.registers.c = result as u8;
            } // DEC BC
            // 新增指令: INC DE
            0x13 => {
                let de = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                let result = de.wrapping_add(1);
                self.registers.d = (result >> 8) as u8;
                self.registers.e = result as u8;
            } // INC DE
            // 新增指令: LD (DE),A
            0x12 => {
                let addr = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                self.mmu.write_byte(addr, self.registers.a);
            } // LD (DE),A
            // 新增指令: LD HL,d16
            0x21 => {
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.l = lo as u8;
                self.registers.h = hi as u8;
            } // LD HL,d16
            // 新增指令: LD (HL+),A
            0x22 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.a);
                let result = addr.wrapping_add(1);
                self.registers.h = (result >> 8) as u8;
                self.registers.l = result as u8;
            } // LD (HL+),A
            // 新增指令: LD A,(HL+)
            0x2A => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.a = self.mmu.read_byte(addr);
                let result = addr.wrapping_add(1);
                self.registers.h = (result >> 8) as u8;
                self.registers.l = result as u8;
            } // LD A,(HL+)
            // 新增指令: LD (HL-),A
            0x32 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.a);
                let result = addr.wrapping_sub(1);
                self.registers.h = (result >> 8) as u8;
                self.registers.l = result as u8;
            } // LD (HL-),A
            // 新增指令: LD SP,d16
            0x31 => {
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.sp = (hi << 8) | lo;
            } // LD SP,d16
            // 新增指令: INC SP
            0x33 => {
                self.registers.sp = self.registers.sp.wrapping_add(1);
            } // INC SP
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
            0x0C => {
                self.registers.c = self.registers.c.wrapping_add(1);
            } // INC C
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
            } // LD A, n            // XOR A - 已經在後面的代碼區塊中實現了
            0xC3 => {
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.pc = (hi << 8) | lo;
            }
            0x18 => {
                let offset = self.fetch() as i8;
                self.registers.pc = ((self.registers.pc as i32) + (offset as i32)) as u16;
            }
            // 新增的指令實現            // 0xA7 (AND A) - 已經在後面的代碼區塊中實現
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
            0xF0 => {
                // LDH A, (n) (從 0xFF00+n 讀取到 A)
                let n = self.fetch();
                let addr = 0xFF00 + n as u16;
                self.registers.a = self.mmu.read_byte(addr);
            }
            0xE2 => {
                // LD (0xFF00+C), A
                let addr = 0xFF00 + self.registers.c as u16;
                self.mmu.write_byte(addr, self.registers.a);
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
            // 新增指令: LD DE,d16
            0x11 => {
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                self.registers.e = lo as u8;
                self.registers.d = hi as u8;
            } // LD DE,d16
            // 新增指令: LD A,(DE)
            0x1A => {
                let addr = ((self.registers.d as u16) << 8) | (self.registers.e as u16);
                self.registers.a = self.mmu.read_byte(addr);
            } // LD A,(DE)
            // 新增指令: RLA
            0x17 => {
                let c = if (self.registers.f & 0x10) != 0 { 1 } else { 0 };
                let carry = (self.registers.a & 0x80) != 0;
                self.registers.a = (self.registers.a << 1) | c;
                self.registers.set_z_flag(false);
                self.registers.set_n_flag(false);
                self.registers.set_h_flag(false);
                self.registers.set_c_flag(carry);
            } // RLA
            // 新增指令: AND d8
            0xE6 => {
                let n = self.fetch();
                self.alu_and(n);
            } // AND d8
            // 新增指令: OR d8
            0xF6 => {
                let n = self.fetch();
                self.alu_or(n);
            } // OR d8
            // 新增指令: CP d8
            0xFE => {
                let n = self.fetch();
                self.alu_cp(n);
            } // CP d8
            // 新增指令: EI (Enable Interrupts)
            0xFB => {
                // 在實際的 Game Boy 中，EI 指令會在下一條指令執行後才啟用中斷
                // 為了簡化，我們暫時假定立即啟用了中斷
                // 待實現完整中斷處理系統時再精確模擬
                // 這裡只是一個空操作
            } // EI
            // 8-bit LD r, r' 指令 (0x40~0x7F)
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
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.b = self.mmu.read_byte(addr);
            } // LD B,(HL)
            0x47 => {
                self.registers.b = self.registers.a;
            } // LD B,A
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
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.c = self.mmu.read_byte(addr);
            } // LD C,(HL)
            0x4F => {
                self.registers.c = self.registers.a;
            } // LD C,A
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
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.d = self.mmu.read_byte(addr);
            } // LD D,(HL)
            0x57 => {
                self.registers.d = self.registers.a;
            } // LD D,A
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
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.e = self.mmu.read_byte(addr);
            } // LD E,(HL)
            0x5F => {
                self.registers.e = self.registers.a;
            } // LD E,A
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
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.h = self.mmu.read_byte(addr);
            } // LD H,(HL)
            0x67 => {
                self.registers.h = self.registers.a;
            } // LD H,A
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
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.l = self.mmu.read_byte(addr);
            } // LD L,(HL)
            0x6F => {
                self.registers.l = self.registers.a;
            } // LD L,A
            0x70 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.b);
            } // LD (HL),B
            0x71 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.c);
            } // LD (HL),C
            0x72 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.d);
            } // LD (HL),D
            0x73 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.e);
            } // LD (HL),E
            0x74 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.h);
            } // LD (HL),H
            0x75 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.l);
            } // LD (HL),L
            0x76 => { /* HALT */ } // HALT
            0x77 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.mmu.write_byte(addr, self.registers.a);
            } // LD (HL),A
            0x78 => {
                self.registers.a = self.registers.b;
            } // LD A,B
            0x79 => {
                self.registers.a = self.registers.c;
            } // LD A,C
            0x7A => {
                self.registers.a = self.registers.d;
            } // LD A,D
            0x7B => {
                self.registers.a = self.registers.e;
            } // LD A,E
            0x7C => {
                self.registers.a = self.registers.h;
            } // LD A,H
            0x7D => {
                self.registers.a = self.registers.l;
            } // LD A,L
            0x7E => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                self.registers.a = self.mmu.read_byte(addr);
            } // LD A,(HL)
            0x7F => {
                self.registers.a = self.registers.a;
            } // LD A,A
            // 8-bit ALU 指令 (ADD/ADC/SUB/SBC/AND/OR/XOR/CP)
            0x80 => {
                self.alu_add(self.registers.b);
            } // ADD A,B
            0x81 => {
                self.alu_add(self.registers.c);
            } // ADD A,C
            0x82 => {
                self.alu_add(self.registers.d);
            } // ADD A,D
            0x83 => {
                self.alu_add(self.registers.e);
            } // ADD A,E
            0x84 => {
                self.alu_add(self.registers.h);
            } // ADD A,H
            0x85 => {
                self.alu_add(self.registers.l);
            } // ADD A,L
            0x86 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let v = self.mmu.read_byte(addr);
                self.alu_add(v);
            } // ADD A,(HL)
            0x87 => {
                self.alu_add(self.registers.a);
            } // ADD A,A
            0x88 => {
                self.alu_adc(self.registers.b);
            } // ADC A,B
            0x89 => {
                self.alu_adc(self.registers.c);
            } // ADC A,C
            0x8A => {
                self.alu_adc(self.registers.d);
            } // ADC A,D
            0x8B => {
                self.alu_adc(self.registers.e);
            } // ADC A,E
            0x8C => {
                self.alu_adc(self.registers.h);
            } // ADC A,H
            0x8D => {
                self.alu_adc(self.registers.l);
            } // ADC A,L
            0x8E => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let v = self.mmu.read_byte(addr);
                self.alu_adc(v);
            } // ADC A,(HL)
            0x8F => {
                self.alu_adc(self.registers.a);
            } // ADC A,A
            0x90 => {
                self.alu_sub(self.registers.b);
            } // SUB B
            0x91 => {
                self.alu_sub(self.registers.c);
            } // SUB C
            0x92 => {
                self.alu_sub(self.registers.d);
            } // SUB D
            0x93 => {
                self.alu_sub(self.registers.e);
            } // SUB E
            0x94 => {
                self.alu_sub(self.registers.h);
            } // SUB H
            0x95 => {
                self.alu_sub(self.registers.l);
            } // SUB L
            0x96 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let v = self.mmu.read_byte(addr);
                self.alu_sub(v);
            } // SUB (HL)
            0x97 => {
                self.alu_sub(self.registers.a);
            } // SUB A
            0x98 => {
                self.alu_sbc(self.registers.b);
            } // SBC A,B
            0x99 => {
                self.alu_sbc(self.registers.c);
            } // SBC A,C
            0x9A => {
                self.alu_sbc(self.registers.d);
            } // SBC A,D
            0x9B => {
                self.alu_sbc(self.registers.e);
            } // SBC A,E
            0x9C => {
                self.alu_sbc(self.registers.h);
            } // SBC A,H
            0x9D => {
                self.alu_sbc(self.registers.l);
            } // SBC A,L
            0x9E => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let v = self.mmu.read_byte(addr);
                self.alu_sbc(v);
            } // SBC A,(HL)
            0x9F => {
                self.alu_sbc(self.registers.a);
            } // SBC A,A
            0xA0 => {
                self.alu_and(self.registers.b);
            } // AND B
            0xA1 => {
                self.alu_and(self.registers.c);
            } // AND C
            0xA2 => {
                self.alu_and(self.registers.d);
            } // AND D
            0xA3 => {
                self.alu_and(self.registers.e);
            } // AND E
            0xA4 => {
                self.alu_and(self.registers.h);
            } // AND H
            0xA5 => {
                self.alu_and(self.registers.l);
            } // AND L
            0xA6 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let v = self.mmu.read_byte(addr);
                self.alu_and(v);
            } // AND (HL)
            0xA7 => {
                self.alu_and(self.registers.a);
            } // AND A
            0xA8 => {
                self.alu_xor(self.registers.b);
            } // XOR B
            0xA9 => {
                self.alu_xor(self.registers.c);
            } // XOR C
            0xAA => {
                self.alu_xor(self.registers.d);
            } // XOR D
            0xAB => {
                self.alu_xor(self.registers.e);
            } // XOR E
            0xAC => {
                self.alu_xor(self.registers.h);
            } // XOR H
            0xAD => {
                self.alu_xor(self.registers.l);
            } // XOR L
            0xAE => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let v = self.mmu.read_byte(addr);
                self.alu_xor(v);
            } // XOR (HL)
            0xAF => {
                self.alu_xor(self.registers.a);
            } // XOR A
            0xB0 => {
                self.alu_or(self.registers.b);
            } // OR B
            0xB1 => {
                self.alu_or(self.registers.c);
            } // OR C
            0xB2 => {
                self.alu_or(self.registers.d);
            } // OR D
            0xB3 => {
                self.alu_or(self.registers.e);
            } // OR E
            0xB4 => {
                self.alu_or(self.registers.h);
            } // OR H
            0xB5 => {
                self.alu_or(self.registers.l);
            } // OR L
            0xB6 => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let v = self.mmu.read_byte(addr);
                self.alu_or(v);
            } // OR (HL)
            0xB7 => {
                self.alu_or(self.registers.a);
            } // OR A
            0xB8 => {
                self.alu_cp(self.registers.b);
            } // CP B
            0xB9 => {
                self.alu_cp(self.registers.c);
            } // CP C
            0xBA => {
                self.alu_cp(self.registers.d);
            } // CP D
            0xBB => {
                self.alu_cp(self.registers.e);
            } // CP E
            0xBC => {
                self.alu_cp(self.registers.h);
            } // CP H
            0xBD => {
                self.alu_cp(self.registers.l);
            } // CP L
            0xBE => {
                let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                let v = self.mmu.read_byte(addr);
                self.alu_cp(v);
            } // CP (HL)
            0xBF => {
                self.alu_cp(self.registers.a);
            } // CP A
            // 0xCB 前綴指令建議分開補齊，避免 match 過大
            0xCB => {
                let cb_opcode = self.fetch();
                match cb_opcode {
                    // RLC r
                    0x00 => {
                        self.registers.b = self.rlc(self.registers.b);
                    }
                    0x01 => {
                        self.registers.c = self.rlc(self.registers.c);
                    }
                    0x02 => {
                        self.registers.d = self.rlc(self.registers.d);
                    }
                    0x03 => {
                        self.registers.e = self.rlc(self.registers.e);
                    }
                    0x04 => {
                        self.registers.h = self.rlc(self.registers.h);
                    }
                    0x05 => {
                        self.registers.l = self.rlc(self.registers.l);
                    }
                    0x06 => {
                        let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                        let v = self.mmu.read_byte(addr);
                        let r = self.rlc(v);
                        self.mmu.write_byte(addr, r);
                    }
                    0x07 => {
                        self.registers.a = self.rlc(self.registers.a);
                    }
                    // RRC: Rotate Right Circular
                    0x08 => {
                        self.registers.b = self.rrc(self.registers.b);
                    }
                    0x09 => {
                        self.registers.c = self.rrc(self.registers.c);
                    }
                    0x0A => {
                        self.registers.d = self.rrc(self.registers.d);
                    }
                    0x0B => {
                        self.registers.e = self.rrc(self.registers.e);
                    }
                    0x0C => {
                        self.registers.h = self.rrc(self.registers.h);
                    }
                    0x0D => {
                        self.registers.l = self.rrc(self.registers.l);
                    }
                    0x0E => {
                        let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                        let v = self.mmu.read_byte(addr);
                        let r = self.rrc(v);
                        self.mmu.write_byte(addr, r);
                    }
                    0x0F => {
                        self.registers.a = self.rrc(self.registers.a);
                    }
                    // RL: Rotate Left through Carry
                    0x10 => {
                        self.registers.b = self.rl(self.registers.b);
                    }
                    0x11 => {
                        self.registers.c = self.rl(self.registers.c);
                    }
                    0x12 => {
                        self.registers.d = self.rl(self.registers.d);
                    }
                    0x13 => {
                        self.registers.e = self.rl(self.registers.e);
                    }
                    0x14 => {
                        self.registers.h = self.rl(self.registers.h);
                    }
                    0x15 => {
                        self.registers.l = self.rl(self.registers.l);
                    }
                    0x16 => {
                        let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                        let v = self.mmu.read_byte(addr);
                        let r = self.rl(v);
                        self.mmu.write_byte(addr, r);
                    }
                    0x17 => {
                        self.registers.a = self.rl(self.registers.a);
                    }
                    // RR: Rotate Right through Carry
                    0x18 => {
                        self.registers.b = self.rr(self.registers.b);
                    }
                    0x19 => {
                        self.registers.c = self.rr(self.registers.c);
                    }
                    0x1A => {
                        self.registers.d = self.rr(self.registers.d);
                    }
                    0x1B => {
                        self.registers.e = self.rr(self.registers.e);
                    }
                    0x1C => {
                        self.registers.h = self.rr(self.registers.h);
                    }
                    0x1D => {
                        self.registers.l = self.rr(self.registers.l);
                    }
                    0x1E => {
                        let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                        let v = self.mmu.read_byte(addr);
                        let r = self.rr(v);
                        self.mmu.write_byte(addr, r);
                    }
                    0x1F => {
                        self.registers.a = self.rr(self.registers.a);
                    }
                    // SLA: Shift Left Arithmetic
                    0x20 => {
                        self.registers.b = self.sla(self.registers.b);
                    }
                    0x21 => {
                        self.registers.c = self.sla(self.registers.c);
                    }
                    0x22 => {
                        self.registers.d = self.sla(self.registers.d);
                    }
                    0x23 => {
                        self.registers.e = self.sla(self.registers.e);
                    }
                    0x24 => {
                        self.registers.h = self.sla(self.registers.h);
                    }
                    0x25 => {
                        self.registers.l = self.sla(self.registers.l);
                    }
                    0x26 => {
                        let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                        let v = self.mmu.read_byte(addr);
                        let r = self.sla(v);
                        self.mmu.write_byte(addr, r);
                    }
                    0x27 => {
                        self.registers.a = self.sla(self.registers.a);
                    }
                    // SRA: Shift Right Arithmetic
                    0x28 => {
                        self.registers.b = self.sra(self.registers.b);
                    }
                    0x29 => {
                        self.registers.c = self.sra(self.registers.c);
                    }
                    0x2A => {
                        self.registers.d = self.sra(self.registers.d);
                    }
                    0x2B => {
                        self.registers.e = self.sra(self.registers.e);
                    }
                    0x2C => {
                        self.registers.h = self.sra(self.registers.h);
                    }
                    0x2D => {
                        self.registers.l = self.sra(self.registers.l);
                    }
                    0x2E => {
                        let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                        let v = self.mmu.read_byte(addr);
                        let r = self.sra(v);
                        self.mmu.write_byte(addr, r);
                    }
                    0x2F => {
                        self.registers.a = self.sra(self.registers.a);
                    }
                    // SWAP: Swap nibbles
                    0x30 => {
                        self.registers.b = self.swap(self.registers.b);
                    }
                    0x31 => {
                        self.registers.c = self.swap(self.registers.c);
                    }
                    0x32 => {
                        self.registers.d = self.swap(self.registers.d);
                    }
                    0x33 => {
                        self.registers.e = self.swap(self.registers.e);
                    }
                    0x34 => {
                        self.registers.h = self.swap(self.registers.h);
                    }
                    0x35 => {
                        self.registers.l = self.swap(self.registers.l);
                    }
                    0x36 => {
                        let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                        let v = self.mmu.read_byte(addr);
                        let r = self.swap(v);
                        self.mmu.write_byte(addr, r);
                    }
                    0x37 => {
                        self.registers.a = self.swap(self.registers.a);
                    }
                    // SRL: Shift Right Logical
                    0x38 => {
                        self.registers.b = self.srl(self.registers.b);
                    }
                    0x39 => {
                        self.registers.c = self.srl(self.registers.c);
                    }
                    0x3A => {
                        self.registers.d = self.srl(self.registers.d);
                    }
                    0x3B => {
                        self.registers.e = self.srl(self.registers.e);
                    }
                    0x3C => {
                        self.registers.h = self.srl(self.registers.h);
                    }
                    0x3D => {
                        self.registers.l = self.srl(self.registers.l);
                    }
                    0x3E => {
                        let addr = ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                        let v = self.mmu.read_byte(addr);
                        let r = self.srl(v);
                        self.mmu.write_byte(addr, r);
                    }
                    0x3F => {
                        self.registers.a = self.srl(self.registers.a);
                    }
                    // BIT/RES/SET
                    // BIT b, r
                    0x40..=0x7F => {
                        let bit = (cb_opcode - 0x40) / 8;
                        let reg = (cb_opcode - 0x40) % 8;
                        let value = match reg {
                            0 => self.registers.b,
                            1 => self.registers.c,
                            2 => self.registers.d,
                            3 => self.registers.e,
                            4 => self.registers.h,
                            5 => self.registers.l,
                            6 => {
                                let addr =
                                    ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                                self.mmu.read_byte(addr)
                            }
                            7 => self.registers.a,
                            _ => 0,
                        };
                        let z = (value & (1 << bit)) == 0;
                        self.registers.set_z_flag(z);
                        self.registers.set_n_flag(false);
                        self.registers.set_h_flag(true);
                    }
                    // RES b, r
                    0x80..=0xBF => {
                        let bit = (cb_opcode - 0x80) / 8;
                        let reg = (cb_opcode - 0x80) % 8;
                        match reg {
                            0 => self.registers.b &= !(1 << bit),
                            1 => self.registers.c &= !(1 << bit),
                            2 => self.registers.d &= !(1 << bit),
                            3 => self.registers.e &= !(1 << bit),
                            4 => self.registers.h &= !(1 << bit),
                            5 => self.registers.l &= !(1 << bit),
                            6 => {
                                let addr =
                                    ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                                let v = self.mmu.read_byte(addr) & !(1 << bit);
                                self.mmu.write_byte(addr, v);
                            }
                            7 => self.registers.a &= !(1 << bit),
                            _ => {}
                        }
                    }
                    // SET b, r
                    0xC0..=0xFF => {
                        let bit = (cb_opcode - 0xC0) / 8;
                        let reg = (cb_opcode - 0xC0) % 8;
                        match reg {
                            0 => self.registers.b |= 1 << bit,
                            1 => self.registers.c |= 1 << bit,
                            2 => self.registers.d |= 1 << bit,
                            3 => self.registers.e |= 1 << bit,
                            4 => self.registers.h |= 1 << bit,
                            5 => self.registers.l |= 1 << bit,
                            6 => {
                                let addr =
                                    ((self.registers.h as u16) << 8) | (self.registers.l as u16);
                                let v = self.mmu.read_byte(addr) | (1 << bit);
                                self.mmu.write_byte(addr, v);
                            }
                            7 => self.registers.a |= 1 << bit,
                            _ => {}
                        }
                    } // 所有 CB 指令都已處理
                }
            }
            0xC9 => {
                let lo = self.mmu.read_byte(self.registers.sp);
                self.registers.sp = self.registers.sp.wrapping_add(1);
                let hi = self.mmu.read_byte(self.registers.sp);
                self.registers.sp = self.registers.sp.wrapping_add(1);
                self.registers.pc = ((hi as u16) << 8) | (lo as u16);
            } // RET
            0xCD => {
                let lo = self.fetch() as u16;
                let hi = self.fetch() as u16;
                let addr = (hi << 8) | lo;
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, ((self.registers.pc >> 8) & 0xFF) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                self.registers.pc = addr;
            } // CALL nn
            0xCF => {
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, ((self.registers.pc >> 8) & 0xFF) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                self.registers.pc = 0x0008;
            } // RST 08H
            0xE7 => {
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, ((self.registers.pc >> 8) & 0xFF) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                self.registers.pc = 0x0020;
            } // RST 20H
            0xFF => {
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, ((self.registers.pc >> 8) & 0xFF) as u8);
                self.registers.sp = self.registers.sp.wrapping_sub(1);
                self.mmu
                    .write_byte(self.registers.sp, (self.registers.pc & 0xFF) as u8);
                self.registers.pc = 0x0038;
            } // RST 38H
            _ => {
                println!("未處理的指令: 0x{:02X}", opcode);
            }
        }
    }
}
